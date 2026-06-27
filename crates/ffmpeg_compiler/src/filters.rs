use std::collections::BTreeMap;
use std::path::Path;

use draft_model::{
    MaterialId, MaterialKind, Microseconds, RetimeMode, SegmentBackgroundFilling, SegmentBlendMode,
    SegmentCrop, SegmentFitMode, SegmentId, SegmentMask, SegmentScale, SegmentVisual,
    SourceTimerange, TargetTimerange,
};
use render_graph::{
    OutputDimensions, RenderAudioEffectSlotSupport, RenderAudioMix, RenderAudioMixDiagnostic,
    RenderCanvasBackgroundMode, RenderGraphPlan, RenderIntentSupport, RenderMaterial,
    RenderOutputProfile, RenderTransitionIntent, RenderVideoLayer,
};
use serde::{Deserialize, Serialize};

use crate::effects::{
    compile_audio_retime_filters, compile_dissolve_transition_filter,
    compile_phase19_mask_alpha_filters, compile_production_effect_filters,
    compile_video_retime_filters, retimed_source_timerange_for_output,
};
use crate::job::{
    CompileContext, FfmpegCompileError, FfmpegCompileErrorKind, FfmpegInput, FfmpegSidecar,
    format_seconds, input_index_by_material, sanitize_id,
};

#[allow(dead_code)]
const PHASE19_PRODUCTION_EFFECT_COMPILER_MARKERS: &[&str] = &[
    "RenderRetimeIntent",
    "RenderTransitionWindow",
    "ProductionEffectCapabilityDecision",
    "UnsupportedProductionEffect",
    "compile_production_effect_filters",
    "compile_phase19_mask_alpha_filters",
    "xfade",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedFilterScript {
    pub path: String,
    pub contents: String,
    pub has_audio_output: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<RenderAudioMixDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LayerDimensions {
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LayerPlacement {
    x: i64,
    y: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisualLayerFilter {
    segment_id: SegmentId,
    label: String,
    transition: Option<RenderTransitionIntent>,
    dimensions: LayerDimensions,
    placement: LayerPlacement,
    active_start: Microseconds,
    active_end: Microseconds,
    lines: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayerTimelineAlignment {
    OutputTimeline,
    SegmentLocal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AudioTimelineAlignment {
    OutputTimeline,
    TrackLocal,
}

pub fn generate_filter_script(
    plan: &RenderGraphPlan,
    context: &CompileContext,
    inputs: &[FfmpegInput],
    ass_sidecars: &[FfmpegSidecar],
    job_id: &str,
) -> Result<GeneratedFilterScript, FfmpegCompileError> {
    let path = context.artifact_path(&format!("{job_id}-filter.ffscript"));
    let input_indexes = input_index_by_material(inputs);

    let dimensions = match &plan.output_profile {
        RenderOutputProfile::PreviewFrame { dimensions, .. }
        | RenderOutputProfile::PreviewSegment { dimensions, .. }
        | RenderOutputProfile::ExportMp4 { dimensions, .. } => dimensions,
    };

    let mut lines = Vec::new();
    let material_dimensions = material_dimensions_by_id(plan);
    let concat_visual_post_filters = concat_visual_post_filters(
        plan,
        output_layer_dimensions(dimensions),
        &material_dimensions,
    );
    let concat_visual_layers = concat_visual_post_filters.is_some();
    let visual_alignment = if concat_visual_layers {
        LayerTimelineAlignment::SegmentLocal
    } else {
        LayerTimelineAlignment::OutputTimeline
    };

    let mut visual_layers = Vec::new();
    for (layer_index, layer) in plan.graph.video_layers.iter().enumerate() {
        let input_index = input_indexes
            .get(&layer.material_id)
            .ok_or_else(|| missing_input(&layer.material_id))?;
        let Some(clip) = clipped_layer_source_timerange(layer, output_timerange(plan)) else {
            continue;
        };
        let source_dimensions = material_dimensions
            .get(&layer.material_id)
            .copied()
            .unwrap_or_else(|| output_layer_dimensions(dimensions));
        let visual_layer = compile_visual_layer(
            layer_index,
            *input_index,
            layer,
            &clip,
            source_dimensions,
            output_layer_dimensions(dimensions),
            plan,
            visual_alignment,
        );
        lines.extend(visual_layer.lines.iter().cloned());
        visual_layers.push(visual_layer);
    }
    if !concat_visual_layers {
        append_transition_filters(&mut lines, &mut visual_layers, output_timerange(plan));
    }

    let mut current_video = if visual_layers.is_empty() {
        let background_color = canvas_background_color_arg(plan);
        lines.push(format!(
            "color=c={background_color}:s={width}x{height}:r={rate}:d={duration}[vbase0]",
            width = dimensions.width,
            height = dimensions.height,
            rate = frame_rate_arg(plan),
            duration = format_seconds(output_duration(plan))
        ));
        "vbase0".to_owned()
    } else if concat_visual_layers {
        compose_concatenated_visual_layers(
            &mut lines,
            &visual_layers,
            concat_visual_post_filters.as_deref().unwrap_or(&[]),
        )
    } else {
        compose_placed_visual_layers(&mut lines, plan, dimensions, &visual_layers)
    };

    if !plan.graph.text_overlays.is_empty() && ass_sidecars.is_empty() {
        return Err(FfmpegCompileError::new(
            FfmpegCompileErrorKind::MissingTextFont,
            "text overlays did not produce an ASS sidecar",
            "Compile ASS sidecars before generating the filter script.",
        ));
    }
    for (text_index, sidecar) in ass_sidecars.iter().enumerate() {
        let out = format!("vtext{text_index}");
        lines.push(format!(
            "[{current_video}]{}[{out}]",
            subtitles_filter(sidecar)
        ));
        current_video = out;
    }
    lines.push(format!("[{current_video}]format=yuv420p[vout]"));

    let supports_audio_output = !matches!(
        plan.output_profile,
        RenderOutputProfile::PreviewFrame { .. }
    );
    let mut diagnostics = Vec::new();
    let mut audio_labels = Vec::new();
    if supports_audio_output {
        let output_range = output_timerange(plan);
        let concat_audio_tracks = can_concat_audio_mixes(&plan.graph.audio_mixes, output_range);
        let audio_alignment = if concat_audio_tracks {
            AudioTimelineAlignment::TrackLocal
        } else {
            AudioTimelineAlignment::OutputTimeline
        };
        for (audio_index, audio) in plan.graph.audio_mixes.iter().enumerate() {
            let Some(clip) = clipped_audio_source_timerange(audio, output_range) else {
                continue;
            };
            reject_unsupported_audio_automation(audio)?;
            let input_index = input_indexes
                .get(&audio.material_id)
                .ok_or_else(|| missing_input(&audio.material_id))?;
            let label = format!("a{audio_index}");
            let compiled_audio =
                compile_audio_mix_filters(audio, &clip, output_range, audio_alignment);
            lines.push(format!(
                "[{input_index}:a]{}[{label}]",
                compiled_audio.filters.join(",")
            ));
            diagnostics.extend(compiled_audio.diagnostics);
            diagnostics.extend(audio_effect_slot_diagnostics(audio));
            audio_labels.push(AudioLabel {
                track_id: audio.track_id.as_str().to_owned(),
                label,
                active_start: target_delay_from_output(&audio.target_timerange, output_range),
                active_end: Microseconds::new(
                    target_delay_from_output(&audio.target_timerange, output_range)
                        .get()
                        .saturating_add(clip.duration.get()),
                ),
            });
        }
    }
    let has_audio_output = !audio_labels.is_empty();
    if has_audio_output {
        append_audio_output_filters(
            &mut lines,
            &audio_labels,
            output_timerange(plan),
            can_concat_audio_mixes(&plan.graph.audio_mixes, output_timerange(plan)),
        );
    }

    Ok(GeneratedFilterScript {
        path,
        contents: lines.join(";\n"),
        has_audio_output,
        diagnostics,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AudioLabel {
    track_id: String,
    label: String,
    active_start: Microseconds,
    active_end: Microseconds,
}

fn append_audio_output_filters(
    lines: &mut Vec<String>,
    audio_labels: &[AudioLabel],
    output: &TargetTimerange,
    allow_track_concat: bool,
) {
    let mix_labels = audio_mix_labels(lines, audio_labels, output, allow_track_concat);
    if mix_labels.len() == 1 {
        lines.push(format!("[{}]anull[aout]", mix_labels[0]));
    } else if mix_labels.len() > 1 {
        let inputs = mix_labels
            .iter()
            .map(|label| format!("[{label}]"))
            .collect::<String>();
        lines.push(format!(
            "{inputs}amix=inputs={}:duration=longest:normalize=0[aout]",
            mix_labels.len()
        ));
    }
}

fn audio_mix_labels(
    lines: &mut Vec<String>,
    audio_labels: &[AudioLabel],
    output: &TargetTimerange,
    allow_track_concat: bool,
) -> Vec<String> {
    if !allow_track_concat {
        return audio_labels
            .iter()
            .map(|label| label.label.clone())
            .collect();
    }

    let mut by_track = BTreeMap::<String, Vec<&AudioLabel>>::new();
    for label in audio_labels {
        by_track
            .entry(label.track_id.clone())
            .or_default()
            .push(label);
    }

    if by_track
        .values()
        .all(|labels| audio_track_can_concat(labels, output))
    {
        return by_track
            .into_iter()
            .enumerate()
            .map(|(track_index, (_track_id, labels))| {
                if labels.len() == 1 {
                    return labels[0].label.clone();
                }
                let output_label = format!("atrack{track_index}");
                let inputs = labels
                    .iter()
                    .map(|label| format!("[{}]", label.label))
                    .collect::<String>();
                lines.push(format!(
                    "{inputs}concat=n={}:v=0:a=1[{output_label}]",
                    labels.len()
                ));
                output_label
            })
            .collect();
    }

    audio_labels
        .iter()
        .map(|label| label.label.clone())
        .collect()
}

fn can_concat_audio_mixes(audio_mixes: &[RenderAudioMix], output: &TargetTimerange) -> bool {
    if audio_mixes.is_empty() {
        return false;
    }

    let mut by_track = BTreeMap::<String, Vec<(Microseconds, Microseconds)>>::new();
    for audio in audio_mixes {
        if !is_unity_retime(&audio.retime)
            || !audio.volume_keyframes.is_empty()
            || audio.classification != render_graph::RenderAudioMixClassification::Audible
        {
            return false;
        }
        let Some(clip) = clipped_audio_source_timerange(audio, output) else {
            continue;
        };
        let active_start = target_delay_from_output(&audio.target_timerange, output);
        let active_end = Microseconds::new(active_start.get().saturating_add(clip.duration.get()));
        by_track
            .entry(audio.track_id.as_str().to_owned())
            .or_default()
            .push((active_start, active_end));
    }

    !by_track.is_empty()
        && by_track.values().all(|ranges| {
            let mut expected_start = Microseconds::ZERO;
            let mut ordered = ranges.clone();
            ordered.sort_by_key(|range| range.0);
            for (active_start, active_end) in ordered {
                if active_start != expected_start || active_end < active_start {
                    return false;
                }
                expected_start = active_end;
            }
            expected_start == output.duration
        })
}

fn is_unity_retime(retime: &render_graph::RenderRetimeIntent) -> bool {
    matches!(
        &retime.retiming.mode,
        RetimeMode::Constant { speed } if speed.numerator == 1 && speed.denominator == 1
    ) && retime.support == RenderIntentSupport::Supported
}

fn audio_track_can_concat(labels: &[&AudioLabel], output: &TargetTimerange) -> bool {
    if labels.is_empty() {
        return false;
    }
    let mut expected_start = Microseconds::ZERO;
    let mut ordered = labels.to_vec();
    ordered.sort_by_key(|label| label.active_start);
    for label in ordered {
        if label.active_start != expected_start || label.active_end < label.active_start {
            return false;
        }
        expected_start = label.active_end;
    }
    expected_start == output.duration
}

fn compile_audio_mix_filters(
    audio: &RenderAudioMix,
    clip: &SourceTimerange,
    output: &TargetTimerange,
    alignment: AudioTimelineAlignment,
) -> CompiledAudioMixFilters {
    let mut filters = vec![format!(
        "atrim=start={start}:duration={duration}",
        start = format_seconds(clip.start),
        duration = format_seconds(clip.duration)
    )];
    filters.push("asetpts=PTS-STARTPTS".to_owned());
    let delay = target_delay_from_output(&audio.target_timerange, output);
    if alignment == AudioTimelineAlignment::OutputTimeline && delay.get() > 0 {
        filters.push(format!(
            "adelay={delay}|{delay}",
            delay = delay_millis(delay)
        ));
    }
    filters.push(format!("volume={}", volume_arg(audio.gain_millis)));
    if audio.pan_balance_millis != 0 {
        filters.push(pan_filter(audio.pan_balance_millis));
    }
    if audio.fade_in_duration.get() > 0 {
        filters.push(format!(
            "afade=t=in:st=0:d={}",
            format_seconds(audio.fade_in_duration)
        ));
    }
    if audio.fade_out_duration.get() > 0 {
        let fade_start = clip
            .duration
            .get()
            .saturating_sub(audio.fade_out_duration.get());
        filters.push(format!(
            "afade=t=out:st={}:d={}",
            format_seconds(Microseconds::new(fade_start)),
            format_seconds(audio.fade_out_duration)
        ));
    }
    let retime = compile_audio_retime_filters(
        &audio.track_id,
        &audio.segment_id,
        &audio.material_id,
        &audio.retime,
    );
    filters.extend(retime.filters);
    CompiledAudioMixFilters {
        filters,
        diagnostics: retime.diagnostics,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompiledAudioMixFilters {
    filters: Vec<String>,
    diagnostics: Vec<RenderAudioMixDiagnostic>,
}

fn target_delay_from_output(target: &TargetTimerange, output: &TargetTimerange) -> Microseconds {
    let active_start = target.start.max(output.start);
    Microseconds::new(active_start.get().saturating_sub(output.start.get()))
}

fn delay_millis(value: Microseconds) -> u64 {
    (value.get().saturating_add(999)) / 1_000
}

fn pan_filter(balance_millis: i32) -> String {
    let clamped = balance_millis.clamp(-1_000, 1_000);
    let left = if clamped > 0 {
        1_000_i32.saturating_sub(clamped)
    } else {
        1_000
    };
    let right = if clamped < 0 {
        1_000_i32.saturating_add(clamped)
    } else {
        1_000
    };
    format!(
        "pan=stereo|c0={}.{}*c0|c1={}.{}*c1",
        left / 1_000,
        format!("{:03}", left.rem_euclid(1_000)),
        right / 1_000,
        format!("{:03}", right.rem_euclid(1_000))
    )
}

fn audio_effect_slot_diagnostics(audio: &RenderAudioMix) -> Vec<RenderAudioMixDiagnostic> {
    audio
        .effect_slots
        .iter()
        .filter(|slot| slot.enabled)
        .map(|slot| match slot.support {
            RenderAudioEffectSlotSupport::Unsupported => RenderAudioMixDiagnostic {
                track_id: audio.track_id.clone(),
                segment_id: audio.segment_id.clone(),
                material_id: audio.material_id.clone(),
                property: format!("audioEffectSlot.{}", slot.slot_id),
                support: RenderIntentSupport::Unsupported,
                reason: format!(
                    "unsupported audio effect slot {} is preserved for diagnostics",
                    slot.name
                ),
            },
        })
        .collect()
}

fn reject_unsupported_audio_automation(audio: &RenderAudioMix) -> Result<(), FfmpegCompileError> {
    if audio.volume_keyframes.is_empty() {
        return Ok(());
    }

    Err(FfmpegCompileError::new(
        FfmpegCompileErrorKind::UnsupportedAudioAutomation,
        format!(
            "audio segment {} contains keyframed volume automation that cannot be compiled safely",
            audio.segment_id.as_str()
        ),
        "Remove keyframed audio volume automation or compile it into an FFmpeg volume expression before export.",
    )
    .with_material_id(audio.material_id.clone()))
}

fn compile_visual_layer(
    layer_index: usize,
    input_index: u32,
    layer: &RenderVideoLayer,
    clip: &SourceTimerange,
    source_dimensions: LayerDimensions,
    output_dimensions: LayerDimensions,
    plan: &RenderGraphPlan,
    alignment: LayerTimelineAlignment,
) -> VisualLayerFilter {
    let label = format!("v{layer_index}");
    let active_start = target_delay_from_output(&layer.target_timerange, output_timerange(plan));
    let active_end = Microseconds::new(active_start.get().saturating_add(clip.duration.get()));
    let defer_visual_fit = alignment == LayerTimelineAlignment::SegmentLocal;
    let filter_target_delay = match alignment {
        LayerTimelineAlignment::OutputTimeline => active_start,
        LayerTimelineAlignment::SegmentLocal => Microseconds::ZERO,
    };
    if is_full_canvas_identity(&layer.visual) {
        let mut filters = visual_source_filters(layer, clip, filter_target_delay, plan);
        if source_dimensions != output_dimensions {
            filters.push(format!(
                "scale={width}:{height}",
                width = output_dimensions.width,
                height = output_dimensions.height
            ));
        }
        filters.extend(compile_production_effect_filters(&layer.filters));
        filters.extend(compile_phase19_mask_alpha_filters(
            &layer.mask,
            output_dimensions.width,
            output_dimensions.height,
        ));
        return VisualLayerFilter {
            segment_id: layer.segment_id.clone(),
            label: label.clone(),
            transition: layer.transition.clone(),
            dimensions: output_dimensions,
            placement: LayerPlacement { x: 0, y: 0 },
            active_start,
            active_end,
            lines: vec![format!("[{input_index}:v]{}[{label}]", filters.join(","))],
        };
    }

    let cropped_dimensions = if defer_visual_fit {
        source_dimensions
    } else {
        cropped_dimensions(source_dimensions, &layer.visual.transform.crop)
    };
    let (fit_filters, mut current_dimensions) = if defer_visual_fit {
        (Vec::new(), source_dimensions)
    } else {
        fit_mode_filters(
            &layer.visual.fit_mode,
            cropped_dimensions,
            output_dimensions,
        )
    };
    let mut source_filters = visual_source_filters(layer, clip, filter_target_delay, plan);
    if !defer_visual_fit && crop_is_active(&layer.visual.transform.crop) {
        let left = millis_of(
            source_dimensions.width,
            layer.visual.transform.crop.left_millis,
        );
        let top = millis_of(
            source_dimensions.height,
            layer.visual.transform.crop.top_millis,
        );
        source_filters.push(format!(
            "crop={width}:{height}:{left}:{top}",
            width = cropped_dimensions.width,
            height = cropped_dimensions.height
        ));
    }
    source_filters.extend(fit_filters);

    let mut lines = Vec::new();
    let mut current_label = format!("vstage{layer_index}a");
    lines.push(format!(
        "[{input_index}:v]{}[{current_label}]",
        source_filters.join(",")
    ));

    if let Some(background_color) = segment_background_color_arg(&layer.visual.background_filling)
        && current_dimensions != output_dimensions
    {
        let background_label = format!("vsegbg{layer_index}");
        let composed_label = format!("vstage{layer_index}b");
        let x =
            ((i64::from(output_dimensions.width) - i64::from(current_dimensions.width)) / 2).max(0);
        let y = ((i64::from(output_dimensions.height) - i64::from(current_dimensions.height)) / 2)
            .max(0);
        lines.push(format!(
            "color=c={background_color}:s={width}x{height}:r={rate}:d={duration}[{background_label}]",
            width = output_dimensions.width,
            height = output_dimensions.height,
            rate = frame_rate_arg(plan),
            duration = format_seconds(output_duration(plan))
        ));
        lines.push(format!(
            "[{background_label}][{current_label}]overlay=x={x}:y={y}:shortest=1[{composed_label}]"
        ));
        current_label = composed_label;
        current_dimensions = output_dimensions;
    }

    let effect_filters = compile_production_effect_filters(&layer.filters);
    if !effect_filters.is_empty() {
        let effect_label = format!("vstage{layer_index}effects");
        lines.push(format!(
            "[{current_label}]{}[{effect_label}]",
            effect_filters.join(",")
        ));
        current_label = effect_label;
    }

    let mask_filters = compile_phase19_mask_alpha_filters(
        &layer.mask,
        current_dimensions.width,
        current_dimensions.height,
    );
    if !mask_filters.is_empty() {
        let mask_label = format!("vstage{layer_index}mask");
        lines.push(format!(
            "[{current_label}]{}[{mask_label}]",
            mask_filters.join(",")
        ));
        current_label = mask_label;
    }

    let scaled_dimensions = scaled_dimensions(current_dimensions, &layer.visual.transform.scale);
    let rotated_dimensions =
        rotated_dimensions(scaled_dimensions, layer.visual.transform.rotation.degrees);
    let mut transform_filters = Vec::new();
    if scaled_dimensions != current_dimensions {
        transform_filters.push(format!(
            "scale={}:{}",
            scaled_dimensions.width, scaled_dimensions.height
        ));
    }
    let rotation_is_active = layer.visual.transform.rotation.degrees.rem_euclid(360) != 0;
    if rotation_is_active || layer.visual.transform.opacity.value_millis < 1_000 {
        transform_filters.push("format=rgba".to_owned());
    }
    if rotation_is_active {
        let angle = rotation_radians_arg(layer.visual.transform.rotation.degrees);
        transform_filters.push(format!(
            "rotate={angle}:ow=rotw({angle}):oh=roth({angle}):c=none"
        ));
    }
    if layer.visual.transform.opacity.value_millis < 1_000 {
        transform_filters.push(format!(
            "colorchannelmixer=aa={}",
            millis_decimal(layer.visual.transform.opacity.value_millis)
        ));
    }
    if transform_filters.is_empty() {
        lines.push(format!("[{current_label}]null[{label}]"));
    } else {
        lines.push(format!(
            "[{current_label}]{}[{label}]",
            transform_filters.join(",")
        ));
    }

    VisualLayerFilter {
        segment_id: layer.segment_id.clone(),
        label,
        transition: layer.transition.clone(),
        dimensions: rotated_dimensions,
        placement: layer_placement(&layer.visual, output_dimensions, rotated_dimensions),
        active_start,
        active_end,
        lines,
    }
}

fn append_transition_filters(
    lines: &mut Vec<String>,
    visual_layers: &mut Vec<VisualLayerFilter>,
    output: &TargetTimerange,
) {
    let mut transition_specs = visual_layers
        .iter()
        .filter_map(|layer| layer_transition_spec(layer, visual_layers, output))
        .collect::<Vec<_>>();

    let mut transition_uses = vec![Vec::new(); visual_layers.len()];
    for (spec_index, spec) in transition_specs.iter().enumerate() {
        transition_uses[spec.from_index].push((spec_index, TransitionEndpoint::From));
        transition_uses[spec.to_index].push((spec_index, TransitionEndpoint::To));
    }

    for (layer_index, uses) in transition_uses.into_iter().enumerate() {
        if uses.is_empty() {
            continue;
        }

        let layer_label = visual_layers[layer_index].label.clone();
        let main_label = format!("{layer_label}main");
        let mut output_labels = vec![main_label.clone()];
        for (use_index, (spec_index, endpoint)) in uses.into_iter().enumerate() {
            let transition_label = format!("{layer_label}transition{use_index}");
            match endpoint {
                TransitionEndpoint::From => {
                    transition_specs[spec_index].from_transition_label = transition_label.clone();
                }
                TransitionEndpoint::To => {
                    transition_specs[spec_index].to_transition_label = transition_label.clone();
                }
            }
            output_labels.push(transition_label);
        }
        lines.push(format!(
            "[{from}]split={outputs}{labels}",
            from = layer_label,
            outputs = output_labels.len(),
            labels = output_labels
                .iter()
                .map(|label| format!("[{label}]"))
                .collect::<String>(),
        ));
        visual_layers[layer_index].label = main_label;
    }

    for spec in transition_specs {
        let Some(filter) = compile_dissolve_transition_filter(
            &spec.transition,
            &spec.from_transition_label,
            &spec.to_transition_label,
            &spec.output_label,
            spec.offset,
        ) else {
            continue;
        };
        lines.push(filter);
        visual_layers.push(VisualLayerFilter {
            segment_id: spec.transition.from_segment_id.clone(),
            label: spec.output_label,
            transition: None,
            dimensions: spec.dimensions,
            placement: spec.placement,
            active_start: spec.active_start,
            active_end: spec.active_end,
            lines: Vec::new(),
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransitionEndpoint {
    From,
    To,
}

struct TransitionFilterSpec {
    from_index: usize,
    to_index: usize,
    from_transition_label: String,
    to_transition_label: String,
    output_label: String,
    transition: RenderTransitionIntent,
    offset: Microseconds,
    dimensions: LayerDimensions,
    placement: LayerPlacement,
    active_start: Microseconds,
    active_end: Microseconds,
}

fn layer_transition_spec(
    layer: &VisualLayerFilter,
    visual_layers: &[VisualLayerFilter],
    output: &TargetTimerange,
) -> Option<TransitionFilterSpec> {
    let transition = layer.transition.as_ref()?;
    if transition.support != RenderIntentSupport::Supported {
        return None;
    }
    let from_index = visual_layers
        .iter()
        .position(|candidate| candidate.segment_id == transition.from_segment_id)?;
    let to_index = visual_layers
        .iter()
        .position(|candidate| candidate.segment_id == transition.to_segment_id)?;
    if from_index == to_index {
        return None;
    }
    let active_start = target_delay_from_output(&transition.window.target_timerange, output);
    let active_end = Microseconds::new(
        active_start
            .get()
            .saturating_add(transition.window.target_timerange.duration.get()),
    );
    let offset = Microseconds::new(
        transition
            .window
            .target_timerange
            .start
            .get()
            .saturating_sub(output.start.get()),
    );

    Some(TransitionFilterSpec {
        from_index,
        to_index,
        from_transition_label: String::new(),
        to_transition_label: String::new(),
        output_label: transition_output_label(transition),
        transition: transition.clone(),
        offset,
        dimensions: visual_layers[from_index].dimensions,
        placement: visual_layers[from_index].placement,
        active_start,
        active_end,
    })
}

fn transition_output_label(transition: &RenderTransitionIntent) -> String {
    format!(
        "vtransition_{}_to_{}",
        sanitize_id(transition.from_segment_id.as_str()),
        sanitize_id(transition.to_segment_id.as_str())
    )
}

fn visual_source_filters(
    layer: &RenderVideoLayer,
    clip: &SourceTimerange,
    target_delay: Microseconds,
    plan: &RenderGraphPlan,
) -> Vec<String> {
    match layer.material_kind {
        MaterialKind::Image => vec![
            "loop=loop=-1:size=1:start=0".to_owned(),
            format!("fps={}", frame_rate_arg(plan)),
            format!("trim=duration={}", format_seconds(clip.duration)),
            compile_video_retime_filters(&layer.retime, target_delay).join(","),
        ],
        _ => vec![
            format!(
                "trim=start={start}:duration={duration}",
                start = format_seconds(clip.start),
                duration = format_seconds(clip.duration)
            ),
            compile_video_retime_filters(&layer.retime, target_delay).join(","),
        ],
    }
}

fn compose_placed_visual_layers(
    lines: &mut Vec<String>,
    plan: &RenderGraphPlan,
    dimensions: &OutputDimensions,
    layers: &[VisualLayerFilter],
) -> String {
    let background_color = canvas_background_color_arg(plan);
    lines.push(format!(
        "color=c={background_color}:s={width}x{height}:r={rate}:d={duration}[vbase0]",
        width = dimensions.width,
        height = dimensions.height,
        rate = frame_rate_arg(plan),
        duration = format_seconds(output_duration(plan))
    ));

    let mut current = "vbase0".to_owned();
    for (overlay_index, layer) in layers.iter().enumerate() {
        let out = format!("vbase{}", overlay_index + 1);
        lines.push(format!(
            "[{current}][{label}]overlay=x={x}:y={y}:enable='between(t,{start},{end})'[{out}]",
            label = layer.label,
            x = layer.placement.x,
            y = layer.placement.y,
            start = format_seconds(layer.active_start),
            end = format_seconds(layer.active_end)
        ));
        current = out;
    }
    current
}

fn compose_concatenated_visual_layers(
    lines: &mut Vec<String>,
    layers: &[VisualLayerFilter],
    post_filters: &[String],
) -> String {
    let output = "vconcat0".to_owned();
    if layers.len() == 1 {
        return apply_post_concat_visual_filters(lines, &layers[0].label, &output, post_filters);
    }

    let concat_output = if post_filters.is_empty() {
        output.clone()
    } else {
        "vconcatraw0".to_owned()
    };
    let inputs = layers
        .iter()
        .map(|layer| format!("[{}]", layer.label))
        .collect::<String>();
    lines.push(format!(
        "{inputs}concat=n={}:v=1:a=0[{concat_output}]",
        layers.len()
    ));
    apply_post_concat_visual_filters(lines, &concat_output, &output, post_filters)
}

fn apply_post_concat_visual_filters(
    lines: &mut Vec<String>,
    input: &str,
    output: &str,
    post_filters: &[String],
) -> String {
    if post_filters.is_empty() {
        return input.to_owned();
    }
    lines.push(format!("[{input}]{}[{output}]", post_filters.join(",")));
    output.to_owned()
}

fn concat_visual_post_filters(
    plan: &RenderGraphPlan,
    output_dimensions: LayerDimensions,
    material_dimensions: &BTreeMap<MaterialId, LayerDimensions>,
) -> Option<Vec<String>> {
    let layers = &plan.graph.video_layers;
    let Some(first_layer) = layers.first() else {
        return None;
    };
    let output = output_timerange(plan);
    let stack_index = first_layer.stack_index;
    let mut ranges = Vec::new();
    let mut shared_post_filters = None::<Vec<String>>;

    for layer in layers {
        if layer.stack_index != stack_index
            || layer.transition.is_some()
            || !layer.keyframes.is_empty()
            || !is_unity_retime(&layer.retime)
            || !visual_layer_renders_full_canvas(layer, output_dimensions, material_dimensions)
        {
            return None;
        }
        let Some((active_start, active_end)) = active_target_range(&layer.target_timerange, output)
        else {
            continue;
        };
        ranges.push((active_start, active_end));

        let source_dimensions = material_dimensions
            .get(&layer.material_id)
            .copied()
            .unwrap_or(output_dimensions);
        let post_filters = deferred_full_canvas_fit_filters(
            &layer.visual.fit_mode,
            source_dimensions,
            output_dimensions,
        );
        match &shared_post_filters {
            Some(shared) if shared != &post_filters => return None,
            Some(_) => {}
            None => shared_post_filters = Some(post_filters),
        }
    }

    if ranges_cover_output_contiguously(&ranges, output.duration) {
        shared_post_filters
    } else {
        None
    }
}

fn deferred_full_canvas_fit_filters(
    fit_mode: &SegmentFitMode,
    source_dimensions: LayerDimensions,
    output_dimensions: LayerDimensions,
) -> Vec<String> {
    let (mut filters, _dimensions) =
        fit_mode_filters(fit_mode, source_dimensions, output_dimensions);
    if filters.is_empty() {
        filters.push(format!(
            "scale={}:{}",
            output_dimensions.width, output_dimensions.height
        ));
    }
    filters
}

fn visual_layer_renders_full_canvas(
    layer: &RenderVideoLayer,
    output_dimensions: LayerDimensions,
    material_dimensions: &BTreeMap<MaterialId, LayerDimensions>,
) -> bool {
    if !layer.visual.visible
        || !layer.filters.is_empty()
        || !matches!(layer.mask.mask, SegmentMask::None)
        || layer.blend.blend_mode != SegmentBlendMode::Normal
        || layer.visual.blend_mode != SegmentBlendMode::Normal
        || !matches!(layer.visual.mask, SegmentMask::None)
        || layer.visual.transform.scale.x_millis != 1_000
        || layer.visual.transform.scale.y_millis != 1_000
        || layer.visual.transform.position.x != 0
        || layer.visual.transform.position.y != 0
        || layer.visual.transform.anchor.x_millis != 500
        || layer.visual.transform.anchor.y_millis != 500
        || layer.visual.transform.rotation.degrees.rem_euclid(360) != 0
        || layer.visual.transform.opacity.value_millis != 1_000
        || crop_is_active(&layer.visual.transform.crop)
    {
        return false;
    }

    let source_dimensions = material_dimensions
        .get(&layer.material_id)
        .copied()
        .unwrap_or(output_dimensions);
    let (_fit_filters, fitted_dimensions) =
        fit_mode_filters(&layer.visual.fit_mode, source_dimensions, output_dimensions);
    let placement = layer_placement(&layer.visual, output_dimensions, fitted_dimensions);
    fitted_dimensions == output_dimensions && placement.x == 0 && placement.y == 0
}

fn active_target_range(
    target: &TargetTimerange,
    output: &TargetTimerange,
) -> Option<(Microseconds, Microseconds)> {
    let target_start = target.start.get();
    let target_end = target_start.checked_add(target.duration.get())?;
    let output_start = output.start.get();
    let output_end = output_start.checked_add(output.duration.get())?;
    let active_start = target_start.max(output_start);
    let active_end = target_end.min(output_end);
    if active_start >= active_end {
        return None;
    }
    Some((
        Microseconds::new(active_start.saturating_sub(output_start)),
        Microseconds::new(active_end.saturating_sub(output_start)),
    ))
}

fn ranges_cover_output_contiguously(
    ranges: &[(Microseconds, Microseconds)],
    output_duration: Microseconds,
) -> bool {
    if ranges.is_empty() {
        return false;
    }
    let mut ordered = ranges.to_vec();
    ordered.sort_by_key(|range| range.0);
    let mut expected_start = Microseconds::ZERO;
    for (active_start, active_end) in ordered {
        if active_start != expected_start || active_end < active_start {
            return false;
        }
        expected_start = active_end;
    }
    expected_start == output_duration
}

fn missing_input(material_id: &MaterialId) -> FfmpegCompileError {
    FfmpegCompileError::new(
        FfmpegCompileErrorKind::MissingInputMaterial,
        format!(
            "render graph references material {} without a matching FFmpeg input",
            material_id.as_str()
        ),
        "Ensure render_graph materials include every renderable video/audio material.",
    )
    .with_material_id(material_id.clone())
}

fn is_full_canvas_identity(visual: &SegmentVisual) -> bool {
    visual.fit_mode == SegmentFitMode::Stretch
        && !crop_is_active(&visual.transform.crop)
        && visual.transform.scale.x_millis == 1_000
        && visual.transform.scale.y_millis == 1_000
        && visual.transform.position.x == 0
        && visual.transform.position.y == 0
        && visual.transform.anchor.x_millis == 500
        && visual.transform.anchor.y_millis == 500
        && visual.transform.rotation.degrees.rem_euclid(360) == 0
        && visual.transform.opacity.value_millis == 1_000
}

fn material_dimensions_by_id(plan: &RenderGraphPlan) -> BTreeMap<MaterialId, LayerDimensions> {
    plan.graph
        .materials
        .iter()
        .map(|material| {
            (
                material.material_id.clone(),
                render_material_dimensions(material),
            )
        })
        .collect()
}

fn render_material_dimensions(material: &RenderMaterial) -> LayerDimensions {
    LayerDimensions {
        width: material.width.unwrap_or(1).max(1),
        height: material.height.unwrap_or(1).max(1),
    }
}

fn output_layer_dimensions(dimensions: &OutputDimensions) -> LayerDimensions {
    LayerDimensions {
        width: dimensions.width.max(1),
        height: dimensions.height.max(1),
    }
}

fn cropped_dimensions(source: LayerDimensions, crop: &SegmentCrop) -> LayerDimensions {
    let left = millis_of(source.width, crop.left_millis);
    let right = millis_of(source.width, crop.right_millis);
    let top = millis_of(source.height, crop.top_millis);
    let bottom = millis_of(source.height, crop.bottom_millis);
    LayerDimensions {
        width: source
            .width
            .saturating_sub(left)
            .saturating_sub(right)
            .max(1),
        height: source
            .height
            .saturating_sub(top)
            .saturating_sub(bottom)
            .max(1),
    }
}

fn crop_is_active(crop: &SegmentCrop) -> bool {
    crop.left_millis > 0 || crop.right_millis > 0 || crop.top_millis > 0 || crop.bottom_millis > 0
}

fn fit_mode_filters(
    fit_mode: &SegmentFitMode,
    source: LayerDimensions,
    output: LayerDimensions,
) -> (Vec<String>, LayerDimensions) {
    match fit_mode {
        SegmentFitMode::Stretch => {
            let filters = if source == output {
                Vec::new()
            } else {
                vec![format!("scale={}:{}", output.width, output.height)]
            };
            (filters, output)
        }
        SegmentFitMode::Fit => {
            let fitted = fit_dimensions(source, output);
            let filters = if fitted == source {
                Vec::new()
            } else {
                vec![format!("scale={}:{}", fitted.width, fitted.height)]
            };
            (filters, fitted)
        }
        SegmentFitMode::Fill => {
            let filled = fill_dimensions(source, output);
            let x = ((i64::from(filled.width) - i64::from(output.width)) / 2).max(0);
            let y = ((i64::from(filled.height) - i64::from(output.height)) / 2).max(0);
            let mut filters = Vec::new();
            if filled != source {
                filters.push(format!("scale={}:{}", filled.width, filled.height));
            }
            if filled != output || x != 0 || y != 0 {
                filters.push(format!("crop={}:{}:{x}:{y}", output.width, output.height));
            }
            (filters, output)
        }
    }
}

fn fit_dimensions(source: LayerDimensions, output: LayerDimensions) -> LayerDimensions {
    if u64::from(output.width) * u64::from(source.height)
        <= u64::from(output.height) * u64::from(source.width)
    {
        LayerDimensions {
            width: output.width,
            height: proportional_dimension(source.height, output.width, source.width),
        }
    } else {
        LayerDimensions {
            width: proportional_dimension(source.width, output.height, source.height),
            height: output.height,
        }
    }
}

fn fill_dimensions(source: LayerDimensions, output: LayerDimensions) -> LayerDimensions {
    if u64::from(output.width) * u64::from(source.height)
        >= u64::from(output.height) * u64::from(source.width)
    {
        LayerDimensions {
            width: output.width,
            height: proportional_dimension(source.height, output.width, source.width),
        }
    } else {
        LayerDimensions {
            width: proportional_dimension(source.width, output.height, source.height),
            height: output.height,
        }
    }
}

fn proportional_dimension(source_span: u32, target_span: u32, reference_span: u32) -> u32 {
    round_div_u64(
        u64::from(source_span) * u64::from(target_span),
        u64::from(reference_span.max(1)),
    )
    .max(1)
    .min(u64::from(u32::MAX)) as u32
}

fn scaled_dimensions(dimensions: LayerDimensions, scale: &SegmentScale) -> LayerDimensions {
    LayerDimensions {
        width: millis_of(dimensions.width, scale.x_millis).max(1),
        height: millis_of(dimensions.height, scale.y_millis).max(1),
    }
}

fn rotated_dimensions(dimensions: LayerDimensions, degrees: i32) -> LayerDimensions {
    if degrees.rem_euclid(360) == 0 {
        return dimensions;
    }

    let radians = f64::from(degrees).to_radians();
    let sin = normalized_abs_trig(radians.sin());
    let cos = normalized_abs_trig(radians.cos());
    LayerDimensions {
        width: ceil_dimension(
            f64::from(dimensions.width) * cos + f64::from(dimensions.height) * sin,
        ),
        height: ceil_dimension(
            f64::from(dimensions.width) * sin + f64::from(dimensions.height) * cos,
        ),
    }
}

fn normalized_abs_trig(value: f64) -> f64 {
    let value = value.abs();
    if value < 1e-9 { 0.0 } else { value }
}

fn ceil_dimension(value: f64) -> u32 {
    if !value.is_finite() {
        return 1;
    }
    value.ceil().max(1.0).min(f64::from(u32::MAX)) as u32
}

fn layer_placement(
    visual: &SegmentVisual,
    output: LayerDimensions,
    layer: LayerDimensions,
) -> LayerPlacement {
    let center_x = normalized_millis_to_canvas_pixel(output.width, visual.transform.position.x);
    let center_y = normalized_millis_to_canvas_pixel(output.height, -visual.transform.position.y);
    LayerPlacement {
        x: center_x - i64::from(millis_of(layer.width, visual.transform.anchor.x_millis)),
        y: center_y - i64::from(millis_of(layer.height, visual.transform.anchor.y_millis)),
    }
}

fn normalized_millis_to_canvas_pixel(span: u32, value_millis: i32) -> i64 {
    (i64::from(span) * i64::from(1_000 + value_millis)) / 2_000
}

fn millis_of(span: u32, millis: u32) -> u32 {
    round_div_u64(u64::from(span) * u64::from(millis), 1_000).min(u64::from(u32::MAX)) as u32
}

fn round_div_u64(numerator: u64, denominator: u64) -> u64 {
    if denominator == 0 {
        return 0;
    }
    (numerator + denominator / 2) / denominator
}

fn millis_decimal(millis: u32) -> String {
    format!("{}.{:03}", millis / 1_000, millis % 1_000)
}

fn rotation_radians_arg(degrees: i32) -> String {
    format!("{:.6}", f64::from(degrees).to_radians())
}

fn segment_background_color_arg(background: &SegmentBackgroundFilling) -> Option<String> {
    match background {
        SegmentBackgroundFilling::Black => Some("black".to_owned()),
        SegmentBackgroundFilling::SolidColor { color } => hex_color_to_ffmpeg_arg(color),
        SegmentBackgroundFilling::None
        | SegmentBackgroundFilling::Blur
        | SegmentBackgroundFilling::Image { .. } => None,
    }
}

fn output_duration(plan: &RenderGraphPlan) -> Microseconds {
    output_timerange(plan).duration
}

fn output_timerange(plan: &RenderGraphPlan) -> &TargetTimerange {
    match &plan.output_profile {
        RenderOutputProfile::PreviewFrame {
            target_timerange, ..
        }
        | RenderOutputProfile::PreviewSegment {
            target_timerange, ..
        }
        | RenderOutputProfile::ExportMp4 {
            target_timerange, ..
        } => target_timerange,
    }
}

fn clipped_layer_source_timerange(
    layer: &RenderVideoLayer,
    output: &TargetTimerange,
) -> Option<SourceTimerange> {
    retimed_source_timerange_for_output(&layer.retime.source_mapping, output).or_else(|| {
        clipped_source_timerange(&layer.source_timerange, &layer.target_timerange, output)
    })
}

fn clipped_audio_source_timerange(
    audio: &RenderAudioMix,
    output: &TargetTimerange,
) -> Option<SourceTimerange> {
    retimed_source_timerange_for_output(&audio.retime.source_mapping, output).or_else(|| {
        clipped_source_timerange(&audio.source_timerange, &audio.target_timerange, output)
    })
}

fn clipped_source_timerange(
    source: &SourceTimerange,
    target: &TargetTimerange,
    output: &TargetTimerange,
) -> Option<SourceTimerange> {
    let target_start = target.start.get();
    let target_end = target_start.checked_add(target.duration.get())?;
    let output_start = output.start.get();
    let output_end = output_start.checked_add(output.duration.get())?;
    let active_start = target_start.max(output_start);
    let active_end = target_end.min(output_end);
    if active_start >= active_end {
        return None;
    }
    let source_offset = active_start.checked_sub(target_start)?;
    let source_start = source.start.get().checked_add(source_offset)?;
    Some(SourceTimerange::new(
        Microseconds::new(source_start),
        Microseconds::new(active_end - active_start),
    ))
}

fn frame_rate_arg(plan: &RenderGraphPlan) -> String {
    let frame_rate = match &plan.output_profile {
        RenderOutputProfile::PreviewFrame { frame_rate, .. }
        | RenderOutputProfile::PreviewSegment { frame_rate, .. }
        | RenderOutputProfile::ExportMp4 { frame_rate, .. } => frame_rate,
    };
    format!("{}/{}", frame_rate.numerator, frame_rate.denominator)
}

fn canvas_background_color_arg(plan: &RenderGraphPlan) -> String {
    let background = &plan.graph.canvas.background;
    if background.mode != RenderCanvasBackgroundMode::SolidColor {
        return "black".to_owned();
    }

    background
        .color
        .as_deref()
        .and_then(hex_color_to_ffmpeg_arg)
        .unwrap_or_else(|| "black".to_owned())
}

fn hex_color_to_ffmpeg_arg(value: &str) -> Option<String> {
    let hex = value.trim().strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return None;
    }

    Some(format!("0x{}", hex.to_ascii_uppercase()))
}

fn volume_arg(level_millis: u32) -> String {
    format!("{}.{:03}", level_millis / 1_000, level_millis % 1_000)
}

fn escape_filter_path(path: &str) -> String {
    path.replace('\\', "\\\\").replace('\'', "\\'")
}

fn subtitles_filter(sidecar: &FfmpegSidecar) -> String {
    let filename = escape_filter_path(&sidecar.path);
    let Some(fonts_dir) = ass_font_dir(&sidecar.contents) else {
        return format!("subtitles='{filename}'");
    };
    format!(
        "subtitles=filename='{filename}':fontsdir='{}'",
        escape_filter_path(&fonts_dir)
    )
}

fn ass_font_dir(contents: &str) -> Option<String> {
    let font_path = contents
        .lines()
        .find_map(|line| line.strip_prefix("; FontPath: "))?
        .trim();
    Path::new(font_path)
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned())
        .filter(|parent| !parent.is_empty())
}

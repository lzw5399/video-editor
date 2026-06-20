use std::collections::BTreeMap;

use draft_model::{
    MaterialId, MaterialKind, Microseconds, SegmentBackgroundFilling, SegmentCrop,
    SegmentFitMode, SegmentScale, SegmentVisual, SourceTimerange, TargetTimerange,
};
use render_graph::{
    OutputDimensions, RenderAudioEffectSlotSupport, RenderAudioMix, RenderAudioMixDiagnostic,
    RenderCanvasBackgroundMode, RenderGraphPlan, RenderIntentSupport, RenderMaterial,
    RenderOutputProfile, RenderVideoLayer,
};
use serde::{Deserialize, Serialize};

use crate::job::{
    CompileContext, FfmpegCompileError, FfmpegCompileErrorKind, FfmpegInput, FfmpegSidecar,
    format_seconds, input_index_by_material, sanitize_id,
};

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
    label: String,
    placement: LayerPlacement,
    active_start: Microseconds,
    active_end: Microseconds,
    lines: Vec<String>,
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
    let ass_by_segment = ass_sidecars
        .iter()
        .filter_map(|sidecar| {
            sidecar
                .segment_id
                .as_ref()
                .map(|segment_id| (sanitize_id(segment_id.as_str()), sidecar.path.clone()))
        })
        .collect::<BTreeMap<_, _>>();

    let dimensions = match &plan.output_profile {
        RenderOutputProfile::PreviewFrame { dimensions, .. }
        | RenderOutputProfile::PreviewSegment { dimensions, .. }
        | RenderOutputProfile::ExportMp4 { dimensions, .. } => dimensions,
    };

    let mut lines = Vec::new();
    let material_dimensions = material_dimensions_by_id(plan);
    let mut visual_layers = Vec::new();
    for (layer_index, layer) in plan.graph.video_layers.iter().enumerate() {
        let input_index = input_indexes
            .get(&layer.material_id)
            .ok_or_else(|| missing_input(&layer.material_id))?;
        let Some(clip) = clipped_source_timerange(
            &layer.source_timerange,
            &layer.target_timerange,
            output_timerange(plan),
        ) else {
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
        );
        lines.extend(visual_layer.lines.iter().cloned());
        visual_layers.push(visual_layer);
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
    } else {
        compose_placed_visual_layers(&mut lines, plan, dimensions, &visual_layers)
    };

    for (text_index, overlay) in plan.graph.text_overlays.iter().enumerate() {
        let segment_key = sanitize_id(overlay.overlay.segment_id.as_str());
        let ass_path = ass_by_segment.get(&segment_key).ok_or_else(|| {
            FfmpegCompileError::new(
                FfmpegCompileErrorKind::MissingTextFont,
                format!(
                    "text segment {} did not produce an ASS sidecar",
                    overlay.overlay.segment_id.as_str()
                ),
                "Compile ASS sidecars before generating the filter script.",
            )
        })?;
        let out = format!("vtext{text_index}");
        lines.push(format!(
            "[{current_video}]subtitles='{}'[{out}]",
            escape_filter_path(ass_path)
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
        for (audio_index, audio) in plan.graph.audio_mixes.iter().enumerate() {
            let output_range = output_timerange(plan);
            let Some(clip) = clipped_source_timerange(
                &audio.source_timerange,
                &audio.target_timerange,
                output_range,
            ) else {
                continue;
            };
            reject_unsupported_audio_automation(audio)?;
            let input_index = input_indexes
                .get(&audio.material_id)
                .ok_or_else(|| missing_input(&audio.material_id))?;
            let label = format!("a{audio_index}");
            let filters = compile_audio_mix_filters(audio, &clip, output_range);
            lines.push(format!("[{input_index}:a]{}[{label}]", filters.join(",")));
            diagnostics.extend(audio_effect_slot_diagnostics(audio));
            audio_labels.push(label);
        }
    }
    let has_audio_output = !audio_labels.is_empty();
    if audio_labels.len() == 1 {
        lines.push(format!("[{}]anull[aout]", audio_labels[0]));
    } else if audio_labels.len() > 1 {
        let inputs = audio_labels
            .iter()
            .map(|label| format!("[{label}]"))
            .collect::<String>();
        lines.push(format!(
            "{inputs}amix=inputs={}:duration=longest:normalize=0[aout]",
            audio_labels.len()
        ));
    }

    Ok(GeneratedFilterScript {
        path,
        contents: lines.join(";\n"),
        has_audio_output,
        diagnostics,
    })
}

fn compile_audio_mix_filters(
    audio: &RenderAudioMix,
    clip: &SourceTimerange,
    output: &TargetTimerange,
) -> Vec<String> {
    let mut filters = vec![format!(
        "atrim=start={start}:duration={duration}",
        start = format_seconds(clip.start),
        duration = format_seconds(clip.duration)
    )];
    filters.push("asetpts=PTS-STARTPTS".to_owned());
    let delay = target_delay_from_output(&audio.target_timerange, output);
    if delay.get() > 0 {
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
    filters
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
) -> VisualLayerFilter {
    let label = format!("v{layer_index}");
    let active_start = target_delay_from_output(&layer.target_timerange, output_timerange(plan));
    let active_end = Microseconds::new(active_start.get().saturating_add(clip.duration.get()));
    if is_full_canvas_identity(&layer.visual) {
        return VisualLayerFilter {
            label: label.clone(),
            placement: LayerPlacement { x: 0, y: 0 },
            active_start,
            active_end,
            lines: vec![format!(
                "[{input_index}:v]{source_filters},scale={width}:{height}[{label}]",
                source_filters = visual_source_filters(layer, clip, active_start, plan).join(","),
                width = output_dimensions.width,
                height = output_dimensions.height
            )],
        };
    }

    let cropped_dimensions = cropped_dimensions(source_dimensions, &layer.visual.transform.crop);
    let (fit_filters, mut current_dimensions) = fit_mode_filters(
        &layer.visual.fit_mode,
        cropped_dimensions,
        output_dimensions,
    );
    let mut source_filters = visual_source_filters(layer, clip, active_start, plan);
    if crop_is_active(&layer.visual.transform.crop) {
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

    let scaled_dimensions = scaled_dimensions(current_dimensions, &layer.visual.transform.scale);
    let mut transform_filters = Vec::new();
    if scaled_dimensions != current_dimensions {
        transform_filters.push(format!(
            "scale={}:{}",
            scaled_dimensions.width, scaled_dimensions.height
        ));
    }
    if layer.visual.transform.opacity.value_millis < 1_000 {
        transform_filters.push("format=rgba".to_owned());
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
        label,
        placement: layer_placement(&layer.visual, output_dimensions, scaled_dimensions),
        active_start,
        active_end,
        lines,
    }
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
            visual_setpts_filter(target_delay),
        ],
        _ => vec![
            format!(
                "trim=start={start}:duration={duration}",
                start = format_seconds(clip.start),
                duration = format_seconds(clip.duration)
            ),
            visual_setpts_filter(target_delay),
        ],
    }
}

fn visual_setpts_filter(target_delay: Microseconds) -> String {
    if target_delay == Microseconds::ZERO {
        "setpts=PTS-STARTPTS".to_owned()
    } else {
        format!(
            "setpts=PTS-STARTPTS+{}/TB",
            format_seconds(target_delay)
        )
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
        SegmentFitMode::Stretch => (
            vec![format!("scale={}:{}", output.width, output.height)],
            output,
        ),
        SegmentFitMode::Fit => {
            let fitted = fit_dimensions(source, output);
            (
                vec![format!("scale={}:{}", fitted.width, fitted.height)],
                fitted,
            )
        }
        SegmentFitMode::Fill => {
            let filled = fill_dimensions(source, output);
            let x = ((i64::from(filled.width) - i64::from(output.width)) / 2).max(0);
            let y = ((i64::from(filled.height) - i64::from(output.height)) / 2).max(0);
            (
                vec![
                    format!("scale={}:{}", filled.width, filled.height),
                    format!("crop={}:{}:{x}:{y}", output.width, output.height),
                ],
                output,
            )
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

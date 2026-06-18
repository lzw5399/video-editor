use std::collections::BTreeMap;

use draft_model::{MaterialId, Microseconds, SourceTimerange, TargetTimerange};
use render_graph::{RenderCanvasBackgroundMode, RenderGraphPlan, RenderOutputProfile};
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
    let mut visual_labels = Vec::new();
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
        let label = format!("v{layer_index}");
        lines.push(format!(
            "[{input_index}:v]trim=start={start}:duration={duration},setpts=PTS-STARTPTS,scale={width}:{height}[{label}]",
            start = format_seconds(clip.start),
            duration = format_seconds(clip.duration),
            width = dimensions.width,
            height = dimensions.height
        ));
        visual_labels.push(label);
    }

    let mut current_video = if visual_labels.is_empty() {
        let background_color = canvas_background_color_arg(plan);
        lines.push(format!(
            "color=c={background_color}:s={width}x{height}:r={rate}:d={duration}[vbase0]",
            width = dimensions.width,
            height = dimensions.height,
            rate = frame_rate_arg(plan),
            duration = format_seconds(output_duration(plan))
        ));
        "vbase0".to_owned()
    } else if visual_labels.len() == 1 {
        lines.push(format!("[{}]null[vbase0]", visual_labels[0]));
        "vbase0".to_owned()
    } else {
        let mut current = visual_labels[0].clone();
        for (overlay_index, next) in visual_labels.iter().enumerate().skip(1) {
            let out = format!("vbase{overlay_index}");
            lines.push(format!(
                "[{current}][{next}]overlay=x=0:y=0:shortest=1[{out}]"
            ));
            current = out;
        }
        current
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

    let has_audio_output = !matches!(
        plan.output_profile,
        RenderOutputProfile::PreviewFrame { .. }
    ) && !plan.graph.audio_mixes.is_empty();
    if has_audio_output {
        let mut audio_labels = Vec::new();
        for (audio_index, audio) in plan.graph.audio_mixes.iter().enumerate() {
            let input_index = input_indexes
                .get(&audio.material_id)
                .ok_or_else(|| missing_input(&audio.material_id))?;
            let Some(clip) = clipped_source_timerange(
                &audio.source_timerange,
                &audio.target_timerange,
                output_timerange(plan),
            ) else {
                continue;
            };
            let label = format!("a{audio_index}");
            lines.push(format!(
                "[{input_index}:a]atrim=start={start}:duration={duration},asetpts=PTS-STARTPTS,volume={volume}[{label}]",
                start = format_seconds(clip.start),
                duration = format_seconds(clip.duration),
                volume = volume_arg(audio.volume_level_millis)
            ));
            audio_labels.push(label);
        }
        if audio_labels.len() == 1 {
            lines.push(format!("[{}]anull[aout]", audio_labels[0]));
        } else {
            let inputs = audio_labels
                .iter()
                .map(|label| format!("[{label}]"))
                .collect::<String>();
            lines.push(format!(
                "{inputs}amix=inputs={}:duration=longest:normalize=0[aout]",
                audio_labels.len()
            ));
        }
    }

    Ok(GeneratedFilterScript {
        path,
        contents: lines.join(";\n"),
        has_audio_output,
    })
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

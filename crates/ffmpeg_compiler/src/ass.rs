use std::collections::BTreeSet;

use draft_model::{Microseconds, TargetTimerange, TextAlignment, resolve_bundled_font};
use render_graph::{RenderGraph, RenderGraphPlan, RenderOutputProfile, RenderTextOverlay};
use serde::{Deserialize, Serialize};

use crate::job::{
    CompileContext, FfmpegCompileError, FfmpegCompileErrorKind, FfmpegSidecar, FfmpegSidecarKind,
    format_seconds, sanitize_id,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextRenderCapability {
    pub supports_ass_filter: bool,
    pub supports_subtitles_filter: bool,
    pub env_text_font_path: Option<String>,
    pub available_font_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundled_font_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundled_font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundled_font_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundled_font_license: Option<String>,
}

impl Default for TextRenderCapability {
    fn default() -> Self {
        Self {
            supports_ass_filter: true,
            supports_subtitles_filter: true,
            env_text_font_path: None,
            available_font_paths: Vec::new(),
            bundled_font_ref: None,
            bundled_font_family: None,
            bundled_font_path: None,
            bundled_font_license: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedTextFont {
    pub family: String,
    pub path: String,
    pub candidate: String,
}

pub fn generate_ass_sidecars(
    plan: &RenderGraphPlan,
    context: &CompileContext,
    job_id: &str,
) -> Result<Vec<FfmpegSidecar>, FfmpegCompileError> {
    let graph = &plan.graph;
    if graph.text_overlays.is_empty() {
        return Ok(Vec::new());
    }
    if !context.capabilities.text.supports_ass_filter
        || !context.capabilities.text.supports_subtitles_filter
    {
        return Err(FfmpegCompileError::new(
            FfmpegCompileErrorKind::MissingTextFilterSupport,
            "FFmpeg text rendering requires ASS/subtitles filter support",
            "Use an FFmpeg build with ASS and subtitles filter support enabled.",
        ));
    }

    graph
        .text_overlays
        .iter()
        .map(|overlay| {
            reject_unsupported_text_resources(overlay)?;
            let font = resolve_text_font(overlay, &context.capabilities.text)?;
            let segment_id = overlay.overlay.segment_id.as_str();
            let sidecar_id = format!("{job_id}-text-{}", sanitize_id(segment_id));
            let path = context.artifact_path(&format!("{sidecar_id}.ass"));
            Ok(FfmpegSidecar {
                sidecar_id,
                kind: FfmpegSidecarKind::AssSubtitle,
                segment_id: Some(overlay.overlay.segment_id.clone()),
                path,
                contents: ass_contents(graph, output_timerange(plan), overlay, &font),
            })
        })
        .collect()
}

pub fn resolve_text_font(
    overlay: &RenderTextOverlay,
    capability: &TextRenderCapability,
) -> Result<ResolvedTextFont, FfmpegCompileError> {
    let available = capability
        .available_font_paths
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    if let Some(font_ref) = &overlay.overlay.font_ref {
        if let Some(entry) = resolve_bundled_font(font_ref) {
            if let Some(path) = &capability.bundled_font_path {
                if capability.bundled_font_ref.as_deref() == Some(font_ref.as_str())
                    && available.contains(path)
                {
                    return Ok(ResolvedTextFont {
                        family: capability
                            .bundled_font_family
                            .clone()
                            .unwrap_or_else(|| entry.family.to_owned()),
                        path: path.clone(),
                        candidate: font_ref.clone(),
                    });
                }
            }

            return Err(FfmpegCompileError::new(
                FfmpegCompileErrorKind::MissingTextFont,
                format!(
                    "bundled text font {font_ref} is registered but unavailable to FFmpeg"
                ),
                "Restore the bundled font asset and runtime capability registry before compiling text overlays.",
            )
            .with_material_id(overlay.material_id.clone()));
        }
    }

    if let Some(env_path) = &capability.env_text_font_path {
        if overlay
            .overlay
            .fallback_candidates
            .iter()
            .any(|candidate| candidate == "VE_TEXT_FONT_PATH")
            && available.contains(env_path)
        {
            return Ok(ResolvedTextFont {
                family: overlay.overlay.font_family.clone(),
                path: env_path.clone(),
                candidate: "VE_TEXT_FONT_PATH".to_owned(),
            });
        }
    }

    for candidate in &overlay.overlay.fallback_candidates {
        if candidate == "VE_TEXT_FONT_PATH" {
            continue;
        }
        if available.contains(candidate) {
            return Ok(ResolvedTextFont {
                family: overlay.overlay.font_family.clone(),
                path: candidate.clone(),
                candidate: candidate.clone(),
            });
        }
    }

    Err(FfmpegCompileError::new(
        FfmpegCompileErrorKind::MissingTextFont,
        format!(
            "no deterministic text font resolved from candidates: {}",
            overlay.overlay.fallback_candidates.join(", ")
        ),
        "Set VE_TEXT_FONT_PATH or install one of the pinned fallback fonts before compiling text overlays.",
    )
    .with_material_id(overlay.material_id.clone()))
}

fn ass_contents(
    graph: &RenderGraph,
    output_timerange: &TargetTimerange,
    overlay: &RenderTextOverlay,
    font: &ResolvedTextFont,
) -> String {
    let style = &overlay.overlay.style;
    let stroke = style.stroke.as_ref();
    let shadow = style.shadow.as_ref();
    let background = style.background.as_ref();
    let alignment = ass_alignment(overlay.overlay.alignment);
    let outline_width = stroke.map(|value| value.width).unwrap_or(0);
    let border_style = if background.is_some() { 3 } else { 1 };
    let shadow_size = shadow
        .map(|value| {
            value
                .blur
                .max(value.offset_x.unsigned_abs())
                .max(value.offset_y.unsigned_abs())
        })
        .unwrap_or(0);
    let spacing = letter_spacing_pixels(
        overlay.overlay.font_size,
        overlay.overlay.letter_spacing_millis,
    );
    let (event_start, event_end) =
        clipped_event_timerange(output_timerange, &overlay.overlay.target_timerange);

    format!(
        concat!(
            "[Script Info]\n",
            "ScriptType: v4.00+\n",
            "WrapStyle: 2\n",
            "ScaledBorderAndShadow: yes\n",
            "PlayResX: {play_res_x}\n",
            "PlayResY: {play_res_y}\n",
            "; FontPath: {font_path}\n",
            "; TextBox: {text_box_width}x{text_box_height}\n",
            "; LayoutRegion: {layout_x},{layout_y} {layout_width}x{layout_height}\n",
            "; LineHeightMillis: {line_height_millis}\n\n",
            "[V4+ Styles]\n",
            "Format: Name, Fontname, Fontsize, PrimaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n",
            "Style: Default,{font_family},{font_size},{primary},{outline},{back},0,0,0,0,100,100,{spacing},0,{border_style},{outline_width},{shadow_size},{alignment},{margin_l},{margin_r},{margin_v},1\n\n",
            "[Events]\n",
            "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
            "Dialogue: {layer},{start},{end},Default,{name},{margin_l},{margin_r},{margin_v},,{text}\n"
        ),
        play_res_x = graph.canvas.width,
        play_res_y = graph.canvas.height,
        font_path = font.path,
        text_box_width = overlay.overlay.text_box.width,
        text_box_height = overlay.overlay.text_box.height,
        layout_x = overlay.overlay.layout_region.x,
        layout_y = overlay.overlay.layout_region.y,
        layout_width = overlay.overlay.layout_region.width,
        layout_height = overlay.overlay.layout_region.height,
        line_height_millis = overlay.overlay.line_height_millis,
        font_family = font.family,
        font_size = overlay.overlay.font_size,
        primary = ass_color(&style.color, "ffffff", 0x00),
        outline = ass_color(
            stroke
                .map(|value| value.color.as_str())
                .unwrap_or("#000000"),
            "000000",
            0x00
        ),
        back = ass_color(
            background
                .map(|value| value.color.as_str())
                .unwrap_or("#000000"),
            "000000",
            0x80
        ),
        spacing = spacing,
        border_style = border_style,
        outline_width = outline_width,
        shadow_size = shadow_size,
        alignment = alignment,
        margin_l = overlay.overlay.safe_area.left,
        margin_r = overlay.overlay.safe_area.right,
        margin_v = overlay.overlay.safe_area.bottom,
        layer = overlay.overlay.stack_index,
        start = ass_time(event_start),
        end = ass_time(event_end),
        name = overlay.overlay.segment_id.as_str(),
        text = escape_ass_text(&overlay.overlay.content)
    )
}

fn reject_unsupported_text_resources(
    overlay: &RenderTextOverlay,
) -> Result<(), FfmpegCompileError> {
    let mut unsupported = Vec::new();
    if let Some(font_ref) = &overlay.overlay.font_ref {
        if resolve_bundled_font(font_ref).is_none() {
            unsupported.push(format!("fontRef {font_ref}"));
        }
    }
    unsupported.extend(
        overlay
            .overlay
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.support == "unsupported")
            .map(|diagnostic| diagnostic.property.clone()),
    );

    if unsupported.is_empty() {
        return Ok(());
    }

    unsupported.sort();
    unsupported.dedup();
    Err(FfmpegCompileError::new(
        FfmpegCompileErrorKind::UnsupportedTextResource,
        format!(
            "text segment {} contains unsupported text resources: {}",
            overlay.overlay.segment_id.as_str(),
            unsupported.join(", ")
        ),
        "Remove or replace unsupported text bubble, effect, or fontRef resources before compiling ASS sidecars.",
    )
    .with_material_id(overlay.material_id.clone()))
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

fn clipped_event_timerange(
    output_timerange: &TargetTimerange,
    text_timerange: &TargetTimerange,
) -> (Microseconds, Microseconds) {
    let output_start = output_timerange.start.get();
    let output_end = output_start.saturating_add(output_timerange.duration.get());
    let text_start = text_timerange.start.get();
    let text_end = text_start.saturating_add(text_timerange.duration.get());
    let start = text_start.max(output_start).saturating_sub(output_start);
    let end = text_end.min(output_end).saturating_sub(output_start);
    (Microseconds::new(start), Microseconds::new(end.max(start)))
}

fn ass_time(value: Microseconds) -> String {
    let seconds = format_seconds(value);
    let (whole, micros) = seconds
        .split_once('.')
        .expect("format_seconds always includes decimal point");
    let whole = whole.parse::<u64>().expect("integer seconds");
    let hours = whole / 3_600;
    let minutes = (whole % 3_600) / 60;
    let secs = whole % 60;
    let millis = &micros[..3];
    format!("{hours}:{minutes:02}:{secs:02}.{millis}")
}

fn ass_alignment(alignment: TextAlignment) -> u8 {
    match alignment {
        TextAlignment::Left => 1,
        TextAlignment::Center => 2,
        TextAlignment::Right => 3,
    }
}

fn letter_spacing_pixels(font_size: u32, letter_spacing_millis: u32) -> u32 {
    ((u64::from(font_size) * u64::from(letter_spacing_millis)) / 1_000) as u32
}

fn ass_color(value: &str, fallback: &str, alpha: u8) -> String {
    let hex = value.strip_prefix('#').unwrap_or(value);
    let normalized = if hex.len() == 6 && hex.chars().all(|character| character.is_ascii_hexdigit())
    {
        hex.to_owned()
    } else {
        fallback.to_owned()
    };
    let red = &normalized[0..2];
    let green = &normalized[2..4];
    let blue = &normalized[4..6];
    format!("&H{alpha:02X}{blue}{green}{red}").to_uppercase()
}

fn escape_ass_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('{', "\\\\{")
        .replace('}', "\\\\}")
        .replace('\n', "\\\\N")
}

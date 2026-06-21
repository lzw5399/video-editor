use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::fs;

use draft_model::{
    TextAlignment, bundled_text_font_path, repository_root_from_manifest, resolve_bundled_font,
    validate_bundled_font_registry,
};
use fontdue::{Font, FontSettings, Metrics};
use render_graph::{RenderGraph, RenderTextOverlay, graph::RenderSampledTextOverlay};
use serde::{Deserialize, Serialize};

use crate::{
    RealtimePreviewCapabilityClassifier, RealtimePreviewDiagnostic,
    RealtimePreviewDiagnosticDomain, RealtimePreviewFallbackReason, RealtimePreviewGraphSupport,
    RealtimePreviewSupport,
};

pub const TEXT_PARITY_UNPROVEN_REASON: &str = "gpu text parity has not been proven with repository fonts; realtime preview must use fallback text rasterization";

thread_local! {
    static BUNDLED_TEXT_FONT: RefCell<Option<Result<Font, TextRasterizationError>>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextPreviewOutcome {
    pub support: RealtimePreviewGraphSupport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<RealtimePreviewFallbackReason>,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RasterizedTextLayer {
    pub width: u32,
    pub height: u32,
    pub stride_bytes: u32,
    pub x: i64,
    pub y: i64,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TextRasterizationError {
    MissingFontRef {
        segment_id: String,
    },
    UnregisteredFontRef {
        segment_id: String,
        font_ref: String,
    },
    RegistryValidationFailed(String),
    FontReadFailed(String),
    FontParseFailed(String),
    MissingGlyph {
        segment_id: String,
        character: char,
    },
    InvalidColor(String),
    EmptyTexture {
        segment_id: String,
    },
    TextureOverflow,
}

impl fmt::Display for TextRasterizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingFontRef { segment_id } => write!(
                formatter,
                "text segment {segment_id} requires a bundled fontRef for realtime GPU text"
            ),
            Self::UnregisteredFontRef {
                segment_id,
                font_ref,
            } => write!(
                formatter,
                "text segment {segment_id} fontRef {font_ref} is not registered for realtime GPU text"
            ),
            Self::RegistryValidationFailed(error) => {
                write!(
                    formatter,
                    "bundled realtime font registry validation failed: {error}"
                )
            }
            Self::FontReadFailed(error) => {
                write!(formatter, "bundled text font read failed: {error}")
            }
            Self::FontParseFailed(error) => {
                write!(formatter, "bundled text font parse failed: {error}")
            }
            Self::MissingGlyph {
                segment_id,
                character,
            } => write!(
                formatter,
                "text segment {segment_id} bundled font lacks glyph for {character}"
            ),
            Self::InvalidColor(color) => write!(formatter, "invalid text color {color}"),
            Self::EmptyTexture { segment_id } => {
                write!(
                    formatter,
                    "text segment {segment_id} produced an empty text texture"
                )
            }
            Self::TextureOverflow => formatter.write_str("text texture size overflowed"),
        }
    }
}

impl Error for TextRasterizationError {}

pub fn classify_text_preview_outcome(
    graph: &RenderGraph,
    classifier: &RealtimePreviewCapabilityClassifier,
) -> TextPreviewOutcome {
    let diagnostics = graph
        .text_overlays
        .iter()
        .map(|text| {
            text_preview_diagnostic(
                text,
                classifier.gpu_text_parity,
                classifier.bundled_text_font_registry_available,
            )
        })
        .collect::<Vec<_>>();
    let support = summarize_text_support(&diagnostics);
    let fallback_reason = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.fallback_used)
        .then_some(RealtimePreviewFallbackReason::TextParityUnsupported);

    TextPreviewOutcome {
        support,
        fallback_reason,
        diagnostics,
    }
}

pub(crate) fn text_preview_diagnostic(
    text: &render_graph::RenderTextOverlay,
    gpu_text_parity: bool,
    bundled_text_font_registry_available: bool,
) -> RealtimePreviewDiagnostic {
    if let Some(unsupported) = text
        .overlay
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.support == "unsupported")
    {
        RealtimePreviewDiagnostic::new(
            Some(text.overlay.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Text,
            RealtimePreviewSupport::Unsupported {
                reason: unsupported.reason.clone(),
            },
            unsupported.reason.clone(),
            Some(RealtimePreviewFallbackReason::UnsupportedGraphIntent),
            true,
        )
    } else if let Some(font_ref) = text.overlay.font_ref.as_deref() {
        if resolve_bundled_font(font_ref).is_none() {
            RealtimePreviewDiagnostic::new(
                Some(text.overlay.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::Text,
                RealtimePreviewSupport::Unsupported {
                    reason: format!("text fontRef {font_ref} is not available in realtime preview"),
                },
                format!("text fontRef {font_ref} is not available in realtime preview"),
                Some(RealtimePreviewFallbackReason::TextParityUnsupported),
                true,
            )
        } else if bundled_text_font_registry_available {
            if let Err(error) = validate_bundled_font_registry(&repository_root_from_manifest()) {
                return RealtimePreviewDiagnostic::new(
                    Some(text.overlay.segment_id.as_str().to_owned()),
                    RealtimePreviewDiagnosticDomain::Text,
                    RealtimePreviewSupport::Unsupported {
                        reason: error.to_string(),
                    },
                    error.to_string(),
                    Some(RealtimePreviewFallbackReason::TextParityUnsupported),
                    true,
                );
            }
            RealtimePreviewDiagnostic::new(
                Some(text.overlay.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::Text,
                RealtimePreviewSupport::Supported,
                format!(
                    "text fontRef {font_ref} is resolved through bundled realtime font registry"
                ),
                None,
                false,
            )
        } else if gpu_text_parity {
            RealtimePreviewDiagnostic::new(
                Some(text.overlay.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::Text,
                RealtimePreviewSupport::Supported,
                "text intent is realtime supported by proven GPU text parity",
                None,
                false,
            )
        } else {
            RealtimePreviewDiagnostic::new(
                Some(text.overlay.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::Text,
                RealtimePreviewSupport::Unsupported {
                    reason: TEXT_PARITY_UNPROVEN_REASON.to_owned(),
                },
                TEXT_PARITY_UNPROVEN_REASON,
                Some(RealtimePreviewFallbackReason::TextParityUnsupported),
                true,
            )
        }
    } else if gpu_text_parity {
        RealtimePreviewDiagnostic::new(
            Some(text.overlay.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Text,
            RealtimePreviewSupport::Supported,
            "text intent is realtime supported by proven GPU text parity",
            None,
            false,
        )
    } else {
        RealtimePreviewDiagnostic::new(
            Some(text.overlay.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Text,
            RealtimePreviewSupport::Unsupported {
                reason: TEXT_PARITY_UNPROVEN_REASON.to_owned(),
            },
            TEXT_PARITY_UNPROVEN_REASON,
            Some(RealtimePreviewFallbackReason::TextParityUnsupported),
            true,
        )
    }
}

fn summarize_text_support(
    diagnostics: &[RealtimePreviewDiagnostic],
) -> RealtimePreviewGraphSupport {
    if diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.support,
            RealtimePreviewSupport::Unsupported { .. }
        )
    }) {
        RealtimePreviewGraphSupport::Unsupported
    } else if diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.support, RealtimePreviewSupport::Degraded { .. }))
    {
        RealtimePreviewGraphSupport::Degraded
    } else {
        RealtimePreviewGraphSupport::Supported
    }
}

pub(crate) fn rasterize_text_overlay(
    text: &RenderTextOverlay,
    sampled: Option<&RenderSampledTextOverlay>,
    canvas_width: u32,
    canvas_height: u32,
) -> Result<RasterizedTextLayer, TextRasterizationError> {
    let segment_id = text.overlay.segment_id.as_str().to_owned();
    let font_ref =
        text.overlay
            .font_ref
            .as_deref()
            .ok_or_else(|| TextRasterizationError::MissingFontRef {
                segment_id: segment_id.clone(),
            })?;
    resolve_bundled_font(font_ref).ok_or_else(|| TextRasterizationError::UnregisteredFontRef {
        segment_id: segment_id.clone(),
        font_ref: font_ref.to_owned(),
    })?;
    with_bundled_text_font(|font| {
        let font_size = sampled
            .map(|sample| sample.font_size)
            .unwrap_or(text.overlay.font_size)
            .max(1) as f32;
        let color = parse_text_color(
            sampled
                .map(|sample| sample.color.as_str())
                .unwrap_or(text.overlay.style.color.as_str()),
        )?;
        let line_height_millis = sampled
            .map(|sample| sample.line_height_millis)
            .unwrap_or(text.overlay.line_height_millis)
            .max(1);
        let letter_spacing = font_size
            * sampled
                .map(|sample| sample.letter_spacing_millis)
                .unwrap_or(text.overlay.letter_spacing_millis) as f32
            / 1_000.0;
        let line_metrics = font.horizontal_line_metrics(font_size);
        let ascent = line_metrics
            .map(|metrics| metrics.ascent)
            .unwrap_or(font_size)
            .ceil() as i32;
        let descent = line_metrics
            .map(|metrics| metrics.descent)
            .unwrap_or(-(font_size * 0.25))
            .floor() as i32;
        let line_height = ((font_size * line_height_millis as f32) / 1_000.0)
            .ceil()
            .max((ascent - descent).max(1) as f32) as i32;
        let line_count = text.overlay.content.lines().count().max(1) as u32;
        let min_text_height = u32::try_from(line_height.max(1))
            .ok()
            .and_then(|height| height.checked_mul(line_count))
            .ok_or(TextRasterizationError::TextureOverflow)?;
        let layer_width = text
            .overlay
            .layout_width
            .max(text.overlay.text_box.width)
            .min(canvas_width)
            .max(1);
        let layer_height = text
            .overlay
            .layout_height
            .max(font_size.ceil() as u32)
            .max(min_text_height)
            .min(canvas_height)
            .max(1);
        let len = layer_width
            .checked_mul(layer_height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|bytes| usize::try_from(bytes).ok())
            .ok_or(TextRasterizationError::TextureOverflow)?;
        let mut pixels = vec![0_u8; len];

        for (line_index, line) in text.overlay.content.lines().enumerate() {
            let baseline_y = ascent + line_index as i32 * line_height;
            if baseline_y >= layer_height as i32 {
                break;
            }
            let line_width = measure_line_width(font, line, font_size, letter_spacing);
            let mut cursor_x = aligned_line_x(text.overlay.alignment, layer_width, line_width);
            let mut previous = None;
            for character in line.chars() {
                if character.is_control() {
                    continue;
                }
                if !character.is_whitespace() && !font.has_glyph(character) {
                    return Err(TextRasterizationError::MissingGlyph {
                        segment_id: segment_id.clone(),
                        character,
                    });
                }
                if let Some(previous_character) = previous {
                    cursor_x += font
                        .horizontal_kern(previous_character, character, font_size)
                        .unwrap_or(0.0);
                }
                let (metrics, bitmap) = font.rasterize(character, font_size);
                blend_glyph(
                    &mut pixels,
                    layer_width,
                    layer_height,
                    color,
                    cursor_x,
                    baseline_y,
                    &metrics,
                    &bitmap,
                );
                cursor_x += metrics.advance_width + letter_spacing;
                previous = Some(character);
            }
        }

        if pixels.chunks_exact(4).all(|pixel| pixel[3] == 0) {
            return Err(TextRasterizationError::EmptyTexture { segment_id });
        }

        Ok(RasterizedTextLayer {
            width: layer_width,
            height: layer_height,
            stride_bytes: layer_width * 4,
            x: text.overlay.layout_region.x as i64,
            y: text.overlay.layout_region.y as i64,
            pixels,
        })
    })
}

fn with_bundled_text_font<T>(
    action: impl FnOnce(&Font) -> Result<T, TextRasterizationError>,
) -> Result<T, TextRasterizationError> {
    BUNDLED_TEXT_FONT.with(|cached| {
        let mut cached = cached.borrow_mut();
        if cached.is_none() {
            *cached = Some(load_bundled_text_font());
        }
        match cached
            .as_ref()
            .expect("bundled text font cache initialized")
        {
            Ok(font) => action(font),
            Err(error) => Err(error.clone()),
        }
    })
}

fn load_bundled_text_font() -> Result<Font, TextRasterizationError> {
    validate_bundled_font_registry(&repository_root_from_manifest())
        .map_err(|error| TextRasterizationError::RegistryValidationFailed(error.to_string()))?;
    let font_bytes = fs::read(bundled_text_font_path())
        .map_err(|error| TextRasterizationError::FontReadFailed(error.to_string()))?;
    Font::from_bytes(font_bytes, FontSettings::default())
        .map_err(|error| TextRasterizationError::FontParseFailed(error.to_owned()))
}

fn measure_line_width(font: &Font, line: &str, font_size: f32, letter_spacing: f32) -> f32 {
    let mut width = 0.0;
    let mut previous = None;
    let mut glyph_count = 0_u32;
    for character in line.chars() {
        if character.is_control() {
            continue;
        }
        if let Some(previous_character) = previous {
            width += font
                .horizontal_kern(previous_character, character, font_size)
                .unwrap_or(0.0);
        }
        width += font.metrics(character, font_size).advance_width;
        previous = Some(character);
        glyph_count += 1;
    }
    if glyph_count > 1 {
        width += letter_spacing * (glyph_count - 1) as f32;
    }
    width.max(0.0)
}

fn aligned_line_x(alignment: TextAlignment, layer_width: u32, line_width: f32) -> f32 {
    match alignment {
        TextAlignment::Left => 0.0,
        TextAlignment::Center => ((layer_width as f32 - line_width) / 2.0).max(0.0),
        TextAlignment::Right => (layer_width as f32 - line_width).max(0.0),
    }
}

fn blend_glyph(
    pixels: &mut [u8],
    layer_width: u32,
    layer_height: u32,
    color: [u8; 4],
    cursor_x: f32,
    baseline_y: i32,
    metrics: &Metrics,
    bitmap: &[u8],
) {
    let glyph_x = cursor_x.round() as i32 + metrics.xmin;
    let glyph_y = baseline_y - metrics.height as i32 - metrics.ymin;
    for bitmap_y in 0..metrics.height {
        let y = glyph_y + bitmap_y as i32;
        if y < 0 || y >= layer_height as i32 {
            continue;
        }
        for bitmap_x in 0..metrics.width {
            let x = glyph_x + bitmap_x as i32;
            if x < 0 || x >= layer_width as i32 {
                continue;
            }
            let source_alpha = bitmap[bitmap_y * metrics.width + bitmap_x];
            if source_alpha == 0 {
                continue;
            }
            let dest_index = ((y as u32 * layer_width + x as u32) * 4) as usize;
            let alpha = ((u32::from(source_alpha) * u32::from(color[3])) + 127) / 255;
            let inverse_alpha = 255_u32.saturating_sub(alpha);
            for channel in 0..3 {
                pixels[dest_index + channel] = (((u32::from(color[channel]) * alpha)
                    + (u32::from(pixels[dest_index + channel]) * inverse_alpha)
                    + 127)
                    / 255) as u8;
            }
            pixels[dest_index + 3] = (alpha
                + ((u32::from(pixels[dest_index + 3]) * inverse_alpha + 127) / 255))
                .min(255) as u8;
        }
    }
}

fn parse_text_color(color: &str) -> Result<[u8; 4], TextRasterizationError> {
    let trimmed = color.strip_prefix('#').unwrap_or(color);
    if !trimmed.is_ascii() {
        return Err(TextRasterizationError::InvalidColor(color.to_owned()));
    }
    let (rgb, alpha) = match trimmed.len() {
        6 => (trimmed, 255),
        8 => (&trimmed[0..6], parse_hex_byte(&trimmed[6..8], color)?),
        _ => return Err(TextRasterizationError::InvalidColor(color.to_owned())),
    };
    Ok([
        parse_hex_byte(&rgb[0..2], color)?,
        parse_hex_byte(&rgb[2..4], color)?,
        parse_hex_byte(&rgb[4..6], color)?,
        alpha,
    ])
}

fn parse_hex_byte(value: &str, original: &str) -> Result<u8, TextRasterizationError> {
    u8::from_str_radix(value, 16)
        .map_err(|_| TextRasterizationError::InvalidColor(original.to_owned()))
}

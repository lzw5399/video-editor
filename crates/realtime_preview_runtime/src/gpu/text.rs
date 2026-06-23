use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
#[cfg(not(target_os = "macos"))]
use std::fs;
#[cfg(target_os = "macos")]
use std::path::PathBuf;

use draft_model::{
    TextAlignment, bundled_font_path, repository_root_from_manifest, resolve_bundled_font,
    validate_bundled_font_registry,
};
#[cfg(not(target_os = "macos"))]
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
    #[cfg(not(target_os = "macos"))]
    static BUNDLED_TEXT_FONTS: RefCell<HashMap<String, Result<Font, TextRasterizationError>>> = RefCell::new(HashMap::new());
    #[cfg(target_os = "macos")]
    static REGISTERED_CORE_TEXT_FONTS: RefCell<HashMap<String, Result<String, TextRasterizationError>>> = RefCell::new(HashMap::new());
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
    pub logical_width: u32,
    pub logical_height: u32,
    pub stride_bytes: u32,
    pub x: i64,
    pub y: i64,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextRasterizationTarget {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub canvas_to_target_scale: f32,
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
    #[cfg(not(target_os = "macos"))]
    FontReadFailed(String),
    #[cfg(not(target_os = "macos"))]
    FontParseFailed(String),
    #[cfg(not(target_os = "macos"))]
    MissingGlyph {
        segment_id: String,
        character: char,
    },
    InvalidColor(String),
    EmptyTexture {
        segment_id: String,
    },
    CoreTextRasterizationFailed(String),
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
            #[cfg(not(target_os = "macos"))]
            Self::FontReadFailed(error) => {
                write!(formatter, "bundled text font read failed: {error}")
            }
            #[cfg(not(target_os = "macos"))]
            Self::FontParseFailed(error) => {
                write!(formatter, "bundled text font parse failed: {error}")
            }
            #[cfg(not(target_os = "macos"))]
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
            Self::CoreTextRasterizationFailed(error) => {
                write!(formatter, "CoreText text rasterization failed: {error}")
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
    target: TextRasterizationTarget,
    raster_scale: u32,
) -> Result<RasterizedTextLayer, TextRasterizationError> {
    #[cfg(target_os = "macos")]
    {
        return rasterize_text_overlay_core_text(text, sampled, target, raster_scale);
    }

    #[cfg(not(target_os = "macos"))]
    {
        rasterize_text_overlay_fontdue(text, sampled, target, raster_scale)
    }
}

#[allow(unreachable_code)]
#[cfg(not(target_os = "macos"))]
fn rasterize_text_overlay_fontdue(
    text: &RenderTextOverlay,
    sampled: Option<&RenderSampledTextOverlay>,
    target: TextRasterizationTarget,
    raster_scale: u32,
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
    with_bundled_text_font(font_ref, |font| {
        let raster_scale = raster_scale.clamp(1, 4);
        let font_size = sampled
            .map(|sample| sample.font_size)
            .unwrap_or(text.overlay.font_size)
            .max(1) as f32
            * target.canvas_to_target_scale.max(0.001)
            * raster_scale as f32;
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
        let logical_width = target.width.max(1);
        let logical_height = target
            .height
            .max((font_size / raster_scale as f32).ceil() as u32)
            .max(min_text_height.div_ceil(raster_scale))
            .max(1);
        let layer_width = logical_width
            .checked_mul(raster_scale)
            .ok_or(TextRasterizationError::TextureOverflow)?;
        let layer_height = logical_height
            .checked_mul(raster_scale)
            .ok_or(TextRasterizationError::TextureOverflow)?;
        let len = layer_width
            .checked_mul(layer_height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|bytes| usize::try_from(bytes).ok())
            .ok_or(TextRasterizationError::TextureOverflow)?;
        let mut pixels = vec![0_u8; len];
        for pixel in pixels.chunks_exact_mut(4) {
            pixel[0] = color[0];
            pixel[1] = color[1];
            pixel[2] = color[2];
        }

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
                blend_glyph_alpha(
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
            logical_width,
            logical_height,
            stride_bytes: layer_width * 4,
            x: i64::from(target.x),
            y: i64::from(target.y),
            pixels,
        })
    })
}

#[cfg(target_os = "macos")]
fn rasterize_text_overlay_core_text(
    text: &RenderTextOverlay,
    sampled: Option<&RenderSampledTextOverlay>,
    target: TextRasterizationTarget,
    raster_scale: u32,
) -> Result<RasterizedTextLayer, TextRasterizationError> {
    use std::ffi::c_void;
    use std::ptr;

    use objc2_core_foundation::{
        CFAttributedString, CFDictionary, CFNumber, CFString, CFType, CGPoint, CGRect, CGSize,
    };
    use objc2_core_graphics::{
        CGAffineTransformMake, CGBitmapContextCreate, CGColor, CGColorSpace, CGContext,
        CGImageAlphaInfo, CGImageByteOrderInfo, CGTextDrawingMode,
    };
    use objc2_core_text::{
        CTFont, CTLine, kCTFontAttributeName, kCTForegroundColorAttributeName, kCTKernAttributeName,
    };

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
    let font_name = registered_core_text_font_name(font_ref)?;

    let raster_scale = raster_scale.clamp(1, 4);
    let target_scale = target.canvas_to_target_scale.max(0.001);
    let font_size = sampled
        .map(|sample| sample.font_size)
        .unwrap_or(text.overlay.font_size)
        .max(1) as f32
        * target_scale
        * raster_scale as f32;
    let color = parse_text_color(
        sampled
            .map(|sample| sample.color.as_str())
            .unwrap_or(text.overlay.style.color.as_str()),
    )?;
    let stroke = text
        .overlay
        .style
        .stroke
        .as_ref()
        .map(|stroke| {
            Ok(CoreTextStroke {
                color: parse_text_color(&stroke.color)?,
                width: (stroke.width as f64 * f64::from(target_scale) * f64::from(raster_scale))
                    .max(0.0),
                logical_width: (stroke.width as f64 * f64::from(target_scale)).max(0.0),
            })
        })
        .transpose()?;
    let shadow = text
        .overlay
        .style
        .shadow
        .as_ref()
        .map(|shadow| {
            Ok(CoreTextShadow {
                color: parse_text_color(&shadow.color)?,
                offset_x: f64::from(shadow.offset_x)
                    * f64::from(target_scale)
                    * f64::from(raster_scale),
                offset_y: f64::from(shadow.offset_y)
                    * f64::from(target_scale)
                    * f64::from(raster_scale),
                blur: f64::from(shadow.blur) * f64::from(target_scale) * f64::from(raster_scale),
                logical_offset_x: f64::from(shadow.offset_x) * f64::from(target_scale),
                logical_offset_y: f64::from(shadow.offset_y) * f64::from(target_scale),
                logical_blur: f64::from(shadow.blur) * f64::from(target_scale),
            })
        })
        .transpose()?;
    let background = text
        .overlay
        .style
        .background
        .as_ref()
        .map(|background| parse_text_color(&background.color))
        .transpose()?;
    let padding = core_text_effect_padding(stroke.as_ref(), shadow.as_ref());
    let layer_x = target.x.saturating_sub(padding.left);
    let layer_y = target.y.saturating_sub(padding.top);
    let available_left = target.x.saturating_sub(layer_x);
    let available_top = target.y.saturating_sub(layer_y);
    let target_width_pixels = target
        .width
        .max(1)
        .checked_mul(raster_scale)
        .ok_or(TextRasterizationError::TextureOverflow)?;
    let draw_origin_x = f64::from(
        available_left
            .checked_mul(raster_scale)
            .ok_or(TextRasterizationError::TextureOverflow)?,
    );
    let draw_origin_y = f64::from(
        available_top
            .checked_mul(raster_scale)
            .ok_or(TextRasterizationError::TextureOverflow)?,
    );
    let line_height_millis = sampled
        .map(|sample| sample.line_height_millis)
        .unwrap_or(text.overlay.line_height_millis)
        .max(1);
    let letter_spacing = font_size
        * sampled
            .map(|sample| sample.letter_spacing_millis)
            .unwrap_or(text.overlay.letter_spacing_millis) as f32
        / 1_000.0;

    let logical_width = target
        .width
        .max(1)
        .saturating_add(available_left)
        .saturating_add(padding.right)
        .max(1);
    let layer_width = logical_width
        .checked_mul(raster_scale)
        .ok_or(TextRasterizationError::TextureOverflow)?;
    let font_name = CFString::from_str(&font_name);
    let font = unsafe { CTFont::with_name(&font_name, font_size as f64, ptr::null()) };
    let ascent = unsafe { font.ascent() }.ceil().max(1.0) as i32;
    let descent = unsafe { font.descent() }.ceil().max(0.0) as i32;
    let leading = unsafe { font.leading() }.ceil().max(0.0) as i32;
    let natural_line_height = (ascent + descent + leading).max(1);
    let line_height = ((font_size * line_height_millis as f32) / 1_000.0)
        .ceil()
        .max(natural_line_height as f32) as i32;
    let line_count = text.overlay.content.lines().count().max(1) as u32;
    let min_text_height = u32::try_from(line_height.max(1))
        .ok()
        .and_then(|height| height.checked_mul(line_count))
        .ok_or(TextRasterizationError::TextureOverflow)?;
    let base_logical_height = target
        .height
        .max(min_text_height.div_ceil(raster_scale))
        .max(1);
    let logical_height = base_logical_height
        .saturating_add(available_top)
        .saturating_add(padding.bottom)
        .max(1);
    let base_logical_height_pixels = base_logical_height
        .checked_mul(raster_scale)
        .ok_or(TextRasterizationError::TextureOverflow)?;
    let layer_height = logical_height
        .checked_mul(raster_scale)
        .ok_or(TextRasterizationError::TextureOverflow)?;
    let len = layer_width
        .checked_mul(layer_height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or(TextRasterizationError::TextureOverflow)?;
    let mut pixels = vec![0_u8; len];
    let color_space = CGColorSpace::new_device_rgb().ok_or_else(|| {
        TextRasterizationError::CoreTextRasterizationFailed(
            "could not create device RGB color space".to_owned(),
        )
    })?;
    let bitmap_info = CGImageAlphaInfo::PremultipliedLast.0 | CGImageByteOrderInfo::Order32Big.0;
    let context = unsafe {
        CGBitmapContextCreate(
            pixels.as_mut_ptr().cast::<c_void>(),
            layer_width as usize,
            layer_height as usize,
            8,
            (layer_width * 4) as usize,
            Some(&color_space),
            bitmap_info,
        )
    }
    .ok_or_else(|| {
        TextRasterizationError::CoreTextRasterizationFailed(
            "could not create bitmap context".to_owned(),
        )
    })?;
    CGContext::set_should_antialias(Some(&context), true);
    CGContext::set_allows_antialiasing(Some(&context), true);
    CGContext::set_should_smooth_fonts(Some(&context), true);
    CGContext::set_allows_font_smoothing(Some(&context), true);
    CGContext::set_should_subpixel_position_fonts(Some(&context), true);
    CGContext::set_allows_font_subpixel_positioning(Some(&context), true);
    CGContext::set_should_subpixel_quantize_fonts(Some(&context), true);
    CGContext::set_allows_font_subpixel_quantization(Some(&context), true);
    CGContext::set_text_matrix(
        Some(&context),
        CGAffineTransformMake(1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
    );

    let foreground = CGColor::new_srgb(
        f64::from(color[0]) / 255.0,
        f64::from(color[1]) / 255.0,
        f64::from(color[2]) / 255.0,
        f64::from(color[3]) / 255.0,
    );
    let stroke_color = stroke.as_ref().map(|stroke| cg_color(stroke.color));
    let shadow_color = shadow.as_ref().map(|shadow| cg_color(shadow.color));
    if let Some(background_color) = background {
        let background_color = cg_color(background_color);
        CGContext::set_fill_color_with_color(Some(&context), Some(&background_color));
        CGContext::fill_rect(
            Some(&context),
            CGRect::new(
                CGPoint::new(
                    draw_origin_x,
                    f64::from(layer_height) - draw_origin_y - f64::from(base_logical_height_pixels),
                ),
                CGSize::new(
                    f64::from(target_width_pixels),
                    f64::from(base_logical_height_pixels),
                ),
            ),
        );
    }
    CGContext::set_fill_color_with_color(Some(&context), Some(&foreground));
    let kern = CFNumber::new_cgfloat(letter_spacing as f64);
    let keys = [
        unsafe { kCTFontAttributeName },
        unsafe { kCTForegroundColorAttributeName },
        unsafe { kCTKernAttributeName },
    ];
    let values: [&CFType; 3] = [
        (&*font).as_ref(),
        (&*foreground).as_ref(),
        (&*kern).as_ref(),
    ];
    let attributes = CFDictionary::<CFString, CFType>::from_slices(&keys, &values);

    let draw_lines = || -> Result<(), TextRasterizationError> {
        for (line_index, line) in text.overlay.content.lines().enumerate() {
            let baseline_from_top = ascent + line_index as i32 * line_height;
            if baseline_from_top >= layer_height as i32 {
                break;
            }
            let line_string = CFString::from_str(line);
            let attributed = unsafe {
                CFAttributedString::new(None, Some(&line_string), Some(attributes.as_opaque()))
                    .ok_or_else(|| {
                        TextRasterizationError::CoreTextRasterizationFailed(
                            "could not create attributed string".to_owned(),
                        )
                    })?
            };
            let line = unsafe { CTLine::with_attributed_string(&attributed) };
            let mut line_ascent = 0.0;
            let mut line_descent = 0.0;
            let mut line_leading = 0.0;
            let line_width = unsafe {
                line.typographic_bounds(&mut line_ascent, &mut line_descent, &mut line_leading)
            };
            let flush = match text.overlay.alignment {
                TextAlignment::Left => 0.0,
                TextAlignment::Center => 0.5,
                TextAlignment::Right => 1.0,
            };
            let x = unsafe { line.pen_offset_for_flush(flush, f64::from(target_width_pixels)) }
                .max(0.0)
                .min((f64::from(target_width_pixels) - line_width.max(0.0)).max(0.0))
                + draw_origin_x;
            let y = f64::from(layer_height) - draw_origin_y - f64::from(baseline_from_top);
            CGContext::set_text_position(Some(&context), x, y);
            unsafe {
                line.draw(&context);
            }
        }
        Ok(())
    };

    if let Some(stroke) = stroke.as_ref() {
        if let Some(stroke_color) = stroke_color.as_ref() {
            CGContext::set_stroke_color_with_color(Some(&context), Some(stroke_color));
            CGContext::set_line_width(Some(&context), stroke.width.max(1.0));
            CGContext::set_text_drawing_mode(Some(&context), CGTextDrawingMode::Stroke);
            if let Some(shadow) = shadow.as_ref() {
                CGContext::set_shadow_with_color(
                    Some(&context),
                    CGSize::new(shadow.offset_x, -shadow.offset_y),
                    shadow.blur.max(0.0),
                    shadow_color.as_ref().map(|color| &**color),
                );
            }
            draw_lines()?;
            CGContext::set_shadow_with_color(Some(&context), CGSize::ZERO, 0.0, None);
        }
    }
    CGContext::set_fill_color_with_color(Some(&context), Some(&foreground));
    CGContext::set_text_drawing_mode(Some(&context), CGTextDrawingMode::Fill);
    if stroke.is_none() {
        if let Some(shadow) = shadow.as_ref() {
            CGContext::set_shadow_with_color(
                Some(&context),
                CGSize::new(shadow.offset_x, -shadow.offset_y),
                shadow.blur.max(0.0),
                shadow_color.as_ref().map(|color| &**color),
            );
        }
    }
    draw_lines()?;

    unpremultiply_rgba_in_place(&mut pixels);
    flip_rgba_rows_in_place(&mut pixels, layer_width as usize, layer_height as usize);

    if pixels.chunks_exact(4).all(|pixel| pixel[3] == 0) {
        return Err(TextRasterizationError::EmptyTexture { segment_id });
    }

    Ok(RasterizedTextLayer {
        width: layer_width,
        height: layer_height,
        logical_width,
        logical_height,
        stride_bytes: layer_width * 4,
        x: i64::from(layer_x),
        y: i64::from(layer_y),
        pixels,
    })
}

#[cfg(target_os = "macos")]
struct CoreTextStroke {
    color: [u8; 4],
    width: f64,
    logical_width: f64,
}

#[cfg(target_os = "macos")]
struct CoreTextShadow {
    color: [u8; 4],
    offset_x: f64,
    offset_y: f64,
    blur: f64,
    logical_offset_x: f64,
    logical_offset_y: f64,
    logical_blur: f64,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy)]
struct CoreTextEffectPadding {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

#[cfg(target_os = "macos")]
fn core_text_effect_padding(
    stroke: Option<&CoreTextStroke>,
    shadow: Option<&CoreTextShadow>,
) -> CoreTextEffectPadding {
    let stroke_pad = stroke
        .map(|stroke| ceil_logical_effect_padding(stroke.logical_width) + 1)
        .unwrap_or(0);
    let shadow_left = shadow
        .map(|shadow| {
            ceil_logical_effect_padding(
                stroke.map(|stroke| stroke.logical_width).unwrap_or(0.0)
                    + shadow.logical_blur
                    + (-shadow.logical_offset_x).max(0.0),
            ) + 1
        })
        .unwrap_or(0);
    let shadow_right = shadow
        .map(|shadow| {
            ceil_logical_effect_padding(
                stroke.map(|stroke| stroke.logical_width).unwrap_or(0.0)
                    + shadow.logical_blur
                    + shadow.logical_offset_x.max(0.0),
            ) + 1
        })
        .unwrap_or(0);
    let shadow_top = shadow
        .map(|shadow| {
            ceil_logical_effect_padding(
                stroke.map(|stroke| stroke.logical_width).unwrap_or(0.0)
                    + shadow.logical_blur
                    + (-shadow.logical_offset_y).max(0.0),
            ) + 1
        })
        .unwrap_or(0);
    let shadow_bottom = shadow
        .map(|shadow| {
            ceil_logical_effect_padding(
                stroke.map(|stroke| stroke.logical_width).unwrap_or(0.0)
                    + shadow.logical_blur
                    + shadow.logical_offset_y.max(0.0),
            ) + 1
        })
        .unwrap_or(0);

    CoreTextEffectPadding {
        left: stroke_pad.max(shadow_left),
        top: stroke_pad.max(shadow_top),
        right: stroke_pad.max(shadow_right),
        bottom: stroke_pad.max(shadow_bottom),
    }
}

#[cfg(target_os = "macos")]
fn ceil_logical_effect_padding(value: f64) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    value.ceil().min(f64::from(u32::MAX)) as u32
}

#[cfg(target_os = "macos")]
fn cg_color(color: [u8; 4]) -> objc2_core_foundation::CFRetained<objc2_core_graphics::CGColor> {
    objc2_core_graphics::CGColor::new_srgb(
        f64::from(color[0]) / 255.0,
        f64::from(color[1]) / 255.0,
        f64::from(color[2]) / 255.0,
        f64::from(color[3]) / 255.0,
    )
}

#[cfg(target_os = "macos")]
fn registered_core_text_font_name(font_ref: &str) -> Result<String, TextRasterizationError> {
    REGISTERED_CORE_TEXT_FONTS.with(|cached| {
        let mut cached = cached.borrow_mut();
        let font_name = cached
            .entry(font_ref.to_owned())
            .or_insert_with(|| register_core_text_font(font_ref));
        font_name.clone()
    })
}

#[cfg(target_os = "macos")]
fn register_core_text_font(font_ref: &str) -> Result<String, TextRasterizationError> {
    use objc2_core_foundation::CFURL;
    use objc2_core_text::{CTFontManagerRegisterFontsForURL, CTFontManagerScope};

    validate_bundled_font_registry(&repository_root_from_manifest())
        .map_err(|error| TextRasterizationError::RegistryValidationFailed(error.to_string()))?;
    let font_path =
        bundled_font_path(font_ref).ok_or_else(|| TextRasterizationError::UnregisteredFontRef {
            segment_id: "core-text-font-cache".to_owned(),
            font_ref: font_ref.to_owned(),
        })?;
    let font_url = CFURL::from_file_path(&font_path).ok_or_else(|| {
        TextRasterizationError::CoreTextRasterizationFailed(format!(
            "could not create font file URL for {}",
            font_path.display()
        ))
    })?;
    let registered = unsafe {
        CTFontManagerRegisterFontsForURL(
            &font_url,
            CTFontManagerScope::Process,
            std::ptr::null_mut(),
        )
    };
    if !registered {
        // CoreText returns false for already-registered fonts in some cases. Treat that as
        // non-fatal because process-scope registration is idempotent for this renderer.
        let _ = registered;
    }
    bundled_core_text_font_name(font_ref, font_path)
}

#[cfg(target_os = "macos")]
fn bundled_core_text_font_name(
    font_ref: &str,
    font_path: PathBuf,
) -> Result<String, TextRasterizationError> {
    let name = match font_path.file_stem().and_then(|name| name.to_str()) {
        Some("NotoSansCJKsc-Regular") => "Noto Sans CJK SC",
        Some("NotoSerifCJKsc-Regular") => "Noto Serif CJK SC",
        Some(name) => name,
        None => {
            return Err(TextRasterizationError::CoreTextRasterizationFailed(
                format!("could not derive CoreText font name for {font_ref}"),
            ));
        }
    };
    Ok(name.to_owned())
}

#[cfg(not(target_os = "macos"))]
fn with_bundled_text_font<T>(
    font_ref: &str,
    action: impl FnOnce(&Font) -> Result<T, TextRasterizationError>,
) -> Result<T, TextRasterizationError> {
    BUNDLED_TEXT_FONTS.with(|cached| {
        let mut cached = cached.borrow_mut();
        let font = cached
            .entry(font_ref.to_owned())
            .or_insert_with(|| load_bundled_text_font(font_ref));
        match font {
            Ok(font) => action(font),
            Err(error) => Err(error.clone()),
        }
    })
}

#[cfg(not(target_os = "macos"))]
fn load_bundled_text_font(font_ref: &str) -> Result<Font, TextRasterizationError> {
    validate_bundled_font_registry(&repository_root_from_manifest())
        .map_err(|error| TextRasterizationError::RegistryValidationFailed(error.to_string()))?;
    let font_path =
        bundled_font_path(font_ref).ok_or_else(|| TextRasterizationError::UnregisteredFontRef {
            segment_id: "font-cache".to_owned(),
            font_ref: font_ref.to_owned(),
        })?;
    let font_bytes = fs::read(font_path)
        .map_err(|error| TextRasterizationError::FontReadFailed(error.to_string()))?;
    Font::from_bytes(font_bytes, FontSettings::default())
        .map_err(|error| TextRasterizationError::FontParseFailed(error.to_owned()))
}

#[cfg(not(target_os = "macos"))]
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

#[cfg(not(target_os = "macos"))]
fn aligned_line_x(alignment: TextAlignment, layer_width: u32, line_width: f32) -> f32 {
    match alignment {
        TextAlignment::Left => 0.0,
        TextAlignment::Center => ((layer_width as f32 - line_width) / 2.0).max(0.0),
        TextAlignment::Right => (layer_width as f32 - line_width).max(0.0),
    }
}

#[cfg(not(target_os = "macos"))]
fn blend_glyph_alpha(
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
            pixels[dest_index] = color[0];
            pixels[dest_index + 1] = color[1];
            pixels[dest_index + 2] = color[2];
            pixels[dest_index + 3] = (alpha
                + ((u32::from(pixels[dest_index + 3]) * inverse_alpha + 127) / 255))
                .min(255) as u8;
        }
    }
}

fn unpremultiply_rgba_in_place(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        if alpha == 0 {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
            continue;
        }
        for channel in &mut pixel[0..3] {
            *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
        }
    }
}

fn flip_rgba_rows_in_place(pixels: &mut [u8], width: usize, height: usize) {
    let row_bytes = width.saturating_mul(4);
    if row_bytes == 0 || height <= 1 {
        return;
    }
    for top in 0..(height / 2) {
        let bottom = height - 1 - top;
        let top_start = top * row_bytes;
        let bottom_start = bottom * row_bytes;
        for offset in 0..row_bytes {
            pixels.swap(top_start + offset, bottom_start + offset);
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

#[cfg(test)]
mod tests {
    use draft_model::{
        BUNDLED_SERIF_TEXT_FONT_FAMILY, BUNDLED_SERIF_TEXT_FONT_REF, BUNDLED_TEXT_FONT_FAMILY,
        BUNDLED_TEXT_FONT_REF, Draft, Material, MaterialKind, Microseconds, Segment,
        SourceTimerange, TargetTimerange, TextBackground, TextFont, TextSegment, TextShadow,
        TextStroke, TextStyle, Track, TrackKind,
    };
    use render_graph::OutputDimensions;

    use crate::{RealtimePreviewGraphInput, prepare_realtime_preview_graph};

    use super::{TextRasterizationTarget, rasterize_text_overlay};

    #[test]
    fn rasterizer_uses_the_requested_bundled_font_ref() {
        let sans = rasterized_text_hash(BUNDLED_TEXT_FONT_FAMILY, BUNDLED_TEXT_FONT_REF);
        let serif =
            rasterized_text_hash(BUNDLED_SERIF_TEXT_FONT_FAMILY, BUNDLED_SERIF_TEXT_FONT_REF);

        assert_ne!(
            sans, serif,
            "different bundled fontRefs must produce different text pixels"
        );
    }

    #[test]
    fn rasterizer_outputs_straight_alpha_for_gpu_blending() {
        let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
            draft: text_draft(BUNDLED_TEXT_FONT_FAMILY, BUNDLED_TEXT_FONT_REF),
            target_time: Microseconds::new(500_000),
            preview_dimensions: OutputDimensions::new(960, 540),
        })
        .expect("text draft prepares graph");
        let layer = rasterize_text_overlay(
            &prepared.graph.text_overlays[0],
            None,
            TextRasterizationTarget {
                x: 0,
                y: 0,
                width: 960,
                height: 540,
                canvas_to_target_scale: 0.5,
            },
            3,
        )
        .expect("text rasterizes");

        let edge_pixel = layer
            .pixels
            .chunks_exact(4)
            .find(|pixel| pixel[3] > 0 && pixel[3] < 255)
            .expect("font rasterization should produce anti-aliased edge pixels");

        assert_eq!(
            &edge_pixel[0..3],
            &[255, 255, 255],
            "text textures must use straight alpha; premultiplied RGB is double-blended by the WGPU alpha pipeline and creates fuzzy gray edges"
        );
    }

    #[test]
    fn rasterizer_outputs_target_physical_rect_not_canvas_clamped_rect() {
        let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
            draft: text_draft(BUNDLED_TEXT_FONT_FAMILY, BUNDLED_TEXT_FONT_REF),
            target_time: Microseconds::new(500_000),
            preview_dimensions: OutputDimensions::new(960, 540),
        })
        .expect("text draft prepares graph");
        let target = TextRasterizationTarget {
            x: 31,
            y: 47,
            width: 320,
            height: 96,
            canvas_to_target_scale: 0.5,
        };
        let layer = rasterize_text_overlay(&prepared.graph.text_overlays[0], None, target, 2)
            .expect("text rasterizes at target resolution");

        assert_eq!(layer.x, 31);
        assert_eq!(layer.y, 47);
        assert_eq!(layer.logical_width, 320);
        assert!(layer.logical_height >= 96);
        assert_eq!(layer.width, 640);
        assert_eq!(layer.stride_bytes, 640 * 4);
        assert!(
            layer.pixels.chunks_exact(4).any(|pixel| pixel[3] > 192),
            "target-resolution text texture should contain strong opaque glyph pixels"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn core_text_rasterizer_draws_stroke_shadow_and_background() {
        let mut style = TextStyle::default_title();
        style.font = TextFont {
            family: BUNDLED_TEXT_FONT_FAMILY.to_owned(),
            font_ref: Some(BUNDLED_TEXT_FONT_REF.to_owned()),
        };
        style.font_size = 72;
        style.color = "#ffffff".to_owned();
        style.stroke = Some(TextStroke {
            color: "#000000".to_owned(),
            width: 8,
        });
        style.shadow = Some(TextShadow {
            color: "#ff0000".to_owned(),
            offset_x: 6,
            offset_y: 6,
            blur: 6,
        });
        style.background = Some(TextBackground {
            color: "#204060".to_owned(),
        });
        let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
            draft: text_draft_with_style("text-rasterizer-effects", style),
            target_time: Microseconds::new(500_000),
            preview_dimensions: OutputDimensions::new(960, 540),
        })
        .expect("effect text draft prepares graph");
        let target = TextRasterizationTarget {
            x: 48,
            y: 64,
            width: 360,
            height: 140,
            canvas_to_target_scale: 1.0,
        };
        let layer = rasterize_text_overlay(&prepared.graph.text_overlays[0], None, target, 1)
            .expect("effect text rasterizes");

        assert!(
            layer.x < i64::from(target.x) && layer.y < i64::from(target.y),
            "stroke/shadow padding must expand the text texture instead of clipping effects"
        );
        assert!(layer.logical_width > target.width);
        assert!(layer.logical_height > target.height);

        let background_pixels = layer
            .pixels
            .chunks_exact(4)
            .filter(|pixel| {
                pixel[3] > 220
                    && (24..=40).contains(&pixel[0])
                    && (56..=80).contains(&pixel[1])
                    && (88..=112).contains(&pixel[2])
            })
            .count();
        let fill_pixels = layer
            .pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 220 && pixel[0] > 230 && pixel[1] > 230 && pixel[2] > 230)
            .count();
        let stroke_pixels = layer
            .pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 180 && pixel[0] < 24 && pixel[1] < 24 && pixel[2] < 24)
            .count();
        let shadow_pixels = layer
            .pixels
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 32 && pixel[0] > 150 && pixel[1] < 80 && pixel[2] < 80)
            .count();

        assert!(
            background_pixels > 2_000,
            "realtime text must honor TextBackground instead of rendering transparent fallback only: background={background_pixels} fill={fill_pixels} stroke={stroke_pixels} shadow={shadow_pixels}"
        );
        assert!(
            fill_pixels > 120,
            "foreground glyph fill must remain strong after effect rendering: background={background_pixels} fill={fill_pixels} stroke={stroke_pixels} shadow={shadow_pixels}"
        );
        assert!(
            stroke_pixels > 80,
            "stroke pixels must be present so preview text is not thin and washed out: background={background_pixels} fill={fill_pixels} stroke={stroke_pixels} shadow={shadow_pixels}"
        );
        assert!(
            shadow_pixels > 40,
            "shadow pixels must be present for preview/export text parity: background={background_pixels} fill={fill_pixels} stroke={stroke_pixels} shadow={shadow_pixels}"
        );
    }

    fn rasterized_text_hash(family: &str, font_ref: &str) -> u64 {
        let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
            draft: text_draft(family, font_ref),
            target_time: Microseconds::new(500_000),
            preview_dimensions: OutputDimensions::new(960, 540),
        })
        .expect("text draft prepares graph");
        let layer = rasterize_text_overlay(
            &prepared.graph.text_overlays[0],
            None,
            TextRasterizationTarget {
                x: 0,
                y: 0,
                width: 960,
                height: 540,
                canvas_to_target_scale: 0.5,
            },
            2,
        )
        .expect("text rasterizes");

        layer
            .pixels
            .iter()
            .fold(0xcbf29ce484222325_u64, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
            })
    }

    fn text_draft(family: &str, font_ref: &str) -> Draft {
        let mut style = TextStyle::default_title();
        style.font = TextFont {
            family: family.to_owned(),
            font_ref: Some(font_ref.to_owned()),
        };
        style.font_size = 72;
        style.color = "#ffffff".to_owned();
        text_draft_with_style("text-rasterizer-font", style)
    }

    fn text_draft_with_style(id: &str, style: TextStyle) -> Draft {
        let mut draft = Draft::new("text-rasterizer-font", "Text rasterizer font");
        draft.materials.push(Material::new(
            "text-material",
            MaterialKind::Text,
            "text://title",
            "text-material",
        ));

        let mut segment = Segment::new(
            id,
            "text-material",
            SourceTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
            TargetTimerange::new(Microseconds::new(0), Microseconds::new(1_000_000)),
        );
        segment.text = Some(TextSegment {
            content: "字体字幕".to_owned(),
            source: Default::default(),
            style,
            text_box: Default::default(),
            layout_region: Default::default(),
            wrapping: Default::default(),
            bubble: None,
            effect: None,
        });

        let mut track = Track::new("text-track", TrackKind::Text, "Text");
        track.segments.push(segment);
        draft.tracks.push(track);
        draft
    }
}

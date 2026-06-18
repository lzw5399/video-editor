use draft_model::{
    Microseconds, SegmentId, TargetTimerange, TextAlignment, TextBackground, TextBubbleRef,
    TextEffectRef, TextSegment, TextSegmentSource, TextShadow, TextStroke, TextWrapping, TrackId,
};
use serde::{Deserialize, Serialize};

use crate::{EngineError, EngineErrorKind, NormalizedSegment};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextLayoutProfile {
    pub font_policy: TextFontPolicy,
    pub safe_area: TextSafeArea,
    pub wrapping_policy: TextWrappingPolicy,
    pub line_height_millis: u32,
}

impl TextLayoutProfile {
    pub fn mvp_default() -> Self {
        Self {
            font_policy: TextFontPolicy::mvp_default(),
            safe_area: TextSafeArea {
                left: 96,
                right: 96,
                top: 54,
                bottom: 54,
            },
            wrapping_policy: TextWrappingPolicy::BoundedWidth,
            line_height_millis: 1_200,
        }
    }

    pub fn invalid_for_tests() -> Self {
        Self {
            font_policy: TextFontPolicy {
                font_family: String::new(),
                font_candidate: String::new(),
                fallback_candidates: Vec::new(),
            },
            safe_area: TextSafeArea {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
            },
            wrapping_policy: TextWrappingPolicy::BoundedWidth,
            line_height_millis: 0,
        }
    }

    pub fn validate(&self, canvas_width: u32, canvas_height: u32) -> Result<(), EngineError> {
        self.font_policy.validate()?;
        if self.line_height_millis == 0 {
            return Err(invalid_text_layout(
                "text layout lineHeightMillis must be greater than zero",
            ));
        }
        if self.safe_area.left.saturating_add(self.safe_area.right) >= canvas_width {
            return Err(invalid_text_layout(
                "text layout horizontal safe-area exceeds canvas width",
            ));
        }
        if self.safe_area.top.saturating_add(self.safe_area.bottom) >= canvas_height {
            return Err(invalid_text_layout(
                "text layout vertical safe-area exceeds canvas height",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextFontPolicy {
    pub font_family: String,
    pub font_candidate: String,
    pub fallback_candidates: Vec<String>,
}

impl TextFontPolicy {
    pub fn mvp_default() -> Self {
        Self {
            font_family: "PingFang SC".to_owned(),
            font_candidate: "VE_TEXT_FONT_PATH".to_owned(),
            fallback_candidates: vec![
                "VE_TEXT_FONT_PATH".to_owned(),
                "/System/Library/Fonts/PingFang.ttc".to_owned(),
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf".to_owned(),
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".to_owned(),
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_owned(),
            ],
        }
    }

    fn validate(&self) -> Result<(), EngineError> {
        if self.font_family.trim().is_empty() {
            return Err(invalid_text_layout(
                "text font policy fontFamily must be pinned",
            ));
        }
        if self.font_candidate.trim().is_empty() {
            return Err(invalid_text_layout(
                "text font policy fontCandidate must be resolved by the caller",
            ));
        }
        if self.fallback_candidates.is_empty() {
            return Err(invalid_text_layout(
                "text font policy fallbackCandidates must be pinned",
            ));
        }
        if !self
            .fallback_candidates
            .iter()
            .any(|candidate| candidate == &self.font_candidate)
        {
            return Err(invalid_text_layout(
                "text font policy fontCandidate must come from fallbackCandidates",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextSafeArea {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextWrappingPolicy {
    BoundedWidth,
}

impl TextWrappingPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BoundedWidth => "boundedWidth",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedTextOverlay {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub content: String,
    pub stack_index: u32,
    pub source_position: Microseconds,
    pub target_timerange: TargetTimerange,
    pub source: TextSegmentSource,
    pub font_family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_ref: Option<String>,
    pub font_candidate: String,
    pub fallback_candidates: Vec<String>,
    pub alignment: TextAlignment,
    pub text_box: ResolvedTextBox,
    pub layout_region: ResolvedTextLayoutRegion,
    pub safe_area: TextSafeArea,
    pub wrapping: TextWrapping,
    pub wrapping_policy: TextWrappingPolicy,
    pub line_height_millis: u32,
    pub letter_spacing_millis: u32,
    pub font_size: u32,
    pub style: ResolvedTextStyle,
    pub layout_width: u32,
    pub layout_height: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ResolvedTextDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedTextBox {
    pub width_millis: u32,
    pub height_millis: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedTextLayoutRegion {
    pub x_millis: u32,
    pub y_millis: u32,
    pub width_millis: u32,
    pub height_millis: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedTextDiagnostic {
    pub property: String,
    pub support: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedTextStyle {
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<TextStroke>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<TextShadow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<TextBackground>,
}

pub fn resolve_text_overlay(
    track_id: &TrackId,
    segment: &NormalizedSegment,
    text: &TextSegment,
    stack_index: u32,
    source_position: Microseconds,
    profile: &TextLayoutProfile,
    canvas_width: u32,
    canvas_height: u32,
) -> Result<ResolvedTextOverlay, EngineError> {
    profile.validate(canvas_width, canvas_height)?;
    let line_count = text.content.lines().count().max(1) as u32;
    let text_box = resolve_text_box(text, canvas_width, canvas_height);
    let layout_region = resolve_layout_region(text, canvas_width, canvas_height);
    let safe_area = safe_area_from_region(&layout_region, canvas_width, canvas_height)?;
    let layout_width = text_box.width.min(layout_region.width);
    let layout_height = ceil_div_u32(
        text.style
            .font_size
            .checked_mul(text.style.line_height_millis)
            .and_then(|value| value.checked_mul(line_count))
            .ok_or_else(|| invalid_text_layout("text layout height calculation overflowed"))?,
        1_000,
    )
    .min(text_box.height)
    .min(layout_region.height);

    Ok(ResolvedTextOverlay {
        track_id: track_id.clone(),
        segment_id: segment.segment_id.clone(),
        content: text.content.clone(),
        stack_index,
        source_position,
        target_timerange: segment.target_timerange.clone(),
        source: text.source,
        font_family: if text.style.font.family.trim().is_empty() {
            profile.font_policy.font_family.clone()
        } else {
            text.style.font.family.clone()
        },
        font_ref: text.style.font.font_ref.clone(),
        font_candidate: profile.font_policy.font_candidate.clone(),
        fallback_candidates: profile.font_policy.fallback_candidates.clone(),
        alignment: text.style.alignment,
        text_box,
        layout_region,
        safe_area,
        wrapping: text.wrapping,
        wrapping_policy: profile.wrapping_policy,
        line_height_millis: text.style.line_height_millis,
        letter_spacing_millis: text.style.letter_spacing_millis,
        font_size: text.style.font_size,
        style: ResolvedTextStyle {
            color: text.style.color.clone(),
            stroke: text.style.stroke.clone(),
            shadow: text.style.shadow.clone(),
            background: text.style.background.clone(),
        },
        layout_width,
        layout_height,
        diagnostics: text_diagnostics(text),
    })
}

fn resolve_text_box(text: &TextSegment, canvas_width: u32, canvas_height: u32) -> ResolvedTextBox {
    ResolvedTextBox {
        width_millis: text.text_box.width_millis,
        height_millis: text.text_box.height_millis,
        width: millis_of(canvas_width, text.text_box.width_millis),
        height: millis_of(canvas_height, text.text_box.height_millis),
    }
}

fn resolve_layout_region(
    text: &TextSegment,
    canvas_width: u32,
    canvas_height: u32,
) -> ResolvedTextLayoutRegion {
    ResolvedTextLayoutRegion {
        x_millis: text.layout_region.x_millis,
        y_millis: text.layout_region.y_millis,
        width_millis: text.layout_region.width_millis,
        height_millis: text.layout_region.height_millis,
        x: millis_of(canvas_width, text.layout_region.x_millis),
        y: millis_of(canvas_height, text.layout_region.y_millis),
        width: millis_of(canvas_width, text.layout_region.width_millis),
        height: millis_of(canvas_height, text.layout_region.height_millis),
    }
}

fn safe_area_from_region(
    region: &ResolvedTextLayoutRegion,
    canvas_width: u32,
    canvas_height: u32,
) -> Result<TextSafeArea, EngineError> {
    let right = canvas_width
        .checked_sub(region.x)
        .and_then(|value| value.checked_sub(region.width))
        .ok_or_else(|| invalid_text_layout("text layout region exceeds canvas width"))?;
    let bottom = canvas_height
        .checked_sub(region.y)
        .and_then(|value| value.checked_sub(region.height))
        .ok_or_else(|| invalid_text_layout("text layout region exceeds canvas height"))?;
    Ok(TextSafeArea {
        left: region.x,
        right,
        top: region.y,
        bottom,
    })
}

fn millis_of(value: u32, millis: u32) -> u32 {
    ((u64::from(value) * u64::from(millis)) / 1_000) as u32
}

fn text_diagnostics(text: &TextSegment) -> Vec<ResolvedTextDiagnostic> {
    let mut diagnostics = Vec::new();
    if let Some(TextBubbleRef::Unsupported { name, .. }) = &text.bubble {
        diagnostics.push(ResolvedTextDiagnostic {
            property: "bubble".to_owned(),
            support: "unsupported".to_owned(),
            reason: format!("text bubble {name} is unsupported"),
        });
    }
    if let Some(TextEffectRef::Unsupported { name, .. }) = &text.effect {
        diagnostics.push(ResolvedTextDiagnostic {
            property: "effect".to_owned(),
            support: "unsupported".to_owned(),
            reason: format!("text effect {name} is unsupported"),
        });
    }
    diagnostics
}

fn ceil_div_u32(value: u32, denominator: u32) -> u32 {
    value.div_ceil(denominator)
}

fn invalid_text_layout(message: impl Into<String>) -> EngineError {
    EngineError::new(EngineErrorKind::InvalidTextLayoutProfile, message)
}

use draft_model::{Microseconds, SegmentId, TargetTimerange, TextAlignment, TextSegment, TrackId};
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
    pub font_family: String,
    pub font_candidate: String,
    pub fallback_candidates: Vec<String>,
    pub alignment: TextAlignment,
    pub safe_area: TextSafeArea,
    pub wrapping_policy: TextWrappingPolicy,
    pub font_size: u32,
    pub layout_width: u32,
    pub layout_height: u32,
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
    let layout_width = canvas_width
        .checked_sub(profile.safe_area.left)
        .and_then(|value| value.checked_sub(profile.safe_area.right))
        .ok_or_else(|| invalid_text_layout("text layout safe-area exceeds canvas width"))?;
    let layout_height = ceil_div_u32(
        text.style
            .font_size
            .checked_mul(profile.line_height_millis)
            .and_then(|value| value.checked_mul(line_count))
            .ok_or_else(|| invalid_text_layout("text layout height calculation overflowed"))?,
        1_000,
    );

    Ok(ResolvedTextOverlay {
        track_id: track_id.clone(),
        segment_id: segment.segment_id.clone(),
        content: text.content.clone(),
        stack_index,
        source_position,
        target_timerange: segment.target_timerange.clone(),
        font_family: profile.font_policy.font_family.clone(),
        font_candidate: profile.font_policy.font_candidate.clone(),
        fallback_candidates: profile.font_policy.fallback_candidates.clone(),
        alignment: text.style.alignment,
        safe_area: profile.safe_area.clone(),
        wrapping_policy: profile.wrapping_policy,
        font_size: text.style.font_size,
        layout_width,
        layout_height,
    })
}

fn ceil_div_u32(value: u32, denominator: u32) -> u32 {
    value.div_ceil(denominator)
}

fn invalid_text_layout(message: impl Into<String>) -> EngineError {
    EngineError::new(EngineErrorKind::InvalidTextLayoutProfile, message)
}

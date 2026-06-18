use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{MaterialId, Microseconds, SegmentId, TrackId};

pub const MAX_SEGMENT_VOLUME_MILLIS: u32 = 4_000;
pub const MAX_SEGMENT_OPACITY_MILLIS: u32 = 1_000;
pub const MAX_SEGMENT_CROP_MILLIS: u32 = 1_000;
pub const MAX_SEGMENT_ANCHOR_MILLIS: u32 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum TrackKind {
    Video,
    Audio,
    Text,
    Sticker,
    Filter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTimerange {
    pub start: Microseconds,
    pub duration: Microseconds,
}

impl SourceTimerange {
    pub fn new(start: impl Into<Microseconds>, duration: impl Into<Microseconds>) -> Self {
        Self {
            start: start.into(),
            duration: duration.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TargetTimerange {
    pub start: Microseconds,
    pub duration: Microseconds,
}

impl TargetTimerange {
    pub fn new(start: impl Into<Microseconds>, duration: impl Into<Microseconds>) -> Self {
        Self {
            start: start.into(),
            duration: duration.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MainTrackMagnet {
    pub enabled: bool,
}

impl MainTrackMagnet {
    pub fn enabled() -> Self {
        Self { enabled: true }
    }

    pub fn disabled() -> Self {
        Self { enabled: false }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Keyframe {
    pub at: Microseconds,
    pub property: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Filter {
    pub name: String,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Transition {
    pub name: String,
    pub duration: Microseconds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextStroke {
    pub color: String,
    pub width: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextShadow {
    pub color: String,
    pub offset_x: i32,
    pub offset_y: i32,
    pub blur: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextBackground {
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextStyle {
    pub font_size: u32,
    pub color: String,
    pub alignment: TextAlignment,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub stroke: Option<TextStroke>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub shadow: Option<TextShadow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub background: Option<TextBackground>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextSegment {
    pub content: String,
    pub style: TextStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentVolume {
    pub level_millis: u32,
}

impl SegmentVolume {
    pub const fn unity() -> Self {
        Self {
            level_millis: 1_000,
        }
    }
}

impl Default for SegmentVolume {
    fn default() -> Self {
        Self::unity()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentPosition {
    pub x: i32,
    pub y: i32,
}

impl SegmentPosition {
    pub const fn center() -> Self {
        Self { x: 0, y: 0 }
    }
}

impl Default for SegmentPosition {
    fn default() -> Self {
        Self::center()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentScale {
    pub x_millis: u32,
    pub y_millis: u32,
}

impl SegmentScale {
    pub const fn unity() -> Self {
        Self {
            x_millis: 1_000,
            y_millis: 1_000,
        }
    }
}

impl Default for SegmentScale {
    fn default() -> Self {
        Self::unity()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentRotation {
    pub degrees: i32,
}

impl SegmentRotation {
    pub const fn zero() -> Self {
        Self { degrees: 0 }
    }
}

impl Default for SegmentRotation {
    fn default() -> Self {
        Self::zero()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentOpacity {
    pub value_millis: u32,
}

impl SegmentOpacity {
    pub const fn opaque() -> Self {
        Self {
            value_millis: MAX_SEGMENT_OPACITY_MILLIS,
        }
    }
}

impl Default for SegmentOpacity {
    fn default() -> Self {
        Self::opaque()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentCrop {
    pub left_millis: u32,
    pub right_millis: u32,
    pub top_millis: u32,
    pub bottom_millis: u32,
}

impl SegmentCrop {
    pub const fn none() -> Self {
        Self {
            left_millis: 0,
            right_millis: 0,
            top_millis: 0,
            bottom_millis: 0,
        }
    }
}

impl Default for SegmentCrop {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentAnchor {
    pub x_millis: u32,
    pub y_millis: u32,
}

impl SegmentAnchor {
    pub const fn center() -> Self {
        Self {
            x_millis: 500,
            y_millis: 500,
        }
    }
}

impl Default for SegmentAnchor {
    fn default() -> Self {
        Self::center()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentTransform {
    pub position: SegmentPosition,
    pub scale: SegmentScale,
    pub rotation: SegmentRotation,
    pub opacity: SegmentOpacity,
    pub crop: SegmentCrop,
    pub anchor: SegmentAnchor,
}

impl SegmentTransform {
    pub fn identity() -> Self {
        Self {
            position: SegmentPosition::default(),
            scale: SegmentScale::default(),
            rotation: SegmentRotation::default(),
            opacity: SegmentOpacity::default(),
            crop: SegmentCrop::default(),
            anchor: SegmentAnchor::default(),
        }
    }
}

impl Default for SegmentTransform {
    fn default() -> Self {
        Self::identity()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum SegmentFitMode {
    Fit,
    Fill,
    Stretch,
}

impl Default for SegmentFitMode {
    fn default() -> Self {
        Self::Stretch
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SegmentBackgroundFilling {
    None,
    Black,
    SolidColor {
        color: String,
    },
    Blur,
    Image {
        #[serde(rename = "materialId")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        material_id: Option<MaterialId>,
    },
}

impl Default for SegmentBackgroundFilling {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SegmentBlendMode {
    Normal,
    Unsupported { name: String },
}

impl Default for SegmentBlendMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SegmentMask {
    None,
    Unsupported { name: String },
}

impl Default for SegmentMask {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentVisual {
    pub visible: bool,
    pub transform: SegmentTransform,
    pub fit_mode: SegmentFitMode,
    pub background_filling: SegmentBackgroundFilling,
    pub blend_mode: SegmentBlendMode,
    pub mask: SegmentMask,
}

impl SegmentVisual {
    pub fn visible_identity() -> Self {
        Self {
            visible: true,
            transform: SegmentTransform::default(),
            fit_mode: SegmentFitMode::default(),
            background_filling: SegmentBackgroundFilling::default(),
            blend_mode: SegmentBlendMode::default(),
            mask: SegmentMask::default(),
        }
    }
}

impl Default for SegmentVisual {
    fn default() -> Self {
        Self::visible_identity()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Segment {
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
    pub main_track_magnet: MainTrackMagnet,
    pub keyframes: Vec<Keyframe>,
    pub filters: Vec<Filter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub transition: Option<Transition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub text: Option<TextSegment>,
    #[serde(default)]
    pub volume: SegmentVolume,
    #[serde(default)]
    pub visual: SegmentVisual,
}

impl Segment {
    pub fn new(
        segment_id: impl Into<SegmentId>,
        material_id: impl Into<MaterialId>,
        source_timerange: SourceTimerange,
        target_timerange: TargetTimerange,
    ) -> Self {
        Self {
            segment_id: segment_id.into(),
            material_id: material_id.into(),
            source_timerange,
            target_timerange,
            main_track_magnet: MainTrackMagnet::disabled(),
            keyframes: Vec::new(),
            filters: Vec::new(),
            transition: None,
            text: None,
            volume: SegmentVolume::default(),
            visual: SegmentVisual::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Track {
    pub track_id: TrackId,
    pub kind: TrackKind,
    pub name: String,
    pub muted: bool,
    pub locked: bool,
    pub segments: Vec<Segment>,
}

impl Track {
    pub fn new(track_id: impl Into<TrackId>, kind: TrackKind, name: impl Into<String>) -> Self {
        Self {
            track_id: track_id.into(),
            kind,
            name: name.into(),
            muted: false,
            locked: false,
            segments: Vec::new(),
        }
    }
}

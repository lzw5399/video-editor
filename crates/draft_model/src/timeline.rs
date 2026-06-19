use std::{borrow::Cow, collections::BTreeMap};

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

use crate::{MaterialId, Microseconds, SegmentId, TrackId};

pub const MAX_SEGMENT_VOLUME_MILLIS: u32 = 4_000;
pub const MIN_AUDIO_PAN_BALANCE_MILLIS: i32 = -1_000;
pub const MAX_AUDIO_PAN_BALANCE_MILLIS: i32 = 1_000;
pub const MAX_AUDIO_FADE_DURATION_MICROSECONDS: u64 = 3_600_000_000;
pub const MAX_SEGMENT_OPACITY_MILLIS: u32 = 1_000;
pub const MAX_SEGMENT_CROP_MILLIS: u32 = 1_000;
pub const MAX_SEGMENT_ANCHOR_MILLIS: u32 = 1_000;
pub const MIN_TEXT_LINE_HEIGHT_MILLIS: u32 = 500;
pub const MAX_TEXT_LINE_HEIGHT_MILLIS: u32 = 3_000;
pub const MAX_TEXT_LETTER_SPACING_MILLIS: u32 = 2_000;
pub const MAX_TEXT_LAYOUT_MILLIS: u32 = 1_000;

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

    pub fn checked_end(&self) -> Option<Microseconds> {
        self.start
            .get()
            .checked_add(self.duration.get())
            .map(Microseconds::new)
    }

    pub fn overlaps_half_open(&self, other: &Self) -> Option<bool> {
        let self_end = self.checked_end()?;
        let other_end = other.checked_end()?;
        Some(self.start.get() < other_end.get() && other.start.get() < self_end.get())
    }

    pub fn union(&self, other: &Self) -> Option<Self> {
        let start = self.start.get().min(other.start.get());
        let end = self.checked_end()?.get().max(other.checked_end()?.get());
        Some(Self::new(start, end.checked_sub(start)?))
    }

    pub fn merge_sorted(ranges: impl IntoIterator<Item = Self>) -> Option<Vec<Self>> {
        let mut sorted = ranges.into_iter().collect::<Vec<_>>();
        sorted.sort_by_key(|range| (range.start, range.duration));

        let mut merged: Vec<Self> = Vec::new();
        for range in sorted {
            range.checked_end()?;
            let Some(current) = merged.last_mut() else {
                merged.push(range);
                continue;
            };

            if current.checked_end()?.get() >= range.start.get() {
                *current = current.union(&range)?;
            } else {
                merged.push(range);
            }
        }

        Some(merged)
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
    pub property: KeyframeProperty,
    pub value: KeyframeValue,
    pub interpolation: KeyframeInterpolation,
    pub easing: KeyframeEasing,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum KeyframeProperty {
    VisualPositionX,
    VisualPositionY,
    VisualScaleX,
    VisualScaleY,
    VisualRotation,
    VisualOpacity,
    TextFontSize,
    TextColor,
    TextLineHeight,
    TextLetterSpacing,
    TextLayoutX,
    TextLayoutY,
    TextLayoutWidth,
    TextLayoutHeight,
    Volume,
    StickerPositionX,
    StickerPositionY,
    StickerScaleX,
    StickerScaleY,
    FilterParameterUnsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum KeyframeValue {
    Int { value: i32 },
    Uint { value: u32 },
    Color { value: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum KeyframeInterpolation {
    Hold,
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum KeyframeEasing {
    None,
    EaseIn,
    EaseOut,
    EaseInOut,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum TextSegmentSource {
    Text,
    Subtitle,
}

impl Default for TextSegmentSource {
    fn default() -> Self {
        Self::Text
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextFont {
    pub family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub font_ref: Option<String>,
}

impl TextFont {
    pub fn bundled_default() -> Self {
        Self {
            family: crate::BUNDLED_TEXT_FONT_FAMILY.to_owned(),
            font_ref: Some(crate::BUNDLED_TEXT_FONT_REF.to_owned()),
        }
    }

    pub fn system_default() -> Self {
        Self {
            family: "PingFang SC".to_owned(),
            font_ref: None,
        }
    }
}

impl Default for TextFont {
    fn default() -> Self {
        Self::bundled_default()
    }
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
    #[serde(default)]
    pub font: TextFont,
    pub font_size: u32,
    pub color: String,
    pub alignment: TextAlignment,
    #[serde(default = "default_text_line_height_millis")]
    pub line_height_millis: u32,
    #[serde(default)]
    pub letter_spacing_millis: u32,
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

impl TextStyle {
    pub fn default_title() -> Self {
        Self {
            font: TextFont::default(),
            font_size: 36,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            line_height_millis: default_text_line_height_millis(),
            letter_spacing_millis: 0,
            stroke: None,
            shadow: None,
            background: None,
        }
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::default_title()
    }
}

fn default_text_line_height_millis() -> u32 {
    1_200
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextBox {
    pub width_millis: u32,
    pub height_millis: u32,
}

impl TextBox {
    pub const fn default_box() -> Self {
        Self {
            width_millis: 800,
            height_millis: 200,
        }
    }
}

impl Default for TextBox {
    fn default() -> Self {
        Self::default_box()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextLayoutRegion {
    pub x_millis: u32,
    pub y_millis: u32,
    pub width_millis: u32,
    pub height_millis: u32,
}

impl TextLayoutRegion {
    pub const fn safe_area() -> Self {
        Self {
            x_millis: 100,
            y_millis: 100,
            width_millis: 800,
            height_millis: 800,
        }
    }
}

impl Default for TextLayoutRegion {
    fn default() -> Self {
        Self::safe_area()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum TextWrapping {
    None,
    Auto,
}

impl Default for TextWrapping {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TextBubbleRef {
    Unsupported {
        name: String,
        #[serde(rename = "externalRef")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        external_ref: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TextEffectRef {
    Unsupported {
        name: String,
        #[serde(rename = "externalRef")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        external_ref: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextSegment {
    pub content: String,
    #[serde(default)]
    pub source: TextSegmentSource,
    pub style: TextStyle,
    #[serde(default)]
    pub text_box: TextBox,
    #[serde(default)]
    pub layout_region: TextLayoutRegion,
    #[serde(default)]
    pub wrapping: TextWrapping,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub bubble: Option<TextBubbleRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub effect: Option<TextEffectRef>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, TS)]
#[ts(type = "number")]
pub struct AudioPanBalance {
    pub balance_millis: i32,
}

impl Serialize for AudioPanBalance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(self.balance_millis)
    }
}

impl<'de> Deserialize<'de> for AudioPanBalance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self {
            balance_millis: i32::deserialize(deserializer)?,
        })
    }
}

impl JsonSchema for AudioPanBalance {
    fn schema_name() -> Cow<'static, str> {
        "AudioPanBalance".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::AudioPanBalance").into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "integer",
            "format": "int32",
            "minimum": MIN_AUDIO_PAN_BALANCE_MILLIS,
            "maximum": MAX_AUDIO_PAN_BALANCE_MILLIS
        })
    }
}

impl AudioPanBalance {
    pub const fn center() -> Self {
        Self { balance_millis: 0 }
    }
}

impl Default for AudioPanBalance {
    fn default() -> Self {
        Self::center()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioFade {
    pub duration: Microseconds,
}

impl AudioFade {
    pub const fn none() -> Self {
        Self {
            duration: Microseconds::ZERO,
        }
    }
}

impl Default for AudioFade {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AudioEffectSlotKind {
    Unsupported {
        name: String,
        #[serde(rename = "externalRef")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        external_ref: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioEffectSlot {
    pub slot_id: String,
    pub kind: AudioEffectSlotKind,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentAudio {
    pub gain_millis: u32,
    pub pan_balance_millis: AudioPanBalance,
    pub fade_in_duration: AudioFade,
    pub fade_out_duration: AudioFade,
    pub effect_slots: Vec<AudioEffectSlot>,
}

impl SegmentAudio {
    pub fn unity() -> Self {
        Self {
            gain_millis: SegmentVolume::unity().level_millis,
            pan_balance_millis: AudioPanBalance::center(),
            fade_in_duration: AudioFade::default(),
            fade_out_duration: AudioFade::default(),
            effect_slots: Vec::new(),
        }
    }
}

impl Default for SegmentAudio {
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
        Self::Fit
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
    pub audio: SegmentAudio,
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
            audio: SegmentAudio::default(),
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
    #[serde(default = "default_track_visible")]
    pub visible: bool,
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
            visible: true,
            segments: Vec::new(),
        }
    }
}

fn default_track_visible() -> bool {
    true
}

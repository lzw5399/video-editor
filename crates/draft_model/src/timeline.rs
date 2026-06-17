use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{MaterialId, Microseconds, SegmentId, TrackId};

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

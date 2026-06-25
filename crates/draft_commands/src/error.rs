//! Structured timeline command errors.

use std::error::Error;
use std::fmt;

use draft_model::{
    DraftValidationError, MaterialId, MaterialKind, Microseconds, SegmentId, TrackId, TrackKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineCommandError {
    pub kind: TimelineCommandErrorKind,
}

impl TimelineCommandError {
    pub fn new(kind: TimelineCommandErrorKind) -> Self {
        Self { kind }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineCommandErrorKind {
    TrackNotFound {
        track_id: TrackId,
    },
    SegmentNotFound {
        segment_id: SegmentId,
    },
    MaterialNotFound {
        material_id: MaterialId,
    },
    LockedTrack {
        track_id: TrackId,
    },
    OverlappingSegment {
        track_id: TrackId,
        first_segment_id: SegmentId,
        second_segment_id: SegmentId,
    },
    IncompatibleTrackMaterialKind {
        track_id: TrackId,
        track_kind: TrackKind,
        material_id: MaterialId,
        material_kind: MaterialKind,
    },
    InvalidTrackOperation {
        track_id: TrackId,
        reason: String,
    },
    SourceRangeExceedsMaterialDuration {
        segment_id: SegmentId,
        material_id: MaterialId,
        source_end: Microseconds,
        material_duration: Microseconds,
    },
    TimerangeOverflow {
        field: String,
    },
    ZeroDuration {
        field: String,
    },
    InvalidSplitPoint {
        segment_id: SegmentId,
        split_at: Microseconds,
    },
    InvalidRetime {
        segment_id: SegmentId,
        reason: String,
    },
    InvalidEffectParameter {
        segment_id: SegmentId,
        capability_id: String,
        parameter: String,
        reason: String,
    },
    UnsupportedEffect {
        segment_id: SegmentId,
        capability_id: String,
        reason: String,
    },
    EffectNotFound {
        segment_id: SegmentId,
        effect_index: u32,
    },
    InvalidTransitionRelationship {
        track_id: TrackId,
        from_segment_id: SegmentId,
        to_segment_id: SegmentId,
        reason: String,
    },
    UnsupportedAudioRetime {
        segment_id: SegmentId,
        policy: String,
        reason: String,
    },
    HistoryEmpty {
        direction: String,
    },
    UnsupportedCommand {
        command: String,
    },
    DraftValidationFailed {
        message: String,
    },
}

impl From<DraftValidationError> for TimelineCommandError {
    fn from(error: DraftValidationError) -> Self {
        Self::new(TimelineCommandErrorKind::DraftValidationFailed {
            message: error.to_string(),
        })
    }
}

impl fmt::Display for TimelineCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TimelineCommandErrorKind::TrackNotFound { track_id } => {
                write!(formatter, "track not found: {}", track_id.as_str())
            }
            TimelineCommandErrorKind::SegmentNotFound { segment_id } => {
                write!(formatter, "segment not found: {}", segment_id.as_str())
            }
            TimelineCommandErrorKind::MaterialNotFound { material_id } => {
                write!(formatter, "material not found: {}", material_id.as_str())
            }
            TimelineCommandErrorKind::LockedTrack { track_id } => {
                write!(formatter, "track is locked: {}", track_id.as_str())
            }
            TimelineCommandErrorKind::OverlappingSegment {
                track_id,
                first_segment_id,
                second_segment_id,
            } => write!(
                formatter,
                "segments overlap on track {}: {} and {}",
                track_id.as_str(),
                first_segment_id.as_str(),
                second_segment_id.as_str()
            ),
            TimelineCommandErrorKind::IncompatibleTrackMaterialKind {
                track_id,
                track_kind,
                material_id,
                material_kind,
            } => write!(
                formatter,
                "track {} ({track_kind:?}) is incompatible with material {} ({material_kind:?})",
                track_id.as_str(),
                material_id.as_str()
            ),
            TimelineCommandErrorKind::InvalidTrackOperation { track_id, reason } => write!(
                formatter,
                "invalid track operation on {}: {}",
                track_id.as_str(),
                reason
            ),
            TimelineCommandErrorKind::SourceRangeExceedsMaterialDuration {
                segment_id,
                material_id,
                source_end,
                material_duration,
            } => write!(
                formatter,
                "segment {} source range ends at {} beyond material {} duration {}",
                segment_id.as_str(),
                source_end.get(),
                material_id.as_str(),
                material_duration.get()
            ),
            TimelineCommandErrorKind::TimerangeOverflow { field } => {
                write!(formatter, "timerange overflow: {field}")
            }
            TimelineCommandErrorKind::ZeroDuration { field } => {
                write!(formatter, "zero-duration timerange: {field}")
            }
            TimelineCommandErrorKind::InvalidSplitPoint {
                segment_id,
                split_at,
            } => write!(
                formatter,
                "invalid split point {} for segment {}",
                split_at.get(),
                segment_id.as_str()
            ),
            TimelineCommandErrorKind::InvalidRetime { segment_id, reason } => write!(
                formatter,
                "invalid retime for segment {}: {}",
                segment_id.as_str(),
                reason
            ),
            TimelineCommandErrorKind::InvalidEffectParameter {
                segment_id,
                capability_id,
                parameter,
                reason,
            } => write!(
                formatter,
                "invalid effect parameter {parameter} for {capability_id} on segment {}: {}",
                segment_id.as_str(),
                reason
            ),
            TimelineCommandErrorKind::UnsupportedEffect {
                segment_id,
                capability_id,
                reason,
            } => write!(
                formatter,
                "unsupported effect {capability_id} for segment {}: {}",
                segment_id.as_str(),
                reason
            ),
            TimelineCommandErrorKind::EffectNotFound {
                segment_id,
                effect_index,
            } => write!(
                formatter,
                "effect index {effect_index} not found on segment {}",
                segment_id.as_str()
            ),
            TimelineCommandErrorKind::InvalidTransitionRelationship {
                track_id,
                from_segment_id,
                to_segment_id,
                reason,
            } => write!(
                formatter,
                "invalid transition relationship on track {} from {} to {}: {}",
                track_id.as_str(),
                from_segment_id.as_str(),
                to_segment_id.as_str(),
                reason
            ),
            TimelineCommandErrorKind::UnsupportedAudioRetime {
                segment_id,
                policy,
                reason,
            } => write!(
                formatter,
                "unsupported audio retime policy {policy} for segment {}: {}",
                segment_id.as_str(),
                reason
            ),
            TimelineCommandErrorKind::HistoryEmpty { direction } => {
                write!(formatter, "{direction} history is empty")
            }
            TimelineCommandErrorKind::UnsupportedCommand { command } => {
                write!(formatter, "unsupported timeline command: {command}")
            }
            TimelineCommandErrorKind::DraftValidationFailed { message } => {
                write!(formatter, "draft validation failed: {message}")
            }
        }
    }
}

impl Error for TimelineCommandError {}

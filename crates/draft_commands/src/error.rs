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

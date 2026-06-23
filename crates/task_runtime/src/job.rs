use draft_model::Microseconds;
use serde::{Deserialize, Serialize};

use crate::{PlaybackGeneration, TaskCancellationToken, config::QueuePolicy};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(String);

impl JobId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for JobId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for JobId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobDomain {
    InteractivePreview,
    ScrubSeek,
    Decode,
    Audio,
    ArtifactGeneration,
    Export,
    MediaProbe,
    FilesystemIo,
    Analysis,
}

impl JobDomain {
    pub const fn all() -> &'static [Self] {
        &[
            Self::InteractivePreview,
            Self::ScrubSeek,
            Self::Decode,
            Self::Audio,
            Self::ArtifactGeneration,
            Self::Export,
            Self::MediaProbe,
            Self::FilesystemIo,
            Self::Analysis,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobPriority {
    Realtime,
    Interactive,
    UserVisible,
    Background,
    Maintenance,
}

impl JobPriority {
    pub const fn rank(self) -> u8 {
        match self {
            Self::Realtime => 0,
            Self::Interactive => 1,
            Self::UserVisible => 2,
            Self::Background => 3,
            Self::Maintenance => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceClass {
    GpuPresent,
    GpuDecode,
    CpuDecode,
    AudioRealtime,
    FfmpegProcess,
    DiskIo,
    SqliteWrite,
    BackgroundCpu,
    ValidationProbe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", deny_unknown_fields)]
pub enum JobFreshness {
    None,
    Timeline {
        #[serde(rename = "targetTime")]
        target_time: Microseconds,
        #[serde(rename = "playbackGeneration")]
        playback_generation: PlaybackGeneration,
        #[serde(
            default,
            rename = "projectSessionId",
            skip_serializing_if = "Option::is_none"
        )]
        project_session_id: Option<String>,
        #[serde(
            default,
            rename = "expectedRevision",
            skip_serializing_if = "Option::is_none"
        )]
        expected_revision: Option<u64>,
    },
}

impl JobFreshness {
    pub const fn none() -> Self {
        Self::None
    }

    pub fn timeline(target_time: Microseconds, playback_generation: PlaybackGeneration) -> Self {
        Self::Timeline {
            target_time,
            playback_generation,
            project_session_id: None,
            expected_revision: None,
        }
    }

    pub fn with_project_session(
        self,
        project_session_id: impl Into<String>,
        expected_revision: u64,
    ) -> Self {
        match self {
            Self::Timeline {
                target_time,
                playback_generation,
                ..
            } => Self::Timeline {
                target_time,
                playback_generation,
                project_session_id: Some(project_session_id.into()),
                expected_revision: Some(expected_revision),
            },
            Self::None => Self::None,
        }
    }

    pub fn target_time(&self) -> Option<Microseconds> {
        match self {
            Self::Timeline { target_time, .. } => Some(*target_time),
            Self::None => None,
        }
    }

    pub fn playback_generation(&self) -> Option<PlaybackGeneration> {
        match self {
            Self::Timeline {
                playback_generation,
                ..
            } => Some(*playback_generation),
            Self::None => None,
        }
    }

    pub fn is_obsolete_compared_to(&self, newer: &Self) -> bool {
        match (self, newer) {
            (
                Self::Timeline {
                    target_time,
                    playback_generation,
                    project_session_id,
                    ..
                },
                Self::Timeline {
                    target_time: newer_target_time,
                    playback_generation: newer_generation,
                    project_session_id: newer_project_session_id,
                    ..
                },
            ) => {
                project_session_id == newer_project_session_id
                    && newer_generation >= playback_generation
                    && (newer_generation > playback_generation || newer_target_time >= target_time)
            }
            _ => false,
        }
    }

    pub fn is_stale_for(&self, current: CompletionFreshness) -> bool {
        match self {
            Self::None => false,
            Self::Timeline {
                playback_generation,
                expected_revision,
                ..
            } => {
                current
                    .playback_generation
                    .is_some_and(|current| current != *playback_generation)
                    || expected_revision.is_some_and(|expected| {
                        current
                            .expected_revision
                            .map_or(true, |current| current != expected)
                    })
            }
        }
    }
}

impl Default for JobFreshness {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobEnvelope {
    pub job_id: JobId,
    pub domain: JobDomain,
    pub priority: JobPriority,
    pub resource_class: ResourceClass,
    pub freshness: JobFreshness,
    pub cancellation_token: TaskCancellationToken,
    pub submitted_at_us: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_at_us: Option<u64>,
    pub queue_policy: QueuePolicy,
}

impl JobEnvelope {
    pub fn new(
        job_id: JobId,
        domain: JobDomain,
        priority: JobPriority,
        resource_class: ResourceClass,
        cancellation_token: TaskCancellationToken,
        submitted_at_us: u64,
    ) -> Self {
        Self {
            job_id,
            domain,
            priority,
            resource_class,
            freshness: JobFreshness::None,
            cancellation_token,
            submitted_at_us,
            deadline_at_us: None,
            queue_policy: QueuePolicy::default_for_domain(domain),
        }
    }

    pub fn with_freshness(mut self, freshness: JobFreshness) -> Self {
        self.freshness = freshness;
        self
    }

    pub fn with_queue_policy(mut self, queue_policy: QueuePolicy) -> Self {
        self.queue_policy = queue_policy;
        self
    }

    pub fn with_deadline_at_us(mut self, deadline_at_us: u64) -> Self {
        self.deadline_at_us = Some(deadline_at_us);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompletionFreshness {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playback_generation: Option<PlaybackGeneration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<u64>,
}

impl CompletionFreshness {
    pub const fn none() -> Self {
        Self {
            playback_generation: None,
            expected_revision: None,
        }
    }

    pub const fn playback_generation(playback_generation: PlaybackGeneration) -> Self {
        Self {
            playback_generation: Some(playback_generation),
            expected_revision: None,
        }
    }

    pub const fn with_expected_revision(mut self, expected_revision: u64) -> Self {
        self.expected_revision = Some(expected_revision);
        self
    }
}

impl Default for CompletionFreshness {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobDiagnosticClassification {
    RuntimeFallback,
    RuntimeUnavailable,
    ResourceUnavailable,
    UnsupportedMedia,
    CacheOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", deny_unknown_fields)]
pub enum JobResultKind {
    Completed,
    Failed,
    Fallback {
        classification: JobDiagnosticClassification,
    },
    Unavailable {
        classification: JobDiagnosticClassification,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobResult {
    pub job_id: JobId,
    pub kind: JobResultKind,
    pub cache_hit: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_frame_time_us: Option<u64>,
    pub dropped_frame_count: u64,
    pub repeated_frame_count: u64,
}

impl JobResult {
    pub fn new(job_id: JobId, kind: JobResultKind) -> Self {
        Self {
            job_id,
            kind,
            cache_hit: false,
            first_frame_time_us: None,
            dropped_frame_count: 0,
            repeated_frame_count: 0,
        }
    }

    pub fn completed(job_id: JobId) -> Self {
        Self::new(job_id, JobResultKind::Completed)
    }

    pub fn with_cache_hit(mut self, cache_hit: bool) -> Self {
        self.cache_hit = cache_hit;
        self
    }

    pub fn with_first_frame_time_us(mut self, first_frame_time_us: u64) -> Self {
        self.first_frame_time_us = Some(first_frame_time_us);
        self
    }

    pub fn with_dropped_frame_count(mut self, dropped_frame_count: u64) -> Self {
        self.dropped_frame_count = dropped_frame_count;
        self
    }

    pub fn with_repeated_frame_count(mut self, repeated_frame_count: u64) -> Self {
        self.repeated_frame_count = repeated_frame_count;
        self
    }
}

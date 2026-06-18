use draft_model::Microseconds;
use serde::{Deserialize, Serialize};

use crate::{PlaybackGeneration, PreviewRequestMode, RealtimePreviewFrameRequest};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewTelemetry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_frame_latency_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seek_latency_ms: Option<u64>,
    pub queue_latency_ms: u64,
    pub render_duration_ms: u64,
    pub presented_frame_count: u64,
    pub dropped_frame_count: u64,
    pub repeated_frame_count: u64,
    pub stale_rejected_count: u64,
    pub canceled_request_count: u64,
    pub fallback_count: u64,
    pub cache_hit_count: u64,
    pub target_time: Microseconds,
    pub generation: PlaybackGeneration,
}

impl RealtimePreviewTelemetry {
    pub fn new(target_time: Microseconds, generation: PlaybackGeneration) -> Self {
        Self {
            first_frame_latency_ms: None,
            seek_latency_ms: None,
            queue_latency_ms: 0,
            render_duration_ms: 0,
            presented_frame_count: 0,
            dropped_frame_count: 0,
            repeated_frame_count: 0,
            stale_rejected_count: 0,
            canceled_request_count: 0,
            fallback_count: 0,
            cache_hit_count: 0,
            target_time,
            generation,
        }
    }

    pub fn record_request(
        &mut self,
        request: &RealtimePreviewFrameRequest,
        presented: bool,
        stale_rejected: bool,
        canceled: bool,
    ) {
        self.target_time = request.target_time;
        self.generation = request.playback_generation;
        self.queue_latency_ms = request.queue_latency_ms;
        self.render_duration_ms = request.render_duration_ms;

        if request.repeated_frame {
            self.repeated_frame_count = self.repeated_frame_count.saturating_add(1);
        }
        if request.dropped_frame || stale_rejected {
            self.dropped_frame_count = self.dropped_frame_count.saturating_add(1);
        }
        if request.fallback_reason.is_some() {
            self.fallback_count = self.fallback_count.saturating_add(1);
        }
        if request.cache_hit {
            self.cache_hit_count = self.cache_hit_count.saturating_add(1);
        }
        if stale_rejected {
            self.stale_rejected_count = self.stale_rejected_count.saturating_add(1);
        }
        if canceled {
            self.canceled_request_count = self.canceled_request_count.saturating_add(1);
        }
        if presented {
            self.presented_frame_count = self.presented_frame_count.saturating_add(1);
            let latency = request
                .queue_latency_ms
                .saturating_add(request.render_duration_ms);
            if self.first_frame_latency_ms.is_none() {
                self.first_frame_latency_ms = Some(latency);
            }
            if matches!(
                request.mode,
                PreviewRequestMode::Seek | PreviewRequestMode::Scrub
            ) {
                self.seek_latency_ms = Some(latency);
            }
        }
    }
}

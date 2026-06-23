use draft_model::Microseconds;
use realtime_preview_runtime::PlaybackGeneration;
use serde::{Deserialize, Serialize};
use task_runtime::SchedulerTelemetrySnapshot;

use crate::session::AudioBufferRequest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioPreviewTelemetry {
    pub presented_buffer_count: u64,
    pub stale_rejected_count: u64,
    pub canceled_buffer_count: u64,
    pub underrun_count: u64,
    pub degraded_output_count: u64,
    pub bounded_rejected_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_queue_latency_p95_us: Option<u64>,
    pub scheduler_queue_depth: usize,
    pub scheduler_resource_saturation_count: u64,
    pub scheduler_rejected_count: u64,
    pub scheduler_canceled_count: u64,
    pub scheduler_stale_rejected_count: u64,
    pub target_time: Microseconds,
    pub generation: PlaybackGeneration,
}

impl AudioPreviewTelemetry {
    pub fn new(target_time: Microseconds, generation: PlaybackGeneration) -> Self {
        Self {
            presented_buffer_count: 0,
            stale_rejected_count: 0,
            canceled_buffer_count: 0,
            underrun_count: 0,
            degraded_output_count: 0,
            bounded_rejected_count: 0,
            scheduler_queue_latency_p95_us: None,
            scheduler_queue_depth: 0,
            scheduler_resource_saturation_count: 0,
            scheduler_rejected_count: 0,
            scheduler_canceled_count: 0,
            scheduler_stale_rejected_count: 0,
            target_time,
            generation,
        }
    }

    pub(crate) fn record_buffer(
        &mut self,
        request: &AudioBufferRequest,
        presented: bool,
        stale_rejected: bool,
        canceled: bool,
        bounded_rejected: bool,
    ) {
        self.target_time = request.target_time;
        self.generation = request.playback_generation;
        if presented {
            self.presented_buffer_count = self.presented_buffer_count.saturating_add(1);
        }
        if stale_rejected {
            self.stale_rejected_count = self.stale_rejected_count.saturating_add(1);
        }
        if canceled {
            self.canceled_buffer_count = self.canceled_buffer_count.saturating_add(1);
        }
        if bounded_rejected {
            self.bounded_rejected_count = self.bounded_rejected_count.saturating_add(1);
        }
    }

    pub fn record_scheduler_snapshot(&mut self, snapshot: &SchedulerTelemetrySnapshot) {
        self.scheduler_queue_latency_p95_us = snapshot.queue_latency_us.p95;
        self.scheduler_queue_depth = snapshot.current_queue_depth;
        self.scheduler_resource_saturation_count = snapshot.resource_saturation_count;
        self.scheduler_rejected_count = snapshot.rejected_count;
        self.scheduler_canceled_count = snapshot.canceled_count;
        self.scheduler_stale_rejected_count = snapshot.stale_rejected_count;
    }
}

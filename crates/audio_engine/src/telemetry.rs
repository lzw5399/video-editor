use draft_model::Microseconds;
use realtime_preview_runtime::PlaybackGeneration;
use serde::{Deserialize, Serialize};

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
}

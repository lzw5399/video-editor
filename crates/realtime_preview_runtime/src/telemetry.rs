use std::collections::VecDeque;

use draft_model::Microseconds;
use serde::{Deserialize, Serialize};

use crate::{PlaybackGeneration, PreviewRequestMode, RealtimePreviewFrameRequest};

const MAX_FRAME_PACING_SAMPLES: usize = 180;

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
    pub frame_pacing: RealtimePreviewFramePacingTelemetry,
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
            frame_pacing: RealtimePreviewFramePacingTelemetry::new(),
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

    pub fn record_presented_output(
        &mut self,
        target_time: Microseconds,
        generation: PlaybackGeneration,
        render_duration_ms: u64,
        dropped_frame_count: u64,
        pacing_sample: RealtimePreviewFramePacingSample,
    ) {
        self.target_time = target_time;
        self.generation = generation;
        self.queue_latency_ms = 0;
        self.render_duration_ms = render_duration_ms;
        self.presented_frame_count = self.presented_frame_count.saturating_add(1);
        self.dropped_frame_count = self.dropped_frame_count.saturating_add(dropped_frame_count);
        self.frame_pacing.record_sample(pacing_sample);
        if self.first_frame_latency_ms.is_none() {
            self.first_frame_latency_ms = Some(render_duration_ms);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewFramePacingTelemetry {
    pub sample_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_p50_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_p95_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_max_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_lateness_p95_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_lateness_max_ms: Option<u64>,
    pub samples: VecDeque<RealtimePreviewFramePacingSample>,
}

impl RealtimePreviewFramePacingTelemetry {
    pub fn new() -> Self {
        Self {
            sample_count: 0,
            interval_p50_ms: None,
            interval_p95_ms: None,
            interval_max_ms: None,
            schedule_lateness_p95_ms: None,
            schedule_lateness_max_ms: None,
            samples: VecDeque::new(),
        }
    }

    pub fn record_sample(&mut self, sample: RealtimePreviewFramePacingSample) {
        self.sample_count = self.sample_count.saturating_add(1);
        self.samples.push_back(sample);
        while self.samples.len() > MAX_FRAME_PACING_SAMPLES {
            self.samples.pop_front();
        }
        self.recompute_summary();
    }

    fn recompute_summary(&mut self) {
        let mut intervals = self
            .samples
            .iter()
            .filter_map(|sample| sample.interval_ms)
            .collect::<Vec<_>>();
        intervals.sort_unstable();
        self.interval_p50_ms = percentile(&intervals, 50);
        self.interval_p95_ms = percentile(&intervals, 95);
        self.interval_max_ms = intervals.last().copied();

        let mut lateness = self
            .samples
            .iter()
            .map(|sample| sample.schedule_lateness_ms)
            .collect::<Vec<_>>();
        lateness.sort_unstable();
        self.schedule_lateness_p95_ms = percentile(&lateness, 95);
        self.schedule_lateness_max_ms = lateness.last().copied();
    }
}

impl Default for RealtimePreviewFramePacingTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewFramePacingSample {
    pub target_time_microseconds: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_ms: Option<u64>,
    pub schedule_lateness_ms: u64,
    pub render_duration_ms: u64,
    pub dropped_frame_count: u64,
}

fn percentile(sorted_values: &[u64], percentile: usize) -> Option<u64> {
    if sorted_values.is_empty() {
        return None;
    }
    let last_index = sorted_values.len().saturating_sub(1);
    let index = last_index.saturating_mul(percentile).saturating_add(99) / 100;
    sorted_values.get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_records_frame_pacing_samples_and_summary() {
        let mut telemetry =
            RealtimePreviewTelemetry::new(Microseconds::ZERO, PlaybackGeneration::new(3));

        telemetry.record_presented_output(
            Microseconds::ZERO,
            PlaybackGeneration::new(3),
            5,
            0,
            RealtimePreviewFramePacingSample {
                target_time_microseconds: 0,
                interval_ms: None,
                schedule_lateness_ms: 0,
                render_duration_ms: 5,
                dropped_frame_count: 0,
            },
        );
        telemetry.record_presented_output(
            Microseconds::new(33_333),
            PlaybackGeneration::new(3),
            6,
            0,
            RealtimePreviewFramePacingSample {
                target_time_microseconds: 33_333,
                interval_ms: Some(34),
                schedule_lateness_ms: 3,
                render_duration_ms: 6,
                dropped_frame_count: 0,
            },
        );
        telemetry.record_presented_output(
            Microseconds::new(66_666),
            PlaybackGeneration::new(3),
            7,
            1,
            RealtimePreviewFramePacingSample {
                target_time_microseconds: 66_666,
                interval_ms: Some(48),
                schedule_lateness_ms: 9,
                render_duration_ms: 7,
                dropped_frame_count: 1,
            },
        );
        telemetry.record_presented_output(
            Microseconds::new(99_999),
            PlaybackGeneration::new(3),
            5,
            0,
            RealtimePreviewFramePacingSample {
                target_time_microseconds: 99_999,
                interval_ms: Some(33),
                schedule_lateness_ms: 1,
                render_duration_ms: 5,
                dropped_frame_count: 0,
            },
        );

        assert_eq!(telemetry.presented_frame_count, 4);
        assert_eq!(telemetry.dropped_frame_count, 1);
        assert_eq!(telemetry.frame_pacing.sample_count, 4);
        assert_eq!(telemetry.frame_pacing.samples.len(), 4);
        assert_eq!(telemetry.frame_pacing.interval_p50_ms, Some(34));
        assert_eq!(telemetry.frame_pacing.interval_p95_ms, Some(48));
        assert_eq!(telemetry.frame_pacing.interval_max_ms, Some(48));
        assert_eq!(telemetry.frame_pacing.schedule_lateness_p95_ms, Some(9));
        assert_eq!(telemetry.frame_pacing.schedule_lateness_max_ms, Some(9));
    }
}

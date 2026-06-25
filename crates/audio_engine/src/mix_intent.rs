use draft_model::{
    AudioRetimePolicy, MaterialId, Microseconds as TimelineTime, SegmentId, SegmentRetiming,
    SourceTimerange, TargetTimerange, TrackId,
};
use serde::{Deserialize, Serialize};

use crate::dsp_timeline::{
    DspEffectSlotClassification, DspFadeEnvelope, DspGainEnvelope, DspPanEnvelope,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioMixIntent {
    pub sample_rate_hz: u32,
    pub segments: Vec<AudioMixSegment>,
    pub summary: AudioMixSummary,
}

impl AudioMixIntent {
    pub(crate) fn new(sample_rate_hz: u32, segments: Vec<AudioMixSegment>) -> Self {
        let summary = AudioMixSummary::from_segments(&segments);
        Self {
            sample_rate_hz,
            segments,
            summary,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioMixSegment {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
    pub source_start_sample: u64,
    pub target_start_sample: u64,
    pub target_duration_samples: u64,
    pub retime: AudioRetimeMixIntent,
    pub gain_envelope: DspGainEnvelope,
    pub pan_envelope: DspPanEnvelope,
    pub fade_envelope: DspFadeEnvelope,
    pub effect_slots: Vec<DspEffectSlotClassification>,
    pub classification: AudioMixClassification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioRetimeMixIntent {
    pub retiming: SegmentRetiming,
    pub source_sample_map: AudioRetimeSourceSampleMap,
    pub policy: AudioRetimePolicy,
    pub follow_speed: bool,
    pub support: AudioRetimeMixSupport,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioRetimeSourceSampleMap {
    pub source_timerange: SourceTimerange,
    pub retimed_source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
    pub source_start_sample: u64,
    pub retimed_source_start_sample: u64,
    pub retimed_source_duration_samples: u64,
    pub target_start_sample: u64,
    pub target_duration_samples: u64,
    pub points: Vec<AudioRetimeSamplePoint>,
}

impl AudioRetimeSourceSampleMap {
    pub fn source_sample_at_target(&self, target_offset: TimelineTime) -> Option<u64> {
        self.points
            .iter()
            .find(|point| point.target_offset == target_offset)
            .map(|point| point.source_sample)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioRetimeSamplePoint {
    pub target_offset: TimelineTime,
    pub target_time: TimelineTime,
    pub target_sample: u64,
    pub source_time: TimelineTime,
    pub source_sample: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioRetimeMixSupport {
    Supported,
    Degraded,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioMixClassification {
    Audible,
    SilentMutedTrack,
    SilentZeroGain,
}

impl AudioMixClassification {
    pub(crate) fn is_silent(self) -> bool {
        matches!(self, Self::SilentMutedTrack | Self::SilentZeroGain)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioMixSummary {
    pub segment_count: u64,
    pub audible_segment_count: u64,
    pub silent_segment_count: u64,
    pub earliest_target_time: Option<TimelineTime>,
    pub latest_target_time: Option<TimelineTime>,
}

impl AudioMixSummary {
    fn from_segments(segments: &[AudioMixSegment]) -> Self {
        let mut earliest = None::<TimelineTime>;
        let mut latest = None::<TimelineTime>;
        let mut audible = 0_u64;
        let mut silent = 0_u64;

        for segment in segments {
            if segment.classification.is_silent() {
                silent = silent.saturating_add(1);
            } else {
                audible = audible.saturating_add(1);
            }
            earliest = Some(
                earliest
                    .map(|value| value.min(segment.target_timerange.start))
                    .unwrap_or(segment.target_timerange.start),
            );
            if let Some(end) = segment.target_timerange.checked_end() {
                latest = Some(latest.map(|value| value.max(end)).unwrap_or(end));
            }
        }

        Self {
            segment_count: segments.len() as u64,
            audible_segment_count: audible,
            silent_segment_count: silent,
            earliest_target_time: earliest,
            latest_target_time: latest,
        }
    }
}

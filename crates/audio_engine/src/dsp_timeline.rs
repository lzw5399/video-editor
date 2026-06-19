use std::error::Error;
use std::fmt;

use draft_model::{
    AudioEffectSlotKind, KeyframeProperty, KeyframeValue, MaterialId, Microseconds as TimelineTime,
    Segment, SegmentId, SourceTimerange, TargetTimerange, TrackId, TrackKind,
};
use serde::{Deserialize, Serialize};

use crate::mix_intent::{AudioMixClassification, AudioMixIntent, AudioMixSegment};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DspTimelineConfig {
    pub sample_rate_hz: u32,
}

impl DspTimelineConfig {
    pub const fn new(sample_rate_hz: u32) -> Self {
        Self { sample_rate_hz }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DspTimelinePlan {
    pub sample_rate_hz: u32,
    pub tracks: Vec<DspTrack>,
    pub mix_intent: AudioMixIntent,
    pub diagnostics: Vec<DspEvaluationDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DspTrack {
    pub track_id: TrackId,
    pub muted: bool,
    pub segments: Vec<DspSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DspSegment {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
    pub source_start_sample: u64,
    pub target_start_sample: u64,
    pub target_duration_samples: u64,
    pub gain_envelope: DspGainEnvelope,
    pub pan_envelope: DspPanEnvelope,
    pub fade_envelope: DspFadeEnvelope,
    pub effect_slots: Vec<DspEffectSlotClassification>,
    pub mix_classification: DspMixClassification,
}

impl DspSegment {
    fn to_mix_segment(&self) -> AudioMixSegment {
        AudioMixSegment {
            track_id: self.track_id.clone(),
            segment_id: self.segment_id.clone(),
            material_id: self.material_id.clone(),
            source_timerange: self.source_timerange.clone(),
            target_timerange: self.target_timerange.clone(),
            source_start_sample: self.source_start_sample,
            target_start_sample: self.target_start_sample,
            target_duration_samples: self.target_duration_samples,
            gain_envelope: self.gain_envelope.clone(),
            pan_envelope: self.pan_envelope.clone(),
            fade_envelope: self.fade_envelope.clone(),
            effect_slots: self.effect_slots.clone(),
            classification: self.mix_classification.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DspGainEnvelope {
    pub base_gain_millis: u32,
    pub points: Vec<DspGainPoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DspGainPoint {
    pub at: TimelineTime,
    pub target_sample: u64,
    pub gain_millis: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DspPanEnvelope {
    pub balance_millis: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DspFadeEnvelope {
    pub fade_in_duration: TimelineTime,
    pub fade_out_duration: TimelineTime,
    pub fade_in_sample_count: u64,
    pub fade_out_sample_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DspEffectSlotClassification {
    pub slot_id: String,
    pub enabled: bool,
    pub support: DspEffectSlotSupport,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DspEffectSlotSupport {
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DspMixClassification {
    Audible,
    SilentMutedTrack,
    SilentZeroGain,
}

impl From<DspMixClassification> for AudioMixClassification {
    fn from(value: DspMixClassification) -> Self {
        match value {
            DspMixClassification::Audible => Self::Audible,
            DspMixClassification::SilentMutedTrack => Self::SilentMutedTrack,
            DspMixClassification::SilentZeroGain => Self::SilentZeroGain,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DspEvaluationDiagnostic {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub target_time: TimelineTime,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DspTimelineError {
    InvalidSampleRate,
    SampleIndexOverflow,
}

impl fmt::Display for DspTimelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSampleRate => write!(formatter, "audio sample rate must be nonzero"),
            Self::SampleIndexOverflow => write!(formatter, "audio sample index overflowed"),
        }
    }
}

impl Error for DspTimelineError {}

pub fn evaluate_dsp_timeline(
    draft: &draft_model::Draft,
    config: DspTimelineConfig,
) -> Result<DspTimelinePlan, DspTimelineError> {
    if config.sample_rate_hz == 0 {
        return Err(DspTimelineError::InvalidSampleRate);
    }

    let mut tracks = Vec::new();
    let mut mix_segments = Vec::new();
    let mut diagnostics = Vec::new();

    for track in &draft.tracks {
        if track.kind != TrackKind::Audio {
            continue;
        }

        let mut segments = Vec::new();
        for segment in &track.segments {
            let dsp_segment = evaluate_segment(
                &track.track_id,
                track.muted,
                segment,
                config.sample_rate_hz,
                &mut diagnostics,
            )?;
            mix_segments.push(dsp_segment.to_mix_segment());
            segments.push(dsp_segment);
        }

        segments.sort_by(|first, second| {
            first
                .target_timerange
                .start
                .cmp(&second.target_timerange.start)
                .then_with(|| first.segment_id.cmp(&second.segment_id))
        });
        tracks.push(DspTrack {
            track_id: track.track_id.clone(),
            muted: track.muted,
            segments,
        });
    }

    tracks.sort_by(|first, second| first.track_id.cmp(&second.track_id));
    mix_segments.sort_by(|first, second| {
        first
            .target_timerange
            .start
            .cmp(&second.target_timerange.start)
            .then_with(|| first.track_id.cmp(&second.track_id))
            .then_with(|| first.segment_id.cmp(&second.segment_id))
    });

    Ok(DspTimelinePlan {
        sample_rate_hz: config.sample_rate_hz,
        tracks,
        mix_intent: AudioMixIntent::new(config.sample_rate_hz, mix_segments),
        diagnostics,
    })
}

fn evaluate_segment(
    track_id: &TrackId,
    track_muted: bool,
    segment: &Segment,
    sample_rate_hz: u32,
    diagnostics: &mut Vec<DspEvaluationDiagnostic>,
) -> Result<DspSegment, DspTimelineError> {
    let source_start_sample =
        timeline_time_to_samples(segment.source_timerange.start, sample_rate_hz)?;
    let target_start_sample =
        timeline_time_to_samples(segment.target_timerange.start, sample_rate_hz)?;
    let target_duration_samples =
        timeline_time_to_samples(segment.target_timerange.duration, sample_rate_hz)?;
    let gain_envelope = gain_envelope(segment, sample_rate_hz)?;
    let effect_slots = effect_slots(segment, track_id, diagnostics);
    let mix_classification = if track_muted {
        DspMixClassification::SilentMutedTrack
    } else if segment.audio.gain_millis == 0 {
        DspMixClassification::SilentZeroGain
    } else {
        DspMixClassification::Audible
    };

    Ok(DspSegment {
        track_id: track_id.clone(),
        segment_id: segment.segment_id.clone(),
        material_id: segment.material_id.clone(),
        source_timerange: segment.source_timerange.clone(),
        target_timerange: segment.target_timerange.clone(),
        source_start_sample,
        target_start_sample,
        target_duration_samples,
        gain_envelope,
        pan_envelope: DspPanEnvelope {
            balance_millis: segment.audio.pan_balance_millis.balance_millis,
        },
        fade_envelope: DspFadeEnvelope {
            fade_in_duration: segment.audio.fade_in_duration.duration,
            fade_out_duration: segment.audio.fade_out_duration.duration,
            fade_in_sample_count: timeline_time_to_samples(
                segment.audio.fade_in_duration.duration,
                sample_rate_hz,
            )?,
            fade_out_sample_count: timeline_time_to_samples(
                segment.audio.fade_out_duration.duration,
                sample_rate_hz,
            )?,
        },
        effect_slots,
        mix_classification,
    })
}

fn gain_envelope(
    segment: &Segment,
    sample_rate_hz: u32,
) -> Result<DspGainEnvelope, DspTimelineError> {
    let mut points = segment
        .keyframes
        .iter()
        .filter(|keyframe| keyframe.property == KeyframeProperty::Volume)
        .filter_map(|keyframe| match keyframe.value {
            KeyframeValue::Uint { value } => Some((keyframe.at, value)),
            _ => None,
        })
        .map(|(at, gain_millis)| {
            segment
                .target_timerange
                .start
                .get()
                .checked_add(at.get())
                .map(TimelineTime::new)
                .ok_or(DspTimelineError::SampleIndexOverflow)
                .and_then(|target_time| {
                    Ok(DspGainPoint {
                        at: target_time,
                        target_sample: timeline_time_to_samples(target_time, sample_rate_hz)?,
                        gain_millis,
                    })
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    points.sort_by_key(|point| (point.at, point.gain_millis));

    Ok(DspGainEnvelope {
        base_gain_millis: segment.audio.gain_millis,
        points,
    })
}

fn effect_slots(
    segment: &Segment,
    track_id: &TrackId,
    diagnostics: &mut Vec<DspEvaluationDiagnostic>,
) -> Vec<DspEffectSlotClassification> {
    segment
        .audio
        .effect_slots
        .iter()
        .map(|slot| match &slot.kind {
            AudioEffectSlotKind::Unsupported { name, external_ref } => {
                diagnostics.push(DspEvaluationDiagnostic {
                    track_id: track_id.clone(),
                    segment_id: segment.segment_id.clone(),
                    target_time: segment.target_timerange.start,
                    reason: format!("unsupported audio effect slot: {name}"),
                });
                DspEffectSlotClassification {
                    slot_id: slot.slot_id.clone(),
                    enabled: slot.enabled,
                    support: DspEffectSlotSupport::Unsupported,
                    name: name.clone(),
                    external_ref: external_ref.clone(),
                }
            }
        })
        .collect()
}

fn timeline_time_to_samples(
    value: TimelineTime,
    sample_rate_hz: u32,
) -> Result<u64, DspTimelineError> {
    let samples = u128::from(value.get())
        .checked_mul(u128::from(sample_rate_hz))
        .ok_or(DspTimelineError::SampleIndexOverflow)?
        / 1_000_000_u128;
    u64::try_from(samples).map_err(|_| DspTimelineError::SampleIndexOverflow)
}

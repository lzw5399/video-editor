use std::error::Error;
use std::fmt;

use draft_model::{
    AudioEffectSlotKind, AudioRetimePolicy, KeyframeProperty, KeyframeValue, MaterialId,
    Microseconds as TimelineTime, RetimeMode, Segment, SegmentId, SourceTimerange, TargetTimerange,
    TrackId, TrackKind,
};
use engine_core::{audio_retime_diagnostic, retimed_source_range, source_position_for_retime};
use serde::{Deserialize, Serialize};

use crate::mix_intent::{
    AudioMixClassification, AudioMixIntent, AudioMixSegment, AudioRetimeMixIntent,
    AudioRetimeMixSupport, AudioRetimeSamplePoint, AudioRetimeSourceSampleMap,
};

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
    pub retime: AudioRetimeMixIntent,
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
            retime: self.retime.clone(),
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
    pub source_sample: u64,
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
    pub kind: DspEvaluationDiagnosticKind,
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub target_time: TimelineTime,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DspEvaluationDiagnosticKind {
    UnsupportedAudioEffectSlot,
    UnsupportedPitchPreservation,
    DegradedSpeedCurveFollowSpeed,
    MutedUnsupportedRetime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DspTimelineError {
    InvalidSampleRate,
    SampleIndexOverflow,
    RetimeMappingFailed(String),
}

impl fmt::Display for DspTimelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSampleRate => write!(formatter, "audio sample rate must be nonzero"),
            Self::SampleIndexOverflow => write!(formatter, "audio sample index overflowed"),
            Self::RetimeMappingFailed(reason) => {
                write!(formatter, "audio retime mapping failed: {reason}")
            }
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
    let retime = retime_mix_intent(track_id, segment, sample_rate_hz, diagnostics)?;
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
        retime,
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
                    let source_time = source_position_for_retime(
                        &segment.source_timerange,
                        at,
                        &segment.retiming,
                    )
                    .map_err(|error| DspTimelineError::RetimeMappingFailed(error.to_string()))?;
                    Ok(DspGainPoint {
                        at: target_time,
                        target_sample: timeline_time_to_samples(target_time, sample_rate_hz)?,
                        source_sample: timeline_time_to_samples(source_time, sample_rate_hz)?,
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

fn retime_mix_intent(
    track_id: &TrackId,
    segment: &Segment,
    sample_rate_hz: u32,
    diagnostics: &mut Vec<DspEvaluationDiagnostic>,
) -> Result<AudioRetimeMixIntent, DspTimelineError> {
    let source_sample_map = retime_source_sample_map(segment, sample_rate_hz)?;
    let (follow_speed, support, reason) = retime_audio_support(track_id, segment, diagnostics);

    Ok(AudioRetimeMixIntent {
        retiming: segment.retiming.clone(),
        source_sample_map,
        policy: segment.retiming.audio_policy,
        follow_speed,
        support,
        reason,
    })
}

fn retime_source_sample_map(
    segment: &Segment,
    sample_rate_hz: u32,
) -> Result<AudioRetimeSourceSampleMap, DspTimelineError> {
    let retimed_source_timerange = retimed_source_range(
        &segment.source_timerange,
        segment.target_timerange.duration,
        &segment.retiming,
    )
    .map_err(|error| DspTimelineError::RetimeMappingFailed(error.to_string()))?;
    let points = retime_sample_points(segment, sample_rate_hz)?;

    Ok(AudioRetimeSourceSampleMap {
        source_timerange: segment.source_timerange.clone(),
        retimed_source_timerange: retimed_source_timerange.clone(),
        target_timerange: segment.target_timerange.clone(),
        source_start_sample: timeline_time_to_samples(
            segment.source_timerange.start,
            sample_rate_hz,
        )?,
        retimed_source_start_sample: timeline_time_to_samples(
            retimed_source_timerange.start,
            sample_rate_hz,
        )?,
        retimed_source_duration_samples: timeline_time_to_samples(
            retimed_source_timerange.duration,
            sample_rate_hz,
        )?,
        target_start_sample: timeline_time_to_samples(
            segment.target_timerange.start,
            sample_rate_hz,
        )?,
        target_duration_samples: timeline_time_to_samples(
            segment.target_timerange.duration,
            sample_rate_hz,
        )?,
        points,
    })
}

fn retime_sample_points(
    segment: &Segment,
    sample_rate_hz: u32,
) -> Result<Vec<AudioRetimeSamplePoint>, DspTimelineError> {
    let mut offsets = vec![
        TimelineTime::ZERO,
        TimelineTime::new(segment.target_timerange.duration.get() / 2),
        segment.target_timerange.duration,
    ];
    offsets.extend(
        segment
            .keyframes
            .iter()
            .filter(|keyframe| keyframe.at.get() <= segment.target_timerange.duration.get())
            .map(|keyframe| keyframe.at),
    );
    if let RetimeMode::SpeedCurve { points } = &segment.retiming.mode {
        offsets.extend(
            points
                .iter()
                .filter(|point| point.target_time.get() <= segment.target_timerange.duration.get())
                .map(|point| point.target_time),
        );
    }
    offsets.sort();
    offsets.dedup();

    offsets
        .into_iter()
        .map(|target_offset| {
            let target_time = add_timeline_time(segment.target_timerange.start, target_offset)?;
            let source_time = source_position_for_retime(
                &segment.source_timerange,
                target_offset,
                &segment.retiming,
            )
            .map_err(|error| DspTimelineError::RetimeMappingFailed(error.to_string()))?;
            Ok(AudioRetimeSamplePoint {
                target_offset,
                target_time,
                target_sample: timeline_time_to_samples(target_time, sample_rate_hz)?,
                source_time,
                source_sample: timeline_time_to_samples(source_time, sample_rate_hz)?,
            })
        })
        .collect()
}

fn retime_audio_support(
    track_id: &TrackId,
    segment: &Segment,
    diagnostics: &mut Vec<DspEvaluationDiagnostic>,
) -> (bool, AudioRetimeMixSupport, String) {
    if let Some(diagnostic) = audio_retime_diagnostic(&segment.retiming) {
        diagnostics.push(DspEvaluationDiagnostic {
            kind: DspEvaluationDiagnosticKind::UnsupportedPitchPreservation,
            track_id: track_id.clone(),
            segment_id: segment.segment_id.clone(),
            target_time: segment.target_timerange.start,
            reason: diagnostic.message.clone(),
        });
        return (
            false,
            AudioRetimeMixSupport::Unsupported,
            diagnostic.message,
        );
    }

    match segment.retiming.audio_policy {
        AudioRetimePolicy::FollowVideoSpeed => match &segment.retiming.mode {
            RetimeMode::Constant { .. } => (
                true,
                AudioRetimeMixSupport::Supported,
                "constant-speed audio follow-speed is represented in DSP mix intent".to_owned(),
            ),
            RetimeMode::SpeedCurve { .. } => {
                let reason = "speed curve audio follow-speed is typed in DSP but sample-accurate time-stretch output remains degraded".to_owned();
                diagnostics.push(DspEvaluationDiagnostic {
                    kind: DspEvaluationDiagnosticKind::DegradedSpeedCurveFollowSpeed,
                    track_id: track_id.clone(),
                    segment_id: segment.segment_id.clone(),
                    target_time: segment.target_timerange.start,
                    reason: reason.clone(),
                });
                (true, AudioRetimeMixSupport::Degraded, reason)
            }
        },
        AudioRetimePolicy::PreservePitch => (
            false,
            AudioRetimeMixSupport::Supported,
            "preserve-pitch audio retime is a no-op at 1x speed".to_owned(),
        ),
        AudioRetimePolicy::MuteUnsupported => {
            let reason =
                "audio will mute when retime follow-speed support is unavailable".to_owned();
            diagnostics.push(DspEvaluationDiagnostic {
                kind: DspEvaluationDiagnosticKind::MutedUnsupportedRetime,
                track_id: track_id.clone(),
                segment_id: segment.segment_id.clone(),
                target_time: segment.target_timerange.start,
                reason: reason.clone(),
            });
            (false, AudioRetimeMixSupport::Degraded, reason)
        }
    }
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
                    kind: DspEvaluationDiagnosticKind::UnsupportedAudioEffectSlot,
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

fn add_timeline_time(
    start: TimelineTime,
    offset: TimelineTime,
) -> Result<TimelineTime, DspTimelineError> {
    start
        .get()
        .checked_add(offset.get())
        .map(TimelineTime::new)
        .ok_or(DspTimelineError::SampleIndexOverflow)
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

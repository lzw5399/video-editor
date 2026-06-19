use std::collections::{BTreeMap, BTreeSet};

use audio_engine::{AudioMixClassification, AudioMixIntent, AudioMixSegment};
use draft_model::{MaterialId, Microseconds, SegmentId, SourceTimerange, TargetTimerange, TrackId};
use render_graph::{
    RenderAudioEffectSlotSupport, RenderAudioMix, RenderAudioMixClassification,
    RenderAudioVolumeKeyframe,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioPreviewExportParityDiagnostic {
    pub status: AudioMixParityStatus,
    pub preview_summary: AudioSampleSummary,
    pub export_summary: AudioSampleSummary,
    pub differences: Vec<AudioMixParityDifference>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioMixParityStatus {
    Match,
    Diverged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioSampleSummary {
    pub sample_rate_hz: u32,
    pub segment_count: u64,
    pub audible_segment_count: u64,
    pub silent_segment_count: u64,
    pub earliest_target_time: Option<Microseconds>,
    pub latest_target_time: Option<Microseconds>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioMixParityDifference {
    PreviewOnly {
        track_id: TrackId,
        segment_id: SegmentId,
        material_id: MaterialId,
    },
    UnsupportedPreviewOnly {
        track_id: TrackId,
        segment_id: SegmentId,
        material_id: MaterialId,
    },
    ExportOnly {
        track_id: TrackId,
        segment_id: SegmentId,
        material_id: MaterialId,
    },
    EffectSlotUnsupported {
        track_id: TrackId,
        segment_id: SegmentId,
        slot_id: String,
        name: String,
    },
    SampleRateMismatch {
        preview_sample_rate_hz: u32,
        export_sample_rate_hz: u32,
    },
    MissingMaterial {
        material_id: MaterialId,
    },
    MutedTrackSilence {
        track_id: TrackId,
        segment_id: SegmentId,
        material_id: MaterialId,
    },
    GainMismatch {
        track_id: TrackId,
        segment_id: SegmentId,
        preview_gain_millis: u32,
        export_gain_millis: u32,
    },
    PanMismatch {
        track_id: TrackId,
        segment_id: SegmentId,
        preview_pan_balance_millis: i32,
        export_pan_balance_millis: i32,
    },
    FadeMismatch {
        track_id: TrackId,
        segment_id: SegmentId,
        preview_fade_in_duration: Microseconds,
        export_fade_in_duration: Microseconds,
        preview_fade_out_duration: Microseconds,
        export_fade_out_duration: Microseconds,
    },
    VolumeKeyframeMismatch {
        track_id: TrackId,
        segment_id: SegmentId,
    },
    SourceRangeMismatch {
        track_id: TrackId,
        segment_id: SegmentId,
        preview_source_timerange: SourceTimerange,
        export_source_timerange: SourceTimerange,
    },
    TargetRangeMismatch {
        track_id: TrackId,
        segment_id: SegmentId,
        preview_target_timerange: TargetTimerange,
        export_target_timerange: TargetTimerange,
    },
}

pub fn audio_preview_export_parity_diagnostic(
    preview: &AudioMixIntent,
    export: &[RenderAudioMix],
    export_sample_rate_hz: u32,
    missing_material_ids: &[MaterialId],
) -> AudioPreviewExportParityDiagnostic {
    let mut differences = Vec::new();
    if preview.sample_rate_hz != export_sample_rate_hz {
        differences.push(AudioMixParityDifference::SampleRateMismatch {
            preview_sample_rate_hz: preview.sample_rate_hz,
            export_sample_rate_hz,
        });
    }

    for material_id in missing_material_ids {
        differences.push(AudioMixParityDifference::MissingMaterial {
            material_id: material_id.clone(),
        });
    }

    let preview_by_key = preview
        .segments
        .iter()
        .map(|segment| (segment_key(&segment.track_id, &segment.segment_id), segment))
        .collect::<BTreeMap<_, _>>();
    let export_by_key = export
        .iter()
        .map(|segment| (segment_key(&segment.track_id, &segment.segment_id), segment))
        .collect::<BTreeMap<_, _>>();

    for key in preview_by_key
        .keys()
        .chain(export_by_key.keys())
        .cloned()
        .collect::<BTreeSet<_>>()
    {
        match (preview_by_key.get(&key), export_by_key.get(&key)) {
            (Some(preview_segment), Some(export_segment)) => {
                compare_segment_pair(preview_segment, export_segment, &mut differences)
            }
            (Some(preview_segment), None) => {
                differences.push(AudioMixParityDifference::PreviewOnly {
                    track_id: preview_segment.track_id.clone(),
                    segment_id: preview_segment.segment_id.clone(),
                    material_id: preview_segment.material_id.clone(),
                });
                if preview_segment.effect_slots.iter().any(|slot| slot.enabled) {
                    differences.push(AudioMixParityDifference::UnsupportedPreviewOnly {
                        track_id: preview_segment.track_id.clone(),
                        segment_id: preview_segment.segment_id.clone(),
                        material_id: preview_segment.material_id.clone(),
                    });
                }
                if preview_segment.classification == AudioMixClassification::SilentMutedTrack {
                    differences.push(AudioMixParityDifference::MutedTrackSilence {
                        track_id: preview_segment.track_id.clone(),
                        segment_id: preview_segment.segment_id.clone(),
                        material_id: preview_segment.material_id.clone(),
                    });
                }
            }
            (None, Some(export_segment)) => {
                differences.push(AudioMixParityDifference::ExportOnly {
                    track_id: export_segment.track_id.clone(),
                    segment_id: export_segment.segment_id.clone(),
                    material_id: export_segment.material_id.clone(),
                });
            }
            (None, None) => {}
        }
    }

    let status = if differences.is_empty() {
        AudioMixParityStatus::Match
    } else {
        AudioMixParityStatus::Diverged
    };

    AudioPreviewExportParityDiagnostic {
        status,
        preview_summary: AudioSampleSummary::from_preview(preview),
        export_summary: AudioSampleSummary::from_export(export, export_sample_rate_hz),
        differences,
    }
}

impl AudioSampleSummary {
    fn from_preview(intent: &AudioMixIntent) -> Self {
        let mut summary = Self::empty(intent.sample_rate_hz);
        for segment in &intent.segments {
            summary.record_segment(
                segment.target_timerange.start,
                segment.target_timerange.checked_end(),
                !matches!(
                    segment.classification,
                    AudioMixClassification::SilentMutedTrack
                        | AudioMixClassification::SilentZeroGain
                ),
            );
        }
        summary
    }

    fn from_export(mixes: &[RenderAudioMix], sample_rate_hz: u32) -> Self {
        let mut summary = Self::empty(sample_rate_hz);
        for mix in mixes {
            summary.record_segment(
                mix.target_timerange.start,
                mix.target_timerange.checked_end(),
                mix.classification == RenderAudioMixClassification::Audible,
            );
        }
        summary
    }

    fn empty(sample_rate_hz: u32) -> Self {
        Self {
            sample_rate_hz,
            segment_count: 0,
            audible_segment_count: 0,
            silent_segment_count: 0,
            earliest_target_time: None,
            latest_target_time: None,
        }
    }

    fn record_segment(
        &mut self,
        target_start: Microseconds,
        target_end: Option<Microseconds>,
        audible: bool,
    ) {
        self.segment_count = self.segment_count.saturating_add(1);
        if audible {
            self.audible_segment_count = self.audible_segment_count.saturating_add(1);
        } else {
            self.silent_segment_count = self.silent_segment_count.saturating_add(1);
        }
        self.earliest_target_time = Some(
            self.earliest_target_time
                .map(|value| value.min(target_start))
                .unwrap_or(target_start),
        );
        if let Some(target_end) = target_end {
            self.latest_target_time = Some(
                self.latest_target_time
                    .map(|value| value.max(target_end))
                    .unwrap_or(target_end),
            );
        }
    }
}

fn compare_segment_pair(
    preview: &AudioMixSegment,
    export: &RenderAudioMix,
    differences: &mut Vec<AudioMixParityDifference>,
) {
    if preview.source_timerange != export.source_timerange {
        differences.push(AudioMixParityDifference::SourceRangeMismatch {
            track_id: preview.track_id.clone(),
            segment_id: preview.segment_id.clone(),
            preview_source_timerange: preview.source_timerange.clone(),
            export_source_timerange: export.source_timerange.clone(),
        });
    }
    if preview.target_timerange != export.target_timerange {
        differences.push(AudioMixParityDifference::TargetRangeMismatch {
            track_id: preview.track_id.clone(),
            segment_id: preview.segment_id.clone(),
            preview_target_timerange: preview.target_timerange.clone(),
            export_target_timerange: export.target_timerange.clone(),
        });
    }
    if preview.gain_envelope.base_gain_millis != export.gain_millis {
        differences.push(AudioMixParityDifference::GainMismatch {
            track_id: preview.track_id.clone(),
            segment_id: preview.segment_id.clone(),
            preview_gain_millis: preview.gain_envelope.base_gain_millis,
            export_gain_millis: export.gain_millis,
        });
    }
    if preview.pan_envelope.balance_millis != export.pan_balance_millis {
        differences.push(AudioMixParityDifference::PanMismatch {
            track_id: preview.track_id.clone(),
            segment_id: preview.segment_id.clone(),
            preview_pan_balance_millis: preview.pan_envelope.balance_millis,
            export_pan_balance_millis: export.pan_balance_millis,
        });
    }
    if preview.fade_envelope.fade_in_duration != export.fade_in_duration
        || preview.fade_envelope.fade_out_duration != export.fade_out_duration
    {
        differences.push(AudioMixParityDifference::FadeMismatch {
            track_id: preview.track_id.clone(),
            segment_id: preview.segment_id.clone(),
            preview_fade_in_duration: preview.fade_envelope.fade_in_duration,
            export_fade_in_duration: export.fade_in_duration,
            preview_fade_out_duration: preview.fade_envelope.fade_out_duration,
            export_fade_out_duration: export.fade_out_duration,
        });
    }
    if preview_volume_keyframes(preview) != export.volume_keyframes {
        differences.push(AudioMixParityDifference::VolumeKeyframeMismatch {
            track_id: preview.track_id.clone(),
            segment_id: preview.segment_id.clone(),
        });
    }
    for slot in &export.effect_slots {
        if slot.enabled && slot.support == RenderAudioEffectSlotSupport::Unsupported {
            differences.push(AudioMixParityDifference::EffectSlotUnsupported {
                track_id: export.track_id.clone(),
                segment_id: export.segment_id.clone(),
                slot_id: slot.slot_id.clone(),
                name: slot.name.clone(),
            });
        }
    }
    if preview.classification == AudioMixClassification::SilentMutedTrack {
        differences.push(AudioMixParityDifference::MutedTrackSilence {
            track_id: preview.track_id.clone(),
            segment_id: preview.segment_id.clone(),
            material_id: preview.material_id.clone(),
        });
    }
}

fn preview_volume_keyframes(preview: &AudioMixSegment) -> Vec<RenderAudioVolumeKeyframe> {
    preview
        .gain_envelope
        .points
        .iter()
        .map(|point| RenderAudioVolumeKeyframe {
            target_time: point.at,
            target_sample: 0,
            gain_millis: point.gain_millis,
        })
        .collect()
}

fn segment_key(track_id: &TrackId, segment_id: &SegmentId) -> (TrackId, SegmentId) {
    (track_id.clone(), segment_id.clone())
}

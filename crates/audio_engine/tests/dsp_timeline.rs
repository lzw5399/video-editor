use audio_engine::{
    AudioMixClassification, AudioRetimeMixSupport, DspEffectSlotSupport,
    DspEvaluationDiagnosticKind, DspMixClassification, DspTimelineConfig, evaluate_dsp_timeline,
};
use draft_model::{
    AudioEffectSlot, AudioEffectSlotKind, AudioFade, AudioPanBalance, AudioRetimePolicy, Draft,
    Keyframe, KeyframeEasing, KeyframeInterpolation, KeyframeProperty, KeyframeValue, Material,
    MaterialKind, Microseconds, RetimeMode, Segment, SegmentRetiming, SourceTimerange,
    SpeedCurvePoint, SpeedRatio, TargetTimerange, Track, TrackKind,
};

#[test]
fn dsp_timeline_evaluates_audio_tracks_with_integer_sample_rows() {
    let draft = audio_draft(false);

    let plan = evaluate_dsp_timeline(&draft, DspTimelineConfig::new(48_000)).unwrap();

    assert_eq!(plan.sample_rate_hz, 48_000);
    assert_eq!(plan.tracks.len(), 1);
    assert_eq!(plan.tracks[0].segments.len(), 1);
    let segment = &plan.tracks[0].segments[0];
    assert_eq!(segment.segment_id.as_str(), "segment-audio-001");
    assert_eq!(segment.target_timerange.start, Microseconds::new(250_000));
    assert_eq!(segment.target_start_sample, 12_000);
    assert_eq!(segment.target_duration_samples, 96_000);
    assert_eq!(segment.source_start_sample, 24_000);
    assert_eq!(segment.gain_envelope.base_gain_millis, 800);
    assert_eq!(segment.gain_envelope.points.len(), 3);
    assert_eq!(
        segment.gain_envelope.points[1].at,
        Microseconds::new(1_000_000)
    );
    assert_eq!(segment.gain_envelope.points[1].target_sample, 48_000);
    assert_eq!(segment.gain_envelope.points[1].source_sample, 60_000);
    assert_eq!(segment.gain_envelope.points[1].gain_millis, 1_200);
    assert_eq!(segment.pan_envelope.balance_millis, -250);
    assert_eq!(segment.fade_envelope.fade_in_sample_count, 12_000);
    assert_eq!(segment.fade_envelope.fade_out_sample_count, 24_000);
    assert_eq!(segment.mix_classification, DspMixClassification::Audible);
    assert_eq!(plan.mix_intent.segments.len(), 1);
    assert_eq!(
        plan.mix_intent.segments[0].classification,
        AudioMixClassification::Audible
    );
    assert_eq!(plan.mix_intent.summary.audible_segment_count, 1);
    assert_eq!(
        plan.mix_intent.segments[0]
            .retime
            .source_sample_map
            .retimed_source_duration_samples,
        96_000
    );
}

#[test]
fn dsp_timeline_muted_tracks_emit_silent_mix_segments_with_identity() {
    let draft = audio_draft(true);

    let plan = evaluate_dsp_timeline(&draft, DspTimelineConfig::new(48_000)).unwrap();

    let segment = &plan.tracks[0].segments[0];
    assert_eq!(segment.segment_id.as_str(), "segment-audio-001");
    assert_eq!(
        segment.mix_classification,
        DspMixClassification::SilentMutedTrack
    );
    assert_eq!(
        plan.mix_intent.segments[0].segment_id.as_str(),
        "segment-audio-001"
    );
    assert_eq!(
        plan.mix_intent.segments[0].classification,
        AudioMixClassification::SilentMutedTrack
    );
    assert_eq!(plan.mix_intent.summary.silent_segment_count, 1);
}

#[test]
fn dsp_timeline_carries_unsupported_effect_slots_without_changing_audio_math() {
    let draft = audio_draft(false);

    let plan = evaluate_dsp_timeline(&draft, DspTimelineConfig::new(48_000)).unwrap();

    let segment = &plan.tracks[0].segments[0];
    assert_eq!(segment.effect_slots.len(), 1);
    assert_eq!(segment.effect_slots[0].slot_id, "effect-slot-001");
    assert_eq!(
        segment.effect_slots[0].support,
        DspEffectSlotSupport::Unsupported
    );
    assert_eq!(segment.gain_envelope.base_gain_millis, 800);
    assert_eq!(segment.pan_envelope.balance_millis, -250);
    assert_eq!(segment.fade_envelope.fade_in_sample_count, 12_000);
}

fn audio_draft(track_muted: bool) -> Draft {
    let mut material = Material::new(
        "material-audio-001",
        MaterialKind::Audio,
        "media/bgm.wav",
        "BGM",
    );
    material.metadata.duration = Some(Microseconds::new(5_000_000));
    material.metadata.has_audio = true;
    material.metadata.audio_sample_rate = Some(48_000);
    material.metadata.audio_channels = Some(2);

    let mut segment = Segment::new(
        "segment-audio-001",
        material.material_id.clone(),
        SourceTimerange::new(500_000, 2_000_000),
        TargetTimerange::new(250_000, 2_000_000),
    );
    segment.audio.gain_millis = 800;
    segment.audio.pan_balance_millis = AudioPanBalance {
        balance_millis: -250,
    };
    segment.audio.fade_in_duration = AudioFade {
        duration: Microseconds::new(250_000),
    };
    segment.audio.fade_out_duration = AudioFade {
        duration: Microseconds::new(500_000),
    };
    segment.audio.effect_slots.push(AudioEffectSlot {
        slot_id: "effect-slot-001".to_owned(),
        kind: AudioEffectSlotKind::Unsupported {
            name: "external-space".to_owned(),
            external_ref: Some("jianying://effect/space".to_owned()),
        },
        enabled: true,
    });
    segment
        .keyframes
        .push(volume_keyframe(0, segment.audio.gain_millis));
    segment.keyframes.push(volume_keyframe(750_000, 1_200));
    segment.keyframes.push(volume_keyframe(1_500_000, 400));

    let mut track = Track::new("track-audio-001", TrackKind::Audio, "BGM");
    track.muted = track_muted;
    track.segments.push(segment);

    let mut draft = Draft::new("draft-audio-001", "Audio draft");
    draft.materials.push(material);
    draft.tracks.push(track);
    draft
}

fn volume_keyframe(at: u64, value: u32) -> Keyframe {
    Keyframe {
        at: Microseconds::new(at),
        property: KeyframeProperty::Volume,
        value: KeyframeValue::Uint { value },
        interpolation: KeyframeInterpolation::Linear,
        easing: KeyframeEasing::None,
    }
}

#[test]
fn phase19_audio_dsp_timeline_constant_speed_maps_source_samples_and_mix_intent() {
    let retiming = SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    };
    let draft = retimed_audio_draft(retiming.clone());

    let plan = evaluate_dsp_timeline(&draft, DspTimelineConfig::new(48_000)).unwrap();

    let segment = &plan.tracks[0].segments[0];
    assert_eq!(segment.retime.retiming, retiming);
    assert_eq!(segment.target_duration_samples, 72_000);
    assert_eq!(segment.retime.source_sample_map.source_start_sample, 24_000);
    assert_eq!(
        segment
            .retime
            .source_sample_map
            .retimed_source_duration_samples,
        144_000
    );
    assert_eq!(
        segment.retime.source_sample_map.points[1].source_sample, 96_000,
        "750ms target offset at 2x must map to 1.5s source offset"
    );
    assert_eq!(segment.gain_envelope.points[1].source_sample, 96_000);
    assert!(segment.retime.follow_speed);
    assert_eq!(segment.retime.support, AudioRetimeMixSupport::Supported);
    assert_eq!(plan.mix_intent.segments[0].retime, segment.retime);
}

#[test]
fn phase19_audio_dsp_timeline_speed_curve_maps_source_samples_without_floats() {
    let retiming = SegmentRetiming {
        mode: RetimeMode::SpeedCurve {
            points: vec![
                SpeedCurvePoint {
                    target_time: Microseconds::ZERO,
                    speed: SpeedRatio::new(1, 1),
                },
                SpeedCurvePoint {
                    target_time: Microseconds::new(1_000_000),
                    speed: SpeedRatio::new(2, 1),
                },
                SpeedCurvePoint {
                    target_time: Microseconds::new(2_000_000),
                    speed: SpeedRatio::new(1, 2),
                },
            ],
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    };
    let mut draft = retimed_audio_draft(retiming);
    draft.tracks[0].segments[0].target_timerange.duration = Microseconds::new(2_500_000);

    let plan = evaluate_dsp_timeline(&draft, DspTimelineConfig::new(48_000)).unwrap();

    let segment = &plan.tracks[0].segments[0];
    assert_eq!(
        segment
            .retime
            .source_sample_map
            .retimed_source_duration_samples,
        156_000,
        "2.5s target maps to 3.25s source under the integer speed curve"
    );
    assert_eq!(
        segment
            .retime
            .source_sample_map
            .source_sample_at_target(Microseconds::new(1_500_000)),
        Some(120_000),
        "1.5s target maps to absolute 2.5s source time under the curve"
    );
    assert_eq!(segment.gain_envelope.points[2].source_sample, 120_000);
    assert!(segment.retime.follow_speed);
    assert_eq!(segment.retime.support, AudioRetimeMixSupport::Degraded);
    assert!(
        segment.retime.reason.contains("speed curve"),
        "speed-curve follow-speed should carry explicit degradation reason"
    );
}

#[test]
fn phase19_audio_dsp_timeline_preserve_pitch_emits_typed_diagnostics() {
    let retiming = SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::PreservePitch,
    };
    let draft = retimed_audio_draft(retiming);

    let plan = evaluate_dsp_timeline(&draft, DspTimelineConfig::new(48_000)).unwrap();

    let segment = &plan.tracks[0].segments[0];
    assert!(!segment.retime.follow_speed);
    assert_eq!(segment.retime.support, AudioRetimeMixSupport::Unsupported);
    assert!(
        segment.retime.reason.contains("preserve-pitch"),
        "unsupported preserve-pitch retime should explain the degradation"
    );
    assert!(plan.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == DspEvaluationDiagnosticKind::UnsupportedPitchPreservation
            && diagnostic.segment_id.as_str() == "retimed-audio-segment"
    }));
    assert_eq!(plan.mix_intent.segments[0].retime, segment.retime);
}

fn retimed_audio_draft(retiming: SegmentRetiming) -> Draft {
    let mut material = Material::new(
        "retimed-audio-material",
        MaterialKind::Audio,
        "media/retimed-bgm.wav",
        "Retimed BGM",
    );
    material.metadata.duration = Some(Microseconds::new(6_000_000));
    material.metadata.has_audio = true;
    material.metadata.audio_sample_rate = Some(48_000);
    material.metadata.audio_channels = Some(2);

    let mut segment = Segment::new(
        "retimed-audio-segment",
        material.material_id.clone(),
        SourceTimerange::new(500_000, 4_000_000),
        TargetTimerange::new(250_000, 1_500_000),
    );
    segment.retiming = retiming;
    segment.keyframes.push(volume_keyframe(0, 1_000));
    segment.keyframes.push(volume_keyframe(750_000, 1_200));
    segment.keyframes.push(volume_keyframe(1_500_000, 800));

    let mut track = Track::new("retimed-audio-track", TrackKind::Audio, "Retimed audio");
    track.segments.push(segment);

    let mut draft = Draft::new("retimed-audio-draft", "Retimed audio draft");
    draft.materials.push(material);
    draft.tracks.push(track);
    draft
}

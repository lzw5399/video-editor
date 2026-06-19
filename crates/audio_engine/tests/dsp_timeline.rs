use audio_engine::{
    AudioMixClassification, DspEffectSlotSupport, DspMixClassification, DspTimelineConfig,
    evaluate_dsp_timeline,
};
use draft_model::{
    AudioEffectSlot, AudioEffectSlotKind, AudioFade, AudioPanBalance, Draft, Keyframe,
    KeyframeEasing, KeyframeInterpolation, KeyframeProperty, KeyframeValue, Material,
    MaterialKind, Microseconds, Segment, SourceTimerange, TargetTimerange, Track, TrackKind,
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
    assert_eq!(segment.gain_envelope.points[1].at, Microseconds::new(1_000_000));
    assert_eq!(segment.gain_envelope.points[1].target_sample, 60_000);
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
    assert_eq!(plan.mix_intent.segments[0].segment_id.as_str(), "segment-audio-001");
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
    assert_eq!(segment.effect_slots[0].support, DspEffectSlotSupport::Unsupported);
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

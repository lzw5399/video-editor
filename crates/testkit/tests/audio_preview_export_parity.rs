use draft_model::{
    AudioEffectSlot, AudioEffectSlotKind, AudioFade, AudioPanBalance, AudioRetimePolicy, Draft,
    Keyframe, KeyframeEasing, KeyframeInterpolation, KeyframeProperty, KeyframeValue, Material,
    MaterialId, MaterialKind, Microseconds, RetimeMode, Segment, SegmentRetiming, SourceTimerange,
    SpeedRatio, TargetTimerange, Track, TrackKind,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use render_graph::{RenderAudioMix, build_render_graph};
use testkit::{
    AudioMixParityDifference, AudioMixParityStatus, audio_preview_export_parity_diagnostic,
};

#[test]
fn audio_preview_export_parity_matches_gain_pan_fade_keyframes_and_ranges() {
    let draft = audio_parity_draft(false, false);
    let preview =
        audio_engine::evaluate_dsp_timeline(&draft, audio_engine::DspTimelineConfig::new(48_000))
            .expect("preview audio mix intent should evaluate")
            .mix_intent;
    let export = export_audio_mixes(&draft);

    let diagnostic = audio_preview_export_parity_diagnostic(
        &preview,
        &export,
        48_000,
        &Vec::<MaterialId>::new(),
    );

    assert_eq!(diagnostic.status, AudioMixParityStatus::Match);
    assert!(diagnostic.differences.is_empty());
    assert_eq!(diagnostic.preview_summary.sample_rate_hz, 48_000);
    assert_eq!(diagnostic.preview_summary.segment_count, 1);
    assert_eq!(diagnostic.export_summary.segment_count, 1);
    assert_eq!(diagnostic.preview_summary.audible_segment_count, 1);
    assert_eq!(diagnostic.export_summary.audible_segment_count, 1);
}

#[test]
fn audio_preview_export_parity_classifies_supported_difference_categories() {
    let draft = audio_parity_draft(false, true);
    let preview =
        audio_engine::evaluate_dsp_timeline(&draft, audio_engine::DspTimelineConfig::new(44_100))
            .expect("preview audio mix intent should evaluate")
            .mix_intent;
    let mut export = export_audio_mixes(&draft);
    export.push(extra_export_mix());

    let diagnostic = audio_preview_export_parity_diagnostic(
        &preview,
        &export,
        48_000,
        &[MaterialId::new("missing-audio-material")],
    );

    assert_eq!(diagnostic.status, AudioMixParityStatus::Diverged);
    assert!(
        diagnostic
            .differences
            .contains(&AudioMixParityDifference::SampleRateMismatch {
                preview_sample_rate_hz: 44_100,
                export_sample_rate_hz: 48_000,
            })
    );
    assert!(diagnostic.differences.iter().any(|difference| {
        matches!(difference, AudioMixParityDifference::EffectSlotUnsupported { slot_id, .. } if slot_id == "slot-vendor-space")
    }));
    assert!(diagnostic.differences.iter().any(|difference| {
        matches!(difference, AudioMixParityDifference::ExportOnly { segment_id, .. } if segment_id.as_str() == "export-only-segment")
    }));
    assert!(diagnostic.differences.iter().any(|difference| {
        matches!(difference, AudioMixParityDifference::MissingMaterial { material_id } if material_id.as_str() == "missing-audio-material")
    }));
}

#[test]
fn audio_preview_export_parity_classifies_preview_only_and_muted_track_silence_without_ffmpeg() {
    let muted = audio_parity_draft(true, false);
    let preview =
        audio_engine::evaluate_dsp_timeline(&muted, audio_engine::DspTimelineConfig::new(48_000))
            .expect("preview audio mix intent should evaluate")
            .mix_intent;

    let diagnostic =
        audio_preview_export_parity_diagnostic(&preview, &Vec::new(), 48_000, &Vec::new());

    assert_eq!(diagnostic.status, AudioMixParityStatus::Diverged);
    assert!(diagnostic.differences.iter().any(|difference| {
        matches!(difference, AudioMixParityDifference::PreviewOnly { segment_id, .. } if segment_id.as_str() == "audio-segment")
    }));
    assert!(diagnostic.differences.iter().any(|difference| {
        matches!(difference, AudioMixParityDifference::MutedTrackSilence { segment_id, .. } if segment_id.as_str() == "audio-segment")
    }));
}

#[test]
fn audio_preview_export_parity_classifies_retime_source_sample_mismatch() {
    let draft = retimed_audio_parity_draft(SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    });
    let preview =
        audio_engine::evaluate_dsp_timeline(&draft, audio_engine::DspTimelineConfig::new(48_000))
            .expect("preview audio mix intent should evaluate")
            .mix_intent;
    let mut export = export_audio_mixes(&draft);
    export[0]
        .retime
        .source_mapping
        .retimed_source_timerange
        .duration = Microseconds::new(1_000_000);

    let diagnostic = audio_preview_export_parity_diagnostic(
        &preview,
        &export,
        48_000,
        &Vec::<MaterialId>::new(),
    );

    assert_eq!(diagnostic.status, AudioMixParityStatus::Diverged);
    assert!(diagnostic.differences.iter().any(|difference| {
        matches!(
            difference,
            AudioMixParityDifference::RetimeSourceSampleMismatch { segment_id, .. }
                if segment_id.as_str() == "retimed-audio-segment"
        )
    }));
}

#[test]
fn audio_preview_export_parity_classifies_unsupported_audio_follow_speed() {
    let draft = retimed_audio_parity_draft(SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::PreservePitch,
    });
    let preview =
        audio_engine::evaluate_dsp_timeline(&draft, audio_engine::DspTimelineConfig::new(48_000))
            .expect("preview audio mix intent should evaluate")
            .mix_intent;
    let export = export_audio_mixes(&draft);

    let diagnostic = audio_preview_export_parity_diagnostic(
        &preview,
        &export,
        48_000,
        &Vec::<MaterialId>::new(),
    );

    assert_eq!(diagnostic.status, AudioMixParityStatus::Diverged);
    assert!(diagnostic.differences.iter().any(|difference| {
        matches!(
            difference,
            AudioMixParityDifference::UnsupportedAudioFollowSpeed { segment_id, .. }
                if segment_id.as_str() == "retimed-audio-segment"
        )
    }));
}

#[test]
fn audio_preview_export_parity_source_guard_requires_retime_audio_coverage() {
    const AUDIO_PARITY_RS: &str = include_str!("../src/audio_parity.rs");
    const PHASE19_SOURCE_GUARDS: &str = include_str!("../../../scripts/phase19-source-guards.sh");

    assert!(
        AUDIO_PARITY_RS.contains("RetimeSourceSampleMismatch")
            && AUDIO_PARITY_RS.contains("UnsupportedAudioFollowSpeed"),
        "audio parity must keep retime/source-sample and unsupported follow-speed diagnostics"
    );
    assert!(
        PHASE19_SOURCE_GUARDS.contains("require_retiming_audio_parity_coverage"),
        "retiming-audio source guard must require audio parity coverage"
    );
}

fn export_audio_mixes(draft: &Draft) -> Vec<RenderAudioMix> {
    let normalized = normalize_draft(draft, &EngineProfile::mvp_default()).expect("normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::new(500_000), Microseconds::new(250_000)),
    )
    .expect("range state");
    build_render_graph(&normalized, &range)
        .expect("render graph")
        .audio_mixes
}

fn retimed_audio_parity_draft(retiming: SegmentRetiming) -> Draft {
    let mut draft = audio_parity_draft(false, false);
    draft.draft_id = "retimed-audio-parity-draft".into();
    draft.materials[0].metadata.duration = Some(Microseconds::new(6_000_000));
    let segment = &mut draft.tracks[0].segments[0];
    segment.segment_id = "retimed-audio-segment".into();
    segment.source_timerange = SourceTimerange::new(500_000, 4_000_000);
    segment.target_timerange = TargetTimerange::new(250_000, 1_500_000);
    segment.retiming = retiming;
    segment.keyframes.clear();
    segment.keyframes.push(volume_keyframe(0, 750));
    segment.keyframes.push(volume_keyframe(750_000, 1_250));
    draft
}

fn audio_parity_draft(track_muted: bool, include_effect: bool) -> Draft {
    let mut material = Material::new(
        "audio-material",
        MaterialKind::Audio,
        "file:///media/audio.wav",
        "BGM",
    );
    material.metadata.duration = Some(Microseconds::new(2_000_000));
    material.metadata.has_audio = true;
    material.metadata.audio_sample_rate = Some(48_000);
    material.metadata.audio_channels = Some(2);

    let mut segment = Segment::new(
        "audio-segment",
        "audio-material",
        SourceTimerange::new(250_000, 1_000_000),
        TargetTimerange::new(500_000, 1_000_000),
    );
    segment.audio.gain_millis = 750;
    segment.audio.pan_balance_millis = AudioPanBalance {
        balance_millis: -400,
    };
    segment.audio.fade_in_duration = AudioFade {
        duration: Microseconds::new(100_000),
    };
    segment.audio.fade_out_duration = AudioFade {
        duration: Microseconds::new(200_000),
    };
    if include_effect {
        segment.audio.effect_slots.push(AudioEffectSlot {
            slot_id: "slot-vendor-space".to_owned(),
            enabled: true,
            kind: AudioEffectSlotKind::Unsupported {
                name: "vendor-space".to_owned(),
                external_ref: Some("jianying://audio/space".to_owned()),
            },
        });
    }
    segment.keyframes.push(volume_keyframe(0, 750));
    segment.keyframes.push(volume_keyframe(250_000, 1_250));

    let mut track = Track::new("audio-track", TrackKind::Audio, "音频");
    track.muted = track_muted;
    track.segments.push(segment);

    let mut draft = Draft::new("audio-parity-draft", "Audio parity");
    draft.materials.push(material);
    draft.tracks.push(track);
    draft
}

fn extra_export_mix() -> RenderAudioMix {
    let draft = audio_parity_draft(false, false);
    let mut export = export_audio_mixes(&draft);
    let mut mix = export.pop().expect("base mix");
    mix.segment_id = "export-only-segment".into();
    mix
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

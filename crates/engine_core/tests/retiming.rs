use draft_model::{
    AudioRetimePolicy, Draft, Filter, Material, MaterialKind, Microseconds, RetimeMode, Segment,
    SegmentRetiming, SourceTimerange, SpeedCurvePoint, SpeedRatio, TargetTimerange, Track,
    TrackKind, Transition,
};
use engine_core::{
    AudioRetimeDiagnosticKind, EngineProfile, audio_retime_diagnostic, normalize_draft,
    resolve_frame_state,
    time_mapping::{retimed_source_range, source_position_for_retime},
};

#[test]
fn phase19_retiming_engine_core_maps_constant_speed_with_integer_ratios() {
    let source = SourceTimerange::new(100_000, 4_000_000);
    let half_speed = SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(1, 2),
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    };
    let normal_speed = SegmentRetiming::default();
    let double_speed = SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    };

    assert_eq!(
        source_position_for_retime(&source, Microseconds::new(1_000_000), &half_speed)
            .expect("0.5x source position"),
        Microseconds::new(600_000)
    );
    assert_eq!(
        source_position_for_retime(&source, Microseconds::new(1_000_000), &normal_speed)
            .expect("1x source position"),
        Microseconds::new(1_100_000)
    );
    assert_eq!(
        source_position_for_retime(&source, Microseconds::new(1_000_000), &double_speed)
            .expect("2x source position"),
        Microseconds::new(2_100_000)
    );
    assert_eq!(
        retimed_source_range(&source, Microseconds::new(1_000_000), &double_speed)
            .expect("2x source range")
            .duration,
        Microseconds::new(2_000_000)
    );
}

#[test]
fn phase19_retiming_engine_core_maps_speed_curve_boundaries_and_middle_samples() {
    let source = SourceTimerange::new(0, 5_000_000);
    let curve = SegmentRetiming {
        mode: RetimeMode::SpeedCurve {
            points: vec![
                SpeedCurvePoint {
                    target_time: Microseconds::new(0),
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

    assert_eq!(
        source_position_for_retime(&source, Microseconds::new(0), &curve).expect("curve start"),
        Microseconds::new(0)
    );
    assert_eq!(
        source_position_for_retime(&source, Microseconds::new(500_000), &curve)
            .expect("curve first span middle"),
        Microseconds::new(500_000)
    );
    assert_eq!(
        source_position_for_retime(&source, Microseconds::new(1_000_000), &curve)
            .expect("curve second span start"),
        Microseconds::new(1_000_000)
    );
    assert_eq!(
        source_position_for_retime(&source, Microseconds::new(1_500_000), &curve)
            .expect("curve second span middle"),
        Microseconds::new(2_000_000)
    );
    assert_eq!(
        source_position_for_retime(&source, Microseconds::new(2_500_000), &curve)
            .expect("curve third span middle"),
        Microseconds::new(3_250_000)
    );
}

#[test]
fn phase19_retiming_frame_state_carries_retime_transition_effect_and_audio_diagnostics() {
    let mut draft = retimed_draft();
    let video = &mut draft.tracks[0].segments[0];
    let video_retiming = SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    };
    video.retiming = video_retiming.clone();
    video.filters.push(Filter::gaussian_blur(250));
    video.transition = Some(Transition::dissolve(Microseconds::new(120_000)));
    let expected_filters = video.filters.clone();
    let expected_transition = video.transition.clone();

    let audio = &mut draft.tracks[1].segments[0];
    let audio_retiming = SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::PreservePitch,
    };
    audio.retiming = audio_retiming;

    let normalized = normalize_draft(&draft, &EngineProfile::mvp_default())
        .expect("retimed draft should normalize");
    let frame = resolve_frame_state(&normalized, Microseconds::new(500_000))
        .expect("retimed frame should resolve");

    let layer = &frame.visual_layers[0];
    assert_eq!(layer.source_position, Microseconds::new(1_000_000));
    assert_eq!(layer.retiming, video_retiming);
    assert_eq!(layer.filters, expected_filters);
    assert_eq!(layer.transition, expected_transition);

    let audio = &frame.audio_segments[0];
    assert_eq!(audio.source_position, Microseconds::new(1_000_000));
    assert_eq!(
        audio.retiming.audio_policy,
        AudioRetimePolicy::PreservePitch
    );
    assert_eq!(
        audio
            .audio_retime_diagnostic
            .as_ref()
            .expect("unsupported preserve pitch diagnostic")
            .kind,
        AudioRetimeDiagnosticKind::UnsupportedPitchPreservation
    );

    let diagnostic = audio_retime_diagnostic(&audio.retiming)
        .expect("direct audio retime diagnostic should match frame state");
    assert_eq!(
        diagnostic.kind,
        AudioRetimeDiagnosticKind::UnsupportedPitchPreservation
    );
}

fn retimed_draft() -> Draft {
    let mut draft = Draft::new("engine-retime-draft", "Engine Retime Draft");
    let mut video_material = Material::new(
        "video-material",
        MaterialKind::Video,
        "media/video.mp4",
        "video.mp4",
    );
    video_material.metadata.duration = Some(Microseconds::new(4_000_000));
    draft.materials.push(video_material);
    let mut audio_material = Material::new(
        "audio-material",
        MaterialKind::Audio,
        "media/audio.wav",
        "audio.wav",
    );
    audio_material.metadata.duration = Some(Microseconds::new(4_000_000));
    draft.materials.push(audio_material);

    let mut video_track = Track::new("video-track", TrackKind::Video, "Video");
    video_track.segments.push(Segment::new(
        "video-a",
        "video-material",
        SourceTimerange::new(0, 4_000_000),
        TargetTimerange::new(0, 2_000_000),
    ));
    draft.tracks.push(video_track);

    let mut audio_track = Track::new("audio-track", TrackKind::Audio, "Audio");
    audio_track.segments.push(Segment::new(
        "audio-a",
        "audio-material",
        SourceTimerange::new(0, 4_000_000),
        TargetTimerange::new(0, 2_000_000),
    ));
    draft.tracks.push(audio_track);
    draft
}

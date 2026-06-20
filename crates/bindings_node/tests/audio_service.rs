use bindings_node::audio_service::{
    AudioPreviewBindingErrorKind, AudioPreviewBindingRegistry, AudioPreviewSessionBindingConfig,
};
use draft_model::{AudioPreviewPlaybackStatus, Microseconds, WaveformDisplayStatus};
use realtime_preview_runtime::{PlaybackGeneration, RealtimePreviewAudioSyncState};

fn test_config() -> AudioPreviewSessionBindingConfig {
    AudioPreviewSessionBindingConfig {
        session_label: "audio-main".to_owned(),
        frame_rate_numerator: 30,
        frame_rate_denominator: 1,
        playback_rate_numerator: 1,
        playback_rate_denominator: 1,
        sample_rate_hz: 48_000,
        max_buffer_duration_microseconds: Microseconds::new(50_000),
        max_channel_count: 2,
        max_frame_count: 2_400,
    }
}

#[test]
fn audio_service_creates_opaque_session_and_classifies_bad_ids() {
    let mut registry = AudioPreviewBindingRegistry::new();
    let created = registry
        .create_session(test_config())
        .expect("audio session should be created");

    assert!(created.session_id.starts_with("audio-session-"));
    assert_eq!(created.generation, 0);
    assert_eq!(created.status, AudioPreviewPlaybackStatus::Ready);

    let malformed = registry
        .status("audio-session-not-hex")
        .expect_err("malformed binding ID should be rejected");
    assert_eq!(
        malformed.kind(),
        AudioPreviewBindingErrorKind::MalformedSessionId
    );

    let unknown = registry
        .status("audio-session-000000000000ffff")
        .expect_err("unknown binding ID should be rejected");
    assert_eq!(unknown.kind(), AudioPreviewBindingErrorKind::UnknownSession);
}

#[test]
fn audio_service_maps_play_pause_stop_seek_cancel_and_stale_generation() {
    let mut registry = AudioPreviewBindingRegistry::new();
    let created = registry
        .create_session(test_config())
        .expect("audio session should be created");

    let play_without_draft = registry
        .play(
            &created.session_id,
            None,
            Microseconds::new(0),
            created.generation,
        )
        .expect_err("audio play must fail closed without draft PCM");
    assert_eq!(
        play_without_draft.kind(),
        AudioPreviewBindingErrorKind::InvalidPayload
    );

    let status_after_failed_play = registry
        .status(&created.session_id)
        .expect("status should remain readable after failed play");
    assert_eq!(status_after_failed_play.target_time, Microseconds::new(0));

    let sought = registry
        .seek(&created.session_id, Microseconds::new(500_000))
        .expect("audio seek should advance generation");
    assert!(sought.generation > created.generation);
    assert_eq!(sought.target_time, Microseconds::new(500_000));

    let stale = registry
        .play(
            &created.session_id,
            None,
            Microseconds::new(600_000),
            created.generation,
        )
        .expect("stale audio play should return classified response");
    assert!(!stale.accepted);
    assert_eq!(stale.status, AudioPreviewPlaybackStatus::StaleRejected);
    assert!(
        stale
            .diagnostics
            .iter()
            .any(|message| message.contains("stale"))
    );

    let current = registry
        .status(&created.session_id)
        .expect("current generation should be readable");
    let fresh_without_draft = registry
        .play(
            &created.session_id,
            None,
            Microseconds::new(600_000),
            current.generation,
        )
        .expect_err("fresh generation play still requires draft PCM");
    assert_eq!(
        fresh_without_draft.kind(),
        AudioPreviewBindingErrorKind::InvalidPayload
    );
    let status_after_seek = registry
        .status(&created.session_id)
        .expect("failed play must not clear seek target time");
    assert_eq!(
        status_after_seek.target_time,
        Microseconds::new(500_000)
    );

    let paused = registry
        .pause(&created.session_id)
        .expect("audio pause should be accepted");
    assert_eq!(paused.status, AudioPreviewPlaybackStatus::Paused);

    let canceled = registry
        .cancel(&created.session_id)
        .expect("audio cancel should be accepted");
    assert_eq!(canceled.status, AudioPreviewPlaybackStatus::Canceled);

    let stopped = registry
        .stop(&created.session_id)
        .expect("audio stop should be accepted");
    assert_eq!(stopped.status, AudioPreviewPlaybackStatus::Stopped);
}

#[test]
fn audio_service_status_can_seed_realtime_preview_sync_state() {
    let mut registry = AudioPreviewBindingRegistry::new();
    let created = registry
        .create_session(test_config())
        .expect("audio session should be created");
    let sought = registry
        .seek(&created.session_id, Microseconds::new(900_000))
        .expect("audio seek should be accepted");
    let status = registry
        .status(&created.session_id)
        .expect("audio status should be readable");

    let sync = RealtimePreviewAudioSyncState {
        session_id: status.session_id.clone(),
        playback_generation: PlaybackGeneration::new(status.generation),
        target_time: status.target_time,
        buffered_until: status.buffered_until,
        status: status.status,
        diagnostics: status.diagnostics.clone(),
    };

    assert!(sought.accepted);
    assert_eq!(sync.session_id, created.session_id);
    assert_eq!(
        sync.playback_generation,
        PlaybackGeneration::new(status.generation)
    );
    assert_eq!(sync.target_time, Microseconds::new(900_000));
    assert_eq!(sync.buffered_until, Microseconds::new(900_000));
    assert_eq!(sync.status, AudioPreviewPlaybackStatus::Ready);

    let serialized = serde_json::to_string(&sync).expect("audio sync state serializes");
    for forbidden in [
        "nativeHandle",
        "rawBuffer",
        "audioDeviceHandle",
        "sampleData",
        "ffmpegFilter",
        "cacheKey",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "audio sync state must not expose {forbidden}"
        );
    }
}

#[test]
fn audio_service_returns_safe_devices_and_bounded_waveform_payloads() {
    let mut registry = AudioPreviewBindingRegistry::new();
    let created = registry
        .create_session(test_config())
        .expect("audio session should be created");

    let devices = registry
        .list_output_devices()
        .expect("device summaries should be returned");
    assert!(!devices.is_empty());
    assert!(devices.iter().all(|device| !device.selection_id.is_empty()));
    assert!(devices.iter().all(|device| !device.display_name.is_empty()));

    let selected = registry
        .select_output_device(&created.session_id, &devices[0].selection_id)
        .expect("safe output device selection should be accepted");
    assert!(selected.accepted);
    assert_eq!(selected.session_id, created.session_id);

    let waveform = registry
        .waveform_display_peaks(
            Some("material-audio".to_owned()),
            Some(draft_model::TargetTimerange {
                start: Microseconds::new(0),
                duration: Microseconds::new(1_000_000),
            }),
            16,
        )
        .expect("waveform display payload should be returned");
    assert_eq!(waveform.status, WaveformDisplayStatus::Missing);
    assert_eq!(waveform.requested_peak_bins, 16);
    assert_eq!(waveform.returned_peak_bins, 0);
    assert!(waveform.peaks.is_empty());
    assert!(
        waveform
            .diagnostics
            .iter()
            .any(|message| { message.contains("ready waveform artifact") })
    );

    let serialized = serde_json::to_string(&waveform).expect("waveform should serialize");
    for forbidden in [
        "nativeHandle",
        "rawBuffer",
        "artifactRoot",
        "blobPath",
        "SQLite",
        "fingerprint",
        "dirtyRange",
        "cacheKey",
        "ffmpegFilter",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "audio waveform response must not expose {forbidden}"
        );
    }
}

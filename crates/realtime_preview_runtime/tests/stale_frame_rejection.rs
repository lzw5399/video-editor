use draft_model::{AudioPreviewPlaybackStatus, Microseconds, RationalFrameRate};
use realtime_preview_runtime::{
    PlaybackGeneration, PlaybackRate, PreviewGpuBackend, PreviewRequestMode,
    RealtimePreviewAudioSyncState, RealtimePreviewFallbackReason, RealtimePreviewRuntime,
    RealtimePreviewSessionConfig,
};

fn test_runtime() -> (
    RealtimePreviewRuntime,
    realtime_preview_runtime::PreviewSessionId,
) {
    let mut runtime = RealtimePreviewRuntime::new();
    let session_id = runtime
        .create_session(RealtimePreviewSessionConfig {
            session_label: "stale-test".to_owned(),
            preferred_backend: PreviewGpuBackend::Mock,
            frame_rate: RationalFrameRate::new(30, 1),
            playback_rate: PlaybackRate::normal(),
        })
        .expect("session created");
    (runtime, session_id)
}

#[test]
fn stale_frame_rejection_current_generation_presents_and_stale_generation_is_rejected() {
    let (mut runtime, session_id) = test_runtime();
    let generation = runtime
        .seek(session_id, Microseconds::new(1_000_000))
        .expect("seek advances generation");

    let presented = runtime
        .request_frame(
            session_id,
            realtime_preview_runtime::RealtimePreviewFrameRequest {
                target_time: Microseconds::new(1_000_000),
                playback_generation: generation,
                audio_sync: None,
                cancellation_token: None,
                mode: PreviewRequestMode::Seek,
                queue_latency_ms: 2,
                render_duration_ms: 5,
                fallback_reason: None,
                cache_hit: true,
                repeated_frame: false,
                dropped_frame: false,
            },
        )
        .expect("current generation request succeeds");

    assert!(presented.presented);
    assert!(!presented.stale_rejected);
    assert!(!presented.canceled);
    assert_eq!(presented.telemetry.presented_frame_count, 1);
    assert_eq!(presented.telemetry.cache_hit_count, 1);
    assert_eq!(presented.telemetry.seek_latency_ms, Some(7));

    runtime
        .seek(session_id, Microseconds::new(2_000_000))
        .expect("second seek makes first generation stale");

    let stale = runtime
        .request_frame(
            session_id,
            realtime_preview_runtime::RealtimePreviewFrameRequest {
                target_time: Microseconds::new(1_000_000),
                playback_generation: generation,
                audio_sync: None,
                cancellation_token: None,
                mode: PreviewRequestMode::PlaybackTick,
                queue_latency_ms: 3,
                render_duration_ms: 4,
                fallback_reason: None,
                cache_hit: false,
                repeated_frame: false,
                dropped_frame: false,
            },
        )
        .expect("stale request returns classified result");

    assert!(!stale.presented);
    assert!(stale.stale_rejected);
    assert_eq!(stale.telemetry.presented_frame_count, 1);
    assert_eq!(stale.telemetry.stale_rejected_count, 1);
    assert_eq!(stale.telemetry.dropped_frame_count, 1);
    assert_eq!(stale.diagnostics[0].fallback_used, false);
    assert_eq!(stale.diagnostics[0].canceled, false);
}

#[test]
fn stale_frame_rejection_audio_sync_generation_mismatch_is_rejected() {
    let (mut runtime, session_id) = test_runtime();
    let generation = runtime
        .seek(session_id, Microseconds::new(3_000_000))
        .expect("seek advances generation");

    let result = runtime
        .request_frame(
            session_id,
            realtime_preview_runtime::RealtimePreviewFrameRequest {
                target_time: Microseconds::new(3_000_000),
                playback_generation: generation,
                audio_sync: Some(RealtimePreviewAudioSyncState {
                    session_id: "audio-session-0000000000000001".to_owned(),
                    playback_generation: PlaybackGeneration::initial(),
                    target_time: Microseconds::new(3_000_000),
                    buffered_until: Microseconds::new(3_000_000),
                    status: AudioPreviewPlaybackStatus::Playing,
                    diagnostics: Vec::new(),
                }),
                cancellation_token: None,
                mode: PreviewRequestMode::PlaybackTick,
                queue_latency_ms: 1,
                render_duration_ms: 2,
                fallback_reason: None,
                cache_hit: false,
                repeated_frame: false,
                dropped_frame: false,
            },
        )
        .expect("audio sync stale request returns classified result");

    assert!(!result.presented);
    assert!(result.stale_rejected);
    assert_eq!(
        result.fallback,
        Some(RealtimePreviewFallbackReason::StaleGeneration)
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.reason.contains("audio generation")),
        "diagnostics should name the audio sync mismatch: {:?}",
        result.diagnostics
    );
}

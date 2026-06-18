use draft_model::{Microseconds, RationalFrameRate};
use realtime_preview_runtime::{
    PlaybackRate, PreviewGpuBackend, PreviewRequestMode, RealtimePreviewRuntime,
    RealtimePreviewSessionConfig,
};

fn test_runtime() -> (RealtimePreviewRuntime, realtime_preview_runtime::PreviewSessionId) {
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

    let stale = runtime
        .request_frame(
            session_id,
            realtime_preview_runtime::RealtimePreviewFrameRequest {
                target_time: Microseconds::new(1_000_000),
                playback_generation: generation,
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

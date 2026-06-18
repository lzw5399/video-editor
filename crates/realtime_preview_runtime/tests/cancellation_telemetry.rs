use draft_model::{Microseconds, RationalFrameRate};
use realtime_preview_runtime::{
    PlaybackRate, PreviewGpuBackend, PreviewRequestMode, RealtimePreviewDiagnostic,
    RealtimePreviewDiagnosticDomain, RealtimePreviewFallbackReason, RealtimePreviewRuntime,
    RealtimePreviewSessionConfig, RealtimePreviewSupport,
};

#[test]
fn cancellation_telemetry_canceled_current_generation_is_not_presented() {
    let mut runtime = RealtimePreviewRuntime::new();
    let session_id = runtime
        .create_session(RealtimePreviewSessionConfig {
            session_label: "cancel-test".to_owned(),
            preferred_backend: PreviewGpuBackend::Mock,
            frame_rate: RationalFrameRate::new(24, 1),
            playback_rate: PlaybackRate::normal(),
        })
        .expect("session created");
    let generation = runtime
        .seek(session_id, Microseconds::new(2_000_000))
        .expect("seek advances generation");
    let token = runtime
        .next_cancellation_token(session_id)
        .expect("token allocated");
    runtime
        .cancel_request(session_id, token)
        .expect("token canceled");

    let result = runtime
        .request_frame(
            session_id,
            realtime_preview_runtime::RealtimePreviewFrameRequest {
                target_time: Microseconds::new(2_000_000),
                playback_generation: generation,
                cancellation_token: Some(token),
                mode: PreviewRequestMode::Scrub,
                queue_latency_ms: 8,
                render_duration_ms: 13,
                fallback_reason: Some(RealtimePreviewFallbackReason::SurfaceUnavailable),
                cache_hit: false,
                repeated_frame: true,
                dropped_frame: false,
            },
        )
        .expect("canceled request returns result");

    assert!(!result.presented);
    assert!(!result.stale_rejected);
    assert!(result.canceled);
    assert_eq!(result.cancellation_token, Some(token));
    assert_eq!(result.telemetry.canceled_request_count, 1);
    assert_eq!(result.telemetry.presented_frame_count, 0);
    assert_eq!(result.telemetry.repeated_frame_count, 1);
    assert_eq!(result.telemetry.fallback_count, 1);
    assert_eq!(result.telemetry.queue_latency_ms, 8);
    assert_eq!(result.telemetry.render_duration_ms, 13);
    assert_eq!(result.telemetry.first_frame_latency_ms, None);
    assert_eq!(result.diagnostics[0].fallback_used, true);
    assert_eq!(result.diagnostics[0].canceled, true);
}

#[test]
fn cancellation_telemetry_fallback_reasons_and_diagnostics_serialize_for_bindings() {
    let fallback_json =
        serde_json::to_string(&RealtimePreviewFallbackReason::UnsupportedGraphIntent)
            .expect("fallback serializes");
    assert_eq!(fallback_json, "\"unsupportedGraphIntent\"");

    let diagnostic = RealtimePreviewDiagnostic {
        entity_id: Some("segment-1".to_owned()),
        domain: RealtimePreviewDiagnosticDomain::VisualLayer,
        support: RealtimePreviewSupport::Unsupported {
            reason: "mask unsupported".to_owned(),
        },
        reason: "visual layer requires fallback".to_owned(),
        fallback: Some(RealtimePreviewFallbackReason::UnsupportedGraphIntent),
        fallback_used: true,
        canceled: false,
        cancellation_token: None,
    };

    let json = serde_json::to_value(&diagnostic).expect("diagnostic serializes");
    assert_eq!(json["domain"], "visualLayer");
    assert_eq!(json["support"]["unsupported"]["reason"], "mask unsupported");
    assert_eq!(json["fallback"], "unsupportedGraphIntent");
    assert_eq!(json["fallbackUsed"], true);
    assert_eq!(json["canceled"], false);
}

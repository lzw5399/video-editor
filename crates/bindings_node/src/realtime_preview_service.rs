#[cfg(test)]
mod realtime_preview_bindings {
    use super::{
        RealtimePreviewBindingErrorKind, RealtimePreviewBindingRegistry,
        RealtimePreviewFrameBindingRequest, RealtimePreviewSessionBindingConfig,
        RealtimePreviewSurfaceBindingDescriptor, RealtimePreviewSurfaceBindingKind,
    };

    fn registry_with_session() -> (RealtimePreviewBindingRegistry, String) {
        let mut registry = RealtimePreviewBindingRegistry::new();
        let created = registry
            .create_session(RealtimePreviewSessionBindingConfig {
                session_label: "preview-main".to_owned(),
                frame_rate_numerator: 30,
                frame_rate_denominator: 1,
                playback_rate_numerator: 1,
                playback_rate_denominator: 1,
            })
            .expect("session is created");
        (registry, created.session_id)
    }

    #[test]
    fn malformed_session_ids_fail_safely() {
        let mut registry = RealtimePreviewBindingRegistry::new();
        let error = registry
            .close_session("not-a-runtime-session")
            .expect_err("malformed ids are rejected");

        assert_eq!(error.kind(), RealtimePreviewBindingErrorKind::MalformedSessionId);
    }

    #[test]
    fn surface_validation_reaches_rust_runtime() {
        let (mut registry, session_id) = registry_with_session();
        let error = registry
            .attach_surface(
                &session_id,
                RealtimePreviewSurfaceBindingDescriptor {
                    kind: RealtimePreviewSurfaceBindingKind::WindowsHwnd,
                    parent_handle: Some(0),
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                    scale_factor_millis: 1000,
                },
            )
            .expect_err("runtime rejects zero native parent handles");

        assert_eq!(error.kind(), RealtimePreviewBindingErrorKind::RuntimeSurface);
        assert!(
            error
                .message()
                .contains("MissingParentHandle"),
            "diagnostic should come from Rust surface validation: {}",
            error.message()
        );
    }

    #[test]
    fn generation_and_target_microseconds_round_trip_as_integers() {
        let (mut registry, session_id) = registry_with_session();
        let generation = registry
            .seek(&session_id, 1_234_567)
            .expect("seek returns generation");
        let result = registry
            .request_frame(
                &session_id,
                RealtimePreviewFrameBindingRequest {
                    target_time_microseconds: 1_234_567,
                    playback_generation: generation.playback_generation,
                    queue_latency_ms: 3,
                    render_duration_ms: 4,
                },
            )
            .expect("frame request succeeds");

        assert_eq!(result.target_time_microseconds, 1_234_567);
        assert_eq!(result.playback_generation, generation.playback_generation);
        assert!(result.presented);
    }

    #[test]
    fn telemetry_is_queryable_without_native_or_gpu_handles() {
        let (mut registry, session_id) = registry_with_session();
        let generation = registry.seek(&session_id, 42).expect("seek succeeds");
        registry
            .request_frame(
                &session_id,
                RealtimePreviewFrameBindingRequest {
                    target_time_microseconds: 42,
                    playback_generation: generation.playback_generation,
                    queue_latency_ms: 5,
                    render_duration_ms: 7,
                },
            )
            .expect("frame request records telemetry");

        let telemetry = registry.telemetry(&session_id).expect("telemetry is returned");
        let telemetry_json =
            serde_json::to_value(&telemetry).expect("telemetry response serializes");

        assert_eq!(telemetry.target_time_microseconds, 42);
        assert_eq!(telemetry.presented_frame_count, 1);
        assert!(telemetry_json.get("gpuDevice").is_none());
        assert!(telemetry_json.get("commandEncoder").is_none());
        assert!(telemetry_json.get("nativeChildHandle").is_none());
        assert!(telemetry_json.get("cacheKey").is_none());
    }
}

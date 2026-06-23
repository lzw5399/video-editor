#[test]
fn scheduler_preview_audio_preview_binding_routes_realtime_work_through_task_runtime() {
    let source = include_str!("../src/realtime_preview_service.rs");

    for forbidden in [
        "still_frame_workers",
        "playback_workers",
        "rt-preview-still",
        "rt-preview-playback",
        "REALTIME_PLAYBACK_IDLE_POLL_INTERVAL",
    ] {
        assert!(
            !source.contains(forbidden),
            "preview binding must not keep binding-owned worker policy: {forbidden}"
        );
    }

    for required in [
        "task_runtime::JobScheduler",
        "JobDomain::InteractivePreview",
        "JobDomain::ScrubSeek",
        "JobPriority::Realtime",
        "JobPriority::Interactive",
        "ResourceClass::GpuPresent",
        "CompletionFreshness::playback_generation",
    ] {
        assert!(
            source.contains(required),
            "preview binding must expose scheduler admission evidence: {required}"
        );
    }
}

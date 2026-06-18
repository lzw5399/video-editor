use media_runtime::{
    FrameDimensions, FrameLeaseRequest, FramePool, FramePoolLimits, FrameStorageKind,
    FrameStorageRequest, MediaSessionId, RuntimeDeviceId, TextureBackend, TextureHandle,
    TextureHandleId, VideoColorMetadata, VideoPixelFormat,
};

#[test]
fn session_leaks_close_reports_unreleased_cpu_frame_leases() {
    let mut pool = frame_pool("desktop-session-cpu");

    let frame = pool
        .acquire_video_frame(cpu_request(11))
        .expect("CPU frame lease should be acquired");

    let report = pool.close_session();

    assert_eq!(pool.outstanding_lease_count(), 0);
    assert_eq!(
        report.owner_session,
        MediaSessionId("desktop-session-cpu".to_owned())
    );
    assert_eq!(report.leak_diagnostics.len(), 1);
    let leak = &report.leak_diagnostics[0];
    assert_eq!(leak.lease_id, frame.release);
    assert_eq!(leak.frame_handle_id, frame.handle_id);
    assert_eq!(
        leak.owner_session,
        MediaSessionId("desktop-session-cpu".to_owned())
    );
    assert_eq!(leak.generation, Some(11));
    assert_eq!(leak.storage_kind, FrameStorageKind::Cpu);
    assert_eq!(leak.texture_handle_id, None);
}

#[test]
fn session_leaks_close_reports_unreleased_platform_opaque_frame_leases() {
    let mut pool = frame_pool("desktop-session-opaque");

    let frame = pool
        .acquire_video_frame(platform_opaque_request(12))
        .expect("platform opaque frame lease should be acquired");

    let report = pool.close_session();

    assert_eq!(pool.outstanding_lease_count(), 0);
    assert_eq!(report.leak_diagnostics.len(), 1);
    let leak = &report.leak_diagnostics[0];
    assert_eq!(leak.lease_id, frame.release);
    assert_eq!(leak.frame_handle_id, frame.handle_id);
    assert_eq!(
        leak.owner_session,
        MediaSessionId("desktop-session-opaque".to_owned())
    );
    assert_eq!(leak.generation, Some(12));
    assert_eq!(leak.storage_kind, FrameStorageKind::PlatformOpaque);
    assert_eq!(leak.texture_handle_id, None);
}

#[test]
fn session_leaks_close_reports_unreleased_texture_leases_with_device_metadata() {
    let mut pool = frame_pool("desktop-session-texture");
    let device_id = RuntimeDeviceId {
        backend: TextureBackend::D3d11Texture2D,
        adapter_id: "adapter-luid-1".to_owned(),
        device_id: "device-1".to_owned(),
    };

    let frame = pool
        .acquire_video_frame(texture_request(13, device_id.clone()))
        .expect("texture frame lease should be acquired");

    let report = pool.close_session();

    assert_eq!(pool.outstanding_lease_count(), 0);
    assert_eq!(report.leak_diagnostics.len(), 1);
    let leak = &report.leak_diagnostics[0];
    assert_eq!(leak.lease_id, frame.release);
    assert_eq!(leak.frame_handle_id, frame.handle_id);
    assert_eq!(
        leak.owner_session,
        MediaSessionId("desktop-session-texture".to_owned())
    );
    assert_eq!(leak.generation, Some(13));
    assert_eq!(leak.storage_kind, FrameStorageKind::Texture);
    assert_eq!(
        leak.texture_handle_id,
        Some(TextureHandleId("texture-leak-13".to_owned()))
    );
    assert_eq!(leak.texture_backend, Some(TextureBackend::D3d11Texture2D));
    assert_eq!(leak.texture_device_id, Some(device_id));
    assert_eq!(leak.texture_compatible, Some(true));
}

fn frame_pool(session_id: &str) -> FramePool {
    FramePool::new(
        MediaSessionId(session_id.to_owned()),
        FramePoolLimits {
            max_outstanding_leases: 4,
        },
    )
}

fn cpu_request(playback_generation: u64) -> FrameLeaseRequest {
    base_request(
        playback_generation,
        FrameStorageRequest::Cpu {
            estimated_byte_len: 1920 * 1080 * 3 / 2,
        },
    )
}

fn platform_opaque_request(playback_generation: u64) -> FrameLeaseRequest {
    base_request(
        playback_generation,
        FrameStorageRequest::PlatformOpaque {
            label: "MediaFoundationSample(opaque)".to_owned(),
        },
    )
}

fn texture_request(playback_generation: u64, device_id: RuntimeDeviceId) -> FrameLeaseRequest {
    base_request(
        playback_generation,
        FrameStorageRequest::Texture(TextureHandle {
            handle_id: TextureHandleId(format!("texture-leak-{playback_generation}")),
            owner_session: MediaSessionId("desktop-session-texture".to_owned()),
            generation: playback_generation,
            backend: device_id.backend,
            device_id,
            dimensions: dimensions(),
            pixel_format: VideoPixelFormat::Nv12,
            color: VideoColorMetadata::unknown_with_diagnostic("test texture color"),
        }),
    )
}

fn base_request(playback_generation: u64, storage: FrameStorageRequest) -> FrameLeaseRequest {
    FrameLeaseRequest {
        playback_generation: Some(playback_generation),
        source_time_us: 0,
        duration_us: Some(33_333),
        frame_index: Some(0),
        dimensions: dimensions(),
        pixel_format: VideoPixelFormat::Nv12,
        color: VideoColorMetadata::unknown_with_diagnostic("test frame color"),
        storage,
    }
}

fn dimensions() -> FrameDimensions {
    FrameDimensions {
        width: 1920,
        height: 1080,
    }
}

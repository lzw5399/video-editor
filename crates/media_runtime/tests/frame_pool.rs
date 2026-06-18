use media_runtime::{
    FrameDimensions, FrameLeaseRequest, FramePool, FramePoolErrorKind, FramePoolLimits,
    FrameStorageRequest, MediaSessionId, RuntimeDeviceId, TextureBackend, TextureHandle,
    TextureHandleId, VideoColorMetadata, VideoFrameStorage, VideoPixelFormat,
};

#[test]
fn frame_pool_acquire_cpu_frame_increments_and_release_decrements_outstanding_leases() {
    let mut pool = FramePool::new(
        MediaSessionId("session-1".to_owned()),
        FramePoolLimits {
            max_outstanding_leases: 2,
        },
    );

    let frame = pool
        .acquire_video_frame(cpu_request(
            Some(3),
            VideoColorMetadata::unknown_with_diagnostic("unknown color"),
        ))
        .expect("CPU frame lease should be acquired");

    assert_eq!(pool.outstanding_lease_count(), 1);
    assert_eq!(frame.owner_session, MediaSessionId("session-1".to_owned()));
    assert_eq!(frame.playback_generation, Some(3));
    assert!(matches!(frame.storage, VideoFrameStorage::Cpu(_)));

    pool.release(frame.release.clone())
        .expect("release should succeed");

    assert_eq!(pool.outstanding_lease_count(), 0);
}

#[test]
fn frame_pool_session_close_releases_unreleased_frame_and_texture_handles_with_leak_diagnostics() {
    let mut pool = FramePool::new(
        MediaSessionId("session-1".to_owned()),
        FramePoolLimits {
            max_outstanding_leases: 4,
        },
    );

    let cpu = pool
        .acquire_video_frame(cpu_request(
            Some(5),
            VideoColorMetadata::unknown_with_diagnostic("cpu color"),
        ))
        .expect("CPU frame lease should be acquired");
    let texture = pool
        .acquire_video_frame(FrameLeaseRequest {
            playback_generation: Some(6),
            source_time_us: 1_000,
            duration_us: Some(33_333),
            frame_index: Some(30),
            dimensions: FrameDimensions {
                width: 3840,
                height: 2160,
            },
            pixel_format: VideoPixelFormat::Nv12,
            color: VideoColorMetadata::unknown_with_diagnostic("texture color"),
            storage: FrameStorageRequest::Texture(TextureHandle {
                handle_id: TextureHandleId("texture-1".to_owned()),
                owner_session: MediaSessionId("session-1".to_owned()),
                generation: 6,
                backend: TextureBackend::MetalTexture,
                device_id: RuntimeDeviceId {
                    backend: TextureBackend::MetalTexture,
                    adapter_id: "adapter".to_owned(),
                    device_id: "device".to_owned(),
                },
                dimensions: FrameDimensions {
                    width: 3840,
                    height: 2160,
                },
                pixel_format: VideoPixelFormat::Nv12,
                color: VideoColorMetadata::unknown_with_diagnostic("texture color"),
            }),
        })
        .expect("texture frame lease should be acquired");

    assert_eq!(pool.outstanding_lease_count(), 2);

    let report = pool.close_session();

    assert_eq!(pool.outstanding_lease_count(), 0);
    assert_eq!(report.owner_session, MediaSessionId("session-1".to_owned()));
    assert_eq!(report.leak_diagnostics.len(), 2);
    assert!(
        report
            .leak_diagnostics
            .iter()
            .any(|leak| leak.frame_handle_id == cpu.handle_id
                && leak.owner_session == MediaSessionId("session-1".to_owned())
                && leak.generation == Some(5))
    );
    assert!(
        report
            .leak_diagnostics
            .iter()
            .any(|leak| leak.frame_handle_id == texture.handle_id
                && leak.texture_handle_id == Some(TextureHandleId("texture-1".to_owned()))
                && leak.owner_session == MediaSessionId("session-1".to_owned())
                && leak.generation == Some(6))
    );
}

#[test]
fn frame_pool_unknown_color_metadata_is_preserved_with_diagnostics_on_decoded_frame() {
    let mut pool = FramePool::new(
        MediaSessionId("session-1".to_owned()),
        FramePoolLimits {
            max_outstanding_leases: 1,
        },
    );
    let color = VideoColorMetadata::unknown_with_diagnostic("container omitted color metadata");

    let frame = pool
        .acquire_video_frame(cpu_request(None, color.clone()))
        .expect("frame should be acquired");

    assert_eq!(frame.color, color);
    assert_eq!(
        frame.color.diagnostics[0].message,
        "container omitted color metadata"
    );
}

#[test]
fn frame_pool_acquires_platform_opaque_frame_without_exposing_native_pointer() {
    let mut pool = FramePool::new(
        MediaSessionId("session-opaque".to_owned()),
        FramePoolLimits {
            max_outstanding_leases: 1,
        },
    );

    let frame = pool
        .acquire_video_frame(FrameLeaseRequest {
            playback_generation: Some(9),
            source_time_us: 2_000,
            duration_us: Some(33_333),
            frame_index: Some(1),
            dimensions: FrameDimensions {
                width: 160,
                height: 90,
            },
            pixel_format: VideoPixelFormat::Nv12,
            color: VideoColorMetadata::unknown_with_diagnostic("platform color attachment missing"),
            storage: FrameStorageRequest::PlatformOpaque {
                label: "CoreVideoPixelBuffer(opaque)".to_owned(),
            },
        })
        .expect("platform opaque frame lease should be acquired");

    match frame.storage {
        VideoFrameStorage::PlatformOpaque(handle) => {
            assert_eq!(
                handle.owner_session,
                MediaSessionId("session-opaque".to_owned())
            );
            assert_eq!(handle.generation, Some(9));
            assert_eq!(handle.label, "CoreVideoPixelBuffer(opaque)");
        }
        other => panic!("expected platform opaque frame storage, got {other:?}"),
    }
}

#[test]
fn frame_pool_rejects_release_from_wrong_owner_session() {
    let mut pool = FramePool::new(
        MediaSessionId("session-1".to_owned()),
        FramePoolLimits {
            max_outstanding_leases: 1,
        },
    );
    let frame = pool
        .acquire_video_frame(cpu_request(
            Some(1),
            VideoColorMetadata::unknown_with_diagnostic("unknown"),
        ))
        .expect("frame should be acquired");

    let error = pool
        .release_for_session(
            &MediaSessionId("foreign-session".to_owned()),
            frame.release.clone(),
        )
        .expect_err("foreign session must not release a lease it does not own");

    assert_eq!(error.kind, FramePoolErrorKind::OwnerSessionMismatch);
    assert_eq!(pool.outstanding_lease_count(), 1);
}

fn cpu_request(playback_generation: Option<u64>, color: VideoColorMetadata) -> FrameLeaseRequest {
    FrameLeaseRequest {
        playback_generation,
        source_time_us: 1_000,
        duration_us: Some(33_333),
        frame_index: Some(30),
        dimensions: FrameDimensions {
            width: 1920,
            height: 1080,
        },
        pixel_format: VideoPixelFormat::Nv12,
        color,
        storage: FrameStorageRequest::Cpu {
            estimated_byte_len: 1920 * 1080 * 3 / 2,
        },
    }
}

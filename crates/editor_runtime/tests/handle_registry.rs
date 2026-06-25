use editor_runtime::{
    HandleAcquireRequest, HandleKind, HandleRegistry, HandleReleaseState, HandleToken,
    RuntimeErrorKind, RuntimeSessionConfig, RuntimeSessionId, RuntimeSessionRegistry,
    TextureHandleDescriptor, TextureResolveExpectation,
};
use media_runtime::{
    ColorMatrix, ColorPrimaries, ColorRange, ColorTransfer, FrameDimensions, RuntimeDeviceId,
    TextureBackend, VideoColorMetadata, VideoPixelFormat,
};

#[test]
fn acquire_and_release_all_handle_kinds_updates_outstanding_counts() {
    let owner = runtime_session_id();
    let texture_device = device("adapter-a", "device-a");
    let mut registry = HandleRegistry::default();

    let handles = vec![
        registry
            .acquire(HandleAcquireRequest::runtime_session(owner.clone()))
            .expect("runtime handle should be acquired"),
        registry
            .acquire(HandleAcquireRequest::project_session(owner.clone()))
            .expect("project handle should be acquired"),
        registry
            .acquire(HandleAcquireRequest::media(owner.clone()))
            .expect("media handle should be acquired"),
        registry
            .acquire(HandleAcquireRequest::frame(owner.clone()))
            .expect("frame handle should be acquired"),
        registry
            .acquire(HandleAcquireRequest::texture(
                owner.clone(),
                texture_descriptor(texture_device.clone()),
            ))
            .expect("texture handle should be acquired"),
        registry
            .acquire(HandleAcquireRequest::artifact(owner.clone()))
            .expect("artifact handle should be acquired"),
    ];

    for handle in &handles {
        assert_eq!(registry.outstanding_count(&owner, handle.kind()), 1);
    }

    for handle in handles {
        let kind = handle.kind();
        let report = registry
            .release(&owner, &handle)
            .expect("handle release should succeed");
        assert_eq!(report.token, handle);
        assert_eq!(report.kind, kind);
        assert_eq!(report.owner_session, owner);
        assert_eq!(report.generation, 1);
        assert_eq!(report.outstanding_count, 0);
        assert_eq!(report.release_state, HandleReleaseState::Explicit);
        assert_eq!(registry.outstanding_count(&owner, kind), 0);
    }
}

#[test]
fn stale_wrong_owner_wrong_device_expired_double_release_and_unknown_fail_closed() {
    let owner = runtime_session_id();
    let other_owner = other_runtime_session_id();
    let texture_device = device("adapter-a", "device-a");
    let mut registry = HandleRegistry::default();
    let texture = registry
        .acquire(HandleAcquireRequest::texture(
            owner.clone(),
            texture_descriptor(texture_device.clone()),
        ))
        .expect("texture handle should be acquired");

    let stale = texture.with_generation(texture.generation() + 1);
    assert_runtime_error(
        registry.resolve(&owner, &stale, 0),
        RuntimeErrorKind::StaleGeneration,
    );

    assert_runtime_error(
        registry.release(&other_owner, &texture),
        RuntimeErrorKind::WrongOwner,
    );

    let wrong_device = device("adapter-b", "device-b");
    assert_runtime_error(
        registry.resolve_texture(
            &owner,
            &texture,
            &TextureResolveExpectation {
                descriptor: texture_descriptor(wrong_device),
            },
            0,
        ),
        RuntimeErrorKind::WrongDevice,
    );

    let mut wrong_backend = texture_descriptor(texture_device.clone());
    wrong_backend.backend = TextureBackend::D3d12Resource;
    assert_runtime_error(
        registry.resolve_texture(
            &owner,
            &texture,
            &TextureResolveExpectation {
                descriptor: wrong_backend,
            },
            0,
        ),
        RuntimeErrorKind::WrongDevice,
    );

    let mut wrong_dimensions = texture_descriptor(texture_device.clone());
    wrong_dimensions.dimensions.width = 1280;
    assert_runtime_error(
        registry.resolve_texture(
            &owner,
            &texture,
            &TextureResolveExpectation {
                descriptor: wrong_dimensions,
            },
            0,
        ),
        RuntimeErrorKind::TextureMetadataMismatch,
    );

    let mut wrong_pixel_format = texture_descriptor(texture_device.clone());
    wrong_pixel_format.pixel_format = VideoPixelFormat::Rgba8;
    assert_runtime_error(
        registry.resolve_texture(
            &owner,
            &texture,
            &TextureResolveExpectation {
                descriptor: wrong_pixel_format,
            },
            0,
        ),
        RuntimeErrorKind::TextureMetadataMismatch,
    );

    let mut wrong_color = texture_descriptor(texture_device.clone());
    wrong_color.color.range = ColorRange::Limited;
    assert_runtime_error(
        registry.resolve_texture(
            &owner,
            &texture,
            &TextureResolveExpectation {
                descriptor: wrong_color,
            },
            0,
        ),
        RuntimeErrorKind::TextureMetadataMismatch,
    );

    let expiring = registry
        .acquire(HandleAcquireRequest::frame(owner.clone()).with_lease_expires_at_us(10))
        .expect("expiring frame handle should be acquired");
    assert_runtime_error(
        registry.resolve(&owner, &expiring, 11),
        RuntimeErrorKind::LeaseExpired,
    );

    let unknown =
        HandleToken::from_raw_parts(HandleKind::Media, "fabricated-media", owner.clone(), 1);
    assert_runtime_error(
        registry.resolve(&owner, &unknown, 0),
        RuntimeErrorKind::UnknownHandle,
    );

    registry
        .release(&owner, &texture)
        .expect("first release should succeed");
    assert_runtime_error(
        registry.release(&owner, &texture),
        RuntimeErrorKind::DoubleRelease,
    );
}

#[test]
fn closing_runtime_session_cascades_owned_handles_and_reports_leaks() {
    let owner = runtime_session_id();
    let device = device("adapter-a", "device-a");
    let mut registry = HandleRegistry::default();
    let media = registry
        .acquire(HandleAcquireRequest::media(owner.clone()))
        .expect("media handle should be acquired");
    let frame = registry
        .acquire(HandleAcquireRequest::frame(owner.clone()))
        .expect("frame handle should be acquired");
    let texture = registry
        .acquire(HandleAcquireRequest::texture(
            owner.clone(),
            texture_descriptor(device),
        ))
        .expect("texture handle should be acquired");
    let artifact = registry
        .acquire(HandleAcquireRequest::artifact(owner.clone()))
        .expect("artifact handle should be acquired");

    registry
        .release(&owner, &frame)
        .expect("explicitly released frame should not be reported as leaked");

    let report = registry.close_runtime_session(&owner);

    assert_eq!(report.owner_session, owner);
    assert_eq!(report.leak_diagnostics.len(), 3);
    assert!(
        report
            .leak_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.token == media)
    );
    assert!(
        report
            .leak_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.token == texture)
    );
    assert!(
        report
            .leak_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.token == artifact)
    );
    for diagnostic in &report.leak_diagnostics {
        assert_eq!(diagnostic.owner_session, report.owner_session);
        assert_eq!(diagnostic.generation, 1);
        assert_eq!(diagnostic.outstanding_count, 1);
        assert_eq!(diagnostic.release_state, HandleReleaseState::CascadeClose);
        assert_ne!(diagnostic.kind, HandleKind::Frame);
    }
    for kind in [
        HandleKind::Media,
        HandleKind::Frame,
        HandleKind::Texture,
        HandleKind::Artifact,
    ] {
        assert_eq!(registry.outstanding_count(&report.owner_session, kind), 0);
    }
}

fn assert_runtime_error<T: std::fmt::Debug>(
    result: Result<T, editor_runtime::RuntimeError>,
    kind: RuntimeErrorKind,
) {
    let error = result.expect_err("operation should fail closed");
    assert_eq!(error.kind, kind);
}

fn runtime_session_id() -> RuntimeSessionId {
    let mut registry = RuntimeSessionRegistry::default();
    registry
        .create_session(RuntimeSessionConfig::default())
        .expect("runtime session should be created")
        .id
}

fn other_runtime_session_id() -> RuntimeSessionId {
    let mut registry = RuntimeSessionRegistry::default();
    registry
        .create_session(RuntimeSessionConfig {
            diagnostic_label: Some("other".to_owned()),
        })
        .expect("runtime session should be created")
        .id
        .with_generation(2)
}

fn device(adapter_id: &str, device_id: &str) -> RuntimeDeviceId {
    RuntimeDeviceId {
        backend: TextureBackend::MetalTexture,
        adapter_id: adapter_id.to_owned(),
        device_id: device_id.to_owned(),
    }
}

fn texture_descriptor(device: RuntimeDeviceId) -> TextureHandleDescriptor {
    TextureHandleDescriptor {
        backend: device.backend,
        device,
        dimensions: FrameDimensions {
            width: 1920,
            height: 1080,
        },
        pixel_format: VideoPixelFormat::Bgra8,
        color: VideoColorMetadata {
            primaries: ColorPrimaries::Bt709,
            transfer: ColorTransfer::Srgb,
            matrix: ColorMatrix::Identity,
            range: ColorRange::Full,
            diagnostics: Vec::new(),
        },
    }
}

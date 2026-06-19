use draft_model::{MaterialId, Microseconds};
use media_runtime::{
    MediaSessionId, RuntimeDeviceId, TextureBackend, VideoColorMetadata,
};
use realtime_preview_runtime::{
    CpuVideoFrame, FrameColorInfo, FrameValidationErrorKind, PlaybackGeneration, PreviewFrameInput,
    PreviewFrameProvider, PreviewFrameProviderError, TextureHandleDescriptor,
};

#[test]
fn frame_provider_valid_cpu_rgba_frames_pass_stride_and_pixel_validation() {
    let frame = CpuVideoFrame::new(
        MaterialId::new("video-material"),
        Microseconds::new(250_000),
        PlaybackGeneration::new(7),
        2,
        2,
        8,
        FrameColorInfo::srgb_rgba8(),
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
    )
    .expect("valid 2x2 RGBA frame");

    assert_eq!(frame.material_id.as_str(), "video-material");
    assert_eq!(frame.source_position, Microseconds::new(250_000));
    assert_eq!(frame.playback_generation, PlaybackGeneration::new(7));
    assert_eq!(frame.pixel_len(), 16);
}

#[test]
fn frame_provider_invalid_dimensions_stride_and_pixel_lengths_return_typed_errors() {
    let zero_width = CpuVideoFrame::new(
        MaterialId::new("video-material"),
        Microseconds::new(0),
        PlaybackGeneration::initial(),
        0,
        1,
        4,
        FrameColorInfo::srgb_rgba8(),
        vec![0, 0, 0, 255],
    )
    .expect_err("zero width rejected");
    assert_eq!(
        zero_width.kind(),
        FrameValidationErrorKind::InvalidDimensions
    );

    let short_stride = CpuVideoFrame::new(
        MaterialId::new("video-material"),
        Microseconds::new(0),
        PlaybackGeneration::initial(),
        2,
        1,
        4,
        FrameColorInfo::srgb_rgba8(),
        vec![0; 8],
    )
    .expect_err("stride shorter than width * 4 rejected");
    assert_eq!(short_stride.kind(), FrameValidationErrorKind::InvalidStride);

    let short_pixels = CpuVideoFrame::new(
        MaterialId::new("video-material"),
        Microseconds::new(0),
        PlaybackGeneration::initial(),
        2,
        2,
        8,
        FrameColorInfo::srgb_rgba8(),
        vec![0; 8],
    )
    .expect_err("pixel buffer shorter than stride * height rejected");
    assert_eq!(
        short_pixels.kind(),
        FrameValidationErrorKind::InvalidPixelLength
    );
}

#[test]
fn frame_provider_material_and_generation_metadata_are_validated_and_preserved_for_static_images() {
    let empty_material = CpuVideoFrame::new(
        MaterialId::new(""),
        Microseconds::new(0),
        PlaybackGeneration::initial(),
        1,
        1,
        4,
        FrameColorInfo::srgb_rgba8(),
        vec![0, 0, 0, 255],
    )
    .expect_err("empty material id rejected");
    assert_eq!(
        empty_material.kind(),
        FrameValidationErrorKind::MissingMaterialId
    );

    let static_frame = PreviewFrameInput::static_image(
        MaterialId::new("poster-image"),
        Microseconds::new(0),
        PlaybackGeneration::new(3),
        1,
        1,
        vec![10, 20, 30, 255],
    )
    .expect("valid static image frame");

    let PreviewFrameInput::StaticImage(frame) = static_frame else {
        panic!("expected static image input");
    };
    assert_eq!(frame.material_id.as_str(), "poster-image");
    assert_eq!(frame.source_position, Microseconds::ZERO);
    assert_eq!(frame.playback_generation, PlaybackGeneration::new(3));
}

#[test]
fn frame_provider_unavailable_frames_include_provider_diagnostics() {
    struct UnavailableProvider;

    impl PreviewFrameProvider for UnavailableProvider {
        fn provider_name(&self) -> &'static str {
            "test-unavailable-provider"
        }

        fn frame_for(
            &mut self,
            material_id: &MaterialId,
            source_position: Microseconds,
            playback_generation: PlaybackGeneration,
        ) -> Result<PreviewFrameInput, PreviewFrameProviderError> {
            Err(PreviewFrameProviderError::unavailable(
                self.provider_name(),
                material_id.clone(),
                source_position,
                playback_generation,
                "frame was not decoded into the session cache",
            ))
        }
    }

    let mut provider = UnavailableProvider;
    let error = provider
        .frame_for(
            &MaterialId::new("video-material"),
            Microseconds::new(500_000),
            PlaybackGeneration::new(2),
        )
        .expect_err("unavailable frame returns typed diagnostics");

    assert_eq!(error.provider_name(), "test-unavailable-provider");
    assert_eq!(
        error.material_id().map(MaterialId::as_str),
        Some("video-material")
    );
    assert_eq!(error.source_position(), Some(Microseconds::new(500_000)));
    assert_eq!(
        error.playback_generation(),
        Some(PlaybackGeneration::new(2))
    );
    assert!(error.to_string().contains("session cache"));
}

#[test]
fn frame_provider_texture_handle_descriptors_serialize_without_native_pointers() {
    let descriptor = TextureHandleDescriptor::new(
        MaterialId::new("video-material"),
        Microseconds::new(125_000),
        "macos-metal-texture-42",
        MediaSessionId("session-texture-1".to_owned()),
        PlaybackGeneration::new(9),
        "metalTexture",
        RuntimeDeviceId {
            backend: TextureBackend::MetalTexture,
            adapter_id: "metal-adapter".to_owned(),
            device_id: "metal-device".to_owned(),
        },
        1920,
        1080,
        "nv12",
        VideoColorMetadata::unknown_with_diagnostic("test texture color"),
    )
    .expect("valid opaque descriptor");

    let value = serde_json::to_value(PreviewFrameInput::TextureHandle(descriptor))
        .expect("texture descriptor serializes");

    assert_eq!(value["kind"], "textureHandle");
    assert_eq!(value["handle"]["materialId"], "video-material");
    assert_eq!(value["handle"]["sourcePosition"], 125_000);
    assert_eq!(value["handle"]["handleId"], "macos-metal-texture-42");
    assert_eq!(value["handle"]["playbackGeneration"], 9);
    assert_eq!(value["handle"]["backend"], "metalTexture");
    assert_eq!(value["handle"]["pixelFormat"], "nv12");
    assert!(value.get("nativePointer").is_none());
    assert!(value["handle"].get("nativePointer").is_none());
}

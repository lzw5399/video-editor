use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use draft_model::{MaterialId, Microseconds, RationalFrameRate};
use media_runtime::{
    DecodedVideoFrame, FrameDimensions, FrameHandleId, FrameLeaseId, MediaSessionId,
    RuntimeDeviceId, TextureBackend, TextureHandle, TextureHandleId, VideoColorMetadata,
    VideoFrameStorage, VideoPixelFormat,
};
use realtime_preview_runtime::{
    CpuVideoFrame, DecodedVideoFrameCache, FrameColorInfo, PlaybackGeneration, PreviewFrameInput,
    PreviewFrameProvider, PreviewFrameProviderError, SoftwareVideoFrameProvider,
    TextureHandleDescriptor,
};

#[test]
fn video_frame_provider_returns_seeded_h264_frames_without_process_work() {
    let fixture = testkit::generate_h264_preview_fixture()
        .expect("ffmpeg/ffprobe must be available to generate the deterministic H.264 fixture");
    assert_eq!(
        fixture.path().extension().and_then(|value| value.to_str()),
        Some("mp4")
    );
    assert_eq!(fixture.expected_codec(), "h264");

    let material_id = MaterialId::new("h264-material");
    let playback_generation = PlaybackGeneration::new(4);
    let mut cache = DecodedVideoFrameCache::new();
    cache
        .insert_h264_frames(
            material_id.clone(),
            RationalFrameRate::new(10, 1),
            2,
            vec![
                (0, rgba_frame(&material_id, 0, playback_generation, [255, 0, 0, 255])),
                (
                    1,
                    rgba_frame(&material_id, 100_000, playback_generation, [0, 0, 255, 255]),
                ),
            ],
        )
        .expect("seeded H.264 frames are valid");

    let process_calls = Arc::new(AtomicUsize::new(0));
    let mut provider =
        SoftwareVideoFrameProvider::new(cache).with_process_invocation_counter(process_calls.clone());

    let first = provider
        .frame_for(&material_id, Microseconds::ZERO, playback_generation)
        .expect("first seeded frame returned");
    let second = provider
        .frame_for(
            &material_id,
            Microseconds::new(100_000),
            playback_generation,
        )
        .expect("second seeded frame returned");

    let first = expect_cpu(first);
    let second = expect_cpu(second);
    assert_eq!((first.width, first.height), (2, 1));
    assert_eq!((second.width, second.height), (2, 1));
    assert_ne!(first.pixels, second.pixels);
    assert_eq!(first.source_position, Microseconds::ZERO);
    assert_eq!(second.source_position, Microseconds::new(100_000));
    assert_eq!(process_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn video_frame_provider_reports_uncached_out_of_range_and_unsupported_codec() {
    let material_id = MaterialId::new("h264-material");
    let generation = PlaybackGeneration::new(8);
    let mut cache = DecodedVideoFrameCache::new();
    cache
        .insert_h264_frames(
            material_id.clone(),
            RationalFrameRate::new(10, 1),
            3,
            vec![(0, rgba_frame(&material_id, 0, generation, [255, 0, 0, 255]))],
        )
        .expect("sparse H.264 cache can be seeded");
    cache
        .insert_codec_frames(
            MaterialId::new("prores-material"),
            "prores",
            RationalFrameRate::new(10, 1),
            1,
            vec![(
                0,
                rgba_frame(
                    &MaterialId::new("prores-material"),
                    0,
                    generation,
                    [0, 255, 0, 255],
                ),
            )],
        )
        .expect("unsupported codec entry can be seeded for diagnostics");

    let mut provider = SoftwareVideoFrameProvider::new(cache);

    let uncached = provider
        .frame_for(&material_id, Microseconds::new(100_000), generation)
        .expect_err("missing decoded frame reports cache miss");
    assert!(matches!(uncached, PreviewFrameProviderError::Unavailable { .. }));
    assert_eq!(uncached.material_id().map(MaterialId::as_str), Some("h264-material"));
    assert_eq!(uncached.source_position(), Some(Microseconds::new(100_000)));

    let out_of_range = provider
        .frame_for(&material_id, Microseconds::new(300_000), generation)
        .expect_err("request past cached duration is out of range");
    assert!(matches!(out_of_range, PreviewFrameProviderError::OutOfRange { .. }));

    let unsupported = provider
        .frame_for(&MaterialId::new("prores-material"), Microseconds::ZERO, generation)
        .expect_err("non-H.264 cache entry is unsupported");
    assert!(matches!(
        unsupported,
        PreviewFrameProviderError::UnsupportedCodec { .. }
    ));
    assert!(unsupported.to_string().contains("prores"));
}

#[test]
fn video_frame_provider_texture_descriptor_preserves_owner_device_color_and_generation() {
    let material_id = MaterialId::new("native-texture-material");
    let owner_session = MediaSessionId("session-texture-1".to_owned());
    let device_id = RuntimeDeviceId {
        backend: TextureBackend::MetalTexture,
        adapter_id: "metal-adapter".to_owned(),
        device_id: "metal-device".to_owned(),
    };
    let color = VideoColorMetadata::unknown_with_diagnostic("test texture color");
    let frame = DecodedVideoFrame {
        handle_id: FrameHandleId("frame-1".to_owned()),
        owner_session: owner_session.clone(),
        playback_generation: Some(77),
        source_time_us: 222_222,
        duration_us: Some(33_333),
        frame_index: Some(3),
        dimensions: FrameDimensions {
            width: 320,
            height: 180,
        },
        pixel_format: VideoPixelFormat::Nv12,
        color: color.clone(),
        storage: VideoFrameStorage::Texture(TextureHandle {
            handle_id: TextureHandleId("texture-live-1".to_owned()),
            owner_session: owner_session.clone(),
            generation: 77,
            backend: TextureBackend::MetalTexture,
            device_id: device_id.clone(),
            dimensions: FrameDimensions {
                width: 320,
                height: 180,
            },
            pixel_format: VideoPixelFormat::Nv12,
            color: color.clone(),
        }),
        release: FrameLeaseId("lease-1".to_owned()),
    };

    let descriptor = TextureHandleDescriptor::from_decoded_frame(
        material_id.clone(),
        Microseconds::new(222_222),
        &frame,
    )
    .expect("decoded texture frame should validate")
    .expect("decoded texture frame should produce a descriptor");

    assert_eq!(descriptor.material_id, material_id);
    assert_eq!(descriptor.source_position, Microseconds::new(222_222));
    assert_eq!(descriptor.handle_id, "texture-live-1");
    assert_eq!(descriptor.owner_session, owner_session);
    assert_eq!(descriptor.playback_generation, PlaybackGeneration::new(77));
    assert_eq!(descriptor.device_id, device_id);
    assert_eq!(descriptor.width, 320);
    assert_eq!(descriptor.height, 180);
    assert_eq!(descriptor.pixel_format, "nv12");
    assert_eq!(descriptor.color, color);
}

fn rgba_frame(
    material_id: &MaterialId,
    source_position_micros: u64,
    playback_generation: PlaybackGeneration,
    rgba: [u8; 4],
) -> CpuVideoFrame {
    let mut pixels = Vec::new();
    pixels.extend_from_slice(&rgba);
    pixels.extend_from_slice(&rgba);
    CpuVideoFrame::new(
        material_id.clone(),
        Microseconds::new(source_position_micros),
        playback_generation,
        2,
        1,
        8,
        FrameColorInfo::srgb_rgba8(),
        pixels,
    )
    .expect("test frame is valid")
}

fn expect_cpu(input: PreviewFrameInput) -> CpuVideoFrame {
    match input {
        PreviewFrameInput::CpuRgba(frame) => frame,
        other => panic!("expected CPU RGBA frame, got {other:?}"),
    }
}

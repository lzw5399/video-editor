use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use draft_model::{MaterialId, Microseconds};
use media_runtime::{
    AudioDecoder, DecodeError, DecodedVideoFrame, FrameDimensions, FrameLeaseRequest, FramePool,
    FramePoolLimits, FrameStorageRequest, MediaIoError, MediaIoFallbackCandidate,
    MediaIoFallbackReason, MediaIoFallbackSelection, MediaOpenRequest, MediaReader, MediaSession,
    MediaSessionId, MediaStreamInfo, MediaStreamKind, RationalFrameRate, RuntimeDeviceId,
    SelectedDecodePath, StreamId, TextureBackend, TextureHandle, TextureHandleId,
    VideoColorMetadata, VideoDecodeRequest, VideoDecoder, VideoFrameStorage, VideoPixelFormat,
    select_media_io_fallback,
};
use realtime_preview_runtime::{
    MediaIoFrameProvider, PlaybackGeneration, PreviewDecodeDeviceContext, PreviewFrameStorageKind,
    PreviewFrameStoragePreference, PreviewMaterialDecodeRequest, PreviewMaterialDecodeSource,
    RealtimePreviewFallbackReason,
};

#[test]
fn media_io_handoff_converts_preview_request_to_video_decode_request_and_reports_cpu_storage() {
    let material_id = MaterialId::new("video-material");
    let recorded_requests = Rc::new(RefCell::new(Vec::new()));
    let reader = MockMediaReader::new(
        recorded_requests.clone(),
        MockStorage::Cpu,
        selected_fallback(
            SelectedDecodePath::FfmpegCpuFrame,
            MediaIoFallbackReason::HardwareDecodeUnavailable,
        ),
    );
    let mut provider = MediaIoFrameProvider::new(Box::new(reader));
    provider
        .register_material(PreviewMaterialDecodeSource {
            material_id: material_id.clone(),
            material_uri: PathBuf::from("/fixtures/video.mp4"),
            stream_id: StreamId(0),
            selected_path: SelectedDecodePath::FfmpegCpuFrame,
            fallback_selection: selected_fallback(
                SelectedDecodePath::FfmpegCpuFrame,
                MediaIoFallbackReason::HardwareDecodeUnavailable,
            ),
        })
        .expect("material registers through media IO");

    let output = provider
        .decode_material_frame(
            PreviewMaterialDecodeRequest {
                material_id: material_id.clone(),
                source_position: Microseconds::new(250_000),
                playback_generation: PlaybackGeneration::new(4),
                desired_storage: PreviewFrameStoragePreference::Any,
                device: PreviewDecodeDeviceContext::cpu_only(),
            },
            PlaybackGeneration::new(4),
        )
        .expect("media IO frame decodes");

    assert_eq!(
        recorded_requests.borrow().as_slice(),
        &[VideoDecodeRequest {
            source_time_us: 250_000,
            playback_generation: Some(4),
        }]
    );
    assert_eq!(output.material_id, material_id);
    assert_eq!(output.storage_kind, PreviewFrameStorageKind::Cpu);
    assert_eq!(output.selected_path, SelectedDecodePath::FfmpegCpuFrame);
    assert_eq!(
        output.fallback,
        Some(RealtimePreviewFallbackReason::MediaIoFfmpegCpuFrame)
    );
    assert!(!output.stale_rejected);
    assert_eq!(provider.telemetry().decode_request_count, 1);
}

#[test]
fn media_io_handoff_preserves_texture_handles_only_for_proven_device_compatibility() {
    let material_id = MaterialId::new("texture-material");
    let device = RuntimeDeviceId {
        backend: TextureBackend::D3d11Texture2D,
        adapter_id: "adapter-1".to_owned(),
        device_id: "device-1".to_owned(),
    };
    let reader = MockMediaReader::new(
        Rc::new(RefCell::new(Vec::new())),
        MockStorage::Texture(device.clone()),
        selected_fallback(
            SelectedDecodePath::NativeHardwareTexture,
            MediaIoFallbackReason::TextureInteropUnavailable,
        ),
    );
    let mut provider = MediaIoFrameProvider::new(Box::new(reader));
    provider
        .register_material(PreviewMaterialDecodeSource {
            material_id: material_id.clone(),
            material_uri: PathBuf::from("/fixtures/texture.mp4"),
            stream_id: StreamId(0),
            selected_path: SelectedDecodePath::NativeHardwareTexture,
            fallback_selection: selected_fallback(
                SelectedDecodePath::NativeHardwareTexture,
                MediaIoFallbackReason::TextureInteropUnavailable,
            ),
        })
        .expect("material registers through media IO");

    let output = provider
        .decode_material_frame(
            PreviewMaterialDecodeRequest {
                material_id,
                source_position: Microseconds::ZERO,
                playback_generation: PlaybackGeneration::new(5),
                desired_storage: PreviewFrameStoragePreference::Texture,
                device: PreviewDecodeDeviceContext::compatible(device.clone()),
            },
            PlaybackGeneration::new(5),
        )
        .expect("texture handoff decodes");

    assert_eq!(output.storage_kind, PreviewFrameStorageKind::Texture);
    assert_eq!(
        output.selected_path,
        SelectedDecodePath::NativeHardwareTexture
    );
    assert_eq!(output.fallback, None);
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.texture_compatible
            && diagnostic.preview_device.as_ref() == Some(&device)
            && diagnostic.selected_path == SelectedDecodePath::NativeHardwareTexture
    }));
    assert!(matches!(
        output.decoded_frame.storage,
        VideoFrameStorage::Texture(_)
    ));
}

#[test]
fn media_io_handoff_keeps_cpu_or_platform_frames_valid_when_texture_compatibility_is_unproven() {
    let material_id = MaterialId::new("opaque-material");
    let reader = MockMediaReader::new(
        Rc::new(RefCell::new(Vec::new())),
        MockStorage::PlatformOpaque,
        selected_fallback(
            SelectedDecodePath::NativeHardwareCpuCopy,
            MediaIoFallbackReason::TextureInteropUnavailable,
        ),
    );
    let mut provider = MediaIoFrameProvider::new(Box::new(reader));
    provider
        .register_material(PreviewMaterialDecodeSource {
            material_id: material_id.clone(),
            material_uri: PathBuf::from("/fixtures/opaque.mp4"),
            stream_id: StreamId(0),
            selected_path: SelectedDecodePath::NativeHardwareCpuCopy,
            fallback_selection: selected_fallback(
                SelectedDecodePath::NativeHardwareCpuCopy,
                MediaIoFallbackReason::TextureInteropUnavailable,
            ),
        })
        .expect("material registers through media IO");

    let output = provider
        .decode_material_frame(
            PreviewMaterialDecodeRequest {
                material_id,
                source_position: Microseconds::new(100_000),
                playback_generation: PlaybackGeneration::new(6),
                desired_storage: PreviewFrameStoragePreference::Texture,
                device: PreviewDecodeDeviceContext::unproven("preview/native device mismatch"),
            },
            PlaybackGeneration::new(6),
        )
        .expect("platform-opaque handoff decodes");

    assert_eq!(output.storage_kind, PreviewFrameStorageKind::PlatformOpaque);
    assert_eq!(
        output.selected_path,
        SelectedDecodePath::NativeHardwareCpuCopy
    );
    assert_eq!(
        output.fallback,
        Some(RealtimePreviewFallbackReason::MediaIoTextureInteropUnavailable)
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.fallback_reason == Some(MediaIoFallbackReason::TextureInteropUnavailable)
            && !diagnostic.texture_compatible
    }));
}

#[test]
fn media_io_handoff_rejects_stale_generation_after_decode_and_counts_telemetry() {
    let material_id = MaterialId::new("stale-material");
    let reader = MockMediaReader::new(
        Rc::new(RefCell::new(Vec::new())),
        MockStorage::Cpu,
        selected_fallback(
            SelectedDecodePath::FfmpegCpuFrame,
            MediaIoFallbackReason::HardwareDecodeUnavailable,
        ),
    );
    let mut provider = MediaIoFrameProvider::new(Box::new(reader));
    provider
        .register_material(PreviewMaterialDecodeSource {
            material_id: material_id.clone(),
            material_uri: PathBuf::from("/fixtures/stale.mp4"),
            stream_id: StreamId(0),
            selected_path: SelectedDecodePath::FfmpegCpuFrame,
            fallback_selection: selected_fallback(
                SelectedDecodePath::FfmpegCpuFrame,
                MediaIoFallbackReason::HardwareDecodeUnavailable,
            ),
        })
        .expect("material registers through media IO");

    let output = provider
        .decode_material_frame(
            PreviewMaterialDecodeRequest {
                material_id,
                source_position: Microseconds::new(500_000),
                playback_generation: PlaybackGeneration::new(10),
                desired_storage: PreviewFrameStoragePreference::Any,
                device: PreviewDecodeDeviceContext::cpu_only(),
            },
            PlaybackGeneration::new(11),
        )
        .expect("decode result is still returned with stale rejection metadata");

    assert!(output.stale_rejected);
    assert_eq!(
        output.fallback,
        Some(RealtimePreviewFallbackReason::StaleGeneration)
    );
    assert_eq!(provider.telemetry().stale_rejected_count, 1);
    assert_eq!(provider.telemetry().presentable_frame_count, 0);
}

#[test]
fn media_io_handoff_adapter_does_not_take_timeline_render_or_desktop_runtime_ownership() {
    let source = include_str!("../src/media_io_adapter.rs");

    for forbidden in [
        "engine_core",
        "render_graph",
        "ffmpeg_compiler",
        "media_runtime_desktop",
        "Command::new",
        "std::process",
    ] {
        assert!(
            !source.contains(forbidden),
            "media IO adapter must not own forbidden boundary: {forbidden}"
        );
    }
}

fn selected_fallback(
    selected_path: SelectedDecodePath,
    reason: MediaIoFallbackReason,
) -> Option<MediaIoFallbackSelection> {
    let candidate_for = |path| {
        if path == selected_path {
            MediaIoFallbackCandidate::available(path)
        } else {
            MediaIoFallbackCandidate::unavailable(
                path,
                reason,
                format!("{path:?} unavailable in test"),
            )
        }
    };

    select_media_io_fallback(
        vec![
            candidate_for(SelectedDecodePath::NativeHardwareTexture),
            candidate_for(SelectedDecodePath::NativeHardwareCpuCopy),
            candidate_for(SelectedDecodePath::NativeSoftwareCpuFrame),
            candidate_for(SelectedDecodePath::FfmpegCpuFrame),
            candidate_for(SelectedDecodePath::FfmpegPreviewArtifact),
        ],
        reason,
    )
}

#[derive(Debug, Clone)]
enum MockStorage {
    Cpu,
    Texture(RuntimeDeviceId),
    PlatformOpaque,
}

#[derive(Debug)]
struct MockMediaReader {
    recorded_requests: Rc<RefCell<Vec<VideoDecodeRequest>>>,
    storage: MockStorage,
    fallback_selection: Option<MediaIoFallbackSelection>,
}

impl MockMediaReader {
    fn new(
        recorded_requests: Rc<RefCell<Vec<VideoDecodeRequest>>>,
        storage: MockStorage,
        fallback_selection: Option<MediaIoFallbackSelection>,
    ) -> Self {
        Self {
            recorded_requests,
            storage,
            fallback_selection,
        }
    }
}

impl MediaReader for MockMediaReader {
    fn reader_name(&self) -> &'static str {
        "mock-media-reader"
    }

    fn open(&self, request: MediaOpenRequest) -> Result<Box<dyn MediaSession>, MediaIoError> {
        Ok(Box::new(MockMediaSession {
            session_id: MediaSessionId(format!("mock-session-{}", request.material_uri.display())),
            streams: vec![video_stream()],
            recorded_requests: self.recorded_requests.clone(),
            storage: self.storage.clone(),
            fallback_selection: self.fallback_selection.clone(),
        }))
    }
}

#[derive(Debug)]
struct MockMediaSession {
    session_id: MediaSessionId,
    streams: Vec<MediaStreamInfo>,
    recorded_requests: Rc<RefCell<Vec<VideoDecodeRequest>>>,
    storage: MockStorage,
    fallback_selection: Option<MediaIoFallbackSelection>,
}

impl MediaSession for MockMediaSession {
    fn session_id(&self) -> MediaSessionId {
        self.session_id.clone()
    }

    fn streams(&self) -> &[MediaStreamInfo] {
        &self.streams
    }

    fn video_decoder(&self, _stream_id: StreamId) -> Result<Box<dyn VideoDecoder>, MediaIoError> {
        Ok(Box::new(MockVideoDecoder {
            pool: FramePool::new(
                self.session_id.clone(),
                FramePoolLimits {
                    max_outstanding_leases: 8,
                },
            ),
            recorded_requests: self.recorded_requests.clone(),
            storage: self.storage.clone(),
            _fallback_selection: self.fallback_selection.clone(),
        }))
    }

    fn audio_decoder(&self, _stream_id: StreamId) -> Result<Box<dyn AudioDecoder>, MediaIoError> {
        panic!("media IO handoff tests do not request audio decoders")
    }
}

#[derive(Debug)]
struct MockVideoDecoder {
    pool: FramePool,
    recorded_requests: Rc<RefCell<Vec<VideoDecodeRequest>>>,
    storage: MockStorage,
    _fallback_selection: Option<MediaIoFallbackSelection>,
}

impl VideoDecoder for MockVideoDecoder {
    fn decoder_name(&self) -> &'static str {
        "mock-video-decoder"
    }

    fn decode_at(&mut self, request: VideoDecodeRequest) -> Result<DecodedVideoFrame, DecodeError> {
        self.recorded_requests.borrow_mut().push(request.clone());
        let storage = match &self.storage {
            MockStorage::Cpu => FrameStorageRequest::Cpu {
                estimated_byte_len: 320 * 180 * 4,
            },
            MockStorage::Texture(device) => FrameStorageRequest::Texture(TextureHandle {
                handle_id: TextureHandleId("texture-1".to_owned()),
                owner_session: MediaSessionId("mock-session-/fixtures/texture.mp4".to_owned()),
                generation: request.playback_generation.unwrap_or_default(),
                backend: device.backend,
                device_id: device.clone(),
                dimensions: FrameDimensions {
                    width: 320,
                    height: 180,
                },
                pixel_format: VideoPixelFormat::Nv12,
                color: VideoColorMetadata::unknown_with_diagnostic("test texture color"),
            }),
            MockStorage::PlatformOpaque => FrameStorageRequest::PlatformOpaque {
                label: "mock-native-sample".to_owned(),
            },
        };

        self.pool
            .acquire_video_frame(FrameLeaseRequest {
                playback_generation: request.playback_generation,
                source_time_us: request.source_time_us,
                duration_us: Some(33_333),
                frame_index: Some(7),
                dimensions: FrameDimensions {
                    width: 320,
                    height: 180,
                },
                pixel_format: VideoPixelFormat::Nv12,
                color: VideoColorMetadata::unknown_with_diagnostic("test color metadata"),
                storage,
            })
            .map_err(|error| {
                DecodeError::new(
                    media_runtime::DecodeErrorKind::RuntimeFailure,
                    format!("mock frame pool failed: {error}"),
                )
            })
    }

    fn flush(&mut self) -> Result<(), DecodeError> {
        Ok(())
    }
}

fn video_stream() -> MediaStreamInfo {
    MediaStreamInfo {
        stream_id: StreamId(0),
        kind: MediaStreamKind::Video,
        codec: "h264".to_owned(),
        duration_us: Some(1_000_000),
        frame_rate: Some(RationalFrameRate {
            numerator: 30,
            denominator: 1,
        }),
        dimensions: Some(FrameDimensions {
            width: 320,
            height: 180,
        }),
        pixel_format: Some(VideoPixelFormat::Nv12),
        color: Some(VideoColorMetadata::unknown_with_diagnostic(
            "test stream color",
        )),
        sample_rate: None,
        channels: None,
    }
}

use std::path::PathBuf;

use media_runtime::{
    AudioDecodeRequest, AudioDecoder, ColorMatrix, ColorPrimaries, ColorRange, ColorTransfer,
    DecodeError, DecodeErrorKind, DecodedAudioFrame, DecodedVideoFrame, FrameDimensions,
    MediaIoError, MediaIoErrorKind, MediaOpenRequest, MediaReader, MediaSession, MediaSessionId,
    MediaStreamInfo, MediaStreamKind, RationalFrameRate, RuntimeDeviceId, StreamId, TextureBackend,
    TextureHandle, TextureHandleId, VideoColorMetadata, VideoDecodeRequest, VideoDecoder,
    VideoPixelFormat,
};

#[test]
fn media_io_contracts_fake_reader_opens_session_and_creates_decoder_trait_objects_without_ffmpeg_executor()
 {
    let reader: Box<dyn MediaReader> = Box::new(FakeReader);
    let session = reader
        .open(MediaOpenRequest {
            material_uri: PathBuf::from("/tmp/source.mp4"),
            requested_streams: vec![StreamId(0), StreamId(1)],
        })
        .expect("fake reader should open a media session");

    assert_eq!(reader.reader_name(), "fake-reader");
    assert_eq!(session.session_id(), MediaSessionId("session-1".to_owned()));
    assert_eq!(session.streams().len(), 2);
    assert_eq!(session.streams()[0].stream_id, StreamId(0));
    assert_eq!(session.streams()[0].kind, MediaStreamKind::Video);
    assert_eq!(session.streams()[0].duration_us, Some(2_000_000));
    assert_eq!(
        session.streams()[0].frame_rate,
        Some(RationalFrameRate {
            numerator: 30,
            denominator: 1
        })
    );

    let video: Box<dyn VideoDecoder> = session
        .video_decoder(StreamId(0))
        .expect("video decoder should be available as an object-safe trait");
    let audio: Box<dyn AudioDecoder> = session
        .audio_decoder(StreamId(1))
        .expect("audio decoder should be available as an object-safe trait");

    assert_eq!(video.decoder_name(), "fake-video-decoder");
    assert_eq!(audio.decoder_name(), "fake-audio-decoder");
}

#[test]
fn media_io_contracts_texture_handle_serializes_as_opaque_ids_and_metadata_without_native_pointer_fields()
 {
    let handle = TextureHandle {
        handle_id: TextureHandleId("texture-42".to_owned()),
        owner_session: MediaSessionId("session-1".to_owned()),
        generation: 7,
        backend: TextureBackend::MetalTexture,
        device_id: RuntimeDeviceId {
            backend: TextureBackend::MetalTexture,
            adapter_id: "apple-m2".to_owned(),
            device_id: "preview-device".to_owned(),
        },
        dimensions: FrameDimensions {
            width: 3840,
            height: 2160,
        },
        pixel_format: VideoPixelFormat::Nv12,
        color: VideoColorMetadata::unknown_with_diagnostic("sample attachments missing"),
    };

    let value = serde_json::to_value(&handle).expect("texture handle should serialize");

    assert_eq!(value["handleId"], "texture-42");
    assert_eq!(value["ownerSession"], "session-1");
    assert_eq!(value["generation"], 7);
    assert_eq!(value["backend"], "metalTexture");
    assert_eq!(value["deviceId"]["backend"], "metalTexture");
    assert_eq!(value["deviceId"]["adapterId"], "apple-m2");
    assert_eq!(value["dimensions"]["width"], 3840);
    assert_eq!(value["pixelFormat"], "nv12");
    assert_eq!(value["color"]["primaries"], "unknown");
    assert_eq!(
        value["color"]["diagnostics"][0]["message"],
        "sample attachments missing"
    );

    let object = value
        .as_object()
        .expect("texture should serialize to object");
    assert!(!object.contains_key("nativePointer"));
    assert!(!object.contains_key("rawHandle"));
    assert!(!object.contains_key("bytes"));
}

struct FakeReader;

impl MediaReader for FakeReader {
    fn reader_name(&self) -> &'static str {
        "fake-reader"
    }

    fn open(&self, _request: MediaOpenRequest) -> Result<Box<dyn MediaSession>, MediaIoError> {
        Ok(Box::new(FakeSession {
            streams: vec![
                MediaStreamInfo {
                    stream_id: StreamId(0),
                    kind: MediaStreamKind::Video,
                    codec: "h264".to_owned(),
                    duration_us: Some(2_000_000),
                    frame_rate: Some(RationalFrameRate {
                        numerator: 30,
                        denominator: 1,
                    }),
                    dimensions: Some(FrameDimensions {
                        width: 1920,
                        height: 1080,
                    }),
                    pixel_format: Some(VideoPixelFormat::Nv12),
                    color: Some(VideoColorMetadata {
                        primaries: ColorPrimaries::Bt709,
                        transfer: ColorTransfer::Bt709,
                        matrix: ColorMatrix::Bt709,
                        range: ColorRange::Limited,
                        diagnostics: Vec::new(),
                    }),
                    sample_rate: None,
                    channels: None,
                },
                MediaStreamInfo {
                    stream_id: StreamId(1),
                    kind: MediaStreamKind::Audio,
                    codec: "aac".to_owned(),
                    duration_us: Some(2_000_000),
                    frame_rate: None,
                    dimensions: None,
                    pixel_format: None,
                    color: None,
                    sample_rate: Some(48_000),
                    channels: Some(2),
                },
            ],
        }))
    }
}

struct FakeSession {
    streams: Vec<MediaStreamInfo>,
}

impl MediaSession for FakeSession {
    fn session_id(&self) -> MediaSessionId {
        MediaSessionId("session-1".to_owned())
    }

    fn streams(&self) -> &[MediaStreamInfo] {
        &self.streams
    }

    fn video_decoder(&self, stream_id: StreamId) -> Result<Box<dyn VideoDecoder>, MediaIoError> {
        if stream_id == StreamId(0) {
            Ok(Box::new(FakeVideoDecoder))
        } else {
            Err(MediaIoError::new(
                MediaIoErrorKind::StreamNotFound,
                "video stream not found",
            ))
        }
    }

    fn audio_decoder(&self, stream_id: StreamId) -> Result<Box<dyn AudioDecoder>, MediaIoError> {
        if stream_id == StreamId(1) {
            Ok(Box::new(FakeAudioDecoder))
        } else {
            Err(MediaIoError::new(
                MediaIoErrorKind::StreamNotFound,
                "audio stream not found",
            ))
        }
    }
}

struct FakeVideoDecoder;

impl VideoDecoder for FakeVideoDecoder {
    fn decoder_name(&self) -> &'static str {
        "fake-video-decoder"
    }

    fn decode_at(
        &mut self,
        _request: VideoDecodeRequest,
    ) -> Result<DecodedVideoFrame, DecodeError> {
        Err(DecodeError::new(
            DecodeErrorKind::Unsupported,
            "fake decoder does not decode frames",
        ))
    }

    fn flush(&mut self) -> Result<(), DecodeError> {
        Ok(())
    }
}

struct FakeAudioDecoder;

impl AudioDecoder for FakeAudioDecoder {
    fn decoder_name(&self) -> &'static str {
        "fake-audio-decoder"
    }

    fn read_range(
        &mut self,
        _request: AudioDecodeRequest,
    ) -> Result<DecodedAudioFrame, DecodeError> {
        Err(DecodeError::new(
            DecodeErrorKind::Unsupported,
            "fake decoder does not decode audio",
        ))
    }

    fn flush(&mut self) -> Result<(), DecodeError> {
        Ok(())
    }
}

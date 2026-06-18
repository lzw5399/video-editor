use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use media_runtime::{
    AudioDecoder, DecodeError, DecodeErrorKind, DecodedVideoFrame, FrameDimensions, FrameLeaseId,
    FrameLeaseRequest, FramePool, FramePoolCloseReport, FramePoolError, FramePoolLimits,
    FrameReleaseDiagnostic, FrameStorageRequest, MacosMediaIoCapabilities, MediaIoError,
    MediaIoErrorKind, MediaIoFallbackCandidate, MediaIoFallbackReason, MediaIoFallbackSelection,
    MediaOpenRequest, MediaReader, MediaSession, MediaSessionId, MediaStreamInfo, MediaStreamKind,
    RationalFrameRate, RuntimeCapabilityStatus, RuntimeDeviceId, RuntimeFeatureCapability,
    SelectedDecodePath, StreamId, TextureBackend, VideoColorMetadata, VideoDecodeRequest,
    VideoDecoder, VideoPixelFormat,
};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

const DEFAULT_MAX_OUTSTANDING_LEASES: usize = 8;

pub fn probe_macos_media_io_capabilities() -> MacosMediaIoCapabilities {
    platform_macos_capabilities()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacosTextureInteropPolicy {
    enabled: bool,
    preview_device: Option<RuntimeDeviceId>,
}

impl Default for MacosTextureInteropPolicy {
    fn default() -> Self {
        Self::disabled()
    }
}

impl MacosTextureInteropPolicy {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            preview_device: None,
        }
    }

    pub fn for_preview_device(preview_device: RuntimeDeviceId) -> Self {
        Self {
            enabled: true,
            preview_device: Some(preview_device),
        }
    }

    pub fn enabled_without_preview_device() -> Self {
        Self {
            enabled: true,
            preview_device: None,
        }
    }
}

pub fn select_macos_texture_interop_fallback(
    native_decode_available: bool,
    devices: Option<(&RuntimeDeviceId, &RuntimeDeviceId)>,
    texture_cache_available: bool,
) -> Option<MediaIoFallbackSelection> {
    let texture_candidate = if !native_decode_available {
        MediaIoFallbackCandidate::unavailable(
            SelectedDecodePath::NativeHardwareTexture,
            MediaIoFallbackReason::HardwareDecodeUnavailable,
            "macOS native decode is unavailable, so Metal texture interop cannot be selected",
        )
    } else if !texture_cache_available {
        MediaIoFallbackCandidate::unavailable(
            SelectedDecodePath::NativeHardwareTexture,
            MediaIoFallbackReason::TextureInteropUnavailable,
            "CVMetalTextureCache is unavailable or disabled for this decode session",
        )
    } else if let Some((preview_device, native_device)) = devices {
        if preview_device == native_device {
            MediaIoFallbackCandidate::available(SelectedDecodePath::NativeHardwareTexture)
        } else {
            MediaIoFallbackCandidate::unavailable(
                SelectedDecodePath::NativeHardwareTexture,
                MediaIoFallbackReason::DeviceMismatch,
                "preview Metal device identity does not match the native decode device",
            )
        }
    } else {
        MediaIoFallbackCandidate::unavailable(
            SelectedDecodePath::NativeHardwareTexture,
            MediaIoFallbackReason::TextureInteropUnavailable,
            "preview/native Metal device identity was not proven",
        )
    };
    let reason = texture_candidate
        .reason
        .unwrap_or(MediaIoFallbackReason::TextureInteropUnavailable);

    media_runtime::select_media_io_fallback(
        vec![
            texture_candidate,
            if native_decode_available {
                MediaIoFallbackCandidate::available(SelectedDecodePath::NativeHardwareCpuCopy)
            } else {
                MediaIoFallbackCandidate::unavailable(
                    SelectedDecodePath::NativeHardwareCpuCopy,
                    MediaIoFallbackReason::HardwareDecodeUnavailable,
                    "macOS native hardware frame fallback is unavailable",
                )
            },
            if native_decode_available {
                MediaIoFallbackCandidate::available(SelectedDecodePath::NativeSoftwareCpuFrame)
            } else {
                MediaIoFallbackCandidate::unavailable(
                    SelectedDecodePath::NativeSoftwareCpuFrame,
                    MediaIoFallbackReason::HardwareDecodeUnavailable,
                    "macOS native software frame fallback is unavailable",
                )
            },
            MediaIoFallbackCandidate::available(SelectedDecodePath::FfmpegCpuFrame),
            MediaIoFallbackCandidate::available(SelectedDecodePath::FfmpegPreviewArtifact),
        ],
        reason,
    )
}

#[derive(Debug, Clone)]
pub struct MacosMediaReader {
    frame_pool_limits: FramePoolLimits,
    texture_policy: MacosTextureInteropPolicy,
}

impl Default for MacosMediaReader {
    fn default() -> Self {
        Self::new()
    }
}

impl MacosMediaReader {
    pub fn new() -> Self {
        Self {
            frame_pool_limits: FramePoolLimits {
                max_outstanding_leases: DEFAULT_MAX_OUTSTANDING_LEASES,
            },
            texture_policy: MacosTextureInteropPolicy::default(),
        }
    }

    pub fn with_frame_pool_limits(mut self, limits: FramePoolLimits) -> Self {
        self.frame_pool_limits = limits;
        self
    }

    pub fn with_texture_interop_policy(mut self, policy: MacosTextureInteropPolicy) -> Self {
        self.texture_policy = policy;
        self
    }

    pub fn open_session(
        &self,
        request: MediaOpenRequest,
    ) -> Result<MacosMediaSession, MediaIoError> {
        platform_open_session(self, request)
    }
}

impl MediaReader for MacosMediaReader {
    fn reader_name(&self) -> &'static str {
        "macos-native-media-reader"
    }

    fn open(&self, request: MediaOpenRequest) -> Result<Box<dyn MediaSession>, MediaIoError> {
        Ok(Box::new(self.open_session(request)?))
    }
}

#[derive(Debug, Clone)]
pub struct MacosMediaSession {
    session_id: MediaSessionId,
    material_uri: PathBuf,
    streams: Vec<MediaStreamInfo>,
    frame_state: Rc<RefCell<MacosFrameState>>,
    texture_policy: MacosTextureInteropPolicy,
    last_fallback_selection: Rc<RefCell<Option<MediaIoFallbackSelection>>>,
}

impl MacosMediaSession {
    pub fn session_id(&self) -> MediaSessionId {
        self.session_id.clone()
    }

    pub fn native_video_decoder(
        &self,
        stream_id: StreamId,
    ) -> Result<MacosVideoDecoder, MediaIoError> {
        let stream = stream_by_id(&self.streams, stream_id)?;
        if stream.kind != MediaStreamKind::Video {
            return Err(MediaIoError::new(
                MediaIoErrorKind::UnsupportedStream,
                format!("stream {} is not a video stream", stream_id.0),
            ));
        }

        Ok(MacosVideoDecoder {
            material_uri: self.material_uri.clone(),
            stream: stream.clone(),
            frame_state: Rc::clone(&self.frame_state),
            texture_policy: self.texture_policy.clone(),
            last_fallback_selection: Rc::clone(&self.last_fallback_selection),
        })
    }

    pub fn release_frame(
        &self,
        lease_id: FrameLeaseId,
    ) -> Result<FrameReleaseDiagnostic, MediaIoError> {
        self.frame_state
            .borrow_mut()
            .release_frame(&self.session_id, lease_id)
            .map_err(media_error_from_frame_pool)
    }

    pub fn close(&self) -> FramePoolCloseReport {
        self.frame_state.borrow_mut().close()
    }

    pub fn outstanding_native_lease_count(&self) -> usize {
        self.frame_state.borrow().native_leases.len()
    }

    pub fn last_fallback_selection(&self) -> Option<MediaIoFallbackSelection> {
        self.last_fallback_selection.borrow().clone()
    }
}

impl MediaSession for MacosMediaSession {
    fn session_id(&self) -> MediaSessionId {
        self.session_id()
    }

    fn streams(&self) -> &[MediaStreamInfo] {
        &self.streams
    }

    fn video_decoder(&self, stream_id: StreamId) -> Result<Box<dyn VideoDecoder>, MediaIoError> {
        Ok(Box::new(self.native_video_decoder(stream_id)?))
    }

    fn audio_decoder(&self, stream_id: StreamId) -> Result<Box<dyn AudioDecoder>, MediaIoError> {
        let stream = stream_by_id(&self.streams, stream_id)?;
        Err(MediaIoError::new(
            MediaIoErrorKind::UnsupportedStream,
            format!(
                "macOS native audio decoder is not implemented for stream {} ({})",
                stream.stream_id.0, stream.codec
            ),
        ))
    }
}

#[derive(Debug)]
pub struct MacosVideoDecoder {
    material_uri: PathBuf,
    stream: MediaStreamInfo,
    frame_state: Rc<RefCell<MacosFrameState>>,
    texture_policy: MacosTextureInteropPolicy,
    last_fallback_selection: Rc<RefCell<Option<MediaIoFallbackSelection>>>,
}

impl MacosVideoDecoder {
    pub fn decode_at(
        &mut self,
        request: VideoDecodeRequest,
    ) -> Result<DecodedVideoFrame, DecodeError> {
        self.decode_native_frame(request)
    }

    fn decode_native_frame(
        &mut self,
        request: VideoDecodeRequest,
    ) -> Result<DecodedVideoFrame, DecodeError> {
        platform_decode_frame(self, request)
    }
}

impl VideoDecoder for MacosVideoDecoder {
    fn decoder_name(&self) -> &'static str {
        "macos-native-video-decoder"
    }

    fn decode_at(&mut self, request: VideoDecodeRequest) -> Result<DecodedVideoFrame, DecodeError> {
        self.decode_native_frame(request)
    }

    fn flush(&mut self) -> Result<(), DecodeError> {
        Ok(())
    }
}

#[derive(Debug)]
struct MacosFrameState {
    pool: FramePool,
    native_leases: BTreeMap<FrameLeaseId, MacosNativeLease>,
}

impl MacosFrameState {
    fn new(session_id: MediaSessionId, limits: FramePoolLimits) -> Self {
        Self {
            pool: FramePool::new(session_id, limits),
            native_leases: BTreeMap::new(),
        }
    }

    fn acquire_platform_frame(
        &mut self,
        request: FrameLeaseRequest,
        native_lease: MacosNativeLease,
    ) -> Result<DecodedVideoFrame, FramePoolError> {
        let frame = self.pool.acquire_video_frame(request)?;
        self.native_leases
            .insert(frame.release.clone(), native_lease);
        Ok(frame)
    }

    fn release_frame(
        &mut self,
        owner_session: &MediaSessionId,
        lease_id: FrameLeaseId,
    ) -> Result<FrameReleaseDiagnostic, FramePoolError> {
        let diagnostic = self
            .pool
            .release_for_session(owner_session, lease_id.clone())?;
        self.native_leases.remove(&lease_id);
        Ok(diagnostic)
    }

    fn close(&mut self) -> FramePoolCloseReport {
        self.native_leases.clear();
        self.pool.close_session()
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug)]
struct MacosNativeLease {
    _sample_buffer: objc2::rc::Retained<objc2_core_media::CMSampleBuffer>,
    _metal_texture: Option<objc2_core_foundation::CFRetained<objc2_core_video::CVMetalTexture>>,
}

#[cfg(not(target_os = "macos"))]
#[derive(Debug)]
struct MacosNativeLease;

#[cfg(target_os = "macos")]
fn platform_macos_capabilities() -> MacosMediaIoCapabilities {
    MacosMediaIoCapabilities {
        status: RuntimeCapabilityStatus::Warning,
        av_foundation: ready_feature("AVFoundation"),
        video_toolbox: ready_feature("VideoToolbox"),
        core_video: ready_feature("CoreVideo"),
        metal_texture_interop: warning_feature(
            "Metal texture interop",
            "Metal texture interop is available only after preview/native device identity is proven",
        ),
        fallback_reason: Some(MediaIoFallbackReason::TextureInteropUnavailable),
        diagnostic: Some(
            "macOS native H.264/CoreVideo decode is available; Metal texture interop remains gated by device compatibility."
                .to_owned(),
        ),
    }
}

#[cfg(not(target_os = "macos"))]
fn platform_macos_capabilities() -> MacosMediaIoCapabilities {
    MacosMediaIoCapabilities {
        status: RuntimeCapabilityStatus::Unavailable,
        av_foundation: unsupported_feature("AVFoundation"),
        video_toolbox: unsupported_feature("VideoToolbox"),
        core_video: unsupported_feature("CoreVideo"),
        metal_texture_interop: unsupported_feature("Metal texture interop"),
        fallback_reason: Some(MediaIoFallbackReason::UnsupportedPlatform),
        diagnostic: Some(
            "macOS AVFoundation/VideoToolbox/CoreVideo/Metal media IO is unavailable on this unsupported platform."
                .to_owned(),
        ),
    }
}

#[cfg(target_os = "macos")]
fn ready_feature(name: &str) -> RuntimeFeatureCapability {
    RuntimeFeatureCapability {
        name: name.to_owned(),
        available: true,
        status: RuntimeCapabilityStatus::Ready,
        diagnostic: None,
    }
}

#[cfg(target_os = "macos")]
fn warning_feature(name: &str, diagnostic: &str) -> RuntimeFeatureCapability {
    RuntimeFeatureCapability {
        name: name.to_owned(),
        available: false,
        status: RuntimeCapabilityStatus::Warning,
        diagnostic: Some(diagnostic.to_owned()),
    }
}

#[cfg(not(target_os = "macos"))]
fn unsupported_feature(name: &str) -> RuntimeFeatureCapability {
    RuntimeFeatureCapability {
        name: name.to_owned(),
        available: false,
        status: RuntimeCapabilityStatus::Unavailable,
        diagnostic: Some(format!(
            "{name} capability is unavailable on this unsupported platform"
        )),
    }
}

#[cfg(not(target_os = "macos"))]
fn platform_open_session(
    _reader: &MacosMediaReader,
    _request: MediaOpenRequest,
) -> Result<MacosMediaSession, MediaIoError> {
    Err(MediaIoError::new(
        MediaIoErrorKind::RuntimeUnavailable,
        "UnsupportedPlatform: macOS native media IO requires target_os=macos",
    ))
}

#[cfg(not(target_os = "macos"))]
fn platform_decode_frame(
    _decoder: &mut MacosVideoDecoder,
    _request: VideoDecodeRequest,
) -> Result<DecodedVideoFrame, DecodeError> {
    Err(DecodeError::new(
        DecodeErrorKind::Unsupported,
        "UnsupportedPlatform: macOS native media IO requires target_os=macos",
    ))
}

#[cfg(target_os = "macos")]
fn platform_open_session(
    reader: &MacosMediaReader,
    request: MediaOpenRequest,
) -> Result<MacosMediaSession, MediaIoError> {
    ensure_input_file(&request.material_uri)?;
    let stream = probe_first_video_stream(&request.material_uri)?;
    validate_requested_streams(std::slice::from_ref(&stream), &request.requested_streams)?;
    let session_id = next_session_id();

    Ok(MacosMediaSession {
        session_id: session_id.clone(),
        material_uri: request.material_uri,
        streams: vec![stream],
        frame_state: Rc::new(RefCell::new(MacosFrameState::new(
            session_id,
            reader.frame_pool_limits.clone(),
        ))),
        texture_policy: reader.texture_policy.clone(),
        last_fallback_selection: Rc::new(RefCell::new(None)),
    })
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn platform_decode_frame(
    decoder: &mut MacosVideoDecoder,
    request: VideoDecodeRequest,
) -> Result<DecodedVideoFrame, DecodeError> {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_av_foundation::{
        AVAssetReader, AVAssetReaderOutput, AVAssetReaderTrackOutput, AVMediaTypeVideo,
    };
    use objc2_core_media::CMSampleBuffer;
    use objc2_core_video::{
        CVPixelBufferGetHeight, CVPixelBufferGetPixelFormatType, CVPixelBufferGetWidth,
    };
    use objc2_foundation::{NSDictionary, NSNumber, NSString};

    let asset = asset_from_path(&decoder.material_uri).map_err(decode_error_from_media)?;
    let video_media_type = unsafe { AVMediaTypeVideo }
        .ok_or_else(|| platform_decode_error("AVMediaTypeVideo is unavailable"))?;
    let tracks = unsafe { asset.tracksWithMediaType(video_media_type) };
    let track = tracks
        .firstObject()
        .ok_or_else(|| platform_decode_error("AVFoundation reported no video track"))?;
    let reader = unsafe { AVAssetReader::assetReaderWithAsset_error(&asset) }
        .map_err(|error| platform_decode_error(format!("AVAssetReader failed: {error:?}")))?;
    let output_settings = output_settings_nv12();
    let output_settings_ref: &NSDictionary<NSString, AnyObject> = unsafe {
        &*((&*output_settings as *const NSDictionary<NSString, NSNumber>)
            as *const NSDictionary<NSString, AnyObject>)
    };
    let output = unsafe {
        AVAssetReaderTrackOutput::assetReaderTrackOutputWithTrack_outputSettings(
            &track,
            Some(output_settings_ref),
        )
    };
    unsafe {
        output.setAlwaysCopiesSampleData(false);
    }
    let output_ref: &AVAssetReaderOutput = output.as_ref();
    if !unsafe { reader.canAddOutput(output_ref) } {
        return Err(platform_decode_error(
            "AVAssetReader cannot add the CoreVideo track output",
        ));
    }
    unsafe {
        reader.addOutput(output_ref);
    }
    if !unsafe { reader.startReading() } {
        return Err(platform_decode_error(format!(
            "AVAssetReader startReading failed: status={:?} error={:?}",
            unsafe { reader.status() },
            unsafe { reader.error() }
        )));
    }

    let raw_sample_buffer: *mut CMSampleBuffer =
        unsafe { objc2::msg_send![output_ref, copyNextSampleBuffer] };
    let sample_buffer = unsafe { Retained::from_raw(raw_sample_buffer) }
        .ok_or_else(|| platform_decode_error("AVAssetReader produced no sample buffer"))?;
    let image_buffer = unsafe { sample_buffer.image_buffer() }
        .ok_or_else(|| platform_decode_error("CMSampleBuffer did not contain a CVImageBuffer"))?;
    let pixel_buffer = image_buffer.as_ref();
    let dimensions = FrameDimensions {
        width: u32::try_from(CVPixelBufferGetWidth(pixel_buffer)).map_err(|_| {
            platform_decode_error("CVPixelBuffer width does not fit the frame contract")
        })?,
        height: u32::try_from(CVPixelBufferGetHeight(pixel_buffer)).map_err(|_| {
            platform_decode_error("CVPixelBuffer height does not fit the frame contract")
        })?,
    };
    let cv_pixel_format = CVPixelBufferGetPixelFormatType(pixel_buffer);
    let pixel_format = cv_pixel_format_to_video_pixel_format(cv_pixel_format);
    if pixel_format != VideoPixelFormat::Nv12 {
        return Err(DecodeError::new(
            DecodeErrorKind::Unsupported,
            format!(
                "UnsupportedPixelFormat: AVFoundation returned unsupported CV pixel format {cv_pixel_format:#x}"
            ),
        ));
    }

    let native_device = if decoder.texture_policy.enabled {
        system_metal_device_id()
    } else {
        None
    };
    let devices = decoder
        .texture_policy
        .preview_device
        .as_ref()
        .zip(native_device.as_ref());
    let texture_cache_available = false;
    let fallback_selection =
        select_macos_texture_interop_fallback(true, devices, texture_cache_available)
            .ok_or_else(|| platform_decode_error("macOS fallback ladder had no available path"))?;
    *decoder.last_fallback_selection.borrow_mut() = Some(fallback_selection.clone());

    let source_time_us =
        cm_time_to_microseconds(unsafe { sample_buffer.presentation_time_stamp() })
            .unwrap_or(request.source_time_us);
    let duration_us = decoder
        .stream
        .frame_rate
        .and_then(frame_duration_us)
        .or(Some(100_000));
    let frame_index = decoder
        .stream
        .frame_rate
        .and_then(|rate| frame_index_at(source_time_us, rate));
    let color = decoder.stream.color.clone().unwrap_or_else(|| {
        VideoColorMetadata::unknown_with_diagnostic(
            "AVFoundation/CoreVideo did not expose complete color attachments for this frame",
        )
    });
    let storage = FrameStorageRequest::PlatformOpaque {
        label: format!("CoreVideoPixelBuffer({cv_pixel_format:#x})"),
    };
    let native_lease = MacosNativeLease {
        _sample_buffer: sample_buffer,
        _metal_texture: None,
    };

    decoder
        .frame_state
        .borrow_mut()
        .acquire_platform_frame(
            FrameLeaseRequest {
                playback_generation: request.playback_generation,
                source_time_us,
                duration_us,
                frame_index,
                dimensions,
                pixel_format,
                color,
                storage,
            },
            native_lease,
        )
        .map_err(decode_error_from_frame_pool)
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn probe_first_video_stream(path: &Path) -> Result<MediaStreamInfo, MediaIoError> {
    use objc2_av_foundation::AVMediaTypeVideo;

    let asset = asset_from_path(path)?;
    let video_media_type = unsafe { AVMediaTypeVideo }.ok_or_else(|| {
        MediaIoError::new(
            MediaIoErrorKind::RuntimeUnavailable,
            "AVMediaTypeVideo is unavailable",
        )
    })?;
    let tracks = unsafe { asset.tracksWithMediaType(video_media_type) };
    let track = tracks.firstObject().ok_or_else(|| {
        MediaIoError::new(
            MediaIoErrorKind::StreamNotFound,
            "AVFoundation reported no video track",
        )
    })?;
    if !unsafe { track.isDecodable() } {
        return Err(MediaIoError::new(
            MediaIoErrorKind::UnsupportedStream,
            "UnsupportedCodec: AVFoundation reports the first video track is not decodable",
        ));
    }

    let dimensions = unsafe { track.naturalSize() };
    let width = dimensions.width.round().max(0.0) as u32;
    let height = dimensions.height.round().max(0.0) as u32;
    let nominal_frame_rate = unsafe { track.nominalFrameRate() };

    Ok(MediaStreamInfo {
        stream_id: StreamId(0),
        kind: MediaStreamKind::Video,
        codec: "h264".to_owned(),
        duration_us: None,
        frame_rate: rational_frame_rate_from_f32(nominal_frame_rate),
        dimensions: Some(FrameDimensions { width, height }),
        pixel_format: Some(VideoPixelFormat::Nv12),
        color: Some(VideoColorMetadata::unknown_with_diagnostic(
            "AVFoundation probe did not expose complete color metadata",
        )),
        sample_rate: None,
        channels: None,
    })
}

#[cfg(target_os = "macos")]
fn asset_from_path(
    path: &Path,
) -> Result<objc2::rc::Retained<objc2_av_foundation::AVAsset>, MediaIoError> {
    use objc2_av_foundation::AVAsset;
    use objc2_foundation::{NSString, NSURL};

    let path = path.to_str().ok_or_else(|| {
        MediaIoError::new(
            MediaIoErrorKind::OpenFailed,
            "material path is not valid UTF-8 for NSURL",
        )
    })?;
    let path = NSString::from_str(path);
    let url = NSURL::fileURLWithPath(&path);
    Ok(unsafe { AVAsset::assetWithURL(&url) })
}

#[cfg(target_os = "macos")]
fn output_settings_nv12() -> objc2::rc::Retained<
    objc2_foundation::NSDictionary<objc2_foundation::NSString, objc2_foundation::NSNumber>,
> {
    use objc2_core_video::kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange;
    use objc2_foundation::{NSDictionary, NSNumber, NSString};

    let key = NSString::from_str("PixelFormatType");
    let value = NSNumber::numberWithUnsignedInt(kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange);
    NSDictionary::from_slices(&[&*key], &[&*value])
}

#[cfg(target_os = "macos")]
fn cv_pixel_format_to_video_pixel_format(format: u32) -> VideoPixelFormat {
    match format {
        objc2_core_video::kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange
        | objc2_core_video::kCVPixelFormatType_420YpCbCr8BiPlanarFullRange => {
            VideoPixelFormat::Nv12
        }
        objc2_core_video::kCVPixelFormatType_32BGRA => VideoPixelFormat::Bgra8,
        _ => VideoPixelFormat::Unknown,
    }
}

#[cfg(target_os = "macos")]
fn system_metal_device_id() -> Option<RuntimeDeviceId> {
    use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice};

    let device = MTLCreateSystemDefaultDevice()?;
    let registry_id = device.registryID();
    Some(RuntimeDeviceId {
        backend: TextureBackend::MetalTexture,
        adapter_id: "apple-metal".to_owned(),
        device_id: format!("registry-{registry_id}"),
    })
}

fn ensure_input_file(path: &Path) -> Result<(), MediaIoError> {
    if path.is_file() {
        Ok(())
    } else {
        Err(MediaIoError::new(
            MediaIoErrorKind::OpenFailed,
            format!(
                "material path does not exist or is not a file: {}",
                path.display()
            ),
        ))
    }
}

fn validate_requested_streams(
    streams: &[MediaStreamInfo],
    requested: &[StreamId],
) -> Result<(), MediaIoError> {
    for stream_id in requested {
        stream_by_id(streams, *stream_id)?;
    }
    Ok(())
}

fn stream_by_id(
    streams: &[MediaStreamInfo],
    stream_id: StreamId,
) -> Result<&MediaStreamInfo, MediaIoError> {
    streams
        .iter()
        .find(|stream| stream.stream_id == stream_id)
        .ok_or_else(|| {
            MediaIoError::new(
                MediaIoErrorKind::StreamNotFound,
                format!("stream {} not found", stream_id.0),
            )
        })
}

fn next_session_id() -> MediaSessionId {
    MediaSessionId(format!(
        "macos-native-session-{}",
        NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

fn media_error_from_frame_pool(error: FramePoolError) -> MediaIoError {
    MediaIoError::new(
        MediaIoErrorKind::RuntimeUnavailable,
        format!("macOS native frame pool operation failed: {error}"),
    )
}

fn decode_error_from_frame_pool(error: FramePoolError) -> DecodeError {
    DecodeError::new(
        DecodeErrorKind::RuntimeFailure,
        format!("failed to acquire macOS native frame lease: {error}"),
    )
}

fn decode_error_from_media(error: MediaIoError) -> DecodeError {
    DecodeError::new(error_decode_kind(error.kind), error.message)
}

fn error_decode_kind(kind: MediaIoErrorKind) -> DecodeErrorKind {
    match kind {
        MediaIoErrorKind::UnsupportedStream | MediaIoErrorKind::StreamNotFound => {
            DecodeErrorKind::Unsupported
        }
        MediaIoErrorKind::OpenFailed => DecodeErrorKind::InvalidRequest,
        MediaIoErrorKind::RuntimeUnavailable => DecodeErrorKind::RuntimeFailure,
    }
}

fn platform_decode_error(message: impl Into<String>) -> DecodeError {
    DecodeError::new(
        DecodeErrorKind::RuntimeFailure,
        format!("PlatformApiFailure: {}", message.into()),
    )
}

fn frame_duration_us(frame_rate: RationalFrameRate) -> Option<u64> {
    if frame_rate.numerator == 0 {
        return None;
    }
    Some(
        1_000_000_u64.saturating_mul(u64::from(frame_rate.denominator))
            / u64::from(frame_rate.numerator),
    )
}

fn frame_index_at(source_time_us: u64, frame_rate: RationalFrameRate) -> Option<u64> {
    if frame_rate.denominator == 0 {
        return None;
    }
    Some(
        source_time_us.saturating_mul(u64::from(frame_rate.numerator))
            / 1_000_000_u64.saturating_mul(u64::from(frame_rate.denominator)),
    )
}

fn rational_frame_rate_from_f32(value: f32) -> Option<RationalFrameRate> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let rounded = value.round();
    if (value - rounded).abs() < f32::EPSILON {
        return Some(RationalFrameRate {
            numerator: rounded as u32,
            denominator: 1,
        });
    }
    Some(RationalFrameRate {
        numerator: (value * 1_000.0).round() as u32,
        denominator: 1_000,
    })
}

#[cfg(target_os = "macos")]
fn cm_time_to_microseconds(time: objc2_core_media::CMTime) -> Option<u64> {
    if time.timescale <= 0 || time.value < 0 {
        return None;
    }
    Some((time.value as u64).saturating_mul(1_000_000) / time.timescale as u64)
}

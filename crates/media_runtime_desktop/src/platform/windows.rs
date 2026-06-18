#![cfg_attr(not(windows), allow(dead_code, unused_imports))]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use media_runtime::{
    AudioDecoder, DecodeError, DecodeErrorKind, DecodedVideoFrame, FrameDimensions, FrameLeaseId,
    FrameLeaseRequest, FramePool, FramePoolCloseReport, FramePoolError, FramePoolLimits,
    FrameReleaseDiagnostic, FrameStorageRequest, MediaIoError, MediaIoErrorKind,
    MediaIoFallbackCandidate, MediaIoFallbackReason, MediaIoFallbackSelection, MediaOpenRequest,
    MediaReader, MediaSession, MediaSessionId, MediaStreamInfo, MediaStreamKind, RationalFrameRate,
    RuntimeCapabilityStatus, RuntimeDeviceId, RuntimeFeatureCapability, SelectedDecodePath,
    StreamId, TextureBackend, VideoColorMetadata, VideoDecodeRequest, VideoDecoder,
    VideoPixelFormat, WindowsMediaIoCapabilities,
};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

const DEFAULT_MAX_OUTSTANDING_LEASES: usize = 8;

pub fn probe_windows_media_io_capabilities() -> WindowsMediaIoCapabilities {
    platform_windows_capabilities()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsTextureInteropPolicy {
    enabled: bool,
    preview_device: Option<RuntimeDeviceId>,
}

impl Default for WindowsTextureInteropPolicy {
    fn default() -> Self {
        Self::disabled()
    }
}

impl WindowsTextureInteropPolicy {
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

pub fn select_windows_texture_interop_fallback(
    native_decode_available: bool,
    devices: Option<(&RuntimeDeviceId, &RuntimeDeviceId)>,
    texture_interop_available: bool,
) -> Option<MediaIoFallbackSelection> {
    let texture_candidate = if !native_decode_available {
        MediaIoFallbackCandidate::unavailable(
            SelectedDecodePath::NativeHardwareTexture,
            MediaIoFallbackReason::HardwareDecodeUnavailable,
            "Windows native decode is unavailable, so D3D texture interop cannot be selected",
        )
    } else if !texture_interop_available {
        MediaIoFallbackCandidate::unavailable(
            SelectedDecodePath::NativeHardwareTexture,
            MediaIoFallbackReason::TextureInteropUnavailable,
            "D3D texture interop is unavailable or disabled for this decode session",
        )
    } else if let Some((preview_device, native_device)) = devices {
        if preview_device == native_device {
            MediaIoFallbackCandidate::available(SelectedDecodePath::NativeHardwareTexture)
        } else {
            MediaIoFallbackCandidate::unavailable(
                SelectedDecodePath::NativeHardwareTexture,
                MediaIoFallbackReason::DeviceMismatch,
                "preview D3D device identity does not match the native decode device",
            )
        }
    } else {
        MediaIoFallbackCandidate::unavailable(
            SelectedDecodePath::NativeHardwareTexture,
            MediaIoFallbackReason::TextureInteropUnavailable,
            "preview/native D3D device identity was not proven",
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
                    "Windows native hardware frame fallback is unavailable",
                )
            },
            if native_decode_available {
                MediaIoFallbackCandidate::available(SelectedDecodePath::NativeSoftwareCpuFrame)
            } else {
                MediaIoFallbackCandidate::unavailable(
                    SelectedDecodePath::NativeSoftwareCpuFrame,
                    MediaIoFallbackReason::HardwareDecodeUnavailable,
                    "Windows native software frame fallback is unavailable",
                )
            },
            MediaIoFallbackCandidate::available(SelectedDecodePath::FfmpegCpuFrame),
            MediaIoFallbackCandidate::available(SelectedDecodePath::FfmpegPreviewArtifact),
        ],
        reason,
    )
}

#[derive(Debug, Clone)]
pub struct WindowsMediaReader {
    frame_pool_limits: FramePoolLimits,
    texture_policy: WindowsTextureInteropPolicy,
}

impl Default for WindowsMediaReader {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsMediaReader {
    pub fn new() -> Self {
        Self {
            frame_pool_limits: FramePoolLimits {
                max_outstanding_leases: DEFAULT_MAX_OUTSTANDING_LEASES,
            },
            texture_policy: WindowsTextureInteropPolicy::default(),
        }
    }

    pub fn with_frame_pool_limits(mut self, limits: FramePoolLimits) -> Self {
        self.frame_pool_limits = limits;
        self
    }

    pub fn with_texture_interop_policy(mut self, policy: WindowsTextureInteropPolicy) -> Self {
        self.texture_policy = policy;
        self
    }

    pub fn open_session(
        &self,
        request: MediaOpenRequest,
    ) -> Result<WindowsMediaSession, MediaIoError> {
        platform_open_session(self, request)
    }
}

impl MediaReader for WindowsMediaReader {
    fn reader_name(&self) -> &'static str {
        "windows-native-media-reader"
    }

    fn open(&self, request: MediaOpenRequest) -> Result<Box<dyn MediaSession>, MediaIoError> {
        Ok(Box::new(self.open_session(request)?))
    }
}

#[derive(Debug, Clone)]
pub struct WindowsMediaSession {
    session_id: MediaSessionId,
    material_uri: PathBuf,
    streams: Vec<MediaStreamInfo>,
    frame_state: Rc<RefCell<WindowsFrameState>>,
    texture_policy: WindowsTextureInteropPolicy,
    last_fallback_selection: Rc<RefCell<Option<MediaIoFallbackSelection>>>,
}

impl WindowsMediaSession {
    pub fn session_id(&self) -> MediaSessionId {
        self.session_id.clone()
    }

    pub fn native_video_decoder(
        &self,
        stream_id: StreamId,
    ) -> Result<WindowsVideoDecoder, MediaIoError> {
        let stream = stream_by_id(&self.streams, stream_id)?;
        if stream.kind != MediaStreamKind::Video {
            return Err(MediaIoError::new(
                MediaIoErrorKind::UnsupportedStream,
                format!("stream {} is not a video stream", stream_id.0),
            ));
        }

        Ok(WindowsVideoDecoder {
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

impl MediaSession for WindowsMediaSession {
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
                "Windows native audio decoder is not implemented for stream {} ({})",
                stream.stream_id.0, stream.codec
            ),
        ))
    }
}

#[derive(Debug)]
pub struct WindowsVideoDecoder {
    material_uri: PathBuf,
    stream: MediaStreamInfo,
    frame_state: Rc<RefCell<WindowsFrameState>>,
    texture_policy: WindowsTextureInteropPolicy,
    last_fallback_selection: Rc<RefCell<Option<MediaIoFallbackSelection>>>,
}

impl WindowsVideoDecoder {
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

impl VideoDecoder for WindowsVideoDecoder {
    fn decoder_name(&self) -> &'static str {
        "windows-native-video-decoder"
    }

    fn decode_at(&mut self, request: VideoDecodeRequest) -> Result<DecodedVideoFrame, DecodeError> {
        self.decode_native_frame(request)
    }

    fn flush(&mut self) -> Result<(), DecodeError> {
        Ok(())
    }
}

#[derive(Debug)]
struct WindowsFrameState {
    pool: FramePool,
    native_leases: BTreeMap<FrameLeaseId, WindowsNativeLease>,
}

impl WindowsFrameState {
    fn new(session_id: MediaSessionId, limits: FramePoolLimits) -> Self {
        Self {
            pool: FramePool::new(session_id, limits),
            native_leases: BTreeMap::new(),
        }
    }

    fn acquire_platform_frame(
        &mut self,
        request: FrameLeaseRequest,
        native_lease: WindowsNativeLease,
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

#[cfg(windows)]
#[derive(Debug)]
struct WindowsNativeLease {
    _sample: windows::Win32::Media::MediaFoundation::IMFSample,
}

#[cfg(not(windows))]
#[derive(Debug)]
struct WindowsNativeLease;

#[cfg(windows)]
fn platform_windows_capabilities() -> WindowsMediaIoCapabilities {
    WindowsMediaIoCapabilities {
        status: RuntimeCapabilityStatus::Warning,
        media_foundation: ready_feature("Media Foundation"),
        dxva: warning_feature(
            "DXVA",
            "DXVA decode is used when Media Foundation returns hardware-backed samples; texture readiness remains gated by device compatibility.",
        ),
        d3d_texture_interop: warning_feature(
            "D3D texture interop",
            "D3D texture interop is available only after preview/native device identity is proven.",
        ),
        fallback_reason: Some(MediaIoFallbackReason::TextureInteropUnavailable),
        diagnostic: Some(
            "Windows Media Foundation source reading is available; D3D texture interop remains gated by device compatibility."
                .to_owned(),
        ),
    }
}

#[cfg(not(windows))]
fn platform_windows_capabilities() -> WindowsMediaIoCapabilities {
    WindowsMediaIoCapabilities {
        status: RuntimeCapabilityStatus::Unavailable,
        media_foundation: unsupported_feature("Media Foundation"),
        dxva: unsupported_feature("DXVA"),
        d3d_texture_interop: unsupported_feature("D3D texture interop"),
        fallback_reason: Some(MediaIoFallbackReason::UnsupportedPlatform),
        diagnostic: Some(
            "Windows Media Foundation/DXVA/D3D media IO is unavailable on this unsupported platform."
                .to_owned(),
        ),
    }
}

#[cfg(windows)]
fn ready_feature(name: &str) -> RuntimeFeatureCapability {
    RuntimeFeatureCapability {
        name: name.to_owned(),
        available: true,
        status: RuntimeCapabilityStatus::Ready,
        diagnostic: None,
    }
}

#[cfg(windows)]
fn warning_feature(name: &str, diagnostic: &str) -> RuntimeFeatureCapability {
    RuntimeFeatureCapability {
        name: name.to_owned(),
        available: false,
        status: RuntimeCapabilityStatus::Warning,
        diagnostic: Some(diagnostic.to_owned()),
    }
}

#[cfg(not(windows))]
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

#[cfg(not(windows))]
fn platform_open_session(
    _reader: &WindowsMediaReader,
    _request: MediaOpenRequest,
) -> Result<WindowsMediaSession, MediaIoError> {
    Err(MediaIoError::new(
        MediaIoErrorKind::RuntimeUnavailable,
        "UnsupportedPlatform: Windows native media IO requires cfg(windows)",
    ))
}

#[cfg(not(windows))]
fn platform_decode_frame(
    _decoder: &mut WindowsVideoDecoder,
    _request: VideoDecodeRequest,
) -> Result<DecodedVideoFrame, DecodeError> {
    Err(DecodeError::new(
        DecodeErrorKind::Unsupported,
        "UnsupportedPlatform: Windows native media IO requires cfg(windows)",
    ))
}

#[cfg(windows)]
fn platform_open_session(
    reader: &WindowsMediaReader,
    request: MediaOpenRequest,
) -> Result<WindowsMediaSession, MediaIoError> {
    ensure_input_file(&request.material_uri)?;
    let stream = probe_first_video_stream(&request.material_uri)?;
    validate_requested_streams(std::slice::from_ref(&stream), &request.requested_streams)?;
    let session_id = next_session_id();

    Ok(WindowsMediaSession {
        session_id: session_id.clone(),
        material_uri: request.material_uri,
        streams: vec![stream],
        frame_state: Rc::new(RefCell::new(WindowsFrameState::new(
            session_id,
            reader.frame_pool_limits.clone(),
        ))),
        texture_policy: reader.texture_policy.clone(),
        last_fallback_selection: Rc::new(RefCell::new(None)),
    })
}

#[cfg(windows)]
fn platform_decode_frame(
    decoder: &mut WindowsVideoDecoder,
    request: VideoDecodeRequest,
) -> Result<DecodedVideoFrame, DecodeError> {
    let source_reader =
        create_source_reader(&decoder.material_uri).map_err(decode_error_from_media)?;
    configure_source_reader_nv12(&source_reader).map_err(decode_error_from_media)?;
    let media_type = current_video_media_type(&source_reader).map_err(decode_error_from_media)?;
    let dimensions = media_type_dimensions(&media_type).map_err(decode_error_from_media)?;
    let pixel_format = media_type_pixel_format(&media_type).map_err(decode_error_from_media)?;
    if !matches!(
        pixel_format,
        VideoPixelFormat::Nv12 | VideoPixelFormat::Bgra8
    ) {
        return Err(DecodeError::new(
            DecodeErrorKind::Unsupported,
            format!("UnsupportedPixelFormat: Media Foundation returned {pixel_format:?}"),
        ));
    }

    let sample = read_first_video_sample(&source_reader)?;
    let sample_time_us = sample_time_us(&sample).unwrap_or(request.source_time_us);
    let duration_us = sample_duration_us(&sample).or_else(|| {
        decoder
            .stream
            .frame_rate
            .and_then(frame_duration_us)
            .or(Some(100_000))
    });
    let frame_index = decoder
        .stream
        .frame_rate
        .and_then(|rate| frame_index_at(sample_time_us, rate));
    let native_device = if decoder.texture_policy.enabled {
        system_d3d_device_id()
    } else {
        None
    };
    let devices = decoder
        .texture_policy
        .preview_device
        .as_ref()
        .zip(native_device.as_ref());
    let texture_interop_available = false;
    let fallback_selection =
        select_windows_texture_interop_fallback(true, devices, texture_interop_available)
            .ok_or_else(|| {
                platform_decode_error("Windows fallback ladder had no available path")
            })?;
    *decoder.last_fallback_selection.borrow_mut() = Some(fallback_selection);

    let color = decoder.stream.color.clone().unwrap_or_else(|| {
        VideoColorMetadata::unknown_with_diagnostic(
            "Media Foundation did not expose complete color metadata for this frame",
        )
    });
    let storage = FrameStorageRequest::PlatformOpaque {
        label: format!("MediaFoundationSample({pixel_format:?})"),
    };
    let native_lease = WindowsNativeLease { _sample: sample };

    decoder
        .frame_state
        .borrow_mut()
        .acquire_platform_frame(
            FrameLeaseRequest {
                playback_generation: request.playback_generation,
                source_time_us: sample_time_us,
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

#[cfg(windows)]
fn probe_first_video_stream(path: &Path) -> Result<MediaStreamInfo, MediaIoError> {
    let source_reader = create_source_reader(path)?;
    configure_source_reader_nv12(&source_reader)?;
    let media_type = current_video_media_type(&source_reader)?;
    let dimensions = media_type_dimensions(&media_type)?;
    let pixel_format = media_type_pixel_format(&media_type)?;
    let frame_rate = media_type_frame_rate(&media_type);

    Ok(MediaStreamInfo {
        stream_id: StreamId(0),
        kind: MediaStreamKind::Video,
        codec: "h264".to_owned(),
        duration_us: None,
        frame_rate,
        dimensions: Some(dimensions),
        pixel_format: Some(pixel_format),
        color: Some(VideoColorMetadata::unknown_with_diagnostic(
            "Media Foundation probe did not expose complete color metadata",
        )),
        sample_rate: None,
        channels: None,
    })
}

#[cfg(windows)]
fn create_source_reader(
    path: &Path,
) -> Result<windows::Win32::Media::MediaFoundation::IMFSourceReader, MediaIoError> {
    use std::iter;
    use std::os::windows::ffi::OsStrExt;

    use windows::Win32::Media::MediaFoundation::{
        MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, MF_VERSION, MFCreateAttributes,
        MFCreateSourceReaderFromURL, MFSTARTUP_FULL, MFStartup,
    };
    use windows::core::PCWSTR;

    let mut attributes = None;
    unsafe {
        MFStartup(MF_VERSION, MFSTARTUP_FULL).map_err(media_error_from_windows)?;
        MFCreateAttributes(&mut attributes, 1).map_err(media_error_from_windows)?;
    }
    if let Some(attributes) = attributes.as_ref() {
        unsafe {
            attributes
                .SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1)
                .map_err(media_error_from_windows)?;
        }
    }

    let path_wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    unsafe {
        MFCreateSourceReaderFromURL(PCWSTR(path_wide.as_ptr()), attributes.as_ref())
            .map_err(media_error_from_windows)
    }
}

#[cfg(windows)]
fn configure_source_reader_nv12(
    source_reader: &windows::Win32::Media::MediaFoundation::IMFSourceReader,
) -> Result<(), MediaIoError> {
    use windows::Win32::Media::MediaFoundation::{
        MF_MT_MAJOR_TYPE, MF_MT_SUBTYPE, MFCreateMediaType, MFMediaType_Video, MFVideoFormat_NV12,
    };

    let media_type = unsafe { MFCreateMediaType().map_err(media_error_from_windows)? };
    unsafe {
        media_type
            .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
            .map_err(media_error_from_windows)?;
        media_type
            .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)
            .map_err(media_error_from_windows)?;
        source_reader
            .SetCurrentMediaType(first_video_stream(), None, &media_type)
            .map_err(media_error_from_windows)?;
    }
    Ok(())
}

#[cfg(windows)]
fn current_video_media_type(
    source_reader: &windows::Win32::Media::MediaFoundation::IMFSourceReader,
) -> Result<windows::Win32::Media::MediaFoundation::IMFMediaType, MediaIoError> {
    unsafe {
        source_reader
            .GetCurrentMediaType(first_video_stream())
            .map_err(media_error_from_windows)
    }
}

#[cfg(windows)]
fn read_first_video_sample(
    source_reader: &windows::Win32::Media::MediaFoundation::IMFSourceReader,
) -> Result<windows::Win32::Media::MediaFoundation::IMFSample, DecodeError> {
    use windows::Win32::Media::MediaFoundation::MF_SOURCE_READERF_ENDOFSTREAM;

    let mut flags = 0;
    let mut timestamp = 0;
    let mut sample = None;
    unsafe {
        source_reader
            .ReadSample(
                first_video_stream(),
                0,
                None,
                Some(&mut flags),
                Some(&mut timestamp),
                Some(&mut sample),
            )
            .map_err(decode_error_from_windows)?;
    }

    if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
        return Err(DecodeError::new(
            DecodeErrorKind::EndOfStream,
            "Media Foundation reached end of stream before producing a video sample",
        ));
    }

    sample.ok_or_else(|| {
        platform_decode_error("Media Foundation did not produce a sample for the first video frame")
    })
}

#[cfg(windows)]
fn media_type_dimensions(
    media_type: &windows::Win32::Media::MediaFoundation::IMFMediaType,
) -> Result<FrameDimensions, MediaIoError> {
    use windows::Win32::Media::MediaFoundation::MF_MT_FRAME_SIZE;

    let packed = unsafe {
        media_type
            .GetUINT64(&MF_MT_FRAME_SIZE)
            .map_err(media_error_from_windows)?
    };
    Ok(FrameDimensions {
        width: (packed >> 32) as u32,
        height: (packed & 0xffff_ffff) as u32,
    })
}

#[cfg(windows)]
fn media_type_frame_rate(
    media_type: &windows::Win32::Media::MediaFoundation::IMFMediaType,
) -> Option<RationalFrameRate> {
    use windows::Win32::Media::MediaFoundation::MF_MT_FRAME_RATE;

    let packed = unsafe { media_type.GetUINT64(&MF_MT_FRAME_RATE).ok()? };
    let numerator = (packed >> 32) as u32;
    let denominator = (packed & 0xffff_ffff) as u32;
    if numerator == 0 || denominator == 0 {
        None
    } else {
        Some(RationalFrameRate {
            numerator,
            denominator,
        })
    }
}

#[cfg(windows)]
fn media_type_pixel_format(
    media_type: &windows::Win32::Media::MediaFoundation::IMFMediaType,
) -> Result<VideoPixelFormat, MediaIoError> {
    use windows::Win32::Media::MediaFoundation::{
        MF_MT_SUBTYPE, MFVideoFormat_ARGB32, MFVideoFormat_NV12, MFVideoFormat_P010,
        MFVideoFormat_RGB32,
    };

    let subtype = unsafe {
        media_type
            .GetGUID(&MF_MT_SUBTYPE)
            .map_err(media_error_from_windows)?
    };
    Ok(if subtype == MFVideoFormat_NV12 {
        VideoPixelFormat::Nv12
    } else if subtype == MFVideoFormat_P010 {
        VideoPixelFormat::P010
    } else if subtype == MFVideoFormat_RGB32 || subtype == MFVideoFormat_ARGB32 {
        VideoPixelFormat::Bgra8
    } else {
        VideoPixelFormat::Unknown
    })
}

#[cfg(windows)]
fn sample_time_us(sample: &windows::Win32::Media::MediaFoundation::IMFSample) -> Option<u64> {
    let hundred_ns = unsafe { sample.GetSampleTime().ok()? };
    hns_to_microseconds(hundred_ns)
}

#[cfg(windows)]
fn sample_duration_us(sample: &windows::Win32::Media::MediaFoundation::IMFSample) -> Option<u64> {
    let hundred_ns = unsafe { sample.GetSampleDuration().ok()? };
    hns_to_microseconds(hundred_ns)
}

#[cfg(windows)]
fn hns_to_microseconds(value: i64) -> Option<u64> {
    if value < 0 {
        return None;
    }
    Some((value as u64) / 10)
}

#[cfg(windows)]
fn system_d3d_device_id() -> Option<RuntimeDeviceId> {
    Some(RuntimeDeviceId {
        backend: TextureBackend::D3d11Texture2D,
        adapter_id: "d3d-device-unproven".to_owned(),
        device_id: "media-foundation-source-reader".to_owned(),
    })
}

#[cfg(windows)]
fn first_video_stream() -> u32 {
    use windows::Win32::Media::MediaFoundation::MF_SOURCE_READER_FIRST_VIDEO_STREAM;

    MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32
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
        "windows-native-session-{}",
        NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

fn media_error_from_frame_pool(error: FramePoolError) -> MediaIoError {
    MediaIoError::new(
        MediaIoErrorKind::RuntimeUnavailable,
        format!("Windows native frame pool operation failed: {error}"),
    )
}

fn decode_error_from_frame_pool(error: FramePoolError) -> DecodeError {
    DecodeError::new(
        DecodeErrorKind::RuntimeFailure,
        format!("failed to acquire Windows native frame lease: {error}"),
    )
}

#[cfg(windows)]
fn media_error_from_windows(error: windows::core::Error) -> MediaIoError {
    MediaIoError::new(
        MediaIoErrorKind::RuntimeUnavailable,
        format!("PlatformApiFailure: Windows Media Foundation call failed: {error:?}"),
    )
}

#[cfg(windows)]
fn decode_error_from_windows(error: windows::core::Error) -> DecodeError {
    DecodeError::new(
        DecodeErrorKind::RuntimeFailure,
        format!("PlatformApiFailure: Windows Media Foundation decode failed: {error:?}"),
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

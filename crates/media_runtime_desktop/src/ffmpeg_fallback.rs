use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use media_runtime::{
    AudioDecoder, DecodeError, DecodeErrorKind, DecodedVideoFrame, FfmpegExecutor, FrameDimensions,
    FrameLeaseId, FrameLeaseRequest, FramePool, FramePoolError, FramePoolLimits,
    FrameReleaseDiagnostic, FrameStorageRequest, MAX_STDERR_SUMMARY_BYTES, MediaIoError,
    MediaIoErrorKind, MediaIoFallbackReason, MediaOpenRequest, MediaReader, MediaSession,
    MediaSessionId, MediaStreamInfo, MediaStreamKind, RationalFrameRate, RuntimeConfig,
    SelectedDecodePath, StreamId, VideoColorMetadata, VideoDecodeRequest, VideoDecoder,
    VideoPixelFormat,
};
use serde::{Deserialize, Serialize};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

const DEFAULT_MAX_OUTSTANDING_LEASES: usize = 8;
const OUTPUT_PIXEL_FORMAT: VideoPixelFormat = VideoPixelFormat::Rgba8;
const OUTPUT_BYTES_PER_PIXEL: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegCpuFrameDecodeRequest {
    pub material_uri: PathBuf,
    pub stream_id: StreamId,
    pub source_time_us: u64,
    pub playback_generation: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegCpuFrameFingerprintRequest {
    pub material_uri: PathBuf,
    pub source_time_us: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegCpuFrameFingerprint {
    pub digest: String,
    pub width: u32,
    pub height: u32,
    pub byte_count: usize,
    pub source_time_us: u64,
    pub stream_id: StreamId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegCpuFrameFingerprintError {
    message: String,
}

impl FfmpegCpuFrameFingerprintError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for FfmpegCpuFrameFingerprintError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for FfmpegCpuFrameFingerprintError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegDecodeDiagnostic {
    pub selected_path: SelectedDecodePath,
    pub fallback_reason: Option<MediaIoFallbackReason>,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
    pub message: String,
}

#[derive(Debug)]
pub struct FfmpegFallbackMediaReader<E> {
    executor: Rc<E>,
    runtime: RuntimeConfig,
    frame_pool_limits: FramePoolLimits,
}

impl<E> FfmpegFallbackMediaReader<E> {
    pub fn new(executor: E, runtime: RuntimeConfig) -> Self {
        Self {
            executor: Rc::new(executor),
            runtime,
            frame_pool_limits: FramePoolLimits {
                max_outstanding_leases: DEFAULT_MAX_OUTSTANDING_LEASES,
            },
        }
    }

    pub fn with_frame_pool_limits(mut self, limits: FramePoolLimits) -> Self {
        self.frame_pool_limits = limits;
        self
    }
}

impl<E> MediaReader for FfmpegFallbackMediaReader<E>
where
    E: FfmpegExecutor + 'static,
{
    fn reader_name(&self) -> &'static str {
        "ffmpeg-fallback-media-reader"
    }

    fn open(&self, request: MediaOpenRequest) -> Result<Box<dyn MediaSession>, MediaIoError> {
        ensure_input_file(&request.material_uri)?;
        ensure_binary_available(self.executor.as_ref(), &self.runtime.ffmpeg.path, "ffmpeg")?;
        ensure_binary_available(
            self.executor.as_ref(),
            &self.runtime.ffprobe.path,
            "ffprobe",
        )?;

        let streams = probe_streams(self.executor.as_ref(), &self.runtime, &request.material_uri)?;
        validate_requested_streams(&streams, &request.requested_streams)?;

        Ok(Box::new(FfmpegFallbackMediaSession {
            session_id: next_session_id(),
            material_uri: request.material_uri,
            streams,
            executor: Rc::clone(&self.executor),
            runtime: self.runtime.clone(),
            frame_pool_limits: self.frame_pool_limits.clone(),
        }))
    }
}

#[derive(Debug)]
pub struct FfmpegFallbackMediaSession<E> {
    session_id: MediaSessionId,
    material_uri: PathBuf,
    streams: Vec<MediaStreamInfo>,
    executor: Rc<E>,
    runtime: RuntimeConfig,
    frame_pool_limits: FramePoolLimits,
}

impl<E> MediaSession for FfmpegFallbackMediaSession<E>
where
    E: FfmpegExecutor + 'static,
{
    fn session_id(&self) -> MediaSessionId {
        self.session_id.clone()
    }

    fn streams(&self) -> &[MediaStreamInfo] {
        &self.streams
    }

    fn video_decoder(&self, stream_id: StreamId) -> Result<Box<dyn VideoDecoder>, MediaIoError> {
        let stream = stream_by_id(&self.streams, stream_id)?;
        if stream.kind != MediaStreamKind::Video {
            return Err(MediaIoError::new(
                MediaIoErrorKind::UnsupportedStream,
                format!("stream {} is not a video stream", stream_id.0),
            ));
        }
        if stream.dimensions.is_none() {
            return Err(MediaIoError::new(
                MediaIoErrorKind::UnsupportedStream,
                format!("stream {} does not report video dimensions", stream_id.0),
            ));
        }

        Ok(Box::new(FfmpegCpuVideoDecoder::new(
            self.session_id.clone(),
            self.material_uri.clone(),
            stream.clone(),
            Rc::clone(&self.executor),
            self.runtime.clone(),
            self.frame_pool_limits.clone(),
        )))
    }

    fn audio_decoder(&self, stream_id: StreamId) -> Result<Box<dyn AudioDecoder>, MediaIoError> {
        let stream = stream_by_id(&self.streams, stream_id)?;
        Err(MediaIoError::new(
            MediaIoErrorKind::UnsupportedStream,
            format!(
                "FFmpeg CPU fallback audio decoder is not implemented for stream {} ({})",
                stream.stream_id.0, stream.codec
            ),
        ))
    }
}

#[derive(Debug)]
pub struct FfmpegCpuVideoDecoder<E> {
    material_uri: PathBuf,
    stream: MediaStreamInfo,
    executor: Rc<E>,
    runtime: RuntimeConfig,
    frame_pool: FramePool,
    last_diagnostic: Option<FfmpegDecodeDiagnostic>,
}

impl<E> FfmpegCpuVideoDecoder<E>
where
    E: FfmpegExecutor,
{
    fn new(
        session_id: MediaSessionId,
        material_uri: PathBuf,
        stream: MediaStreamInfo,
        executor: Rc<E>,
        runtime: RuntimeConfig,
        frame_pool_limits: FramePoolLimits,
    ) -> Self {
        Self {
            frame_pool: FramePool::new(session_id.clone(), frame_pool_limits),
            material_uri,
            stream,
            executor,
            runtime,
            last_diagnostic: None,
        }
    }

    pub fn last_diagnostic(&self) -> Option<&FfmpegDecodeDiagnostic> {
        self.last_diagnostic.as_ref()
    }

    fn decode_cpu_frame(
        &mut self,
        request: VideoDecodeRequest,
    ) -> Result<DecodedVideoFrame, DecodeError> {
        if !self.executor.can_execute(&self.runtime.ffmpeg.path) {
            let diagnostic = FfmpegDecodeDiagnostic {
                selected_path: SelectedDecodePath::FfmpegCpuFrame,
                fallback_reason: Some(MediaIoFallbackReason::FfmpegUnavailable),
                stdout_summary: None,
                stderr_summary: None,
                message: format!(
                    "FfmpegUnavailable: {} cannot execute ffmpeg at {}",
                    self.executor.executor_name(),
                    self.runtime.ffmpeg.path.display()
                ),
            };
            self.last_diagnostic = Some(diagnostic.clone());
            return Err(decode_error_from_diagnostic(diagnostic));
        }

        let dimensions = self.stream.dimensions.ok_or_else(|| {
            DecodeError::new(
                DecodeErrorKind::InvalidRequest,
                format!(
                    "stream {} does not report video dimensions",
                    self.stream.stream_id.0
                ),
            )
        })?;
        let expected_byte_len = expected_rgba_byte_len(dimensions)?;
        let args = self.decode_args(&request);
        let output = self
            .executor
            .run(&self.runtime.ffmpeg.path, &args)
            .map_err(|error| self.process_launch_error(error))?;

        if !output.status.success() {
            let diagnostic = FfmpegDecodeDiagnostic {
                selected_path: SelectedDecodePath::FfmpegCpuFrame,
                fallback_reason: Some(MediaIoFallbackReason::PlatformApiFailure),
                stdout_summary: optional_summary(&output.stdout),
                stderr_summary: optional_summary(&output.stderr),
                message: format!(
                    "ffmpeg CPU frame decode failed for {} at {} us",
                    self.material_uri.display(),
                    request.source_time_us
                ),
            };
            self.last_diagnostic = Some(diagnostic.clone());
            return Err(decode_error_from_diagnostic(diagnostic));
        }

        if output.stdout.len() < expected_byte_len {
            let diagnostic = FfmpegDecodeDiagnostic {
                selected_path: SelectedDecodePath::FfmpegCpuFrame,
                fallback_reason: Some(MediaIoFallbackReason::PlatformApiFailure),
                stdout_summary: optional_summary(&output.stdout),
                stderr_summary: optional_summary(&output.stderr),
                message: format!(
                    "ffmpeg CPU frame decode returned {} bytes, expected at least {}",
                    output.stdout.len(),
                    expected_byte_len
                ),
            };
            self.last_diagnostic = Some(diagnostic.clone());
            return Err(decode_error_from_diagnostic(diagnostic));
        }

        self.frame_pool
            .acquire_video_frame(FrameLeaseRequest {
                playback_generation: request.playback_generation,
                source_time_us: request.source_time_us,
                duration_us: frame_duration_us(self.stream.frame_rate),
                frame_index: frame_index_at(request.source_time_us, self.stream.frame_rate),
                dimensions,
                pixel_format: OUTPUT_PIXEL_FORMAT,
                color: self.stream.color.clone().unwrap_or_else(|| {
                    VideoColorMetadata::unknown_with_diagnostic(
                        "ffmpeg fallback did not report source color metadata",
                    )
                }),
                storage: FrameStorageRequest::Cpu {
                    estimated_byte_len: expected_byte_len,
                },
            })
            .map_err(decode_error_from_frame_pool)
    }

    fn decode_args(&self, request: &VideoDecodeRequest) -> Vec<OsString> {
        raw_rgba_decode_args(
            &self.material_uri,
            self.stream.stream_id,
            request.source_time_us,
        )
    }

    fn process_launch_error(&mut self, error: io::Error) -> DecodeError {
        let fallback_reason = if error.kind() == io::ErrorKind::NotFound {
            MediaIoFallbackReason::FfmpegUnavailable
        } else {
            MediaIoFallbackReason::PlatformApiFailure
        };
        let diagnostic = FfmpegDecodeDiagnostic {
            selected_path: SelectedDecodePath::FfmpegCpuFrame,
            fallback_reason: Some(fallback_reason),
            stdout_summary: None,
            stderr_summary: optional_summary(error.to_string().as_bytes()),
            message: format!("failed to launch ffmpeg CPU frame decode: {error}"),
        };
        self.last_diagnostic = Some(diagnostic.clone());
        decode_error_from_diagnostic(diagnostic)
    }
}

impl<E> VideoDecoder for FfmpegCpuVideoDecoder<E>
where
    E: FfmpegExecutor + 'static,
{
    fn decoder_name(&self) -> &'static str {
        "ffmpeg-cpu-video-decoder"
    }

    fn decode_at(&mut self, request: VideoDecodeRequest) -> Result<DecodedVideoFrame, DecodeError> {
        self.decode_cpu_frame(request)
    }

    fn release_frame(
        &mut self,
        lease_id: FrameLeaseId,
    ) -> Result<FrameReleaseDiagnostic, DecodeError> {
        self.frame_pool
            .release(lease_id)
            .map_err(decode_error_from_frame_pool)
    }

    fn flush(&mut self) -> Result<(), DecodeError> {
        Ok(())
    }
}

pub fn decode_ffmpeg_cpu_frame_fingerprint(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    request: &FfmpegCpuFrameFingerprintRequest,
) -> Result<FfmpegCpuFrameFingerprint, FfmpegCpuFrameFingerprintError> {
    ensure_input_file(&request.material_uri)
        .map_err(|error| FfmpegCpuFrameFingerprintError::new(error.to_string()))?;
    ensure_binary_available(executor, &runtime.ffmpeg.path, "ffmpeg")
        .map_err(|error| FfmpegCpuFrameFingerprintError::new(error.to_string()))?;
    ensure_binary_available(executor, &runtime.ffprobe.path, "ffprobe")
        .map_err(|error| FfmpegCpuFrameFingerprintError::new(error.to_string()))?;

    let stream = probe_streams(executor, runtime, &request.material_uri)
        .map_err(|error| FfmpegCpuFrameFingerprintError::new(error.to_string()))?
        .into_iter()
        .find(|stream| stream.kind == MediaStreamKind::Video && stream.dimensions.is_some())
        .ok_or_else(|| {
            FfmpegCpuFrameFingerprintError::new(format!(
                "ffprobe reported no decodable video stream for {}",
                request.material_uri.display()
            ))
        })?;
    let dimensions = stream.dimensions.ok_or_else(|| {
        FfmpegCpuFrameFingerprintError::new(format!(
            "video stream {} did not report dimensions",
            stream.stream_id.0
        ))
    })?;
    let expected_byte_len = expected_rgba_byte_len(dimensions)
        .map_err(|error| FfmpegCpuFrameFingerprintError::new(error.to_string()))?;
    let args = raw_rgba_decode_args(
        &request.material_uri,
        stream.stream_id,
        request.source_time_us,
    );
    let output = executor
        .run(&runtime.ffmpeg.path, &args)
        .map_err(|error| FfmpegCpuFrameFingerprintError::new(error.to_string()))?;

    if !output.status.success() {
        return Err(FfmpegCpuFrameFingerprintError::new(format!(
            "ffmpeg frame fingerprint decode failed for {} at {} us: stdout={} stderr={}",
            request.material_uri.display(),
            request.source_time_us,
            summary_or_empty(&output.stdout),
            summary_or_empty(&output.stderr)
        )));
    }
    if output.stdout.len() < expected_byte_len {
        return Err(FfmpegCpuFrameFingerprintError::new(format!(
            "ffmpeg frame fingerprint decode returned {} bytes, expected at least {}",
            output.stdout.len(),
            expected_byte_len
        )));
    }

    let pixels = &output.stdout[..expected_byte_len];
    Ok(FfmpegCpuFrameFingerprint {
        digest: format!("blake3:v1:{}", blake3::hash(pixels).to_hex()),
        width: dimensions.width,
        height: dimensions.height,
        byte_count: expected_byte_len,
        source_time_us: request.source_time_us,
        stream_id: stream.stream_id,
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

fn ensure_binary_available(
    executor: &impl FfmpegExecutor,
    binary: &Path,
    label: &str,
) -> Result<(), MediaIoError> {
    if executor.can_execute(binary) {
        Ok(())
    } else {
        Err(MediaIoError::new(
            MediaIoErrorKind::RuntimeUnavailable,
            format!(
                "FfmpegUnavailable: {} cannot execute {label} at {}",
                executor.executor_name(),
                binary.display()
            ),
        ))
    }
}

fn probe_streams(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    path: &Path,
) -> Result<Vec<MediaStreamInfo>, MediaIoError> {
    let args = vec![
        OsString::from("-v"),
        OsString::from("error"),
        OsString::from("-print_format"),
        OsString::from("json"),
        OsString::from("-show_entries"),
        OsString::from(
            "stream=index,codec_type,codec_name,width,height,pix_fmt,r_frame_rate,avg_frame_rate,duration,sample_rate,channels:format=duration",
        ),
        path.as_os_str().to_owned(),
    ];
    let output = executor
        .run(&runtime.ffprobe.path, &args)
        .map_err(|error| {
            MediaIoError::new(
                MediaIoErrorKind::RuntimeUnavailable,
                format!("failed to run ffprobe for FFmpeg fallback session: {error}"),
            )
        })?;

    if !output.status.success() {
        return Err(MediaIoError::new(
            MediaIoErrorKind::OpenFailed,
            format!(
                "ffprobe failed for FFmpeg fallback session: stdout={} stderr={}",
                summary_or_empty(&output.stdout),
                summary_or_empty(&output.stderr)
            ),
        ));
    }

    let parsed = serde_json::from_slice::<FfprobeOutput>(&output.stdout).map_err(|error| {
        MediaIoError::new(
            MediaIoErrorKind::OpenFailed,
            format!("ffprobe returned malformed JSON for FFmpeg fallback session: {error}"),
        )
    })?;
    let streams = normalize_probe_streams(parsed)?;
    if streams.is_empty() {
        return Err(MediaIoError::new(
            MediaIoErrorKind::OpenFailed,
            "ffprobe reported no supported audio or video streams for FFmpeg fallback session",
        ));
    }
    Ok(streams)
}

fn normalize_probe_streams(parsed: FfprobeOutput) -> Result<Vec<MediaStreamInfo>, MediaIoError> {
    let format_duration_us = parsed
        .format
        .as_ref()
        .and_then(|format| format.duration.as_deref())
        .map(parse_decimal_seconds_to_microseconds)
        .transpose()
        .map_err(|message| MediaIoError::new(MediaIoErrorKind::OpenFailed, message))?;

    parsed
        .streams
        .into_iter()
        .enumerate()
        .filter_map(|(index, stream)| {
            let kind = match stream.codec_type.as_deref() {
                Some("video") => MediaStreamKind::Video,
                Some("audio") => MediaStreamKind::Audio,
                _ => return None,
            };
            Some(normalize_stream(index, stream, kind, format_duration_us))
        })
        .collect()
}

fn normalize_stream(
    index: usize,
    stream: FfprobeStream,
    kind: MediaStreamKind,
    format_duration_us: Option<u64>,
) -> Result<MediaStreamInfo, MediaIoError> {
    let stream_id = StreamId(stream.index.unwrap_or(index as u32));
    let duration_us = stream
        .duration
        .as_deref()
        .map(parse_decimal_seconds_to_microseconds)
        .transpose()
        .map_err(|message| MediaIoError::new(MediaIoErrorKind::OpenFailed, message))?
        .or(format_duration_us);
    let frame_rate = preferred_rate(stream.r_frame_rate.as_deref())
        .or_else(|| preferred_rate(stream.avg_frame_rate.as_deref()))
        .map(parse_rational_frame_rate)
        .transpose()
        .map_err(|message| MediaIoError::new(MediaIoErrorKind::OpenFailed, message))?;
    let dimensions = match (stream.width, stream.height) {
        (Some(width), Some(height)) if kind == MediaStreamKind::Video => {
            Some(FrameDimensions { width, height })
        }
        _ => None,
    };
    let pixel_format = stream.pix_fmt.as_deref().and_then(video_pixel_format);
    let color = (kind == MediaStreamKind::Video).then(|| {
        VideoColorMetadata::unknown_with_diagnostic(
            "ffmpeg fallback probe did not surface complete color metadata",
        )
    });
    let sample_rate = stream
        .sample_rate
        .as_deref()
        .and_then(|value| value.parse::<u32>().ok());
    let channels = stream.channels.and_then(|value| u16::try_from(value).ok());

    Ok(MediaStreamInfo {
        stream_id,
        kind,
        codec: stream.codec_name.unwrap_or_else(|| "unknown".to_owned()),
        duration_us,
        frame_rate,
        dimensions,
        pixel_format,
        color,
        sample_rate,
        channels,
    })
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

fn raw_rgba_decode_args(
    material_uri: &Path,
    stream_id: StreamId,
    source_time_us: u64,
) -> Vec<OsString> {
    os_args(&[
        "-hide_banner",
        "-v",
        "error",
        "-nostdin",
        "-ss",
        &format_microseconds(source_time_us),
        "-i",
    ])
    .into_iter()
    .chain([material_uri.as_os_str().to_owned()])
    .chain(os_args(&[
        "-map",
        &format!("0:{}", stream_id.0),
        "-frames:v",
        "1",
        "-an",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgba",
        "-",
    ]))
    .collect()
}

fn expected_rgba_byte_len(dimensions: FrameDimensions) -> Result<usize, DecodeError> {
    let width = usize::try_from(dimensions.width).map_err(|_| {
        DecodeError::new(DecodeErrorKind::InvalidRequest, "frame width is too large")
    })?;
    let height = usize::try_from(dimensions.height).map_err(|_| {
        DecodeError::new(DecodeErrorKind::InvalidRequest, "frame height is too large")
    })?;
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(OUTPUT_BYTES_PER_PIXEL))
        .ok_or_else(|| DecodeError::new(DecodeErrorKind::InvalidRequest, "frame size overflow"))
}

fn frame_duration_us(frame_rate: Option<RationalFrameRate>) -> Option<u64> {
    let frame_rate = frame_rate?;
    if frame_rate.numerator == 0 {
        return None;
    }
    Some(
        1_000_000_u64.saturating_mul(u64::from(frame_rate.denominator))
            / u64::from(frame_rate.numerator),
    )
}

fn frame_index_at(source_time_us: u64, frame_rate: Option<RationalFrameRate>) -> Option<u64> {
    let frame_rate = frame_rate?;
    if frame_rate.denominator == 0 {
        return None;
    }
    Some(
        source_time_us.saturating_mul(u64::from(frame_rate.numerator))
            / 1_000_000_u64.saturating_mul(u64::from(frame_rate.denominator)),
    )
}

fn decode_error_from_frame_pool(error: FramePoolError) -> DecodeError {
    DecodeError::new(
        DecodeErrorKind::RuntimeFailure,
        format!("failed to acquire FFmpeg CPU frame lease: {error}"),
    )
}

fn decode_error_from_diagnostic(diagnostic: FfmpegDecodeDiagnostic) -> DecodeError {
    let stdout = diagnostic.stdout_summary.unwrap_or_default();
    let stderr = diagnostic.stderr_summary.unwrap_or_default();
    DecodeError::new(
        DecodeErrorKind::RuntimeFailure,
        format!(
            "{}; selected_path={:?}; fallback_reason={:?}; stdout={}; stderr={}",
            diagnostic.message,
            diagnostic.selected_path,
            diagnostic.fallback_reason,
            stdout,
            stderr
        ),
    )
}

fn next_session_id() -> MediaSessionId {
    MediaSessionId(format!(
        "ffmpeg-fallback-session-{}",
        NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

fn os_args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn format_microseconds(value: u64) -> String {
    let seconds = value / 1_000_000;
    let micros = value % 1_000_000;
    format!("{seconds}.{micros:06}")
}

fn video_pixel_format(value: &str) -> Option<VideoPixelFormat> {
    match value {
        "nv12" => Some(VideoPixelFormat::Nv12),
        "bgra" | "bgra8" => Some(VideoPixelFormat::Bgra8),
        "rgba" | "rgba8" => Some(VideoPixelFormat::Rgba8),
        "p010le" | "p010" => Some(VideoPixelFormat::P010),
        "yuv420p" => Some(VideoPixelFormat::Yuv420P),
        _ => Some(VideoPixelFormat::Unknown),
    }
}

fn preferred_rate(value: Option<&str>) -> Option<&str> {
    value.filter(|rate| !rate.is_empty() && *rate != "0/0")
}

fn parse_rational_frame_rate(value: &str) -> Result<RationalFrameRate, String> {
    let (numerator, denominator) = value
        .split_once('/')
        .ok_or_else(|| format!("invalid frame rate `{value}`"))?;
    let numerator = numerator
        .parse::<u32>()
        .map_err(|_| format!("invalid frame rate numerator `{value}`"))?;
    let denominator = denominator
        .parse::<u32>()
        .map_err(|_| format!("invalid frame rate denominator `{value}`"))?;

    if numerator == 0 {
        return Err("frame rate numerator cannot be zero".to_owned());
    }
    if denominator == 0 {
        return Err("frame rate denominator cannot be zero".to_owned());
    }

    Ok(RationalFrameRate {
        numerator,
        denominator,
    })
}

fn parse_decimal_seconds_to_microseconds(value: &str) -> Result<u64, String> {
    let (whole, fractional) = value
        .split_once('.')
        .map_or((value, ""), |(whole, fractional)| (whole, fractional));
    let whole = whole
        .parse::<u64>()
        .map_err(|_| format!("invalid duration seconds `{value}`"))?;
    let mut micros = String::from(fractional);
    if micros.len() > 6 {
        micros.truncate(6);
    }
    while micros.len() < 6 {
        micros.push('0');
    }
    let micros = if micros.is_empty() {
        0
    } else {
        micros
            .parse::<u64>()
            .map_err(|_| format!("invalid duration fraction `{value}`"))?
    };

    Ok(whole.saturating_mul(1_000_000).saturating_add(micros))
}

fn optional_summary(bytes: &[u8]) -> Option<String> {
    let summary = bounded_summary(bytes);
    if summary.is_empty() {
        None
    } else {
        Some(summary)
    }
}

fn summary_or_empty(bytes: &[u8]) -> String {
    optional_summary(bytes).unwrap_or_default()
}

fn bounded_summary(bytes: &[u8]) -> String {
    let value = String::from_utf8_lossy(bytes);
    let trimmed = value.trim();
    let mut summary = String::new();

    for character in trimmed.chars() {
        if summary.len() + character.len_utf8() > MAX_STDERR_SUMMARY_BYTES {
            break;
        }
        summary.push(character);
    }

    summary
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    index: Option<u32>,
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    pix_fmt: Option<String>,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    duration: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
}

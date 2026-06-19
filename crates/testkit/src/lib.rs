//! Test harness shell for fixtures, goldens, and render smoke checks.
//!
//! Later Phase 1 plans add deterministic schema fixtures, tiny media generation,
//! and FFmpeg render smoke helpers here. This shell exists so downstream plans
//! can depend on a stable testkit crate without introducing media behavior early.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use media_runtime::{
    FfmpegExecutor, MAX_STDERR_SUMMARY_BYTES, MaterialProbeKind, MaterialProbeMetadata,
    RationalFrameRate, RuntimeConfig, discover_runtime_config,
};
use media_runtime_desktop::DesktopFfmpegExecutor;

pub mod audio_parity;
pub mod large_timeline;
pub mod render_compare;

pub use audio_parity::{
    AudioMixParityDifference, AudioMixParityStatus, AudioPreviewExportParityDiagnostic,
    AudioSampleSummary, audio_preview_export_parity_diagnostic,
};

/// Boundary marker for Phase 1 test harness helpers.
pub const TESTKIT_BOUNDARY: &str = "fixtures-goldens-render-smoke-shell";

const TINY_WIDTH: u32 = 160;
const TINY_HEIGHT: u32 = 90;
const TINY_FPS: u32 = 10;
const TINY_DURATION_SECONDS: &str = "1";
const TINY_DURATION_MIN_MICROS: u64 = 900_000;
const TINY_DURATION_MAX_MICROS: u64 = 1_200_000;

const MATERIAL_VIDEO_WIDTH: u32 = 160;
const MATERIAL_VIDEO_HEIGHT: u32 = 90;
const MATERIAL_VIDEO_FPS: u32 = 10;
const MATERIAL_IMAGE_WIDTH: u32 = 80;
const MATERIAL_IMAGE_HEIGHT: u32 = 60;
const MATERIAL_AUDIO_SAMPLE_RATE: u32 = 44_100;
const MATERIAL_AUDIO_CHANNELS: u16 = 1;
const MATERIAL_DURATION_MICROS: u64 = 1_000_000;

/// Result type for deterministic smoke helpers.
pub type SmokeResult<T> = Result<T, SmokeError>;

/// Error raised by deterministic smoke helpers.
#[derive(Debug)]
pub struct SmokeError {
    message: String,
}

impl SmokeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SmokeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SmokeError {}

impl From<media_runtime::DiscoveryError> for SmokeError {
    fn from(error: media_runtime::DiscoveryError) -> Self {
        Self::new(format!("{error}: {}", error.remediation))
    }
}

impl From<std::io::Error> for SmokeError {
    fn from(error: std::io::Error) -> Self {
        Self::new(error.to_string())
    }
}

/// Temporary media generated from FFmpeg lavfi sources.
#[derive(Debug)]
pub struct TinyLavfiMedia {
    _temp_dir: tempfile::TempDir,
    output_path: PathBuf,
}

impl TinyLavfiMedia {
    /// Path to the generated MP4 output. The file is removed when this value is dropped.
    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
}

/// ffprobe metadata used by the Phase 1 render smoke harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmokeMetadata {
    pub duration_microseconds: u64,
    pub width: u32,
    pub height: u32,
    pub frame_rate_numerator: u32,
    pub frame_rate_denominator: u32,
    pub has_video_stream: bool,
    pub has_audio_stream: bool,
}

/// Tiny render smoke output and parsed metadata.
#[derive(Debug)]
pub struct TinyRenderSmoke {
    media: TinyLavfiMedia,
    metadata: SmokeMetadata,
}

impl TinyRenderSmoke {
    /// Path to the generated MP4 output. The file is removed when this value is dropped.
    pub fn output_path(&self) -> &Path {
        self.media.output_path()
    }

    /// Parsed ffprobe metadata for the generated output.
    pub fn metadata(&self) -> &SmokeMetadata {
        &self.metadata
    }
}

/// Temporary media fixture for material import/probe tests.
#[derive(Debug)]
pub struct GeneratedMaterialFixture {
    _temp_dir: tempfile::TempDir,
    path: PathBuf,
    expected: ExpectedMaterialMetadata,
}

impl GeneratedMaterialFixture {
    /// Path to the generated media file. The file is removed when this value is dropped.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Expected metadata contract for this generated fixture.
    pub fn expected(&self) -> &ExpectedMaterialMetadata {
        &self.expected
    }

    /// Assert that normalized `media_runtime` probe output matches this fixture.
    pub fn assert_probe_metadata(&self, metadata: &MaterialProbeMetadata) -> SmokeResult<()> {
        self.expected.assert_probe_metadata(metadata)
    }
}

/// Deterministic generated H.264 material fixture for realtime preview tests.
#[derive(Debug)]
pub struct H264PreviewFixture {
    inner: GeneratedMaterialFixture,
}

impl H264PreviewFixture {
    /// Path to the generated MP4 fixture. The file is removed when this value is dropped.
    pub fn path(&self) -> &Path {
        self.inner.path()
    }

    /// Codec name expected by realtime preview fixture tests.
    pub fn expected_codec(&self) -> &'static str {
        "h264"
    }

    /// Expected normalized probe metadata for the generated material.
    pub fn expected(&self) -> &ExpectedMaterialMetadata {
        self.inner.expected()
    }
}

/// Expected normalized probe metadata for a generated material fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedMaterialMetadata {
    pub kind: MaterialProbeKind,
    pub duration_microseconds: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<RationalFrameRate>,
    pub has_video_stream: bool,
    pub has_audio_stream: bool,
    pub audio_sample_rate: Option<u32>,
    pub audio_channels: Option<u16>,
}

impl ExpectedMaterialMetadata {
    /// Assert normalized material probe metadata against this fixture contract.
    pub fn assert_probe_metadata(&self, metadata: &MaterialProbeMetadata) -> SmokeResult<()> {
        if metadata.kind != self.kind {
            return Err(SmokeError::new(format!(
                "expected material kind {:?}, got {:?}",
                self.kind, metadata.kind
            )));
        }

        if metadata.duration_microseconds != self.duration_microseconds {
            return Err(SmokeError::new(format!(
                "expected duration {:?}, got {:?}",
                self.duration_microseconds, metadata.duration_microseconds
            )));
        }

        if metadata.width != self.width || metadata.height != self.height {
            return Err(SmokeError::new(format!(
                "expected dimensions {:?}x{:?}, got {:?}x{:?}",
                self.width, self.height, metadata.width, metadata.height
            )));
        }

        if metadata.frame_rate != self.frame_rate {
            return Err(SmokeError::new(format!(
                "expected frame rate {:?}, got {:?}",
                self.frame_rate, metadata.frame_rate
            )));
        }

        if metadata.has_video_stream != self.has_video_stream
            || metadata.has_audio_stream != self.has_audio_stream
        {
            return Err(SmokeError::new(format!(
                "expected stream flags video={} audio={}, got video={} audio={}",
                self.has_video_stream,
                self.has_audio_stream,
                metadata.has_video_stream,
                metadata.has_audio_stream
            )));
        }

        let actual_sample_rate = metadata.audio.map(|audio| audio.sample_rate);
        let actual_channels = metadata.audio.map(|audio| audio.channels);
        if actual_sample_rate != self.audio_sample_rate || actual_channels != self.audio_channels {
            return Err(SmokeError::new(format!(
                "expected audio {:?}Hz {:?}ch, got {:?}Hz {:?}ch",
                self.audio_sample_rate, self.audio_channels, actual_sample_rate, actual_channels
            )));
        }

        Ok(())
    }
}

/// Generate a tiny deterministic MP4 using FFmpeg lavfi sources.
pub fn generate_tiny_lavfi_media() -> SmokeResult<TinyLavfiMedia> {
    let runtime = discover_runtime_config()?;
    let executor = DesktopFfmpegExecutor::default();
    let temp_dir = tempfile::Builder::new()
        .prefix("media-generated-")
        .tempdir()?;
    let media_dir = temp_dir.path().join("media-generated");
    std::fs::create_dir_all(&media_dir)?;
    let output_path = media_dir.join("tiny-render-smoke.mp4");

    run_ffmpeg_generate(&executor, &runtime, &output_path)?;

    if !output_path.is_file() {
        return Err(SmokeError::new(format!(
            "ffmpeg completed but did not create {}",
            output_path.display()
        )));
    }

    Ok(TinyLavfiMedia {
        _temp_dir: temp_dir,
        output_path,
    })
}

/// Generate tiny media and assert it through ffprobe metadata.
pub fn run_tiny_render_smoke() -> SmokeResult<TinyRenderSmoke> {
    let media = generate_tiny_lavfi_media()?;
    let metadata = probe_media_metadata(media.output_path())?;
    assert_tiny_smoke_metadata(&metadata)?;

    Ok(TinyRenderSmoke { media, metadata })
}

/// Generate deterministic video material for probe/import tests.
pub fn generate_video_material_fixture(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
) -> SmokeResult<GeneratedMaterialFixture> {
    let fixture = GeneratedMaterialFixture::new(
        "video",
        "mp4",
        ExpectedMaterialMetadata {
            kind: MaterialProbeKind::Video,
            duration_microseconds: Some(MATERIAL_DURATION_MICROS),
            width: Some(MATERIAL_VIDEO_WIDTH),
            height: Some(MATERIAL_VIDEO_HEIGHT),
            frame_rate: Some(RationalFrameRate {
                numerator: MATERIAL_VIDEO_FPS,
                denominator: 1,
            }),
            has_video_stream: true,
            has_audio_stream: true,
            audio_sample_rate: Some(MATERIAL_AUDIO_SAMPLE_RATE),
            audio_channels: Some(MATERIAL_AUDIO_CHANNELS),
        },
    )?;
    fixture.run_ffmpeg(
        executor,
        runtime,
        &[
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=160x90:rate=10:duration=1",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100:duration=1",
            "-shortest",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-ac",
            "1",
        ],
    )
}

/// Generate a deterministic H.264 MP4 fixture for realtime preview cache tests.
pub fn generate_h264_preview_fixture() -> SmokeResult<H264PreviewFixture> {
    let runtime = discover_runtime_config()?;
    let executor = DesktopFfmpegExecutor::default();
    let fixture = generate_video_material_fixture(&executor, &runtime)?;
    Ok(H264PreviewFixture { inner: fixture })
}

/// Generate deterministic still image material for probe/import tests.
pub fn generate_image_material_fixture(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
) -> SmokeResult<GeneratedMaterialFixture> {
    let fixture = GeneratedMaterialFixture::new(
        "image",
        "png",
        ExpectedMaterialMetadata {
            kind: MaterialProbeKind::Image,
            duration_microseconds: None,
            width: Some(MATERIAL_IMAGE_WIDTH),
            height: Some(MATERIAL_IMAGE_HEIGHT),
            frame_rate: Some(RationalFrameRate {
                numerator: 25,
                denominator: 1,
            }),
            has_video_stream: true,
            has_audio_stream: false,
            audio_sample_rate: None,
            audio_channels: None,
        },
    )?;
    fixture.run_ffmpeg(
        executor,
        runtime,
        &[
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:size=80x60",
            "-frames:v",
            "1",
        ],
    )
}

/// Generate deterministic audio-only material for probe/import tests.
pub fn generate_audio_material_fixture(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
) -> SmokeResult<GeneratedMaterialFixture> {
    let fixture = GeneratedMaterialFixture::new(
        "audio",
        "wav",
        ExpectedMaterialMetadata {
            kind: MaterialProbeKind::Audio,
            duration_microseconds: Some(MATERIAL_DURATION_MICROS),
            width: None,
            height: None,
            frame_rate: None,
            has_video_stream: false,
            has_audio_stream: true,
            audio_sample_rate: Some(MATERIAL_AUDIO_SAMPLE_RATE),
            audio_channels: Some(MATERIAL_AUDIO_CHANNELS),
        },
    )?;
    fixture.run_ffmpeg(
        executor,
        runtime,
        &[
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:sample_rate=44100:duration=1",
            "-ac",
            "1",
        ],
    )
}

/// Generate all supported deterministic material fixture kinds.
pub fn generate_material_fixtures(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
) -> SmokeResult<Vec<GeneratedMaterialFixture>> {
    Ok(vec![
        generate_video_material_fixture(executor, runtime)?,
        generate_image_material_fixture(executor, runtime)?,
        generate_audio_material_fixture(executor, runtime)?,
    ])
}

/// Probe an existing media file with ffprobe and return metadata needed by the smoke gate.
pub fn probe_media_metadata(path: impl AsRef<Path>) -> SmokeResult<SmokeMetadata> {
    let path = path.as_ref();
    if !path.is_file() {
        return Err(SmokeError::new(format!(
            "cannot probe missing media file {}",
            path.display()
        )));
    }

    let runtime = discover_runtime_config()?;
    let executor = DesktopFfmpegExecutor::default();
    let args = vec![
        OsString::from("-v"),
        OsString::from("error"),
        OsString::from("-output_format"),
        OsString::from("json"),
        OsString::from("-show_entries"),
        OsString::from("stream=codec_type,width,height,r_frame_rate,duration:format=duration"),
        path.as_os_str().to_owned(),
    ];
    let output = executor
        .run(&runtime.ffprobe.path, &args)
        .map_err(|error| {
            SmokeError::new(format!(
                "failed to launch ffprobe at {}: {error}",
                runtime.ffprobe.path.display()
            ))
        })?;

    if !output.status.success() {
        return Err(SmokeError::new(format!(
            "ffprobe metadata probe failed: stdout=`{}` stderr=`{}`",
            bounded_summary(&output.stdout),
            bounded_summary(&output.stderr)
        )));
    }

    parse_ffprobe_metadata(&output.stdout)
}

/// Assert that smoke metadata matches the Phase 1 tiny lavfi contract.
pub fn assert_tiny_smoke_metadata(metadata: &SmokeMetadata) -> SmokeResult<()> {
    if !metadata.has_video_stream {
        return Err(SmokeError::new("expected a video stream"));
    }

    if !metadata.has_audio_stream {
        return Err(SmokeError::new("expected an audio stream"));
    }

    if metadata.width != TINY_WIDTH || metadata.height != TINY_HEIGHT {
        return Err(SmokeError::new(format!(
            "expected {TINY_WIDTH}x{TINY_HEIGHT}, got {}x{}",
            metadata.width, metadata.height
        )));
    }

    if metadata.frame_rate_denominator == 0
        || metadata.frame_rate_numerator != TINY_FPS * metadata.frame_rate_denominator
    {
        return Err(SmokeError::new(format!(
            "expected {TINY_FPS} fps, got {}/{}",
            metadata.frame_rate_numerator, metadata.frame_rate_denominator
        )));
    }

    if !(TINY_DURATION_MIN_MICROS..=TINY_DURATION_MAX_MICROS)
        .contains(&metadata.duration_microseconds)
    {
        return Err(SmokeError::new(format!(
            "expected about one second, got {} microseconds",
            metadata.duration_microseconds
        )));
    }

    Ok(())
}

fn run_ffmpeg_generate(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
) -> SmokeResult<()> {
    let args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-y"),
        OsString::from("-f"),
        OsString::from("lavfi"),
        OsString::from("-i"),
        OsString::from(format!(
            "testsrc2=size={TINY_WIDTH}x{TINY_HEIGHT}:rate={TINY_FPS}:duration={TINY_DURATION_SECONDS}"
        )),
        OsString::from("-f"),
        OsString::from("lavfi"),
        OsString::from("-i"),
        OsString::from(format!(
            "sine=frequency=440:duration={TINY_DURATION_SECONDS}"
        )),
        OsString::from("-shortest"),
        OsString::from("-c:v"),
        OsString::from("libx264"),
        OsString::from("-pix_fmt"),
        OsString::from("yuv420p"),
        OsString::from("-c:a"),
        OsString::from("aac"),
        output_path.as_os_str().to_owned(),
    ];

    let output = executor.run(&runtime.ffmpeg.path, &args).map_err(|error| {
        SmokeError::new(format!(
            "failed to launch ffmpeg at {}: {error}",
            runtime.ffmpeg.path.display()
        ))
    })?;

    if !output.status.success() {
        return Err(SmokeError::new(format!(
            "ffmpeg lavfi generation failed: stdout=`{}` stderr=`{}`",
            bounded_summary(&output.stdout),
            bounded_summary(&output.stderr)
        )));
    }

    Ok(())
}

impl GeneratedMaterialFixture {
    fn new(kind: &str, extension: &str, expected: ExpectedMaterialMetadata) -> SmokeResult<Self> {
        let temp_dir = tempfile::Builder::new()
            .prefix("material-fixture-")
            .tempdir()?;
        let media_dir = temp_dir.path().join("media-generated");
        std::fs::create_dir_all(&media_dir)?;
        let path = media_dir.join(format!("{kind}.{extension}"));

        Ok(Self {
            _temp_dir: temp_dir,
            path,
            expected,
        })
    }

    fn run_ffmpeg(
        self,
        executor: &impl FfmpegExecutor,
        runtime: &RuntimeConfig,
        args: &[&str],
    ) -> SmokeResult<Self> {
        let mut ffmpeg_args = args
            .iter()
            .map(|argument| OsString::from(*argument))
            .collect::<Vec<_>>();
        ffmpeg_args.push(self.path.as_os_str().to_owned());
        let output = executor
            .run(&runtime.ffmpeg.path, &ffmpeg_args)
            .map_err(|error| {
                SmokeError::new(format!(
                    "failed to launch ffmpeg at {}: {error}",
                    runtime.ffmpeg.path.display()
                ))
            })?;

        if !output.status.success() {
            return Err(SmokeError::new(format!(
                "ffmpeg material fixture generation failed: stdout=`{}` stderr=`{}`",
                bounded_summary(&output.stdout),
                bounded_summary(&output.stderr)
            )));
        }

        if !self.path.is_file() {
            return Err(SmokeError::new(format!(
                "ffmpeg completed but did not create {}",
                self.path.display()
            )));
        }

        Ok(self)
    }
}

fn parse_ffprobe_metadata(bytes: &[u8]) -> SmokeResult<SmokeMetadata> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
        SmokeError::new(format!("failed to parse ffprobe JSON metadata: {error}"))
    })?;
    let streams = value
        .get("streams")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| SmokeError::new("ffprobe JSON metadata did not include streams"))?;
    let video_stream = streams.iter().find(|stream| {
        stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("video")
    });
    let audio_stream = streams.iter().find(|stream| {
        stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("audio")
    });
    let video_stream =
        video_stream.ok_or_else(|| SmokeError::new("ffprobe did not report a video stream"))?;
    let format_duration = value
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(serde_json::Value::as_str);
    let stream_duration = video_stream
        .get("duration")
        .and_then(serde_json::Value::as_str);
    let duration_microseconds = format_duration
        .or(stream_duration)
        .ok_or_else(|| SmokeError::new("ffprobe did not report media duration"))
        .and_then(parse_decimal_seconds_to_microseconds)?;
    let frame_rate = video_stream
        .get("r_frame_rate")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| SmokeError::new("ffprobe did not report video frame rate"))
        .and_then(parse_rational_frame_rate)?;

    Ok(SmokeMetadata {
        duration_microseconds,
        width: json_u32(video_stream, "width")?,
        height: json_u32(video_stream, "height")?,
        frame_rate_numerator: frame_rate.0,
        frame_rate_denominator: frame_rate.1,
        has_video_stream: true,
        has_audio_stream: audio_stream.is_some(),
    })
}

fn json_u32(value: &serde_json::Value, key: &str) -> SmokeResult<u32> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| SmokeError::new(format!("ffprobe did not report numeric {key}")))
}

fn parse_decimal_seconds_to_microseconds(value: &str) -> SmokeResult<u64> {
    let (whole, fractional) = value
        .split_once('.')
        .map_or((value, ""), |(whole, fractional)| (whole, fractional));
    let whole_micros = whole
        .parse::<u64>()
        .map_err(|error| SmokeError::new(format!("invalid duration seconds `{value}`: {error}")))?
        .checked_mul(1_000_000)
        .ok_or_else(|| SmokeError::new(format!("duration is too large `{value}`")))?;
    if !fractional.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(SmokeError::new(format!(
            "invalid duration fraction `{value}`"
        )));
    }

    let mut fraction = fractional.chars().take(6).collect::<String>();

    while fraction.len() < 6 {
        fraction.push('0');
    }

    let fraction_micros = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u64>().map_err(|error| {
            SmokeError::new(format!("invalid duration fraction `{value}`: {error}"))
        })?
    };

    whole_micros
        .checked_add(fraction_micros)
        .ok_or_else(|| SmokeError::new(format!("duration is too large `{value}`")))
}

fn parse_rational_frame_rate(value: &str) -> SmokeResult<(u32, u32)> {
    let (numerator, denominator) = value
        .split_once('/')
        .ok_or_else(|| SmokeError::new(format!("invalid frame rate `{value}`")))?;
    let numerator = numerator.parse::<u32>().map_err(|error| {
        SmokeError::new(format!("invalid frame rate numerator `{value}`: {error}"))
    })?;
    let denominator = denominator.parse::<u32>().map_err(|error| {
        SmokeError::new(format!("invalid frame rate denominator `{value}`: {error}"))
    })?;

    if numerator == 0 {
        return Err(SmokeError::new("frame rate numerator cannot be zero"));
    }

    if denominator == 0 {
        return Err(SmokeError::new("frame rate denominator cannot be zero"));
    }

    Ok((numerator, denominator))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_tiny_lavfi_media_creates_temporary_mp4() {
        let media = generate_tiny_lavfi_media().expect(
            "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
        );

        assert!(
            media.output_path().is_file(),
            "tiny lavfi smoke output should exist"
        );
        assert_eq!(
            media
                .output_path()
                .extension()
                .and_then(|value| value.to_str()),
            Some("mp4")
        );
        assert!(
            media
                .output_path()
                .ancestors()
                .any(|path| path.file_name().and_then(|value| value.to_str())
                    == Some("media-generated")),
            "generated media should live under a media-generated temp directory"
        );
    }

    #[test]
    fn generated_material_fixtures_probe_to_expected_metadata() {
        let runtime = discover_runtime_config().expect(
            "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
        );
        let executor = DesktopFfmpegExecutor::default();

        for fixture in generate_material_fixtures(&executor, &runtime)
            .expect("material fixtures should generate")
        {
            assert!(
                fixture.path().is_file(),
                "generated material fixture should exist"
            );
            assert!(
                fixture
                    .path()
                    .ancestors()
                    .any(|path| path.file_name().and_then(|value| value.to_str())
                        == Some("media-generated")),
                "generated media should live under a media-generated temp directory"
            );

            let metadata =
                media_runtime::probe_material_metadata(&executor, &runtime, fixture.path())
                    .expect("generated material fixture should probe");
            fixture
                .assert_probe_metadata(&metadata)
                .expect("generated fixture metadata should match expectations");
        }
    }

    #[test]
    fn smoke_metadata_parsers_reject_malformed_duration_and_zero_frame_rate() {
        let invalid_fraction = parse_decimal_seconds_to_microseconds("1.-5")
            .expect_err("malformed duration fraction should fail");
        assert!(invalid_fraction.to_string().contains("duration fraction"));

        let overflowing_duration = parse_decimal_seconds_to_microseconds(&u64::MAX.to_string())
            .expect_err("overflowing duration should fail");
        assert!(overflowing_duration.to_string().contains("too large"));

        let zero_numerator =
            parse_rational_frame_rate("0/1").expect_err("zero numerator frame rate should fail");
        assert!(zero_numerator.to_string().contains("numerator"));
    }
}

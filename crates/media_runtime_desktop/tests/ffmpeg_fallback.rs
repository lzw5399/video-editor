use std::cell::RefCell;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::rc::Rc;

use media_runtime::{
    BinaryKind, DecodeErrorKind, DiscoveredBinary, DiscoverySource, FfmpegExecutor,
    MAX_STDERR_SUMMARY_BYTES, MediaIoErrorKind, MediaOpenRequest, MediaReader, RuntimeConfig,
    StreamId, VideoDecodeRequest, VideoFrameStorage, VideoPixelFormat, discover_runtime_config,
};
use media_runtime_desktop::{
    DesktopFfmpegExecutor, FfmpegCpuFrameFingerprintRequest, FfmpegFallbackMediaReader,
    decode_ffmpeg_cpu_frame_fingerprint,
};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

#[test]
fn ffmpeg_fallback_decodes_h264_fixture_into_cpu_frame_lease() {
    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available in the bundled runtime directory; run pnpm --dir apps/desktop-electron run provision:ffmpeg-runtime or set VE_BUNDLED_FFMPEG_DIR",
    );
    let executor = DesktopFfmpegExecutor::default();
    let fixture = H264Fixture::generate(&executor, &runtime);
    let reader = FfmpegFallbackMediaReader::new(executor, runtime);

    let session = reader
        .open(MediaOpenRequest {
            material_uri: fixture.path.clone(),
            requested_streams: vec![StreamId(0)],
        })
        .expect("fixture should open through the FFmpeg fallback reader");
    let mut decoder = session
        .video_decoder(StreamId(0))
        .expect("video stream decoder should be available");

    let frame = decoder
        .decode_at(VideoDecodeRequest {
            source_time_us: 0,
            playback_generation: Some(42),
        })
        .expect("first frame should decode into a CPU frame lease");

    assert_eq!(frame.owner_session, session.session_id());
    assert_eq!(frame.playback_generation, Some(42));
    assert_eq!(frame.source_time_us, 0);
    assert_eq!(frame.dimensions.width, 160);
    assert_eq!(frame.dimensions.height, 90);
    assert_eq!(frame.pixel_format, VideoPixelFormat::Rgba8);
    assert!(
        !frame.color.diagnostics.is_empty(),
        "unknown color metadata must carry diagnostics"
    );
    match frame.storage {
        VideoFrameStorage::Cpu(handle) => {
            assert_eq!(handle.owner_session, session.session_id());
            assert_eq!(handle.generation, Some(42));
            assert_eq!(handle.estimated_byte_len, 160 * 90 * 4);
        }
        other => panic!("expected CPU frame storage, got {other:?}"),
    }
}

#[test]
fn ffmpeg_fallback_frame_fingerprint_hashes_decoded_rgba_bytes_without_returning_pixels() {
    let temp = TempDir::new("ffmpeg-fallback-fingerprint");
    let media = temp.path.join("input.mp4");
    fs::write(&media, b"placeholder").expect("media placeholder should write");
    let runtime = fake_runtime(temp.path.join("ffmpeg"), temp.path.join("ffprobe"));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let pixels = vec![
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let executor = ScriptedExecutor {
        calls: Rc::clone(&calls),
        ffprobe_stdout: br#"{"streams":[{"index":0,"codec_type":"video","codec_name":"h264","width":2,"height":2,"r_frame_rate":"30/1","duration":"1.000000"}],"format":{"duration":"1.000000"}}"#
            .to_vec(),
        ffmpeg_output: ScriptedFfmpegOutput::Success {
            stdout: pixels.clone(),
        },
    };

    let fingerprint = decode_ffmpeg_cpu_frame_fingerprint(
        &executor,
        &runtime,
        &FfmpegCpuFrameFingerprintRequest {
            material_uri: media.clone(),
            source_time_us: 500_000,
        },
    )
    .expect("scripted RGBA frame should fingerprint");

    assert!(fingerprint.digest.starts_with("blake3:v1:"));
    assert_eq!(fingerprint.width, 2);
    assert_eq!(fingerprint.height, 2);
    assert_eq!(fingerprint.byte_count, pixels.len());
    assert_eq!(fingerprint.source_time_us, 500_000);

    let calls = calls.borrow();
    let ffmpeg_call = calls
        .iter()
        .find(|call| call.binary.ends_with("ffmpeg"))
        .expect("fingerprint should call ffmpeg");
    assert!(ffmpeg_call.args.contains(&"-ss".to_owned()));
    assert!(ffmpeg_call.args.contains(&"0.500000".to_owned()));
    assert!(ffmpeg_call.args.contains(&"-f".to_owned()));
    assert!(ffmpeg_call.args.contains(&"rawvideo".to_owned()));
    assert!(ffmpeg_call.args.contains(&"rgba".to_owned()));
    assert!(ffmpeg_call.args.contains(&media.display().to_string()));
}

#[test]
fn ffmpeg_fallback_classifies_missing_ffmpeg_without_panicking() {
    let temp = TempDir::new("ffmpeg-fallback-missing");
    let media = temp.path.join("input.mp4");
    fs::write(&media, b"placeholder").expect("media placeholder should write");
    let runtime = fake_runtime(temp.path.join("missing-ffmpeg"), temp.path.join("ffprobe"));
    let reader = FfmpegFallbackMediaReader::new(UnavailableExecutor, runtime);

    let error = match reader.open(MediaOpenRequest {
        material_uri: media,
        requested_streams: vec![StreamId(0)],
    }) {
        Ok(_) => panic!("missing FFmpeg should be classified"),
        Err(error) => error,
    };

    assert_eq!(error.kind, MediaIoErrorKind::RuntimeUnavailable);
    assert!(error.message.contains("FfmpegUnavailable"));
}

#[test]
fn ffmpeg_fallback_uses_argument_arrays_and_bounds_decode_output_summaries() {
    let temp = TempDir::new("ffmpeg-fallback-args");
    let media = temp.path.join("input.mp4");
    fs::write(&media, b"placeholder").expect("media placeholder should write");
    let runtime = fake_runtime(temp.path.join("ffmpeg"), temp.path.join("ffprobe"));
    let calls = Rc::new(RefCell::new(Vec::new()));
    let executor = ScriptedExecutor {
        calls: Rc::clone(&calls),
        ffprobe_stdout: br#"{"streams":[{"index":0,"codec_type":"video","codec_name":"h264","width":2,"height":2,"r_frame_rate":"30/1","duration":"1.000000"}],"format":{"duration":"1.000000"}}"#
            .to_vec(),
        ffmpeg_output: ScriptedFfmpegOutput::Failure {
            stdout: "x".repeat(MAX_STDERR_SUMMARY_BYTES + 512).into_bytes(),
            stderr: "y".repeat(MAX_STDERR_SUMMARY_BYTES + 512).into_bytes(),
        },
    };
    let reader = FfmpegFallbackMediaReader::new(executor, runtime);
    let session = reader
        .open(MediaOpenRequest {
            material_uri: media.clone(),
            requested_streams: vec![StreamId(0)],
        })
        .expect("scripted ffprobe metadata should open a session");
    let mut decoder = session
        .video_decoder(StreamId(0))
        .expect("scripted video decoder should be available");

    let error = decoder
        .decode_at(VideoDecodeRequest {
            source_time_us: 500_000,
            playback_generation: Some(7),
        })
        .expect_err("scripted FFmpeg failure should be classified");

    assert_eq!(error.kind, DecodeErrorKind::RuntimeFailure);
    assert!(error.message.len() <= MAX_STDERR_SUMMARY_BYTES * 2 + 256);

    let calls = calls.borrow();
    let ffmpeg_call = calls
        .iter()
        .find(|call| call.binary.ends_with("ffmpeg"))
        .expect("decode should call ffmpeg");
    assert!(ffmpeg_call.args.contains(&"-ss".to_owned()));
    assert!(ffmpeg_call.args.contains(&"0.500000".to_owned()));
    assert!(ffmpeg_call.args.contains(&"-i".to_owned()));
    assert!(ffmpeg_call.args.contains(&media.display().to_string()));
    assert!(ffmpeg_call.args.contains(&"-f".to_owned()));
    assert!(ffmpeg_call.args.contains(&"rawvideo".to_owned()));
    assert!(
        ffmpeg_call
            .args
            .iter()
            .all(|arg| !arg.contains("&&") && !arg.contains(";") && !arg.contains("|")),
        "FFmpeg fallback must pass argv elements, not shell-concatenated command strings"
    );
}

struct H264Fixture {
    _temp: TempDir,
    path: PathBuf,
}

impl H264Fixture {
    fn generate(executor: &impl FfmpegExecutor, runtime: &RuntimeConfig) -> Self {
        let temp = TempDir::new("ffmpeg-fallback-fixture");
        let path = temp.path.join("fixture.mp4");
        let args = os_args(&[
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=160x90:rate=10:duration=1",
            "-frames:v",
            "10",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            path.to_str().expect("fixture path should be UTF-8"),
        ]);
        let output = executor
            .run(&runtime.ffmpeg.path, &args)
            .expect("FFmpeg fixture generation should launch");
        assert!(
            output.status.success(),
            "FFmpeg fixture generation failed: stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(path.is_file());
        Self { _temp: temp, path }
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&path).expect("temp directory should create");
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Debug)]
struct RecordedCall {
    binary: String,
    args: Vec<String>,
}

struct ScriptedExecutor {
    calls: Rc<RefCell<Vec<RecordedCall>>>,
    ffprobe_stdout: Vec<u8>,
    ffmpeg_output: ScriptedFfmpegOutput,
}

enum ScriptedFfmpegOutput {
    Success { stdout: Vec<u8> },
    Failure { stdout: Vec<u8>, stderr: Vec<u8> },
}

impl FfmpegExecutor for ScriptedExecutor {
    fn executor_name(&self) -> &'static str {
        "scripted-ffmpeg-fallback-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        true
    }

    fn run_version_probe(&self, binary: &Path) -> io::Result<Output> {
        self.run(binary, &[])
    }

    fn run(&self, binary: &Path, args: &[OsString]) -> io::Result<Output> {
        self.calls.borrow_mut().push(RecordedCall {
            binary: binary.display().to_string(),
            args: args
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
        });

        if binary.ends_with("ffprobe") {
            return Ok(Output {
                status: success_status(),
                stdout: self.ffprobe_stdout.clone(),
                stderr: Vec::new(),
            });
        }

        match &self.ffmpeg_output {
            ScriptedFfmpegOutput::Success { stdout } => Ok(Output {
                status: success_status(),
                stdout: stdout.clone(),
                stderr: Vec::new(),
            }),
            ScriptedFfmpegOutput::Failure { stdout, stderr } => Ok(Output {
                status: failure_status(),
                stdout: stdout.clone(),
                stderr: stderr.clone(),
            }),
        }
    }
}

struct UnavailableExecutor;

impl FfmpegExecutor for UnavailableExecutor {
    fn executor_name(&self) -> &'static str {
        "unavailable-ffmpeg-fallback-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        false
    }

    fn run_version_probe(&self, _binary: &Path) -> io::Result<Output> {
        unreachable!("missing binary should be classified before process launch")
    }

    fn run(&self, _binary: &Path, _args: &[OsString]) -> io::Result<Output> {
        unreachable!("missing binary should be classified before process launch")
    }
}

fn fake_runtime(ffmpeg_path: PathBuf, ffprobe_path: PathBuf) -> RuntimeConfig {
    let directory = ffmpeg_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("/runtime/bin"));
    RuntimeConfig {
        ffmpeg: DiscoveredBinary {
            kind: BinaryKind::Ffmpeg,
            path: ffmpeg_path,
            source: DiscoverySource::Bundled {
                directory: directory.clone(),
            },
            version: "ffmpeg version fake".to_owned(),
        },
        ffprobe: DiscoveredBinary {
            kind: BinaryKind::Ffprobe,
            path: ffprobe_path,
            source: DiscoverySource::Bundled { directory },
            version: "ffprobe version fake".to_owned(),
        },
    }
}

fn os_args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos()
}

#[cfg(unix)]
fn success_status() -> ExitStatus {
    ExitStatus::from_raw(0)
}

#[cfg(unix)]
fn failure_status() -> ExitStatus {
    ExitStatus::from_raw(1 << 8)
}

#[cfg(windows)]
fn success_status() -> ExitStatus {
    ExitStatus::from_raw(0)
}

#[cfg(windows)]
fn failure_status() -> ExitStatus {
    ExitStatus::from_raw(1)
}

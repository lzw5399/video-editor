use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use media_runtime::{
    FfmpegExecutor, MediaIoFallbackReason, MediaOpenRequest, RuntimeDeviceId, SelectedDecodePath,
    StreamId, TextureBackend, VideoDecodeRequest, VideoFrameStorage, VideoPixelFormat,
    discover_runtime_config,
};
use media_runtime_desktop::{
    DesktopFfmpegExecutor, MacosMediaReader, MacosTextureInteropPolicy,
    select_macos_texture_interop_fallback,
};

#[test]
fn macos_texture_interop_defaults_to_native_frame_fallback_until_device_is_proven() {
    let selection = select_macos_texture_interop_fallback(true, None, false).expect(
        "native decode frame path should remain available when texture interop is unproven",
    );

    assert_eq!(
        selection.selected_path,
        SelectedDecodePath::NativeHardwareCpuCopy
    );
    assert_eq!(
        selection.reason,
        Some(MediaIoFallbackReason::TextureInteropUnavailable)
    );
    assert_eq!(
        selection.diagnostics[0].path,
        SelectedDecodePath::NativeHardwareTexture
    );
    assert_eq!(
        selection.diagnostics[0].reason,
        Some(MediaIoFallbackReason::TextureInteropUnavailable)
    );
}

#[test]
fn macos_texture_interop_reports_device_mismatch_before_cpu_or_ffmpeg_fallback() {
    let preview_device = RuntimeDeviceId {
        backend: TextureBackend::MetalTexture,
        adapter_id: "preview-adapter".to_owned(),
        device_id: "preview-device".to_owned(),
    };
    let native_device = RuntimeDeviceId {
        backend: TextureBackend::MetalTexture,
        adapter_id: "native-adapter".to_owned(),
        device_id: "native-device".to_owned(),
    };

    let selection =
        select_macos_texture_interop_fallback(true, Some((&preview_device, &native_device)), true)
            .expect("native decode frame path should remain available when devices mismatch");

    assert_eq!(
        selection.selected_path,
        SelectedDecodePath::NativeHardwareCpuCopy
    );
    assert_eq!(
        selection.reason,
        Some(MediaIoFallbackReason::DeviceMismatch)
    );
    assert_eq!(
        selection.diagnostics[0].reason,
        Some(MediaIoFallbackReason::DeviceMismatch)
    );
}

#[test]
fn macos_texture_interop_selects_texture_only_for_compatible_metal_device() {
    let device = RuntimeDeviceId {
        backend: TextureBackend::MetalTexture,
        adapter_id: "apple-gpu".to_owned(),
        device_id: "registry-123".to_owned(),
    };

    let selection = select_macos_texture_interop_fallback(true, Some((&device, &device)), true)
        .expect("compatible Metal device should select native texture path");

    assert_eq!(
        selection.selected_path,
        SelectedDecodePath::NativeHardwareTexture
    );
    assert_eq!(selection.reason, None);
}

#[test]
#[cfg(not(target_os = "macos"))]
fn macos_reader_reports_unsupported_platform_without_panicking() {
    let reader = MacosMediaReader::new();
    let error = reader
        .open(MediaOpenRequest {
            material_uri: PathBuf::from("/tmp/input.mp4"),
            requested_streams: vec![StreamId(0)],
        })
        .expect_err("macOS native reader should be unavailable on non-macOS");

    assert!(error.message.contains("UnsupportedPlatform"));
}

#[test]
#[cfg(target_os = "macos")]
fn macos_native_decodes_h264_fixture_into_corevideo_frame_lease_when_enabled() {
    if std::env::var_os("VIDEO_EDITOR_TEST_NATIVE_MEDIA").is_none() {
        eprintln!("skipping macOS native decode proof; set VIDEO_EDITOR_TEST_NATIVE_MEDIA=1");
        return;
    }

    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available in the bundled runtime directory; run pnpm --dir apps/desktop-electron run provision:ffmpeg-runtime",
    );
    let executor = DesktopFfmpegExecutor::default();
    let fixture = H264Fixture::generate(&executor, &runtime);
    let reader = MacosMediaReader::new();

    let session = reader
        .open_session(MediaOpenRequest {
            material_uri: fixture.path.clone(),
            requested_streams: vec![StreamId(0)],
        })
        .expect("fixture should open through AVFoundation");
    let mut decoder = session
        .native_video_decoder(StreamId(0))
        .expect("H.264 video stream should have a native decoder");

    let frame = decoder
        .decode_at(VideoDecodeRequest {
            source_time_us: 0,
            playback_generation: Some(17),
        })
        .expect("first H.264 frame should decode through AVFoundation/CoreVideo");

    assert_eq!(frame.owner_session, session.session_id());
    assert_eq!(frame.playback_generation, Some(17));
    assert_eq!(frame.source_time_us, 0);
    assert_eq!(frame.dimensions.width, 160);
    assert_eq!(frame.dimensions.height, 90);
    assert_eq!(frame.pixel_format, VideoPixelFormat::Nv12);
    assert!(
        !frame.color.diagnostics.is_empty(),
        "native decode must preserve unknown color metadata as a diagnostic"
    );

    match &frame.storage {
        VideoFrameStorage::PlatformOpaque(handle) => {
            assert_eq!(handle.owner_session, session.session_id());
            assert_eq!(handle.generation, Some(17));
            assert!(handle.label.contains("CoreVideoPixelBuffer"));
        }
        other => panic!("expected CoreVideo platform-opaque frame storage, got {other:?}"),
    }

    let diagnostic = session
        .release_frame(frame.release.clone())
        .expect("native frame lease should release");
    assert_eq!(diagnostic.lease_id, frame.release);
    assert_eq!(session.outstanding_native_lease_count(), 0);
}

#[test]
#[cfg(target_os = "macos")]
fn macos_native_decode_honors_requested_source_time() {
    if std::env::var_os("VIDEO_EDITOR_TEST_NATIVE_MEDIA").is_none() {
        eprintln!("skipping macOS native decode seek proof; set VIDEO_EDITOR_TEST_NATIVE_MEDIA=1");
        return;
    }

    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available in the bundled runtime directory; run pnpm --dir apps/desktop-electron run provision:ffmpeg-runtime",
    );
    let executor = DesktopFfmpegExecutor::default();
    let fixture = H264Fixture::generate(&executor, &runtime);
    let reader = MacosMediaReader::new();

    let session = reader
        .open_session(MediaOpenRequest {
            material_uri: fixture.path.clone(),
            requested_streams: vec![StreamId(0)],
        })
        .expect("fixture should open through AVFoundation");
    let mut decoder = session
        .native_video_decoder(StreamId(0))
        .expect("H.264 video stream should have a native decoder");

    let first = decoder
        .decode_at(VideoDecodeRequest {
            source_time_us: 0,
            playback_generation: Some(21),
        })
        .expect("first frame should decode through AVFoundation/CoreVideo");
    let later = decoder
        .decode_at(VideoDecodeRequest {
            source_time_us: 500_000,
            playback_generation: Some(21),
        })
        .expect("later frame should decode through AVFoundation/CoreVideo");

    assert_eq!(first.source_time_us, 0);
    assert!(
        later.source_time_us >= 400_000,
        "later decode must not silently return the first frame: {:?}",
        later.source_time_us
    );
    assert_ne!(
        later.frame_index, first.frame_index,
        "later decode must advance frame index instead of reusing the first frame"
    );

    session
        .release_frame(first.release)
        .expect("first native frame should release");
    session
        .release_frame(later.release)
        .expect("later native frame should release");
}

#[test]
#[cfg(target_os = "macos")]
fn macos_texture_decode_degrades_when_texture_interop_is_disabled() {
    if std::env::var_os("VIDEO_EDITOR_TEST_NATIVE_MEDIA").is_none() {
        eprintln!("skipping macOS texture fallback proof; set VIDEO_EDITOR_TEST_NATIVE_MEDIA=1");
        return;
    }

    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available in the bundled runtime directory; run pnpm --dir apps/desktop-electron run provision:ffmpeg-runtime",
    );
    let executor = DesktopFfmpegExecutor::default();
    let fixture = H264Fixture::generate(&executor, &runtime);
    let reader =
        MacosMediaReader::new().with_texture_interop_policy(MacosTextureInteropPolicy::disabled());
    let session = reader
        .open_session(MediaOpenRequest {
            material_uri: fixture.path.clone(),
            requested_streams: vec![StreamId(0)],
        })
        .expect("fixture should open through AVFoundation");
    let mut decoder = session
        .native_video_decoder(StreamId(0))
        .expect("H.264 video stream should have a native decoder");

    let frame = decoder
        .decode_at(VideoDecodeRequest {
            source_time_us: 0,
            playback_generation: Some(18),
        })
        .expect("disabled texture interop should still return a native frame lease");

    assert!(matches!(
        frame.storage,
        VideoFrameStorage::PlatformOpaque(_)
    ));
    assert!(
        session
            .last_fallback_selection()
            .expect("texture fallback should be recorded")
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.reason
                == Some(MediaIoFallbackReason::TextureInteropUnavailable))
    );

    session
        .release_frame(frame.release)
        .expect("native platform-opaque frame should release");
}

#[test]
#[cfg(target_os = "macos")]
fn macos_texture_decode_returns_metal_texture_when_preview_device_is_compatible() {
    if std::env::var_os("VIDEO_EDITOR_TEST_NATIVE_MEDIA").is_none() {
        eprintln!("skipping macOS texture interop proof; set VIDEO_EDITOR_TEST_NATIVE_MEDIA=1");
        return;
    }

    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available in the bundled runtime directory; run pnpm --dir apps/desktop-electron run provision:ffmpeg-runtime",
    );
    let executor = DesktopFfmpegExecutor::default();
    let fixture = H264Fixture::generate(&executor, &runtime);
    let preview_device = macos_system_metal_device_id()
        .expect("test host should expose the system Metal device when native media is enabled");
    let reader = MacosMediaReader::new().with_texture_interop_policy(
        MacosTextureInteropPolicy::for_preview_device(preview_device.clone()),
    );
    let session = reader
        .open_session(MediaOpenRequest {
            material_uri: fixture.path.clone(),
            requested_streams: vec![StreamId(0)],
        })
        .expect("fixture should open through AVFoundation");
    let mut decoder = session
        .native_video_decoder(StreamId(0))
        .expect("H.264 video stream should have a native decoder");

    let frame = decoder
        .decode_at(VideoDecodeRequest {
            source_time_us: 0,
            playback_generation: Some(20),
        })
        .expect("compatible Metal preview device should decode into a texture lease");

    match &frame.storage {
        VideoFrameStorage::Texture(texture) => {
            assert_eq!(texture.owner_session, session.session_id());
            assert_eq!(texture.generation, 20);
            assert_eq!(texture.backend, TextureBackend::MetalTexture);
            assert_eq!(texture.device_id, preview_device);
            assert_eq!(texture.dimensions.width, 160);
            assert_eq!(texture.dimensions.height, 90);
        }
        other => panic!("expected compatible Metal texture frame storage, got {other:?}"),
    }
    assert_eq!(
        session
            .last_fallback_selection()
            .expect("texture path selection should be recorded")
            .selected_path,
        SelectedDecodePath::NativeHardwareTexture
    );

    session
        .release_frame(frame.release)
        .expect("native texture frame should release");
}

#[test]
#[cfg(target_os = "macos")]
fn macos_native_close_reports_unreleased_corevideo_leases() {
    if std::env::var_os("VIDEO_EDITOR_TEST_NATIVE_MEDIA").is_none() {
        eprintln!("skipping macOS native lease close proof; set VIDEO_EDITOR_TEST_NATIVE_MEDIA=1");
        return;
    }

    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available in the bundled runtime directory; run pnpm --dir apps/desktop-electron run provision:ffmpeg-runtime",
    );
    let executor = DesktopFfmpegExecutor::default();
    let fixture = H264Fixture::generate(&executor, &runtime);
    let reader = MacosMediaReader::new();
    let session = reader
        .open_session(MediaOpenRequest {
            material_uri: fixture.path.clone(),
            requested_streams: vec![StreamId(0)],
        })
        .expect("fixture should open through AVFoundation");
    let mut decoder = session
        .native_video_decoder(StreamId(0))
        .expect("H.264 video stream should have a native decoder");

    let frame = decoder
        .decode_at(VideoDecodeRequest {
            source_time_us: 0,
            playback_generation: Some(19),
        })
        .expect("first frame should decode into a native lease");
    let release = frame.release.clone();

    let close = session.close();
    assert_eq!(close.leak_diagnostics.len(), 1);
    assert_eq!(close.leak_diagnostics[0].lease_id, release);
    assert_eq!(session.outstanding_native_lease_count(), 0);
}

struct H264Fixture {
    _temp: TempDir,
    path: PathBuf,
}

#[cfg(target_os = "macos")]
fn macos_system_metal_device_id() -> Option<RuntimeDeviceId> {
    use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice};

    let device = MTLCreateSystemDefaultDevice()?;
    Some(RuntimeDeviceId {
        backend: TextureBackend::MetalTexture,
        adapter_id: "apple-metal".to_owned(),
        device_id: format!("registry-{}", device.registryID()),
    })
}

impl H264Fixture {
    fn generate(executor: &impl FfmpegExecutor, runtime: &media_runtime::RuntimeConfig) -> Self {
        let temp = TempDir::new("macos-native-media-fixture");
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
            "-g",
            "1",
            "-keyint_min",
            "1",
            "-sc_threshold",
            "0",
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

fn os_args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos()
}

use std::path::PathBuf;

use media_runtime::{
    MediaIoFallbackReason, MediaOpenRequest, RuntimeDeviceId, SelectedDecodePath, StreamId,
    TextureBackend,
};
use media_runtime_desktop::{WindowsMediaReader, select_windows_texture_interop_fallback};

#[cfg(windows)]
use media_runtime_desktop::WindowsTextureInteropPolicy;

#[test]
fn windows_texture_interop_defaults_to_native_frame_fallback_until_device_is_proven() {
    let selection = select_windows_texture_interop_fallback(true, None, false).expect(
        "native decode frame path should remain available when D3D texture interop is unproven",
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
fn windows_texture_interop_reports_device_mismatch_before_cpu_or_ffmpeg_fallback() {
    let preview_device = RuntimeDeviceId {
        backend: TextureBackend::D3d12Resource,
        adapter_id: "preview-adapter".to_owned(),
        device_id: "preview-device".to_owned(),
    };
    let native_device = RuntimeDeviceId {
        backend: TextureBackend::D3d11Texture2D,
        adapter_id: "native-adapter".to_owned(),
        device_id: "native-device".to_owned(),
    };

    let selection = select_windows_texture_interop_fallback(
        true,
        Some((&preview_device, &native_device)),
        true,
    )
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
fn windows_texture_interop_selects_texture_only_for_compatible_d3d_device() {
    let device = RuntimeDeviceId {
        backend: TextureBackend::D3d11Texture2D,
        adapter_id: "luid-00000001-00000002".to_owned(),
        device_id: "vendor-10de-device-2684".to_owned(),
    };

    let selection = select_windows_texture_interop_fallback(true, Some((&device, &device)), true)
        .expect("compatible D3D device should select native texture path");

    assert_eq!(
        selection.selected_path,
        SelectedDecodePath::NativeHardwareTexture
    );
    assert_eq!(selection.reason, None);
}

#[test]
#[cfg(not(windows))]
fn windows_reader_reports_unsupported_platform_without_panicking() {
    let reader = WindowsMediaReader::new();
    let error = reader
        .open_session(MediaOpenRequest {
            material_uri: PathBuf::from("/tmp/input.mp4"),
            requested_streams: vec![StreamId(0)],
        })
        .expect_err("Windows native reader should be unavailable on non-Windows");

    assert!(error.message.contains("UnsupportedPlatform"));
}

#[test]
#[cfg(windows)]
fn windows_native_decode_proof_is_explicitly_env_gated() {
    use media_runtime::{VideoDecodeRequest, VideoFrameStorage, discover_runtime_config};
    use media_runtime_desktop::DesktopFfmpegExecutor;

    if std::env::var_os("VIDEO_EDITOR_TEST_NATIVE_MEDIA").is_none() {
        eprintln!("skipping Windows native decode proof; set VIDEO_EDITOR_TEST_NATIVE_MEDIA=1");
        return;
    }

    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
    );
    let executor = DesktopFfmpegExecutor::default();
    let fixture = H264Fixture::generate(&executor, &runtime);
    let reader = WindowsMediaReader::new();

    let session = reader
        .open_session(MediaOpenRequest {
            material_uri: fixture.path.clone(),
            requested_streams: vec![StreamId(0)],
        })
        .expect("fixture should open through Media Foundation");
    let mut decoder = session
        .native_video_decoder(StreamId(0))
        .expect("H.264 video stream should have a native decoder");

    let frame = decoder
        .decode_at(VideoDecodeRequest {
            source_time_us: 0,
            playback_generation: Some(21),
        })
        .expect("first H.264 frame should decode through Media Foundation");

    assert_eq!(frame.owner_session, session.session_id());
    assert_eq!(frame.playback_generation, Some(21));
    assert_eq!(frame.source_time_us, 0);
    assert_eq!(frame.dimensions.width, 160);
    assert_eq!(frame.dimensions.height, 90);
    assert!(
        !frame.color.diagnostics.is_empty(),
        "native decode must preserve unknown color metadata as a diagnostic"
    );

    match &frame.storage {
        VideoFrameStorage::PlatformOpaque(handle) => {
            assert_eq!(handle.owner_session, session.session_id());
            assert_eq!(handle.generation, Some(21));
            assert!(handle.label.contains("MediaFoundationSample"));
        }
        other => panic!("expected Media Foundation platform-opaque frame storage, got {other:?}"),
    }

    let diagnostic = session
        .release_frame(frame.release.clone())
        .expect("native frame lease should release");
    assert_eq!(diagnostic.lease_id, frame.release);
    assert_eq!(session.outstanding_native_lease_count(), 0);
}

#[test]
#[cfg(windows)]
fn windows_texture_decode_degrades_when_texture_interop_is_disabled() {
    use media_runtime::{VideoDecodeRequest, VideoFrameStorage, discover_runtime_config};
    use media_runtime_desktop::DesktopFfmpegExecutor;

    if std::env::var_os("VIDEO_EDITOR_TEST_NATIVE_MEDIA").is_none() {
        eprintln!("skipping Windows texture fallback proof; set VIDEO_EDITOR_TEST_NATIVE_MEDIA=1");
        return;
    }

    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
    );
    let executor = DesktopFfmpegExecutor::default();
    let fixture = H264Fixture::generate(&executor, &runtime);
    let reader = WindowsMediaReader::new()
        .with_texture_interop_policy(WindowsTextureInteropPolicy::disabled());
    let session = reader
        .open_session(MediaOpenRequest {
            material_uri: fixture.path.clone(),
            requested_streams: vec![StreamId(0)],
        })
        .expect("fixture should open through Media Foundation");
    let mut decoder = session
        .native_video_decoder(StreamId(0))
        .expect("H.264 video stream should have a native decoder");

    let frame = decoder
        .decode_at(VideoDecodeRequest {
            source_time_us: 0,
            playback_generation: Some(22),
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
#[cfg(windows)]
fn windows_native_close_reports_unreleased_media_foundation_leases() {
    use media_runtime::{VideoDecodeRequest, discover_runtime_config};
    use media_runtime_desktop::DesktopFfmpegExecutor;

    if std::env::var_os("VIDEO_EDITOR_TEST_NATIVE_MEDIA").is_none() {
        eprintln!(
            "skipping Windows native lease close proof; set VIDEO_EDITOR_TEST_NATIVE_MEDIA=1"
        );
        return;
    }

    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
    );
    let executor = DesktopFfmpegExecutor::default();
    let fixture = H264Fixture::generate(&executor, &runtime);
    let reader = WindowsMediaReader::new();
    let session = reader
        .open_session(MediaOpenRequest {
            material_uri: fixture.path.clone(),
            requested_streams: vec![StreamId(0)],
        })
        .expect("fixture should open through Media Foundation");
    let mut decoder = session
        .native_video_decoder(StreamId(0))
        .expect("H.264 video stream should have a native decoder");

    let frame = decoder
        .decode_at(VideoDecodeRequest {
            source_time_us: 0,
            playback_generation: Some(23),
        })
        .expect("first frame should decode into a native lease");
    let release = frame.release.clone();

    let close = session.close();
    assert_eq!(close.leak_diagnostics.len(), 1);
    assert_eq!(close.leak_diagnostics[0].lease_id, release);
    assert_eq!(session.outstanding_native_lease_count(), 0);
}

#[cfg(windows)]
struct H264Fixture {
    _temp: TempDir,
    path: PathBuf,
}

#[cfg(windows)]
impl H264Fixture {
    fn generate(
        executor: &impl media_runtime::FfmpegExecutor,
        runtime: &media_runtime::RuntimeConfig,
    ) -> Self {
        let temp = TempDir::new("windows-native-media-fixture");
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

#[cfg(windows)]
struct TempDir {
    path: PathBuf,
}

#[cfg(windows)]
impl TempDir {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        std::fs::create_dir_all(&path).expect("temp directory should create");
        Self { path }
    }
}

#[cfg(windows)]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(windows)]
fn os_args(values: &[&str]) -> Vec<std::ffi::OsString> {
    values.iter().map(std::ffi::OsString::from).collect()
}

#[cfg(windows)]
fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos()
}

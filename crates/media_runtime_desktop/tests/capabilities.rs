use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};

use media_runtime::{
    BinaryKind, DiscoveredBinary, DiscoverySource, FfmpegExecutor, MediaIoFallbackReason,
    RuntimeCapabilityStatus, RuntimeConfig,
};
use media_runtime_desktop::probe_desktop_runtime_capabilities;

#[test]
fn desktop_capabilities_report_preserves_ffmpeg_fields_and_adds_media_io_domains() {
    let runtime = fake_runtime_config();
    let executor = FakeExecutor::new()
        .with_version(
            "ffmpeg version test-build\nconfiguration: --enable-libx264 --enable-libass\n",
        )
        .with_probe(
            &["-hide_banner", "-encoders"],
            " V..... libx264 H.264 encoder\n A..... aac AAC encoder\n",
        )
        .with_probe(
            &["-hide_banner", "-filters"],
            " ... ass Render ASS subtitles\n ... subtitles Render text subtitles\n",
        );

    let report = probe_desktop_runtime_capabilities(&executor, &runtime);
    let value = serde_json::to_value(&report).expect("desktop capability report serializes");

    assert_eq!(report.ffmpeg.executor_name, "fake-executor");
    assert_eq!(
        report.ffmpeg.ffmpeg.path,
        PathBuf::from("/runtime/bin/ffmpeg")
    );
    assert!(report.ffmpeg.h264_encoder.available);
    assert!(report.ffmpeg.aac_encoder.available);
    assert!(report.ffmpeg.ass_filter.available);
    assert!(report.ffmpeg.subtitles_filter.available);
    assert!(report.ffmpeg.license_posture.external_runtime);

    assert!(value["mediaIo"]["windows"].is_object());
    assert!(value["mediaIo"]["macos"].is_object());
    assert!(value["mediaIo"]["codecs"].is_array());
    assert!(value["mediaIo"]["pixelFormats"].is_array());
    assert!(value["mediaIo"]["textureInterop"].is_object());
    assert!(value["mediaIo"]["fallbackLadder"].is_object());
}

#[test]
#[cfg(not(windows))]
fn desktop_capabilities_report_marks_windows_domain_unavailable_on_non_windows() {
    let report = probe_desktop_runtime_capabilities(&FakeExecutor::ready(), &fake_runtime_config());

    assert_eq!(
        report.media_io.windows.status,
        RuntimeCapabilityStatus::Unavailable
    );
    assert_eq!(
        report.media_io.windows.fallback_reason,
        Some(MediaIoFallbackReason::UnsupportedPlatform)
    );
    assert!(
        report
            .media_io
            .windows
            .diagnostic
            .as_deref()
            .unwrap_or_default()
            .contains("unsupported platform")
    );
}

#[test]
#[cfg(not(target_os = "macos"))]
fn desktop_capabilities_report_marks_macos_domain_unavailable_on_non_macos() {
    let report = probe_desktop_runtime_capabilities(&FakeExecutor::ready(), &fake_runtime_config());

    assert_eq!(
        report.media_io.macos.status,
        RuntimeCapabilityStatus::Unavailable
    );
    assert_eq!(
        report.media_io.macos.fallback_reason,
        Some(MediaIoFallbackReason::UnsupportedPlatform)
    );
    assert!(
        report
            .media_io
            .macos
            .diagnostic
            .as_deref()
            .unwrap_or_default()
            .contains("unsupported platform")
    );
}

#[test]
fn desktop_capabilities_report_keeps_h264_mp4_mov_as_first_native_acceptance_target_only() {
    let report = probe_desktop_runtime_capabilities(&FakeExecutor::ready(), &fake_runtime_config());

    let h264 = report
        .media_io
        .codecs
        .iter()
        .find(|codec| codec.codec == "h264")
        .expect("H.264 capability should be reported");
    assert!(h264.first_native_hardware_decode_target);
    assert_eq!(h264.containers, vec!["mp4".to_owned(), "mov".to_owned()]);

    for codec_name in ["hevc", "prores", "av1"] {
        let codec = report
            .media_io
            .codecs
            .iter()
            .find(|codec| codec.codec == codec_name)
            .unwrap_or_else(|| panic!("{codec_name} capability should be reported"));
        assert!(!codec.first_native_hardware_decode_target);
        assert_ne!(codec.status, RuntimeCapabilityStatus::Ready);
        assert!(codec.fallback_reason.is_some());
    }
}

#[test]
fn desktop_capabilities_report_orders_fallback_ladder_from_native_texture_to_preview_artifact() {
    let report = probe_desktop_runtime_capabilities(&FakeExecutor::ready(), &fake_runtime_config());
    let paths = report
        .media_io
        .fallback_ladder
        .paths
        .iter()
        .map(|path| path.path)
        .collect::<Vec<_>>();

    assert_eq!(
        paths,
        vec![
            media_runtime::SelectedDecodePath::NativeHardwareTexture,
            media_runtime::SelectedDecodePath::NativeHardwareCpuCopy,
            media_runtime::SelectedDecodePath::NativeSoftwareCpuFrame,
            media_runtime::SelectedDecodePath::FfmpegCpuFrame,
            media_runtime::SelectedDecodePath::FfmpegPreviewArtifact,
        ]
    );
}

fn fake_runtime_config() -> RuntimeConfig {
    RuntimeConfig {
        ffmpeg: DiscoveredBinary {
            kind: BinaryKind::Ffmpeg,
            path: PathBuf::from("/runtime/bin/ffmpeg"),
            source: DiscoverySource::Env {
                variable: "VE_FFMPEG_PATH".to_owned(),
            },
            version: "ffmpeg version test-build".to_owned(),
        },
        ffprobe: DiscoveredBinary {
            kind: BinaryKind::Ffprobe,
            path: PathBuf::from("/runtime/bin/ffprobe"),
            source: DiscoverySource::Path,
            version: "ffprobe version test-build".to_owned(),
        },
    }
}

#[derive(Default)]
struct FakeExecutor {
    version_stdout: Vec<u8>,
    probes: BTreeMap<Vec<String>, Output>,
}

impl FakeExecutor {
    fn new() -> Self {
        Self::default()
    }

    fn ready() -> Self {
        Self::new()
            .with_version("ffmpeg version test-build\n")
            .with_probe(
                &["-hide_banner", "-encoders"],
                " V..... libx264 H.264 encoder\n A..... aac AAC encoder\n",
            )
            .with_probe(
                &["-hide_banner", "-filters"],
                " ... ass Render ASS subtitles\n ... subtitles Render text subtitles\n",
            )
    }

    fn with_version(mut self, stdout: &str) -> Self {
        self.version_stdout = stdout.as_bytes().to_vec();
        self
    }

    fn with_probe(mut self, args: &[&str], stdout: &str) -> Self {
        self.probes.insert(
            args.iter().map(|value| (*value).to_owned()).collect(),
            Output {
                status: success_status(),
                stdout: stdout.as_bytes().to_vec(),
                stderr: Vec::new(),
            },
        );
        self
    }
}

impl FfmpegExecutor for FakeExecutor {
    fn executor_name(&self) -> &'static str {
        "fake-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        true
    }

    fn run_version_probe(&self, _binary: &Path) -> std::io::Result<Output> {
        Ok(Output {
            status: success_status(),
            stdout: self.version_stdout.clone(),
            stderr: Vec::new(),
        })
    }

    fn run(&self, _binary: &Path, args: &[OsString]) -> std::io::Result<Output> {
        let key = args
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        Ok(self.probes.get(&key).cloned().unwrap_or_else(|| Output {
            status: failure_status(),
            stdout: Vec::new(),
            stderr: format!("unexpected probe args: {key:?}").into_bytes(),
        }))
    }
}

#[cfg(unix)]
fn success_status() -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(0)
}

#[cfg(unix)]
fn failure_status() -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(1 << 8)
}

#[cfg(windows)]
fn success_status() -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    ExitStatus::from_raw(0)
}

#[cfg(windows)]
fn failure_status() -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    ExitStatus::from_raw(1)
}

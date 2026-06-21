use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};

use media_runtime::{
    BinaryKind, DiscoveredBinary, DiscoverySource, FfmpegExecutor, MediaIoFallbackCandidate,
    MediaIoFallbackReason, RuntimeConfig, SelectedDecodePath, media_io_fallback_ladder,
    select_media_io_fallback,
};
use media_runtime_desktop::probe_desktop_runtime_capabilities;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

#[test]
fn fallback_ladder_capability_uses_canonical_media_io_order() {
    let expected = vec![
        SelectedDecodePath::NativeHardwareTexture,
        SelectedDecodePath::NativeHardwareCpuCopy,
        SelectedDecodePath::NativeSoftwareCpuFrame,
        SelectedDecodePath::FfmpegCpuFrame,
        SelectedDecodePath::FfmpegPreviewArtifact,
    ];
    assert_eq!(media_io_fallback_ladder(), expected);

    let report = probe_desktop_runtime_capabilities(&FakeExecutor::ready(), &fake_runtime_config());
    let capability_paths = report
        .media_io
        .fallback_ladder
        .paths
        .iter()
        .map(|path| path.path)
        .collect::<Vec<_>>();

    assert_eq!(capability_paths, expected);
}

#[test]
fn fallback_ladder_selection_records_unsupported_codec_reason() {
    let selection = select_media_io_fallback(
        vec![
            MediaIoFallbackCandidate::unavailable(
                SelectedDecodePath::NativeHardwareTexture,
                MediaIoFallbackReason::UnsupportedCodec,
                "native hardware texture cannot decode this codec",
            ),
            MediaIoFallbackCandidate::unavailable(
                SelectedDecodePath::NativeHardwareCpuCopy,
                MediaIoFallbackReason::UnsupportedCodec,
                "native hardware CPU copy cannot decode this codec",
            ),
            MediaIoFallbackCandidate::unavailable(
                SelectedDecodePath::NativeSoftwareCpuFrame,
                MediaIoFallbackReason::UnsupportedCodec,
                "native software decode cannot decode this codec",
            ),
            MediaIoFallbackCandidate::available(SelectedDecodePath::FfmpegCpuFrame),
            MediaIoFallbackCandidate::available(SelectedDecodePath::FfmpegPreviewArtifact),
        ],
        MediaIoFallbackReason::UnsupportedCodec,
    )
    .expect("FFmpeg CPU frame path should be selected");

    assert_eq!(selection.selected_path, SelectedDecodePath::FfmpegCpuFrame);
    assert_eq!(
        selection.reason,
        Some(MediaIoFallbackReason::UnsupportedCodec)
    );
    assert_eq!(selection.diagnostics.len(), 4);
    assert_eq!(
        selection.diagnostics[0].path,
        SelectedDecodePath::NativeHardwareTexture
    );
    assert!(!selection.diagnostics[0].available);
    assert_eq!(
        selection.diagnostics[0].reason,
        Some(MediaIoFallbackReason::UnsupportedCodec)
    );

    let value = serde_json::to_value(&selection).expect("selection should serialize");
    assert_eq!(value["selectedPath"], "ffmpegCpuFrame");
    assert_eq!(value["reason"], "unsupportedCodec");
}

#[test]
fn fallback_ladder_selection_records_unsupported_pixel_format_before_preview_artifact() {
    let selection = select_media_io_fallback(
        vec![
            MediaIoFallbackCandidate::unavailable(
                SelectedDecodePath::NativeHardwareTexture,
                MediaIoFallbackReason::UnsupportedPixelFormat,
                "native texture path cannot import this pixel format",
            ),
            MediaIoFallbackCandidate::unavailable(
                SelectedDecodePath::NativeHardwareCpuCopy,
                MediaIoFallbackReason::UnsupportedPixelFormat,
                "hardware CPU copy cannot convert this pixel format",
            ),
            MediaIoFallbackCandidate::unavailable(
                SelectedDecodePath::NativeSoftwareCpuFrame,
                MediaIoFallbackReason::UnsupportedPixelFormat,
                "native software path cannot decode this pixel format",
            ),
            MediaIoFallbackCandidate::unavailable(
                SelectedDecodePath::FfmpegCpuFrame,
                MediaIoFallbackReason::UnsupportedPixelFormat,
                "FFmpeg CPU frame decode could not produce an accepted format",
            ),
            MediaIoFallbackCandidate::available(SelectedDecodePath::FfmpegPreviewArtifact),
        ],
        MediaIoFallbackReason::UnsupportedPixelFormat,
    )
    .expect("preview artifact should remain the final selected fallback");

    assert_eq!(
        selection.selected_path,
        SelectedDecodePath::FfmpegPreviewArtifact
    );
    assert_eq!(
        selection.reason,
        Some(MediaIoFallbackReason::UnsupportedPixelFormat)
    );
    assert_eq!(selection.diagnostics.len(), 5);
    assert_eq!(
        selection.diagnostics.last().unwrap().path,
        SelectedDecodePath::FfmpegPreviewArtifact
    );
    assert!(selection.diagnostics.last().unwrap().available);
}

fn fake_runtime_config() -> RuntimeConfig {
    let directory = PathBuf::from("/runtime/bin");
    RuntimeConfig {
        ffmpeg: DiscoveredBinary {
            kind: BinaryKind::Ffmpeg,
            path: directory.join("ffmpeg"),
            source: DiscoverySource::Bundled {
                directory: directory.clone(),
            },
            version: "ffmpeg version test-build".to_owned(),
        },
        ffprobe: DiscoveredBinary {
            kind: BinaryKind::Ffprobe,
            path: directory.join("ffprobe"),
            source: DiscoverySource::Bundled { directory },
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
    fn ready() -> Self {
        Self::default()
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
        "fake-fallback-ladder-executor"
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

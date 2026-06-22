use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use draft_model::{
    BUNDLED_TEXT_FONT_FAMILY, BUNDLED_TEXT_FONT_LICENSE_SPDX, BUNDLED_TEXT_FONT_REF,
    bundled_font_registry, bundled_text_font_path, repository_root_from_manifest,
    validate_bundled_font_registry,
};
use serde::{Deserialize, Serialize};

use crate::{
    BinaryKind, DiscoveredBinary, FfmpegExecutor, MAX_STDERR_SUMMARY_BYTES, MediaIoFallbackReason,
    RuntimeConfig, RuntimeDeviceId, SelectedDecodePath, TextureBackend, VideoPixelFormat,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeCapabilityStatus {
    Ready,
    Warning,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeBinaryCapability {
    pub kind: BinaryKind,
    pub path: PathBuf,
    pub source: String,
    pub version: String,
    pub configure_summary: Option<String>,
    pub status: RuntimeCapabilityStatus,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeFeatureCapability {
    pub name: String,
    pub available: bool,
    pub status: RuntimeCapabilityStatus,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeFontCapability {
    pub env_text_font_path: Option<PathBuf>,
    pub available_font_paths: Vec<PathBuf>,
    pub bundled_font_ref: Option<String>,
    pub bundled_font_family: Option<String>,
    pub bundled_font_path: Option<PathBuf>,
    pub bundled_font_license: Option<String>,
    pub status: RuntimeCapabilityStatus,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLicensePosture {
    pub external_runtime: bool,
    pub redistributable_build: bool,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilityReport {
    pub status: RuntimeCapabilityStatus,
    pub executor_name: String,
    pub ffmpeg: RuntimeBinaryCapability,
    pub ffprobe: RuntimeBinaryCapability,
    pub h264_encoder: RuntimeFeatureCapability,
    pub aac_encoder: RuntimeFeatureCapability,
    pub ass_filter: RuntimeFeatureCapability,
    pub subtitles_filter: RuntimeFeatureCapability,
    pub font_readiness: RuntimeFontCapability,
    pub license_posture: RuntimeLicensePosture,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilities {
    pub ffmpeg: RuntimeCapabilityReport,
    pub media_io: RuntimeMediaIoCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeMediaIoCapabilities {
    pub windows: WindowsMediaIoCapabilities,
    pub macos: MacosMediaIoCapabilities,
    pub codecs: Vec<CodecCapability>,
    pub pixel_formats: Vec<PixelFormatCapability>,
    pub texture_interop: TextureInteropCapability,
    pub fallback_ladder: FallbackLadderCapability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowsMediaIoCapabilities {
    pub status: RuntimeCapabilityStatus,
    pub media_foundation: RuntimeFeatureCapability,
    pub dxva: RuntimeFeatureCapability,
    pub d3d_texture_interop: RuntimeFeatureCapability,
    pub fallback_reason: Option<MediaIoFallbackReason>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacosMediaIoCapabilities {
    pub status: RuntimeCapabilityStatus,
    pub av_foundation: RuntimeFeatureCapability,
    pub video_toolbox: RuntimeFeatureCapability,
    pub core_video: RuntimeFeatureCapability,
    pub metal_texture_interop: RuntimeFeatureCapability,
    pub fallback_reason: Option<MediaIoFallbackReason>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodecCapability {
    pub codec: String,
    pub containers: Vec<String>,
    pub first_native_hardware_decode_target: bool,
    pub status: RuntimeCapabilityStatus,
    pub fallback_reason: Option<MediaIoFallbackReason>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PixelFormatCapability {
    pub pixel_format: VideoPixelFormat,
    pub status: RuntimeCapabilityStatus,
    pub fallback_reason: Option<MediaIoFallbackReason>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureInteropCapability {
    pub status: RuntimeCapabilityStatus,
    pub backend: Option<TextureBackend>,
    pub device_id: Option<RuntimeDeviceId>,
    pub compatible_with_preview_device: bool,
    pub fallback_reason: Option<MediaIoFallbackReason>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackLadderCapability {
    pub paths: Vec<FallbackDecodePathCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackDecodePathCapability {
    pub path: SelectedDecodePath,
    pub status: RuntimeCapabilityStatus,
    pub fallback_reason: Option<MediaIoFallbackReason>,
    pub diagnostic: Option<String>,
}

pub fn probe_runtime_capabilities(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
) -> RuntimeCapabilityReport {
    let ffmpeg = binary_capability(executor, &runtime.ffmpeg);
    let ffprobe = binary_capability(executor, &runtime.ffprobe);

    let encoders_probe = run_ffmpeg_probe(executor, runtime, &["-hide_banner", "-encoders"]);
    let filters_probe = run_ffmpeg_probe(executor, runtime, &["-hide_banner", "-filters"]);

    let h264_encoder = feature_capability(
        "H.264",
        probe_output_has_feature(encoders_probe.as_deref().unwrap_or_default(), "libx264")
            || probe_output_has_feature(encoders_probe.as_deref().unwrap_or_default(), "h264"),
        encoders_probe.as_ref().err(),
        "当前 FFmpeg 不支持 H.264 导出，请更换可用构建。",
    );
    let aac_encoder = feature_capability(
        "AAC",
        probe_output_has_feature(encoders_probe.as_deref().unwrap_or_default(), "aac"),
        encoders_probe.as_ref().err(),
        "当前 FFmpeg 不支持 AAC 导出，请更换可用构建。",
    );
    let ass_filter = feature_capability(
        "ASS",
        probe_output_has_feature(filters_probe.as_deref().unwrap_or_default(), "ass"),
        filters_probe.as_ref().err(),
        "当前 FFmpeg 缺少 ASS 字幕滤镜，文字预览和导出可能受限。",
    );
    let subtitles_filter = feature_capability(
        "subtitles",
        probe_output_has_feature(filters_probe.as_deref().unwrap_or_default(), "subtitles"),
        filters_probe.as_ref().err(),
        "当前 FFmpeg 缺少 subtitles 字幕滤镜，文字预览和导出可能受限。",
    );
    let font_readiness = font_capability();

    let mut diagnostics = Vec::new();
    collect_diagnostic(&mut diagnostics, &ffmpeg.diagnostic);
    collect_diagnostic(&mut diagnostics, &ffprobe.diagnostic);
    collect_diagnostic(&mut diagnostics, &h264_encoder.diagnostic);
    collect_diagnostic(&mut diagnostics, &aac_encoder.diagnostic);
    collect_diagnostic(&mut diagnostics, &ass_filter.diagnostic);
    collect_diagnostic(&mut diagnostics, &subtitles_filter.diagnostic);
    collect_diagnostic(&mut diagnostics, &font_readiness.diagnostic);

    let status = aggregate_status([
        ffmpeg.status,
        ffprobe.status,
        h264_encoder.status,
        aac_encoder.status,
        ass_filter.status,
        subtitles_filter.status,
        font_readiness.status,
    ]);

    RuntimeCapabilityReport {
        status,
        executor_name: executor.executor_name().to_owned(),
        ffmpeg,
        ffprobe,
        h264_encoder,
        aac_encoder,
        ass_filter,
        subtitles_filter,
        font_readiness,
        license_posture: license_posture(runtime),
        diagnostics,
    }
}

fn binary_capability(
    executor: &impl FfmpegExecutor,
    binary: &DiscoveredBinary,
) -> RuntimeBinaryCapability {
    let version_probe = executor.run_version_probe(&binary.path);
    let configure_summary = version_probe.as_ref().ok().and_then(|output| {
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        combined
            .lines()
            .find(|line| line.trim_start().starts_with("configuration:"))
            .map(|line| bound_text(line.trim()))
    });

    let diagnostic = match version_probe {
        Ok(output) if output.status.success() => None,
        Ok(output) => Some(format!(
            "{} 版本探测失败：stdout=`{}` stderr=`{}`",
            binary.kind.binary_name(),
            bounded_summary(&output.stdout),
            bounded_summary(&output.stderr)
        )),
        Err(error) => Some(format!(
            "{} 版本探测启动失败：{error}",
            binary.kind.binary_name()
        )),
    };

    RuntimeBinaryCapability {
        kind: binary.kind,
        path: binary.path.clone(),
        source: source_label(binary),
        version: binary.version.clone(),
        configure_summary,
        status: if diagnostic.is_some() {
            RuntimeCapabilityStatus::Warning
        } else {
            RuntimeCapabilityStatus::Ready
        },
        diagnostic,
    }
}

fn source_label(binary: &DiscoveredBinary) -> String {
    match &binary.source {
        crate::DiscoverySource::Bundled { .. } => "bundled".to_owned(),
    }
}

fn license_posture(_runtime: &RuntimeConfig) -> RuntimeLicensePosture {
    RuntimeLicensePosture {
        external_runtime: false,
        redistributable_build: false,
        source: "bundledRuntime".to_owned(),
        message:
            "当前使用打包内置 FFmpeg/ffprobe；工程已记录运行时清单，但公开再发行仍需完成法律审查。"
                .to_owned(),
    }
}

fn run_ffmpeg_probe(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    args: &[&str],
) -> Result<String, String> {
    let args = args.iter().map(OsString::from).collect::<Vec<_>>();
    let output = executor
        .run(&runtime.ffmpeg.path, &args)
        .map_err(|error| format!("FFmpeg 能力探测启动失败：{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "FFmpeg 能力探测失败：stdout=`{}` stderr=`{}`",
            bounded_summary(&output.stdout),
            bounded_summary(&output.stderr)
        ));
    }

    Ok(format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn feature_capability(
    name: &str,
    available: bool,
    probe_error: Option<&String>,
    missing_message: &str,
) -> RuntimeFeatureCapability {
    let diagnostic = if let Some(error) = probe_error {
        Some(error.clone())
    } else if available {
        None
    } else {
        Some(missing_message.to_owned())
    };

    RuntimeFeatureCapability {
        name: name.to_owned(),
        available,
        status: if available {
            RuntimeCapabilityStatus::Ready
        } else {
            RuntimeCapabilityStatus::Warning
        },
        diagnostic,
    }
}

fn font_capability() -> RuntimeFontCapability {
    let env_text_font_path = env::var_os("VE_TEXT_FONT_PATH").map(PathBuf::from);
    let bundled_font_path = bundled_text_font_path();
    let bundled_validation = validate_bundled_font_registry(&repository_root_from_manifest())
        .map_err(|error| error.to_string());
    let available_font_paths = resolved_text_font_paths()
        .into_iter()
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    let diagnostic = if let Err(error) = &bundled_validation {
        Some(format!("内置字体注册表不可用：{error}"))
    } else if available_font_paths.is_empty() {
        Some("字体环境未完全就绪，文字渲染可能与导出结果不一致。".to_owned())
    } else {
        None
    };

    RuntimeFontCapability {
        env_text_font_path,
        available_font_paths,
        bundled_font_ref: bundled_validation
            .as_ref()
            .ok()
            .map(|_| BUNDLED_TEXT_FONT_REF.to_owned()),
        bundled_font_family: bundled_validation
            .as_ref()
            .ok()
            .map(|_| BUNDLED_TEXT_FONT_FAMILY.to_owned()),
        bundled_font_path: bundled_validation.as_ref().ok().map(|_| bundled_font_path),
        bundled_font_license: bundled_validation
            .as_ref()
            .ok()
            .map(|_| BUNDLED_TEXT_FONT_LICENSE_SPDX.to_owned()),
        status: if diagnostic.is_some() {
            RuntimeCapabilityStatus::Warning
        } else {
            RuntimeCapabilityStatus::Ready
        },
        diagnostic,
    }
}

fn resolved_text_font_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = env::var_os("VE_TEXT_FONT_PATH").map(PathBuf::from) {
        paths.push(path);
    }
    let repository_root = repository_root_from_manifest();
    paths.extend(
        bundled_font_registry()
            .iter()
            .map(|font| font.font_path(&repository_root)),
    );
    paths.extend([
        PathBuf::from("/System/Library/Fonts/PingFang.ttc"),
        PathBuf::from("/System/Library/Fonts/Supplemental/Arial Unicode.ttf"),
        PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"),
        PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
    ]);
    paths
}

fn probe_output_has_feature(output: &str, feature: &str) -> bool {
    output.lines().any(|line| {
        line.split_whitespace()
            .any(|field| field == feature || field == format!("{feature},"))
    })
}

fn aggregate_status(
    statuses: impl IntoIterator<Item = RuntimeCapabilityStatus>,
) -> RuntimeCapabilityStatus {
    let mut has_warning = false;
    for status in statuses {
        match status {
            RuntimeCapabilityStatus::Unavailable => return RuntimeCapabilityStatus::Unavailable,
            RuntimeCapabilityStatus::Warning => has_warning = true,
            RuntimeCapabilityStatus::Ready => {}
        }
    }
    if has_warning {
        RuntimeCapabilityStatus::Warning
    } else {
        RuntimeCapabilityStatus::Ready
    }
}

fn collect_diagnostic(diagnostics: &mut Vec<String>, diagnostic: &Option<String>) {
    if let Some(diagnostic) = diagnostic {
        diagnostics.push(diagnostic.clone());
    }
}

fn bounded_summary(bytes: &[u8]) -> String {
    bound_text(String::from_utf8_lossy(bytes).trim())
}

fn bound_text(value: &str) -> String {
    let mut summary = String::new();
    for character in value.chars() {
        if summary.len() + character.len_utf8() > MAX_STDERR_SUMMARY_BYTES {
            break;
        }
        summary.push(character);
    }
    summary
}

#[allow(dead_code)]
fn _assert_path_send_sync(_: &Path) {}

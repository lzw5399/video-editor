use draft_model::{
    RuntimeBinaryCapability as CommandRuntimeBinaryCapability, RuntimeBinaryKind,
    RuntimeCapabilityReport as CommandRuntimeCapabilityReport,
    RuntimeCapabilityStatus as CommandRuntimeCapabilityStatus,
    RuntimeFeatureCapability as CommandRuntimeFeatureCapability,
    RuntimeFontCapability as CommandRuntimeFontCapability,
    RuntimeLicensePosture as CommandRuntimeLicensePosture,
};
use media_runtime::{
    BinaryKind, DiscoveryError, RuntimeBinaryCapability, RuntimeCapabilityReport,
    RuntimeCapabilityStatus, RuntimeFeatureCapability, RuntimeFontCapability,
    RuntimeLicensePosture, discover_runtime_config, probe_runtime_capabilities,
};
use media_runtime_desktop::DesktopFfmpegExecutor;

pub fn probe_runtime_capabilities_command() -> Result<CommandRuntimeCapabilityReport, DiscoveryError>
{
    let runtime = discover_runtime_config()?;
    let executor = DesktopFfmpegExecutor::default();
    Ok(command_runtime_capability_report(
        probe_runtime_capabilities(&executor, &runtime),
    ))
}

fn command_runtime_capability_report(
    report: RuntimeCapabilityReport,
) -> CommandRuntimeCapabilityReport {
    CommandRuntimeCapabilityReport {
        status: command_status(report.status),
        executor_name: report.executor_name,
        ffmpeg: command_binary_capability(report.ffmpeg),
        ffprobe: command_binary_capability(report.ffprobe),
        h264_encoder: command_feature_capability(report.h264_encoder),
        aac_encoder: command_feature_capability(report.aac_encoder),
        ass_filter: command_feature_capability(report.ass_filter),
        subtitles_filter: command_feature_capability(report.subtitles_filter),
        font_readiness: command_font_capability(report.font_readiness),
        license_posture: command_license_posture(report.license_posture),
        diagnostics: report.diagnostics,
    }
}

fn command_status(status: RuntimeCapabilityStatus) -> CommandRuntimeCapabilityStatus {
    match status {
        RuntimeCapabilityStatus::Ready => CommandRuntimeCapabilityStatus::Ready,
        RuntimeCapabilityStatus::Warning => CommandRuntimeCapabilityStatus::Warning,
        RuntimeCapabilityStatus::Unavailable => CommandRuntimeCapabilityStatus::Unavailable,
    }
}

fn command_binary_kind(kind: BinaryKind) -> RuntimeBinaryKind {
    match kind {
        BinaryKind::Ffmpeg => RuntimeBinaryKind::Ffmpeg,
        BinaryKind::Ffprobe => RuntimeBinaryKind::Ffprobe,
    }
}

fn command_binary_capability(
    capability: RuntimeBinaryCapability,
) -> CommandRuntimeBinaryCapability {
    CommandRuntimeBinaryCapability {
        kind: command_binary_kind(capability.kind),
        path: capability.path.display().to_string(),
        source: capability.source,
        version: capability.version,
        configure_summary: capability.configure_summary,
        status: command_status(capability.status),
        diagnostic: capability.diagnostic,
    }
}

fn command_feature_capability(
    capability: RuntimeFeatureCapability,
) -> CommandRuntimeFeatureCapability {
    CommandRuntimeFeatureCapability {
        name: capability.name,
        available: capability.available,
        status: command_status(capability.status),
        diagnostic: capability.diagnostic,
    }
}

fn command_font_capability(capability: RuntimeFontCapability) -> CommandRuntimeFontCapability {
    CommandRuntimeFontCapability {
        env_text_font_path: capability
            .env_text_font_path
            .map(|path| path.display().to_string()),
        available_font_paths: capability
            .available_font_paths
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
        status: command_status(capability.status),
        diagnostic: capability.diagnostic,
    }
}

fn command_license_posture(posture: RuntimeLicensePosture) -> CommandRuntimeLicensePosture {
    CommandRuntimeLicensePosture {
        external_runtime: posture.external_runtime,
        redistributable_build: posture.redistributable_build,
        source: posture.source,
        message: posture.message,
    }
}

use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Output;

use artifact_store::paths::derived_root_path;
use draft_model::{Draft, MaterialId, Microseconds, TargetTimerange};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use ffmpeg_compiler::{CompileContext, CompilerCapabilities, FfmpegJob, compile_ffmpeg_job};
use media_runtime::{FfmpegExecutor, MAX_STDERR_SUMMARY_BYTES};
use render_graph::{
    OutputDimensions, RenderGraphPlan, RenderGraphSnapshot, RenderOutputProfile,
    build_render_graph, deterministic_fingerprint,
};
use serde::{Deserialize, Serialize};

use crate::cache::{PreviewArtifact, PreviewCacheEntry, PreviewCacheKey, PreviewCacheProfile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewServiceConfig {
    pub cache_root: PathBuf,
    pub project_artifact_root: Option<PathBuf>,
    pub ffmpeg_path: PathBuf,
    pub compiler_capabilities: CompilerCapabilities,
    pub preview_frame_max_dimensions: OutputDimensions,
    pub preview_segment_max_dimensions: OutputDimensions,
}

impl PreviewServiceConfig {
    pub fn new(cache_root: impl Into<PathBuf>, ffmpeg_path: impl Into<PathBuf>) -> Self {
        Self {
            cache_root: cache_root.into(),
            project_artifact_root: None,
            ffmpeg_path: ffmpeg_path.into(),
            compiler_capabilities: CompilerCapabilities::all_available_for_tests(),
            preview_frame_max_dimensions: OutputDimensions::new(960, 540),
            preview_segment_max_dimensions: OutputDimensions::new(960, 540),
        }
    }

    pub fn with_compiler_capabilities(mut self, capabilities: CompilerCapabilities) -> Self {
        self.compiler_capabilities = capabilities;
        self
    }

    pub fn with_project_artifact_root(mut self, bundle_path: impl AsRef<Path>) -> Self {
        self.project_artifact_root = Some(project_preview_artifact_root(bundle_path));
        self
    }

    pub fn artifact_root(&self) -> &Path {
        self.project_artifact_root
            .as_deref()
            .unwrap_or(&self.cache_root)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewFrameRequest {
    pub draft: Draft,
    pub target_time: Microseconds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewSegmentRequest {
    pub draft: Draft,
    pub target_timerange: TargetTimerange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewFrameResponse {
    pub artifact: PreviewArtifact,
    pub cache_entry: PreviewCacheEntry,
    pub ffmpeg_job: FfmpegJob,
    pub from_cache: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewSegmentResponse {
    pub artifact: PreviewArtifact,
    pub cache_entry: PreviewCacheEntry,
    pub ffmpeg_job: FfmpegJob,
    pub from_cache: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewServiceError {
    pub kind: PreviewServiceErrorKind,
    pub message: String,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
}

impl PreviewServiceError {
    pub(crate) fn new(kind: PreviewServiceErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            stdout_summary: None,
            stderr_summary: None,
        }
    }

    fn with_output(mut self, output: &Output) -> Self {
        self.stdout_summary = bounded_summary(&output.stdout);
        self.stderr_summary = bounded_summary(&output.stderr);
        self
    }
}

impl fmt::Display for PreviewServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for PreviewServiceError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewServiceErrorKind {
    EngineFailed,
    RenderGraphFailed,
    CompileFailed,
    IoFailed,
    RuntimeUnavailable,
    RuntimeFailed,
}

pub fn request_preview_frame(
    executor: &impl FfmpegExecutor,
    config: &PreviewServiceConfig,
    request: &PreviewFrameRequest,
) -> Result<PreviewFrameResponse, PreviewServiceError> {
    let target_timerange = TargetTimerange::new(request.target_time, Microseconds::new(33_333));
    let prepared = prepare_preview(
        &request.draft,
        target_timerange,
        PreviewCacheProfile::FramePng,
        config,
    )?;

    let from_cache = artifact_exists(&prepared.artifact.path);
    if !from_cache {
        write_sidecars_and_run(executor, config, &prepared.ffmpeg_job)?;
    }

    Ok(PreviewFrameResponse {
        artifact: prepared.artifact,
        cache_entry: prepared.cache_entry,
        ffmpeg_job: prepared.ffmpeg_job,
        from_cache,
    })
}

pub fn request_preview_segment(
    executor: &impl FfmpegExecutor,
    config: &PreviewServiceConfig,
    request: &PreviewSegmentRequest,
) -> Result<PreviewSegmentResponse, PreviewServiceError> {
    let prepared = prepare_preview(
        &request.draft,
        request.target_timerange.clone(),
        PreviewCacheProfile::SegmentMp4,
        config,
    )?;

    let from_cache = artifact_exists(&prepared.artifact.path);
    if !from_cache {
        write_sidecars_and_run(executor, config, &prepared.ffmpeg_job)?;
    }

    Ok(PreviewSegmentResponse {
        artifact: prepared.artifact,
        cache_entry: prepared.cache_entry,
        ffmpeg_job: prepared.ffmpeg_job,
        from_cache,
    })
}

struct PreparedPreview {
    artifact: PreviewArtifact,
    cache_entry: PreviewCacheEntry,
    ffmpeg_job: FfmpegJob,
}

fn prepare_preview(
    draft: &Draft,
    target_timerange: TargetTimerange,
    profile: PreviewCacheProfile,
    config: &PreviewServiceConfig,
) -> Result<PreparedPreview, PreviewServiceError> {
    let engine_profile = EngineProfile::from_draft_canvas(draft).map_err(|error| {
        PreviewServiceError::new(
            PreviewServiceErrorKind::EngineFailed,
            format!("preview engine profile resolution failed: {error}"),
        )
    })?;
    let normalized = normalize_draft(draft, &engine_profile).map_err(|error| {
        PreviewServiceError::new(
            PreviewServiceErrorKind::EngineFailed,
            format!("preview engine normalization failed: {error}"),
        )
    })?;
    let range = resolve_render_range(&normalized, target_timerange.clone()).map_err(|error| {
        PreviewServiceError::new(
            PreviewServiceErrorKind::EngineFailed,
            format!("preview range resolution failed: {error}"),
        )
    })?;
    let graph = build_render_graph(&normalized, &range).map_err(|error| {
        PreviewServiceError::new(
            PreviewServiceErrorKind::RenderGraphFailed,
            format!("preview render graph failed: {error}"),
        )
    })?;

    let output_profile = match profile {
        PreviewCacheProfile::FramePng => RenderOutputProfile::preview_frame_png(
            preview_output_dimensions(&normalized.profile, config.preview_frame_max_dimensions),
            range.frame_rate.clone(),
            target_timerange,
        ),
        PreviewCacheProfile::SegmentMp4 => RenderOutputProfile::preview_segment_mp4(
            preview_output_dimensions(&normalized.profile, config.preview_segment_max_dimensions),
            range.frame_rate.clone(),
            target_timerange,
        ),
    };
    let runtime_capability_fingerprint = deterministic_fingerprint(
        "preview-runtime-capabilities",
        &config.compiler_capabilities,
    );
    let snapshot =
        RenderGraphSnapshot::from_graph(&graph, &output_profile, &runtime_capability_fingerprint);
    let material_dependencies = graph
        .materials
        .iter()
        .map(|material| material.material_id.clone())
        .collect::<Vec<_>>();
    let key = preview_cache_key(
        profile,
        snapshot.target_timerange.clone(),
        &snapshot,
        material_dependencies,
    );
    let artifact_path = artifact_path(config.artifact_root(), &key);
    let artifact = PreviewArtifact {
        profile,
        path: path_to_string(&artifact_path),
        mime_type: profile.mime_type().to_owned(),
    };
    let plan = RenderGraphPlan::new(graph, output_profile).map_err(|error| {
        PreviewServiceError::new(
            PreviewServiceErrorKind::RenderGraphFailed,
            format!("preview output profile failed: {error}"),
        )
    })?;
    let sidecar_dir = config.artifact_root().join("sidecars");
    let compile_context = CompileContext::new(&artifact_path, &sidecar_dir)
        .with_capabilities(config.compiler_capabilities.clone());
    let ffmpeg_job = compile_ffmpeg_job(&plan, &compile_context).map_err(|error| {
        PreviewServiceError::new(
            PreviewServiceErrorKind::CompileFailed,
            format!("preview FFmpeg compile failed: {error}"),
        )
    })?;
    let cache_entry = PreviewCacheEntry {
        key,
        artifact: artifact.clone(),
    };

    Ok(PreparedPreview {
        artifact,
        cache_entry,
        ffmpeg_job,
    })
}

fn preview_output_dimensions(
    profile: &EngineProfile,
    max_dimensions: OutputDimensions,
) -> OutputDimensions {
    fit_canvas_dimensions(profile.canvas_width, profile.canvas_height, max_dimensions)
}

fn fit_canvas_dimensions(
    canvas_width: u32,
    canvas_height: u32,
    max_dimensions: OutputDimensions,
) -> OutputDimensions {
    if canvas_width == 0
        || canvas_height == 0
        || max_dimensions.width == 0
        || max_dimensions.height == 0
    {
        return max_dimensions;
    }

    let width_limited = u128::from(max_dimensions.width) * u128::from(canvas_height)
        <= u128::from(max_dimensions.height) * u128::from(canvas_width);
    let (scale_numerator, scale_denominator) = if width_limited {
        (max_dimensions.width, canvas_width)
    } else {
        (max_dimensions.height, canvas_height)
    };

    if scale_numerator >= scale_denominator {
        return OutputDimensions::new(
            stable_preview_dimension(canvas_width, max_dimensions.width),
            stable_preview_dimension(canvas_height, max_dimensions.height),
        );
    }

    let width = round_scaled_dimension(canvas_width, scale_numerator, scale_denominator)
        .min(max_dimensions.width);
    let height = round_scaled_dimension(canvas_height, scale_numerator, scale_denominator)
        .min(max_dimensions.height);

    OutputDimensions::new(
        stable_preview_dimension(width, max_dimensions.width),
        stable_preview_dimension(height, max_dimensions.height),
    )
}

fn round_scaled_dimension(value: u32, numerator: u32, denominator: u32) -> u32 {
    let scaled = (u128::from(value) * u128::from(numerator))
        .saturating_add(u128::from(denominator) / 2)
        / u128::from(denominator);
    u32::try_from(scaled).unwrap_or(u32::MAX).max(1)
}

fn stable_preview_dimension(value: u32, max: u32) -> u32 {
    let max = max.max(1);
    let value = value.max(1).min(max);
    if value <= 2 || value % 2 == 0 {
        return value;
    }
    if value < max {
        value + 1
    } else {
        value.saturating_sub(1).max(1)
    }
}

fn write_sidecars_and_run(
    executor: &impl FfmpegExecutor,
    config: &PreviewServiceConfig,
    job: &FfmpegJob,
) -> Result<(), PreviewServiceError> {
    fs::create_dir_all(config.artifact_root()).map_err(io_error)?;
    for sidecar in &job.sidecars {
        let path = Path::new(&sidecar.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        fs::write(path, sidecar.contents.as_bytes()).map_err(io_error)?;
    }

    if !executor.can_execute(&config.ffmpeg_path) {
        return Err(PreviewServiceError::new(
            PreviewServiceErrorKind::RuntimeUnavailable,
            format!(
                "{} cannot execute FFmpeg at {}",
                executor.executor_name(),
                config.ffmpeg_path.display()
            ),
        ));
    }

    let output = executor
        .run(&config.ffmpeg_path, &job.args)
        .map_err(|error| {
            PreviewServiceError::new(
                PreviewServiceErrorKind::RuntimeFailed,
                format!("preview FFmpeg execution failed: {error}"),
            )
        })?;
    if !output.status.success() {
        return Err(PreviewServiceError::new(
            PreviewServiceErrorKind::RuntimeFailed,
            "preview FFmpeg execution returned a non-zero exit status",
        )
        .with_output(&output));
    }
    Ok(())
}

fn preview_cache_key(
    profile: PreviewCacheProfile,
    target_timerange: TargetTimerange,
    snapshot: &RenderGraphSnapshot,
    material_dependencies: Vec<MaterialId>,
) -> PreviewCacheKey {
    PreviewCacheKey::from_node_fingerprints(
        profile,
        target_timerange,
        &snapshot.node_fingerprints,
        material_dependencies,
    )
}

fn artifact_path(cache_root: &Path, key: &PreviewCacheKey) -> PathBuf {
    cache_root.join(format!("{}.{}", key.key_id, key.profile.extension()))
}

fn project_preview_artifact_root(bundle_path: impl AsRef<Path>) -> PathBuf {
    derived_root_path(bundle_path.as_ref())
        .join("blobs")
        .join("preview")
}

fn artifact_exists(path: &str) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.len() > 0)
        .unwrap_or(false)
}

fn io_error(error: io::Error) -> PreviewServiceError {
    PreviewServiceError::new(
        PreviewServiceErrorKind::IoFailed,
        format!("preview cache IO failed: {error}"),
    )
}

fn bounded_summary(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let limit = MAX_STDERR_SUMMARY_BYTES.min(bytes.len());
    Some(String::from_utf8_lossy(&bytes[..limit]).into_owned())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[allow(dead_code)]
fn _args_as_strings(args: &[OsString]) -> Vec<String> {
    args.iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

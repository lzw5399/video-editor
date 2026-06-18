use std::path::PathBuf;
use std::sync::Mutex;

use draft_model::{Draft, Microseconds, RationalFrameRate};
use ffmpeg_compiler::{CompilerCapabilities, FfmpegJob};
use media_runtime::FfmpegExecutor;
use realtime_preview_runtime::{
    PlaybackGeneration, PlaybackRate, PreviewCancellationToken, PreviewFrameProvider,
    PreviewGpuBackend, PreviewRequestMode, RealtimePreviewCapabilityClassifier,
    RealtimePreviewDiagnostic, RealtimePreviewDiagnosticDomain, RealtimePreviewFallbackReason,
    RealtimePreviewFrameRequest, RealtimePreviewFrameResult, RealtimePreviewGraphInput,
    RealtimePreviewGraphSupport, RealtimePreviewRuntime, RealtimePreviewSessionConfig,
    gpu::{
        RealtimePreviewCompositor, RealtimePreviewGpuBackend, RealtimePreviewGpuDevice,
        RealtimePreviewGpuDeviceDescriptor, RealtimePreviewTargetFormat,
        RealtimePreviewTextureCache,
    },
    prepare_realtime_preview_graph,
};
use serde::{Deserialize, Serialize};

use crate::{
    PreviewArtifact, PreviewCacheEntry, PreviewFrameRequest, PreviewServiceConfig,
    PreviewServiceError, PreviewServiceErrorKind, request_preview_frame,
};

#[derive(Debug)]
pub struct RealtimePreviewServiceConfig {
    artifact_config: PreviewServiceConfig,
    runtime_backend_available: bool,
    surface_available: bool,
    gpu_text_parity: bool,
    preferred_backend: PreviewGpuBackend,
    gpu_backend: RealtimePreviewGpuBackend,
    runtime: Mutex<RealtimePreviewRuntime>,
    session_id: realtime_preview_runtime::PreviewSessionId,
}

impl RealtimePreviewServiceConfig {
    pub fn new(cache_root: impl Into<PathBuf>, ffmpeg_path: impl Into<PathBuf>) -> Self {
        let artifact_config = PreviewServiceConfig::new(cache_root, ffmpeg_path);
        let mut runtime = RealtimePreviewRuntime::new();
        let session_id = runtime
            .create_session(default_session_config(PreviewGpuBackend::OffscreenOnly))
            .expect("default realtime preview session config is valid");
        Self {
            artifact_config,
            runtime_backend_available: true,
            surface_available: true,
            gpu_text_parity: true,
            preferred_backend: PreviewGpuBackend::OffscreenOnly,
            gpu_backend: RealtimePreviewGpuBackend::OffscreenOnly,
            runtime: Mutex::new(runtime),
            session_id,
        }
    }

    pub fn with_mock_realtime_backend(mut self) -> Self {
        self.preferred_backend = PreviewGpuBackend::Mock;
        self.gpu_backend = RealtimePreviewGpuBackend::Mock;
        self.reset_runtime_session();
        self
    }

    pub fn with_runtime_backend_available(mut self, available: bool) -> Self {
        self.runtime_backend_available = available;
        self
    }

    pub fn with_surface_available(mut self, available: bool) -> Self {
        self.surface_available = available;
        self
    }

    pub fn with_gpu_text_parity(mut self, enabled: bool) -> Self {
        self.gpu_text_parity = enabled;
        self
    }

    pub fn with_compiler_capabilities(mut self, capabilities: CompilerCapabilities) -> Self {
        self.artifact_config = self
            .artifact_config
            .with_compiler_capabilities(capabilities);
        self
    }

    pub fn artifact_config(&self) -> &PreviewServiceConfig {
        &self.artifact_config
    }

    fn reset_runtime_session(&mut self) {
        let mut runtime = RealtimePreviewRuntime::new();
        self.session_id = runtime
            .create_session(default_session_config(self.preferred_backend))
            .expect("realtime preview session config is valid");
        self.runtime = Mutex::new(runtime);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewFrameServiceRequest {
    pub draft: Draft,
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub mode: PreviewRequestMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_token: Option<PreviewCancellationToken>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewServiceFrameResponse {
    pub realtime: RealtimePreviewFrameResult,
    pub fallback_decision: RealtimePreviewFallbackDecision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<PreviewArtifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_entry: Option<PreviewCacheEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ffmpeg_job: Option<FfmpegJob>,
    pub from_cache: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewFallbackDecision {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<RealtimePreviewFallbackReason>,
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub cache_hit: bool,
    pub fallback_counted: bool,
    pub canceled: bool,
    pub stale_rejected: bool,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
}

pub fn request_realtime_preview_frame(
    executor: &impl FfmpegExecutor,
    config: &RealtimePreviewServiceConfig,
    request: &RealtimePreviewFrameServiceRequest,
    frame_provider: &mut impl PreviewFrameProvider,
) -> Result<RealtimePreviewServiceFrameResponse, PreviewServiceError> {
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: request.draft.clone(),
        target_time: request.target_time,
        preview_dimensions: config.artifact_config.preview_frame_max_dimensions,
    })
    .map_err(|error| {
        PreviewServiceError::new(
            PreviewServiceErrorKind::EngineFailed,
            format!("realtime preview graph preparation failed: {error}"),
        )
    })?;

    let classifier = RealtimePreviewCapabilityClassifier {
        runtime_backend_available: config.runtime_backend_available,
        surface_available: config.surface_available,
        gpu_text_parity: config.gpu_text_parity,
    };
    let capability = classifier.classify(&prepared.graph);
    let mut diagnostics = prepared.diagnostics;
    diagnostics.extend(capability.diagnostics.clone());

    let mut fallback_reason = preflight_fallback_reason(config, &capability.diagnostics);
    if fallback_reason.is_none() && capability.support == RealtimePreviewGraphSupport::Unsupported {
        fallback_reason = Some(classified_unsupported_reason(&capability.diagnostics));
    }

    if fallback_reason.is_none() {
        let render = render_realtime_frame(config, &prepared.graph, frame_provider)?;
        diagnostics.extend(render.diagnostics);
        if render.support == RealtimePreviewGraphSupport::Unsupported {
            fallback_reason = Some(classified_unsupported_reason(&diagnostics));
        }
    }

    if let Some(reason) = fallback_reason {
        return request_artifact_fallback(executor, config, request, diagnostics, reason);
    }

    let realtime = runtime_frame(config, request, None, false)?;
    Ok(RealtimePreviewServiceFrameResponse {
        fallback_decision: decision(request, None, false, &realtime, diagnostics),
        realtime,
        artifact: None,
        cache_entry: None,
        ffmpeg_job: None,
        from_cache: false,
    })
}

fn render_realtime_frame(
    config: &RealtimePreviewServiceConfig,
    graph: &render_graph::RenderGraph,
    frame_provider: &mut impl PreviewFrameProvider,
) -> Result<realtime_preview_runtime::gpu::RealtimePreviewCompositorOutput, PreviewServiceError> {
    let device = RealtimePreviewGpuDevice::bootstrap(RealtimePreviewGpuDeviceDescriptor {
        backend: config.gpu_backend,
        label: Some("preview-service-realtime-backend".to_owned()),
    })
    .map_err(|error| {
        PreviewServiceError::new(
            PreviewServiceErrorKind::RuntimeUnavailable,
            format!("realtime preview GPU backend unavailable: {error}"),
        )
    })?;
    let target = device
        .create_offscreen_target(
            graph.canvas.width,
            graph.canvas.height,
            1000,
            RealtimePreviewTargetFormat::Rgba8UnormSrgb,
        )
        .map_err(|error| {
            PreviewServiceError::new(
                PreviewServiceErrorKind::RuntimeFailed,
                format!("realtime preview target creation failed: {error}"),
            )
        })?;
    let classifier = RealtimePreviewCapabilityClassifier {
        runtime_backend_available: config.runtime_backend_available,
        surface_available: true,
        gpu_text_parity: config.gpu_text_parity,
    };
    let mut compositor = RealtimePreviewCompositor::new(device, classifier);
    let mut texture_cache = RealtimePreviewTextureCache::new();
    compositor
        .render_offscreen(graph, &target, frame_provider, &mut texture_cache)
        .map_err(|error| {
            PreviewServiceError::new(
                PreviewServiceErrorKind::RuntimeFailed,
                format!("realtime preview render failed: {error}"),
            )
        })
}

fn request_artifact_fallback(
    executor: &impl FfmpegExecutor,
    config: &RealtimePreviewServiceConfig,
    request: &RealtimePreviewFrameServiceRequest,
    diagnostics: Vec<RealtimePreviewDiagnostic>,
    requested_reason: RealtimePreviewFallbackReason,
) -> Result<RealtimePreviewServiceFrameResponse, PreviewServiceError> {
    let artifact = request_preview_frame(
        executor,
        &config.artifact_config,
        &PreviewFrameRequest {
            draft: request.draft.clone(),
            target_time: request.target_time,
        },
    )?;
    let reason = if artifact.from_cache {
        RealtimePreviewFallbackReason::PreviewArtifactCacheHit
    } else {
        requested_reason
    };
    let reason = if reason == RealtimePreviewFallbackReason::PreviewArtifactCacheHit {
        reason
    } else {
        RealtimePreviewFallbackReason::FfmpegArtifactGenerated
    };
    let realtime = runtime_frame(config, request, Some(reason), artifact.from_cache)?;

    Ok(RealtimePreviewServiceFrameResponse {
        fallback_decision: decision(
            request,
            Some(reason),
            artifact.from_cache,
            &realtime,
            diagnostics,
        ),
        realtime,
        artifact: Some(artifact.artifact),
        cache_entry: Some(artifact.cache_entry),
        ffmpeg_job: Some(artifact.ffmpeg_job),
        from_cache: artifact.from_cache,
    })
}

fn runtime_frame(
    config: &RealtimePreviewServiceConfig,
    request: &RealtimePreviewFrameServiceRequest,
    fallback_reason: Option<RealtimePreviewFallbackReason>,
    cache_hit: bool,
) -> Result<RealtimePreviewFrameResult, PreviewServiceError> {
    let mut runtime = config.runtime.lock().map_err(|_| {
        PreviewServiceError::new(
            PreviewServiceErrorKind::RuntimeFailed,
            "realtime preview runtime lock poisoned",
        )
    })?;
    runtime
        .request_frame(
            config.session_id,
            RealtimePreviewFrameRequest {
                target_time: request.target_time,
                playback_generation: request.playback_generation,
                cancellation_token: request.cancellation_token,
                mode: request.mode,
                queue_latency_ms: 0,
                render_duration_ms: if fallback_reason.is_some() { 0 } else { 1 },
                fallback_reason,
                cache_hit,
                repeated_frame: false,
                dropped_frame: false,
            },
        )
        .map_err(|error| {
            PreviewServiceError::new(
                PreviewServiceErrorKind::RuntimeFailed,
                format!("realtime preview runtime request failed: {error}"),
            )
        })
}

fn preflight_fallback_reason(
    config: &RealtimePreviewServiceConfig,
    diagnostics: &[RealtimePreviewDiagnostic],
) -> Option<RealtimePreviewFallbackReason> {
    if !config.runtime_backend_available {
        return Some(RealtimePreviewFallbackReason::NoGpuAdapter);
    }
    if !config.surface_available {
        return Some(RealtimePreviewFallbackReason::SurfaceUnavailable);
    }
    if !config.gpu_text_parity
        && diagnostics
            .iter()
            .any(|diagnostic| diagnostic.domain == RealtimePreviewDiagnosticDomain::Text)
    {
        return Some(RealtimePreviewFallbackReason::TextParityUnsupported);
    }
    None
}

fn classified_unsupported_reason(
    diagnostics: &[RealtimePreviewDiagnostic],
) -> RealtimePreviewFallbackReason {
    if diagnostics.iter().any(|diagnostic| {
        diagnostic.domain == RealtimePreviewDiagnosticDomain::MaterialFrame
            || diagnostic.reason.contains("frame")
            || diagnostic.reason.contains("decoded")
            || diagnostic.reason.contains("provider")
    }) {
        RealtimePreviewFallbackReason::FrameProviderUnavailable
    } else {
        RealtimePreviewFallbackReason::UnsupportedGraphIntent
    }
}

fn decision(
    request: &RealtimePreviewFrameServiceRequest,
    reason: Option<RealtimePreviewFallbackReason>,
    cache_hit: bool,
    realtime: &RealtimePreviewFrameResult,
    diagnostics: Vec<RealtimePreviewDiagnostic>,
) -> RealtimePreviewFallbackDecision {
    RealtimePreviewFallbackDecision {
        reason,
        target_time: request.target_time,
        playback_generation: request.playback_generation,
        cache_hit,
        fallback_counted: reason.is_some(),
        canceled: realtime.canceled,
        stale_rejected: realtime.stale_rejected,
        diagnostics,
    }
}

fn default_session_config(preferred_backend: PreviewGpuBackend) -> RealtimePreviewSessionConfig {
    RealtimePreviewSessionConfig {
        session_label: "preview-service-realtime".to_owned(),
        preferred_backend,
        frame_rate: RationalFrameRate::new(30, 1),
        playback_rate: PlaybackRate::normal(),
    }
}

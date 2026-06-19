use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use draft_model::{Draft, Material, MaterialId, MaterialKind, Microseconds, RationalFrameRate, TrackKind};
use media_runtime::{FfmpegExecutor, RuntimeConfig};
use media_runtime_desktop::{
    decode_ffmpeg_cpu_frame_fingerprint, FfmpegCpuFrameFingerprintRequest,
};
use project_store::resolve_material_uri;
use realtime_preview_runtime::{
    gpu::{NativeParentWindowHandle, PreviewSurfaceBounds, PreviewSurfaceDescriptor},
    PlaybackGeneration, PlaybackRate, PreviewCancellationToken, PreviewGpuBackend,
    PreviewRequestMode, PreviewSessionId, RealtimePreviewBackendUsed, RealtimePreviewDiagnostic,
    RealtimePreviewError, RealtimePreviewFallbackReason, RealtimePreviewFrameRequest,
    RealtimePreviewRuntime, RealtimePreviewSessionConfig, RealtimePreviewTelemetry,
};
use serde::{de::Error as SerdeDeError, Deserialize, Deserializer, Serialize};

const SESSION_PREFIX: &str = "rtprev-session-";

#[derive(Debug, Default)]
pub struct RealtimePreviewBindingRegistry {
    runtime: RealtimePreviewRuntime,
    next_binding_id: u64,
    sessions: BTreeMap<String, PreviewSessionId>,
}

impl RealtimePreviewBindingRegistry {
    pub fn new() -> Self {
        Self {
            runtime: RealtimePreviewRuntime::new(),
            next_binding_id: 1,
            sessions: BTreeMap::new(),
        }
    }

    pub fn create_session(
        &mut self,
        config: RealtimePreviewSessionBindingConfig,
    ) -> Result<RealtimePreviewSessionBindingResponse, RealtimePreviewBindingError> {
        let frame_rate =
            validate_frame_rate(config.frame_rate_numerator, config.frame_rate_denominator)?;
        let playback_rate = PlaybackRate::new(
            config.playback_rate_numerator,
            config.playback_rate_denominator,
        )
        .map_err(|error| {
            RealtimePreviewBindingError::new(
                RealtimePreviewBindingErrorKind::InvalidPayload,
                error.to_string(),
            )
        })?;
        let runtime_id = self
            .runtime
            .create_session(RealtimePreviewSessionConfig {
                session_label: config.session_label,
                preferred_backend: PreviewGpuBackend::Mock,
                frame_rate,
                playback_rate,
            })
            .map_err(RealtimePreviewBindingError::runtime)?;
        let binding_id = format!("{SESSION_PREFIX}{:016x}", self.next_binding_id);
        self.next_binding_id = self.next_binding_id.saturating_add(1);
        self.sessions.insert(binding_id.clone(), runtime_id);
        let generation = self
            .runtime
            .clock(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?
            .generation()
            .get();

        Ok(RealtimePreviewSessionBindingResponse {
            session_id: binding_id,
            playback_generation: generation,
        })
    }

    pub fn close_session(
        &mut self,
        session_id: &str,
    ) -> Result<RealtimePreviewClosedBindingResponse, RealtimePreviewBindingError> {
        validate_binding_session_id(session_id)?;
        let runtime_id = self
            .sessions
            .remove(session_id)
            .ok_or_else(|| RealtimePreviewBindingError::unknown_session(session_id))?;
        let closed = self.runtime.close_session(runtime_id);
        Ok(RealtimePreviewClosedBindingResponse {
            session_id: session_id.to_owned(),
            closed,
        })
    }

    pub fn attach_surface(
        &mut self,
        session_id: &str,
        descriptor: RealtimePreviewSurfaceBindingDescriptor,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let descriptor = descriptor.to_runtime_descriptor()?;
        let generation = self
            .runtime
            .attach_surface(runtime_id, descriptor)
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(generation_response(generation))
    }

    pub fn update_surface_bounds(
        &mut self,
        session_id: &str,
        bounds: RealtimePreviewSurfaceBoundsBindingRequest,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let generation = self
            .runtime
            .update_surface_bounds(runtime_id, bounds.to_runtime_bounds())
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(generation_response(generation))
    }

    pub fn detach_surface(
        &mut self,
        session_id: &str,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let generation = self
            .runtime
            .detach_surface(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(generation_response(generation))
    }

    pub fn update_draft_snapshot(
        &mut self,
        session_id: &str,
        draft: Draft,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let generation = self
            .runtime
            .update_draft_snapshot(runtime_id, draft)
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(generation_response(generation))
    }

    pub fn seek(
        &mut self,
        session_id: &str,
        target_time_microseconds: u64,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let generation = self
            .runtime
            .seek(runtime_id, Microseconds::new(target_time_microseconds))
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(generation_response(generation))
    }

    pub fn play(
        &mut self,
        session_id: &str,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let generation = self
            .runtime
            .play(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(generation_response(generation))
    }

    pub fn pause(
        &mut self,
        session_id: &str,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let generation = self
            .runtime
            .pause(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(generation_response(generation))
    }

    pub fn stop(
        &mut self,
        session_id: &str,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let generation = self
            .runtime
            .stop(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(generation_response(generation))
    }

    pub fn request_frame(
        &mut self,
        session_id: &str,
        request: RealtimePreviewFrameBindingRequest,
    ) -> Result<RealtimePreviewFrameBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let result = self
            .runtime
            .request_frame(runtime_id, request.to_runtime_request())
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(RealtimePreviewFrameBindingResponse {
            target_time_microseconds: result.target_time.get(),
            playback_generation: result.playback_generation.get(),
            presented: result.presented,
            stale_rejected: result.stale_rejected,
            canceled: result.canceled,
            cancellation_token: result.cancellation_token,
            backend: result.backend,
            fallback: result.fallback,
            diagnostics: result.diagnostics,
            telemetry: result.telemetry,
        })
    }

    pub fn next_cancellation_token(
        &mut self,
        session_id: &str,
    ) -> Result<PreviewCancellationToken, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        self.runtime
            .next_cancellation_token(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)
    }

    pub fn cancel_request(
        &mut self,
        session_id: &str,
        cancellation_token: PreviewCancellationToken,
    ) -> Result<RealtimePreviewCanceledBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        self.runtime
            .cancel_request(runtime_id, cancellation_token)
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(RealtimePreviewCanceledBindingResponse {
            cancellation_token,
            canceled: true,
        })
    }

    pub fn telemetry(
        &self,
        session_id: &str,
    ) -> Result<RealtimePreviewTelemetryBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        Ok(RealtimePreviewTelemetryBindingResponse::from_runtime(
            self.runtime
                .telemetry(runtime_id)
                .map_err(RealtimePreviewBindingError::runtime)?,
        ))
    }

    pub fn request_content_evidence(
        &self,
        request: RealtimePreviewContentEvidenceBindingRequest,
        executor: &impl FfmpegExecutor,
        runtime: &RuntimeConfig,
    ) -> Result<Option<RealtimePreviewContentEvidenceBindingResponse>, RealtimePreviewBindingError>
    {
        let runtime_id = self.runtime_session_id(&request.session_id)?;
        let active_generation = self
            .runtime
            .clock(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?
            .generation();
        if active_generation != PlaybackGeneration::new(request.playback_generation) {
            return Ok(None);
        }

        let Some(probe) = active_video_content_probe(
            &request.draft,
            request.bundle_path.as_deref(),
            request.target_time_microseconds,
        )?
        else {
            return Ok(None);
        };

        let fingerprint = decode_ffmpeg_cpu_frame_fingerprint(
            executor,
            runtime,
            &FfmpegCpuFrameFingerprintRequest {
                material_uri: probe.material_path,
                source_time_us: probe.source_time_microseconds,
            },
        )
        .map_err(|error| {
            RealtimePreviewBindingError::new(
                RealtimePreviewBindingErrorKind::Runtime,
                format!("realtime preview content fingerprint failed: {error}"),
            )
        })?;

        Ok(Some(RealtimePreviewContentEvidenceBindingResponse {
            source: RealtimePreviewContentEvidenceSource::Decoded,
            digest: fingerprint.digest,
            width: fingerprint.width,
            height: fingerprint.height,
            byte_count: fingerprint.byte_count,
            target_time_microseconds: request.target_time_microseconds,
            source_time_microseconds: fingerprint.source_time_us,
            material_id: probe.material_id,
            stream_id: fingerprint.stream_id.0,
        }))
    }

    fn runtime_session_id(
        &self,
        session_id: &str,
    ) -> Result<PreviewSessionId, RealtimePreviewBindingError> {
        validate_binding_session_id(session_id)?;
        self.sessions
            .get(session_id)
            .copied()
            .ok_or_else(|| RealtimePreviewBindingError::unknown_session(session_id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewSessionBindingConfig {
    pub session_label: String,
    pub frame_rate_numerator: u32,
    pub frame_rate_denominator: u32,
    pub playback_rate_numerator: i32,
    pub playback_rate_denominator: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewSessionBindingResponse {
    pub session_id: String,
    pub playback_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewClosedBindingResponse {
    pub session_id: String,
    pub closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RealtimePreviewSurfaceBindingKind {
    WindowsHwnd,
    MacosNsView,
    Mock,
    Offscreen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewSurfaceBindingDescriptor {
    pub kind: RealtimePreviewSurfaceBindingKind,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_u64_from_js_number",
        skip_serializing_if = "Option::is_none"
    )]
    pub parent_handle: Option<u64>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor_millis: u32,
}

impl RealtimePreviewSurfaceBindingDescriptor {
    fn to_runtime_descriptor(
        &self,
    ) -> Result<PreviewSurfaceDescriptor, RealtimePreviewBindingError> {
        let bounds = PreviewSurfaceBounds {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            scale_factor_millis: self.scale_factor_millis,
        };
        let descriptor = match self.kind {
            RealtimePreviewSurfaceBindingKind::WindowsHwnd => {
                PreviewSurfaceDescriptor::NativeChild {
                    parent_window_handle: NativeParentWindowHandle::WindowsHwnd(
                        self.parent_handle.unwrap_or_default(),
                    ),
                    bounds,
                }
            }
            RealtimePreviewSurfaceBindingKind::MacosNsView => {
                PreviewSurfaceDescriptor::NativeChild {
                    parent_window_handle: NativeParentWindowHandle::MacosNsView(
                        self.parent_handle.unwrap_or_default(),
                    ),
                    bounds,
                }
            }
            RealtimePreviewSurfaceBindingKind::Mock => PreviewSurfaceDescriptor::NativeChild {
                parent_window_handle: NativeParentWindowHandle::Mock(
                    self.parent_handle.unwrap_or_default(),
                ),
                bounds,
            },
            RealtimePreviewSurfaceBindingKind::Offscreen => PreviewSurfaceDescriptor::Offscreen {
                width: self.width,
                height: self.height,
                scale_factor_millis: self.scale_factor_millis,
            },
        };
        Ok(descriptor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewSurfaceBoundsBindingRequest {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor_millis: u32,
}

impl RealtimePreviewSurfaceBoundsBindingRequest {
    fn to_runtime_bounds(&self) -> PreviewSurfaceBounds {
        PreviewSurfaceBounds {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            scale_factor_millis: self.scale_factor_millis,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewGenerationBindingResponse {
    pub playback_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewFrameBindingRequest {
    pub target_time_microseconds: u64,
    pub playback_generation: u64,
    pub queue_latency_ms: u64,
    pub render_duration_ms: u64,
    pub mode: PreviewRequestMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_token: Option<PreviewCancellationToken>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<RealtimePreviewFallbackReason>,
    pub cache_hit: bool,
}

impl RealtimePreviewFrameBindingRequest {
    fn to_runtime_request(&self) -> RealtimePreviewFrameRequest {
        RealtimePreviewFrameRequest {
            target_time: Microseconds::new(self.target_time_microseconds),
            playback_generation: PlaybackGeneration::new(self.playback_generation),
            cancellation_token: self.cancellation_token,
            mode: self.mode,
            queue_latency_ms: self.queue_latency_ms,
            render_duration_ms: self.render_duration_ms,
            fallback_reason: self.fallback_reason,
            cache_hit: self.cache_hit,
            repeated_frame: false,
            dropped_frame: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewFrameBindingResponse {
    pub target_time_microseconds: u64,
    pub playback_generation: u64,
    pub presented: bool,
    pub stale_rejected: bool,
    pub canceled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_token: Option<PreviewCancellationToken>,
    pub backend: RealtimePreviewBackendUsed,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<RealtimePreviewFallbackReason>,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
    pub telemetry: RealtimePreviewTelemetry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewCanceledBindingResponse {
    pub cancellation_token: PreviewCancellationToken,
    pub canceled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewTelemetryBindingResponse {
    pub first_frame_latency_ms: Option<u64>,
    pub seek_latency_ms: Option<u64>,
    pub queue_latency_ms: u64,
    pub render_duration_ms: u64,
    pub presented_frame_count: u64,
    pub dropped_frame_count: u64,
    pub repeated_frame_count: u64,
    pub stale_rejected_count: u64,
    pub canceled_request_count: u64,
    pub fallback_count: u64,
    pub cache_hit_count: u64,
    pub target_time_microseconds: u64,
    pub playback_generation: u64,
}

impl RealtimePreviewTelemetryBindingResponse {
    fn from_runtime(telemetry: &RealtimePreviewTelemetry) -> Self {
        Self {
            first_frame_latency_ms: telemetry.first_frame_latency_ms,
            seek_latency_ms: telemetry.seek_latency_ms,
            queue_latency_ms: telemetry.queue_latency_ms,
            render_duration_ms: telemetry.render_duration_ms,
            presented_frame_count: telemetry.presented_frame_count,
            dropped_frame_count: telemetry.dropped_frame_count,
            repeated_frame_count: telemetry.repeated_frame_count,
            stale_rejected_count: telemetry.stale_rejected_count,
            canceled_request_count: telemetry.canceled_request_count,
            fallback_count: telemetry.fallback_count,
            cache_hit_count: telemetry.cache_hit_count,
            target_time_microseconds: telemetry.target_time.get(),
            playback_generation: telemetry.generation.get(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewContentEvidenceBindingRequest {
    pub session_id: String,
    pub draft: Draft,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_path: Option<String>,
    pub target_time_microseconds: u64,
    pub playback_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RealtimePreviewContentEvidenceSource {
    Decoded,
    Composited,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewContentEvidenceBindingResponse {
    pub source: RealtimePreviewContentEvidenceSource,
    pub digest: String,
    pub width: u32,
    pub height: u32,
    pub byte_count: usize,
    pub target_time_microseconds: u64,
    pub source_time_microseconds: u64,
    pub material_id: MaterialId,
    pub stream_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimePreviewBindingError {
    kind: RealtimePreviewBindingErrorKind,
    message: String,
}

impl RealtimePreviewBindingError {
    fn new(kind: RealtimePreviewBindingErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn runtime(error: RealtimePreviewError) -> Self {
        match error {
            RealtimePreviewError::UnknownSession { session_id } => Self::new(
                RealtimePreviewBindingErrorKind::UnknownSession,
                format!("unknown runtime session {}", session_id.get()),
            ),
            RealtimePreviewError::Surface { source, .. } => Self::new(
                RealtimePreviewBindingErrorKind::RuntimeSurface,
                source.to_string(),
            ),
        }
    }

    fn unknown_session(session_id: &str) -> Self {
        Self::new(
            RealtimePreviewBindingErrorKind::UnknownSession,
            format!("unknown realtime preview binding session: {session_id}"),
        )
    }

    pub const fn kind(&self) -> RealtimePreviewBindingErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for RealtimePreviewBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for RealtimePreviewBindingError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimePreviewBindingErrorKind {
    MalformedSessionId,
    UnknownSession,
    InvalidPayload,
    RuntimeSurface,
    Runtime,
}

struct ActiveVideoContentProbe {
    material_id: MaterialId,
    material_path: PathBuf,
    source_time_microseconds: u64,
}

fn active_video_content_probe(
    draft: &Draft,
    bundle_path: Option<&str>,
    target_time_microseconds: u64,
) -> Result<Option<ActiveVideoContentProbe>, RealtimePreviewBindingError> {
    for track in draft.tracks.iter().filter(|track| track.kind == TrackKind::Video && !track.muted)
    {
        for segment in &track.segments {
            let target_start = segment.target_timerange.start.get();
            let target_duration = segment.target_timerange.duration.get();
            let Some(target_end) = target_start.checked_add(target_duration) else {
                continue;
            };
            if target_time_microseconds < target_start || target_time_microseconds >= target_end {
                continue;
            }
            let Some(material) = material_for(draft, &segment.material_id) else {
                continue;
            };
            if material.kind != MaterialKind::Video || !material.metadata.has_video {
                continue;
            }
            let Some(material_path) = resolve_content_material_path(material, bundle_path)? else {
                continue;
            };
            let source_offset = target_time_microseconds.saturating_sub(target_start);
            let source_duration = segment.source_timerange.duration.get();
            let bounded_offset = if source_duration == 0 {
                0
            } else {
                source_offset.min(source_duration.saturating_sub(1))
            };
            return Ok(Some(ActiveVideoContentProbe {
                material_id: material.material_id.clone(),
                material_path,
                source_time_microseconds: segment
                    .source_timerange
                    .start
                    .get()
                    .saturating_add(bounded_offset),
            }));
        }
    }

    Ok(None)
}

fn material_for<'a>(draft: &'a Draft, material_id: &MaterialId) -> Option<&'a Material> {
    draft
        .materials
        .iter()
        .find(|material| &material.material_id == material_id)
}

fn resolve_content_material_path(
    material: &Material,
    bundle_path: Option<&str>,
) -> Result<Option<PathBuf>, RealtimePreviewBindingError> {
    let uri = material.uri.trim();
    if uri.is_empty() {
        return Ok(None);
    }

    let path = Path::new(uri);
    if path.is_absolute() {
        return Ok(Some(path.to_path_buf()));
    }

    let Some(bundle_path) = bundle_path else {
        return Ok(None);
    };
    resolve_material_uri(bundle_path, uri).map_err(|error| {
        RealtimePreviewBindingError::new(
            RealtimePreviewBindingErrorKind::InvalidPayload,
            error.to_string(),
        )
    })
}

fn validate_binding_session_id(session_id: &str) -> Result<(), RealtimePreviewBindingError> {
    let suffix = session_id.strip_prefix(SESSION_PREFIX).ok_or_else(|| {
        RealtimePreviewBindingError::new(
            RealtimePreviewBindingErrorKind::MalformedSessionId,
            "realtime preview session IDs are opaque binding IDs",
        )
    })?;
    if suffix.len() != 16 || !suffix.chars().all(|char| char.is_ascii_hexdigit()) {
        return Err(RealtimePreviewBindingError::new(
            RealtimePreviewBindingErrorKind::MalformedSessionId,
            "realtime preview session ID has invalid shape",
        ));
    }
    Ok(())
}

fn deserialize_optional_u64_from_js_number<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };

    match value {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::Number(number) => {
            if let Some(value) = number.as_u64() {
                return Ok(Some(value));
            }
            let Some(value) = number.as_f64() else {
                return Err(D::Error::custom("native parent handle must be an integer"));
            };
            if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > u64::MAX as f64
            {
                return Err(D::Error::custom(
                    "native parent handle must be a nonnegative integer",
                ));
            }
            Ok(Some(value as u64))
        }
        serde_json::Value::String(value) => {
            value.parse::<u64>().map(Some).map_err(D::Error::custom)
        }
        _ => Err(D::Error::custom("native parent handle must be an integer")),
    }
}

fn validate_frame_rate(
    numerator: u32,
    denominator: u32,
) -> Result<RationalFrameRate, RealtimePreviewBindingError> {
    if numerator == 0 || denominator == 0 {
        return Err(RealtimePreviewBindingError::new(
            RealtimePreviewBindingErrorKind::InvalidPayload,
            "frame rate numerator and denominator must be nonzero",
        ));
    }
    Ok(RationalFrameRate::new(numerator, denominator))
}

fn generation_response(
    playback_generation: PlaybackGeneration,
) -> RealtimePreviewGenerationBindingResponse {
    RealtimePreviewGenerationBindingResponse {
        playback_generation: playback_generation.get(),
    }
}

#[cfg(test)]
mod realtime_preview_bindings {
    use super::{
        RealtimePreviewBindingErrorKind, RealtimePreviewBindingRegistry,
        RealtimePreviewFrameBindingRequest, RealtimePreviewSessionBindingConfig,
        RealtimePreviewSurfaceBindingDescriptor, RealtimePreviewSurfaceBindingKind,
    };
    use realtime_preview_runtime::PreviewRequestMode;

    fn registry_with_session() -> (RealtimePreviewBindingRegistry, String) {
        let mut registry = RealtimePreviewBindingRegistry::new();
        let created = registry
            .create_session(RealtimePreviewSessionBindingConfig {
                session_label: "preview-main".to_owned(),
                frame_rate_numerator: 30,
                frame_rate_denominator: 1,
                playback_rate_numerator: 1,
                playback_rate_denominator: 1,
            })
            .expect("session is created");
        (registry, created.session_id)
    }

    #[test]
    fn malformed_session_ids_fail_safely() {
        let mut registry = RealtimePreviewBindingRegistry::new();
        let error = registry
            .close_session("not-a-runtime-session")
            .expect_err("malformed ids are rejected");

        assert_eq!(
            error.kind(),
            RealtimePreviewBindingErrorKind::MalformedSessionId
        );
    }

    #[test]
    fn surface_validation_reaches_rust_runtime() {
        let (mut registry, session_id) = registry_with_session();
        let error = registry
            .attach_surface(
                &session_id,
                RealtimePreviewSurfaceBindingDescriptor {
                    kind: RealtimePreviewSurfaceBindingKind::WindowsHwnd,
                    parent_handle: Some(0),
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                    scale_factor_millis: 1000,
                },
            )
            .expect_err("runtime rejects zero native parent handles");

        assert_eq!(
            error.kind(),
            RealtimePreviewBindingErrorKind::RuntimeSurface
        );
        assert!(
            error.message().contains("MissingParentHandle"),
            "diagnostic should come from Rust surface validation: {}",
            error.message()
        );
    }

    #[test]
    fn surface_parent_handle_accepts_integral_js_number_values() {
        let descriptor: RealtimePreviewSurfaceBindingDescriptor =
            serde_json::from_value(serde_json::json!({
                "kind": "mock",
                "parentHandle": 1357210896576.0,
                "x": 12,
                "y": 34,
                "width": 320,
                "height": 180,
                "scaleFactorMillis": 1250
            }))
            .expect("integral JS number handles deserialize");

        assert_eq!(descriptor.parent_handle, Some(1_357_210_896_576));
    }

    #[test]
    fn generation_and_target_microseconds_round_trip_as_integers() {
        let (mut registry, session_id) = registry_with_session();
        let generation = registry
            .seek(&session_id, 1_234_567)
            .expect("seek returns generation");
        let result = registry
            .request_frame(
                &session_id,
                RealtimePreviewFrameBindingRequest {
                    target_time_microseconds: 1_234_567,
                    playback_generation: generation.playback_generation,
                    queue_latency_ms: 3,
                    render_duration_ms: 4,
                    mode: PreviewRequestMode::Seek,
                    cancellation_token: None,
                    fallback_reason: None,
                    cache_hit: false,
                },
            )
            .expect("frame request succeeds");

        assert_eq!(result.target_time_microseconds, 1_234_567);
        assert_eq!(result.playback_generation, generation.playback_generation);
        assert!(result.presented);
    }

    #[test]
    fn playback_controls_return_monotonic_generations() {
        let (mut registry, session_id) = registry_with_session();

        let seek = registry
            .seek(&session_id, 500_000)
            .expect("seek returns generation");
        let play = registry.play(&session_id).expect("play returns generation");
        let pause = registry
            .pause(&session_id)
            .expect("pause returns generation");
        let stop = registry.stop(&session_id).expect("stop returns generation");

        assert!(seek.playback_generation < play.playback_generation);
        assert!(play.playback_generation < pause.playback_generation);
        assert!(pause.playback_generation < stop.playback_generation);
    }

    #[test]
    fn telemetry_is_queryable_without_native_or_gpu_handles() {
        let (mut registry, session_id) = registry_with_session();
        let generation = registry.seek(&session_id, 42).expect("seek succeeds");
        registry
            .request_frame(
                &session_id,
                RealtimePreviewFrameBindingRequest {
                    target_time_microseconds: 42,
                    playback_generation: generation.playback_generation,
                    queue_latency_ms: 5,
                    render_duration_ms: 7,
                    mode: PreviewRequestMode::Seek,
                    cancellation_token: None,
                    fallback_reason: None,
                    cache_hit: false,
                },
            )
            .expect("frame request records telemetry");

        let telemetry = registry
            .telemetry(&session_id)
            .expect("telemetry is returned");
        let telemetry_json =
            serde_json::to_value(&telemetry).expect("telemetry response serializes");

        assert_eq!(telemetry.target_time_microseconds, 42);
        assert_eq!(telemetry.presented_frame_count, 1);
        assert!(telemetry_json.get("gpuDevice").is_none());
        assert!(telemetry_json.get("commandEncoder").is_none());
        assert!(telemetry_json.get("nativeChildHandle").is_none());
        assert!(telemetry_json.get("cacheKey").is_none());
    }
}

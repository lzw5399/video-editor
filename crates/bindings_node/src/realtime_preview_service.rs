use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use draft_model::{Draft, Microseconds, RationalFrameRate};
use realtime_preview_runtime::{
    PlaybackGeneration, PlaybackRate, PreviewGpuBackend, PreviewRequestMode, PreviewSessionId,
    RealtimePreviewBackendUsed, RealtimePreviewError, RealtimePreviewFrameRequest,
    RealtimePreviewRuntime, RealtimePreviewSessionConfig, RealtimePreviewTelemetry,
    gpu::{NativeParentWindowHandle, PreviewSurfaceBounds, PreviewSurfaceDescriptor},
};
use serde::{Deserialize, Serialize};

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
            backend: result.backend,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
}

impl RealtimePreviewFrameBindingRequest {
    fn to_runtime_request(&self) -> RealtimePreviewFrameRequest {
        RealtimePreviewFrameRequest {
            target_time: Microseconds::new(self.target_time_microseconds),
            playback_generation: PlaybackGeneration::new(self.playback_generation),
            cancellation_token: None,
            mode: PreviewRequestMode::Seek,
            queue_latency_ms: self.queue_latency_ms,
            render_duration_ms: self.render_duration_ms,
            fallback_reason: None,
            cache_hit: false,
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
    pub backend: RealtimePreviewBackendUsed,
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
                },
            )
            .expect("frame request succeeds");

        assert_eq!(result.target_time_microseconds, 1_234_567);
        assert_eq!(result.playback_generation, generation.playback_generation);
        assert!(result.presented);
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

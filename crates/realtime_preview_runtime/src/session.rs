use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use draft_model::{AudioPreviewPlaybackStatus, Draft, Microseconds, RationalFrameRate};
use render_graph::RenderGraph;
use serde::{Deserialize, Serialize};

use crate::{
    PlaybackGeneration, PlaybackRate, PreviewCancellationToken, RealtimePreviewBackendUsed,
    RealtimePreviewDiagnostic, RealtimePreviewFallbackReason, RealtimePreviewFrameRequest,
    RealtimePreviewFrameResult, RealtimePreviewSupport, RealtimePreviewTelemetry, TimelineClock,
    gpu::surface::{
        PreviewSurfaceBounds, PreviewSurfaceDescriptor, PreviewSurfaceError, PreviewSurfaceHost,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PreviewSessionId(u64);

impl PreviewSessionId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewSessionConfig {
    pub session_label: String,
    pub preferred_backend: PreviewGpuBackend,
    pub frame_rate: RationalFrameRate,
    pub playback_rate: PlaybackRate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewGpuBackend {
    Auto,
    D3d12,
    Metal,
    OffscreenOnly,
    Mock,
}

impl PreviewGpuBackend {
    pub const fn resolve_for_current_platform(self) -> Self {
        match self {
            Self::Auto => {
                #[cfg(target_os = "windows")]
                {
                    Self::D3d12
                }
                #[cfg(target_os = "macos")]
                {
                    Self::Metal
                }
                #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                {
                    Self::OffscreenOnly
                }
            }
            backend => backend,
        }
    }
}

#[derive(Debug, Default)]
pub struct RealtimePreviewRuntime {
    next_session_id: u64,
    sessions: BTreeMap<PreviewSessionId, RealtimePreviewSession>,
}

impl RealtimePreviewRuntime {
    pub fn new() -> Self {
        Self {
            next_session_id: 1,
            sessions: BTreeMap::new(),
        }
    }

    pub fn create_session(
        &mut self,
        config: RealtimePreviewSessionConfig,
    ) -> Result<PreviewSessionId, RealtimePreviewError> {
        let session_id = PreviewSessionId::new(self.next_session_id);
        self.next_session_id = self.next_session_id.saturating_add(1);
        let clock = TimelineClock::new(
            Microseconds::ZERO,
            config.frame_rate.clone(),
            config.playback_rate,
        );
        self.sessions
            .insert(session_id, RealtimePreviewSession::new(config, clock));
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: PreviewSessionId) -> bool {
        self.sessions.remove(&session_id).is_some()
    }

    pub fn clock(
        &self,
        session_id: PreviewSessionId,
    ) -> Result<&TimelineClock, RealtimePreviewError> {
        Ok(&self.session(session_id)?.clock)
    }

    pub fn seek(
        &mut self,
        session_id: PreviewSessionId,
        target_time: Microseconds,
    ) -> Result<PlaybackGeneration, RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        session.clock.seek(target_time);
        Ok(session.clock.generation())
    }

    pub fn play(
        &mut self,
        session_id: PreviewSessionId,
    ) -> Result<PlaybackGeneration, RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        session.clock.play();
        Ok(session.clock.generation())
    }

    pub fn pause(
        &mut self,
        session_id: PreviewSessionId,
    ) -> Result<PlaybackGeneration, RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        session.clock.pause();
        Ok(session.clock.generation())
    }

    pub fn stop(
        &mut self,
        session_id: PreviewSessionId,
    ) -> Result<PlaybackGeneration, RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        session.clock.stop();
        Ok(session.clock.generation())
    }

    pub fn update_draft_snapshot(
        &mut self,
        session_id: PreviewSessionId,
        draft: Draft,
    ) -> Result<PlaybackGeneration, RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        session.draft_snapshot = Some(draft);
        session.clock.draft_reloaded();
        Ok(session.clock.generation())
    }

    pub fn update_render_graph_snapshot(
        &mut self,
        session_id: PreviewSessionId,
        graph: RenderGraph,
    ) -> Result<PlaybackGeneration, RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        session.render_graph_snapshot = Some(graph);
        session.clock.accepted_edit();
        Ok(session.clock.generation())
    }

    pub fn attach_surface(
        &mut self,
        session_id: PreviewSessionId,
        descriptor: PreviewSurfaceDescriptor,
    ) -> Result<PlaybackGeneration, RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        session
            .surface
            .attach(descriptor)
            .map_err(|source| RealtimePreviewError::Surface { session_id, source })?;
        session.clock.accepted_edit();
        Ok(session.clock.generation())
    }

    pub fn update_surface_bounds(
        &mut self,
        session_id: PreviewSessionId,
        bounds: PreviewSurfaceBounds,
    ) -> Result<PlaybackGeneration, RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        session
            .surface
            .update_bounds(bounds)
            .map_err(|source| RealtimePreviewError::Surface { session_id, source })?;
        session.clock.accepted_edit();
        Ok(session.clock.generation())
    }

    pub fn detach_surface(
        &mut self,
        session_id: PreviewSessionId,
    ) -> Result<PlaybackGeneration, RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        session
            .surface
            .detach()
            .map_err(|source| RealtimePreviewError::Surface { session_id, source })?;
        session.clock.accepted_edit();
        Ok(session.clock.generation())
    }

    pub fn telemetry(
        &self,
        session_id: PreviewSessionId,
    ) -> Result<&RealtimePreviewTelemetry, RealtimePreviewError> {
        Ok(&self.session(session_id)?.telemetry)
    }

    pub fn record_presented_output(
        &mut self,
        session_id: PreviewSessionId,
        playback_generation: PlaybackGeneration,
        target_time: Microseconds,
        render_duration_ms: u64,
        dropped_frame_count: u64,
    ) -> Result<(), RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        if session.clock.generation() != playback_generation {
            return Ok(());
        }
        session.clock.record_playback_position(target_time);
        session.telemetry.record_presented_output(
            target_time,
            playback_generation,
            render_duration_ms,
            dropped_frame_count,
        );
        Ok(())
    }

    pub fn next_cancellation_token(
        &mut self,
        session_id: PreviewSessionId,
    ) -> Result<PreviewCancellationToken, RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        let token = PreviewCancellationToken::new(session.next_cancellation_token);
        session.next_cancellation_token = session.next_cancellation_token.saturating_add(1);
        Ok(token)
    }

    pub fn cancel_request(
        &mut self,
        session_id: PreviewSessionId,
        token: PreviewCancellationToken,
    ) -> Result<(), RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        session.canceled_tokens.insert(token);
        Ok(())
    }

    pub fn request_frame(
        &mut self,
        session_id: PreviewSessionId,
        request: RealtimePreviewFrameRequest,
    ) -> Result<RealtimePreviewFrameResult, RealtimePreviewError> {
        let session = self.session_mut(session_id)?;
        Ok(session.request_frame(request))
    }

    fn session(
        &self,
        session_id: PreviewSessionId,
    ) -> Result<&RealtimePreviewSession, RealtimePreviewError> {
        self.sessions
            .get(&session_id)
            .ok_or(RealtimePreviewError::UnknownSession { session_id })
    }

    fn session_mut(
        &mut self,
        session_id: PreviewSessionId,
    ) -> Result<&mut RealtimePreviewSession, RealtimePreviewError> {
        self.sessions
            .get_mut(&session_id)
            .ok_or(RealtimePreviewError::UnknownSession { session_id })
    }
}

#[derive(Debug)]
struct RealtimePreviewSession {
    config: RealtimePreviewSessionConfig,
    clock: TimelineClock,
    telemetry: RealtimePreviewTelemetry,
    canceled_tokens: BTreeSet<PreviewCancellationToken>,
    next_cancellation_token: u64,
    draft_snapshot: Option<Draft>,
    render_graph_snapshot: Option<RenderGraph>,
    surface: PreviewSurfaceHost,
}

impl RealtimePreviewSession {
    fn new(config: RealtimePreviewSessionConfig, clock: TimelineClock) -> Self {
        Self {
            config,
            telemetry: RealtimePreviewTelemetry::new(clock.position(), clock.generation()),
            clock,
            canceled_tokens: BTreeSet::new(),
            next_cancellation_token: 1,
            draft_snapshot: None,
            render_graph_snapshot: None,
            surface: PreviewSurfaceHost::new(),
        }
    }

    fn request_frame(
        &mut self,
        request: RealtimePreviewFrameRequest,
    ) -> RealtimePreviewFrameResult {
        let audio_rejection = audio_sync_rejection(&request);
        let stale_rejected =
            request.playback_generation != self.clock.generation() || audio_rejection.is_some();
        let canceled = request
            .cancellation_token
            .map(|token| self.canceled_tokens.contains(&token))
            .unwrap_or(false);
        let presented = !stale_rejected && !canceled;

        self.telemetry
            .record_request(&request, presented, stale_rejected, canceled);

        let fallback = if stale_rejected {
            Some(RealtimePreviewFallbackReason::StaleGeneration)
        } else if canceled {
            Some(RealtimePreviewFallbackReason::Canceled)
        } else {
            request.fallback_reason
        };
        let backend = backend_for(self.config.preferred_backend, presented, fallback);
        let diagnostics = diagnostics_for(&request, stale_rejected, canceled, fallback);

        RealtimePreviewFrameResult {
            target_time: request.target_time,
            playback_generation: request.playback_generation,
            presented,
            stale_rejected,
            canceled,
            cancellation_token: request.cancellation_token,
            audio_sync: request.audio_sync,
            backend,
            fallback,
            diagnostics,
            telemetry: self.telemetry.clone(),
        }
    }
}

fn audio_sync_rejection(request: &RealtimePreviewFrameRequest) -> Option<String> {
    let audio = request.audio_sync.as_ref()?;
    if audio.session_id.trim().is_empty() {
        return Some("audio preview session id is missing".to_owned());
    }
    if audio.playback_generation != request.playback_generation {
        return Some(format!(
            "audio generation {} does not match preview generation {}",
            audio.playback_generation.get(),
            request.playback_generation.get()
        ));
    }
    if audio.target_time != request.target_time {
        return Some(format!(
            "audio target time {} does not match preview target time {}",
            audio.target_time.get(),
            request.target_time.get()
        ));
    }
    if audio.buffered_until < request.target_time {
        return Some(format!(
            "audio buffered until {} but preview target is {}",
            audio.buffered_until.get(),
            request.target_time.get()
        ));
    }
    if matches!(
        audio.status,
        AudioPreviewPlaybackStatus::StaleRejected
            | AudioPreviewPlaybackStatus::Unavailable
            | AudioPreviewPlaybackStatus::Failed
    ) {
        return Some(format!(
            "audio preview status {:?} cannot be synchronized for presentation",
            audio.status
        ));
    }
    None
}

fn backend_for(
    preferred_backend: PreviewGpuBackend,
    presented: bool,
    fallback: Option<RealtimePreviewFallbackReason>,
) -> RealtimePreviewBackendUsed {
    if !presented {
        return RealtimePreviewBackendUsed::None;
    }
    match fallback {
        Some(RealtimePreviewFallbackReason::PreviewArtifactCacheHit) => {
            RealtimePreviewBackendUsed::PreviewArtifact
        }
        Some(RealtimePreviewFallbackReason::FfmpegArtifactGenerated) => {
            RealtimePreviewBackendUsed::FfmpegArtifact
        }
        Some(_) => RealtimePreviewBackendUsed::Offscreen,
        None => match preferred_backend.resolve_for_current_platform() {
            PreviewGpuBackend::Mock => RealtimePreviewBackendUsed::Mock,
            PreviewGpuBackend::OffscreenOnly => RealtimePreviewBackendUsed::Offscreen,
            PreviewGpuBackend::D3d12 | PreviewGpuBackend::Metal => RealtimePreviewBackendUsed::Gpu,
            PreviewGpuBackend::Auto => unreachable!("Auto backend must resolve first"),
        },
    }
}

fn diagnostics_for(
    request: &RealtimePreviewFrameRequest,
    stale_rejected: bool,
    canceled: bool,
    fallback: Option<RealtimePreviewFallbackReason>,
) -> Vec<RealtimePreviewDiagnostic> {
    if stale_rejected {
        let reason =
            audio_sync_rejection(request).unwrap_or_else(|| "stale playback generation".to_owned());
        let message = format!("preview result rejected because {reason}");
        return vec![RealtimePreviewDiagnostic::runtime(
            message,
            RealtimePreviewSupport::Unsupported { reason },
            Some(RealtimePreviewFallbackReason::StaleGeneration),
            false,
            false,
            request.cancellation_token,
        )];
    }
    if canceled {
        return vec![RealtimePreviewDiagnostic::runtime(
            "preview request canceled before presentation",
            RealtimePreviewSupport::Degraded {
                reason: "request canceled".to_owned(),
            },
            Some(RealtimePreviewFallbackReason::Canceled),
            true,
            true,
            request.cancellation_token,
        )];
    }
    if let Some(fallback) = fallback {
        return vec![RealtimePreviewDiagnostic::runtime(
            "preview request used runtime fallback",
            RealtimePreviewSupport::Degraded {
                reason: format!("{fallback:?}"),
            },
            Some(fallback),
            true,
            false,
            request.cancellation_token,
        )];
    }
    Vec::new()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealtimePreviewError {
    UnknownSession {
        session_id: PreviewSessionId,
    },
    Surface {
        session_id: PreviewSessionId,
        source: PreviewSurfaceError,
    },
}

impl fmt::Display for RealtimePreviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSession { session_id } => {
                write!(
                    formatter,
                    "unknown realtime preview session {}",
                    session_id.get()
                )
            }
            Self::Surface { session_id, source } => {
                write!(
                    formatter,
                    "realtime preview session {} surface error: {}",
                    session_id.get(),
                    source
                )
            }
        }
    }
}

impl Error for RealtimePreviewError {}

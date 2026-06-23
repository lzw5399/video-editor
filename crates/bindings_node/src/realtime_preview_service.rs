use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use draft_model::{Draft, MaterialId, MaterialKind, Microseconds, RationalFrameRate};
use media_runtime::{
    NativeTextureLease, NativeTextureLeaseRegistry, RuntimeDeviceId, SelectedDecodePath, StreamId,
};
#[cfg(target_os = "macos")]
use media_runtime_desktop::{
    MacosMediaReader, MacosRegisteredTextureLease, MacosTextureInteropPolicy,
    macos_system_metal_device_id,
};
use project_store::resolve_material_uri;
use realtime_preview_runtime::{
    MediaIoFrameProvider, PendingPreviewFrameRelease, PlaybackGeneration, PlaybackRate,
    PlaybackState, PreviewCancellationToken, PreviewDecodeDeviceContext, PreviewFrameInput,
    PreviewFrameProvider, PreviewFrameProviderError, PreviewFrameStoragePreference,
    PreviewGpuBackend, PreviewMaterialDecodeSource, PreviewRequestMode, PreviewSessionId,
    RealtimePlaybackCadence, RealtimePlaybackDueTick, RealtimePlaybackPresentationQueuePolicy,
    RealtimePlaybackPresentedFrame, RealtimePlaybackScheduler, RealtimePlaybackSchedulerConfig,
    RealtimePlaybackSchedulerError, RealtimePlaybackSchedulerEvidence,
    RealtimePlaybackSchedulerPresentation, RealtimePlaybackSchedulerPresenter,
    RealtimePlaybackSelectedSegment, RealtimePlaybackTextOverlayEvidence, RealtimePlaybackTimeline,
    RealtimePreviewAudioSyncState, RealtimePreviewBackendUsed, RealtimePreviewCapabilityClassifier,
    RealtimePreviewCompositor, RealtimePreviewDiagnostic, RealtimePreviewError,
    RealtimePreviewFallbackReason, RealtimePreviewFramePacingSample,
    RealtimePreviewFramePacingTelemetry, RealtimePreviewFrameRequest, RealtimePreviewRuntime,
    RealtimePreviewSessionConfig, RealtimePreviewTelemetry, RealtimePreviewUiChrome,
    TextureHandleDescriptor,
    gpu::{NativeParentWindowHandle, PreviewSurfaceBounds, PreviewSurfaceDescriptor},
    gpu::{
        RealtimePreviewExternalTexturePlanes, RealtimePreviewGpuBackend, RealtimePreviewGpuDevice,
        RealtimePreviewGpuDeviceDescriptor, RealtimePreviewGpuPresentationTarget,
        RealtimePreviewSurfaceSubmissionFence, RealtimePreviewTargetFormat,
        RealtimePreviewTextureCache,
    },
};
use render_graph::{OutputDimensions, RenderGraph};
use serde::{Deserialize, Deserializer, Serialize, de::Error as SerdeDeError};
use task_runtime::{
    CompletionFreshness, JobCompletion, JobDomain, JobEnvelope, JobFreshness, JobId, JobPriority,
    JobResult, JobResultKind, ResourceClass, SchedulerTelemetrySnapshot, TaskCancellationToken,
    TaskRuntimeConfig,
};

use crate::native_preview_presenter::{
    NativePreviewContentEvidence, NativePreviewContentEvidenceSource,
    NativePreviewPresentationState, NativePreviewPresenter, NativePreviewPresenterError,
    NativePreviewScreenRect, NativePreviewSurfacePlacementEvidence,
};
use crate::timeline_selection::timeline_segment_selection_handle;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};

const SESSION_PREFIX: &str = "rtprev-session-";
static NEXT_SCHEDULER_MEDIA_PIPELINE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimePreviewSelectedSegmentBinding {
    pub track_id: String,
    pub segment_id: String,
}

thread_local! {
    static SCHEDULER_MEDIA_PIPELINES: RefCell<BTreeMap<u64, SchedulerMediaPipeline>> =
        RefCell::new(BTreeMap::new());
}

#[derive(Clone)]
struct RealtimePreviewEventSink {
    callback: Arc<Mutex<Option<ThreadsafeFunction<String>>>>,
}

impl RealtimePreviewEventSink {
    fn new() -> Self {
        Self {
            callback: Arc::new(Mutex::new(None)),
        }
    }

    fn subscribe(
        &self,
        callback: ThreadsafeFunction<String>,
    ) -> Result<(), RealtimePreviewBindingError> {
        let mut sink = self.callback.lock().map_err(|_| {
            RealtimePreviewBindingError::new(
                RealtimePreviewBindingErrorKind::Runtime,
                "realtime preview event sink lock poisoned",
            )
        })?;
        *sink = Some(callback);
        Ok(())
    }

    fn unsubscribe(&self) -> Result<(), RealtimePreviewBindingError> {
        let mut sink = self.callback.lock().map_err(|_| {
            RealtimePreviewBindingError::new(
                RealtimePreviewBindingErrorKind::Runtime,
                "realtime preview event sink lock poisoned",
            )
        })?;
        *sink = None;
        Ok(())
    }

    fn emit(&self, event: RealtimePreviewBindingEvent) {
        let Ok(payload) = serde_json::to_string(&event) else {
            return;
        };
        if let Ok(sink) = self.callback.lock() {
            if let Some(callback) = sink.as_ref() {
                let _ = callback.call(Ok(payload), ThreadsafeFunctionCallMode::NonBlocking);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealtimePreviewBindingEvent {
    pub session_id: String,
    pub kind: RealtimePreviewBindingEventKind,
    pub playback_generation: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_time_microseconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dropped_frame_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

impl RealtimePreviewBindingEvent {
    fn new(
        session_id: &str,
        kind: RealtimePreviewBindingEventKind,
        playback_generation: u64,
    ) -> Self {
        Self {
            session_id: session_id.to_owned(),
            kind,
            playback_generation,
            target_time_microseconds: None,
            dropped_frame_count: None,
            error_message: None,
        }
    }

    fn presented(
        session_id: &str,
        playback_generation: PlaybackGeneration,
        target_time_microseconds: u64,
        dropped_frame_count: u64,
    ) -> Self {
        Self {
            session_id: session_id.to_owned(),
            kind: RealtimePreviewBindingEventKind::FramePresented,
            playback_generation: playback_generation.get(),
            target_time_microseconds: Some(target_time_microseconds),
            dropped_frame_count: Some(dropped_frame_count),
            error_message: None,
        }
    }

    fn error(
        session_id: &str,
        playback_generation: PlaybackGeneration,
        error_message: String,
    ) -> Self {
        Self {
            session_id: session_id.to_owned(),
            kind: RealtimePreviewBindingEventKind::PlaybackError,
            playback_generation: playback_generation.get(),
            target_time_microseconds: None,
            dropped_frame_count: None,
            error_message: Some(error_message),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RealtimePreviewBindingEventKind {
    SessionCreated,
    SessionClosed,
    ControlChanged,
    FramePresented,
    PlaybackEnded,
    PlaybackError,
}

pub struct RealtimePreviewBindingRegistry {
    runtime: Arc<Mutex<RealtimePreviewRuntime>>,
    event_sink: RealtimePreviewEventSink,
    next_binding_id: u64,
    sessions: BTreeMap<String, PreviewSessionId>,
    presenters: BTreeMap<String, NativePreviewPresenter>,
    schedulers: BTreeMap<String, RealtimePreviewSchedulerSession>,
}

impl RealtimePreviewBindingRegistry {
    pub fn new() -> Self {
        Self {
            runtime: Arc::new(Mutex::new(RealtimePreviewRuntime::new())),
            event_sink: RealtimePreviewEventSink::new(),
            next_binding_id: 1,
            sessions: BTreeMap::new(),
            presenters: BTreeMap::new(),
            schedulers: BTreeMap::new(),
        }
    }

    pub fn subscribe_events(
        &mut self,
        callback: ThreadsafeFunction<String>,
    ) -> Result<RealtimePreviewEventSubscriptionResponse, RealtimePreviewBindingError> {
        self.event_sink.subscribe(callback)?;
        Ok(RealtimePreviewEventSubscriptionResponse { subscribed: true })
    }

    pub fn unsubscribe_events(
        &mut self,
    ) -> Result<RealtimePreviewEventSubscriptionResponse, RealtimePreviewBindingError> {
        self.event_sink.unsubscribe()?;
        Ok(RealtimePreviewEventSubscriptionResponse { subscribed: false })
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
            .runtime_lock()?
            .create_session(RealtimePreviewSessionConfig {
                session_label: config.session_label,
                preferred_backend: PreviewGpuBackend::Auto,
                frame_rate,
                playback_rate,
            })
            .map_err(RealtimePreviewBindingError::runtime)?;
        let binding_id = format!("{SESSION_PREFIX}{:016x}", self.next_binding_id);
        self.next_binding_id = self.next_binding_id.saturating_add(1);
        self.sessions.insert(binding_id.clone(), runtime_id);
        self.presenters
            .insert(binding_id.clone(), NativePreviewPresenter::detached());
        self.schedulers.insert(
            binding_id.clone(),
            RealtimePreviewSchedulerSession::new(RealtimePlaybackSchedulerConfig {
                preview_dimensions: OutputDimensions {
                    width: 1280,
                    height: 720,
                },
            }),
        );
        let generation = self
            .runtime_lock()?
            .clock(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?
            .generation()
            .get();
        self.event_sink.emit(RealtimePreviewBindingEvent::new(
            &binding_id,
            RealtimePreviewBindingEventKind::SessionCreated,
            generation,
        ));

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
        self.stop_playback_driver(session_id);
        if let Some(mut presenter) = self.presenters.remove(session_id) {
            presenter.detach();
        }
        self.schedulers.remove(session_id);
        let closed = self.runtime_lock()?.close_session(runtime_id);
        self.event_sink.emit(RealtimePreviewBindingEvent::new(
            session_id,
            RealtimePreviewBindingEventKind::SessionClosed,
            0,
        ));
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
        self.cancel_playback_driver(session_id);
        self.with_scheduler_mut(session_id, |scheduler| scheduler.attach_surface(descriptor))?;
        let generation = match self.runtime_lock()?.attach_surface(runtime_id, descriptor) {
            Ok(generation) => generation,
            Err(error) => {
                self.with_scheduler_mut(session_id, |scheduler| {
                    scheduler.detach_surface();
                    Ok(())
                })?;
                return Err(RealtimePreviewBindingError::runtime(error));
            }
        };
        self.with_scheduler_mut(session_id, |scheduler| {
            scheduler.set_active_generation(generation);
            Ok(())
        })?;
        self.emit_control_event(session_id, generation);
        Ok(generation_response(generation))
    }

    pub fn update_surface_bounds(
        &mut self,
        session_id: &str,
        bounds: RealtimePreviewSurfaceBoundsBindingRequest,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let (playback_state, target_time) = {
            let runtime = self.runtime_lock()?;
            let clock = runtime
                .clock(runtime_id)
                .map_err(RealtimePreviewBindingError::runtime)?;
            (clock.state(), clock.position())
        };
        let generation = self
            .runtime_lock()?
            .update_surface_bounds(runtime_id, bounds.to_runtime_bounds())
            .map_err(RealtimePreviewBindingError::runtime)?;
        self.with_scheduler_mut(session_id, |scheduler| {
            scheduler.update_surface_bounds(bounds.to_runtime_bounds())
        })?;
        if playback_state != PlaybackState::Playing {
            self.present_scheduler_still_frame(
                session_id,
                runtime_id,
                generation,
                target_time,
                PreviewRequestMode::FirstFrame,
            )?;
        }
        Ok(generation_response(generation))
    }

    pub fn detach_surface(
        &mut self,
        session_id: &str,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        self.cancel_playback_driver(session_id);
        let generation = self
            .runtime_lock()?
            .detach_surface(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?;
        self.presenter_mut(session_id)?.detach();
        self.with_scheduler_mut(session_id, |scheduler| {
            scheduler.detach_surface();
            scheduler.set_active_generation(generation);
            Ok(())
        })?;
        self.emit_control_event(session_id, generation);
        Ok(generation_response(generation))
    }

    pub fn update_draft_snapshot(
        &mut self,
        session_id: &str,
        draft: Draft,
        bundle_path: Option<PathBuf>,
        selected_segment: Option<RealtimePreviewSelectedSegmentBinding>,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        self.cancel_playback_driver(session_id);
        self.with_scheduler_mut(session_id, |scheduler| {
            scheduler.update_draft_snapshot(
                draft.clone(),
                bundle_path.clone(),
                selected_segment.clone(),
            );
            Ok(())
        })?;
        let generation = self
            .runtime_lock()?
            .update_draft_snapshot(runtime_id, draft)
            .map_err(RealtimePreviewBindingError::runtime)?;
        self.with_scheduler_mut(session_id, |scheduler| {
            scheduler.set_active_generation(generation);
            Ok(())
        })?;
        self.emit_control_event(session_id, generation);
        Ok(generation_response(generation))
    }

    pub fn seek(
        &mut self,
        session_id: &str,
        target_time_microseconds: u64,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let target_time = Microseconds::new(target_time_microseconds);
        self.cancel_playback_driver(session_id);
        let generation = self
            .runtime_lock()?
            .seek(runtime_id, target_time)
            .map_err(RealtimePreviewBindingError::runtime)?;
        self.with_scheduler_mut(session_id, |scheduler| {
            scheduler.seek(target_time);
            scheduler.set_active_generation(generation);
            Ok(())
        })?;
        self.present_scheduler_still_frame(
            session_id,
            runtime_id,
            generation,
            target_time,
            PreviewRequestMode::Seek,
        )?;
        let mut event = RealtimePreviewBindingEvent::new(
            session_id,
            RealtimePreviewBindingEventKind::ControlChanged,
            generation.get(),
        );
        event.target_time_microseconds = Some(target_time.get());
        self.event_sink.emit(event);
        Ok(generation_response(generation))
    }

    pub fn play(
        &mut self,
        session_id: &str,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let (target_time, cadence) = {
            let runtime = self.runtime_lock()?;
            let clock = runtime
                .clock(runtime_id)
                .map_err(RealtimePreviewBindingError::runtime)?;
            let cadence = RealtimePlaybackCadence::new(clock.frame_rate(), clock.playback_rate())
                .map_err(playback_cadence_error)?;
            (clock.position(), cadence)
        };
        self.with_scheduler_mut(session_id, |scheduler| {
            scheduler.validate_playback_ready()?;
            Ok(())
        })?;
        self.cancel_playback_driver(session_id);
        let generation = self
            .runtime_lock()?
            .play(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?;
        self.with_scheduler_mut(session_id, |scheduler| {
            scheduler.set_active_generation(generation);
            Ok(())
        })?;
        self.start_playback_driver(session_id, runtime_id, generation, target_time, cadence)?;
        self.emit_control_event(session_id, generation);
        Ok(generation_response(generation))
    }

    pub fn pause(
        &mut self,
        session_id: &str,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        self.cancel_playback_driver(session_id);
        self.with_scheduler_mut(session_id, |scheduler| {
            scheduler.pause_playback();
            Ok(())
        })?;
        let generation = self
            .runtime_lock()?
            .pause(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?;
        self.with_scheduler_mut(session_id, |scheduler| {
            scheduler.set_active_generation(generation);
            Ok(())
        })?;
        self.emit_control_event(session_id, generation);
        Ok(generation_response(generation))
    }

    pub fn stop(
        &mut self,
        session_id: &str,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        self.cancel_playback_driver(session_id);
        self.with_scheduler_mut(session_id, |scheduler| {
            scheduler.stop_playback();
            Ok(())
        })?;
        let generation = self
            .runtime_lock()?
            .stop(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?;
        self.with_scheduler_mut(session_id, |scheduler| {
            scheduler.set_active_generation(generation);
            Ok(())
        })?;
        self.emit_control_event(session_id, generation);
        Ok(generation_response(generation))
    }

    pub fn request_frame(
        &mut self,
        session_id: &str,
        request: RealtimePreviewFrameBindingRequest,
    ) -> Result<RealtimePreviewFrameBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let result = self
            .runtime_lock()?
            .request_frame(runtime_id, request.to_runtime_request())
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(RealtimePreviewFrameBindingResponse {
            target_time_microseconds: result.target_time.get(),
            playback_generation: result.playback_generation.get(),
            presented: result.presented,
            stale_rejected: result.stale_rejected,
            canceled: result.canceled,
            cancellation_token: result.cancellation_token,
            audio_sync: result.audio_sync,
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
        self.runtime_lock()?
            .next_cancellation_token(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)
    }

    pub fn cancel_request(
        &mut self,
        session_id: &str,
        cancellation_token: PreviewCancellationToken,
    ) -> Result<RealtimePreviewCanceledBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        self.runtime_lock()?
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
            self.runtime_lock()?
                .telemetry(runtime_id)
                .map_err(RealtimePreviewBindingError::runtime)?,
        ))
    }

    pub fn presentation_state(
        &mut self,
        session_id: &str,
    ) -> Result<NativePreviewPresentationState, RealtimePreviewBindingError> {
        let snapshot = self.playback_snapshot(session_id)?;
        let evidence = snapshot.evidence.map(native_evidence_from_scheduler);
        let surface_placement = snapshot.surface_placement;
        match evidence {
            Some(evidence) => Ok(
                NativePreviewPresentationState::render_graph_gpu_available(Some(evidence))
                    .with_surface_placement(surface_placement),
            ),
            None => Ok(NativePreviewPresentationState::unavailable(
                snapshot.unsupported_reason.unwrap_or_else(|| {
                    "render graph GPU compositor scheduler has not presented product content"
                        .to_owned()
                }),
            )),
        }
    }

    pub fn hit_test_text_overlay(
        &self,
        session_id: &str,
        request: RealtimePreviewTextHitTestBindingRequest,
    ) -> Result<RealtimePreviewTextHitTestBindingResponse, RealtimePreviewBindingError> {
        let snapshot = self.playback_snapshot(session_id)?;
        let Some(evidence) = snapshot.evidence else {
            return Ok(RealtimePreviewTextHitTestBindingResponse::miss());
        };
        let hit = evidence.active_text_overlays.iter().rev().find(|text| {
            transformed_text_overlay_contains(
                text,
                evidence.width,
                evidence.height,
                request.x,
                request.y,
            )
        });
        let Some(text) = hit else {
            return Ok(RealtimePreviewTextHitTestBindingResponse::miss());
        };
        Ok(RealtimePreviewTextHitTestBindingResponse {
            hit: true,
            track_id: Some(text.track_id.clone()),
            segment_id: Some(text.segment_id.clone()),
            selection_handle: Some(timeline_segment_selection_handle(
                &draft_model::TrackId::new(text.track_id.clone()),
                &draft_model::SegmentId::new(text.segment_id.clone()),
            )),
            source: Some(text.source),
            content: Some(text.content.clone()),
            x: Some(text.x),
            y: Some(text.y),
            width: Some(text.width),
            height: Some(text.height),
            target_time_microseconds: Some(evidence.target_time_microseconds),
        })
    }

    fn emit_control_event(&self, session_id: &str, generation: PlaybackGeneration) {
        self.event_sink.emit(RealtimePreviewBindingEvent::new(
            session_id,
            RealtimePreviewBindingEventKind::ControlChanged,
            generation.get(),
        ));
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

    fn runtime_lock(
        &self,
    ) -> Result<MutexGuard<'_, RealtimePreviewRuntime>, RealtimePreviewBindingError> {
        self.runtime.lock().map_err(|_| {
            RealtimePreviewBindingError::new(
                RealtimePreviewBindingErrorKind::Runtime,
                "realtime preview runtime lock poisoned",
            )
        })
    }

    fn presenter_mut(
        &mut self,
        session_id: &str,
    ) -> Result<&mut NativePreviewPresenter, RealtimePreviewBindingError> {
        validate_binding_session_id(session_id)?;
        self.presenters
            .get_mut(session_id)
            .ok_or_else(|| RealtimePreviewBindingError::unknown_session(session_id))
    }

    fn scheduler_session(
        &self,
        session_id: &str,
    ) -> Result<&RealtimePreviewSchedulerSession, RealtimePreviewBindingError> {
        validate_binding_session_id(session_id)?;
        self.schedulers
            .get(session_id)
            .ok_or_else(|| RealtimePreviewBindingError::unknown_session(session_id))
    }

    fn scheduler_handle(
        &self,
        session_id: &str,
    ) -> Result<Arc<Mutex<RealtimePreviewBindingScheduler>>, RealtimePreviewBindingError> {
        Ok(Arc::clone(&self.scheduler_session(session_id)?.scheduler))
    }

    fn with_scheduler_mut<T>(
        &self,
        session_id: &str,
        action: impl FnOnce(
            &mut RealtimePreviewBindingScheduler,
        ) -> Result<T, RealtimePreviewBindingError>,
    ) -> Result<T, RealtimePreviewBindingError> {
        let scheduler = self.scheduler_handle(session_id)?;
        let mut scheduler = scheduler.lock().map_err(|_| {
            RealtimePreviewBindingError::new(
                RealtimePreviewBindingErrorKind::Runtime,
                "realtime preview scheduler lock poisoned",
            )
        })?;
        action(&mut scheduler)
    }

    fn playback_snapshot(
        &self,
        session_id: &str,
    ) -> Result<BindingPlaybackSnapshot, RealtimePreviewBindingError> {
        self.scheduler_session(session_id)?
            .snapshot
            .lock()
            .map(|snapshot| snapshot.clone())
            .map_err(|_| {
                RealtimePreviewBindingError::new(
                    RealtimePreviewBindingErrorKind::Runtime,
                    "realtime preview snapshot lock poisoned",
                )
            })
    }

    fn stop_playback_driver(&mut self, session_id: &str) {
        if let Some(session) = self.schedulers.get_mut(session_id) {
            session.stop_playback_driver();
        }
    }

    fn cancel_playback_driver(&mut self, session_id: &str) {
        if let Some(session) = self.schedulers.get_mut(session_id) {
            session.cancel_playback_driver();
        }
    }

    fn present_scheduler_still_frame(
        &mut self,
        session_id: &str,
        runtime_id: PreviewSessionId,
        playback_generation: PlaybackGeneration,
        target_time: Microseconds,
        mode: PreviewRequestMode,
    ) -> Result<(), RealtimePreviewBindingError> {
        let scheduler = self.scheduler_handle(session_id)?;
        run_scheduler_still_frame_present(
            Arc::clone(&self.runtime),
            runtime_id,
            scheduler,
            playback_generation,
            target_time,
            mode,
            session_id.to_owned(),
            self.event_sink.clone(),
        );
        Ok(())
    }

    fn start_playback_driver(
        &mut self,
        session_id: &str,
        runtime_id: PreviewSessionId,
        playback_generation: PlaybackGeneration,
        start_time: Microseconds,
        cadence: RealtimePlaybackCadence,
    ) -> Result<(), RealtimePreviewBindingError> {
        let scheduler = self.scheduler_handle(session_id)?;
        let driver = PreviewPlaybackTaskDriver::spawn(
            Arc::clone(&self.runtime),
            runtime_id,
            scheduler,
            playback_generation,
            start_time,
            cadence,
            session_id.to_owned(),
            self.event_sink.clone(),
        )?;
        let session = self
            .schedulers
            .get_mut(session_id)
            .ok_or_else(|| RealtimePreviewBindingError::unknown_session(session_id))?;
        session.playback_driver = Some(driver);
        Ok(())
    }
}

impl Drop for RealtimePreviewBindingRegistry {
    fn drop(&mut self) {
        for session in self.schedulers.values_mut() {
            session.stop_playback_driver();
        }
    }
}

fn run_scheduler_playback_driver(
    runtime: Arc<Mutex<RealtimePreviewRuntime>>,
    runtime_id: PreviewSessionId,
    scheduler: Arc<Mutex<RealtimePreviewBindingScheduler>>,
    playback_generation: PlaybackGeneration,
    start_time: Microseconds,
    cadence: RealtimePlaybackCadence,
    event_session_id: String,
    event_sink: RealtimePreviewEventSink,
    stop: Arc<AtomicBool>,
) {
    let playback_started_at = Instant::now();
    let first_frame = {
        let mut scheduler = match scheduler.lock() {
            Ok(scheduler) => scheduler,
            Err(_) => return,
        };
        let frame_started = Instant::now();
        match scheduler.present_next_tick(
            start_time,
            playback_generation,
            PreviewRequestMode::FirstFrame,
        ) {
            Ok(presentation) => {
                if let Ok(mut runtime) = runtime.lock() {
                    let _ = runtime.record_scheduler_telemetry(runtime_id, &presentation.telemetry);
                }
                let Some(evidence) = presentation.evidence else {
                    return;
                };
                scheduler.start_playback_after_prewarm(
                    start_time,
                    playback_generation,
                    cadence,
                    playback_started_at,
                );
                let presented_at = Instant::now();
                Ok((
                    evidence,
                    u64::try_from(frame_started.elapsed().as_millis()).unwrap_or(u64::MAX),
                    presented_at,
                ))
            }
            Err(error) => {
                scheduler.publish_snapshot(
                    scheduler.evidence().cloned(),
                    scheduler.surface_placement(),
                    Some(error.to_string()),
                );
                event_sink.emit(RealtimePreviewBindingEvent::error(
                    &event_session_id,
                    playback_generation,
                    error.to_string(),
                ));
                Err(error.to_string())
            }
        }
    };

    let mut last_presented_at = match first_frame {
        Ok((evidence, render_duration_ms, presented_at)) => {
            if let Ok(mut runtime) = runtime.lock() {
                let _ = runtime.record_presented_output(
                    runtime_id,
                    playback_generation,
                    Microseconds::new(evidence.target_time_microseconds),
                    render_duration_ms,
                    0,
                    RealtimePreviewFramePacingSample {
                        target_time_microseconds: evidence.target_time_microseconds,
                        interval_ms: None,
                        schedule_lateness_ms: 0,
                        render_duration_ms,
                        dropped_frame_count: 0,
                    },
                );
            }
            event_sink.emit(RealtimePreviewBindingEvent::presented(
                &event_session_id,
                playback_generation,
                evidence.target_time_microseconds,
                0,
            ));
            presented_at
        }
        Err(_) => {
            if let Ok(mut runtime) = runtime.lock() {
                let _ = runtime.pause(runtime_id);
            }
            return;
        }
    };

    while !stop.load(Ordering::Acquire) {
        let frame = {
            let mut scheduler = match scheduler.lock() {
                Ok(scheduler) => scheduler,
                Err(_) => break,
            };
            let Some(due_tick) = scheduler.playback_timeline.due_tick(playback_generation) else {
                drop(scheduler);
                thread::sleep(Duration::from_millis(1));
                continue;
            };

            let frame_started = Instant::now();
            match scheduler.present_playback_tick(playback_generation, due_tick) {
                Ok(scheduled) => {
                    if let Ok(mut runtime) = runtime.lock() {
                        let _ =
                            runtime.record_scheduler_telemetry(runtime_id, &scheduled.telemetry);
                    }
                    if let Some(frame) = scheduled.frame {
                        let render_duration_ms =
                            u64::try_from(frame_started.elapsed().as_millis()).unwrap_or(u64::MAX);
                        let presented_at = Instant::now();
                        Ok(Some((frame, render_duration_ms, presented_at)))
                    } else {
                        Ok(None)
                    }
                }
                Err(error) => {
                    let transient = is_transient_playback_presentation_error(&error);
                    if !transient {
                        scheduler.pause_playback();
                    }
                    scheduler.publish_snapshot(
                        scheduler.evidence().cloned(),
                        scheduler.surface_placement(),
                        Some(error.to_string()),
                    );
                    if !transient {
                        event_sink.emit(RealtimePreviewBindingEvent::error(
                            &event_session_id,
                            playback_generation,
                            error.to_string(),
                        ));
                    }
                    if transient { Ok(None) } else { Err(error) }
                }
            }
        };

        match frame {
            Ok(Some((frame, render_duration_ms, presented_at))) => {
                if frame.evidence.presented_frames > 0 {
                    let interval_ms =
                        u64::try_from(presented_at.duration_since(last_presented_at).as_millis())
                            .unwrap_or(u64::MAX);
                    if let Ok(mut runtime) = runtime.lock() {
                        let _ = runtime.record_presented_output(
                            runtime_id,
                            playback_generation,
                            Microseconds::new(frame.evidence.target_time_microseconds),
                            render_duration_ms,
                            frame.dropped_frames,
                            RealtimePreviewFramePacingSample {
                                target_time_microseconds: frame.evidence.target_time_microseconds,
                                interval_ms: Some(interval_ms),
                                schedule_lateness_ms: frame.schedule_lateness_ms,
                                render_duration_ms,
                                dropped_frame_count: frame.dropped_frames,
                            },
                        );
                        if frame.reached_end {
                            let _ = runtime.pause(runtime_id);
                        }
                    }
                    event_sink.emit(RealtimePreviewBindingEvent::presented(
                        &event_session_id,
                        playback_generation,
                        frame.evidence.target_time_microseconds,
                        frame.dropped_frames,
                    ));
                    last_presented_at = presented_at;
                }
                if frame.reached_end {
                    if let Ok(mut scheduler) = scheduler.lock() {
                        scheduler.pause_playback();
                    }
                    let mut event = RealtimePreviewBindingEvent::new(
                        &event_session_id,
                        RealtimePreviewBindingEventKind::PlaybackEnded,
                        playback_generation.get(),
                    );
                    event.target_time_microseconds = Some(frame.evidence.target_time_microseconds);
                    event_sink.emit(event);
                    break;
                }
            }
            Ok(None) => {}
            Err(_) => {
                if let Ok(mut runtime) = runtime.lock() {
                    let _ = runtime.pause(runtime_id);
                }
                break;
            }
        }
    }
}

fn run_scheduler_still_frame_present(
    runtime: Arc<Mutex<RealtimePreviewRuntime>>,
    runtime_id: PreviewSessionId,
    scheduler: Arc<Mutex<RealtimePreviewBindingScheduler>>,
    playback_generation: PlaybackGeneration,
    target_time: Microseconds,
    mode: PreviewRequestMode,
    event_session_id: String,
    event_sink: RealtimePreviewEventSink,
) {
    let frame_started = Instant::now();
    let evidence = {
        let mut scheduler = match scheduler.lock() {
            Ok(scheduler) => scheduler,
            Err(_) => return,
        };
        if !scheduler.can_present_still_frame(playback_generation) {
            return;
        }
        match scheduler.present_still_tick(playback_generation, target_time, mode) {
            Ok(presentation) => {
                if let Ok(mut runtime) = runtime.lock() {
                    let _ = runtime.record_scheduler_telemetry(runtime_id, &presentation.telemetry);
                }
                let Some(evidence) = presentation.evidence else {
                    return;
                };
                evidence
            }
            Err(error) => {
                scheduler.publish_snapshot(
                    scheduler.evidence().cloned(),
                    scheduler.surface_placement(),
                    Some(error.to_string()),
                );
                event_sink.emit(RealtimePreviewBindingEvent::error(
                    &event_session_id,
                    playback_generation,
                    error.to_string(),
                ));
                return;
            }
        }
    };

    let render_duration_ms = u64::try_from(frame_started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let generation_still_current = runtime
        .lock()
        .ok()
        .and_then(|mut runtime| {
            let generation = runtime.clock(runtime_id).ok()?.generation();
            if generation != playback_generation {
                return Some(false);
            }
            let _ = runtime.record_presented_output(
                runtime_id,
                playback_generation,
                Microseconds::new(evidence.target_time_microseconds),
                render_duration_ms,
                0,
                RealtimePreviewFramePacingSample {
                    target_time_microseconds: evidence.target_time_microseconds,
                    interval_ms: None,
                    schedule_lateness_ms: 0,
                    render_duration_ms,
                    dropped_frame_count: 0,
                },
            );
            Some(true)
        })
        .unwrap_or(false);
    if !generation_still_current {
        return;
    }

    event_sink.emit(RealtimePreviewBindingEvent::presented(
        &event_session_id,
        playback_generation,
        evidence.target_time_microseconds,
        0,
    ));
}

struct RealtimePreviewSchedulerSession {
    scheduler: Arc<Mutex<RealtimePreviewBindingScheduler>>,
    snapshot: Arc<Mutex<BindingPlaybackSnapshot>>,
    playback_driver: Option<PreviewPlaybackTaskDriver>,
}

impl RealtimePreviewSchedulerSession {
    fn new(config: RealtimePlaybackSchedulerConfig) -> Self {
        let snapshot = Arc::new(Mutex::new(BindingPlaybackSnapshot::default()));
        Self {
            scheduler: Arc::new(Mutex::new(RealtimePreviewBindingScheduler::new(
                config,
                Arc::clone(&snapshot),
            ))),
            snapshot,
            playback_driver: None,
        }
    }

    fn stop_playback_driver(&mut self) {
        if let Some(driver) = self.playback_driver.take() {
            driver.stop();
        }
    }

    fn cancel_playback_driver(&mut self) {
        if let Some(driver) = self.playback_driver.take() {
            driver.cancel();
        }
    }
}

#[derive(Default, Clone)]
struct BindingPlaybackSnapshot {
    evidence: Option<RealtimePlaybackSchedulerEvidence>,
    surface_placement: Option<NativePreviewSurfacePlacementEvidence>,
    unsupported_reason: Option<String>,
}

struct ScheduledPreviewPresentation {
    evidence: Option<RealtimePlaybackSchedulerEvidence>,
    telemetry: SchedulerTelemetrySnapshot,
}

struct ScheduledPlaybackFrame {
    frame: Option<RealtimePlaybackPresentedFrame>,
    telemetry: SchedulerTelemetrySnapshot,
}

struct PreviewPlaybackTaskDriver {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl PreviewPlaybackTaskDriver {
    fn spawn(
        runtime: Arc<Mutex<RealtimePreviewRuntime>>,
        runtime_id: PreviewSessionId,
        scheduler: Arc<Mutex<RealtimePreviewBindingScheduler>>,
        playback_generation: PlaybackGeneration,
        start_time: Microseconds,
        cadence: RealtimePlaybackCadence,
        event_session_id: String,
        event_sink: RealtimePreviewEventSink,
    ) -> Result<Self, RealtimePreviewBindingError> {
        let stop = Arc::new(AtomicBool::new(false));
        let driver_stop = Arc::clone(&stop);
        let handle = thread::Builder::new()
            .name(format!("task-runtime-preview-driver-{event_session_id}"))
            .spawn(move || {
                run_scheduler_playback_driver(
                    runtime,
                    runtime_id,
                    scheduler,
                    playback_generation,
                    start_time,
                    cadence,
                    event_session_id,
                    event_sink,
                    driver_stop,
                );
            })
            .map_err(|error| {
                RealtimePreviewBindingError::new(
                    RealtimePreviewBindingErrorKind::Runtime,
                    format!("failed to start realtime preview scheduler driver: {error}"),
                )
            })?;
        Ok(Self {
            stop,
            handle: Some(handle),
        })
    }

    fn cancel(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    fn stop(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

struct RealtimePreviewBindingScheduler {
    scheduler: RealtimePlaybackScheduler,
    task_scheduler: task_runtime::JobScheduler,
    snapshot: Arc<Mutex<BindingPlaybackSnapshot>>,
    gpu_device: Option<RealtimePreviewGpuDevice>,
    surface_target: Option<RealtimePreviewGpuPresentationTarget>,
    surface_placement: Option<NativePreviewSurfacePlacementEvidence>,
    draft_snapshot: Option<Draft>,
    bundle_path: Option<PathBuf>,
    media_pipeline_id: u64,
    last_evidence: Option<RealtimePlaybackSchedulerEvidence>,
    playback_timeline: RealtimePlaybackTimeline,
    active_generation: PlaybackGeneration,
    task_scheduler_started_at: Instant,
    next_task_job_id: u64,
    #[cfg(test)]
    test_mock_surface_attached: bool,
}

impl RealtimePreviewBindingScheduler {
    fn new(
        config: RealtimePlaybackSchedulerConfig,
        snapshot: Arc<Mutex<BindingPlaybackSnapshot>>,
    ) -> Self {
        Self {
            scheduler: RealtimePlaybackScheduler::new(config),
            task_scheduler: task_runtime::JobScheduler::new(TaskRuntimeConfig::portable_default()),
            snapshot,
            gpu_device: None,
            surface_target: None,
            surface_placement: None,
            draft_snapshot: None,
            bundle_path: None,
            media_pipeline_id: next_scheduler_media_pipeline_id(),
            last_evidence: None,
            playback_timeline: RealtimePlaybackTimeline::new(),
            active_generation: PlaybackGeneration::initial(),
            task_scheduler_started_at: Instant::now(),
            next_task_job_id: 1,
            #[cfg(test)]
            test_mock_surface_attached: false,
        }
    }

    fn set_active_generation(&mut self, playback_generation: PlaybackGeneration) {
        self.active_generation = playback_generation;
    }

    fn attach_surface(
        &mut self,
        descriptor: PreviewSurfaceDescriptor,
    ) -> Result<(), RealtimePreviewBindingError> {
        self.update_scheduler_preview_dimensions_for_surface(descriptor);
        #[cfg(test)]
        if matches!(
            descriptor,
            PreviewSurfaceDescriptor::NativeChild {
                parent_window_handle: NativeParentWindowHandle::Mock(_),
                ..
            }
        ) {
            self.surface_target = None;
            self.gpu_device = None;
            self.surface_placement = None;
            self.reset_media_pipeline();
            self.test_mock_surface_attached = true;
            self.publish_snapshot(None, self.surface_placement(), None);
            return Ok(());
        }
        if std::env::var("VIDEO_EDITOR_TEST_DISABLE_RENDER_GRAPH_COMPOSITOR")
            .ok()
            .as_deref()
            == Some("1")
        {
            self.surface_target = None;
            self.gpu_device = None;
            self.surface_placement = None;
            self.reset_media_pipeline();
            #[cfg(test)]
            {
                self.test_mock_surface_attached = false;
            }
            self.publish_snapshot(None, self.surface_placement(), None);
            return Ok(());
        }
        let device = RealtimePreviewGpuDevice::bootstrap(RealtimePreviewGpuDeviceDescriptor {
            backend: RealtimePreviewGpuBackend::Auto,
            label: Some("desktop-realtime-preview-scheduler".to_owned()),
        })
        .map_err(|error| {
            RealtimePreviewBindingError::new(
                RealtimePreviewBindingErrorKind::Runtime,
                format!("realtime preview GPU bootstrap failed: {error}"),
            )
        })?;
        let target = device
            .create_presentation_target(descriptor, RealtimePreviewTargetFormat::Bgra8UnormSrgb)
            .map_err(|error| {
                RealtimePreviewBindingError::new(
                    RealtimePreviewBindingErrorKind::RuntimeSurface,
                    format!("render graph GPU presentation target unavailable: {error}"),
                )
            })?;
        self.surface_placement = native_surface_placement_from_runtime(&target);
        self.gpu_device = Some(device);
        self.surface_target = Some(target);
        self.reset_media_pipeline();
        #[cfg(test)]
        {
            self.test_mock_surface_attached = false;
        }
        self.publish_snapshot(None, self.surface_placement(), None);
        Ok(())
    }

    fn update_surface_bounds(
        &mut self,
        bounds: PreviewSurfaceBounds,
    ) -> Result<(), RealtimePreviewBindingError> {
        if let (Some(device), Some(target)) =
            (self.gpu_device.as_ref(), self.surface_target.as_mut())
        {
            device
                .resize_presentation_target(target, bounds)
                .map_err(|error| {
                    RealtimePreviewBindingError::new(
                        RealtimePreviewBindingErrorKind::RuntimeSurface,
                        format!("render graph GPU presentation target resize failed: {error}"),
                    )
                })?;
            self.surface_placement = native_surface_placement_from_runtime(target);
            let descriptor = target.descriptor();
            self.update_scheduler_preview_dimensions_for_surface(descriptor);
        } else {
            self.surface_placement = None;
            self.scheduler.update_preview_dimensions(OutputDimensions {
                width: bounds.width,
                height: bounds.height,
            });
        }
        self.last_evidence = None;
        self.publish_snapshot(None, self.surface_placement(), None);
        Ok(())
    }

    fn update_scheduler_preview_dimensions_for_surface(
        &mut self,
        descriptor: PreviewSurfaceDescriptor,
    ) {
        let (width, height) = descriptor.presentation_size();
        self.scheduler
            .update_preview_dimensions(OutputDimensions { width, height });
    }

    fn update_draft_snapshot(
        &mut self,
        draft: Draft,
        bundle_path: Option<PathBuf>,
        selected_segment: Option<RealtimePreviewSelectedSegmentBinding>,
    ) {
        self.scheduler.update_draft_snapshot(draft.clone());
        self.scheduler
            .update_selected_segment(selected_segment.map(|selected| {
                RealtimePlaybackSelectedSegment {
                    track_id: selected.track_id,
                    segment_id: selected.segment_id,
                }
            }));
        self.draft_snapshot = Some(draft);
        self.bundle_path = bundle_path;
        self.reset_media_pipeline();
        self.last_evidence = None;
        self.playback_timeline.reset();
        self.publish_snapshot(None, self.surface_placement(), None);
    }

    fn seek(&mut self, target_time: Microseconds) {
        self.playback_timeline.seek(target_time);
        self.last_evidence = None;
        self.publish_snapshot(None, self.surface_placement(), None);
    }

    fn detach_surface(&mut self) {
        self.surface_target = None;
        self.surface_placement = None;
        self.reset_media_pipeline();
        self.last_evidence = None;
        self.playback_timeline.pause();
        self.publish_snapshot(None, None, None);
        #[cfg(test)]
        {
            self.test_mock_surface_attached = false;
        }
    }

    fn start_playback_after_prewarm(
        &mut self,
        start_time: Microseconds,
        playback_generation: PlaybackGeneration,
        cadence: RealtimePlaybackCadence,
        started_at: Instant,
    ) {
        self.playback_timeline.start_after_prewarm_at(
            start_time,
            playback_generation,
            self.sequence_duration(),
            cadence,
            started_at,
        );
    }

    fn pause_playback(&mut self) {
        self.playback_timeline.pause();
    }

    fn stop_playback(&mut self) {
        self.playback_timeline.stop();
    }

    fn evidence(&self) -> Option<&RealtimePlaybackSchedulerEvidence> {
        self.last_evidence
            .as_ref()
            .or_else(|| self.scheduler.last_evidence())
    }

    fn surface_placement(&self) -> Option<NativePreviewSurfacePlacementEvidence> {
        self.surface_placement.clone()
    }

    fn validate_playback_ready(&self) -> Result<(), RealtimePreviewBindingError> {
        #[cfg(not(test))]
        if self.gpu_device.is_none() || self.surface_target.is_none() {
            return Err(RealtimePreviewBindingError::presenter(
                NativePreviewPresenterError::new(
                    "render graph GPU compositor scheduler presentation surface is not attached",
                ),
            ));
        }
        #[cfg(test)]
        if !self.test_mock_surface_attached
            && (self.gpu_device.is_none() || self.surface_target.is_none())
        {
            return Err(RealtimePreviewBindingError::presenter(
                NativePreviewPresenterError::new(
                    "render graph GPU compositor scheduler presentation surface is not attached",
                ),
            ));
        }
        if self.draft_snapshot.is_none() {
            return Err(RealtimePreviewBindingError::presenter(
                NativePreviewPresenterError::new(
                    "accepted draft snapshot is required before scheduler playback",
                ),
            ));
        }
        Ok(())
    }

    fn can_present_still_frame(&self, playback_generation: PlaybackGeneration) -> bool {
        if self.active_generation != playback_generation || self.draft_snapshot.is_none() {
            return false;
        }
        #[cfg(test)]
        {
            self.test_mock_surface_attached
                || (self.gpu_device.is_some() && self.surface_target.is_some())
        }
        #[cfg(not(test))]
        {
            self.gpu_device.is_some() && self.surface_target.is_some()
        }
    }

    fn publish_snapshot(
        &self,
        evidence: Option<RealtimePlaybackSchedulerEvidence>,
        surface_placement: Option<NativePreviewSurfacePlacementEvidence>,
        unsupported_reason: Option<String>,
    ) {
        if let Ok(mut snapshot) = self.snapshot.lock() {
            snapshot.evidence = evidence;
            snapshot.surface_placement = surface_placement;
            snapshot.unsupported_reason = unsupported_reason;
        }
    }

    fn scheduler_now_us(&self) -> u64 {
        u64::try_from(self.task_scheduler_started_at.elapsed().as_micros()).unwrap_or(u64::MAX)
    }

    fn next_task_job_id(&mut self, prefix: &str) -> JobId {
        let sequence = self.next_task_job_id;
        self.next_task_job_id = self.next_task_job_id.saturating_add(1);
        JobId::new(format!("{prefix}-{sequence}"))
    }

    fn preview_job_policy(mode: PreviewRequestMode) -> (JobDomain, JobPriority) {
        match mode {
            PreviewRequestMode::Seek | PreviewRequestMode::Scrub => {
                (JobDomain::ScrubSeek, JobPriority::Interactive)
            }
            PreviewRequestMode::FirstFrame => {
                (JobDomain::InteractivePreview, JobPriority::Interactive)
            }
            PreviewRequestMode::PlaybackTick => {
                (JobDomain::InteractivePreview, JobPriority::Realtime)
            }
        }
    }

    fn admit_preview_job(
        &mut self,
        target_time: Microseconds,
        playback_generation: PlaybackGeneration,
        mode: PreviewRequestMode,
    ) -> Result<JobId, RealtimePreviewBindingError> {
        let (domain, priority) = Self::preview_job_policy(mode);
        let submitted_at_us = self.scheduler_now_us();
        let job_id = self.next_task_job_id(match mode {
            PreviewRequestMode::Seek => "preview-seek",
            PreviewRequestMode::Scrub => "preview-scrub",
            PreviewRequestMode::FirstFrame => "preview-first-frame",
            PreviewRequestMode::PlaybackTick => "preview-playback-tick",
        });
        let token = TaskCancellationToken::new(self.next_task_job_id);
        let envelope = JobEnvelope::new(
            job_id.clone(),
            domain,
            priority,
            ResourceClass::GpuPresent,
            token,
            submitted_at_us,
        )
        .with_freshness(JobFreshness::timeline(target_time, playback_generation))
        .with_deadline_at_us(submitted_at_us.saturating_add(match priority {
            JobPriority::Realtime => 16_667,
            JobPriority::Interactive => 33_333,
            JobPriority::UserVisible | JobPriority::Background | JobPriority::Maintenance => {
                100_000
            }
        }));
        self.task_scheduler
            .submit(envelope)
            .map_err(task_scheduler_error)?;
        let started = self
            .task_scheduler
            .start_next(self.scheduler_now_us())
            .map_err(task_scheduler_error)?;
        match started {
            Some(started) if started.job_id == job_id => Ok(job_id),
            Some(started) => Err(RealtimePreviewBindingError::new(
                RealtimePreviewBindingErrorKind::Runtime,
                format!(
                    "task scheduler admitted unexpected preview job {} while waiting for {}",
                    started.job_id.as_str(),
                    job_id.as_str()
                ),
            )),
            None => Err(RealtimePreviewBindingError::new(
                RealtimePreviewBindingErrorKind::Runtime,
                "task scheduler could not start realtime preview GPU presentation job",
            )),
        }
    }

    fn complete_preview_job(
        &mut self,
        job_id: JobId,
        result: JobResult,
        evidence: Option<&RealtimePlaybackSchedulerEvidence>,
    ) -> Result<ScheduledPreviewPresentation, RealtimePreviewBindingError> {
        let mut accepted = false;
        let completion = self
            .task_scheduler
            .complete_with_commit(
                &job_id,
                result,
                self.scheduler_now_us(),
                CompletionFreshness::playback_generation(self.active_generation),
                |_| accepted = true,
            )
            .map_err(task_scheduler_error)?;
        let telemetry = self.task_scheduler.telemetry_snapshot();
        let evidence = if matches!(completion, JobCompletion::Accepted { .. }) && accepted {
            evidence.cloned()
        } else {
            None
        };
        if let Some(evidence) = evidence.as_ref() {
            self.commit_presented_evidence(evidence.clone());
        }
        if matches!(
            completion,
            JobCompletion::Cancelled { .. } | JobCompletion::StaleRejected { .. }
        ) {
            return Ok(ScheduledPreviewPresentation {
                evidence: None,
                telemetry,
            });
        }
        Ok(ScheduledPreviewPresentation {
            evidence,
            telemetry,
        })
    }

    fn commit_presented_evidence(&mut self, evidence: RealtimePlaybackSchedulerEvidence) {
        self.last_evidence = Some(evidence.clone());
        self.publish_snapshot(Some(evidence), self.surface_placement(), None);
    }

    fn present_next_tick(
        &mut self,
        target_time: Microseconds,
        playback_generation: PlaybackGeneration,
        mode: PreviewRequestMode,
    ) -> Result<ScheduledPreviewPresentation, RealtimePreviewBindingError> {
        let job_id = self.admit_preview_job(target_time, playback_generation, mode)?;
        let frame_started = Instant::now();
        let evidence = match self.render_next_tick(target_time, playback_generation) {
            Ok(evidence) => evidence,
            Err(error) => {
                let _ = self.complete_preview_job(
                    job_id.clone(),
                    JobResult::new(job_id, JobResultKind::Failed),
                    None,
                );
                return Err(error);
            }
        };
        let mut result = JobResult::completed(job_id.clone());
        if mode == PreviewRequestMode::FirstFrame {
            result = result.with_first_frame_time_us(
                u64::try_from(frame_started.elapsed().as_micros()).unwrap_or(u64::MAX),
            );
        }
        self.complete_preview_job(job_id, result, Some(&evidence))
    }

    fn render_next_tick(
        &mut self,
        target_time: Microseconds,
        playback_generation: PlaybackGeneration,
    ) -> Result<RealtimePlaybackSchedulerEvidence, RealtimePreviewBindingError> {
        if self.active_generation != playback_generation {
            return Err(RealtimePreviewBindingError::presenter(
                NativePreviewPresenterError::new(
                    "render graph GPU compositor skipped stale realtime preview generation",
                ),
            ));
        }
        #[cfg(not(test))]
        if self.gpu_device.is_none() || self.surface_target.is_none() {
            return Err(RealtimePreviewBindingError::presenter(
                NativePreviewPresenterError::new(
                    "render graph GPU compositor scheduler presentation surface is not attached",
                ),
            ));
        }
        #[cfg(test)]
        if !self.test_mock_surface_attached
            && (self.gpu_device.is_none() || self.surface_target.is_none())
        {
            return Err(RealtimePreviewBindingError::presenter(
                NativePreviewPresenterError::new(
                    "render graph GPU compositor scheduler presentation surface is not attached",
                ),
            ));
        }
        let tick_time = target_time;
        #[cfg(test)]
        if self.test_mock_surface_attached {
            let mut presenter = BindingSchedulerTestPresenter;
            let evidence = self
                .scheduler
                .present_tick(tick_time, playback_generation, &mut presenter)
                .map_err(scheduler_error)?;
            return Ok(evidence);
        }
        self.ensure_media_provider()?;
        let media_pipeline_id = self.media_pipeline_id;
        let mut presenter = BindingSchedulerPresenter {
            gpu_device: self.gpu_device.clone(),
            surface_target: self.surface_target.as_mut(),
            media_pipeline_id,
        };
        let evidence =
            match self
                .scheduler
                .present_tick(tick_time, playback_generation, &mut presenter)
            {
                Ok(evidence) => evidence,
                Err(error) => {
                    let error = scheduler_error(error);
                    self.publish_snapshot(
                        self.last_evidence.clone(),
                        self.surface_placement(),
                        Some(error.to_string()),
                    );
                    return Err(error);
                }
            };
        Ok(evidence)
    }

    fn present_still_tick(
        &mut self,
        playback_generation: PlaybackGeneration,
        target_time: Microseconds,
        mode: PreviewRequestMode,
    ) -> Result<ScheduledPreviewPresentation, RealtimePreviewBindingError> {
        self.present_next_tick(target_time, playback_generation, mode)
    }

    fn present_playback_tick(
        &mut self,
        playback_generation: PlaybackGeneration,
        due_tick: RealtimePlaybackDueTick,
    ) -> Result<ScheduledPlaybackFrame, RealtimePreviewBindingError> {
        let presentation = self.present_next_tick(
            due_tick.target_time,
            playback_generation,
            PreviewRequestMode::PlaybackTick,
        )?;
        let frame = presentation.evidence.map(|evidence| {
            self.playback_timeline
                .advance_after_presented_tick(due_tick);
            RealtimePlaybackPresentedFrame {
                evidence,
                dropped_frames: due_tick.dropped_frames,
                schedule_lateness_ms: due_tick.schedule_lateness_ms,
                reached_end: due_tick.reaches_sequence_end,
            }
        });
        Ok(ScheduledPlaybackFrame {
            frame,
            telemetry: presentation.telemetry,
        })
    }

    fn sequence_duration(&self) -> Microseconds {
        self.draft_snapshot
            .as_ref()
            .map(sequence_duration)
            .unwrap_or(Microseconds::ZERO)
    }

    fn ensure_media_provider(&mut self) -> Result<(), RealtimePreviewBindingError> {
        let media_pipeline_id = self.media_pipeline_id;
        let exists = SCHEDULER_MEDIA_PIPELINES
            .with(|pipelines| pipelines.borrow().contains_key(&media_pipeline_id));
        if !exists {
            let pipeline = self.build_media_pipeline()?;
            SCHEDULER_MEDIA_PIPELINES.with(|pipelines| {
                pipelines.borrow_mut().insert(media_pipeline_id, pipeline);
            });
        }
        Ok(())
    }

    fn reset_media_pipeline(&mut self) {
        let previous_id = self.media_pipeline_id;
        SCHEDULER_MEDIA_PIPELINES.with(|pipelines| {
            pipelines.borrow_mut().remove(&previous_id);
        });
        self.media_pipeline_id = next_scheduler_media_pipeline_id();
    }

    fn build_media_pipeline(&self) -> Result<SchedulerMediaPipeline, RealtimePreviewBindingError> {
        let registry = NativeTextureLeaseRegistry::new();
        Ok(SchedulerMediaPipeline {
            provider: self.build_media_provider(registry.clone())?,
            texture_cache: RealtimePreviewTextureCache::new()
                .with_native_texture_registry(registry.clone())
                .with_native_texture_importer(Box::new(import_native_nv12_external_texture)),
            compositor: None,
            in_flight_presentations: VecDeque::new(),
        })
    }

    fn build_media_provider(
        &self,
        registry: NativeTextureLeaseRegistry,
    ) -> Result<SchedulerFrameProvider, RealtimePreviewBindingError> {
        let draft = self.draft_snapshot.as_ref().ok_or_else(|| {
            RealtimePreviewBindingError::presenter(NativePreviewPresenterError::new(
                "accepted draft snapshot is required before scheduler playback",
            ))
        })?;
        let preview_device = preview_runtime_device_id()?;
        let mut provider = platform_media_provider(preview_device.clone(), registry.clone())?
            .with_desired_storage(PreviewFrameStoragePreference::Texture)
            .with_preview_device_context(PreviewDecodeDeviceContext::compatible(preview_device))
            .with_native_texture_registry(registry);
        let mut static_images = BTreeMap::new();
        let mut registered_video_count = 0usize;
        for material in &draft.materials {
            match material.kind {
                MaterialKind::Video if material.metadata.has_video => {
                    let material_uri = resolve_scheduler_material_path(
                        self.bundle_path.as_deref(),
                        &material.uri,
                    )?;
                    provider
                        .register_material(PreviewMaterialDecodeSource {
                            material_id: material.material_id.clone(),
                            material_uri,
                            stream_id: StreamId(0),
                            selected_path: SelectedDecodePath::NativeHardwareTexture,
                            fallback_selection: None,
                        })
                        .map_err(|error| {
                            RealtimePreviewBindingError::presenter(
                                NativePreviewPresenterError::new(error.to_string()),
                            )
                        })?;
                    registered_video_count = registered_video_count.saturating_add(1);
                }
                MaterialKind::Image
                    if material.metadata.width.is_some() && material.metadata.height.is_some() =>
                {
                    let material_uri = resolve_scheduler_material_path(
                        self.bundle_path.as_deref(),
                        &material.uri,
                    )?;
                    static_images.insert(
                        material.material_id.clone(),
                        load_static_image_frame(material.material_id.clone(), &material_uri)?,
                    );
                }
                _ => {}
            }
        }
        if registered_video_count == 0 && static_images.is_empty() {
            return Err(RealtimePreviewBindingError::presenter(
                NativePreviewPresenterError::new(
                    "no visual material is registered for scheduler media IO",
                ),
            ));
        }
        Ok(SchedulerFrameProvider::new(provider, static_images))
    }
}

struct SchedulerMediaPipeline {
    provider: SchedulerFrameProvider,
    texture_cache: RealtimePreviewTextureCache,
    compositor: Option<RealtimePreviewCompositor>,
    in_flight_presentations: VecDeque<InFlightSurfacePresentation>,
}

struct InFlightSurfacePresentation {
    fence: RealtimePreviewSurfaceSubmissionFence,
    frame_releases: Vec<PendingPreviewFrameRelease>,
}

impl SchedulerMediaPipeline {
    fn release_completed_presentations(&mut self) -> Result<(), MediaIoHandoffReleaseError> {
        let Some(compositor) = self.compositor.as_ref() else {
            return Ok(());
        };
        compositor
            .poll_surface_submissions()
            .map_err(|source| MediaIoHandoffReleaseError {
                source: source.to_string(),
            })?;
        while self
            .in_flight_presentations
            .front()
            .map(|presentation| presentation.fence.is_complete())
            .unwrap_or(false)
        {
            let presentation = self
                .in_flight_presentations
                .pop_front()
                .expect("front presentation was checked");
            self.release_presented_frame_batch(presentation.frame_releases)?;
        }
        Ok(())
    }

    fn apply_presentation_backpressure(&mut self) -> Result<(), MediaIoHandoffReleaseError> {
        let policy = RealtimePlaybackPresentationQueuePolicy::production();
        if policy.has_capacity(self.in_flight_presentations.len()) {
            return Ok(());
        }
        let Some(compositor) = self.compositor.as_ref() else {
            return Ok(());
        };
        if let Some(presentation) = self.in_flight_presentations.front() {
            compositor
                .wait_for_surface_submission(&presentation.fence, policy.backpressure_timeout)
                .map_err(|source| MediaIoHandoffReleaseError {
                    source: source.to_string(),
                })?;
        }
        self.release_completed_presentations()
    }

    fn track_presented_frame_releases(
        &mut self,
        fence: Option<RealtimePreviewSurfaceSubmissionFence>,
        frame_releases: Vec<PendingPreviewFrameRelease>,
    ) -> Result<(), MediaIoHandoffReleaseError> {
        if frame_releases.is_empty() {
            return Ok(());
        }
        match fence {
            Some(fence) => {
                self.in_flight_presentations
                    .push_back(InFlightSurfacePresentation {
                        fence,
                        frame_releases,
                    });
                Ok(())
            }
            None => self.release_presented_frame_batch(frame_releases),
        }
    }

    fn release_all_presentations(&mut self) -> Result<(), MediaIoHandoffReleaseError> {
        if self.compositor.is_none() {
            while let Some(presentation) = self.in_flight_presentations.pop_front() {
                self.release_presented_frame_batch(presentation.frame_releases)?;
            }
            return Ok(());
        }
        while let Some(presentation) = self.in_flight_presentations.pop_front() {
            {
                let compositor = self
                    .compositor
                    .as_ref()
                    .expect("compositor was checked before draining presentations");
                compositor
                    .wait_for_surface_submission(&presentation.fence, Duration::from_secs(5))
                    .map_err(|source| MediaIoHandoffReleaseError {
                        source: source.to_string(),
                    })?;
            }
            self.release_presented_frame_batch(presentation.frame_releases)?;
        }
        Ok(())
    }

    fn release_presented_frame_batch(
        &mut self,
        frame_releases: Vec<PendingPreviewFrameRelease>,
    ) -> Result<(), MediaIoHandoffReleaseError> {
        for release in &frame_releases {
            if let Some(handle_id) = release.texture_handle_id() {
                self.texture_cache.evict_native_texture(handle_id);
            }
        }
        self.provider.release_presented_frame_batch(frame_releases)
    }
}

impl Drop for SchedulerMediaPipeline {
    fn drop(&mut self) {
        let _ = self.release_all_presentations();
    }
}

struct SchedulerFrameProvider {
    media_io: MediaIoFrameProvider,
    static_images: BTreeMap<MaterialId, StaticImageFrame>,
}

impl SchedulerFrameProvider {
    fn new(
        media_io: MediaIoFrameProvider,
        static_images: BTreeMap<MaterialId, StaticImageFrame>,
    ) -> Self {
        Self {
            media_io,
            static_images,
        }
    }

    fn take_presented_frame_releases(&mut self) -> Vec<PendingPreviewFrameRelease> {
        self.media_io.take_presented_frame_releases()
    }

    fn release_presented_frame_batch(
        &mut self,
        frame_releases: Vec<PendingPreviewFrameRelease>,
    ) -> Result<(), MediaIoHandoffReleaseError> {
        self.media_io
            .release_presented_frame_batch(frame_releases)
            .map(|_| ())
            .map_err(|source| MediaIoHandoffReleaseError {
                source: source.to_string(),
            })
    }
}

#[derive(Debug)]
struct MediaIoHandoffReleaseError {
    source: String,
}

impl fmt::Display for MediaIoHandoffReleaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to release scheduler media IO frame leases: {}",
            self.source
        )
    }
}

impl Error for MediaIoHandoffReleaseError {}

impl PreviewFrameProvider for SchedulerFrameProvider {
    fn provider_name(&self) -> &'static str {
        "scheduler-frame-provider"
    }

    fn frame_for(
        &mut self,
        material_id: &MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
    ) -> Result<PreviewFrameInput, PreviewFrameProviderError> {
        if let Some(frame) = self.static_images.get(material_id) {
            return PreviewFrameInput::static_image(
                frame.material_id.clone(),
                source_position,
                playback_generation,
                frame.width,
                frame.height,
                frame.pixels.clone(),
            )
            .map_err(|error| {
                PreviewFrameProviderError::invalid_frame(
                    self.provider_name(),
                    Some(material_id.clone()),
                    Some(source_position),
                    Some(playback_generation),
                    error,
                )
            });
        }

        self.media_io
            .frame_for(material_id, source_position, playback_generation)
    }
}

#[derive(Clone)]
struct StaticImageFrame {
    material_id: MaterialId,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

fn load_static_image_frame(
    material_id: MaterialId,
    path: &Path,
) -> Result<StaticImageFrame, RealtimePreviewBindingError> {
    let rgba = image::open(path)
        .map_err(|error| {
            RealtimePreviewBindingError::presenter(NativePreviewPresenterError::new(format!(
                "failed to decode realtime preview image material {} from {}: {error}",
                material_id.as_str(),
                path.display()
            )))
        })?
        .to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    if width == 0 || height == 0 {
        return Err(RealtimePreviewBindingError::presenter(
            NativePreviewPresenterError::new(format!(
                "realtime preview image material {} decoded to an empty frame",
                material_id.as_str()
            )),
        ));
    }

    Ok(StaticImageFrame {
        material_id,
        width,
        height,
        pixels: rgba.into_raw(),
    })
}

struct BindingSchedulerPresenter<'a> {
    gpu_device: Option<RealtimePreviewGpuDevice>,
    surface_target: Option<&'a mut RealtimePreviewGpuPresentationTarget>,
    media_pipeline_id: u64,
}

impl RealtimePlaybackSchedulerPresenter for BindingSchedulerPresenter<'_> {
    fn present_render_graph(
        &mut self,
        graph: &RenderGraph,
        target_time: Microseconds,
        playback_generation: PlaybackGeneration,
        ui_chrome: &RealtimePreviewUiChrome,
    ) -> Result<RealtimePlaybackSchedulerPresentation, RealtimePlaybackSchedulerError> {
        let gpu_device = self.gpu_device.clone().ok_or_else(|| {
            RealtimePlaybackSchedulerError::MissingPrerequisite {
                reason: "render graph GPU device is not attached".to_owned(),
            }
        })?;
        let target = self.surface_target.as_deref_mut().ok_or_else(|| {
            RealtimePlaybackSchedulerError::MissingPrerequisite {
                reason: "render graph GPU presentation surface is not attached".to_owned(),
            }
        })?;
        let output = SCHEDULER_MEDIA_PIPELINES.with(|pipelines| {
            let mut pipelines = pipelines.borrow_mut();
            let pipeline = pipelines.get_mut(&self.media_pipeline_id).ok_or_else(|| {
                RealtimePlaybackSchedulerError::MissingPrerequisite {
                    reason: "scheduler media pipeline is not initialized".to_owned(),
                }
            })?;
            pipeline
                .release_completed_presentations()
                .map_err(|error| RealtimePlaybackSchedulerError::Presentation {
                    reason: error.to_string(),
                })?;
            pipeline
                .apply_presentation_backpressure()
                .map_err(|error| RealtimePlaybackSchedulerError::Presentation {
                    reason: error.to_string(),
                })?;
            let compositor = pipeline.compositor.get_or_insert_with(|| {
                RealtimePreviewCompositor::new(
                    gpu_device,
                    RealtimePreviewCapabilityClassifier {
                        runtime_backend_available: true,
                        surface_available: true,
                        gpu_text_parity: false,
                        bundled_text_font_registry_available: true,
                    },
                )
            });
            let mut presentation = compositor.present_to_surface_with_generation(
                graph,
                target,
                &mut pipeline.provider,
                &mut pipeline.texture_cache,
                playback_generation,
                ui_chrome,
            );
            let frame_releases = pipeline.provider.take_presented_frame_releases();
            match presentation.as_mut() {
                Ok(output) => {
                    let fence = if output.presented_frames > 0 {
                        output.submission_fence.take()
                    } else {
                        None
                    };
                    pipeline
                        .track_presented_frame_releases(fence, frame_releases)
                        .map_err(|error| RealtimePlaybackSchedulerError::Presentation {
                            reason: error.to_string(),
                        })?;
                }
                Err(_) => {
                    pipeline
                        .provider
                        .release_presented_frame_batch(frame_releases)
                        .map_err(|error| RealtimePlaybackSchedulerError::Presentation {
                            reason: error.to_string(),
                        })?;
                }
            }
            presentation.map_err(|error| RealtimePlaybackSchedulerError::Presentation {
                reason: error.to_string(),
            })
        })?;
        if output.presented_frames == 0 {
            let details = output
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.reason.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(RealtimePlaybackSchedulerError::MissingPrerequisite {
                reason: format!(
                    "render graph GPU compositor produced no presented surface frame: support={:?} submittedDraws={} diagnostics={}",
                    output.support, output.submitted_draws, details
                ),
            });
        }
        Ok(RealtimePlaybackSchedulerPresentation {
            width: output.width,
            height: output.height,
            byte_count: 0,
            presented_frames: output.presented_frames,
            submitted_draws: output.submitted_draws,
            digest: compositor_digest(
                target_time,
                playback_generation,
                output.presented_frames,
                output.submitted_draws,
            ),
        })
    }
}

#[cfg(test)]
struct BindingSchedulerTestPresenter;

#[cfg(test)]
impl RealtimePlaybackSchedulerPresenter for BindingSchedulerTestPresenter {
    fn present_render_graph(
        &mut self,
        graph: &RenderGraph,
        target_time: Microseconds,
        playback_generation: PlaybackGeneration,
        _ui_chrome: &RealtimePreviewUiChrome,
    ) -> Result<RealtimePlaybackSchedulerPresentation, RealtimePlaybackSchedulerError> {
        Ok(RealtimePlaybackSchedulerPresentation {
            width: graph.canvas.width,
            height: graph.canvas.height,
            byte_count: 0,
            presented_frames: 1,
            submitted_draws: graph.video_layers.len() as u32,
            digest: compositor_digest(
                target_time,
                playback_generation,
                1,
                graph.video_layers.len() as u32,
            ),
        })
    }
}

fn next_scheduler_media_pipeline_id() -> u64 {
    NEXT_SCHEDULER_MEDIA_PIPELINE_ID.fetch_add(1, Ordering::Relaxed)
}

fn preview_runtime_device_id() -> Result<RuntimeDeviceId, RealtimePreviewBindingError> {
    #[cfg(target_os = "macos")]
    {
        return macos_system_metal_device_id().ok_or_else(|| {
            RealtimePreviewBindingError::presenter(NativePreviewPresenterError::new(
                "macOS Metal device identity is unavailable for scheduler texture interop",
            ))
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(RealtimePreviewBindingError::presenter(
            NativePreviewPresenterError::new(
                "native texture import bridge is not attached for this platform",
            ),
        ))
    }
}

fn platform_media_provider(
    preview_device: RuntimeDeviceId,
    registry: NativeTextureLeaseRegistry,
) -> Result<MediaIoFrameProvider, RealtimePreviewBindingError> {
    #[cfg(target_os = "macos")]
    {
        let reader = MacosMediaReader::new()
            .with_texture_interop_policy(MacosTextureInteropPolicy::for_preview_device(
                preview_device,
            ))
            .with_native_texture_registry(registry);
        return Ok(MediaIoFrameProvider::new(Box::new(reader)));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = preview_device;
        let _ = registry;
        Err(RealtimePreviewBindingError::presenter(
            NativePreviewPresenterError::new(
                "native texture media provider is not attached for this platform",
            ),
        ))
    }
}

fn import_native_nv12_external_texture(
    device: &wgpu::Device,
    descriptor: &TextureHandleDescriptor,
    lease: &NativeTextureLease,
) -> Result<Option<Rc<RealtimePreviewExternalTexturePlanes>>, String> {
    #[cfg(target_os = "macos")]
    {
        let Some(macos_lease) = lease.resource_as::<MacosRegisteredTextureLease>() else {
            return Ok(None);
        };
        use objc2_core_video::CVMetalTextureGetTexture;

        let luma = CVMetalTextureGetTexture(macos_lease.luma_texture())
            .ok_or_else(|| "metal:nv12:luma-plane-unavailable".to_owned())?;
        let chroma = CVMetalTextureGetTexture(macos_lease.chroma_texture())
            .ok_or_else(|| "metal:nv12:chroma-plane-unavailable".to_owned())?;
        let planes =
            RealtimePreviewGpuDevice::create_nv12_external_texture_planes_from_metal_device(
                device,
                descriptor.width,
                descriptor.height,
                luma,
                chroma,
            )
            .map_err(|error| error.to_string())?;
        return Ok(Some(Rc::new(planes)));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = device;
        let _ = descriptor;
        let _ = lease;
        Ok(None)
    }
}

fn resolve_scheduler_material_path(
    bundle_path: Option<&std::path::Path>,
    uri: &str,
) -> Result<PathBuf, RealtimePreviewBindingError> {
    let trimmed = uri.trim();
    if let Some(path) = trimmed.strip_prefix("file://") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(RealtimePreviewBindingError::presenter(
            NativePreviewPresenterError::new(format!(
                "scheduler material file URI does not resolve to a file: {trimmed}"
            )),
        ));
    }
    let bundle_path = bundle_path.ok_or_else(|| {
        RealtimePreviewBindingError::presenter(NativePreviewPresenterError::new(
            "bundlePath is required to resolve scheduler material URIs",
        ))
    })?;
    let path = resolve_material_uri(bundle_path, trimmed)
        .map_err(|error| {
            RealtimePreviewBindingError::presenter(NativePreviewPresenterError::new(
                error.to_string(),
            ))
        })?
        .ok_or_else(|| {
            RealtimePreviewBindingError::presenter(NativePreviewPresenterError::new(format!(
                "scheduler material URI is not a local file path: {trimmed}"
            )))
        })?;
    if !path.is_file() {
        return Err(RealtimePreviewBindingError::presenter(
            NativePreviewPresenterError::new(format!(
                "scheduler material path does not exist or is not a file: {}",
                path.display()
            )),
        ));
    }
    Ok(path)
}

fn native_evidence_from_scheduler(
    evidence: RealtimePlaybackSchedulerEvidence,
) -> NativePreviewContentEvidence {
    NativePreviewContentEvidence {
        source: NativePreviewContentEvidenceSource::RenderGraphGpuComposited,
        digest: evidence.digest,
        width: evidence.width,
        height: evidence.height,
        byte_count: evidence.byte_count,
        target_time_microseconds: evidence.target_time_microseconds,
        presented_frames: evidence.presented_frames,
        submitted_draws: evidence.submitted_draws,
        active_text_overlays: evidence
            .active_text_overlays
            .into_iter()
            .map(
                |text| crate::native_preview_presenter::NativePreviewTextOverlayEvidence {
                    selection_handle: timeline_segment_selection_handle(
                        &draft_model::TrackId::new(text.track_id.clone()),
                        &draft_model::SegmentId::new(text.segment_id.clone()),
                    ),
                    track_id: text.track_id,
                    segment_id: text.segment_id,
                    source: text.source,
                    content: text.content,
                    font_family: text.font_family,
                    font_ref: text.font_ref,
                    font_size: text.font_size,
                    color: text.color,
                    alignment: text.alignment,
                    line_height_millis: text.line_height_millis,
                    letter_spacing_millis: text.letter_spacing_millis,
                    x: text.x,
                    y: text.y,
                    width: text.width,
                    height: text.height,
                    visual_position_x: text.visual_position_x,
                    visual_position_y: text.visual_position_y,
                    visual_scale_x_millis: text.visual_scale_x_millis,
                    visual_scale_y_millis: text.visual_scale_y_millis,
                    visual_rotation_degrees: text.visual_rotation_degrees,
                    visual_opacity_millis: text.visual_opacity_millis,
                    selected: text.selected,
                },
            )
            .collect(),
    }
}

fn transformed_text_overlay_contains(
    text: &RealtimePlaybackTextOverlayEvidence,
    target_width: u32,
    target_height: u32,
    x: u32,
    y: u32,
) -> bool {
    let target_width = f64::from(target_width.max(1));
    let target_height = f64::from(target_height.max(1));
    let scale_x = f64::from(text.visual_scale_x_millis.max(1)) / 1000.0;
    let scale_y = f64::from(text.visual_scale_y_millis.max(1)) / 1000.0;
    let width = f64::from(text.width.max(1)) * scale_x;
    let height = f64::from(text.height.max(1)) * scale_y;
    let center_x = f64::from(text.x)
        + f64::from(text.width) / 2.0
        + (target_width * f64::from(text.visual_position_x)) / 2000.0;
    let center_y = f64::from(text.y) + f64::from(text.height) / 2.0
        - (target_height * f64::from(text.visual_position_y)) / 2000.0;
    let radians = f64::from(text.visual_rotation_degrees).to_radians();
    let rotated_width = width * radians.cos().abs() + height * radians.sin().abs();
    let rotated_height = width * radians.sin().abs() + height * radians.cos().abs();
    let left = center_x - rotated_width / 2.0;
    let top = center_y - rotated_height / 2.0;
    let right = center_x + rotated_width / 2.0;
    let bottom = center_y + rotated_height / 2.0;
    let x = f64::from(x);
    let y = f64::from(y);
    x >= left && x < right && y >= top && y < bottom
}

fn native_surface_placement_from_runtime(
    target: &RealtimePreviewGpuPresentationTarget,
) -> Option<NativePreviewSurfacePlacementEvidence> {
    target
        .screen_rect()
        .map(|rect| NativePreviewSurfacePlacementEvidence {
            native_screen_rect: NativePreviewScreenRect {
                x: rect.x.round() as i32,
                y: rect.y.round() as i32,
                width: rect.width.round() as i32,
                height: rect.height.round() as i32,
            },
            drawable_lifecycle_diagnostic: target.drawable_lifecycle_diagnostic(),
        })
}

fn sequence_duration(draft: &Draft) -> Microseconds {
    draft
        .tracks
        .iter()
        .flat_map(|track| track.segments.iter())
        .filter_map(|segment| segment.target_timerange.checked_end())
        .max()
        .unwrap_or(Microseconds::ZERO)
}

fn scheduler_error(error: RealtimePlaybackSchedulerError) -> RealtimePreviewBindingError {
    RealtimePreviewBindingError::presenter(NativePreviewPresenterError::new(format!(
        "render graph GPU scheduler failed: {error}"
    )))
}

fn task_scheduler_error(error: impl fmt::Display) -> RealtimePreviewBindingError {
    RealtimePreviewBindingError::new(
        RealtimePreviewBindingErrorKind::Runtime,
        format!("task runtime scheduler rejected realtime preview job: {error}"),
    )
}

fn is_transient_playback_presentation_error(error: &RealtimePreviewBindingError) -> bool {
    let message = error.message();
    message.contains("wgpu surface texture acquire failed: surface is occluded")
        || message.contains("wgpu surface texture acquire failed: surface acquire timed out")
}

fn compositor_digest(
    target_time: Microseconds,
    playback_generation: PlaybackGeneration,
    presented_frames: u32,
    submitted_draws: u32,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&target_time.get().to_le_bytes());
    hasher.update(&playback_generation.get().to_le_bytes());
    hasher.update(&presented_frames.to_le_bytes());
    hasher.update(&submitted_draws.to_le_bytes());
    hasher.finalize().to_hex().to_string()
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_handle_hex: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor_millis: u32,
}

impl RealtimePreviewSurfaceBindingDescriptor {
    fn native_parent_handle(&self) -> Result<Option<u64>, RealtimePreviewBindingError> {
        if let Some(hex) = self.parent_handle_hex.as_deref() {
            let trimmed = hex.trim();
            let trimmed = trimmed.strip_prefix("0x").unwrap_or(trimmed);
            if trimmed.is_empty() {
                return Ok(None);
            }
            return u64::from_str_radix(trimmed, 16).map(Some).map_err(|error| {
                RealtimePreviewBindingError::new(
                    RealtimePreviewBindingErrorKind::InvalidPayload,
                    format!("native parent handle hex is invalid: {error}"),
                )
            });
        }
        Ok(self.parent_handle)
    }

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
        let parent_handle = self.native_parent_handle()?.unwrap_or_default();
        let descriptor = match self.kind {
            RealtimePreviewSurfaceBindingKind::WindowsHwnd => {
                PreviewSurfaceDescriptor::NativeChild {
                    parent_window_handle: NativeParentWindowHandle::WindowsHwnd(parent_handle),
                    bounds,
                }
            }
            RealtimePreviewSurfaceBindingKind::MacosNsView => {
                PreviewSurfaceDescriptor::NativeChild {
                    parent_window_handle: NativeParentWindowHandle::MacosNsView(parent_handle),
                    bounds,
                }
            }
            RealtimePreviewSurfaceBindingKind::Mock => PreviewSurfaceDescriptor::NativeChild {
                parent_window_handle: NativeParentWindowHandle::Mock(parent_handle),
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
pub struct RealtimePreviewEventSubscriptionResponse {
    pub subscribed: bool,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_sync: Option<RealtimePreviewAudioSyncState>,
    pub queue_latency_ms: u64,
    pub render_duration_ms: u64,
    pub mode: PreviewRequestMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_token: Option<PreviewCancellationToken>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<RealtimePreviewFallbackReason>,
    pub cache_hit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewTextHitTestBindingRequest {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewTextHitTestBindingResponse {
    pub hit: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<draft_model::TextSegmentSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_time_microseconds: Option<u64>,
}

impl RealtimePreviewTextHitTestBindingResponse {
    fn miss() -> Self {
        Self {
            hit: false,
            track_id: None,
            segment_id: None,
            selection_handle: None,
            source: None,
            content: None,
            x: None,
            y: None,
            width: None,
            height: None,
            target_time_microseconds: None,
        }
    }
}

impl RealtimePreviewFrameBindingRequest {
    fn to_runtime_request(&self) -> RealtimePreviewFrameRequest {
        RealtimePreviewFrameRequest {
            target_time: Microseconds::new(self.target_time_microseconds),
            playback_generation: PlaybackGeneration::new(self.playback_generation),
            audio_sync: self.audio_sync.clone(),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_sync: Option<RealtimePreviewAudioSyncState>,
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
    pub scheduler_queue_latency_p95_us: Option<u64>,
    pub scheduler_queue_depth: u32,
    pub scheduler_resource_saturation_count: u64,
    pub scheduler_rejected_count: u64,
    pub scheduler_canceled_count: u64,
    pub scheduler_stale_rejected_count: u64,
    pub target_time_microseconds: u64,
    pub playback_generation: u64,
    pub frame_pacing: RealtimePreviewFramePacingTelemetry,
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
            scheduler_queue_latency_p95_us: telemetry.scheduler_queue_latency_p95_us,
            scheduler_queue_depth: u32::try_from(telemetry.scheduler_queue_depth)
                .unwrap_or(u32::MAX),
            scheduler_resource_saturation_count: telemetry.scheduler_resource_saturation_count,
            scheduler_rejected_count: telemetry.scheduler_rejected_count,
            scheduler_canceled_count: telemetry.scheduler_canceled_count,
            scheduler_stale_rejected_count: telemetry.scheduler_stale_rejected_count,
            target_time_microseconds: telemetry.target_time.get(),
            playback_generation: telemetry.generation.get(),
            frame_pacing: telemetry.frame_pacing.clone(),
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

    fn presenter(error: NativePreviewPresenterError) -> Self {
        Self::new(
            RealtimePreviewBindingErrorKind::NativePresenter,
            error.to_string(),
        )
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
    NativePresenter,
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

fn playback_cadence_error(error: impl fmt::Display) -> RealtimePreviewBindingError {
    RealtimePreviewBindingError::new(RealtimePreviewBindingErrorKind::Runtime, error.to_string())
}

#[cfg(test)]
mod realtime_preview_bindings {
    use super::{
        BindingPlaybackSnapshot, RealtimePreviewBackendUsed, RealtimePreviewBindingErrorKind,
        RealtimePreviewBindingRegistry, RealtimePreviewBindingScheduler,
        RealtimePreviewFrameBindingRequest, RealtimePreviewSessionBindingConfig,
        RealtimePreviewSurfaceBindingDescriptor, RealtimePreviewSurfaceBindingKind,
        RealtimePreviewSurfaceBoundsBindingRequest, RealtimePreviewTelemetryBindingResponse,
        RealtimePreviewTextHitTestBindingRequest, RealtimePreviewTextureCache,
        SCHEDULER_MEDIA_PIPELINES, SchedulerFrameProvider, SchedulerMediaPipeline,
        StaticImageFrame, transformed_text_overlay_contains,
    };
    use crate::native_preview_presenter::{
        NativePreviewContentEvidenceSource, NativePreviewPresentationBackend,
    };
    use draft_model::{
        AudioPreviewPlaybackStatus, Draft, Material, MaterialId, MaterialKind, MaterialMetadata,
        Microseconds, RationalFrameRate, Segment, SourceTimerange, TargetTimerange, TextSegment,
        TextSegmentSource, Track, TrackKind,
    };
    use realtime_preview_runtime::{
        MediaIoFrameProvider, PlaybackGeneration, PreviewFrameInput, PreviewFrameProvider,
        PreviewRequestMode, RealtimePlaybackSchedulerConfig, RealtimePlaybackTextOverlayEvidence,
        RealtimePreviewAudioSyncState, RealtimePreviewFallbackReason,
    };
    use render_graph::OutputDimensions;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    #[test]
    fn text_overlay_hit_test_uses_transformed_visual_bounds() {
        let text = RealtimePlaybackTextOverlayEvidence {
            track_id: "track-text-001".to_owned(),
            segment_id: "segment-text-001".to_owned(),
            source: TextSegmentSource::Text,
            content: "变换命中".to_owned(),
            font_family: "Noto Sans CJK SC".to_owned(),
            font_ref: None,
            font_size: 32,
            color: "#ffffff".to_owned(),
            alignment: "center".to_owned(),
            line_height_millis: 1200,
            letter_spacing_millis: 0,
            x: 100,
            y: 100,
            width: 120,
            height: 60,
            visual_position_x: 500,
            visual_position_y: 0,
            visual_scale_x_millis: 1000,
            visual_scale_y_millis: 1000,
            visual_rotation_degrees: 0,
            visual_opacity_millis: 1000,
            selected: false,
        };

        assert!(
            transformed_text_overlay_contains(&text, 640, 360, 330, 130),
            "hit-test must include the visual-position transformed text bounds"
        );
        assert!(
            !transformed_text_overlay_contains(&text, 640, 360, 105, 105),
            "hit-test must not keep using the stale untransformed layout bounds"
        );
    }

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
                    parent_handle_hex: None,
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
                    audio_sync: None,
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
    fn runtime_frame_request_reports_diagnostic_backend_without_mock_default() {
        let (mut registry, session_id) = registry_with_session();
        let generation = registry
            .seek(&session_id, 33_333)
            .expect("seek returns generation");
        let result = registry
            .request_frame(
                &session_id,
                RealtimePreviewFrameBindingRequest {
                    target_time_microseconds: 33_333,
                    playback_generation: generation.playback_generation,
                    audio_sync: None,
                    queue_latency_ms: 1,
                    render_duration_ms: 1,
                    mode: PreviewRequestMode::PlaybackTick,
                    cancellation_token: None,
                    fallback_reason: None,
                    cache_hit: false,
                },
            )
            .expect("frame request succeeds");

        #[cfg(any(target_os = "macos", target_os = "windows"))]
        assert_eq!(result.backend, RealtimePreviewBackendUsed::Gpu);

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        assert_eq!(result.backend, RealtimePreviewBackendUsed::Offscreen);
    }

    #[test]
    fn playback_controls_fail_closed_without_native_presenter_and_non_play_controls_return_generations()
     {
        let (mut registry, session_id) = registry_with_session();

        let seek = registry
            .seek(&session_id, 500_000)
            .expect("seek returns generation");
        let play_error = registry
            .play(&session_id)
            .expect_err("play requires an attached production presenter");
        let pause = registry
            .pause(&session_id)
            .expect("pause returns generation");
        let stop = registry.stop(&session_id).expect("stop returns generation");

        assert_eq!(
            play_error.kind(),
            RealtimePreviewBindingErrorKind::NativePresenter
        );
        assert!(
            play_error.message().contains("not attached"),
            "play should fail closed without silently using a fallback: {}",
            play_error.message()
        );
        assert!(pause.playback_generation < stop.playback_generation);
        assert!(seek.playback_generation < pause.playback_generation);
    }

    #[test]
    fn scheduler_frame_provider_serves_static_image_inputs_without_media_io() {
        let image_id = MaterialId::new("image-material");
        let mut static_images = BTreeMap::new();
        static_images.insert(
            image_id.clone(),
            StaticImageFrame {
                material_id: image_id.clone(),
                width: 2,
                height: 1,
                pixels: vec![255, 0, 0, 255, 0, 255, 0, 255],
            },
        );
        let media_io = MediaIoFrameProvider::new(Box::new(PanicMediaReader));
        let mut provider = SchedulerFrameProvider::new(media_io, static_images);

        let input = provider
            .frame_for(
                &image_id,
                Microseconds::new(123_456),
                PlaybackGeneration::new(9),
            )
            .expect("static image material should produce a compositor input");
        let PreviewFrameInput::StaticImage(frame) = input else {
            panic!("expected static image compositor input");
        };
        assert_eq!(frame.material_id, image_id);
        assert_eq!(frame.source_position, Microseconds::new(123_456));
        assert_eq!(frame.playback_generation, PlaybackGeneration::new(9));
        assert_eq!(frame.width, 2);
        assert_eq!(frame.height, 1);
        assert_eq!(frame.pixels, vec![255, 0, 0, 255, 0, 255, 0, 255]);
    }

    #[test]
    fn scheduler_draft_update_invalidates_cached_media_pipeline() {
        let mut scheduler = RealtimePreviewBindingScheduler::new(
            RealtimePlaybackSchedulerConfig {
                preview_dimensions: OutputDimensions {
                    width: 640,
                    height: 360,
                },
            },
            Arc::new(Mutex::new(BindingPlaybackSnapshot::default())),
        );
        let previous_pipeline_id = scheduler.media_pipeline_id;
        SCHEDULER_MEDIA_PIPELINES.with(|pipelines| {
            pipelines.borrow_mut().insert(
                previous_pipeline_id,
                SchedulerMediaPipeline {
                    provider: SchedulerFrameProvider::new(
                        MediaIoFrameProvider::new(Box::new(PanicMediaReader)),
                        BTreeMap::new(),
                    ),
                    texture_cache: RealtimePreviewTextureCache::new(),
                    compositor: None,
                    in_flight_presentations: std::collections::VecDeque::new(),
                },
            );
        });

        scheduler.update_draft_snapshot(scheduler_video_draft(), None, None);

        assert_ne!(
            scheduler.media_pipeline_id, previous_pipeline_id,
            "draft changes must allocate a fresh media pipeline id"
        );
        assert!(
            SCHEDULER_MEDIA_PIPELINES
                .with(|pipelines| !pipelines.borrow().contains_key(&previous_pipeline_id)),
            "draft changes must remove the previous thread-local media pipeline"
        );
    }

    #[test]
    fn scheduler_playback_presents_render_graph_gpu_evidence_after_draft_and_surface_ready() {
        let (mut registry, session_id) = registry_with_session();
        registry
            .attach_surface(
                &session_id,
                RealtimePreviewSurfaceBindingDescriptor {
                    kind: RealtimePreviewSurfaceBindingKind::Mock,
                    parent_handle: Some(42),
                    parent_handle_hex: None,
                    x: 0,
                    y: 0,
                    width: 640,
                    height: 360,
                    scale_factor_millis: 1000,
                },
            )
            .expect("scheduler can validate attached compositor surface");
        registry
            .update_draft_snapshot(&session_id, scheduler_video_draft(), None, None)
            .expect("scheduler stores accepted draft snapshot");
        registry
            .seek(&session_id, 500_000)
            .expect("scheduler clock seeks to timeline time");

        let play = registry
            .play(&session_id)
            .expect("scheduler play starts the background playback pump");
        let mut presentation = registry
            .presentation_state(&session_id)
            .expect("scheduler presentation snapshot is queryable");
        let mut telemetry = registry
            .telemetry(&session_id)
            .expect("scheduler telemetry snapshot is queryable");
        for _ in 0..20 {
            if presentation.available && telemetry.presented_frame_count > 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            presentation = registry
                .presentation_state(&session_id)
                .expect("background playback evidence is queryable");
            telemetry = registry
                .telemetry(&session_id)
                .expect("background playback telemetry is queryable");
        }
        let presented_before_snapshot_query = telemetry.presented_frame_count;
        let _ = registry
            .presentation_state(&session_id)
            .expect("presentation state query must remain lightweight");
        let telemetry_after_snapshot_query = registry
            .telemetry(&session_id)
            .expect("telemetry remains queryable after snapshot query");

        assert!(
            play.playback_generation > 0,
            "scheduler play returns an advanced playback generation"
        );
        assert!(presentation.available);
        assert_eq!(
            presentation.backend,
            NativePreviewPresentationBackend::RenderGraphGpu
        );
        let evidence = presentation
            .evidence
            .as_ref()
            .expect("presented compositor evidence is required");
        assert_eq!(
            evidence.source,
            NativePreviewContentEvidenceSource::RenderGraphGpuComposited
        );
        assert!(evidence.target_time_microseconds >= 500_000);
        assert_eq!(
            evidence
                .active_text_overlays
                .iter()
                .map(|text| (text.source, text.content.as_str()))
                .collect::<Vec<_>>(),
            vec![(TextSegmentSource::Subtitle, "调度字幕证据")]
        );
        let overlay = evidence
            .active_text_overlays
            .first()
            .expect("text overlay evidence is required");
        assert_eq!(overlay.track_id, "track-subtitle-001");
        assert_eq!(overlay.segment_id, "segment-subtitle-001");
        assert_eq!(
            overlay.selection_handle,
            "timeline-segment:track-subtitle-001:segment-subtitle-001"
        );
        let hit = registry
            .hit_test_text_overlay(
                &session_id,
                RealtimePreviewTextHitTestBindingRequest {
                    x: overlay.x.saturating_add(1),
                    y: overlay.y.saturating_add(1),
                },
            )
            .expect("text hit-test uses latest native evidence");
        assert!(hit.hit);
        assert_eq!(
            hit.selection_handle.as_deref(),
            Some(overlay.selection_handle.as_str())
        );
        assert_eq!(hit.track_id.as_deref(), Some("track-subtitle-001"));
        assert_eq!(hit.segment_id.as_deref(), Some("segment-subtitle-001"));
        assert!(telemetry.presented_frame_count > 0);
        assert!(telemetry.target_time_microseconds >= 500_000);
        assert_eq!(
            telemetry_after_snapshot_query.presented_frame_count, presented_before_snapshot_query,
            "presentation_state must not synchronously present an extra frame"
        );
    }

    #[test]
    fn scheduler_seek_presents_still_frame_without_electron_frame_pump() {
        let (mut registry, session_id) = registry_with_session();
        registry
            .attach_surface(
                &session_id,
                RealtimePreviewSurfaceBindingDescriptor {
                    kind: RealtimePreviewSurfaceBindingKind::Mock,
                    parent_handle: Some(42),
                    parent_handle_hex: None,
                    x: 0,
                    y: 0,
                    width: 640,
                    height: 360,
                    scale_factor_millis: 1000,
                },
            )
            .expect("scheduler can validate attached compositor surface");
        registry
            .update_draft_snapshot(&session_id, scheduler_video_draft(), None, None)
            .expect("scheduler stores accepted draft snapshot");

        let seek = registry
            .seek(&session_id, 500_000)
            .expect("seek starts a Rust-owned still-frame present");
        let mut presentation = registry
            .presentation_state(&session_id)
            .expect("scheduler presentation snapshot is queryable");
        let mut telemetry = registry
            .telemetry(&session_id)
            .expect("scheduler telemetry snapshot is queryable");
        for _ in 0..20 {
            if presentation.available && telemetry.presented_frame_count > 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            presentation = registry
                .presentation_state(&session_id)
                .expect("still-frame evidence is queryable");
            telemetry = registry
                .telemetry(&session_id)
                .expect("still-frame telemetry is queryable");
        }
        let presented_before_snapshot_query = telemetry.presented_frame_count;
        let _ = registry
            .presentation_state(&session_id)
            .expect("presentation state query remains lightweight");
        let telemetry_after_snapshot_query = registry
            .telemetry(&session_id)
            .expect("telemetry remains queryable after snapshot query");

        assert!(seek.playback_generation > 0);
        assert!(presentation.available);
        assert_eq!(
            presentation.backend,
            NativePreviewPresentationBackend::RenderGraphGpu
        );
        let evidence = presentation
            .evidence
            .as_ref()
            .expect("still-frame compositor evidence is required");
        assert_eq!(
            evidence.source,
            NativePreviewContentEvidenceSource::RenderGraphGpuComposited
        );
        assert_eq!(evidence.target_time_microseconds, 500_000);
        assert_eq!(telemetry.presented_frame_count, 1);
        assert_eq!(telemetry.target_time_microseconds, 500_000);
        assert_eq!(
            telemetry_after_snapshot_query.presented_frame_count, presented_before_snapshot_query,
            "presentation_state must not synchronously present the still frame"
        );
    }

    #[test]
    fn scheduler_surface_resize_during_playback_grow_and_shrink_keeps_generation_and_worker() {
        let (mut registry, session_id) = registry_with_session();
        registry
            .attach_surface(
                &session_id,
                RealtimePreviewSurfaceBindingDescriptor {
                    kind: RealtimePreviewSurfaceBindingKind::Mock,
                    parent_handle: Some(42),
                    parent_handle_hex: None,
                    x: 0,
                    y: 0,
                    width: 640,
                    height: 360,
                    scale_factor_millis: 1000,
                },
            )
            .expect("scheduler can validate attached compositor surface");
        registry
            .update_draft_snapshot(&session_id, scheduler_video_draft(), None, None)
            .expect("scheduler stores accepted draft snapshot");
        registry
            .seek(&session_id, 250_000)
            .expect("scheduler clock seeks to timeline time");
        let play = registry
            .play(&session_id)
            .expect("scheduler play starts the background playback pump");

        let mut before_resize = registry
            .telemetry(&session_id)
            .expect("scheduler telemetry is queryable before resize");
        for _ in 0..40 {
            if before_resize.presented_frame_count > 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            before_resize = registry
                .telemetry(&session_id)
                .expect("scheduler telemetry is queryable before resize");
        }
        assert!(
            before_resize.presented_frame_count > 0,
            "playback should present at least one frame before resize"
        );

        let after_grow = assert_resize_continues_playback(
            &mut registry,
            &session_id,
            RealtimePreviewSurfaceBoundsBindingRequest {
                x: 4,
                y: 6,
                width: 800,
                height: 450,
                scale_factor_millis: 1000,
            },
            play.playback_generation,
            before_resize.presented_frame_count,
            "grow",
        );

        assert_resize_continues_playback(
            &mut registry,
            &session_id,
            RealtimePreviewSurfaceBoundsBindingRequest {
                x: 2,
                y: 3,
                width: 320,
                height: 180,
                scale_factor_millis: 1000,
            },
            play.playback_generation,
            after_grow.presented_frame_count,
            "shrink",
        );
    }

    fn assert_resize_continues_playback(
        registry: &mut RealtimePreviewBindingRegistry,
        session_id: &str,
        bounds: RealtimePreviewSurfaceBoundsBindingRequest,
        expected_generation: u64,
        presented_before_resize: u64,
        direction: &str,
    ) -> RealtimePreviewTelemetryBindingResponse {
        let resized = registry
            .update_surface_bounds(session_id, bounds)
            .unwrap_or_else(|error| {
                panic!("surface {direction} should not stop playback: {error}")
            });
        assert_eq!(
            resized.playback_generation, expected_generation,
            "surface {direction} must not advance playback generation or restart the frame pump"
        );

        let mut after_resize = registry.telemetry(session_id).unwrap_or_else(|error| {
            panic!("scheduler telemetry is queryable after {direction}: {error}")
        });
        for _ in 0..60 {
            if after_resize.playback_generation == expected_generation
                && after_resize.presented_frame_count > presented_before_resize
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            after_resize = registry.telemetry(session_id).unwrap_or_else(|error| {
                panic!("scheduler telemetry is queryable after {direction}: {error}")
            });
        }

        assert_eq!(
            after_resize.playback_generation, expected_generation,
            "{direction} continuation should present frames under the existing playback generation"
        );
        assert!(
            after_resize.presented_frame_count > presented_before_resize,
            "playback should continue presenting after a surface {direction}"
        );
        after_resize
    }

    #[test]
    fn realtime_frame_request_carries_audio_preview_sync_state() {
        let (mut registry, session_id) = registry_with_session();
        let generation = registry
            .seek(&session_id, 700_000)
            .expect("seek returns generation");
        let audio_sync = RealtimePreviewAudioSyncState {
            session_id: "audio-session-0000000000000001".to_owned(),
            playback_generation: PlaybackGeneration::new(generation.playback_generation),
            target_time: Microseconds::new(700_000),
            buffered_until: Microseconds::new(733_333),
            status: AudioPreviewPlaybackStatus::Playing,
            diagnostics: vec!["audio output primed".to_owned()],
        };

        let result = registry
            .request_frame(
                &session_id,
                RealtimePreviewFrameBindingRequest {
                    target_time_microseconds: 700_000,
                    playback_generation: generation.playback_generation,
                    audio_sync: Some(audio_sync.clone()),
                    queue_latency_ms: 2,
                    render_duration_ms: 4,
                    mode: PreviewRequestMode::PlaybackTick,
                    cancellation_token: None,
                    fallback_reason: None,
                    cache_hit: false,
                },
            )
            .expect("synchronized audio frame request succeeds");

        assert!(result.presented);
        assert_eq!(result.audio_sync, Some(audio_sync));
        assert_eq!(result.fallback, None);
    }

    #[test]
    fn realtime_frame_request_rejects_stale_audio_preview_state() {
        let (mut registry, session_id) = registry_with_session();
        let generation = registry
            .seek(&session_id, 800_000)
            .expect("seek returns generation");
        let audio_sync = RealtimePreviewAudioSyncState {
            session_id: "audio-session-0000000000000001".to_owned(),
            playback_generation: PlaybackGeneration::new(0),
            target_time: Microseconds::new(800_000),
            buffered_until: Microseconds::new(800_000),
            status: AudioPreviewPlaybackStatus::Playing,
            diagnostics: Vec::new(),
        };

        let result = registry
            .request_frame(
                &session_id,
                RealtimePreviewFrameBindingRequest {
                    target_time_microseconds: 800_000,
                    playback_generation: generation.playback_generation,
                    audio_sync: Some(audio_sync),
                    queue_latency_ms: 2,
                    render_duration_ms: 4,
                    mode: PreviewRequestMode::PlaybackTick,
                    cancellation_token: None,
                    fallback_reason: None,
                    cache_hit: false,
                },
            )
            .expect("stale audio state returns classified preview response");

        assert!(!result.presented);
        assert!(result.stale_rejected);
        assert_eq!(result.backend, RealtimePreviewBackendUsed::None);
        assert_eq!(
            result.fallback,
            Some(RealtimePreviewFallbackReason::StaleGeneration)
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.reason.contains("audio generation")),
            "diagnostics should identify audio sync rejection: {:?}",
            result.diagnostics
        );
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
                    audio_sync: None,
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

    fn scheduler_video_draft() -> Draft {
        let mut draft = Draft::new("draft-scheduler-001", "Scheduler playback");
        let mut material = Material::new(
            "material-video-001",
            MaterialKind::Video,
            "file:///repo-owned-fixture/p0-moving-testsrc.mp4",
            "p0-moving-testsrc.mp4",
        );
        material.metadata = MaterialMetadata {
            duration: Some(Microseconds::new(2_000_000)),
            width: Some(640),
            height: Some(360),
            frame_rate: Some(RationalFrameRate::new(30, 1)),
            has_video: true,
            has_audio: false,
            audio_sample_rate: None,
            audio_channels: None,
            probe_error: None,
        };
        draft.materials.push(material);

        let segment = Segment::new(
            "segment-video-001",
            "material-video-001",
            SourceTimerange::new(0, 2_000_000),
            TargetTimerange::new(0, 2_000_000),
        );
        let mut track = Track::new("track-video-001", TrackKind::Video, "视频");
        track.segments.push(segment);
        draft.tracks.push(track);
        let text_material = Material::new(
            "material-subtitle-001",
            MaterialKind::Text,
            "text://scheduler-subtitle-001",
            "调度字幕证据",
        );
        draft.materials.push(text_material);
        let mut text_segment = Segment::new(
            "segment-subtitle-001",
            "material-subtitle-001",
            SourceTimerange::new(0, 2_000_000),
            TargetTimerange::new(0, 2_000_000),
        );
        text_segment.text = Some(TextSegment {
            content: "调度字幕证据".to_owned(),
            source: TextSegmentSource::Subtitle,
            style: Default::default(),
            text_box: Default::default(),
            layout_region: Default::default(),
            wrapping: Default::default(),
            bubble: None,
            effect: None,
        });
        let mut text_track = Track::new("track-subtitle-001", TrackKind::Text, "字幕");
        text_track.segments.push(text_segment);
        draft.tracks.push(text_track);
        draft
    }

    struct PanicMediaReader;

    impl media_runtime::MediaReader for PanicMediaReader {
        fn reader_name(&self) -> &'static str {
            "panic-media-reader"
        }

        fn open(
            &self,
            _request: media_runtime::MediaOpenRequest,
        ) -> Result<Box<dyn media_runtime::MediaSession>, media_runtime::MediaIoError> {
            panic!("static image provider should not open media IO");
        }
    }
}

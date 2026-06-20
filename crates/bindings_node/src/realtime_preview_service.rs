use std::cell::RefCell;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

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
    MediaIoFrameProvider, PlaybackGeneration, PlaybackRate, PlaybackState,
    PreviewCancellationToken, PreviewDecodeDeviceContext, PreviewFrameInput, PreviewFrameProvider,
    PreviewFrameProviderError, PreviewFrameStoragePreference, PreviewGpuBackend,
    PreviewMaterialDecodeSource, PreviewRequestMode, PreviewSessionId, RealtimePlaybackScheduler,
    RealtimePlaybackSchedulerConfig, RealtimePlaybackSchedulerError,
    RealtimePlaybackSchedulerEvidence, RealtimePlaybackSchedulerPresentation,
    RealtimePlaybackSchedulerPresenter, RealtimePreviewAudioSyncState, RealtimePreviewBackendUsed,
    RealtimePreviewCapabilityClassifier, RealtimePreviewCompositor, RealtimePreviewDiagnostic,
    RealtimePreviewError, RealtimePreviewFallbackReason, RealtimePreviewFrameRequest,
    RealtimePreviewRuntime, RealtimePreviewSessionConfig, RealtimePreviewTelemetry,
    TextureHandleDescriptor,
    gpu::PreviewSurfaceScreenRect,
    gpu::{NativeParentWindowHandle, PreviewSurfaceBounds, PreviewSurfaceDescriptor},
    gpu::{
        RealtimePreviewExternalTexturePlanes, RealtimePreviewGpuBackend, RealtimePreviewGpuDevice,
        RealtimePreviewGpuDeviceDescriptor, RealtimePreviewGpuPresentationTarget,
        RealtimePreviewTargetFormat, RealtimePreviewTextureCache,
    },
};
use render_graph::{OutputDimensions, RenderGraph};
use serde::{Deserialize, Deserializer, Serialize, de::Error as SerdeDeError};

use crate::native_preview_presenter::{
    NativePreviewContentEvidence, NativePreviewContentEvidenceSource,
    NativePreviewPresentationState, NativePreviewPresenter, NativePreviewPresenterError,
    NativePreviewScreenRect, NativePreviewSurfacePlacementEvidence,
};

const SESSION_PREFIX: &str = "rtprev-session-";
static NEXT_SCHEDULER_MEDIA_PIPELINE_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static SCHEDULER_MEDIA_PIPELINES: RefCell<BTreeMap<u64, SchedulerMediaPipeline>> =
        RefCell::new(BTreeMap::new());
}

#[derive(Default)]
pub struct RealtimePreviewBindingRegistry {
    runtime: RealtimePreviewRuntime,
    next_binding_id: u64,
    sessions: BTreeMap<String, PreviewSessionId>,
    presenters: BTreeMap<String, NativePreviewPresenter>,
    schedulers: BTreeMap<String, RealtimePreviewBindingScheduler>,
}

impl RealtimePreviewBindingRegistry {
    pub fn new() -> Self {
        Self {
            runtime: RealtimePreviewRuntime::new(),
            next_binding_id: 1,
            sessions: BTreeMap::new(),
            presenters: BTreeMap::new(),
            schedulers: BTreeMap::new(),
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
            RealtimePreviewBindingScheduler::new(RealtimePlaybackSchedulerConfig {
                preview_dimensions: OutputDimensions {
                    width: 1280,
                    height: 720,
                },
            }),
        );
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
        if let Some(mut presenter) = self.presenters.remove(session_id) {
            presenter.detach();
        }
        self.schedulers.remove(session_id);
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
        self.scheduler_mut(session_id)?.attach_surface(descriptor)?;
        let generation = match self.runtime.attach_surface(runtime_id, descriptor) {
            Ok(generation) => generation,
            Err(error) => {
                self.scheduler_mut(session_id)?.detach_surface();
                return Err(RealtimePreviewBindingError::runtime(error));
            }
        };
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
        self.scheduler_mut(session_id)?
            .update_surface_bounds(bounds.to_runtime_bounds())?;
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
        self.presenter_mut(session_id)?.detach();
        self.scheduler_mut(session_id)?.detach_surface();
        Ok(generation_response(generation))
    }

    pub fn update_draft_snapshot(
        &mut self,
        session_id: &str,
        draft: Draft,
        bundle_path: Option<PathBuf>,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        self.scheduler_mut(session_id)?
            .update_draft_snapshot(draft.clone(), bundle_path.clone());
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
        let target_time = Microseconds::new(target_time_microseconds);
        self.scheduler_mut(session_id)?.seek(target_time);
        let generation = self
            .runtime
            .seek(runtime_id, target_time)
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(generation_response(generation))
    }

    pub fn play(
        &mut self,
        session_id: &str,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let target_time = self
            .runtime
            .clock(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?
            .position();
        let generation = self
            .runtime
            .play(runtime_id)
            .map_err(RealtimePreviewBindingError::runtime)?;
        self.scheduler_mut(session_id)?
            .start_playback(target_time, generation);
        let evidence = match self
            .scheduler_mut(session_id)?
            .present_playback_tick(generation)
        {
            Ok(evidence) => evidence,
            Err(error) => {
                let _ = self.runtime.pause(runtime_id);
                self.scheduler_mut(session_id)?.pause_playback();
                return Err(error);
            }
        };
        self.runtime
            .record_presented_output(
                runtime_id,
                Microseconds::new(evidence.target_time_microseconds),
                u64::from(evidence.presented_frames),
            )
            .map_err(RealtimePreviewBindingError::runtime)?;
        Ok(generation_response(generation))
    }

    pub fn pause(
        &mut self,
        session_id: &str,
    ) -> Result<RealtimePreviewGenerationBindingResponse, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        self.scheduler_mut(session_id)?.pause_playback();
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
        self.scheduler_mut(session_id)?.stop_playback();
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

    pub fn presentation_state(
        &mut self,
        session_id: &str,
    ) -> Result<NativePreviewPresentationState, RealtimePreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let (clock_state, generation) = {
            let clock = self
                .runtime
                .clock(runtime_id)
                .map_err(RealtimePreviewBindingError::runtime)?;
            (clock.state(), clock.generation())
        };
        if clock_state == PlaybackState::Playing {
            let evidence = self
                .scheduler_mut(session_id)?
                .present_playback_tick(generation)?;
            self.runtime
                .record_presented_output(
                    runtime_id,
                    Microseconds::new(evidence.target_time_microseconds),
                    u64::from(evidence.presented_frames),
                )
                .map_err(RealtimePreviewBindingError::runtime)?;
        }
        let scheduler = self.scheduler_mut(session_id)?;
        let evidence = scheduler
            .evidence()
            .cloned()
            .map(native_evidence_from_scheduler);
        let surface_placement = scheduler.surface_placement();
        match evidence {
            Some(evidence) => Ok(
                NativePreviewPresentationState::render_graph_gpu_available(Some(evidence))
                    .with_surface_placement(surface_placement),
            ),
            None => Ok(NativePreviewPresentationState::unavailable(
                "render graph GPU compositor scheduler has not presented product content",
            )),
        }
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

    fn presenter_mut(
        &mut self,
        session_id: &str,
    ) -> Result<&mut NativePreviewPresenter, RealtimePreviewBindingError> {
        validate_binding_session_id(session_id)?;
        self.presenters
            .get_mut(session_id)
            .ok_or_else(|| RealtimePreviewBindingError::unknown_session(session_id))
    }

    fn scheduler_mut(
        &mut self,
        session_id: &str,
    ) -> Result<&mut RealtimePreviewBindingScheduler, RealtimePreviewBindingError> {
        validate_binding_session_id(session_id)?;
        self.schedulers
            .get_mut(session_id)
            .ok_or_else(|| RealtimePreviewBindingError::unknown_session(session_id))
    }
}

struct RealtimePreviewBindingScheduler {
    scheduler: RealtimePlaybackScheduler,
    gpu_device: Option<RealtimePreviewGpuDevice>,
    surface_target: Option<RealtimePreviewGpuPresentationTarget>,
    draft_snapshot: Option<Draft>,
    bundle_path: Option<PathBuf>,
    media_pipeline_id: u64,
    last_evidence: Option<RealtimePlaybackSchedulerEvidence>,
    next_tick_time: Microseconds,
    playback_anchor: Option<BindingPlaybackAnchor>,
    #[cfg(test)]
    test_mock_surface_attached: bool,
}

#[derive(Debug, Clone)]
struct BindingPlaybackAnchor {
    started_at: Instant,
    start_time: Microseconds,
    playback_generation: PlaybackGeneration,
    sequence_duration: Microseconds,
}

impl RealtimePreviewBindingScheduler {
    fn new(config: RealtimePlaybackSchedulerConfig) -> Self {
        Self {
            scheduler: RealtimePlaybackScheduler::new(config),
            gpu_device: None,
            surface_target: None,
            draft_snapshot: None,
            bundle_path: None,
            media_pipeline_id: next_scheduler_media_pipeline_id(),
            last_evidence: None,
            next_tick_time: Microseconds::ZERO,
            playback_anchor: None,
            #[cfg(test)]
            test_mock_surface_attached: false,
        }
    }

    fn attach_surface(
        &mut self,
        descriptor: PreviewSurfaceDescriptor,
    ) -> Result<(), RealtimePreviewBindingError> {
        let bounds = descriptor.bounds();
        self.scheduler.update_preview_dimensions(OutputDimensions {
            width: bounds.width,
            height: bounds.height,
        });
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
            self.reset_media_pipeline();
            self.test_mock_surface_attached = true;
            return Ok(());
        }
        if std::env::var("VIDEO_EDITOR_TEST_DISABLE_RENDER_GRAPH_COMPOSITOR")
            .ok()
            .as_deref()
            == Some("1")
        {
            self.surface_target = None;
            self.gpu_device = None;
            self.reset_media_pipeline();
            #[cfg(test)]
            {
                self.test_mock_surface_attached = false;
            }
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
        self.gpu_device = Some(device);
        self.surface_target = Some(target);
        self.reset_media_pipeline();
        #[cfg(test)]
        {
            self.test_mock_surface_attached = false;
        }
        Ok(())
    }

    fn update_surface_bounds(
        &mut self,
        bounds: PreviewSurfaceBounds,
    ) -> Result<(), RealtimePreviewBindingError> {
        self.scheduler.update_preview_dimensions(OutputDimensions {
            width: bounds.width,
            height: bounds.height,
        });
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
        }
        Ok(())
    }

    fn update_draft_snapshot(&mut self, draft: Draft, bundle_path: Option<PathBuf>) {
        self.scheduler.update_draft_snapshot(draft.clone());
        self.draft_snapshot = Some(draft);
        self.bundle_path = bundle_path;
        self.reset_media_pipeline();
        self.last_evidence = None;
        self.next_tick_time = Microseconds::ZERO;
        self.playback_anchor = None;
    }

    fn seek(&mut self, target_time: Microseconds) {
        self.next_tick_time = target_time;
        self.last_evidence = None;
        self.playback_anchor = None;
    }

    fn detach_surface(&mut self) {
        self.surface_target = None;
        self.reset_media_pipeline();
        self.last_evidence = None;
        self.playback_anchor = None;
        #[cfg(test)]
        {
            self.test_mock_surface_attached = false;
        }
    }

    fn start_playback(
        &mut self,
        start_time: Microseconds,
        playback_generation: PlaybackGeneration,
    ) {
        self.playback_anchor = Some(BindingPlaybackAnchor {
            started_at: Instant::now(),
            start_time,
            playback_generation,
            sequence_duration: self.sequence_duration(),
        });
        self.next_tick_time = start_time;
    }

    fn pause_playback(&mut self) {
        self.playback_anchor = None;
    }

    fn stop_playback(&mut self) {
        self.playback_anchor = None;
        self.next_tick_time = Microseconds::ZERO;
    }

    fn evidence(&self) -> Option<&RealtimePlaybackSchedulerEvidence> {
        self.last_evidence
            .as_ref()
            .or_else(|| self.scheduler.last_evidence())
    }

    fn surface_placement(&self) -> Option<NativePreviewSurfacePlacementEvidence> {
        self.surface_target
            .as_ref()
            .and_then(|target| target.screen_rect())
            .map(native_surface_placement_from_runtime)
    }

    fn present_next_tick(
        &mut self,
        target_time: Microseconds,
        playback_generation: PlaybackGeneration,
    ) -> Result<RealtimePlaybackSchedulerEvidence, RealtimePreviewBindingError> {
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
        let tick_time = if target_time.get() > self.next_tick_time.get() {
            target_time
        } else {
            Microseconds::new(self.next_tick_time.get().saturating_add(33_333))
        };
        self.next_tick_time = tick_time;
        #[cfg(test)]
        if self.test_mock_surface_attached {
            let mut presenter = BindingSchedulerTestPresenter;
            let evidence = self
                .scheduler
                .present_tick(tick_time, playback_generation, &mut presenter)
                .map_err(scheduler_error)?;
            self.last_evidence = Some(evidence.clone());
            return Ok(evidence);
        }
        self.ensure_media_provider()?;
        let media_pipeline_id = self.media_pipeline_id;
        let mut presenter = BindingSchedulerPresenter {
            gpu_device: self.gpu_device.clone(),
            surface_target: self.surface_target.as_mut(),
            media_pipeline_id,
        };
        let evidence = self
            .scheduler
            .present_tick(tick_time, playback_generation, &mut presenter)
            .map_err(scheduler_error)?;
        self.last_evidence = Some(evidence.clone());
        Ok(evidence)
    }

    fn present_playback_tick(
        &mut self,
        playback_generation: PlaybackGeneration,
    ) -> Result<RealtimePlaybackSchedulerEvidence, RealtimePreviewBindingError> {
        let target_time = self.playback_target_time(playback_generation);
        self.present_next_tick(target_time, playback_generation)
    }

    fn playback_target_time(&self, playback_generation: PlaybackGeneration) -> Microseconds {
        let Some(anchor) = self.playback_anchor.as_ref() else {
            return self.next_tick_time;
        };
        if anchor.playback_generation != playback_generation {
            return self.next_tick_time;
        }

        let elapsed_us = u64::try_from(anchor.started_at.elapsed().as_micros()).unwrap_or(u64::MAX);
        let target = anchor.start_time.get().saturating_add(elapsed_us);
        Microseconds::new(target.min(anchor.sequence_duration.get()))
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
            registry,
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
    registry: NativeTextureLeaseRegistry,
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

    fn release_presented_frames(&mut self) -> Result<(), MediaIoHandoffReleaseError> {
        self.media_io
            .release_presented_frames()
            .map(|_| ())
            .map_err(|source| MediaIoHandoffReleaseError { source })
    }
}

#[derive(Debug)]
struct MediaIoHandoffReleaseError {
    source: realtime_preview_runtime::MediaIoHandoffError,
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
            let mut texture_cache = RealtimePreviewTextureCache::new()
                .with_native_texture_registry(pipeline.registry.clone())
                .with_native_texture_importer(Box::new(import_native_nv12_external_texture));
            let mut compositor = RealtimePreviewCompositor::new(
                gpu_device,
                RealtimePreviewCapabilityClassifier {
                    runtime_backend_available: true,
                    surface_available: true,
                    gpu_text_parity: false,
                    bundled_text_font_registry_available: true,
                },
            );
            let presentation = compositor.present_to_surface_with_generation(
                graph,
                target,
                &mut pipeline.provider,
                &mut texture_cache,
                playback_generation,
            );
            let release = pipeline.provider.release_presented_frames();
            match (presentation, release) {
                (Ok(output), Ok(())) => Ok(output),
                (Err(error), _) => Err(RealtimePlaybackSchedulerError::Presentation {
                    reason: error.to_string(),
                }),
                (Ok(_), Err(error)) => Err(RealtimePlaybackSchedulerError::Presentation {
                    reason: error.to_string(),
                }),
            }
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
    }
}

fn native_surface_placement_from_runtime(
    rect: PreviewSurfaceScreenRect,
) -> NativePreviewSurfacePlacementEvidence {
    NativePreviewSurfacePlacementEvidence {
        native_screen_rect: NativePreviewScreenRect {
            x: rect.x.round() as i32,
            y: rect.y.round() as i32,
            width: rect.width.round() as i32,
            height: rect.height.round() as i32,
        },
    }
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

#[cfg(test)]
mod realtime_preview_bindings {
    use super::{
        RealtimePreviewBackendUsed, RealtimePreviewBindingErrorKind,
        RealtimePreviewBindingRegistry, RealtimePreviewBindingScheduler,
        RealtimePreviewFrameBindingRequest, RealtimePreviewSessionBindingConfig,
        RealtimePreviewSurfaceBindingDescriptor, RealtimePreviewSurfaceBindingKind,
        SCHEDULER_MEDIA_PIPELINES, SchedulerFrameProvider, SchedulerMediaPipeline,
        StaticImageFrame,
    };
    use crate::native_preview_presenter::{
        NativePreviewContentEvidenceSource, NativePreviewPresentationBackend,
    };
    use draft_model::{
        AudioPreviewPlaybackStatus, Draft, Material, MaterialId, MaterialKind, MaterialMetadata,
        Microseconds, RationalFrameRate, Segment, SourceTimerange, TargetTimerange, Track,
        TrackKind,
    };
    use realtime_preview_runtime::{
        MediaIoFrameProvider, PlaybackGeneration, PreviewFrameInput, PreviewFrameProvider,
        PreviewRequestMode, RealtimePlaybackSchedulerConfig, RealtimePreviewAudioSyncState,
        RealtimePreviewFallbackReason,
    };
    use render_graph::OutputDimensions;
    use std::collections::BTreeMap;

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
        let mut scheduler = RealtimePreviewBindingScheduler::new(RealtimePlaybackSchedulerConfig {
            preview_dimensions: OutputDimensions {
                width: 640,
                height: 360,
            },
        });
        let previous_pipeline_id = scheduler.media_pipeline_id;
        SCHEDULER_MEDIA_PIPELINES.with(|pipelines| {
            pipelines.borrow_mut().insert(
                previous_pipeline_id,
                SchedulerMediaPipeline {
                    provider: SchedulerFrameProvider::new(
                        MediaIoFrameProvider::new(Box::new(PanicMediaReader)),
                        BTreeMap::new(),
                    ),
                    registry: media_runtime::NativeTextureLeaseRegistry::new(),
                },
            );
        });

        scheduler.update_draft_snapshot(scheduler_video_draft(), None);

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
            .update_draft_snapshot(&session_id, scheduler_video_draft(), None)
            .expect("scheduler stores accepted draft snapshot");
        registry
            .seek(&session_id, 500_000)
            .expect("scheduler clock seeks to timeline time");

        let play = registry
            .play(&session_id)
            .expect("scheduler play decodes, builds render graph, and presents");
        let presentation = registry
            .presentation_state(&session_id)
            .expect("scheduler presentation evidence is queryable");
        let telemetry = registry
            .telemetry(&session_id)
            .expect("scheduler presentation updates telemetry");

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
        assert!(telemetry.presented_frame_count > 0);
        assert!(telemetry.target_time_microseconds >= 500_000);
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

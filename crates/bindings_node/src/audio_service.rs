use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use audio_engine::{
    AudioOutputDevice, AudioPreviewError, AudioPreviewRuntime, AudioPreviewSessionConfig,
    AudioPreviewSessionId,
};
use audio_output_desktop::{
    CpalAudioOutputDevice, CpalAudioOutputQueue, CpalAudioOutputSink, DesktopAudioOutputBackend,
    DesktopAudioOutputBackendCapabilities, DesktopAudioOutputCapabilityStatus,
    probe_desktop_audio_output_capabilities,
};
use draft_model::{
    AudioOutputDeviceStatus, AudioOutputDeviceSummary, AudioPreviewCommandPayload,
    AudioPreviewCommandResponse, AudioPreviewPlaybackStatus, AudioPreviewStatusResponse,
    CommandError, CommandErrorKind, CommandName, CommandPayload, CommandResultEnvelope, Draft,
    Material, MaterialId, MaterialKind, Microseconds, RationalFrameRate, TargetTimerange,
    WaveformDisplayPeaksResponse, WaveformDisplayStatus,
};
use media_runtime::{FfmpegExecutor, discover_runtime_config};
use media_runtime_desktop::DesktopFfmpegExecutor;
use project_store::resolve_material_uri;
use realtime_preview_runtime::{PlaybackRate, PlaybackState};
use serde::{Deserialize, Serialize};

const SESSION_PREFIX: &str = "audio-session-";
const MAX_WAVEFORM_PEAK_BINS: u16 = 512;
const AUDIO_PREVIEW_CHUNK_DURATION: Microseconds = Microseconds(2_000_000);
const AUDIO_PREVIEW_TARGET_QUEUE_DURATION: Microseconds = Microseconds(4_000_000);
const AUDIO_PREVIEW_REFILL_LOW_WATER: Microseconds = Microseconds(1_500_000);
const AUDIO_PREVIEW_REFILL_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Default)]
pub struct AudioPreviewBindingRegistry {
    runtime: AudioPreviewRuntime,
    next_binding_id: u64,
    sessions: BTreeMap<String, AudioPreviewBindingSession>,
    outputs: BTreeMap<String, NativeAudioPreviewOutput>,
    selected_device_id: Option<String>,
}

#[derive(Debug, Clone)]
struct AudioPreviewBindingSession {
    runtime_id: AudioPreviewSessionId,
    project_session_id: String,
}

struct NativeAudioPreviewOutput {
    sink: CpalAudioOutputSink,
    device: AudioOutputDeviceSummary,
    refill_stop: Arc<AtomicBool>,
    refill_thread: Option<JoinHandle<()>>,
}

impl Drop for NativeAudioPreviewOutput {
    fn drop(&mut self) {
        self.refill_stop.store(true, Ordering::Release);
        if let Some(thread) = self.refill_thread.take() {
            let _ = thread.join();
        }
    }
}

impl AudioPreviewBindingRegistry {
    pub fn new() -> Self {
        Self {
            runtime: AudioPreviewRuntime::new(),
            next_binding_id: 1,
            sessions: BTreeMap::new(),
            outputs: BTreeMap::new(),
            selected_device_id: None,
        }
    }

    pub fn create_session(
        &mut self,
        config: AudioPreviewSessionBindingConfig,
        project_session_id: &str,
    ) -> Result<AudioPreviewStatusResponse, AudioPreviewBindingError> {
        let runtime_id = self
            .runtime
            .create_session(config.to_runtime_config()?)
            .map_err(AudioPreviewBindingError::runtime)?;
        let binding_id = format!("{SESSION_PREFIX}{:016x}", self.next_binding_id);
        self.next_binding_id = self.next_binding_id.saturating_add(1);
        self.sessions.insert(
            binding_id.clone(),
            AudioPreviewBindingSession {
                runtime_id,
                project_session_id: project_session_id.to_owned(),
            },
        );
        self.status(&binding_id, project_session_id)
    }

    pub fn play(
        &mut self,
        session_id: &str,
        project_session_id: &str,
        draft: &Draft,
        target_time: Microseconds,
        playback_generation: u64,
    ) -> Result<AudioPreviewCommandResponse, AudioPreviewBindingError> {
        let current = self.status(session_id, project_session_id)?;
        if playback_generation != current.generation {
            return Ok(command_response(
                session_id,
                current.generation,
                false,
                AudioPreviewPlaybackStatus::StaleRejected,
                "声音已同步到最新播放头",
                current.target_time,
                vec!["stale playback generation rejected".to_owned()],
            ));
        }
        self.outputs.remove(session_id);
        let output = self.open_native_output_for_draft(draft, target_time)?;
        let runtime_id = self.runtime_session_id_for_project(session_id, project_session_id)?;
        let generation = self
            .runtime
            .seek(runtime_id, target_time)
            .and_then(|_| self.runtime.resume(runtime_id))
            .map_err(AudioPreviewBindingError::runtime)?;
        self.outputs.insert(session_id.to_owned(), output);
        Ok(command_response(
            session_id,
            generation.get(),
            true,
            AudioPreviewPlaybackStatus::Playing,
            "正在播放",
            target_time,
            Vec::new(),
        ))
    }

    pub fn pause(
        &mut self,
        session_id: &str,
        project_session_id: &str,
    ) -> Result<AudioPreviewCommandResponse, AudioPreviewBindingError> {
        self.outputs.remove(session_id);
        let runtime_id = self.runtime_session_id_for_project(session_id, project_session_id)?;
        let generation = self
            .runtime
            .pause(runtime_id)
            .map_err(AudioPreviewBindingError::runtime)?;
        let target_time = self.status(session_id, project_session_id)?.target_time;
        Ok(command_response(
            session_id,
            generation.get(),
            true,
            AudioPreviewPlaybackStatus::Paused,
            "已暂停",
            target_time,
            Vec::new(),
        ))
    }

    pub fn stop(
        &mut self,
        session_id: &str,
        project_session_id: &str,
    ) -> Result<AudioPreviewCommandResponse, AudioPreviewBindingError> {
        self.outputs.remove(session_id);
        let runtime_id = self.runtime_session_id_for_project(session_id, project_session_id)?;
        let generation = self
            .runtime
            .stop(runtime_id)
            .map_err(AudioPreviewBindingError::runtime)?;
        Ok(command_response(
            session_id,
            generation.get(),
            true,
            AudioPreviewPlaybackStatus::Stopped,
            "已停止",
            Microseconds::ZERO,
            Vec::new(),
        ))
    }

    pub fn seek(
        &mut self,
        session_id: &str,
        project_session_id: &str,
        target_time: Microseconds,
    ) -> Result<AudioPreviewCommandResponse, AudioPreviewBindingError> {
        let runtime_id = self.runtime_session_id_for_project(session_id, project_session_id)?;
        let generation = self
            .runtime
            .seek(runtime_id, target_time)
            .map_err(AudioPreviewBindingError::runtime)?;
        Ok(command_response(
            session_id,
            generation.get(),
            true,
            AudioPreviewPlaybackStatus::Seeking,
            "正在定位声音",
            target_time,
            Vec::new(),
        ))
    }

    pub fn cancel(
        &mut self,
        session_id: &str,
        project_session_id: &str,
    ) -> Result<AudioPreviewCommandResponse, AudioPreviewBindingError> {
        let runtime_id = self.runtime_session_id_for_project(session_id, project_session_id)?;
        let token = self
            .runtime
            .next_cancellation_token(runtime_id)
            .map_err(AudioPreviewBindingError::runtime)?;
        self.runtime
            .cancel_request(runtime_id, token)
            .map_err(AudioPreviewBindingError::runtime)?;
        let status = self.status(session_id, project_session_id)?;
        Ok(command_response(
            session_id,
            status.generation,
            true,
            AudioPreviewPlaybackStatus::Canceled,
            "音频请求已取消",
            status.target_time,
            Vec::new(),
        ))
    }

    pub fn status(
        &self,
        session_id: &str,
        project_session_id: &str,
    ) -> Result<AudioPreviewStatusResponse, AudioPreviewBindingError> {
        let runtime_id = self.runtime_session_id_for_project(session_id, project_session_id)?;
        let status = self
            .runtime
            .status(runtime_id)
            .map_err(AudioPreviewBindingError::runtime)?;
        let device = self
            .outputs
            .get(session_id)
            .map(|output| {
                let mut device = output.device.clone();
                device.diagnostics.push(format!(
                    "native queued samples: {}; underrun samples: {}",
                    output.sink.queued_sample_count(),
                    output.sink.underrun_sample_count()
                ));
                device
            })
            .or_else(|| {
                self.selected_device_id.as_deref().and_then(|selected| {
                    self.list_output_devices().ok().and_then(|devices| {
                        devices
                            .into_iter()
                            .find(|device| device.selection_id == selected)
                    })
                })
            })
            .or_else(|| {
                self.list_output_devices()
                    .ok()
                    .and_then(|devices| devices.into_iter().next())
            })
            .unwrap_or_else(missing_device_summary);
        let playback_status = playback_status(status.playback_state);
        Ok(AudioPreviewStatusResponse {
            session_id: session_id.to_owned(),
            generation: status.playback_generation.get(),
            status: playback_status,
            status_label: playback_status_label(playback_status).to_owned(),
            target_time: status.target_time,
            buffered_until: status.target_time,
            device,
            diagnostics: Vec::new(),
        })
    }

    pub fn list_output_devices(
        &self,
    ) -> Result<Vec<AudioOutputDeviceSummary>, AudioPreviewBindingError> {
        let capabilities = probe_desktop_audio_output_capabilities();
        let mut devices = capabilities
            .backends
            .iter()
            .filter(|backend| backend.backend == DesktopAudioOutputBackend::Native)
            .flat_map(device_summaries)
            .collect::<Vec<_>>();
        if devices.is_empty() {
            devices.push(missing_device_summary());
        }
        Ok(devices)
    }

    fn open_native_output_for_draft(
        &self,
        draft: &Draft,
        target_time: Microseconds,
    ) -> Result<NativeAudioPreviewOutput, AudioPreviewBindingError> {
        let device = CpalAudioOutputDevice::default_output().map_err(|diagnostic| {
            AudioPreviewBindingError::new(
                AudioPreviewBindingErrorKind::DeviceUnavailable,
                format!("native audio output unavailable: {}", diagnostic.message),
            )
        })?;
        let capabilities = device.capabilities();
        let mut sink = device.open_stream(&capabilities).map_err(|error| {
            AudioPreviewBindingError::new(
                AudioPreviewBindingErrorKind::DeviceUnavailable,
                error.to_string(),
            )
        })?;
        let output_channels = capabilities.max_channel_count.min(2).max(1);
        let mut cursor = target_time;
        while queued_duration_us(
            sink.queued_sample_count(),
            capabilities.sample_rate_hz,
            output_channels,
        ) < AUDIO_PREVIEW_TARGET_QUEUE_DURATION.get()
        {
            let samples = match render_draft_audio_preview(
                draft,
                cursor,
                capabilities.sample_rate_hz,
                output_channels,
                AUDIO_PREVIEW_CHUNK_DURATION,
            ) {
                Ok(samples) => samples,
                Err(error) if sink.queued_sample_count() > 0 => {
                    eprintln!("audio preview refill reached natural end during prefill: {error}");
                    break;
                }
                Err(error) => return Err(error),
            };
            sink.enqueue_f32_interleaved(&samples).map_err(|error| {
                AudioPreviewBindingError::new(
                    AudioPreviewBindingErrorKind::RuntimeFailed,
                    error.to_string(),
                )
            })?;
            cursor = Microseconds::new(
                cursor
                    .get()
                    .saturating_add(AUDIO_PREVIEW_CHUNK_DURATION.get()),
            );
        }
        sink.start().map_err(|error| {
            AudioPreviewBindingError::new(
                AudioPreviewBindingErrorKind::DeviceUnavailable,
                error.to_string(),
            )
        })?;
        let queue = sink.queue_handle();
        let refill_stop = Arc::new(AtomicBool::new(false));
        let refill_thread = spawn_audio_refill_thread(
            draft.clone(),
            queue,
            cursor,
            capabilities.sample_rate_hz,
            output_channels,
            Arc::clone(&refill_stop),
        )?;
        let summary = native_device_summary(device.summary());
        Ok(NativeAudioPreviewOutput {
            sink,
            device: summary,
            refill_stop,
            refill_thread: Some(refill_thread),
        })
    }

    pub fn select_output_device(
        &mut self,
        session_id: &str,
        project_session_id: &str,
        selection_id: &str,
    ) -> Result<AudioPreviewCommandResponse, AudioPreviewBindingError> {
        let status = self.status(session_id, project_session_id)?;
        let devices = self.list_output_devices()?;
        if !devices
            .iter()
            .any(|device| device.selection_id == selection_id)
        {
            return Err(AudioPreviewBindingError::new(
                AudioPreviewBindingErrorKind::DeviceUnavailable,
                "audio output device selection is not available",
            ));
        }
        self.selected_device_id = Some(selection_id.to_owned());
        Ok(command_response(
            session_id,
            status.generation,
            true,
            status.status,
            "输出设备就绪",
            status.target_time,
            Vec::new(),
        ))
    }

    pub fn waveform_display_peaks(
        &self,
        draft: &Draft,
        material_id: Option<String>,
        target_timerange: Option<TargetTimerange>,
        max_peak_bins: u16,
    ) -> Result<WaveformDisplayPeaksResponse, AudioPreviewBindingError> {
        let requested_peak_bins = max_peak_bins.min(MAX_WAVEFORM_PEAK_BINS);
        let material_id = material_id.map(draft_model::MaterialId::new);
        if requested_peak_bins == 0 {
            return Ok(WaveformDisplayPeaksResponse {
                material_id,
                status: WaveformDisplayStatus::Missing,
                status_label: "暂无波形".to_owned(),
                target_timerange,
                requested_peak_bins: 0,
                returned_peak_bins: 0,
                peaks: Vec::new(),
                diagnostics: vec!["waveform display request had zero bins".to_owned()],
            });
        }
        if let Some(material_id) = material_id.as_ref() {
            let exists = draft
                .materials
                .iter()
                .any(|material| &material.material_id == material_id);
            if !exists {
                return Ok(WaveformDisplayPeaksResponse {
                    material_id: Some(material_id.clone()),
                    status: WaveformDisplayStatus::Missing,
                    status_label: "暂无波形".to_owned(),
                    target_timerange,
                    requested_peak_bins,
                    returned_peak_bins: 0,
                    peaks: Vec::new(),
                    diagnostics: vec![
                        "waveform material is not present in the current project session"
                            .to_owned(),
                    ],
                });
            }
        }
        Ok(WaveformDisplayPeaksResponse {
            material_id,
            status: WaveformDisplayStatus::Missing,
            status_label: "暂无波形".to_owned(),
            target_timerange,
            requested_peak_bins,
            returned_peak_bins: 0,
            peaks: Vec::new(),
            diagnostics: vec![
                "waveform display requires a ready waveform artifact before peaks can be shown"
                    .to_owned(),
            ],
        })
    }

    fn runtime_session_id_for_project(
        &self,
        session_id: &str,
        project_session_id: &str,
    ) -> Result<AudioPreviewSessionId, AudioPreviewBindingError> {
        validate_binding_session_id(session_id)?;
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| AudioPreviewBindingError::unknown_session(session_id))?;
        if session.project_session_id != project_session_id {
            return Err(AudioPreviewBindingError::new(
                AudioPreviewBindingErrorKind::InvalidPayload,
                "audio preview session does not belong to the requested project session",
            ));
        }
        Ok(session.runtime_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioPreviewSessionBindingConfig {
    pub session_label: String,
    pub frame_rate_numerator: u32,
    pub frame_rate_denominator: u32,
    pub playback_rate_numerator: i32,
    pub playback_rate_denominator: u32,
    pub sample_rate_hz: u32,
    pub max_buffer_duration_microseconds: Microseconds,
    pub max_channel_count: u16,
    pub max_frame_count: u32,
}

impl AudioPreviewSessionBindingConfig {
    fn to_runtime_config(&self) -> Result<AudioPreviewSessionConfig, AudioPreviewBindingError> {
        if self.frame_rate_numerator == 0 || self.frame_rate_denominator == 0 {
            return Err(AudioPreviewBindingError::new(
                AudioPreviewBindingErrorKind::InvalidPayload,
                "audio preview frame rate must be nonzero",
            ));
        }
        let playback_rate =
            PlaybackRate::new(self.playback_rate_numerator, self.playback_rate_denominator)
                .map_err(|error| {
                    AudioPreviewBindingError::new(
                        AudioPreviewBindingErrorKind::InvalidPayload,
                        error.to_string(),
                    )
                })?;
        Ok(AudioPreviewSessionConfig {
            session_label: self.session_label.clone(),
            frame_rate: RationalFrameRate::new(
                self.frame_rate_numerator,
                self.frame_rate_denominator,
            ),
            playback_rate,
            sample_rate_hz: self.sample_rate_hz,
            max_buffer_duration_microseconds: self.max_buffer_duration_microseconds,
            max_channel_count: self.max_channel_count,
            max_frame_count: self.max_frame_count,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioPreviewBindingErrorKind {
    MalformedSessionId,
    UnknownSession,
    InvalidPayload,
    RuntimeFailed,
    DeviceUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioPreviewBindingError {
    kind: AudioPreviewBindingErrorKind,
    message: String,
}

impl AudioPreviewBindingError {
    pub fn new(kind: AudioPreviewBindingErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> AudioPreviewBindingErrorKind {
        self.kind
    }

    fn unknown_session(session_id: &str) -> Self {
        Self::new(
            AudioPreviewBindingErrorKind::UnknownSession,
            format!("unknown audio preview binding session: {session_id}"),
        )
    }

    fn runtime(error: AudioPreviewError) -> Self {
        Self::new(
            AudioPreviewBindingErrorKind::RuntimeFailed,
            error.to_string(),
        )
    }
}

impl fmt::Display for AudioPreviewBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl Error for AudioPreviewBindingError {}

pub fn handle_audio_service_command(
    registry: &mut AudioPreviewBindingRegistry,
    command: CommandName,
    payload: CommandPayload,
) -> CommandResultEnvelope<serde_json::Value> {
    match audio_command_result(registry, command.clone(), payload) {
        Ok(value) => CommandResultEnvelope {
            ok: true,
            data: Some(value),
            error: None,
            events: Vec::new(),
        },
        Err(error) => audio_error_envelope(error, command),
    }
}

fn audio_command_result(
    registry: &mut AudioPreviewBindingRegistry,
    command: CommandName,
    payload: CommandPayload,
) -> Result<serde_json::Value, AudioPreviewBindingError> {
    match (command, payload) {
        (
            CommandName::CreateAudioPreviewSession,
            CommandPayload::CreateAudioPreviewSession(payload),
        ) => {
            let context = project_session_audio_context_from_payload(
                &payload,
                "create audio preview session",
            )?;
            let config = session_config_from_draft(&context.draft)?;
            serialize(registry.create_session(config, &context.project_session_id)?)
        }
        (CommandName::PlayAudioPreview, CommandPayload::PlayAudioPreview(payload)) => {
            let session_id = required_session_id(&payload)?;
            let context =
                project_session_audio_context_from_payload(&payload, "play audio preview")?;
            serialize(registry.play(
                session_id,
                &context.project_session_id,
                &context.draft,
                payload.target_time.unwrap_or(Microseconds::ZERO),
                payload.playback_generation.unwrap_or(0),
            )?)
        }
        (CommandName::PauseAudioPreview, CommandPayload::PauseAudioPreview(payload)) => {
            let identity = project_session_identity_from_payload(&payload, "pause audio preview")?;
            serialize(registry.pause(required_session_id(&payload)?, &identity.project_session_id)?)
        }
        (CommandName::StopAudioPreview, CommandPayload::StopAudioPreview(payload)) => {
            let identity = project_session_identity_from_payload(&payload, "stop audio preview")?;
            serialize(registry.stop(required_session_id(&payload)?, &identity.project_session_id)?)
        }
        (CommandName::SeekAudioPreview, CommandPayload::SeekAudioPreview(payload)) => {
            let identity = project_session_identity_from_payload(&payload, "seek audio preview")?;
            serialize(registry.seek(
                required_session_id(&payload)?,
                &identity.project_session_id,
                payload.target_time.unwrap_or(Microseconds::ZERO),
            )?)
        }
        (CommandName::CancelAudioPreview, CommandPayload::CancelAudioPreview(payload)) => {
            let identity = project_session_identity_from_payload(&payload, "cancel audio preview")?;
            serialize(
                registry.cancel(required_session_id(&payload)?, &identity.project_session_id)?,
            )
        }
        (CommandName::GetAudioPreviewStatus, CommandPayload::GetAudioPreviewStatus(payload)) => {
            let identity =
                project_session_identity_from_payload(&payload, "get audio preview status")?;
            serialize(
                registry.status(required_session_id(&payload)?, &identity.project_session_id)?,
            )
        }
        (CommandName::ListAudioOutputDevices, CommandPayload::ListAudioOutputDevices(payload)) => {
            let _identity =
                project_session_identity_from_payload(&payload, "list audio output devices")?;
            serialize(registry.list_output_devices()?)
        }
        (
            CommandName::SelectAudioOutputDevice,
            CommandPayload::SelectAudioOutputDevice(payload),
        ) => {
            let identity =
                project_session_identity_from_payload(&payload, "select audio output device")?;
            let device_id = payload.device_selection_id.as_deref().ok_or_else(|| {
                AudioPreviewBindingError::new(
                    AudioPreviewBindingErrorKind::InvalidPayload,
                    "audio output device selection ID is required",
                )
            })?;
            serialize(registry.select_output_device(
                required_session_id(&payload)?,
                &identity.project_session_id,
                device_id,
            )?)
        }
        (
            CommandName::GetWaveformDisplayPeaks,
            CommandPayload::GetWaveformDisplayPeaks(payload),
        ) => {
            let context =
                project_session_audio_context_from_payload(&payload, "get waveform display peaks")?;
            serialize(
                registry.waveform_display_peaks(
                    &context.draft,
                    payload
                        .material_id
                        .map(|material_id| material_id.as_str().to_owned()),
                    payload.target_timerange,
                    payload.max_peak_bins.unwrap_or(MAX_WAVEFORM_PEAK_BINS),
                )?,
            )
        }
        (CommandName::RefreshWaveformStatus, CommandPayload::RefreshWaveformStatus(payload)) => {
            let context =
                project_session_audio_context_from_payload(&payload, "refresh waveform status")?;
            serialize(
                registry.waveform_display_peaks(
                    &context.draft,
                    payload
                        .material_id
                        .map(|material_id| material_id.as_str().to_owned()),
                    payload.target_timerange,
                    payload.max_peak_bins.unwrap_or(MAX_WAVEFORM_PEAK_BINS),
                )?,
            )
        }
        _ => Err(AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::InvalidPayload,
            "invalid audio preview command payload",
        )),
    }
}

fn session_config_from_draft(
    draft: &Draft,
) -> Result<AudioPreviewSessionBindingConfig, AudioPreviewBindingError> {
    Ok(AudioPreviewSessionBindingConfig {
        session_label: "audio-preview".to_owned(),
        frame_rate_numerator: draft.canvas_config.frame_rate.numerator,
        frame_rate_denominator: draft.canvas_config.frame_rate.denominator,
        playback_rate_numerator: 1,
        playback_rate_denominator: 1,
        sample_rate_hz: 48_000,
        max_buffer_duration_microseconds: Microseconds::new(50_000),
        max_channel_count: 2,
        max_frame_count: 2_400,
    })
}

struct ProjectSessionAudioIdentity {
    project_session_id: String,
    expected_revision: u64,
}

struct ProjectSessionAudioContext {
    project_session_id: String,
    draft: Draft,
}

fn project_session_identity_from_payload(
    payload: &AudioPreviewCommandPayload,
    action: &str,
) -> Result<ProjectSessionAudioIdentity, AudioPreviewBindingError> {
    let session_id = payload.project_session_id.as_deref().ok_or_else(|| {
        AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::InvalidPayload,
            format!("{action} requires projectSessionId"),
        )
    })?;
    let expected_revision = payload.expected_revision.ok_or_else(|| {
        AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::InvalidPayload,
            format!("{action} requires expectedRevision"),
        )
    })?;
    crate::project_session_service::project_session_snapshot(session_id, expected_revision)
        .map_err(|message| {
            AudioPreviewBindingError::new(AudioPreviewBindingErrorKind::InvalidPayload, message)
        })?;
    Ok(ProjectSessionAudioIdentity {
        project_session_id: session_id.to_owned(),
        expected_revision,
    })
}

fn project_session_audio_context_from_payload(
    payload: &AudioPreviewCommandPayload,
    action: &str,
) -> Result<ProjectSessionAudioContext, AudioPreviewBindingError> {
    let identity = project_session_identity_from_payload(payload, action)?;
    let snapshot = crate::project_session_service::project_session_snapshot(
        &identity.project_session_id,
        identity.expected_revision,
    )
    .map_err(|message| {
        AudioPreviewBindingError::new(AudioPreviewBindingErrorKind::InvalidPayload, message)
    })?;
    Ok(ProjectSessionAudioContext {
        project_session_id: identity.project_session_id,
        draft: snapshot.draft,
    })
}

fn required_session_id(
    payload: &AudioPreviewCommandPayload,
) -> Result<&str, AudioPreviewBindingError> {
    payload.session_id.as_deref().ok_or_else(|| {
        AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::InvalidPayload,
            "audio preview session ID is required",
        )
    })
}

fn serialize<T: Serialize>(value: T) -> Result<serde_json::Value, AudioPreviewBindingError> {
    serde_json::to_value(value).map_err(|error| {
        AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::RuntimeFailed,
            format!("audio binding response serialization failed: {error}"),
        )
    })
}

fn command_response(
    session_id: &str,
    generation: u64,
    accepted: bool,
    status: AudioPreviewPlaybackStatus,
    status_label: &str,
    target_time: Microseconds,
    diagnostics: Vec<String>,
) -> AudioPreviewCommandResponse {
    AudioPreviewCommandResponse {
        session_id: session_id.to_owned(),
        generation,
        accepted,
        status,
        status_label: status_label.to_owned(),
        target_time,
        diagnostics,
    }
}

fn playback_status(state: PlaybackState) -> AudioPreviewPlaybackStatus {
    match state {
        PlaybackState::Stopped => AudioPreviewPlaybackStatus::Ready,
        PlaybackState::Paused | PlaybackState::Scrubbing => AudioPreviewPlaybackStatus::Ready,
        PlaybackState::Playing => AudioPreviewPlaybackStatus::Playing,
    }
}

fn playback_status_label(status: AudioPreviewPlaybackStatus) -> &'static str {
    match status {
        AudioPreviewPlaybackStatus::Ready => "音频就绪",
        AudioPreviewPlaybackStatus::Playing => "正在播放",
        AudioPreviewPlaybackStatus::Paused => "已暂停",
        AudioPreviewPlaybackStatus::Stopped => "已停止",
        AudioPreviewPlaybackStatus::Buffering => "音频缓冲中",
        AudioPreviewPlaybackStatus::Seeking => "正在定位声音",
        AudioPreviewPlaybackStatus::Canceled => "音频请求已取消",
        AudioPreviewPlaybackStatus::StaleRejected => "声音已同步到最新播放头",
        AudioPreviewPlaybackStatus::Unavailable => "音频暂不可用",
        AudioPreviewPlaybackStatus::Failed => "音频预览失败",
    }
}

fn device_summaries(
    backend: &DesktopAudioOutputBackendCapabilities,
) -> Vec<AudioOutputDeviceSummary> {
    backend
        .devices
        .iter()
        .map(|device| AudioOutputDeviceSummary {
            selection_id: device.device_id.clone(),
            display_name: device.safe_label.clone(),
            status: match backend.status {
                DesktopAudioOutputCapabilityStatus::Ready => AudioOutputDeviceStatus::Ready,
                DesktopAudioOutputCapabilityStatus::Warning => AudioOutputDeviceStatus::Degraded,
                DesktopAudioOutputCapabilityStatus::Unsupported => {
                    AudioOutputDeviceStatus::Unavailable
                }
            },
            status_label: match backend.status {
                DesktopAudioOutputCapabilityStatus::Ready => "输出设备就绪",
                DesktopAudioOutputCapabilityStatus::Warning => "输出设备降级",
                DesktopAudioOutputCapabilityStatus::Unsupported => "未找到输出设备",
            }
            .to_owned(),
            is_default: device.default_device,
            sample_rate_hz: device.sample_rates_hz.first().copied(),
            channel_count: Some(device.max_channel_count),
            diagnostics: backend_diagnostic_messages(backend).collect(),
        })
        .collect()
}

fn native_device_summary(
    device: &audio_output_desktop::DesktopAudioDeviceSummary,
) -> AudioOutputDeviceSummary {
    AudioOutputDeviceSummary {
        selection_id: device.device_id.clone(),
        display_name: device.safe_label.clone(),
        status: AudioOutputDeviceStatus::Ready,
        status_label: "原生输出设备就绪".to_owned(),
        is_default: device.default_device,
        sample_rate_hz: device.sample_rates_hz.first().copied(),
        channel_count: Some(device.max_channel_count),
        diagnostics: vec!["native CPAL output stream is active".to_owned()],
    }
}

fn render_draft_audio_preview(
    draft: &Draft,
    target_time: Microseconds,
    output_sample_rate_hz: u32,
    output_channels: u16,
    duration: Microseconds,
) -> Result<Vec<f32>, AudioPreviewBindingError> {
    if output_sample_rate_hz == 0 || output_channels == 0 {
        return Err(AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::DeviceUnavailable,
            "native audio output reported invalid sample rate or channel count",
        ));
    }
    let output_channels = usize::from(output_channels.min(2));
    let output_frame_count = usize::try_from(
        duration
            .get()
            .saturating_mul(u64::from(output_sample_rate_hz))
            / 1_000_000,
    )
    .unwrap_or(usize::MAX)
    .max(1);
    let mut mixed = vec![0.0f32; output_frame_count.saturating_mul(output_channels)];
    let materials = draft
        .materials
        .iter()
        .map(|material| (material.material_id.clone(), material))
        .collect::<BTreeMap<MaterialId, &Material>>();
    let mut mixed_any = false;

    for segment in draft.tracks.iter().flat_map(|track| track.segments.iter()) {
        let Some(segment_end) = segment.target_timerange.checked_end() else {
            continue;
        };
        if segment_end <= target_time
            || segment.target_timerange.start
                >= Microseconds::new(
                    target_time.get().saturating_add(
                        u64::try_from(output_frame_count)
                            .unwrap_or(0)
                            .saturating_mul(1_000_000)
                            / u64::from(output_sample_rate_hz),
                    ),
                )
        {
            continue;
        }
        let Some(material) = materials.get(&segment.material_id).copied() else {
            continue;
        };
        if !matches!(material.kind, MaterialKind::Audio | MaterialKind::Video)
            || !material.metadata.has_audio
        {
            continue;
        }
        let path = resolve_material_uri(".", &material.uri)
            .map_err(|error| {
                AudioPreviewBindingError::new(
                    AudioPreviewBindingErrorKind::InvalidPayload,
                    format!("audio material path cannot be resolved: {error}"),
                )
            })?
            .ok_or_else(|| {
                AudioPreviewBindingError::new(
                    AudioPreviewBindingErrorKind::InvalidPayload,
                    "audio material URI does not resolve to a local audio source",
                )
            })?;
        if let Ok(source) = read_pcm16_wav(&path) {
            mix_wav_segment(
                &mut mixed,
                output_channels,
                output_sample_rate_hz,
                target_time,
                segment,
                &source,
            );
        } else {
            mix_ffmpeg_audio_segment(
                &mut mixed,
                output_channels,
                output_sample_rate_hz,
                target_time,
                segment,
                &path,
            )?;
        }
        mixed_any = true;
    }

    if !mixed_any {
        return Err(AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::RuntimeFailed,
            "audio preview found no audio-capable timeline segment for the current playback range",
        ));
    }
    Ok(mixed)
}

fn spawn_audio_refill_thread(
    draft: Draft,
    queue: CpalAudioOutputQueue,
    start_time: Microseconds,
    output_sample_rate_hz: u32,
    output_channels: u16,
    stop: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, AudioPreviewBindingError> {
    thread::Builder::new()
        .name("audio-preview-refill".to_owned())
        .spawn(move || {
            run_audio_refill_loop(
                draft,
                queue,
                start_time,
                output_sample_rate_hz,
                output_channels,
                stop,
            );
        })
        .map_err(|error| {
            AudioPreviewBindingError::new(
                AudioPreviewBindingErrorKind::RuntimeFailed,
                format!("failed to start audio preview refill thread: {error}"),
            )
        })
}

fn run_audio_refill_loop(
    draft: Draft,
    queue: CpalAudioOutputQueue,
    mut cursor: Microseconds,
    output_sample_rate_hz: u32,
    output_channels: u16,
    stop: Arc<AtomicBool>,
) {
    loop {
        if stop.load(Ordering::Acquire) {
            return;
        }
        if queued_duration_us(
            queue.queued_sample_count(),
            output_sample_rate_hz,
            output_channels,
        ) >= AUDIO_PREVIEW_REFILL_LOW_WATER.get()
        {
            thread::sleep(AUDIO_PREVIEW_REFILL_POLL_INTERVAL);
            continue;
        }
        let Ok(samples) = render_draft_audio_preview(
            &draft,
            cursor,
            output_sample_rate_hz,
            output_channels,
            AUDIO_PREVIEW_CHUNK_DURATION,
        ) else {
            return;
        };
        if queue.enqueue_f32_interleaved(&samples).is_err() {
            return;
        }
        cursor = Microseconds::new(
            cursor
                .get()
                .saturating_add(AUDIO_PREVIEW_CHUNK_DURATION.get()),
        );
    }
}

fn queued_duration_us(queued_samples: usize, sample_rate_hz: u32, channels: u16) -> u64 {
    if sample_rate_hz == 0 || channels == 0 {
        return 0;
    }
    u64::try_from(queued_samples)
        .unwrap_or(u64::MAX)
        .saturating_mul(1_000_000)
        / u64::from(sample_rate_hz)
        / u64::from(channels)
}

fn mix_ffmpeg_audio_segment(
    output: &mut [f32],
    output_channels: usize,
    output_sample_rate_hz: u32,
    target_time: Microseconds,
    segment: &draft_model::Segment,
    path: &Path,
) -> Result<(), AudioPreviewBindingError> {
    let output_frames = output.len() / output_channels;
    let output_duration_us = u64::try_from(output_frames)
        .unwrap_or(0)
        .saturating_mul(1_000_000)
        / u64::from(output_sample_rate_hz.max(1));
    let preview_start = target_time.get();
    let preview_end = preview_start.saturating_add(output_duration_us);
    let segment_start = segment.target_timerange.start.get();
    let Some(segment_end) = segment.target_timerange.checked_end() else {
        return Ok(());
    };
    let overlap_start = preview_start.max(segment_start);
    let overlap_end = preview_end.min(segment_end.get());
    if overlap_start >= overlap_end {
        return Ok(());
    }

    let source_start = Microseconds::new(
        segment
            .source_timerange
            .start
            .get()
            .saturating_add(overlap_start.saturating_sub(segment_start)),
    );
    let duration = Microseconds::new(overlap_end.saturating_sub(overlap_start));
    let decoded = decode_audio_window_with_ffmpeg(
        path,
        source_start,
        duration,
        output_sample_rate_hz,
        u16::try_from(output_channels).unwrap_or(2),
    )?;
    let output_offset_frame = usize::try_from(
        overlap_start
            .saturating_sub(preview_start)
            .saturating_mul(u64::from(output_sample_rate_hz))
            / 1_000_000,
    )
    .unwrap_or(usize::MAX);
    mix_decoded_audio_window(output, output_channels, output_offset_frame, &decoded);
    Ok(())
}

fn mix_decoded_audio_window(
    output: &mut [f32],
    output_channels: usize,
    output_offset_frame: usize,
    source: &DecodedWavPcm,
) {
    let output_frames = output.len() / output_channels;
    if output_offset_frame >= output_frames {
        return;
    }
    let frames_to_mix = source
        .frame_count
        .min(output_frames.saturating_sub(output_offset_frame));
    for source_frame in 0..frames_to_mix {
        let output_frame = output_offset_frame.saturating_add(source_frame);
        for channel in 0..output_channels {
            let source_channel = channel.min(source.channels.saturating_sub(1));
            let source_index = source_frame
                .saturating_mul(source.channels)
                .saturating_add(source_channel);
            let output_index = output_frame
                .saturating_mul(output_channels)
                .saturating_add(channel);
            output[output_index] =
                (output[output_index] + source.samples[source_index]).clamp(-1.0, 1.0);
        }
    }
}

fn decode_audio_window_with_ffmpeg(
    path: &Path,
    source_start: Microseconds,
    duration: Microseconds,
    output_sample_rate_hz: u32,
    output_channels: u16,
) -> Result<DecodedWavPcm, AudioPreviewBindingError> {
    if duration.get() == 0 {
        return Ok(DecodedWavPcm {
            sample_rate_hz: output_sample_rate_hz,
            channels: usize::from(output_channels.max(1)),
            frame_count: 0,
            samples: Vec::new(),
        });
    }
    let runtime = discover_runtime_config().map_err(|error| {
        AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::DeviceUnavailable,
            format!("audio preview requires FFmpeg to decode embedded media audio: {error}"),
        )
    })?;
    let executor = DesktopFfmpegExecutor::with_timeout(Duration::from_secs(10));
    if !executor.can_execute(&runtime.ffmpeg.path) {
        return Err(AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::DeviceUnavailable,
            format!(
                "audio preview cannot execute FFmpeg at {}",
                runtime.ffmpeg.path.display()
            ),
        ));
    }

    let output_channels = output_channels.max(1).min(2);
    let args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-nostdin"),
        OsString::from("-ss"),
        OsString::from(seconds_arg(source_start)),
        OsString::from("-t"),
        OsString::from(seconds_arg(duration)),
        OsString::from("-i"),
        path.as_os_str().to_os_string(),
        OsString::from("-vn"),
        OsString::from("-ac"),
        OsString::from(output_channels.to_string()),
        OsString::from("-ar"),
        OsString::from(output_sample_rate_hz.to_string()),
        OsString::from("-f"),
        OsString::from("s16le"),
        OsString::from("-acodec"),
        OsString::from("pcm_s16le"),
        OsString::from("pipe:1"),
    ];
    let output = executor.run(&runtime.ffmpeg.path, &args).map_err(|error| {
        AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::RuntimeFailed,
            format!("failed to launch FFmpeg audio decode: {error}"),
        )
    })?;
    if !output.status.success() {
        return Err(AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::RuntimeFailed,
            format!(
                "FFmpeg audio decode failed for {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }
    let channels = usize::from(output_channels);
    let samples = output
        .stdout
        .chunks_exact(2)
        .map(|sample| f32::from(i16::from_le_bytes([sample[0], sample[1]])) / f32::from(i16::MAX))
        .collect::<Vec<_>>();
    let frame_count = samples.len() / channels;
    Ok(DecodedWavPcm {
        sample_rate_hz: output_sample_rate_hz,
        channels,
        frame_count,
        samples,
    })
}

fn seconds_arg(value: Microseconds) -> String {
    let seconds = value.get() / 1_000_000;
    let micros = value.get() % 1_000_000;
    format!("{seconds}.{micros:06}")
}

fn mix_wav_segment(
    output: &mut [f32],
    output_channels: usize,
    output_sample_rate_hz: u32,
    target_time: Microseconds,
    segment: &draft_model::Segment,
    source: &DecodedWavPcm,
) {
    let output_frames = output.len() / output_channels;
    for output_frame in 0..output_frames {
        let timeline_time = target_time.get().saturating_add(
            u64::try_from(output_frame)
                .unwrap_or(0)
                .saturating_mul(1_000_000)
                / u64::from(output_sample_rate_hz),
        );
        if timeline_time < segment.target_timerange.start.get() {
            continue;
        }
        let segment_offset = timeline_time - segment.target_timerange.start.get();
        if segment_offset >= segment.target_timerange.duration.get() {
            continue;
        }
        let source_time = segment
            .source_timerange
            .start
            .get()
            .saturating_add(segment_offset);
        if source_time
            >= segment
                .source_timerange
                .start
                .get()
                .saturating_add(segment.source_timerange.duration.get())
        {
            continue;
        }
        let source_frame = usize::try_from(
            source_time.saturating_mul(u64::from(source.sample_rate_hz)) / 1_000_000,
        )
        .unwrap_or(usize::MAX);
        if source_frame >= source.frame_count {
            continue;
        }
        for channel in 0..output_channels {
            let source_channel = channel.min(source.channels.saturating_sub(1));
            let source_index = source_frame
                .saturating_mul(source.channels)
                .saturating_add(source_channel);
            let output_index = output_frame
                .saturating_mul(output_channels)
                .saturating_add(channel);
            output[output_index] =
                (output[output_index] + source.samples[source_index]).clamp(-1.0, 1.0);
        }
    }
}

#[derive(Debug, Clone)]
struct DecodedWavPcm {
    sample_rate_hz: u32,
    channels: usize,
    frame_count: usize,
    samples: Vec<f32>,
}

fn read_pcm16_wav(path: &std::path::Path) -> Result<DecodedWavPcm, AudioPreviewBindingError> {
    let bytes = fs::read(path).map_err(|error| {
        AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::RuntimeFailed,
            format!("failed to read audio material {}: {error}", path.display()),
        )
    })?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::RuntimeFailed,
            "audio preview currently requires PCM WAV material",
        ));
    }

    let mut offset = 12usize;
    let mut format: Option<(u16, u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;
    while offset.saturating_add(8) <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size =
            u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        offset = offset.saturating_add(8);
        if offset.saturating_add(chunk_size) > bytes.len() {
            break;
        }
        let chunk = &bytes[offset..offset + chunk_size];
        if chunk_id == b"fmt " && chunk.len() >= 16 {
            format = Some((
                u16::from_le_bytes(chunk[0..2].try_into().unwrap()),
                u16::from_le_bytes(chunk[2..4].try_into().unwrap()),
                u32::from_le_bytes(chunk[4..8].try_into().unwrap()),
                u16::from_le_bytes(chunk[14..16].try_into().unwrap()),
            ));
        } else if chunk_id == b"data" {
            data = Some(chunk);
        }
        offset = offset.saturating_add(chunk_size + (chunk_size % 2));
    }

    let (format_tag, channels, sample_rate_hz, bits_per_sample) = format.ok_or_else(|| {
        AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::RuntimeFailed,
            "PCM WAV fmt chunk is missing",
        )
    })?;
    if format_tag != 1 || bits_per_sample != 16 || channels == 0 {
        return Err(AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::RuntimeFailed,
            "audio preview currently supports PCM s16le WAV only",
        ));
    }
    let data = data.ok_or_else(|| {
        AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::RuntimeFailed,
            "PCM WAV data chunk is missing",
        )
    })?;
    let channels = usize::from(channels);
    let samples = data
        .chunks_exact(2)
        .map(|sample| f32::from(i16::from_le_bytes([sample[0], sample[1]])) / f32::from(i16::MAX))
        .collect::<Vec<_>>();
    let frame_count = samples.len() / channels;
    Ok(DecodedWavPcm {
        sample_rate_hz,
        channels,
        frame_count,
        samples,
    })
}

fn backend_diagnostic_messages(
    backend: &DesktopAudioOutputBackendCapabilities,
) -> impl Iterator<Item = String> + '_ {
    backend
        .fallback_reason
        .iter()
        .cloned()
        .chain(backend.diagnostic.iter().cloned())
}

fn missing_device_summary() -> AudioOutputDeviceSummary {
    AudioOutputDeviceSummary {
        selection_id: "system-default".to_owned(),
        display_name: "系统默认".to_owned(),
        status: AudioOutputDeviceStatus::Missing,
        status_label: "未找到输出设备".to_owned(),
        is_default: true,
        sample_rate_hz: None,
        channel_count: None,
        diagnostics: vec!["audio output device summary unavailable".to_owned()],
    }
}

fn validate_binding_session_id(session_id: &str) -> Result<(), AudioPreviewBindingError> {
    let suffix = session_id.strip_prefix(SESSION_PREFIX).ok_or_else(|| {
        AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::MalformedSessionId,
            "audio preview session IDs are opaque binding IDs",
        )
    })?;
    if suffix.len() != 16
        || !suffix
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(AudioPreviewBindingError::new(
            AudioPreviewBindingErrorKind::MalformedSessionId,
            "audio preview session ID has invalid shape",
        ));
    }
    Ok(())
}

fn audio_error_envelope(
    error: AudioPreviewBindingError,
    command: CommandName,
) -> CommandResultEnvelope<serde_json::Value> {
    CommandResultEnvelope {
        ok: false,
        data: None,
        error: Some(CommandError {
            kind: CommandErrorKind::PreviewServiceFailed,
            message: error.to_string(),
            command: command_wire_name(&command),
        }),
        events: Vec::new(),
    }
}

fn command_wire_name(command: &CommandName) -> Option<String> {
    Some(
        serde_json::to_value(command)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| format!("{command:?}")),
    )
}

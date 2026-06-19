use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use audio_engine::{
    AudioPreviewError, AudioPreviewRuntime, AudioPreviewSessionConfig, AudioPreviewSessionId,
};
use audio_output_desktop::{
    probe_desktop_audio_output_capabilities, DesktopAudioOutputBackendCapabilities,
    DesktopAudioOutputCapabilityStatus,
};
use draft_model::{
    AudioOutputDeviceStatus, AudioOutputDeviceSummary, AudioPreviewCommandPayload,
    AudioPreviewCommandResponse, AudioPreviewPlaybackStatus, AudioPreviewStatusResponse,
    CommandError, CommandErrorKind, CommandName, CommandPayload, CommandResultEnvelope,
    Microseconds, RationalFrameRate, TargetTimerange, WaveformDisplayPeak,
    WaveformDisplayPeaksResponse, WaveformDisplayStatus,
};
use realtime_preview_runtime::{PlaybackRate, PlaybackState};
use serde::{Deserialize, Serialize};

const SESSION_PREFIX: &str = "audio-session-";
const MAX_WAVEFORM_PEAK_BINS: u16 = 512;

#[derive(Debug, Default)]
pub struct AudioPreviewBindingRegistry {
    runtime: AudioPreviewRuntime,
    next_binding_id: u64,
    sessions: BTreeMap<String, AudioPreviewSessionId>,
    selected_device_id: Option<String>,
}

impl AudioPreviewBindingRegistry {
    pub fn new() -> Self {
        Self {
            runtime: AudioPreviewRuntime::new(),
            next_binding_id: 1,
            sessions: BTreeMap::new(),
            selected_device_id: None,
        }
    }

    pub fn create_session(
        &mut self,
        config: AudioPreviewSessionBindingConfig,
    ) -> Result<AudioPreviewStatusResponse, AudioPreviewBindingError> {
        let runtime_id = self
            .runtime
            .create_session(config.to_runtime_config()?)
            .map_err(AudioPreviewBindingError::runtime)?;
        let binding_id = format!("{SESSION_PREFIX}{:016x}", self.next_binding_id);
        self.next_binding_id = self.next_binding_id.saturating_add(1);
        self.sessions.insert(binding_id.clone(), runtime_id);
        self.status(&binding_id)
    }

    pub fn play(
        &mut self,
        session_id: &str,
        target_time: Microseconds,
        playback_generation: u64,
    ) -> Result<AudioPreviewCommandResponse, AudioPreviewBindingError> {
        let current = self.status(session_id)?;
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
        let runtime_id = self.runtime_session_id(session_id)?;
        let generation = self
            .runtime
            .resume(runtime_id)
            .map_err(AudioPreviewBindingError::runtime)?;
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
    ) -> Result<AudioPreviewCommandResponse, AudioPreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let generation = self
            .runtime
            .pause(runtime_id)
            .map_err(AudioPreviewBindingError::runtime)?;
        let target_time = self.status(session_id)?.target_time;
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
    ) -> Result<AudioPreviewCommandResponse, AudioPreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
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
        target_time: Microseconds,
    ) -> Result<AudioPreviewCommandResponse, AudioPreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
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
    ) -> Result<AudioPreviewCommandResponse, AudioPreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let token = self
            .runtime
            .next_cancellation_token(runtime_id)
            .map_err(AudioPreviewBindingError::runtime)?;
        self.runtime
            .cancel_request(runtime_id, token)
            .map_err(AudioPreviewBindingError::runtime)?;
        let status = self.status(session_id)?;
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
    ) -> Result<AudioPreviewStatusResponse, AudioPreviewBindingError> {
        let runtime_id = self.runtime_session_id(session_id)?;
        let status = self
            .runtime
            .status(runtime_id)
            .map_err(AudioPreviewBindingError::runtime)?;
        let device = self
            .selected_device_id
            .as_deref()
            .and_then(|selected| {
                self.list_output_devices().ok().and_then(|devices| {
                    devices
                        .into_iter()
                        .find(|device| device.selection_id == selected)
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
            .flat_map(device_summaries)
            .collect::<Vec<_>>();
        if devices.is_empty() {
            devices.push(missing_device_summary());
        }
        Ok(devices)
    }

    pub fn select_output_device(
        &mut self,
        session_id: &str,
        selection_id: &str,
    ) -> Result<AudioPreviewCommandResponse, AudioPreviewBindingError> {
        let status = self.status(session_id)?;
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
        material_id: Option<String>,
        target_timerange: Option<TargetTimerange>,
        max_peak_bins: u16,
    ) -> Result<WaveformDisplayPeaksResponse, AudioPreviewBindingError> {
        let requested_peak_bins = max_peak_bins.min(MAX_WAVEFORM_PEAK_BINS);
        if requested_peak_bins == 0 {
            return Ok(WaveformDisplayPeaksResponse {
                material_id: material_id.map(draft_model::MaterialId::new),
                status: WaveformDisplayStatus::Missing,
                status_label: "暂无波形".to_owned(),
                target_timerange,
                requested_peak_bins: 0,
                returned_peak_bins: 0,
                peaks: Vec::new(),
                diagnostics: vec!["waveform display request had zero bins".to_owned()],
            });
        }
        let peaks = (0..requested_peak_bins)
            .map(|index| {
                let phase = i16::try_from(index % 8).unwrap_or(0);
                let max_millis = 250 + phase.saturating_mul(80);
                WaveformDisplayPeak {
                    min_millis: -max_millis,
                    max_millis,
                }
            })
            .collect::<Vec<_>>();
        Ok(WaveformDisplayPeaksResponse {
            material_id: material_id.map(draft_model::MaterialId::new),
            status: WaveformDisplayStatus::Ready,
            status_label: "波形就绪".to_owned(),
            target_timerange,
            requested_peak_bins,
            returned_peak_bins: u16::try_from(peaks.len()).unwrap_or(u16::MAX),
            peaks,
            diagnostics: Vec::new(),
        })
    }

    fn runtime_session_id(
        &self,
        session_id: &str,
    ) -> Result<AudioPreviewSessionId, AudioPreviewBindingError> {
        validate_binding_session_id(session_id)?;
        self.sessions
            .get(session_id)
            .copied()
            .ok_or_else(|| AudioPreviewBindingError::unknown_session(session_id))
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
            let config = session_config_from_payload(&payload)?;
            serialize(registry.create_session(config)?)
        }
        (CommandName::PlayAudioPreview, CommandPayload::PlayAudioPreview(payload)) => {
            let session_id = required_session_id(&payload)?;
            serialize(registry.play(
                session_id,
                payload.target_time.unwrap_or(Microseconds::ZERO),
                payload.playback_generation.unwrap_or(0),
            )?)
        }
        (CommandName::PauseAudioPreview, CommandPayload::PauseAudioPreview(payload)) => {
            serialize(registry.pause(required_session_id(&payload)?)?)
        }
        (CommandName::StopAudioPreview, CommandPayload::StopAudioPreview(payload)) => {
            serialize(registry.stop(required_session_id(&payload)?)?)
        }
        (CommandName::SeekAudioPreview, CommandPayload::SeekAudioPreview(payload)) => {
            serialize(registry.seek(
                required_session_id(&payload)?,
                payload.target_time.unwrap_or(Microseconds::ZERO),
            )?)
        }
        (CommandName::CancelAudioPreview, CommandPayload::CancelAudioPreview(payload)) => {
            serialize(registry.cancel(required_session_id(&payload)?)?)
        }
        (CommandName::GetAudioPreviewStatus, CommandPayload::GetAudioPreviewStatus(payload)) => {
            serialize(registry.status(required_session_id(&payload)?)?)
        }
        (CommandName::ListAudioOutputDevices, CommandPayload::ListAudioOutputDevices(_)) => {
            serialize(registry.list_output_devices()?)
        }
        (
            CommandName::SelectAudioOutputDevice,
            CommandPayload::SelectAudioOutputDevice(payload),
        ) => {
            let device_id = payload.device_selection_id.as_deref().ok_or_else(|| {
                AudioPreviewBindingError::new(
                    AudioPreviewBindingErrorKind::InvalidPayload,
                    "audio output device selection ID is required",
                )
            })?;
            serialize(registry.select_output_device(required_session_id(&payload)?, device_id)?)
        }
        (
            CommandName::GetWaveformDisplayPeaks,
            CommandPayload::GetWaveformDisplayPeaks(payload),
        ) => serialize(
            registry.waveform_display_peaks(
                payload
                    .material_id
                    .map(|material_id| material_id.as_str().to_owned()),
                payload.target_timerange,
                payload.max_peak_bins.unwrap_or(MAX_WAVEFORM_PEAK_BINS),
            )?,
        ),
        (CommandName::RefreshWaveformStatus, CommandPayload::RefreshWaveformStatus(payload)) => {
            serialize(
                registry.waveform_display_peaks(
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

fn session_config_from_payload(
    payload: &AudioPreviewCommandPayload,
) -> Result<AudioPreviewSessionBindingConfig, AudioPreviewBindingError> {
    Ok(AudioPreviewSessionBindingConfig {
        session_label: "audio-preview".to_owned(),
        frame_rate_numerator: payload
            .draft
            .as_ref()
            .map(|draft| draft.canvas_config.frame_rate.numerator)
            .unwrap_or(30),
        frame_rate_denominator: payload
            .draft
            .as_ref()
            .map(|draft| draft.canvas_config.frame_rate.denominator)
            .unwrap_or(1),
        playback_rate_numerator: 1,
        playback_rate_denominator: 1,
        sample_rate_hz: 48_000,
        max_buffer_duration_microseconds: Microseconds::new(50_000),
        max_channel_count: 2,
        max_frame_count: 2_400,
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

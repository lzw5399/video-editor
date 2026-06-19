use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use draft_model::{Microseconds, RationalFrameRate};
use realtime_preview_runtime::{PlaybackGeneration, PlaybackRate, PlaybackState, TimelineClock};
use serde::{Deserialize, Serialize};

use crate::telemetry::AudioPreviewTelemetry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AudioPreviewSessionId(u64);

impl AudioPreviewSessionId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AudioCancellationToken(u64);

impl AudioCancellationToken {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioPreviewSessionConfig {
    pub session_label: String,
    pub frame_rate: RationalFrameRate,
    pub playback_rate: PlaybackRate,
    pub sample_rate_hz: u32,
    pub max_buffer_duration_microseconds: Microseconds,
    pub max_channel_count: u16,
    pub max_frame_count: u32,
}

#[derive(Debug, Default)]
pub struct AudioPreviewRuntime {
    next_session_id: u64,
    sessions: BTreeMap<AudioPreviewSessionId, AudioPreviewSession>,
}

impl AudioPreviewRuntime {
    pub fn new() -> Self {
        Self {
            next_session_id: 1,
            sessions: BTreeMap::new(),
        }
    }

    pub fn create_session(
        &mut self,
        config: AudioPreviewSessionConfig,
    ) -> Result<AudioPreviewSessionId, AudioPreviewError> {
        validate_config(&config)?;
        let session_id = AudioPreviewSessionId::new(self.next_session_id);
        self.next_session_id = self.next_session_id.saturating_add(1);
        let clock = TimelineClock::new(
            Microseconds::ZERO,
            config.frame_rate.clone(),
            config.playback_rate,
        );
        self.sessions
            .insert(session_id, AudioPreviewSession::new(config, clock));
        Ok(session_id)
    }

    pub fn close_session(&mut self, session_id: AudioPreviewSessionId) -> bool {
        self.sessions.remove(&session_id).is_some()
    }

    pub fn status(
        &self,
        session_id: AudioPreviewSessionId,
    ) -> Result<AudioPreviewStatus, AudioPreviewError> {
        Ok(self.session(session_id)?.status())
    }

    pub fn telemetry(
        &self,
        session_id: AudioPreviewSessionId,
    ) -> Result<AudioPreviewTelemetry, AudioPreviewError> {
        Ok(self.session(session_id)?.telemetry.clone())
    }

    pub fn seek(
        &mut self,
        session_id: AudioPreviewSessionId,
        target_time: Microseconds,
    ) -> Result<PlaybackGeneration, AudioPreviewError> {
        let session = self.session_mut(session_id)?;
        session.clock.seek(target_time);
        session.sync_generation();
        Ok(session.clock.generation())
    }

    pub fn pause(
        &mut self,
        session_id: AudioPreviewSessionId,
    ) -> Result<PlaybackGeneration, AudioPreviewError> {
        let session = self.session_mut(session_id)?;
        session.clock.pause();
        session.sync_generation();
        Ok(session.clock.generation())
    }

    pub fn stop(
        &mut self,
        session_id: AudioPreviewSessionId,
    ) -> Result<PlaybackGeneration, AudioPreviewError> {
        let session = self.session_mut(session_id)?;
        session.clock.stop();
        session.sync_generation();
        Ok(session.clock.generation())
    }

    pub fn resume(
        &mut self,
        session_id: AudioPreviewSessionId,
    ) -> Result<PlaybackGeneration, AudioPreviewError> {
        let session = self.session_mut(session_id)?;
        session.clock.resume();
        session.sync_generation();
        Ok(session.clock.generation())
    }

    pub fn accepted_edit(
        &mut self,
        session_id: AudioPreviewSessionId,
    ) -> Result<PlaybackGeneration, AudioPreviewError> {
        let session = self.session_mut(session_id)?;
        session.clock.accepted_edit();
        session.sync_generation();
        Ok(session.clock.generation())
    }

    pub fn draft_reloaded(
        &mut self,
        session_id: AudioPreviewSessionId,
    ) -> Result<PlaybackGeneration, AudioPreviewError> {
        let session = self.session_mut(session_id)?;
        session.clock.draft_reloaded();
        session.sync_generation();
        Ok(session.clock.generation())
    }

    pub fn material_relinked(
        &mut self,
        session_id: AudioPreviewSessionId,
    ) -> Result<PlaybackGeneration, AudioPreviewError> {
        let session = self.session_mut(session_id)?;
        session.clock.material_relinked();
        session.sync_generation();
        Ok(session.clock.generation())
    }

    pub fn next_cancellation_token(
        &mut self,
        session_id: AudioPreviewSessionId,
    ) -> Result<AudioCancellationToken, AudioPreviewError> {
        let session = self.session_mut(session_id)?;
        let token = AudioCancellationToken::new(session.next_cancellation_token);
        session.next_cancellation_token = session.next_cancellation_token.saturating_add(1);
        Ok(token)
    }

    pub fn cancel_request(
        &mut self,
        session_id: AudioPreviewSessionId,
        token: AudioCancellationToken,
    ) -> Result<(), AudioPreviewError> {
        let session = self.session_mut(session_id)?;
        session.canceled_tokens.insert(token);
        Ok(())
    }

    pub fn request_buffer(
        &mut self,
        session_id: AudioPreviewSessionId,
        request: AudioBufferRequest,
    ) -> Result<AudioBufferResult, AudioPreviewError> {
        let session = self.session_mut(session_id)?;
        Ok(session.request_buffer(request))
    }

    fn session(
        &self,
        session_id: AudioPreviewSessionId,
    ) -> Result<&AudioPreviewSession, AudioPreviewError> {
        self.sessions
            .get(&session_id)
            .ok_or(AudioPreviewError::UnknownSession { session_id })
    }

    fn session_mut(
        &mut self,
        session_id: AudioPreviewSessionId,
    ) -> Result<&mut AudioPreviewSession, AudioPreviewError> {
        self.sessions
            .get_mut(&session_id)
            .ok_or(AudioPreviewError::UnknownSession { session_id })
    }
}

#[derive(Debug)]
struct AudioPreviewSession {
    config: AudioPreviewSessionConfig,
    clock: TimelineClock,
    telemetry: AudioPreviewTelemetry,
    canceled_tokens: BTreeSet<AudioCancellationToken>,
    next_cancellation_token: u64,
}

impl AudioPreviewSession {
    fn new(config: AudioPreviewSessionConfig, clock: TimelineClock) -> Self {
        Self {
            telemetry: AudioPreviewTelemetry::new(clock.position(), clock.generation()),
            config,
            clock,
            canceled_tokens: BTreeSet::new(),
            next_cancellation_token: 1,
        }
    }

    fn status(&self) -> AudioPreviewStatus {
        AudioPreviewStatus {
            session_label: self.config.session_label.clone(),
            target_time: self.clock.position(),
            playback_generation: self.clock.generation(),
            playback_state: self.clock.state(),
            status_label: match self.clock.state() {
                PlaybackState::Playing => AudioPreviewStatusLabel::Playing,
                PlaybackState::Paused | PlaybackState::Scrubbing => AudioPreviewStatusLabel::Ready,
                PlaybackState::Stopped => AudioPreviewStatusLabel::Stopped,
            },
            telemetry: self.telemetry.clone(),
        }
    }

    fn request_buffer(&mut self, request: AudioBufferRequest) -> AudioBufferResult {
        let stale_rejected = request.playback_generation != self.clock.generation();
        let canceled = request
            .cancellation_token
            .map(|token| self.canceled_tokens.contains(&token))
            .unwrap_or(false);
        let bounded_rejected = self.bound_violations(&request).is_some();
        let presented = !stale_rejected && !canceled && !bounded_rejected;
        let presented_frame_count = if presented {
            request.requested_frame_count
        } else {
            0
        };

        self.telemetry.record_buffer(
            &request,
            presented,
            stale_rejected,
            canceled,
            bounded_rejected,
        );

        let diagnostics =
            self.diagnostics_for(&request, stale_rejected, canceled, bounded_rejected);
        AudioBufferResult {
            target_time: request.target_time,
            playback_generation: request.playback_generation,
            requested_frame_count: request.requested_frame_count,
            presented_frame_count,
            channel_count: request.channel_count,
            sample_rate_hz: request.sample_rate_hz,
            max_buffer_duration_microseconds: self.config.max_buffer_duration_microseconds,
            cancellation_token: request.cancellation_token,
            presented,
            stale_rejected,
            canceled,
            status_label: if presented {
                AudioPreviewStatusLabel::Presented
            } else {
                AudioPreviewStatusLabel::Rejected
            },
            diagnostics,
            telemetry: self.telemetry.clone(),
        }
    }

    fn sync_generation(&mut self) {
        self.telemetry.generation = self.clock.generation();
        self.telemetry.target_time = self.clock.position();
    }

    fn bound_violations(&self, request: &AudioBufferRequest) -> Option<&'static str> {
        if request.sample_rate_hz != self.config.sample_rate_hz {
            return Some("sample rate does not match audio session");
        }
        if request.channel_count == 0 || request.channel_count > self.config.max_channel_count {
            return Some("channel count exceeds audio session bounds");
        }
        if request.requested_frame_count == 0
            || request.requested_frame_count > self.config.max_frame_count
        {
            return Some("frame count exceeds audio session bounds");
        }
        if request.max_buffer_duration_microseconds > self.config.max_buffer_duration_microseconds {
            return Some("buffer duration exceeds audio session bounds");
        }
        None
    }

    fn diagnostics_for(
        &self,
        request: &AudioBufferRequest,
        stale_rejected: bool,
        canceled: bool,
        bounded_rejected: bool,
    ) -> Vec<AudioPreviewDiagnostic> {
        let mut diagnostics = Vec::new();
        if stale_rejected {
            diagnostics.push(AudioPreviewDiagnostic {
                target_time: request.target_time,
                playback_generation: request.playback_generation,
                message: "audio buffer rejected because playback generation is stale".to_owned(),
                stale: true,
                canceled: false,
                bounded: false,
                degraded_output: false,
            });
        }
        if canceled {
            diagnostics.push(AudioPreviewDiagnostic {
                target_time: request.target_time,
                playback_generation: request.playback_generation,
                message: "audio buffer rejected because request was canceled".to_owned(),
                stale: false,
                canceled: true,
                bounded: false,
                degraded_output: false,
            });
        }
        if bounded_rejected {
            diagnostics.push(AudioPreviewDiagnostic {
                target_time: request.target_time,
                playback_generation: request.playback_generation,
                message: self
                    .bound_violations(request)
                    .unwrap_or("audio buffer request exceeds session bounds")
                    .to_owned(),
                stale: false,
                canceled: false,
                bounded: true,
                degraded_output: false,
            });
        }
        diagnostics
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioBufferRequest {
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub requested_frame_count: u32,
    pub channel_count: u16,
    pub sample_rate_hz: u32,
    pub max_buffer_duration_microseconds: Microseconds,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_token: Option<AudioCancellationToken>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioBufferResult {
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub requested_frame_count: u32,
    pub presented_frame_count: u32,
    pub channel_count: u16,
    pub sample_rate_hz: u32,
    pub max_buffer_duration_microseconds: Microseconds,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_token: Option<AudioCancellationToken>,
    pub presented: bool,
    pub stale_rejected: bool,
    pub canceled: bool,
    pub status_label: AudioPreviewStatusLabel,
    pub diagnostics: Vec<AudioPreviewDiagnostic>,
    pub telemetry: AudioPreviewTelemetry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioPreviewStatus {
    pub session_label: String,
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub playback_state: PlaybackState,
    pub status_label: AudioPreviewStatusLabel,
    pub telemetry: AudioPreviewTelemetry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioPreviewStatusLabel {
    Stopped,
    Ready,
    Playing,
    Presented,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioPreviewDiagnostic {
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub message: String,
    pub stale: bool,
    pub canceled: bool,
    pub bounded: bool,
    pub degraded_output: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioPreviewError {
    UnknownSession { session_id: AudioPreviewSessionId },
    InvalidConfig { reason: String },
}

impl fmt::Display for AudioPreviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSession { session_id } => {
                write!(
                    formatter,
                    "unknown audio preview session {}",
                    session_id.get()
                )
            }
            Self::InvalidConfig { reason } => write!(formatter, "invalid audio session: {reason}"),
        }
    }
}

impl Error for AudioPreviewError {}

fn validate_config(config: &AudioPreviewSessionConfig) -> Result<(), AudioPreviewError> {
    if config.sample_rate_hz == 0 {
        return Err(AudioPreviewError::InvalidConfig {
            reason: "sample rate must be nonzero".to_owned(),
        });
    }
    if config.max_channel_count == 0 {
        return Err(AudioPreviewError::InvalidConfig {
            reason: "channel count bound must be nonzero".to_owned(),
        });
    }
    if config.max_frame_count == 0 {
        return Err(AudioPreviewError::InvalidConfig {
            reason: "frame count bound must be nonzero".to_owned(),
        });
    }
    if config.max_buffer_duration_microseconds == Microseconds::ZERO {
        return Err(AudioPreviewError::InvalidConfig {
            reason: "buffer duration bound must be nonzero".to_owned(),
        });
    }
    Ok(())
}

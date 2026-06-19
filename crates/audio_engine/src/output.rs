use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{AudioBufferResult, AudioPreviewStatusLabel};

pub trait AudioOutputDevice {
    type Stream: AudioOutputSink;

    fn capabilities(&self) -> AudioOutputCapabilities;
    fn open_stream(
        &self,
        capabilities: &AudioOutputCapabilities,
    ) -> Result<Self::Stream, AudioOutputError>;
}

pub trait AudioOutputSink {
    fn present(&mut self, result: &AudioBufferResult) -> Result<(), AudioOutputError>;
    fn presented_result_count(&self) -> u64;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioOutputCapabilities {
    pub device_id: String,
    pub display_name: String,
    pub sample_rate_hz: u32,
    pub max_channel_count: u16,
    pub max_frame_count: u32,
    pub mock: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioOutputError {
    InvalidCapabilities { reason: String },
}

impl fmt::Display for AudioOutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCapabilities { reason } => {
                write!(formatter, "invalid audio output capabilities: {reason}")
            }
        }
    }
}

impl Error for AudioOutputError {}

#[derive(Debug, Clone)]
pub struct MockAudioOutputDevice {
    capabilities: AudioOutputCapabilities,
}

impl Default for MockAudioOutputDevice {
    fn default() -> Self {
        Self {
            capabilities: AudioOutputCapabilities {
                device_id: "mock-audio-output".to_owned(),
                display_name: "Mock audio output".to_owned(),
                sample_rate_hz: 48_000,
                max_channel_count: 2,
                max_frame_count: 2_400,
                mock: true,
            },
        }
    }
}

impl MockAudioOutputDevice {
    pub fn new(capabilities: AudioOutputCapabilities) -> Self {
        Self { capabilities }
    }
}

impl AudioOutputDevice for MockAudioOutputDevice {
    type Stream = MockAudioOutputSink;

    fn capabilities(&self) -> AudioOutputCapabilities {
        self.capabilities.clone()
    }

    fn open_stream(
        &self,
        capabilities: &AudioOutputCapabilities,
    ) -> Result<Self::Stream, AudioOutputError> {
        if capabilities.sample_rate_hz == 0
            || capabilities.max_channel_count == 0
            || capabilities.max_frame_count == 0
        {
            return Err(AudioOutputError::InvalidCapabilities {
                reason: "mock output requires nonzero bounds".to_owned(),
            });
        }
        Ok(MockAudioOutputSink {
            capabilities: capabilities.clone(),
            presented_result_count: 0,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MockAudioOutputSink {
    capabilities: AudioOutputCapabilities,
    presented_result_count: u64,
}

impl AudioOutputSink for MockAudioOutputSink {
    fn present(&mut self, result: &AudioBufferResult) -> Result<(), AudioOutputError> {
        if !result.presented {
            self.presented_result_count = self.presented_result_count.saturating_add(1);
            return Ok(());
        }
        if result.sample_rate_hz != self.capabilities.sample_rate_hz {
            return Err(AudioOutputError::InvalidCapabilities {
                reason: "result sample rate does not match mock output".to_owned(),
            });
        }
        if result.channel_count > self.capabilities.max_channel_count {
            return Err(AudioOutputError::InvalidCapabilities {
                reason: "result channel count exceeds mock output".to_owned(),
            });
        }
        if result.requested_frame_count > self.capabilities.max_frame_count {
            return Err(AudioOutputError::InvalidCapabilities {
                reason: "result frame count exceeds mock output".to_owned(),
            });
        }
        let _status = result.status_label == AudioPreviewStatusLabel::Presented;
        self.presented_result_count = self.presented_result_count.saturating_add(1);
        Ok(())
    }

    fn presented_result_count(&self) -> u64 {
        self.presented_result_count
    }
}

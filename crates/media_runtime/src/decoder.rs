use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{DecodedAudioFrame, DecodedVideoFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DecodeErrorKind {
    Unsupported,
    EndOfStream,
    InvalidRequest,
    RuntimeFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[error("decode failed: {message}")]
#[serde(rename_all = "camelCase")]
pub struct DecodeError {
    pub kind: DecodeErrorKind,
    pub message: String,
}

impl DecodeError {
    pub fn new(kind: DecodeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoDecodeRequest {
    pub source_time_us: u64,
    pub playback_generation: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDecodeRequest {
    pub start_time_us: u64,
    pub duration_us: u64,
}

pub trait VideoDecoder {
    fn decoder_name(&self) -> &'static str;
    fn decode_at(&mut self, request: VideoDecodeRequest) -> Result<DecodedVideoFrame, DecodeError>;
    fn flush(&mut self) -> Result<(), DecodeError>;
}

pub trait AudioDecoder {
    fn decoder_name(&self) -> &'static str;
    fn read_range(
        &mut self,
        request: AudioDecodeRequest,
    ) -> Result<DecodedAudioFrame, DecodeError>;
    fn flush(&mut self) -> Result<(), DecodeError>;
}

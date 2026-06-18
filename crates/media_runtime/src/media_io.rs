use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    AudioDecoder, FrameDimensions, MaterialProbeMetadata, RationalFrameRate,
    RuntimeCapabilityReport, VideoColorMetadata, VideoDecoder, VideoPixelFormat,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MediaSessionId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StreamId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaOpenRequest {
    pub material_uri: PathBuf,
    pub requested_streams: Vec<StreamId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaProbeRequest {
    pub material_uri: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaProbeReport {
    pub metadata: Option<MaterialProbeMetadata>,
    pub streams: Vec<MediaStreamInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaStreamKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaStreamInfo {
    pub stream_id: StreamId,
    pub kind: MediaStreamKind,
    pub codec: String,
    pub duration_us: Option<u64>,
    pub frame_rate: Option<RationalFrameRate>,
    pub dimensions: Option<FrameDimensions>,
    pub pixel_format: Option<VideoPixelFormat>,
    pub color: Option<VideoColorMetadata>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaIoErrorKind {
    OpenFailed,
    StreamNotFound,
    UnsupportedStream,
    RuntimeUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[error("media IO failed: {message}")]
#[serde(rename_all = "camelCase")]
pub struct MediaIoError {
    pub kind: MediaIoErrorKind,
    pub message: String,
}

impl MediaIoError {
    pub fn new(kind: MediaIoErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

pub trait MediaProbeService {
    fn probe_material(&self, request: MediaProbeRequest) -> Result<MediaProbeReport, MediaIoError>;
    fn probe_runtime_capabilities(&self) -> RuntimeCapabilityReport;
}

pub trait MediaReader {
    fn reader_name(&self) -> &'static str;
    fn open(&self, request: MediaOpenRequest) -> Result<Box<dyn MediaSession>, MediaIoError>;
}

pub trait MediaSession {
    fn session_id(&self) -> MediaSessionId;
    fn streams(&self) -> &[MediaStreamInfo];
    fn video_decoder(&self, stream_id: StreamId) -> Result<Box<dyn VideoDecoder>, MediaIoError>;
    fn audio_decoder(&self, stream_id: StreamId) -> Result<Box<dyn AudioDecoder>, MediaIoError>;
}

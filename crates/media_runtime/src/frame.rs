use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{MediaSessionId, TextureHandle, TextureHandleId, VideoColorMetadata, VideoPixelFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FrameHandleId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FrameLeaseId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FramePoolLimits {
    pub max_outstanding_leases: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FrameStorageRequest {
    Cpu { estimated_byte_len: usize },
    Texture(TextureHandle),
    PlatformOpaque { label: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameLeaseRequest {
    pub playback_generation: Option<u64>,
    pub source_time_us: u64,
    pub duration_us: Option<u64>,
    pub frame_index: Option<u64>,
    pub dimensions: FrameDimensions,
    pub pixel_format: VideoPixelFormat,
    pub color: VideoColorMetadata,
    pub storage: FrameStorageRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuFrameHandle {
    pub handle_id: FrameHandleId,
    pub owner_session: MediaSessionId,
    pub generation: Option<u64>,
    pub dimensions: FrameDimensions,
    pub pixel_format: VideoPixelFormat,
    pub estimated_byte_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformFrameHandle {
    pub handle_id: FrameHandleId,
    pub owner_session: MediaSessionId,
    pub generation: Option<u64>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "handle")]
pub enum VideoFrameStorage {
    Cpu(CpuFrameHandle),
    Texture(TextureHandle),
    PlatformOpaque(PlatformFrameHandle),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedVideoFrame {
    pub handle_id: FrameHandleId,
    pub owner_session: MediaSessionId,
    pub playback_generation: Option<u64>,
    pub source_time_us: u64,
    pub duration_us: Option<u64>,
    pub frame_index: Option<u64>,
    pub dimensions: FrameDimensions,
    pub pixel_format: VideoPixelFormat,
    pub color: VideoColorMetadata,
    pub storage: VideoFrameStorage,
    pub release: FrameLeaseId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedAudioFrame {
    pub owner_session: MediaSessionId,
    pub start_time_us: u64,
    pub duration_us: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub frame_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FrameStorageKind {
    Cpu,
    Texture,
    PlatformOpaque,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameReleaseDiagnostic {
    pub lease_id: FrameLeaseId,
    pub frame_handle_id: FrameHandleId,
    pub texture_handle_id: Option<TextureHandleId>,
    pub owner_session: MediaSessionId,
    pub generation: Option<u64>,
    pub storage_kind: FrameStorageKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FramePoolCloseReport {
    pub owner_session: MediaSessionId,
    pub leak_diagnostics: Vec<FrameReleaseDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FramePoolErrorKind {
    LeaseNotFound,
    OwnerSessionMismatch,
    LeaseLimitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[error("frame pool failed: {message}")]
#[serde(rename_all = "camelCase")]
pub struct FramePoolError {
    pub kind: FramePoolErrorKind,
    pub message: String,
}

impl FramePoolError {
    fn new(kind: FramePoolErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[derive(Debug)]
pub struct FramePool {
    owner_session: MediaSessionId,
    limits: FramePoolLimits,
    next_id: u64,
    active: BTreeMap<FrameLeaseId, DecodedVideoFrame>,
}

impl FramePool {
    pub fn new(owner_session: MediaSessionId, limits: FramePoolLimits) -> Self {
        Self {
            owner_session,
            limits,
            next_id: 1,
            active: BTreeMap::new(),
        }
    }

    pub fn acquire_video_frame(
        &mut self,
        request: FrameLeaseRequest,
    ) -> Result<DecodedVideoFrame, FramePoolError> {
        if self.active.len() >= self.limits.max_outstanding_leases {
            return Err(FramePoolError::new(
                FramePoolErrorKind::LeaseLimitExceeded,
                "frame pool outstanding lease limit exceeded",
            ));
        }

        let handle_id = FrameHandleId(format!("frame-{}", self.next_id));
        let lease_id = FrameLeaseId(format!("lease-{}", self.next_id));
        self.next_id += 1;

        let storage = match request.storage {
            FrameStorageRequest::Cpu { estimated_byte_len } => {
                VideoFrameStorage::Cpu(CpuFrameHandle {
                    handle_id: handle_id.clone(),
                    owner_session: self.owner_session.clone(),
                    generation: request.playback_generation,
                    dimensions: request.dimensions,
                    pixel_format: request.pixel_format,
                    estimated_byte_len,
                })
            }
            FrameStorageRequest::Texture(texture) => {
                if texture.owner_session != self.owner_session {
                    return Err(FramePoolError::new(
                        FramePoolErrorKind::OwnerSessionMismatch,
                        "texture handle owner session does not match frame pool session",
                    ));
                }
                VideoFrameStorage::Texture(texture)
            }
            FrameStorageRequest::PlatformOpaque { label } => {
                VideoFrameStorage::PlatformOpaque(PlatformFrameHandle {
                    handle_id: handle_id.clone(),
                    owner_session: self.owner_session.clone(),
                    generation: request.playback_generation,
                    label,
                })
            }
        };

        let frame = DecodedVideoFrame {
            handle_id,
            owner_session: self.owner_session.clone(),
            playback_generation: request.playback_generation,
            source_time_us: request.source_time_us,
            duration_us: request.duration_us,
            frame_index: request.frame_index,
            dimensions: request.dimensions,
            pixel_format: request.pixel_format,
            color: request.color,
            storage,
            release: lease_id.clone(),
        };

        self.active.insert(lease_id, frame.clone());
        Ok(frame)
    }

    pub fn outstanding_lease_count(&self) -> usize {
        self.active.len()
    }

    pub fn release(
        &mut self,
        lease_id: FrameLeaseId,
    ) -> Result<FrameReleaseDiagnostic, FramePoolError> {
        let owner_session = self.owner_session.clone();
        self.release_for_session(&owner_session, lease_id)
    }

    pub fn release_for_session(
        &mut self,
        owner_session: &MediaSessionId,
        lease_id: FrameLeaseId,
    ) -> Result<FrameReleaseDiagnostic, FramePoolError> {
        if owner_session != &self.owner_session {
            return Err(FramePoolError::new(
                FramePoolErrorKind::OwnerSessionMismatch,
                "release owner session does not match frame pool session",
            ));
        }

        let frame = self.active.remove(&lease_id).ok_or_else(|| {
            FramePoolError::new(FramePoolErrorKind::LeaseNotFound, "frame lease not found")
        })?;

        Ok(release_diagnostic(lease_id, &frame, "frame lease released"))
    }

    pub fn close_session(&mut self) -> FramePoolCloseReport {
        let leak_diagnostics = std::mem::take(&mut self.active)
            .into_iter()
            .map(|(lease_id, frame)| {
                release_diagnostic(
                    lease_id,
                    &frame,
                    "unreleased frame lease closed with session",
                )
            })
            .collect();

        FramePoolCloseReport {
            owner_session: self.owner_session.clone(),
            leak_diagnostics,
        }
    }
}

fn release_diagnostic(
    lease_id: FrameLeaseId,
    frame: &DecodedVideoFrame,
    message: &str,
) -> FrameReleaseDiagnostic {
    FrameReleaseDiagnostic {
        lease_id,
        frame_handle_id: frame.handle_id.clone(),
        texture_handle_id: texture_handle_id(&frame.storage),
        owner_session: frame.owner_session.clone(),
        generation: frame.playback_generation,
        storage_kind: storage_kind(&frame.storage),
        message: message.to_owned(),
    }
}

fn texture_handle_id(storage: &VideoFrameStorage) -> Option<TextureHandleId> {
    match storage {
        VideoFrameStorage::Texture(texture) => Some(texture.handle_id.clone()),
        VideoFrameStorage::Cpu(_) | VideoFrameStorage::PlatformOpaque(_) => None,
    }
}

fn storage_kind(storage: &VideoFrameStorage) -> FrameStorageKind {
    match storage {
        VideoFrameStorage::Cpu(_) => FrameStorageKind::Cpu,
        VideoFrameStorage::Texture(_) => FrameStorageKind::Texture,
        VideoFrameStorage::PlatformOpaque(_) => FrameStorageKind::PlatformOpaque,
    }
}

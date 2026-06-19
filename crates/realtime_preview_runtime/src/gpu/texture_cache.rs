use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use draft_model::{MaterialId, Microseconds};

use crate::{
    CpuVideoFrame, FrameValidationError, PlaybackGeneration, PreviewFrameInput,
    TextureHandleDescriptor,
};

use super::device::RealtimePreviewGpuDevice;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RealtimePreviewTextureId(u64);

impl RealtimePreviewTextureId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimePreviewTexture {
    pub id: RealtimePreviewTextureId,
    pub material_id: MaterialId,
    pub source_position: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub width: u32,
    pub height: u32,
    storage: RealtimePreviewTextureStorage,
}

impl RealtimePreviewTexture {
    pub fn cpu_pixels(&self) -> Option<&[u8]> {
        match &self.storage {
            RealtimePreviewTextureStorage::CpuRgba { pixels } => Some(pixels),
            RealtimePreviewTextureStorage::ExternalHandle(_) => None,
        }
    }

    pub fn external_handle(&self) -> Option<&TextureHandleDescriptor> {
        match &self.storage {
            RealtimePreviewTextureStorage::CpuRgba { .. } => None,
            RealtimePreviewTextureStorage::ExternalHandle(handle) => Some(handle),
        }
    }

    pub fn storage_kind(&self) -> RealtimePreviewTextureStorageKind {
        match &self.storage {
            RealtimePreviewTextureStorage::CpuRgba { .. } => {
                RealtimePreviewTextureStorageKind::CpuRgba
            }
            RealtimePreviewTextureStorage::ExternalHandle(_) => {
                RealtimePreviewTextureStorageKind::ExternalHandle
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimePreviewTextureStorageKind {
    CpuRgba,
    ExternalHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealtimePreviewTextureStorage {
    CpuRgba { pixels: Vec<u8> },
    ExternalHandle(TextureHandleDescriptor),
}

#[derive(Debug, Default)]
pub struct RealtimePreviewTextureCache {
    next_texture_id: u64,
    textures: BTreeMap<RealtimePreviewTextureId, RealtimePreviewTexture>,
}

impl RealtimePreviewTextureCache {
    pub fn new() -> Self {
        Self {
            next_texture_id: 1,
            textures: BTreeMap::new(),
        }
    }

    pub fn upload_frame(
        &mut self,
        device: &RealtimePreviewGpuDevice,
        input: PreviewFrameInput,
    ) -> Result<RealtimePreviewTexture, RealtimePreviewTextureCacheError> {
        let _physical_upload_path_available = device.device().is_some() && device.queue().is_some();
        match input {
            PreviewFrameInput::CpuRgba(frame) | PreviewFrameInput::StaticImage(frame) => {
                frame
                    .validate()
                    .map_err(RealtimePreviewTextureCacheError::InvalidFrame)?;

                let texture_id = self.next_id();
                let texture = texture_from_frame(texture_id, frame);
                self.textures.insert(texture_id, texture.clone());
                Ok(texture)
            }
            PreviewFrameInput::TextureHandle(handle) => {
                handle
                    .validate()
                    .map_err(RealtimePreviewTextureCacheError::InvalidFrame)?;

                let texture_id = self.next_id();
                let texture = texture_from_handle(texture_id, handle);
                self.textures.insert(texture_id, texture.clone());
                Ok(texture)
            }
            PreviewFrameInput::Unavailable { reason } => {
                Err(RealtimePreviewTextureCacheError::Unavailable { reason })
            }
        }
    }

    pub fn get(&self, texture_id: RealtimePreviewTextureId) -> Option<&RealtimePreviewTexture> {
        self.textures.get(&texture_id)
    }

    fn next_id(&mut self) -> RealtimePreviewTextureId {
        let texture_id = RealtimePreviewTextureId(self.next_texture_id);
        self.next_texture_id = self.next_texture_id.saturating_add(1);
        texture_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealtimePreviewTextureCacheError {
    InvalidFrame(FrameValidationError),
    Unavailable { reason: String },
}

impl fmt::Display for RealtimePreviewTextureCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrame(error) => write!(formatter, "invalid texture frame: {error}"),
            Self::Unavailable { reason } => {
                write!(formatter, "texture frame unavailable: {reason}")
            }
        }
    }
}

impl Error for RealtimePreviewTextureCacheError {}

fn texture_from_frame(
    id: RealtimePreviewTextureId,
    frame: CpuVideoFrame,
) -> RealtimePreviewTexture {
    RealtimePreviewTexture {
        id,
        material_id: frame.material_id,
        source_position: frame.source_position,
        playback_generation: frame.playback_generation,
        width: frame.width,
        height: frame.height,
        storage: RealtimePreviewTextureStorage::CpuRgba {
            pixels: frame.pixels,
        },
    }
}

fn texture_from_handle(
    id: RealtimePreviewTextureId,
    handle: TextureHandleDescriptor,
) -> RealtimePreviewTexture {
    RealtimePreviewTexture {
        id,
        material_id: handle.material_id.clone(),
        source_position: handle.source_position,
        playback_generation: handle.playback_generation,
        width: handle.width,
        height: handle.height,
        storage: RealtimePreviewTextureStorage::ExternalHandle(handle),
    }
}

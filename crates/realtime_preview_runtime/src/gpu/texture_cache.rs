use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use draft_model::{MaterialId, Microseconds};

use crate::{CpuVideoFrame, FrameValidationError, PlaybackGeneration, PreviewFrameInput};

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
    pixels: Vec<u8>,
}

impl RealtimePreviewTexture {
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
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
        let frame = match input {
            PreviewFrameInput::CpuRgba(frame) | PreviewFrameInput::StaticImage(frame) => frame,
            PreviewFrameInput::TextureHandle(handle) => {
                return Err(RealtimePreviewTextureCacheError::UnsupportedTextureHandle {
                    handle_id: handle.handle_id,
                    backend: handle.backend,
                });
            }
            PreviewFrameInput::Unavailable { reason } => {
                return Err(RealtimePreviewTextureCacheError::Unavailable { reason });
            }
        };

        frame
            .validate()
            .map_err(RealtimePreviewTextureCacheError::InvalidFrame)?;

        let texture_id = RealtimePreviewTextureId(self.next_texture_id);
        self.next_texture_id = self.next_texture_id.saturating_add(1);

        let texture = texture_from_frame(texture_id, frame);
        self.textures.insert(texture_id, texture.clone());
        Ok(texture)
    }

    pub fn get(&self, texture_id: RealtimePreviewTextureId) -> Option<&RealtimePreviewTexture> {
        self.textures.get(&texture_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealtimePreviewTextureCacheError {
    InvalidFrame(FrameValidationError),
    UnsupportedTextureHandle { handle_id: u64, backend: String },
    Unavailable { reason: String },
}

impl fmt::Display for RealtimePreviewTextureCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrame(error) => write!(formatter, "invalid texture frame: {error}"),
            Self::UnsupportedTextureHandle { backend, .. } => {
                write!(
                    formatter,
                    "external {backend} texture handles are not uploadable in Phase 11"
                )
            }
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
        pixels: frame.pixels,
    }
}

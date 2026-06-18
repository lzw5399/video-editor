use serde::{Deserialize, Serialize};

use crate::{FrameDimensions, MediaSessionId, VideoColorMetadata, VideoPixelFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextureBackend {
    D3d11Texture2D,
    D3d12Resource,
    MetalTexture,
    CoreVideoPixelBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TextureHandleId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDeviceId {
    pub backend: TextureBackend,
    pub adapter_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureHandle {
    pub handle_id: TextureHandleId,
    pub owner_session: MediaSessionId,
    pub generation: u64,
    pub backend: TextureBackend,
    pub device_id: RuntimeDeviceId,
    pub dimensions: FrameDimensions,
    pub pixel_format: VideoPixelFormat,
    pub color: VideoColorMetadata,
}

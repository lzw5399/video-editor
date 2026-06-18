use std::error::Error;
use std::fmt;

use draft_model::{MaterialId, Microseconds};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use crate::PlaybackGeneration;

pub trait PreviewFrameProvider {
    fn provider_name(&self) -> &'static str;

    fn frame_for(
        &mut self,
        material_id: &MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
    ) -> Result<PreviewFrameInput, PreviewFrameProviderError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewFrameInput {
    CpuRgba(CpuVideoFrame),
    StaticImage(CpuVideoFrame),
    TextureHandle(TextureHandleDescriptor),
    Unavailable { reason: String },
}

impl PreviewFrameInput {
    pub fn cpu_rgba(
        material_id: MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> Result<Self, FrameValidationError> {
        let stride_bytes = width.saturating_mul(4);
        Ok(Self::CpuRgba(CpuVideoFrame::new(
            material_id,
            source_position,
            playback_generation,
            width,
            height,
            stride_bytes,
            FrameColorInfo::srgb_rgba8(),
            pixels,
        )?))
    }

    pub fn static_image(
        material_id: MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> Result<Self, FrameValidationError> {
        let stride_bytes = width.saturating_mul(4);
        Ok(Self::StaticImage(CpuVideoFrame::new(
            material_id,
            source_position,
            playback_generation,
            width,
            height,
            stride_bytes,
            FrameColorInfo::srgb_rgba8(),
            pixels,
        )?))
    }
}

impl Serialize for PreviewFrameInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::CpuRgba(frame) => {
                let mut state = serializer.serialize_struct("PreviewFrameInput", 2)?;
                state.serialize_field("kind", "cpuRgba")?;
                state.serialize_field("frame", frame)?;
                state.end()
            }
            Self::StaticImage(frame) => {
                let mut state = serializer.serialize_struct("PreviewFrameInput", 2)?;
                state.serialize_field("kind", "staticImage")?;
                state.serialize_field("frame", frame)?;
                state.end()
            }
            Self::TextureHandle(handle) => {
                let mut state = serializer.serialize_struct("PreviewFrameInput", 2)?;
                state.serialize_field("kind", "textureHandle")?;
                state.serialize_field("handle", handle)?;
                state.end()
            }
            Self::Unavailable { reason } => {
                let mut state = serializer.serialize_struct("PreviewFrameInput", 2)?;
                state.serialize_field("kind", "unavailable")?;
                state.serialize_field("reason", reason)?;
                state.end()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CpuVideoFrame {
    pub material_id: MaterialId,
    pub source_position: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub width: u32,
    pub height: u32,
    pub stride_bytes: u32,
    pub color: FrameColorInfo,
    pub pixels: Vec<u8>,
}

impl CpuVideoFrame {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        material_id: MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
        width: u32,
        height: u32,
        stride_bytes: u32,
        color: FrameColorInfo,
        pixels: Vec<u8>,
    ) -> Result<Self, FrameValidationError> {
        validate_frame(&material_id, width, height, stride_bytes, pixels.len())?;
        Ok(Self {
            material_id,
            source_position,
            playback_generation,
            width,
            height,
            stride_bytes,
            color,
            pixels,
        })
    }

    pub fn validate(&self) -> Result<(), FrameValidationError> {
        validate_frame(
            &self.material_id,
            self.width,
            self.height,
            self.stride_bytes,
            self.pixels.len(),
        )
    }

    pub fn pixel_len(&self) -> usize {
        self.pixels.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameColorInfo {
    pub pixel_format: String,
    pub color_space: String,
    pub transfer: String,
    pub alpha: String,
}

impl FrameColorInfo {
    pub fn srgb_rgba8() -> Self {
        Self {
            pixel_format: "rgba8".to_owned(),
            color_space: "srgb".to_owned(),
            transfer: "srgb".to_owned(),
            alpha: "premultiplied".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextureHandleDescriptor {
    pub handle_id: u64,
    pub owner_generation: PlaybackGeneration,
    pub backend: String,
    pub width: u32,
    pub height: u32,
    pub pixel_format: String,
}

impl TextureHandleDescriptor {
    pub fn new(
        handle_id: u64,
        owner_generation: PlaybackGeneration,
        backend: impl Into<String>,
        width: u32,
        height: u32,
        pixel_format: impl Into<String>,
    ) -> Result<Self, FrameValidationError> {
        let backend = backend.into();
        let pixel_format = pixel_format.into();
        if handle_id == 0 {
            return Err(FrameValidationError::new(
                FrameValidationErrorKind::InvalidTextureHandle,
                "texture handle id must be nonzero",
            ));
        }
        if backend.trim().is_empty() {
            return Err(FrameValidationError::new(
                FrameValidationErrorKind::InvalidTextureHandle,
                "texture backend must be present",
            ));
        }
        if width == 0 || height == 0 {
            return Err(FrameValidationError::new(
                FrameValidationErrorKind::InvalidDimensions,
                "texture dimensions must be nonzero",
            ));
        }
        if pixel_format.trim().is_empty() {
            return Err(FrameValidationError::new(
                FrameValidationErrorKind::InvalidTextureHandle,
                "texture pixel format must be present",
            ));
        }
        Ok(Self {
            handle_id,
            owner_generation,
            backend,
            width,
            height,
            pixel_format,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FrameValidationErrorKind {
    MissingMaterialId,
    InvalidDimensions,
    InvalidStride,
    InvalidPixelLength,
    InvalidTextureHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameValidationError {
    kind: FrameValidationErrorKind,
    message: String,
}

impl FrameValidationError {
    fn new(kind: FrameValidationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> FrameValidationErrorKind {
        self.kind
    }
}

impl fmt::Display for FrameValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for FrameValidationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewFrameProviderError {
    InvalidFrame {
        provider_name: &'static str,
        material_id: Option<MaterialId>,
        source_position: Option<Microseconds>,
        playback_generation: Option<PlaybackGeneration>,
        error: FrameValidationError,
    },
    Unavailable {
        provider_name: &'static str,
        material_id: MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
        reason: String,
    },
    UnsupportedCodec {
        provider_name: &'static str,
        material_id: MaterialId,
        codec: String,
        reason: String,
    },
    OutOfRange {
        provider_name: &'static str,
        material_id: MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
        reason: String,
    },
}

impl PreviewFrameProviderError {
    pub fn invalid_frame(
        provider_name: &'static str,
        material_id: Option<MaterialId>,
        source_position: Option<Microseconds>,
        playback_generation: Option<PlaybackGeneration>,
        error: FrameValidationError,
    ) -> Self {
        Self::InvalidFrame {
            provider_name,
            material_id,
            source_position,
            playback_generation,
            error,
        }
    }

    pub fn unavailable(
        provider_name: &'static str,
        material_id: MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
        reason: impl Into<String>,
    ) -> Self {
        Self::Unavailable {
            provider_name,
            material_id,
            source_position,
            playback_generation,
            reason: reason.into(),
        }
    }

    pub fn unsupported_codec(
        provider_name: &'static str,
        material_id: MaterialId,
        codec: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::UnsupportedCodec {
            provider_name,
            material_id,
            codec: codec.into(),
            reason: reason.into(),
        }
    }

    pub fn out_of_range(
        provider_name: &'static str,
        material_id: MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
        reason: impl Into<String>,
    ) -> Self {
        Self::OutOfRange {
            provider_name,
            material_id,
            source_position,
            playback_generation,
            reason: reason.into(),
        }
    }

    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::InvalidFrame { provider_name, .. }
            | Self::Unavailable { provider_name, .. }
            | Self::UnsupportedCodec { provider_name, .. }
            | Self::OutOfRange { provider_name, .. } => provider_name,
        }
    }

    pub fn material_id(&self) -> Option<&MaterialId> {
        match self {
            Self::InvalidFrame { material_id, .. } => material_id.as_ref(),
            Self::Unavailable { material_id, .. }
            | Self::UnsupportedCodec { material_id, .. }
            | Self::OutOfRange { material_id, .. } => Some(material_id),
        }
    }

    pub fn source_position(&self) -> Option<Microseconds> {
        match self {
            Self::InvalidFrame {
                source_position, ..
            } => *source_position,
            Self::Unavailable {
                source_position, ..
            }
            | Self::OutOfRange {
                source_position, ..
            } => Some(*source_position),
            Self::UnsupportedCodec { .. } => None,
        }
    }

    pub fn playback_generation(&self) -> Option<PlaybackGeneration> {
        match self {
            Self::InvalidFrame {
                playback_generation,
                ..
            } => *playback_generation,
            Self::Unavailable {
                playback_generation,
                ..
            }
            | Self::OutOfRange {
                playback_generation,
                ..
            } => Some(*playback_generation),
            Self::UnsupportedCodec { .. } => None,
        }
    }
}

impl fmt::Display for PreviewFrameProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrame {
                provider_name,
                error,
                ..
            } => write!(formatter, "{provider_name} returned invalid frame: {error}"),
            Self::Unavailable {
                provider_name,
                reason,
                ..
            } => write!(formatter, "{provider_name} frame unavailable: {reason}"),
            Self::UnsupportedCodec {
                provider_name,
                codec,
                reason,
                ..
            } => write!(
                formatter,
                "{provider_name} unsupported codec {codec}: {reason}"
            ),
            Self::OutOfRange {
                provider_name,
                reason,
                ..
            } => write!(formatter, "{provider_name} frame out of range: {reason}"),
        }
    }
}

impl Error for PreviewFrameProviderError {}

fn validate_frame(
    material_id: &MaterialId,
    width: u32,
    height: u32,
    stride_bytes: u32,
    pixel_len: usize,
) -> Result<(), FrameValidationError> {
    if material_id.is_empty() {
        return Err(FrameValidationError::new(
            FrameValidationErrorKind::MissingMaterialId,
            "frame material id must be present",
        ));
    }
    if width == 0 || height == 0 {
        return Err(FrameValidationError::new(
            FrameValidationErrorKind::InvalidDimensions,
            "frame dimensions must be nonzero",
        ));
    }

    let minimum_stride = width.checked_mul(4).ok_or_else(|| {
        FrameValidationError::new(
            FrameValidationErrorKind::InvalidStride,
            "frame stride overflowed width * 4",
        )
    })?;
    if stride_bytes < minimum_stride {
        return Err(FrameValidationError::new(
            FrameValidationErrorKind::InvalidStride,
            format!("frame stride {stride_bytes} is shorter than required {minimum_stride}"),
        ));
    }

    let expected_pixels = stride_bytes
        .checked_mul(height)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| {
            FrameValidationError::new(
                FrameValidationErrorKind::InvalidPixelLength,
                "frame pixel length overflowed stride * height",
            )
        })?;
    if pixel_len != expected_pixels {
        return Err(FrameValidationError::new(
            FrameValidationErrorKind::InvalidPixelLength,
            format!("frame pixel length {pixel_len} does not match expected {expected_pixels}"),
        ));
    }

    Ok(())
}

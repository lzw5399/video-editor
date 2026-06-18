use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimePreviewTargetFormat {
    Rgba8UnormSrgb,
}

impl RealtimePreviewTargetFormat {
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgba8UnormSrgb => 4,
        }
    }

    pub(crate) const fn wgpu_format(self) -> wgpu::TextureFormat {
        match self {
            Self::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        }
    }
}

#[derive(Debug)]
pub struct RealtimePreviewGpuTarget {
    width: u32,
    height: u32,
    scale_factor_millis: u32,
    format: RealtimePreviewTargetFormat,
    texture: Option<wgpu::Texture>,
}

impl RealtimePreviewGpuTarget {
    pub fn offscreen(
        width: u32,
        height: u32,
        scale_factor_millis: u32,
        format: RealtimePreviewTargetFormat,
    ) -> Result<Self, RealtimePreviewTargetError> {
        validate_offscreen_target(width, height, scale_factor_millis)?;
        Ok(Self {
            width,
            height,
            scale_factor_millis,
            format,
            texture: None,
        })
    }

    pub(crate) fn with_texture(
        width: u32,
        height: u32,
        scale_factor_millis: u32,
        format: RealtimePreviewTargetFormat,
        texture: wgpu::Texture,
    ) -> Result<Self, RealtimePreviewTargetError> {
        validate_offscreen_target(width, height, scale_factor_millis)?;
        Ok(Self {
            width,
            height,
            scale_factor_millis,
            format,
            texture: Some(texture),
        })
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub const fn scale_factor_millis(&self) -> u32 {
        self.scale_factor_millis
    }

    pub const fn format(&self) -> RealtimePreviewTargetFormat {
        self.format
    }

    pub fn pixel_len(&self) -> usize {
        self.width as usize * self.height as usize * self.format.bytes_per_pixel()
    }

    pub(crate) fn texture(&self) -> Option<&wgpu::Texture> {
        self.texture.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimePreviewTargetError {
    message: String,
}

impl RealtimePreviewTargetError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RealtimePreviewTargetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RealtimePreviewTargetError {}

fn validate_offscreen_target(
    width: u32,
    height: u32,
    scale_factor_millis: u32,
) -> Result<(), RealtimePreviewTargetError> {
    if width == 0 || height == 0 {
        return Err(RealtimePreviewTargetError::new(
            "offscreen target dimensions must be nonzero",
        ));
    }
    if scale_factor_millis == 0 {
        return Err(RealtimePreviewTargetError::new(
            "offscreen target scale factor must be nonzero",
        ));
    }
    Ok(())
}

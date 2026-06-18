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

#[cfg(test)]
mod native_surface_contracts {
    use super::{
        NativeParentWindowHandle, PreviewSurfaceBounds, PreviewSurfaceDescriptor,
        PreviewSurfaceDiagnosticKind, PreviewSurfaceHost, PreviewSurfaceStatus,
    };

    fn valid_bounds() -> PreviewSurfaceBounds {
        PreviewSurfaceBounds {
            x: 10,
            y: 20,
            width: 1280,
            height: 720,
            scale_factor_millis: 2000,
        }
    }

    #[test]
    fn rejects_invalid_bounds_and_scale_with_typed_diagnostics() {
        let zero_width = PreviewSurfaceBounds {
            width: 0,
            ..valid_bounds()
        };
        let error = zero_width.validate().expect_err("zero width is invalid");
        assert_eq!(error.kind(), PreviewSurfaceDiagnosticKind::InvalidBounds);

        let zero_scale = PreviewSurfaceBounds {
            scale_factor_millis: 0,
            ..valid_bounds()
        };
        let error = zero_scale.validate().expect_err("zero scale is invalid");
        assert_eq!(error.kind(), PreviewSurfaceDiagnosticKind::InvalidScale);
    }

    #[test]
    fn rejects_missing_parent_handles_before_attach() {
        let descriptor = PreviewSurfaceDescriptor::NativeChild {
            parent_window_handle: NativeParentWindowHandle::WindowsHwnd(0),
            bounds: valid_bounds(),
        };

        let error = descriptor
            .validate()
            .expect_err("zero native parent handle must be rejected");
        assert_eq!(
            error.kind(),
            PreviewSurfaceDiagnosticKind::MissingParentHandle
        );
    }

    #[test]
    fn enforces_attach_detach_lifecycle() {
        let mut host = PreviewSurfaceHost::new();
        let descriptor = PreviewSurfaceDescriptor::NativeChild {
            parent_window_handle: NativeParentWindowHandle::Mock(42),
            bounds: valid_bounds(),
        };

        let attached = host.attach(descriptor).expect("mock surface attaches");
        assert_eq!(attached.status, PreviewSurfaceStatus::Available);

        let error = host
            .attach(PreviewSurfaceDescriptor::Offscreen {
                width: 640,
                height: 360,
                scale_factor_millis: 1000,
            })
            .expect_err("cannot attach twice without detach");
        assert_eq!(error.kind(), PreviewSurfaceDiagnosticKind::AlreadyAttached);

        host.update_bounds(PreviewSurfaceBounds {
            x: 0,
            y: 0,
            width: 640,
            height: 360,
            scale_factor_millis: 1000,
        })
        .expect("attached host accepts bounds updates");

        host.detach().expect("attached host detaches");
        let error = host.detach().expect_err("second detach is invalid");
        assert_eq!(error.kind(), PreviewSurfaceDiagnosticKind::NotAttached);
    }

    #[test]
    fn reports_unavailable_and_lost_surface_statuses() {
        let mut host = PreviewSurfaceHost::new();
        let descriptor = PreviewSurfaceDescriptor::NativeChild {
            parent_window_handle: NativeParentWindowHandle::Mock(42),
            bounds: valid_bounds(),
        };

        host.attach(descriptor).expect("mock surface attaches");
        host.mark_unavailable("native child surface not ready");
        let error = host
            .update_bounds(valid_bounds())
            .expect_err("unavailable surface rejects updates");
        assert_eq!(error.kind(), PreviewSurfaceDiagnosticKind::SurfaceUnavailable);

        host.mark_lost("wgpu surface lost");
        let error = host
            .update_bounds(valid_bounds())
            .expect_err("lost surface rejects updates");
        assert_eq!(error.kind(), PreviewSurfaceDiagnosticKind::SurfaceLost);
    }
}

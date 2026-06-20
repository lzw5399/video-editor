use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewSurfaceBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor_millis: u32,
}

impl PreviewSurfaceBounds {
    pub fn validate(self) -> Result<Self, PreviewSurfaceError> {
        if self.width == 0 || self.height == 0 {
            return Err(PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::InvalidBounds,
                "preview surface bounds must have nonzero width and height",
            ));
        }
        if self.scale_factor_millis == 0 {
            return Err(PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::InvalidScale,
                "preview surface scale factor millis must be nonzero",
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeParentWindowHandle {
    WindowsHwnd(u64),
    MacosNsView(u64),
    Mock(u64),
}

impl NativeParentWindowHandle {
    pub const fn raw_value(self) -> u64 {
        match self {
            Self::WindowsHwnd(value) | Self::MacosNsView(value) | Self::Mock(value) => value,
        }
    }

    pub const fn platform_name(self) -> &'static str {
        match self {
            Self::WindowsHwnd(_) => "windows-hwnd",
            Self::MacosNsView(_) => "macos-nsview",
            Self::Mock(_) => "mock",
        }
    }

    fn validate(self) -> Result<Self, PreviewSurfaceError> {
        if self.raw_value() == 0 {
            return Err(PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::MissingParentHandle,
                format!("{} parent handle must be nonzero", self.platform_name()),
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewSurfaceDescriptor {
    NativeChild {
        parent_window_handle: NativeParentWindowHandle,
        bounds: PreviewSurfaceBounds,
    },
    Offscreen {
        width: u32,
        height: u32,
        scale_factor_millis: u32,
    },
}

impl PreviewSurfaceDescriptor {
    pub fn validate(self) -> Result<Self, PreviewSurfaceError> {
        match self {
            Self::NativeChild {
                parent_window_handle,
                bounds,
            } => {
                parent_window_handle.validate()?;
                bounds.validate()?;
            }
            Self::Offscreen {
                width,
                height,
                scale_factor_millis,
            } => {
                PreviewSurfaceBounds {
                    x: 0,
                    y: 0,
                    width,
                    height,
                    scale_factor_millis,
                }
                .validate()?;
            }
        }
        Ok(self)
    }

    pub const fn bounds(self) -> PreviewSurfaceBounds {
        match self {
            Self::NativeChild { bounds, .. } => bounds,
            Self::Offscreen {
                width,
                height,
                scale_factor_millis,
            } => PreviewSurfaceBounds {
                x: 0,
                y: 0,
                width,
                height,
                scale_factor_millis,
            },
        }
    }

    pub const fn is_native_child(self) -> bool {
        matches!(self, Self::NativeChild { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewSurfaceStatus {
    Available,
    Unavailable,
    Lost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSurfaceAttachment {
    pub descriptor: PreviewSurfaceDescriptor,
    pub status: PreviewSurfaceStatus,
    pub status_message: Option<String>,
}

#[derive(Debug, Default)]
pub struct PreviewSurfaceHost {
    attachment: Option<PreviewSurfaceAttachment>,
}

impl PreviewSurfaceHost {
    pub fn new() -> Self {
        Self { attachment: None }
    }

    pub fn is_attached(&self) -> bool {
        self.attachment.is_some()
    }

    pub fn attachment(&self) -> Option<&PreviewSurfaceAttachment> {
        self.attachment.as_ref()
    }

    pub fn attach(
        &mut self,
        descriptor: PreviewSurfaceDescriptor,
    ) -> Result<&PreviewSurfaceAttachment, PreviewSurfaceError> {
        if self.attachment.is_some() {
            return Err(PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::AlreadyAttached,
                "preview surface is already attached",
            ));
        }
        let descriptor = descriptor.validate()?;
        self.attachment = Some(PreviewSurfaceAttachment {
            descriptor,
            status: PreviewSurfaceStatus::Available,
            status_message: None,
        });
        Ok(self
            .attachment
            .as_ref()
            .expect("attachment was just inserted"))
    }

    pub fn update_bounds(
        &mut self,
        bounds: PreviewSurfaceBounds,
    ) -> Result<&PreviewSurfaceAttachment, PreviewSurfaceError> {
        let attachment = self.attachment.as_mut().ok_or_else(|| {
            PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::NotAttached,
                "preview surface is not attached",
            )
        })?;
        ensure_available(attachment)?;
        let bounds = bounds.validate()?;
        attachment.descriptor = match attachment.descriptor {
            PreviewSurfaceDescriptor::NativeChild {
                parent_window_handle,
                ..
            } => PreviewSurfaceDescriptor::NativeChild {
                parent_window_handle,
                bounds,
            },
            PreviewSurfaceDescriptor::Offscreen { .. } => PreviewSurfaceDescriptor::Offscreen {
                width: bounds.width,
                height: bounds.height,
                scale_factor_millis: bounds.scale_factor_millis,
            },
        };
        Ok(attachment)
    }

    pub fn detach(&mut self) -> Result<PreviewSurfaceAttachment, PreviewSurfaceError> {
        self.attachment.take().ok_or_else(|| {
            PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::NotAttached,
                "preview surface is not attached",
            )
        })
    }

    pub fn mark_unavailable(&mut self, reason: impl Into<String>) {
        self.mark_status(PreviewSurfaceStatus::Unavailable, reason);
    }

    pub fn mark_lost(&mut self, reason: impl Into<String>) {
        self.mark_status(PreviewSurfaceStatus::Lost, reason);
    }

    fn mark_status(&mut self, status: PreviewSurfaceStatus, reason: impl Into<String>) {
        if let Some(attachment) = self.attachment.as_mut() {
            attachment.status = status;
            attachment.status_message = Some(reason.into());
        }
    }
}

fn ensure_available(attachment: &PreviewSurfaceAttachment) -> Result<(), PreviewSurfaceError> {
    match attachment.status {
        PreviewSurfaceStatus::Available => Ok(()),
        PreviewSurfaceStatus::Unavailable => Err(PreviewSurfaceError::new(
            PreviewSurfaceDiagnosticKind::SurfaceUnavailable,
            attachment
                .status_message
                .clone()
                .unwrap_or_else(|| "preview surface is unavailable".to_owned()),
        )),
        PreviewSurfaceStatus::Lost => Err(PreviewSurfaceError::new(
            PreviewSurfaceDiagnosticKind::SurfaceLost,
            attachment
                .status_message
                .clone()
                .unwrap_or_else(|| "preview surface is lost".to_owned()),
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewSurfaceDiagnosticKind {
    MissingParentHandle,
    InvalidBounds,
    InvalidScale,
    AlreadyAttached,
    NotAttached,
    SurfaceUnavailable,
    SurfaceLost,
    PlatformUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSurfaceError {
    kind: PreviewSurfaceDiagnosticKind,
    message: String,
}

impl PreviewSurfaceError {
    pub fn new(kind: PreviewSurfaceDiagnosticKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub const fn kind(&self) -> PreviewSurfaceDiagnosticKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PreviewSurfaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for PreviewSurfaceError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RealtimePreviewTargetFormat {
    Rgba8UnormSrgb,
    Bgra8UnormSrgb,
}

impl RealtimePreviewTargetFormat {
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgba8UnormSrgb | Self::Bgra8UnormSrgb => 4,
        }
    }

    pub(crate) const fn wgpu_format(self) -> wgpu::TextureFormat {
        match self {
            Self::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
        }
    }

    pub(crate) const fn from_wgpu_format(format: wgpu::TextureFormat) -> Option<Self> {
        match format {
            wgpu::TextureFormat::Rgba8UnormSrgb => Some(Self::Rgba8UnormSrgb),
            wgpu::TextureFormat::Bgra8UnormSrgb => Some(Self::Bgra8UnormSrgb),
            _ => None,
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

#[derive(Debug)]
pub struct RealtimePreviewGpuPresentationTarget {
    descriptor: PreviewSurfaceDescriptor,
    bounds: PreviewSurfaceBounds,
    format: RealtimePreviewTargetFormat,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    #[cfg(target_os = "macos")]
    macos_attachment: Option<crate::platform::macos::MacosWgpuSurfaceAttachment>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewSurfaceScreenRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl RealtimePreviewGpuPresentationTarget {
    pub(crate) fn new(
        descriptor: PreviewSurfaceDescriptor,
        format: RealtimePreviewTargetFormat,
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    ) -> Self {
        let bounds = descriptor.bounds();
        Self {
            descriptor,
            bounds,
            format,
            surface,
            config,
            #[cfg(target_os = "macos")]
            macos_attachment: None,
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn with_macos_attachment(
        mut self,
        attachment: crate::platform::macos::MacosWgpuSurfaceAttachment,
    ) -> Self {
        self.macos_attachment = Some(attachment);
        self
    }

    pub const fn descriptor(&self) -> PreviewSurfaceDescriptor {
        self.descriptor
    }

    pub const fn bounds(&self) -> PreviewSurfaceBounds {
        self.bounds
    }

    pub const fn width(&self) -> u32 {
        self.bounds.width
    }

    pub const fn height(&self) -> u32 {
        self.bounds.height
    }

    pub const fn scale_factor_millis(&self) -> u32 {
        self.bounds.scale_factor_millis
    }

    pub const fn format(&self) -> RealtimePreviewTargetFormat {
        self.format
    }

    pub(crate) fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    pub(crate) fn prepare_for_present(&mut self) -> Result<(), PreviewSurfaceError> {
        #[cfg(target_os = "macos")]
        if let Some(attachment) = self.macos_attachment.as_mut() {
            attachment.prepare_for_present(self.bounds)?;
        }
        Ok(())
    }

    pub(crate) fn drawable_lifecycle_diagnostic(&self) -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            return self
                .macos_attachment
                .as_ref()
                .map(|attachment| attachment.drawable_lifecycle_diagnostic());
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    pub fn screen_rect(&self) -> Option<PreviewSurfaceScreenRect> {
        #[cfg(target_os = "macos")]
        {
            return self
                .macos_attachment
                .as_ref()
                .map(|attachment| attachment.screen_rect());
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    pub(crate) fn update_bounds(
        &mut self,
        device: &wgpu::Device,
        bounds: PreviewSurfaceBounds,
    ) -> Result<(), PreviewSurfaceError> {
        let bounds = bounds.validate()?;
        #[cfg(target_os = "macos")]
        if let Some(attachment) = self.macos_attachment.as_mut() {
            attachment.update_bounds(bounds)?;
        }
        self.bounds = bounds;
        self.config.width = bounds.width;
        self.config.height = bounds.height;
        self.surface.configure(device, &self.config);
        self.descriptor = match self.descriptor {
            PreviewSurfaceDescriptor::NativeChild {
                parent_window_handle,
                ..
            } => PreviewSurfaceDescriptor::NativeChild {
                parent_window_handle,
                bounds,
            },
            PreviewSurfaceDescriptor::Offscreen { .. } => PreviewSurfaceDescriptor::Offscreen {
                width: bounds.width,
                height: bounds.height,
                scale_factor_millis: bounds.scale_factor_millis,
            },
        };
        Ok(())
    }
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
        assert_eq!(
            error.kind(),
            PreviewSurfaceDiagnosticKind::SurfaceUnavailable
        );

        host.mark_lost("wgpu surface lost");
        let error = host
            .update_bounds(valid_bounds())
            .expect_err("lost surface rejects updates");
        assert_eq!(error.kind(), PreviewSurfaceDiagnosticKind::SurfaceLost);
    }
}

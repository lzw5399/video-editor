use std::error::Error;
use std::fmt;
use std::sync::Arc;

use super::texture_cache::RealtimePreviewExternalTexturePlanes;

use super::surface::{
    NativeParentWindowHandle, PreviewSurfaceDescriptor, PreviewSurfaceDiagnosticKind,
    PreviewSurfaceError, RealtimePreviewGpuPresentationTarget, RealtimePreviewGpuTarget,
    RealtimePreviewTargetError, RealtimePreviewTargetFormat,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimePreviewGpuBackend {
    Auto,
    D3d12,
    Metal,
    OffscreenOnly,
    Mock,
}

impl RealtimePreviewGpuBackend {
    pub const fn resolve_for_current_platform(self) -> Self {
        match self {
            Self::Auto => {
                #[cfg(target_os = "windows")]
                {
                    Self::D3d12
                }
                #[cfg(target_os = "macos")]
                {
                    Self::Metal
                }
                #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                {
                    Self::OffscreenOnly
                }
            }
            backend => backend,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimePreviewGpuDeviceDescriptor {
    pub backend: RealtimePreviewGpuBackend,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RealtimePreviewGpuDevice {
    backend: RealtimePreviewGpuBackend,
    instance: Option<Arc<wgpu::Instance>>,
    adapter: Option<Arc<wgpu::Adapter>>,
    device: Option<Arc<wgpu::Device>>,
    queue: Option<Arc<wgpu::Queue>>,
}

impl RealtimePreviewGpuDevice {
    pub fn bootstrap(
        descriptor: RealtimePreviewGpuDeviceDescriptor,
    ) -> Result<Self, RealtimePreviewGpuError> {
        let backend = descriptor.backend.resolve_for_current_platform();
        match backend {
            RealtimePreviewGpuBackend::Mock | RealtimePreviewGpuBackend::OffscreenOnly => {
                Ok(Self {
                    backend,
                    instance: None,
                    adapter: None,
                    device: None,
                    queue: None,
                })
            }
            RealtimePreviewGpuBackend::D3d12 | RealtimePreviewGpuBackend::Metal => {
                pollster::block_on(Self::bootstrap_wgpu(backend, descriptor.label))
            }
            RealtimePreviewGpuBackend::Auto => unreachable!("Auto backend must resolve first"),
        }
    }

    async fn bootstrap_wgpu(
        backend: RealtimePreviewGpuBackend,
        label: Option<String>,
    ) -> Result<Self, RealtimePreviewGpuError> {
        let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_descriptor.backends = wgpu_backends(backend)?;
        let instance = wgpu::Instance::new(instance_descriptor);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| RealtimePreviewGpuError::NoGpuAdapter { backend })?;

        let label_ref = label.as_deref();
        let supported_features = adapter.features();
        let required_features = supported_features & wgpu::Features::EXTERNAL_TEXTURE;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: label_ref,
                required_features,
                required_limits: wgpu::Limits::downlevel_defaults(),
                ..Default::default()
            })
            .await
            .map_err(|error| RealtimePreviewGpuError::DeviceRequest {
                backend,
                message: error.to_string(),
            })?;

        Ok(Self {
            backend,
            instance: Some(Arc::new(instance)),
            adapter: Some(Arc::new(adapter)),
            device: Some(Arc::new(device)),
            queue: Some(Arc::new(queue)),
        })
    }

    pub const fn backend(&self) -> RealtimePreviewGpuBackend {
        self.backend
    }

    pub const fn uses_physical_adapter(&self) -> bool {
        self.device.is_some() && self.queue.is_some()
    }

    pub fn supports_external_texture(&self) -> bool {
        self.device
            .as_deref()
            .map(|device| device.features().contains(wgpu::Features::EXTERNAL_TEXTURE))
            .unwrap_or(false)
    }

    pub fn create_offscreen_target(
        &self,
        width: u32,
        height: u32,
        scale_factor_millis: u32,
        format: RealtimePreviewTargetFormat,
    ) -> Result<RealtimePreviewGpuTarget, RealtimePreviewGpuError> {
        let Some(device) = &self.device else {
            return RealtimePreviewGpuTarget::offscreen(width, height, scale_factor_millis, format)
                .map_err(RealtimePreviewGpuError::InvalidTarget);
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("realtime-preview-offscreen-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: format.wgpu_format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        RealtimePreviewGpuTarget::with_texture(width, height, scale_factor_millis, format, texture)
            .map_err(RealtimePreviewGpuError::InvalidTarget)
    }

    pub fn create_presentation_target(
        &self,
        descriptor: PreviewSurfaceDescriptor,
        requested_format: RealtimePreviewTargetFormat,
    ) -> Result<RealtimePreviewGpuPresentationTarget, PreviewSurfaceError> {
        let descriptor = descriptor.validate()?;
        let PreviewSurfaceDescriptor::NativeChild {
            parent_window_handle,
            ..
        } = descriptor
        else {
            return Err(PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::PlatformUnavailable,
                "offscreen targets cannot satisfy product presentation",
            ));
        };
        if matches!(parent_window_handle, NativeParentWindowHandle::Mock(_)) {
            return Err(PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::PlatformUnavailable,
                "mock targets cannot satisfy product presentation",
            ));
        }

        let instance = self.instance.as_deref().ok_or_else(|| {
            PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::PlatformUnavailable,
                "product presentation requires a real WGPU instance",
            )
        })?;
        let adapter = self.adapter.as_deref().ok_or_else(|| {
            PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::PlatformUnavailable,
                "product presentation requires a real WGPU adapter",
            )
        })?;
        let device = self.device.as_deref().ok_or_else(|| {
            PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::PlatformUnavailable,
                "product presentation requires a real WGPU device",
            )
        })?;

        let bounds = descriptor.bounds();
        let native_surface = create_native_surface(instance, parent_window_handle, bounds)?;
        let surface = native_surface.surface;
        #[cfg(target_os = "macos")]
        let macos_attachment = native_surface.macos_attachment;
        let mut config = surface
            .get_default_config(adapter, bounds.width, bounds.height)
            .ok_or_else(|| {
                PreviewSurfaceError::new(
                    PreviewSurfaceDiagnosticKind::PlatformUnavailable,
                    "native preview surface is not compatible with the WGPU adapter",
                )
            })?;
        let requested_wgpu_format = requested_format.wgpu_format();
        let capabilities = surface.get_capabilities(adapter);
        let configured_format = if capabilities.formats.contains(&requested_wgpu_format) {
            requested_format
        } else {
            capabilities
                .formats
                .iter()
                .copied()
                .find_map(RealtimePreviewTargetFormat::from_wgpu_format)
                .ok_or_else(|| {
                    PreviewSurfaceError::new(
                        PreviewSurfaceDiagnosticKind::PlatformUnavailable,
                        "native preview surface has no supported sRGB presentation format",
                    )
                })?
        };
        config.format = configured_format.wgpu_format();
        config.width = bounds.width;
        config.height = bounds.height;
        config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        surface.configure(device, &config);

        let target = RealtimePreviewGpuPresentationTarget::new(
            descriptor,
            configured_format,
            surface,
            config,
        );
        #[cfg(target_os = "macos")]
        {
            if let Some(attachment) = macos_attachment {
                return Ok(target.with_macos_attachment(attachment));
            }
        }
        Ok(target)
    }

    pub fn create_nv12_external_texture_planes(
        &self,
        width: u32,
        height: u32,
        luma_pixels: &[u8],
        chroma_pixels: &[u8],
    ) -> Result<RealtimePreviewExternalTexturePlanes, RealtimePreviewGpuError> {
        let device = self
            .device
            .as_deref()
            .ok_or(RealtimePreviewGpuError::WgpuDeviceUnavailable)?;
        let queue = self
            .queue
            .as_deref()
            .ok_or(RealtimePreviewGpuError::WgpuQueueUnavailable)?;
        if !device.features().contains(wgpu::Features::EXTERNAL_TEXTURE) {
            return Err(RealtimePreviewGpuError::ExternalTextureUnsupported);
        }
        if width == 0 || height == 0 || width % 2 != 0 || height % 2 != 0 {
            return Err(RealtimePreviewGpuError::InvalidExternalTexturePlanes(
                "NV12 external texture planes require non-zero even dimensions".to_owned(),
            ));
        }
        let expected_luma = width as usize * height as usize;
        let expected_chroma = width as usize * (height as usize / 2);
        if luma_pixels.len() != expected_luma || chroma_pixels.len() != expected_chroma {
            return Err(RealtimePreviewGpuError::InvalidExternalTexturePlanes(
                format!(
                    "NV12 plane byte lengths must be luma={expected_luma}, chroma={expected_chroma}"
                ),
            ));
        }

        let luma = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("realtime-preview-nv12-luma-plane"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let chroma = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("realtime-preview-nv12-chroma-plane"),
            size: wgpu::Extent3d {
                width: width / 2,
                height: height / 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &luma,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            luma_pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &chroma,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            chroma_pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height / 2),
            },
            wgpu::Extent3d {
                width: width / 2,
                height: height / 2,
                depth_or_array_layers: 1,
            },
        );

        Ok(RealtimePreviewExternalTexturePlanes::new(
            width, height, luma, chroma,
        ))
    }

    #[cfg(target_os = "macos")]
    pub fn create_nv12_external_texture_planes_from_metal(
        &self,
        width: u32,
        height: u32,
        luma: objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_metal::MTLTexture>>,
        chroma: objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_metal::MTLTexture>>,
    ) -> Result<RealtimePreviewExternalTexturePlanes, RealtimePreviewGpuError> {
        let device = self
            .device
            .as_deref()
            .ok_or(RealtimePreviewGpuError::WgpuDeviceUnavailable)?;
        Self::create_nv12_external_texture_planes_from_metal_device(
            device, width, height, luma, chroma,
        )
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn create_nv12_external_texture_planes_from_metal_device(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        luma: objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_metal::MTLTexture>>,
        chroma: objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_metal::MTLTexture>>,
    ) -> Result<RealtimePreviewExternalTexturePlanes, RealtimePreviewGpuError> {
        use objc2_metal::MTLTextureType;
        use wgpu::hal::{self, CopyExtent};

        if !device.features().contains(wgpu::Features::EXTERNAL_TEXTURE) {
            return Err(RealtimePreviewGpuError::ExternalTextureUnsupported);
        }
        if width == 0 || height == 0 || width % 2 != 0 || height % 2 != 0 {
            return Err(RealtimePreviewGpuError::InvalidExternalTexturePlanes(
                "NV12 Metal external texture planes require non-zero even dimensions".to_owned(),
            ));
        }

        let luma = unsafe {
            wrap_metal_plane_texture(
                device,
                luma,
                "realtime-preview-cvmetal-luma-plane",
                wgpu::TextureFormat::R8Unorm,
                MTLTextureType::Type2D,
                width,
                height,
            )
        };
        let chroma = unsafe {
            wrap_metal_plane_texture(
                device,
                chroma,
                "realtime-preview-cvmetal-chroma-plane",
                wgpu::TextureFormat::Rg8Unorm,
                MTLTextureType::Type2D,
                width / 2,
                height / 2,
            )
        };

        unsafe fn wrap_metal_plane_texture(
            device: &wgpu::Device,
            raw: objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_metal::MTLTexture>>,
            label: &'static str,
            format: wgpu::TextureFormat,
            raw_type: objc2_metal::MTLTextureType,
            width: u32,
            height: u32,
        ) -> wgpu::Texture {
            let hal_texture = unsafe {
                hal::metal::Device::texture_from_raw(
                    raw,
                    format,
                    raw_type,
                    1,
                    1,
                    CopyExtent {
                        width,
                        height,
                        depth: 1,
                    },
                )
            };
            unsafe {
                device.create_texture_from_hal::<hal::api::Metal>(
                    hal_texture,
                    &wgpu::TextureDescriptor {
                        label: Some(label),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                )
            }
        }

        Ok(RealtimePreviewExternalTexturePlanes::new(
            width, height, luma, chroma,
        ))
    }

    pub fn resize_presentation_target(
        &self,
        target: &mut RealtimePreviewGpuPresentationTarget,
        bounds: super::surface::PreviewSurfaceBounds,
    ) -> Result<(), PreviewSurfaceError> {
        let device = self.device.as_deref().ok_or_else(|| {
            PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::PlatformUnavailable,
                "product presentation resize requires a real WGPU device",
            )
        })?;
        target.update_bounds(device, bounds)
    }

    pub(crate) fn device(&self) -> Option<&wgpu::Device> {
        self.device.as_deref()
    }

    pub(crate) fn queue(&self) -> Option<&wgpu::Queue> {
        self.queue.as_deref()
    }
}

struct NativeSurfaceCreation {
    surface: wgpu::Surface<'static>,
    #[cfg(target_os = "macos")]
    macos_attachment: Option<crate::platform::macos::MacosWgpuSurfaceAttachment>,
}

fn create_native_surface(
    instance: &wgpu::Instance,
    handle: NativeParentWindowHandle,
    bounds: super::surface::PreviewSurfaceBounds,
) -> Result<NativeSurfaceCreation, PreviewSurfaceError> {
    #[cfg(target_os = "macos")]
    {
        let attachment = crate::platform::macos::MacosWgpuSurfaceAttachment::new(handle, bounds)?;
        let target =
            wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(attachment.core_animation_layer());
        let surface = unsafe { instance.create_surface_unsafe(target) }
            .map_err(|error| surface_error(error.to_string()))?;
        return Ok(NativeSurfaceCreation {
            surface,
            macos_attachment: Some(attachment),
        });
    }

    #[cfg(target_os = "windows")]
    {
        let raw_handle = crate::platform::windows::raw_window_handle(handle)?;
        let target = wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: None,
            raw_window_handle: raw_handle.into(),
        };
        let _ = bounds;
        let surface = unsafe { instance.create_surface_unsafe(target) }
            .map_err(|error| surface_error(error.to_string()))?;
        return Ok(NativeSurfaceCreation { surface });
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (instance, handle, bounds);
        Err(PreviewSurfaceError::new(
            PreviewSurfaceDiagnosticKind::PlatformUnavailable,
            "native WGPU preview surfaces are supported only on macOS and Windows",
        ))
    }
}

fn surface_error(message: String) -> PreviewSurfaceError {
    PreviewSurfaceError::new(
        PreviewSurfaceDiagnosticKind::PlatformUnavailable,
        format!("failed to create native WGPU preview surface: {message}"),
    )
}

#[derive(Debug)]
pub enum RealtimePreviewGpuError {
    UnsupportedBackend {
        backend: RealtimePreviewGpuBackend,
        platform: &'static str,
    },
    NoGpuAdapter {
        backend: RealtimePreviewGpuBackend,
    },
    DeviceRequest {
        backend: RealtimePreviewGpuBackend,
        message: String,
    },
    WgpuDeviceUnavailable,
    WgpuQueueUnavailable,
    ExternalTextureUnsupported,
    InvalidExternalTexturePlanes(String),
    InvalidTarget(RealtimePreviewTargetError),
}

impl fmt::Display for RealtimePreviewGpuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedBackend { backend, platform } => {
                write!(formatter, "{backend:?} is not supported on {platform}")
            }
            Self::NoGpuAdapter { backend } => {
                write!(formatter, "no wgpu adapter found for {backend:?}")
            }
            Self::DeviceRequest { backend, message } => {
                write!(formatter, "failed to request {backend:?} device: {message}")
            }
            Self::WgpuDeviceUnavailable => formatter.write_str("wgpu device is unavailable"),
            Self::WgpuQueueUnavailable => formatter.write_str("wgpu queue is unavailable"),
            Self::ExternalTextureUnsupported => {
                formatter.write_str("wgpu external texture feature is unavailable")
            }
            Self::InvalidExternalTexturePlanes(message) => {
                write!(formatter, "invalid external texture planes: {message}")
            }
            Self::InvalidTarget(error) => error.fmt(formatter),
        }
    }
}

impl Error for RealtimePreviewGpuError {}

fn wgpu_backends(
    backend: RealtimePreviewGpuBackend,
) -> Result<wgpu::Backends, RealtimePreviewGpuError> {
    match backend {
        RealtimePreviewGpuBackend::D3d12 => {
            if cfg!(target_os = "windows") {
                Ok(wgpu::Backends::DX12)
            } else {
                Err(RealtimePreviewGpuError::UnsupportedBackend {
                    backend,
                    platform: std::env::consts::OS,
                })
            }
        }
        RealtimePreviewGpuBackend::Metal => {
            if cfg!(target_os = "macos") {
                Ok(wgpu::Backends::METAL)
            } else {
                Err(RealtimePreviewGpuError::UnsupportedBackend {
                    backend,
                    platform: std::env::consts::OS,
                })
            }
        }
        RealtimePreviewGpuBackend::Auto
        | RealtimePreviewGpuBackend::OffscreenOnly
        | RealtimePreviewGpuBackend::Mock => unreachable!("only real backends request wgpu"),
    }
}

use std::error::Error;
use std::fmt;
use std::sync::Arc;

use super::surface::{
    RealtimePreviewGpuTarget, RealtimePreviewTargetError, RealtimePreviewTargetFormat,
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

#[derive(Debug)]
pub struct RealtimePreviewGpuDevice {
    backend: RealtimePreviewGpuBackend,
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
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: label_ref,
                required_features: wgpu::Features::empty(),
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

    pub fn create_offscreen_target(
        &self,
        width: u32,
        height: u32,
        scale_factor_millis: u32,
        format: RealtimePreviewTargetFormat,
    ) -> Result<RealtimePreviewGpuTarget, RealtimePreviewGpuError> {
        let Some(device) = &self.device else {
            return RealtimePreviewGpuTarget::offscreen(
                width,
                height,
                scale_factor_millis,
                format,
            )
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

    pub(crate) fn device(&self) -> Option<&wgpu::Device> {
        self.device.as_deref()
    }

    pub(crate) fn queue(&self) -> Option<&wgpu::Queue> {
        self.queue.as_deref()
    }
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

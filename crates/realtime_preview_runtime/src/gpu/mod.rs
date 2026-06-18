//! GPU preview backend contracts.
//!
//! The module consumes render graph and frame-provider outputs. It does not own
//! timeline semantics, FFmpeg compilation, or media process execution.

pub mod device;
pub mod surface;

pub use device::{
    RealtimePreviewGpuBackend, RealtimePreviewGpuDevice, RealtimePreviewGpuDeviceDescriptor,
    RealtimePreviewGpuError,
};
pub use surface::{RealtimePreviewGpuTarget, RealtimePreviewTargetFormat};

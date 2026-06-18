//! GPU preview backend contracts.
//!
//! The module consumes render graph and frame-provider outputs. It does not own
//! timeline semantics, FFmpeg compilation, or media process execution.

pub mod compositor;
pub mod device;
pub mod pipelines;
pub mod surface;
pub mod text;
pub mod texture_cache;

pub use compositor::{
    RealtimePreviewCompositor, RealtimePreviewCompositorError, RealtimePreviewCompositorOutput,
};
pub use device::{
    RealtimePreviewGpuBackend, RealtimePreviewGpuDevice, RealtimePreviewGpuDeviceDescriptor,
    RealtimePreviewGpuError,
};
pub use pipelines::RealtimePreviewPipelineSet;
pub use surface::{
    NativeParentWindowHandle, PreviewSurfaceAttachment, PreviewSurfaceBounds,
    PreviewSurfaceDescriptor, PreviewSurfaceDiagnosticKind, PreviewSurfaceError,
    PreviewSurfaceHost, PreviewSurfaceStatus, RealtimePreviewGpuTarget,
    RealtimePreviewTargetFormat,
};
pub use text::{TEXT_PARITY_UNPROVEN_REASON, TextPreviewOutcome, classify_text_preview_outcome};
pub use texture_cache::{
    RealtimePreviewTexture, RealtimePreviewTextureCache, RealtimePreviewTextureCacheError,
    RealtimePreviewTextureId,
};

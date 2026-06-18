//! Preview service boundary.
//!
//! This crate owns preview frame/segment requests, derived cache artifacts, and
//! cache invalidation. It consumes Rust semantic/render/compiler services and
//! keeps cache keys out of the renderer and `.veproj/project.json`.

pub mod cache;
pub mod realtime_backend;
pub mod realtime_frame_provider;
pub mod service;

pub use cache::{
    PreviewArtifact, PreviewCacheEntry, PreviewCacheKey, PreviewCacheProfile,
    PreviewInvalidationRequest, PreviewInvalidationResult, accepted_audio_edit_invalidation,
    accepted_edit_ranges_invalidation, accepted_text_edit_invalidation,
    accepted_timeline_edit_invalidation, changed_material_invalidation,
    changed_materials_invalidation, changed_range_invalidation, invalidate_preview_cache,
};
pub use realtime_backend::{
    RealtimePreviewFallbackDecision, RealtimePreviewFrameServiceRequest,
    RealtimePreviewServiceConfig, RealtimePreviewServiceFrameResponse,
    request_realtime_preview_frame,
};
pub use realtime_frame_provider::RealtimeMaterialFrameProvider;
pub use service::{
    PreviewFrameRequest, PreviewFrameResponse, PreviewSegmentRequest, PreviewSegmentResponse,
    PreviewServiceConfig, PreviewServiceError, PreviewServiceErrorKind, request_preview_frame,
    request_preview_segment,
};

/// Future preview renderer boundary consumed by preview services.
pub trait PreviewRenderer {
    /// Stable renderer label for diagnostics.
    fn renderer_name(&self) -> &'static str;
}

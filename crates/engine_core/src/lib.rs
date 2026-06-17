//! Pure Rust evaluation engine for draft timelines.
//!
//! This crate will normalize drafts, resolve Track and Segment timing, evaluate
//! frame state, and keep preview/export semantics on one path. It must stay free
//! of filesystem, Electron, FFmpeg process, and platform runtime dependencies.

pub mod frame_state;
pub mod normalize;

pub use frame_state::{
    FrameAudioSegment, FrameState, FrameTextOverlay, FrameVisualLayer, RenderRange,
    RenderRangeState, frame_index_to_microseconds, resolve_frame_state, resolve_render_range,
};
pub use normalize::{
    EngineDiagnostic, EngineError, EngineErrorKind, EngineProfile, MaterialRenderableState,
    NormalizedDraft, NormalizedMaterialRef, NormalizedSegment, NormalizedTrack, normalize_draft,
};

/// Boundary marker for the semantic engine crate.
pub const ENGINE_CORE_BOUNDARY: &str = "pure-semantic";

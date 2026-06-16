//! Pure Rust evaluation engine for draft timelines.
//!
//! This crate will normalize drafts, resolve Track and Segment timing, evaluate
//! frame state, and keep preview/export semantics on one path. It must stay free
//! of filesystem, Electron, FFmpeg process, and platform runtime dependencies.

/// Boundary marker for the semantic engine crate.
pub const ENGINE_CORE_BOUNDARY: &str = "pure-semantic";

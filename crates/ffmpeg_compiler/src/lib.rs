//! FFmpeg command-plan compiler boundary.
//!
//! This crate will compile typed render graph intents into FFmpeg inputs, filter
//! scripts, subtitle artifacts, and encode argument plans. It must not own draft
//! editing behavior, UI state, or process execution.

/// Boundary marker for FFmpeg compilation planning.
pub const FFMPEG_COMPILER_BOUNDARY: &str = "render-graph-to-ffmpeg-plan";

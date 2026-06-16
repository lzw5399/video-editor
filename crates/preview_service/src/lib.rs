//! Preview service boundary.
//!
//! This crate will own preview frames, preview segments, thumbnails, waveform
//! cache, and cache invalidation. Phase 1 defines only the future renderer
//! boundary; it does not generate preview media or implement playback behavior.

/// Future preview renderer boundary consumed by preview services.
pub trait PreviewRenderer {
    /// Stable renderer label for diagnostics.
    fn renderer_name(&self) -> &'static str;
}

//! Realtime preview material frame provider boundary.
//!
//! Phase 11 keeps decoded H.264 software frames in Rust-owned runtime caches.
//! The preview service consumes this provider boundary instead of invoking
//! FFmpeg while handling supported seek/scrub requests.

pub type RealtimeMaterialFrameProvider = realtime_preview_runtime::SoftwareVideoFrameProvider;

//! Rust-owned task scheduling boundary contracts.
//!
//! `task_runtime` is the portable scheduler ownership boundary for preview,
//! audio, artifact, probe, project IO, and export work. Domain crates keep
//! owning their editing or execution semantics; this crate owns the shared
//! scheduler contracts, freshness vocabulary, and later runtime policy.

pub mod freshness;

pub use freshness::{
    PlaybackGeneration, PlaybackRate, PlaybackRateError, PlaybackState, TimelineClock,
    TimelineFreshness,
};

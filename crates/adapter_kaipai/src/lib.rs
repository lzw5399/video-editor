//! Offline Kaipai formula adapter boundary.
//!
//! This crate owns sanitized offline Kaipai formula bundle contracts,
//! external provenance, and fixture-facing validation. Raw provider formula
//! JSON remains adapter input evidence here and must not become canonical
//! `.veproj/project.json` draft, engine, render graph, or FFmpeg semantics.

mod error;
mod formula_bundle;

pub use error::AdapterKaipaiError;

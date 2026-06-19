//! Rust-owned derived artifact store boundary.
//!
//! This crate owns `.veproj/derived` storage facts. The canonical project
//! semantics remain in `.veproj/project.json`.

pub mod blob_store;
pub mod dependencies;
pub mod error;
pub mod fingerprint;
pub mod gc;
pub mod generation;
pub mod invalidation;
pub mod jobs;
pub mod paths;
pub mod resource_index;
pub mod schema;

pub use error::ArtifactStoreError;

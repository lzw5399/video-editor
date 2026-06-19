//! Rust-owned derived artifact store boundary.
//!
//! This crate owns `.veproj/derived` storage facts. The canonical project
//! semantics remain in `.veproj/project.json`.

pub mod blob_store;
pub mod error;
pub mod fingerprint;
pub mod paths;
pub mod resource_index;
pub mod schema;

pub use error::ArtifactStoreError;

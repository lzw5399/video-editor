//! Rust-owned derived artifact store boundary.
//!
//! This crate owns `.veproj/derived` storage facts. The canonical project
//! semantics remain in `.veproj/project.json`.

pub mod error;
pub mod schema;

pub use error::ArtifactStoreError;

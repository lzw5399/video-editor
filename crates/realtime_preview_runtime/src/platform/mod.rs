//! Native desktop preview surface contracts.
//!
//! Platform modules translate Rust-owned preview surface descriptors into the
//! raw-window-handle vocabulary expected by GPU surface setup. They do not
//! expose native child handles to TypeScript.

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

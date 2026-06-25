//! Compile-safe server runtime crate shell over `editor_runtime`.
//!
//! Plan 18-05 fills the Electron-free export runner. This crate exists now so
//! future server code compiles against the shared runtime API instead of
//! reaching into desktop Node bindings.

pub use editor_runtime::{
    EDITOR_RUNTIME_CONTRACT_VERSION, ExportService, ProjectSessionService,
    RuntimeSessionRegistry,
};

pub fn contract_version() -> &'static str {
    EDITOR_RUNTIME_CONTRACT_VERSION
}

//! Compile-safe C ABI adapter crate shell over `editor_runtime`.
//!
//! Plan 18-04 fills the stable C ABI. This crate intentionally exposes no C ABI
//! yet, so no adapter can duplicate runtime/session semantics before the shared
//! Rust authority layer exists.

pub use editor_runtime::{EDITOR_RUNTIME_CONTRACT_VERSION, RuntimeErrorKind};

pub fn contract_version() -> &'static str {
    EDITOR_RUNTIME_CONTRACT_VERSION
}

//! Shared Rust runtime authority below adapter transports.
//!
//! `editor_runtime` owns portable runtime/session/export contracts. Node-API,
//! C ABI, future mobile bindings, and server entrypoints should translate their
//! transport payloads into these typed Rust contracts instead of duplicating
//! draft, project, export, or handle semantics.

pub mod error;
pub mod export;
pub mod project_session;
pub mod session;

pub use error::{RuntimeError, RuntimeErrorKind};
pub use export::{ExportService, ProjectSessionExportJob, StartProjectSessionExportRequest};
pub use project_session::{
    CreateProjectSessionRequest, ProjectSessionHandle, ProjectSessionOpened,
    ProjectSessionService, ProjectSessionSnapshot, ProjectSessionWarning,
    SaveProjectSessionRequest,
};
pub use session::{
    AdapterMetadata, RuntimeSession, RuntimeSessionConfig, RuntimeSessionId,
    RuntimeSessionRegistry,
};

pub const EDITOR_RUNTIME_CONTRACT_VERSION: &str = "0.1.0";

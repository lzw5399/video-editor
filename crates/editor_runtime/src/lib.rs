//! Shared Rust runtime authority below adapter transports.
//!
//! `editor_runtime` owns portable runtime/session/export contracts. Node-API,
//! C ABI, future mobile bindings, and server entrypoints should translate their
//! transport payloads into these typed Rust contracts instead of duplicating
//! draft, project, export, or handle semantics.

pub mod error;
pub mod export;
pub mod handles;
pub mod material_service;
pub mod project_session;
pub mod project_session_node;
pub mod session;
pub mod timeline_selection;

pub use error::{RuntimeError, RuntimeErrorKind};
pub use export::{ExportService, ProjectSessionExportJob, StartProjectSessionExportRequest};
pub use handles::{
    HandleAcquireRequest, HandleKind, HandleRegistry, HandleReleaseReport, HandleReleaseState,
    HandleResolution, HandleToken, RuntimeCloseReport, RuntimeLeakDiagnostic,
    TextureHandleDescriptor, TextureResolveExpectation,
};
pub use project_session::{
    CreateProjectSessionRequest, ProjectSessionHandle, ProjectSessionOpened, ProjectSessionService,
    ProjectSessionSnapshot, ProjectSessionWarning, SaveProjectSessionRequest,
};
pub use session::{
    AdapterMetadata, RuntimeSession, RuntimeSessionConfig, RuntimeSessionId, RuntimeSessionRegistry,
};

pub const EDITOR_RUNTIME_CONTRACT_VERSION: &str = "0.1.0";

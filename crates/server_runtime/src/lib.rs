//! First-party server runtime over the shared Rust editor runtime.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use draft_model::{ExportJobPhase, ExportPreset, StartExportCommandPayload};
use editor_runtime::export::{ExportCommandError, SchedulerExportStatusResponse};
use editor_runtime::{
    EDITOR_RUNTIME_CONTRACT_VERSION, ExportService, ProjectSessionHandle, ProjectSessionOpened,
    ProjectSessionService, ProjectSessionSnapshot, RuntimeError, RuntimeErrorKind, RuntimeSession,
    RuntimeSessionConfig, RuntimeSessionId, RuntimeSessionRegistry,
};
use media_runtime::{DiscoveryError, RuntimeConfig, discover_runtime_config};
use project_store::{ProjectStoreError, resolve_material_uri};
use serde::{Deserialize, Serialize};

pub fn contract_version() -> &'static str {
    EDITOR_RUNTIME_CONTRACT_VERSION
}

pub struct ServerRuntime {
    runtime_sessions: RuntimeSessionRegistry,
    runtime_session: RuntimeSession,
    project_sessions: Mutex<ProjectSessionService>,
    exports: ExportService,
    runtime_config: RuntimeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerExportRequest {
    pub project: ProjectSessionHandle,
    pub output_path: PathBuf,
    pub preset: ExportPreset,
}

impl ServerExportRequest {
    pub fn new(
        project: ProjectSessionHandle,
        output_path: impl Into<PathBuf>,
        preset: ExportPreset,
    ) -> Self {
        Self {
            project,
            output_path: output_path.into(),
            preset,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerRuntimeErrorKind {
    RuntimeDiscovery,
    RuntimeSession,
    ProjectSession,
    Export,
    Timeout,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRuntimeError {
    pub kind: ServerRuntimeErrorKind,
    pub message: String,
}

impl ServerRuntimeError {
    pub fn new(kind: ServerRuntimeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> ServerRuntimeErrorKind {
        self.kind
    }
}

impl fmt::Display for ServerRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ServerRuntimeError {}

impl From<DiscoveryError> for ServerRuntimeError {
    fn from(error: DiscoveryError) -> Self {
        Self::new(
            ServerRuntimeErrorKind::RuntimeDiscovery,
            format!("{error}: {}", error.remediation),
        )
    }
}

impl From<RuntimeError> for ServerRuntimeError {
    fn from(error: RuntimeError) -> Self {
        let kind = match error.kind {
            RuntimeErrorKind::UnknownRuntimeSession => ServerRuntimeErrorKind::RuntimeSession,
            RuntimeErrorKind::UnknownProjectSession | RuntimeErrorKind::ProjectStore => {
                ServerRuntimeErrorKind::ProjectSession
            }
            RuntimeErrorKind::Scheduler => ServerRuntimeErrorKind::Export,
            _ => ServerRuntimeErrorKind::Internal,
        };
        Self::new(kind, error.message)
    }
}

impl From<ProjectStoreError> for ServerRuntimeError {
    fn from(error: ProjectStoreError) -> Self {
        Self::new(ServerRuntimeErrorKind::ProjectSession, error.to_string())
    }
}

impl From<ExportCommandError> for ServerRuntimeError {
    fn from(error: ExportCommandError) -> Self {
        Self::new(ServerRuntimeErrorKind::Export, error.to_string())
    }
}

impl ServerRuntime {
    pub fn new() -> Result<Self, ServerRuntimeError> {
        Self::with_runtime_config(discover_runtime_config()?)
    }

    pub fn with_runtime_config(runtime_config: RuntimeConfig) -> Result<Self, ServerRuntimeError> {
        let mut runtime_sessions = RuntimeSessionRegistry::default();
        let runtime_session = runtime_sessions.create_session(RuntimeSessionConfig {
            diagnostic_label: Some("server_runtime".to_owned()),
        })?;
        Ok(Self {
            runtime_sessions,
            runtime_session,
            project_sessions: Mutex::new(ProjectSessionService::default()),
            exports: ExportService::default(),
            runtime_config,
        })
    }

    pub fn runtime_session(&self) -> &RuntimeSession {
        &self.runtime_session
    }

    pub fn session_id(&self) -> &RuntimeSessionId {
        &self.runtime_session.id
    }

    pub fn runtime_config(&self) -> &RuntimeConfig {
        &self.runtime_config
    }

    pub fn open_project(
        &self,
        bundle_path: impl AsRef<Path>,
    ) -> Result<ProjectSessionOpened, ServerRuntimeError> {
        self.assert_runtime_session()?;
        let mut project_sessions = self.lock_project_sessions()?;
        project_sessions
            .open_project_session(self.runtime_session.id.clone(), bundle_path)
            .map_err(ServerRuntimeError::from)
    }

    pub fn start_export(
        &self,
        request: ServerExportRequest,
    ) -> Result<SchedulerExportStatusResponse, ServerRuntimeError> {
        let snapshot = self.project_snapshot(&request.project)?;
        let draft = resolve_snapshot_materials_for_export(&snapshot)?;
        let payload = StartExportCommandPayload {
            draft,
            output_path: request.output_path.display().to_string(),
            preset: request.preset,
            dirty_facts: None,
        };
        self.exports
            .start_export(self.runtime_config.clone(), payload)
            .map_err(ServerRuntimeError::from)
    }

    pub fn get_export_status(
        &self,
        job_id: &str,
    ) -> Result<SchedulerExportStatusResponse, ServerRuntimeError> {
        self.exports
            .status(job_id)
            .map_err(ServerRuntimeError::from)
    }

    pub fn cancel_export(
        &self,
        job_id: &str,
    ) -> Result<SchedulerExportStatusResponse, ServerRuntimeError> {
        self.exports
            .cancel(job_id)
            .map_err(ServerRuntimeError::from)
    }

    pub fn wait_for_export(
        &self,
        job_id: &str,
        timeout: Duration,
    ) -> Result<SchedulerExportStatusResponse, ServerRuntimeError> {
        let started_at = Instant::now();
        loop {
            let status = self.get_export_status(job_id)?;
            if is_terminal_export_phase(status.status.phase) {
                return Ok(status);
            }
            if started_at.elapsed() >= timeout {
                return Err(ServerRuntimeError::new(
                    ServerRuntimeErrorKind::Timeout,
                    format!("server export timed out waiting for job {job_id}"),
                ));
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn project_snapshot(
        &self,
        handle: &ProjectSessionHandle,
    ) -> Result<ProjectSessionSnapshot, ServerRuntimeError> {
        let project_sessions = self.lock_project_sessions()?;
        project_sessions
            .snapshot(handle)
            .map_err(ServerRuntimeError::from)
    }

    fn lock_project_sessions(
        &self,
    ) -> Result<MutexGuard<'_, ProjectSessionService>, ServerRuntimeError> {
        self.project_sessions.lock().map_err(|_| {
            ServerRuntimeError::new(
                ServerRuntimeErrorKind::Internal,
                "server project session lock is poisoned",
            )
        })
    }

    fn assert_runtime_session(&self) -> Result<(), ServerRuntimeError> {
        self.runtime_sessions
            .session(&self.runtime_session.id)
            .map(|_| ())
            .map_err(ServerRuntimeError::from)
    }
}

pub fn open_project(
    runtime: &ServerRuntime,
    bundle_path: impl AsRef<Path>,
) -> Result<ProjectSessionOpened, ServerRuntimeError> {
    runtime.open_project(bundle_path)
}

pub fn start_export(
    runtime: &ServerRuntime,
    request: ServerExportRequest,
) -> Result<SchedulerExportStatusResponse, ServerRuntimeError> {
    runtime.start_export(request)
}

pub fn get_export_status(
    runtime: &ServerRuntime,
    job_id: &str,
) -> Result<SchedulerExportStatusResponse, ServerRuntimeError> {
    runtime.get_export_status(job_id)
}

pub fn cancel_export(
    runtime: &ServerRuntime,
    job_id: &str,
) -> Result<SchedulerExportStatusResponse, ServerRuntimeError> {
    runtime.cancel_export(job_id)
}

pub fn wait_for_export(
    runtime: &ServerRuntime,
    job_id: &str,
    timeout: Duration,
) -> Result<SchedulerExportStatusResponse, ServerRuntimeError> {
    runtime.wait_for_export(job_id, timeout)
}

pub fn is_terminal_export_phase(phase: ExportJobPhase) -> bool {
    matches!(
        phase,
        ExportJobPhase::Completed
            | ExportJobPhase::Failed
            | ExportJobPhase::ValidationFailed
            | ExportJobPhase::Cancelled
    )
}

fn resolve_snapshot_materials_for_export(
    snapshot: &ProjectSessionSnapshot,
) -> Result<draft_model::Draft, ServerRuntimeError> {
    let mut draft = snapshot.draft.clone();
    for material in &mut draft.materials {
        if let Some(path) = resolve_material_uri(&snapshot.bundle_path, &material.uri)? {
            material.uri = format!("file://{}", path.display());
        }
    }
    Ok(draft)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use draft_model::Draft;
    use project_store::{StdPlatformFileSystem, create_project_bundle};

    use super::*;

    #[test]
    fn opens_project_bundle_with_runtime_owned_session_handle() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let bundle_path = temp_dir.path().join("server-open.veproj");
        let draft = Draft::new("server-open-draft", "Server Open Draft");
        create_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
            .expect("bundle should be created");

        let runtime = ServerRuntime::new().expect("server runtime should start");
        let opened = open_project(&runtime, &bundle_path).expect("server should open bundle");

        assert_eq!(opened.draft_id.as_str(), "server-open-draft");
        assert_eq!(opened.draft_name, "Server Open Draft");
        assert_eq!(opened.handle.owner_session(), runtime.session_id());
        assert!(opened.project_json_path.ends_with("project.json"));
    }

    #[test]
    fn crate_manifest_stays_on_shared_runtime_boundary() {
        let manifest = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
            .expect("manifest should be readable");

        assert!(manifest.contains("editor_runtime"));
        assert!(manifest.contains("media_runtime"));
        assert!(!manifest.contains("bindings_node"));
        assert!(!manifest.contains("napi"));
    }

    #[test]
    fn cli_entrypoint_routes_to_server_library_api() {
        let cli = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"))
            .expect("server CLI should exist");

        assert!(cli.contains("server_runtime"));
        assert!(cli.contains("open_project"));
        assert!(cli.contains("start_export"));
        assert!(cli.contains("get_export_status"));
        assert!(cli.contains("serde_json"));
    }
}

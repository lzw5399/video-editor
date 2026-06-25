use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use draft_model::{Draft, DraftId};
use project_store::{
    ProjectStoreError, ProjectStoreWarning as StoreWarning, StdPlatformFileSystem,
    create_project_bundle, open_project_bundle, save_project_bundle,
};
use serde::{Deserialize, Serialize};

use crate::{RuntimeError, RuntimeErrorKind, RuntimeSessionId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSessionHandle {
    token: String,
    owner_session: RuntimeSessionId,
    generation: u64,
}

impl ProjectSessionHandle {
    fn new(token: String, owner_session: RuntimeSessionId, generation: u64) -> Self {
        Self {
            token,
            owner_session,
            generation,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.token
    }

    pub fn owner_session(&self) -> &RuntimeSessionId {
        &self.owner_session
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}

#[derive(Debug, Clone)]
pub struct CreateProjectSessionRequest {
    pub runtime_session: RuntimeSessionId,
    pub bundle_path: PathBuf,
    pub draft: Draft,
}

#[derive(Debug, Clone)]
pub struct SaveProjectSessionRequest {
    pub handle: ProjectSessionHandle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ProjectSessionWarning {
    MissingMaterial {
        material_id: String,
        uri: String,
        resolved_path: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSessionOpened {
    pub handle: ProjectSessionHandle,
    pub revision: u64,
    pub bundle_path: PathBuf,
    pub project_json_path: PathBuf,
    pub draft_id: DraftId,
    pub draft_name: String,
    pub warnings: Vec<ProjectSessionWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSessionSnapshot {
    pub handle: ProjectSessionHandle,
    pub revision: u64,
    pub bundle_path: PathBuf,
    pub project_json_path: PathBuf,
    pub draft: Draft,
}

#[derive(Debug, Clone)]
struct ProjectSessionRecord {
    handle: ProjectSessionHandle,
    revision: u64,
    bundle_path: PathBuf,
    project_json_path: PathBuf,
    draft: Draft,
}

#[derive(Debug, Default)]
pub struct ProjectSessionService {
    next_id: u64,
    sessions: BTreeMap<ProjectSessionHandle, ProjectSessionRecord>,
}

impl ProjectSessionService {
    pub fn create_project_session(
        &mut self,
        request: CreateProjectSessionRequest,
    ) -> Result<ProjectSessionOpened, RuntimeError> {
        let bundle = create_project_bundle(
            &StdPlatformFileSystem,
            &request.bundle_path,
            &request.draft,
        )
        .map_err(project_store_error)?;
        Ok(self.insert_opened_project(
            request.runtime_session,
            bundle.bundle_path,
            bundle.project_json_path,
            bundle.draft,
            Vec::new(),
        ))
    }

    pub fn open_project_session(
        &mut self,
        runtime_session: RuntimeSessionId,
        bundle_path: impl AsRef<Path>,
    ) -> Result<ProjectSessionOpened, RuntimeError> {
        let opened = open_project_bundle(&StdPlatformFileSystem, bundle_path.as_ref())
            .map_err(project_store_error)?;
        Ok(self.insert_opened_project(
            runtime_session,
            opened.bundle.bundle_path,
            opened.bundle.project_json_path,
            opened.bundle.draft,
            opened.warnings.into_iter().map(runtime_warning).collect(),
        ))
    }

    pub fn save_project_session(
        &mut self,
        request: SaveProjectSessionRequest,
    ) -> Result<ProjectSessionSnapshot, RuntimeError> {
        let record = self.sessions.get_mut(&request.handle).ok_or_else(|| {
            RuntimeError::new(
                RuntimeErrorKind::UnknownProjectSession,
                format!("project session not found: {}", request.handle.as_str()),
            )
        })?;
        let bundle = save_project_bundle(
            &StdPlatformFileSystem,
            &record.bundle_path,
            &record.draft,
        )
        .map_err(project_store_error)?;
        record.revision = record.revision.saturating_add(1);
        record.bundle_path = bundle.bundle_path;
        record.project_json_path = bundle.project_json_path;
        record.draft = bundle.draft;
        Ok(record.snapshot())
    }

    pub fn snapshot(
        &self,
        handle: &ProjectSessionHandle,
    ) -> Result<ProjectSessionSnapshot, RuntimeError> {
        let record = self.sessions.get(handle).ok_or_else(|| {
            RuntimeError::new(
                RuntimeErrorKind::UnknownProjectSession,
                format!("project session not found: {}", handle.as_str()),
            )
        })?;
        Ok(record.snapshot())
    }

    fn insert_opened_project(
        &mut self,
        runtime_session: RuntimeSessionId,
        bundle_path: PathBuf,
        project_json_path: PathBuf,
        draft: Draft,
        warnings: Vec<ProjectSessionWarning>,
    ) -> ProjectSessionOpened {
        let id_number = self.next_id.saturating_add(1);
        self.next_id = id_number;
        let handle =
            ProjectSessionHandle::new(format!("projectSession-{id_number}"), runtime_session, 1);
        let opened = ProjectSessionOpened {
            handle: handle.clone(),
            revision: 1,
            bundle_path: bundle_path.clone(),
            project_json_path: project_json_path.clone(),
            draft_id: draft.draft_id.clone(),
            draft_name: draft.metadata.name.clone(),
            warnings,
        };
        self.sessions.insert(
            handle.clone(),
            ProjectSessionRecord {
                handle,
                revision: opened.revision,
                bundle_path,
                project_json_path,
                draft,
            },
        );
        opened
    }
}

impl ProjectSessionRecord {
    fn snapshot(&self) -> ProjectSessionSnapshot {
        ProjectSessionSnapshot {
            handle: self.handle.clone(),
            revision: self.revision,
            bundle_path: self.bundle_path.clone(),
            project_json_path: self.project_json_path.clone(),
            draft: self.draft.clone(),
        }
    }
}

fn project_store_error(error: ProjectStoreError) -> RuntimeError {
    RuntimeError::new(RuntimeErrorKind::ProjectStore, error.to_string())
}

fn runtime_warning(warning: StoreWarning) -> ProjectSessionWarning {
    match warning {
        StoreWarning::MissingMaterial {
            material_id,
            uri,
            resolved_path,
        } => ProjectSessionWarning::MissingMaterial {
            material_id,
            uri,
            resolved_path,
        },
    }
}

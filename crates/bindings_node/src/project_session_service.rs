use draft_model::{
    AddTimelineSegmentIntentCommandPayload, CommandErrorKind, CommandPayload, CommandState, Draft,
    MaterialId, TimelineCommandResponse, TimelineSelection,
};
use napi::bindgen_prelude::Result;
use project_store::{
    ProjectStoreError, StdPlatformFileSystem, open_project_bundle, save_project_bundle,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OpenProjectSessionRequest {
    bundle_path: String,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSessionOpenResponse {
    session_id: String,
    revision: u64,
    draft: Draft,
    bundle_path: String,
    project_json_path: String,
    warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectSessionRequest {
    session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExecuteProjectIntentRequest {
    session_id: String,
    expected_revision: u64,
    intent: ProjectIntent,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum ProjectIntent {
    AddTimelineSegmentIntent {
        #[serde(rename = "materialId")]
        material_id: MaterialId,
    },
    UndoTimelineEdit {},
    RedoTimelineEdit {},
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSessionClosedResponse {
    session_id: String,
    closed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSessionIntentResponse {
    session_id: String,
    revision: u64,
    draft: Draft,
    command_state: CommandState,
    selection: TimelineSelection,
    events: Vec<draft_model::CommandEvent>,
    delta: draft_model::CommandDelta,
    bundle_path: String,
    project_json_path: String,
}

#[derive(Debug, Default)]
struct ProjectSessionRegistry {
    sessions: HashMap<String, ProjectSession>,
    next_session_id: u64,
}

#[derive(Debug)]
struct ProjectSession {
    session_id: String,
    revision: u64,
    bundle_path: PathBuf,
    project_json_path: PathBuf,
    draft: Draft,
    command_state: CommandState,
    selection: TimelineSelection,
}

pub fn open_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<OpenProjectSessionRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid openProjectSession payload: {error}"),
                Some("openProjectSession".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.open_session(request))
}

pub fn close_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<ProjectSessionRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid closeProjectSession payload: {error}"),
                Some("closeProjectSession".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.close_session(request))
}

pub fn execute_project_intent(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<ExecuteProjectIntentRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid executeProjectIntent payload: {error}"),
                Some("executeProjectIntent".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.execute_intent(request))
}

fn with_project_session_registry(
    f: impl FnOnce(&mut ProjectSessionRegistry) -> Result<serde_json::Value>,
) -> Result<serde_json::Value> {
    static REGISTRY: OnceLock<Mutex<ProjectSessionRegistry>> = OnceLock::new();
    let registry = REGISTRY.get_or_init(|| Mutex::new(ProjectSessionRegistry::default()));
    let mut registry = registry.lock().map_err(|_| {
        napi::Error::from_reason("project session registry lock poisoned".to_string())
    })?;
    f(&mut registry)
}

impl ProjectSessionRegistry {
    fn open_session(&mut self, request: OpenProjectSessionRequest) -> Result<serde_json::Value> {
        let fs = StdPlatformFileSystem;
        let opened = match open_project_bundle(&fs, PathBuf::from(&request.bundle_path)) {
            Ok(opened) => opened,
            Err(error) => {
                return project_session_store_error("openProjectSession", error);
            }
        };
        let session_id = request.session_id.unwrap_or_else(|| self.next_session_id());
        let session = ProjectSession {
            session_id: session_id.clone(),
            revision: 0,
            bundle_path: opened.bundle.bundle_path.clone(),
            project_json_path: opened.bundle.project_json_path.clone(),
            draft: opened.bundle.draft.clone(),
            command_state: CommandState::empty(),
            selection: TimelineSelection::empty(),
        };
        self.sessions.insert(session_id.clone(), session);

        crate::to_js_value(crate::ok_envelope(ProjectSessionOpenResponse {
            session_id,
            revision: 0,
            draft: opened.bundle.draft,
            bundle_path: opened.bundle.bundle_path.display().to_string(),
            project_json_path: opened.bundle.project_json_path.display().to_string(),
            warnings: opened
                .warnings
                .into_iter()
                .map(crate::project_store_warning_message)
                .collect(),
        }))
    }

    fn close_session(&mut self, request: ProjectSessionRequest) -> Result<serde_json::Value> {
        let closed = self.sessions.remove(&request.session_id).is_some();
        crate::to_js_value(crate::ok_envelope(ProjectSessionClosedResponse {
            session_id: request.session_id,
            closed,
        }))
    }

    fn execute_intent(
        &mut self,
        request: ExecuteProjectIntentRequest,
    ) -> Result<serde_json::Value> {
        let Some(session) = self.sessions.get_mut(&request.session_id) else {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidProject,
                format!("Project session not found: {}", request.session_id),
                Some("executeProjectIntent".to_string()),
            ));
        };
        if request.expected_revision != session.revision {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!(
                    "Stale project session revision: expected {}, current {}",
                    request.expected_revision, session.revision
                ),
                Some("executeProjectIntent".to_string()),
            ));
        }

        let payload = session.intent_payload(request.intent);
        let response = match draft_commands::timeline::execute_timeline_edit(payload) {
            Ok(response) => response,
            Err(error) => {
                return crate::to_js_value(crate::error_envelope(
                    CommandErrorKind::InvalidTimelineEdit,
                    error.to_string(),
                    Some("executeProjectIntent".to_string()),
                ));
            }
        };
        session.apply_response(response)
    }

    fn next_session_id(&mut self) -> String {
        self.next_session_id = self.next_session_id.saturating_add(1);
        format!("project-session-{}", self.next_session_id)
    }
}

impl ProjectSession {
    fn intent_payload(&self, intent: ProjectIntent) -> CommandPayload {
        match intent {
            ProjectIntent::AddTimelineSegmentIntent { material_id } => {
                CommandPayload::AddTimelineSegmentIntent(AddTimelineSegmentIntentCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    material_id,
                })
            }
            ProjectIntent::UndoTimelineEdit {} => {
                CommandPayload::UndoTimelineEdit(draft_model::UndoTimelineEditCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                })
            }
            ProjectIntent::RedoTimelineEdit {} => {
                CommandPayload::RedoTimelineEdit(draft_model::RedoTimelineEditCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                })
            }
        }
    }

    fn apply_response(&mut self, response: TimelineCommandResponse) -> Result<serde_json::Value> {
        let fs = StdPlatformFileSystem;
        let saved = match save_project_bundle(&fs, &self.bundle_path, &response.draft) {
            Ok(saved) => saved,
            Err(error) => {
                return project_session_store_error("executeProjectIntent", error);
            }
        };
        self.revision = self.revision.saturating_add(1);
        self.draft = saved.draft;
        self.bundle_path = saved.bundle_path;
        self.project_json_path = saved.project_json_path;
        self.command_state = response.command_state;
        self.selection = response.selection;

        crate::to_js_value(crate::ok_envelope(ProjectSessionIntentResponse {
            session_id: self.session_id.clone(),
            revision: self.revision,
            draft: self.draft.clone(),
            command_state: self.command_state.clone(),
            selection: self.selection.clone(),
            events: response.events,
            delta: response.delta,
            bundle_path: self.bundle_path.display().to_string(),
            project_json_path: self.project_json_path.display().to_string(),
        }))
    }
}

fn project_session_store_error(
    command: &str,
    error: ProjectStoreError,
) -> Result<serde_json::Value> {
    crate::to_js_value(crate::project_store_error_envelope(command, error))
}

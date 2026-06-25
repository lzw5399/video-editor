use napi::bindgen_prelude::Result;

pub(crate) use editor_runtime::project_session_node::{
    ProjectSessionArtifactSnapshot, ProjectSessionPreviewSnapshot,
};

pub fn create_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    runtime_value(editor_runtime::project_session_node::create_project_session(request))
}

pub fn open_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    runtime_value(editor_runtime::project_session_node::open_project_session(
        request,
    ))
}

pub fn close_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    runtime_value(editor_runtime::project_session_node::close_project_session(
        request,
    ))
}

pub fn execute_project_intent(request: serde_json::Value) -> Result<serde_json::Value> {
    runtime_value(editor_runtime::project_session_node::execute_project_intent(request))
}

pub fn begin_project_interaction(request: serde_json::Value) -> Result<serde_json::Value> {
    runtime_value(editor_runtime::project_session_node::begin_project_interaction(request))
}

pub fn update_project_interaction(request: serde_json::Value) -> Result<serde_json::Value> {
    runtime_value(editor_runtime::project_session_node::update_project_interaction(request))
}

pub fn commit_project_interaction(request: serde_json::Value) -> Result<serde_json::Value> {
    runtime_value(editor_runtime::project_session_node::commit_project_interaction(request))
}

pub fn cancel_project_interaction(request: serde_json::Value) -> Result<serde_json::Value> {
    runtime_value(editor_runtime::project_session_node::cancel_project_interaction(request))
}

pub fn import_kaipai_formula_bundle(request: serde_json::Value) -> Result<serde_json::Value> {
    runtime_value(editor_runtime::project_session_node::import_kaipai_formula_bundle(request))
}

pub fn list_project_session_materials(request: serde_json::Value) -> Result<serde_json::Value> {
    runtime_value(editor_runtime::project_session_node::list_project_session_materials(request))
}

pub fn list_project_session_missing_materials(
    request: serde_json::Value,
) -> Result<serde_json::Value> {
    runtime_value(
        editor_runtime::project_session_node::list_project_session_missing_materials(request),
    )
}

pub(crate) fn realtime_preview_snapshot(
    session_id: &str,
    expected_revision: u64,
    interaction_id: Option<&str>,
) -> std::result::Result<ProjectSessionPreviewSnapshot, String> {
    editor_runtime::project_session_node::realtime_preview_snapshot(
        session_id,
        expected_revision,
        interaction_id,
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn project_session_snapshot(
    session_id: &str,
    expected_revision: u64,
) -> std::result::Result<ProjectSessionPreviewSnapshot, String> {
    editor_runtime::project_session_node::project_session_snapshot(session_id, expected_revision)
        .map_err(|error| error.to_string())
}

pub(crate) fn project_session_artifact_snapshot(
    session_id: &str,
    bundle_path: &std::path::Path,
) -> std::result::Result<Option<ProjectSessionArtifactSnapshot>, String> {
    editor_runtime::project_session_node::project_session_artifact_snapshot(session_id, bundle_path)
        .map_err(|error| error.to_string())
}

pub(crate) fn project_session_current_revision(session_id: &str) -> Option<u64> {
    editor_runtime::project_session_node::project_session_current_revision(session_id)
}

pub(crate) fn record_project_session_task_runtime_telemetry_snapshots() {
    editor_runtime::project_session_node::record_project_session_task_runtime_telemetry_snapshots();
}

fn runtime_value(
    value: std::result::Result<serde_json::Value, editor_runtime::RuntimeError>,
) -> Result<serde_json::Value> {
    value.map_err(|error| napi::Error::from_reason(error.to_string()))
}

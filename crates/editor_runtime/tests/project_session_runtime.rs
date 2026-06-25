use std::time::{SystemTime, UNIX_EPOCH};

use draft_model::{Draft, ExportPreset};
use editor_runtime::{
    ExportService, ProjectSessionService, RuntimeSessionConfig, RuntimeSessionRegistry,
    StartProjectSessionExportRequest,
};
use project_store::{StdPlatformFileSystem, create_project_bundle, project_json_path};
use task_runtime::{JobDomain, ResourceClass};

#[test]
fn creating_runtime_session_returns_opaque_generation_one_without_adapter_metadata() {
    let mut registry = RuntimeSessionRegistry::default();

    let session = registry
        .create_session(RuntimeSessionConfig::default())
        .expect("runtime session should be created");

    assert_eq!(session.id.generation(), 1);
    assert!(session.id.as_str().starts_with("runtime-"));
    assert!(
        session.adapter_metadata.is_none(),
        "shared runtime sessions must not carry Node/Electron/C adapter metadata"
    );
}

#[test]
fn opening_project_session_records_project_store_bundle_paths() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("runtime-draft.veproj");
    let draft = Draft::new("draft-runtime-001", "Runtime draft");
    create_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
        .expect("project bundle should be created through project_store");

    let mut runtime_sessions = RuntimeSessionRegistry::default();
    let runtime = runtime_sessions
        .create_session(RuntimeSessionConfig::default())
        .expect("runtime session should be created");
    let mut project_sessions = ProjectSessionService::default();

    let opened = project_sessions
        .open_project_session(runtime.id.clone(), &bundle_path)
        .expect("project session should open through project_store");

    assert_eq!(opened.handle.owner_session(), &runtime.id);
    assert_eq!(opened.bundle_path, bundle_path);
    assert_eq!(opened.project_json_path, project_json_path(&opened.bundle_path));
    assert_eq!(opened.draft_id, draft.draft_id);
    assert_eq!(opened.draft_name, draft.metadata.name);
}

#[test]
fn export_service_returns_shared_job_contract_without_adapter_code() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("export-draft.veproj");
    let draft = Draft::new("draft-export-001", "Export draft");
    create_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
        .expect("project bundle should be created through project_store");

    let mut runtime_sessions = RuntimeSessionRegistry::default();
    let runtime = runtime_sessions
        .create_session(RuntimeSessionConfig::default())
        .expect("runtime session should be created");
    let mut project_sessions = ProjectSessionService::default();
    let opened = project_sessions
        .open_project_session(runtime.id.clone(), &bundle_path)
        .expect("project session should open through project_store");

    let mut exports = ExportService::default();
    let requested_at_us = now_us();
    let export = exports
        .start_project_session_export(StartProjectSessionExportRequest {
            project_session: opened.handle.clone(),
            output_path: temp_dir.path().join("out.mp4"),
            preset: ExportPreset::H264AacBalanced,
            requested_at_us,
        })
        .expect("export job contract should be created");

    assert_eq!(export.project_session, opened.handle);
    assert_eq!(export.scheduler_envelope.domain, JobDomain::Export);
    assert_eq!(
        export.scheduler_envelope.resource_class,
        ResourceClass::FfmpegProcess
    );
    assert_eq!(export.scheduler_envelope.submitted_at_us, requested_at_us);
    assert!(
        export.job_id.as_str().starts_with("export-"),
        "export service should create shared runtime job ids, not adapter ids"
    );
}

fn now_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_micros()
        .try_into()
        .unwrap_or(u64::MAX)
}

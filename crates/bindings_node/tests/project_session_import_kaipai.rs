use artifact_store::schema::open_artifact_store;
use bindings_node::{close_project_session, create_project_session, import_kaipai_formula_bundle};
use project_store::{StdPlatformFileSystem, open_project_bundle};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn project_session_import_kaipai_formula_bundle_applies_atomically_and_indexes_materials() {
    let case = ImportCase::new("success-main-video");
    let source_root = case.seed_fixture_resources("positive/main-video.json");
    create_session(&case, "kaipai-import-success-main");

    let imported = import_kaipai_formula_bundle(json!({
        "sessionId": "kaipai-import-success-main",
        "expectedRevision": 0,
        "bundlePath": fixture_path("positive/main-video.json"),
        "resourceRoot": source_root,
        "importId": "import-main-video",
        "generatedAt": "2026-06-24T00:00:00Z",
        "verifyResourceSha256": false
    }))
    .expect("importKaipaiFormulaBundle should return an envelope");

    assert_eq!(imported["ok"], true, "{imported:#}");
    assert_eq!(imported["data"]["sessionId"], "kaipai-import-success-main");
    assert_eq!(imported["data"]["revision"], 1);
    assert_eq!(imported["data"]["viewModel"]["project"]["materialCount"], 1);
    assert_eq!(
        imported["data"]["adaptationReport"]["sourceKind"],
        "kaipaiOfflineBundle"
    );
    assert!(
        imported["data"]["adaptationReport"]["summary"]["supported"]
            .as_u64()
            .unwrap()
            > 0,
        "successful import should return supported adaptation report entries: {imported:#}"
    );

    let reopened = open_project_bundle(&StdPlatformFileSystem, &case.bundle_path)
        .expect("successful import should save canonical project.json");
    assert_eq!(
        reopened.bundle.draft.metadata.name,
        "导入模板 import-main-video"
    );
    assert_eq!(reopened.bundle.draft.materials.len(), 1);
    assert_eq!(reopened.bundle.draft.tracks.len(), 1);
    assert_eq!(case.saved_project_json(), case.saved_project_json());
    assert_project_json_is_canonical(&case.saved_project_json());

    let rows = resource_rows(&case.bundle_path);
    assert!(
        rows.iter().any(|row| {
            row.kind == "material"
                && row.stable_key == "template-import:material:material-main-video"
                && row.project_relative_ref.as_deref().is_some_and(|value| {
                    value.starts_with("resources/template-import/import-main-video/")
                })
                && row.status == "ready"
        }),
        "successful import should persist localized material resource index rows: {rows:#?}"
    );

    close_session("kaipai-import-success-main");
}

#[test]
fn project_session_import_kaipai_formula_bundle_indexes_localized_font_resources() {
    let case = ImportCase::new("success-text-sticker");
    let source_root = case.seed_fixture_resources("positive/text-sticker.json");
    create_session(&case, "kaipai-import-success-text");

    let imported = import_kaipai_formula_bundle(json!({
        "sessionId": "kaipai-import-success-text",
        "expectedRevision": 0,
        "bundlePath": fixture_path("positive/text-sticker.json"),
        "resourceRoot": source_root,
        "importId": "import-text-sticker",
        "generatedAt": "2026-06-24T00:00:00Z",
        "verifyResourceSha256": false
    }))
    .expect("importKaipaiFormulaBundle should return an envelope");

    assert_eq!(imported["ok"], true, "{imported:#}");
    assert_eq!(imported["data"]["revision"], 1);
    assert_eq!(
        imported["data"]["adaptationReport"]["summary"]["approximated"],
        1
    );
    assert!(
        imported["data"]["viewModel"]["timeline"]["rows"][0]["segments"][0]
            .get("segment")
            .is_none(),
        "template import response must keep using opaque session view models: {imported:#}"
    );

    let rows = resource_rows(&case.bundle_path);
    assert!(
        rows.iter().any(|row| {
            row.kind == "font"
                && row.stable_key == "template-import:font:font-redacted-noto"
                && row.project_relative_ref.as_deref().is_some_and(|value| {
                    value.starts_with("resources/template-import/import-text-sticker/")
                })
                && row.status == "ready"
        }),
        "successful import should persist localized font resource index rows: {rows:#?}"
    );
    assert_project_json_is_canonical(&case.saved_project_json());

    close_session("kaipai-import-success-text");
}

#[test]
fn project_session_import_kaipai_formula_bundle_rejects_stale_revision_without_writing() {
    let case = ImportCase::new("stale");
    let source_root = case.seed_fixture_resources("positive/main-video.json");
    create_session(&case, "kaipai-import-stale");
    let before = case.saved_project_json();

    let stale = import_kaipai_formula_bundle(json!({
        "sessionId": "kaipai-import-stale",
        "expectedRevision": 1,
        "bundlePath": fixture_path("positive/main-video.json"),
        "resourceRoot": source_root,
        "importId": "import-stale",
        "verifyResourceSha256": false
    }))
    .expect("stale import should return an envelope");

    assert_eq!(stale["ok"], false, "{stale:#}");
    assert_eq!(stale["error"]["kind"], "invalidPayload");
    assert!(
        stale["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Stale project session revision"),
        "stale error should explain revision mismatch: {stale:#}"
    );
    assert_eq!(
        case.saved_project_json(),
        before,
        "stale import must not mutate project.json"
    );
    assert!(
        resource_rows(&case.bundle_path).is_empty(),
        "stale import must not persist partial resource index rows"
    );

    close_session("kaipai-import-stale");
}

#[test]
fn project_session_import_kaipai_formula_bundle_rejects_unknown_session() {
    let case = ImportCase::new("unknown-session");
    let source_root = case.seed_fixture_resources("positive/main-video.json");

    let missing = import_kaipai_formula_bundle(json!({
        "sessionId": "kaipai-import-missing-session",
        "expectedRevision": 0,
        "bundlePath": fixture_path("positive/main-video.json"),
        "resourceRoot": source_root,
        "importId": "import-missing-session",
        "verifyResourceSha256": false
    }))
    .expect("missing session import should return an envelope");

    assert_eq!(missing["ok"], false, "{missing:#}");
    assert_eq!(missing["error"]["kind"], "invalidProject");
    assert!(
        missing["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Project session not found"),
        "missing session error should identify the session boundary: {missing:#}"
    );
}

#[test]
fn project_session_import_kaipai_formula_bundle_failed_mapping_leaves_no_partial_index() {
    let case = ImportCase::new("missing-resource-root");
    create_session(&case, "kaipai-import-missing-root");
    let before = case.saved_project_json();

    let failed = import_kaipai_formula_bundle(json!({
        "sessionId": "kaipai-import-missing-root",
        "expectedRevision": 0,
        "bundlePath": fixture_path("positive/main-video.json"),
        "resourceRoot": case.root.join("does-not-exist"),
        "importId": "import-missing-root",
        "verifyResourceSha256": false
    }))
    .expect("failed import should return an envelope");

    assert_eq!(failed["ok"], false, "{failed:#}");
    assert_eq!(failed["error"]["kind"], "invalidPayload");
    assert_eq!(
        case.saved_project_json(),
        before,
        "failed import must not mutate project.json"
    );
    assert!(
        resource_rows(&case.bundle_path).is_empty(),
        "failed import must not persist partial resource index rows"
    );

    close_session("kaipai-import-missing-root");
}

#[derive(Debug)]
struct ResourceRow {
    kind: String,
    stable_key: String,
    project_relative_ref: Option<String>,
    status: String,
}

struct ImportCase {
    root: PathBuf,
    bundle_path: PathBuf,
}

impl ImportCase {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "video-editor-project-session-import-kaipai-{name}-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("stale import test directory should be removable");
        }
        fs::create_dir_all(&root).expect("import test root should create");
        Self {
            bundle_path: root.join("imported.veproj"),
            root,
        }
    }

    fn seed_fixture_resources(&self, fixture: &str) -> PathBuf {
        let source_root = self.root.join("source");
        fs::create_dir_all(&source_root).expect("source root should create");
        let value: Value = serde_json::from_str(
            &fs::read_to_string(fixture_path(fixture)).expect("fixture should be readable"),
        )
        .expect("fixture should parse");
        for resource in value["resources"].as_array().into_iter().flatten() {
            let uri = resource["uri"]
                .as_str()
                .expect("fixture resource should have uri");
            let resource_id = resource["resourceId"]
                .as_str()
                .expect("fixture resource should have resourceId");
            let path = source_root.join(uri);
            fs::create_dir_all(path.parent().expect("resource path should have parent"))
                .expect("resource directory should create");
            fs::write(
                &path,
                format!("project session import fixture {resource_id}"),
            )
            .expect("resource fixture should write");
        }
        source_root
    }

    fn saved_project_json(&self) -> String {
        fs::read_to_string(self.bundle_path.join("project.json"))
            .expect("project.json should be readable")
    }
}

fn create_session(case: &ImportCase, session_id: &str) {
    let created = create_project_session(json!({
        "bundlePath": case.bundle_path.display().to_string(),
        "sessionId": session_id,
        "draftId": format!("{session_id}-draft"),
        "draftName": "Before Kaipai Import"
    }))
    .expect("createProjectSession should return an envelope");
    assert_eq!(created["ok"], true, "{created:#}");
    assert_eq!(created["data"]["revision"], 0);
}

fn close_session(session_id: &str) {
    close_project_session(json!({ "sessionId": session_id }))
        .expect("closeProjectSession should return an envelope");
}

fn fixture_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/kaipai")
        .join(path)
}

fn resource_rows(bundle_path: &Path) -> Vec<ResourceRow> {
    let store = open_artifact_store(bundle_path).expect("artifact store should open");
    let mut statement = store
        .connection()
        .prepare(
            "SELECT resource_kind, stable_key, project_relative_ref, status
             FROM resource
             ORDER BY stable_key",
        )
        .expect("resource query should prepare");
    statement
        .query_map([], |row| {
            Ok(ResourceRow {
                kind: row.get(0)?,
                stable_key: row.get(1)?,
                project_relative_ref: row.get(2)?,
                status: row.get(3)?,
            })
        })
        .expect("resource query should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("resource rows should collect")
}

fn assert_project_json_is_canonical(project_json: &str) {
    let value: Value = serde_json::from_str(project_json).expect("project JSON should parse");
    assert!(
        value.get("materials").is_some(),
        "project JSON should be a Draft"
    );
    let serialized = serde_json::to_string(&value).expect("project JSON should serialize");
    for forbidden in [
        "templateId",
        "recipeId",
        "formulaTaskId",
        "formulaRequestId",
        "rawFormula",
        "\"formula\"",
        "safeArea",
        "remoteRuntimeUrl",
        "remoteRenderUrl",
        "renderUrl",
        "http://",
        "https://",
        "kaipai",
        "provider",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "project.json leaked provider/runtime evidence {forbidden}: {serialized}"
        );
    }
}

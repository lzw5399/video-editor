use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use draft_model::{Draft, DraftValidationError, MaterialStatus, migrate_draft_json};

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("draft_model should live under crates/")
        .to_path_buf()
}

#[test]
fn draft_fixtures_are_explicitly_classified() {
    let root = project_root();
    let fixture_dir = root.join("fixtures/draft");
    let positive = positive_project_fixtures();
    let negative = negative_project_fixtures();

    let actual = project_fixture_paths(&fixture_dir);
    let expected = positive
        .iter()
        .copied()
        .chain(negative.iter().map(|(path, _)| *path))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "every .veproj-style project.json fixture must be explicitly classified"
    );
}

#[test]
fn positive_draft_fixtures_validate_through_model_and_schema() {
    let root = project_root();
    let fixture_dir = root.join("fixtures/draft");
    let schema = draft_schema_validator();

    for fixture_path in positive_project_fixtures() {
        let value = read_project_fixture(&fixture_dir, fixture_path);
        let draft = migrate_draft_json(value.clone())
            .expect("positive fixture should migrate and validate");
        schema
            .validate(&value)
            .expect("positive fixture should validate against generated draft JSON Schema");

        assert_round_trips(&draft, fixture_path);
    }
}

#[test]
fn draft_fixtures_preserve_missing_material_recoverable_status() {
    let root = project_root();
    let fixture_dir = root.join("fixtures/draft");
    let value = read_project_fixture(&fixture_dir, "positive/missing-material/project.json");
    let draft = migrate_draft_json(value).expect("missing material fixture should load");

    assert_eq!(draft.materials.len(), 1);
    let material = &draft.materials[0];
    assert_eq!(material.material_id.as_str(), "material-missing-001");
    assert_eq!(material.status, MaterialStatus::Missing);
    assert_eq!(material.uri, "media/offline.mov");
    assert_eq!(
        material.metadata.probe_error.as_deref(),
        Some("material path is missing: media/offline.mov")
    );
}

#[test]
fn negative_draft_fixtures_fail_expected_gates() {
    let root = project_root();
    let fixture_dir = root.join("fixtures/draft");
    let schema = draft_schema_validator();

    for (fixture_path, expected_error) in negative_project_fixtures() {
        let value = read_project_fixture(&fixture_dir, fixture_path);
        let error = migrate_draft_json(value.clone())
            .expect_err("negative fixture should fail draft migration or validation");

        assert_eq!(error, expected_error, "{fixture_path}");

        assert!(
            schema.validate(&value).is_err(),
            "negative fixture should fail generated draft JSON Schema: {fixture_path}"
        );
    }
}

fn positive_project_fixtures() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "positive/minimal-draft/project.json",
        "positive/materials-round-trip/project.json",
        "positive/missing-material/project.json",
    ])
}

fn negative_project_fixtures() -> Vec<(&'static str, DraftValidationError)> {
    vec![
        (
            "negative/invalid-unknown-field/project.json",
            DraftValidationError::InvalidDraftJson {
                message: "unknown field `unexpectedField`, expected one of `schemaVersion`, `draftId`, `metadata`, `materials`, `tracks`".to_owned(),
            },
        ),
        (
            "negative/invalid-schema-version/project.json",
            DraftValidationError::InvalidSchemaVersion {
                found: "2".to_owned(),
                expected: 1,
            },
        ),
        (
            "negative/derived-artifact-in-project-json/project.json",
            DraftValidationError::DerivedArtifactLeakage {
                field: "renderGraph".to_owned(),
            },
        ),
    ]
}

fn project_fixture_paths(fixture_dir: &Path) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    collect_project_fixtures(fixture_dir, fixture_dir, &mut paths);
    paths
}

fn collect_project_fixtures(root: &Path, dir: &Path, paths: &mut BTreeSet<String>) {
    for entry in fs::read_dir(dir).expect("fixture directory should be readable") {
        let entry = entry.expect("fixture directory entry should be readable");
        let path = entry.path();

        if path.is_dir() {
            collect_project_fixtures(root, &path, paths);
            continue;
        }

        if path.file_name().and_then(|name| name.to_str()) != Some("project.json") {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .expect("project fixture should live under fixture root")
            .to_string_lossy()
            .replace('\\', "/");
        paths.insert(relative);
    }
}

fn read_project_fixture(fixture_dir: &Path, fixture_path: &str) -> serde_json::Value {
    serde_json::from_slice(
        &fs::read(fixture_dir.join(fixture_path)).expect("project fixture should be readable"),
    )
    .expect("project fixture should parse as JSON")
}

fn draft_schema_validator() -> jsonschema::Validator {
    let schema_path = project_root().join("schemas/draft.schema.json");
    let schema_json: serde_json::Value = serde_json::from_slice(
        &fs::read(&schema_path).expect("generated draft schema should be readable"),
    )
    .expect("generated draft schema should parse");
    jsonschema::validator_for(&schema_json).expect("generated draft schema should compile")
}

fn assert_round_trips(draft: &Draft, fixture_path: &str) {
    let serialized = serde_json::to_value(draft).expect("fixture draft should serialize");
    let round_tripped: Draft =
        serde_json::from_value(serialized).expect("serialized fixture draft should deserialize");
    assert_eq!(
        round_tripped, *draft,
        "fixture should preserve semantic draft equality: {fixture_path}"
    );
}

use draft_model::{
    Draft, Material, MaterialKind, MaterialMetadata, MaterialStatus, Microseconds,
    RationalFrameRate, Segment, SourceTimerange, TargetTimerange, Track, TrackKind,
};
use project_store::{
    ProjectStoreError, ProjectStoreWarning, StdPlatformFileSystem, autosave_project_bundle,
    create_project_bundle, open_project_bundle, project_json_path, save_project_bundle,
};
use serde_json::json;

#[test]
fn create_project_bundle_writes_valid_project_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("created.veproj");
    let draft = Draft::new("draft-001", "Created draft");

    let bundle = create_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
        .expect("bundle should be created");
    let opened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("created bundle should open");

    assert_eq!(bundle.project_json_path, project_json_path(&bundle_path));
    assert_eq!(opened.bundle.draft, draft);
    assert!(opened.warnings.is_empty());
}

#[test]
fn round_trip_save_open_preserves_semantic_draft_equality() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("round-trip.veproj");
    let draft = populated_draft("media/missing-video.mp4");

    save_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft).expect("draft should save");
    let opened =
        open_project_bundle(&StdPlatformFileSystem, &bundle_path).expect("saved draft should open");

    assert_eq!(opened.bundle.draft, draft);
    assert_eq!(opened.warnings.len(), 1);
}

#[test]
fn round_trip_autosave_preserves_semantic_draft_equality() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("autosave.veproj");
    let mut draft = Draft::new("draft-001", "Before autosave");

    create_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
        .expect("initial draft should save");
    draft.metadata.name = "After autosave".to_owned();
    autosave_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
        .expect("autosave should write updated semantic draft");
    let opened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("autosaved draft should open");

    assert_eq!(opened.bundle.draft, draft);
}

#[test]
fn save_project_bundle_preserves_existing_project_json_when_temp_write_fails() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("atomic-save.veproj");
    let original = Draft::new("draft-original", "Original draft");
    let replacement = Draft::new("draft-replacement", "Replacement draft");

    save_project_bundle(&StdPlatformFileSystem, &bundle_path, &original)
        .expect("original draft should save");
    std::fs::create_dir(bundle_path.join(".project.json.tmp"))
        .expect("temp path directory should force temp write failure");

    let error = save_project_bundle(&StdPlatformFileSystem, &bundle_path, &replacement)
        .expect_err("temp write failure should fail save");

    assert!(
        matches!(error, ProjectStoreError::Io { .. }),
        "unexpected error: {error}"
    );
    let opened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("original project should remain readable after failed replacement");
    assert_eq!(opened.bundle.draft, original);
}

#[test]
fn save_project_bundle_rejects_invalid_material_uri_before_replacing_project_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("invalid-material-uri.veproj");
    let original = populated_draft("media/original.mp4");
    let invalid = populated_draft("../outside.mp4");

    save_project_bundle(&StdPlatformFileSystem, &bundle_path, &original)
        .expect("original draft should save");
    let error = save_project_bundle(&StdPlatformFileSystem, &bundle_path, &invalid)
        .expect_err("invalid material URI should fail before save");

    assert!(
        matches!(error, ProjectStoreError::InvalidMaterialUri { .. }),
        "unexpected error: {error}"
    );
    let opened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("original project should remain readable after rejected save");
    assert_eq!(opened.bundle.draft, original);
}

#[test]
fn open_project_bundle_rejects_malformed_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("malformed.veproj");
    std::fs::create_dir_all(&bundle_path).expect("bundle dir should be created");
    std::fs::write(project_json_path(&bundle_path), "{not valid json")
        .expect("malformed project should be written");

    let error = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect_err("malformed JSON should fail");

    assert!(matches!(
        error,
        ProjectStoreError::InvalidProjectJson { .. }
    ));
}

#[test]
fn open_project_bundle_rejects_unknown_fields() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("unknown-field.veproj");
    write_project_json(
        &bundle_path,
        json!({
            "schemaVersion": 1,
            "draftId": "draft-001",
            "metadata": { "name": "Unknown field" },
            "materials": [],
            "tracks": [],
            "unexpected": true
        }),
    );

    let error = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect_err("unknown fields should fail");

    assert!(
        matches!(error, ProjectStoreError::SemanticValidation { .. }),
        "unexpected error: {error}"
    );
}

#[test]
fn open_project_bundle_rejects_unsupported_schema_versions() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("future-version.veproj");
    let mut value = serde_json::to_value(Draft::new("draft-001", "Future version"))
        .expect("draft should serialize");
    value["schemaVersion"] = json!(2);
    write_project_json(&bundle_path, value);

    let error = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect_err("unsupported schema version should fail");

    assert!(matches!(
        error,
        ProjectStoreError::UnsupportedSchemaVersion { ref found, .. } if found == "2"
    ));
}

#[test]
fn open_project_bundle_rejects_derived_artifact_fields() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("derived-artifacts.veproj");
    let mut value = serde_json::to_value(Draft::new("draft-001", "Derived artifact leak"))
        .expect("draft should serialize");
    value["previewCaches"] = json!([]);
    write_project_json(&bundle_path, value);

    let error = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect_err("derived artifact field should fail");

    assert!(
        matches!(error, ProjectStoreError::SemanticValidation { .. }),
        "unexpected error: {error}"
    );
}

#[test]
fn open_project_bundle_preserves_missing_material_entries() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("missing-material.veproj");
    let draft = populated_draft("media/missing-video.mp4");

    save_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
        .expect("draft with missing material should save");
    let opened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("missing material path should not block open");

    assert_eq!(opened.bundle.draft, draft);
    assert_eq!(
        opened.warnings,
        vec![ProjectStoreWarning::MissingMaterial {
            material_id: "material-video-001".to_owned(),
            uri: "media/missing-video.mp4".to_owned(),
            resolved_path: Some(bundle_path.join("media/missing-video.mp4")),
        }]
    );
}

fn populated_draft(material_uri: &str) -> Draft {
    let material = Material {
        material_id: "material-video-001".into(),
        kind: MaterialKind::Video,
        uri: material_uri.to_owned(),
        display_name: "missing-video.mp4".to_owned(),
        metadata: MaterialMetadata {
            duration: Some(Microseconds::new(1_500_000)),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(RationalFrameRate::new(30_000, 1_001)),
            has_video: true,
            has_audio: true,
            audio_sample_rate: Some(48_000),
            audio_channels: Some(2),
            probe_error: None,
        },
        status: MaterialStatus::Missing,
    };

    let segment = Segment::new(
        "segment-001",
        material.material_id.clone(),
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    );
    let mut track = Track::new("track-video-001", TrackKind::Video, "Video 1");
    track.segments.push(segment);

    let mut draft = Draft::new("draft-001", "Populated draft");
    draft.materials.push(material);
    draft.tracks.push(track);
    draft
}

fn write_project_json(bundle_path: &std::path::Path, value: serde_json::Value) {
    std::fs::create_dir_all(bundle_path).expect("bundle dir should be created");
    std::fs::write(
        project_json_path(bundle_path),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&value).expect("project JSON should serialize")
        ),
    )
    .expect("project JSON should be written");
}

use std::path::Path;

use bindings_node::material_service::{
    ImportMaterialRequest, MissingMaterialDiagnosticKind, import_material_and_save, list_materials,
    list_missing_materials,
};
use draft_model::{Draft, MaterialKind, MaterialStatus, Microseconds};
use media_runtime::{MaterialProbeKind, discover_runtime_config};
use media_runtime_desktop::DesktopFfmpegExecutor;
use project_store::{StdPlatformFileSystem, open_project_bundle};
use testkit::{
    generate_audio_material_fixture, generate_image_material_fixture,
    generate_video_material_fixture,
};

#[test]
fn material_service_imports_video_image_and_audio_materials() {
    let runtime = discover_runtime_config().expect("ffmpeg runtime should be discoverable");
    let executor = DesktopFfmpegExecutor::default();
    let fs = StdPlatformFileSystem;
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("imports.veproj");
    let mut draft = Draft::new("draft-001", "Imports");

    let video = generate_video_material_fixture(&executor, &runtime)
        .expect("video fixture should be generated");
    let image = generate_image_material_fixture(&executor, &runtime)
        .expect("image fixture should be generated");
    let audio = generate_audio_material_fixture(&executor, &runtime)
        .expect("audio fixture should be generated");

    let video_result = import_material_and_save(
        &mut draft,
        ImportMaterialRequest::new(video.path())
            .with_material_id("material-video")
            .with_display_name("video.mp4"),
        &fs,
        &executor,
        &runtime,
        &bundle_path,
    )
    .expect("video should import");
    let image_result = import_material_and_save(
        &mut draft,
        ImportMaterialRequest::new(image.path())
            .with_material_id("material-image")
            .with_display_name("image.png"),
        &fs,
        &executor,
        &runtime,
        &bundle_path,
    )
    .expect("image should import");
    let audio_result = import_material_and_save(
        &mut draft,
        ImportMaterialRequest::new(audio.path())
            .with_material_id("material-audio")
            .with_display_name("audio.wav"),
        &fs,
        &executor,
        &runtime,
        &bundle_path,
    )
    .expect("audio should import");

    assert_eq!(video_result.material.kind, MaterialKind::Video);
    assert_eq!(image_result.material.kind, MaterialKind::Image);
    assert_eq!(audio_result.material.kind, MaterialKind::Audio);
    assert_eq!(video_result.material.status, MaterialStatus::Available);
    assert_eq!(image_result.material.status, MaterialStatus::Available);
    assert_eq!(audio_result.material.status, MaterialStatus::Available);
    assert_eq!(
        video_result.material.metadata.duration,
        Some(Microseconds::new(1_000_000))
    );
    assert_eq!(image_result.material.metadata.duration, None);
    assert_eq!(
        audio_result.material.metadata.audio_sample_rate,
        Some(44_100)
    );
    assert!(video_result.diagnostic.is_none());
    assert!(image_result.diagnostic.is_none());
    assert!(audio_result.diagnostic.is_none());

    video
        .expected()
        .assert_probe_metadata(
            &media_runtime::probe_material_metadata(&executor, &runtime, video.path())
                .expect("video should still probe"),
        )
        .expect("video probe metadata should match fixture");
    assert_eq!(
        image.expected().kind,
        MaterialProbeKind::Image,
        "image fixture should be a still image"
    );

    let material_ids = list_materials(&draft)
        .into_iter()
        .map(|material| material.material_id.as_str().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        material_ids,
        vec!["material-video", "material-image", "material-audio"]
    );

    let reopened = open_project_bundle(&fs, &bundle_path).expect("saved project should reopen");
    assert!(reopened.warnings.is_empty());
    assert_eq!(reopened.bundle.draft.materials, draft.materials);
    assert!(
        list_missing_materials(&reopened.bundle.draft, &fs, &bundle_path)
            .expect("missing diagnostics should list")
            .is_empty()
    );
}

#[test]
fn material_service_preserves_missing_materials_and_reports_diagnostics() {
    let runtime = discover_runtime_config().expect("ffmpeg runtime should be discoverable");
    let executor = DesktopFfmpegExecutor::default();
    let fs = StdPlatformFileSystem;
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("missing.veproj");
    let missing_path = bundle_path.join("media").join("missing.mp4");
    let mut draft = Draft::new("draft-001", "Missing media");

    let imported = import_material_and_save(
        &mut draft,
        ImportMaterialRequest::new(&missing_path)
            .with_material_id("material-missing")
            .with_display_name("missing.mp4")
            .with_material_kind_hint(MaterialKind::Video),
        &fs,
        &executor,
        &runtime,
        &bundle_path,
    )
    .expect("missing import should be recoverable");

    assert_eq!(imported.material.status, MaterialStatus::Missing);
    assert_eq!(imported.material.uri, "media/missing.mp4");
    assert_eq!(
        imported
            .diagnostic
            .as_ref()
            .expect("missing import should include diagnostic")
            .kind,
        MissingMaterialDiagnosticKind::MissingFile
    );

    let reopened = open_project_bundle(&fs, &bundle_path).expect("missing project should reopen");
    assert_eq!(reopened.warnings.len(), 1);
    assert_eq!(reopened.bundle.draft.materials.len(), 1);
    assert_eq!(
        reopened.bundle.draft.materials[0], draft.materials[0],
        "open/save must preserve missing material entry exactly"
    );

    let diagnostics = list_missing_materials(&reopened.bundle.draft, &fs, &bundle_path)
        .expect("missing diagnostics should be returned");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].kind,
        MissingMaterialDiagnosticKind::MissingFile
    );
    assert_eq!(diagnostics[0].original_uri, "media/missing.mp4");
    assert_eq!(
        diagnostics[0].last_known_resolved_path.as_deref(),
        Some(Path::new(&missing_path))
    );
    assert_eq!(diagnostics[0].status, MaterialStatus::Missing);
}

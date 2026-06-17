use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use adapter_kaipai::{
    CompatibilityStatus, FormulaResourceRef, KaipaiFormulaBundle, LocalizedResourceStatus,
    ResourceKind, ResourceLocalizationMode, ResourceLocalizationRequest, ResourceLocalizer,
};
use serde_json::{Value, json};

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("adapter_kaipai should live under crates/")
        .to_path_buf()
}

#[test]
fn resource_localizer_copies_local_assets_to_bundle_relative_resources() {
    let bundle = read_bundle_fixture("positive/resource-bundle-with-local-assets.json");
    let temp = temp_case_dir("positive");
    let source_root = temp.join("formula-bundle");
    let bundle_path = temp.join("localized.veproj");
    seed_local_assets(&source_root, &bundle);
    fs::create_dir_all(&bundle_path).expect("project bundle dir should create");

    let result = ResourceLocalizer::default()
        .localize(ResourceLocalizationRequest {
            bundle_path: bundle_path.clone(),
            source_root,
            resources: bundle.resources.clone(),
            mode: ResourceLocalizationMode::CopyRenderableResources,
        })
        .expect("local resources should localize");

    assert!(result.diagnostics.is_empty());
    assert_eq!(result.manifest.resources.len(), 3);
    assert!(result.manifest.resources.iter().all(|resource| {
        resource.status == LocalizedResourceStatus::Available
            && resource
                .bundle_relative_uri
                .as_deref()
                .is_some_and(|uri| uri.starts_with("resources/"))
    }));
    assert!(
        bundle_path
            .join("resources/fonts/redacted-font.ttf")
            .exists()
    );
    assert!(
        bundle_path
            .join("resources/stickers/redacted-sticker.png")
            .exists()
    );
    assert!(
        bundle_path
            .join("resources/videos/redacted-pip.mp4")
            .exists()
    );
}

#[test]
fn resource_localizer_reports_missing_and_sha256_mismatch() {
    for (fixture_path, expected_status, expected_id) in [
        (
            "negative/missing-resource.json",
            LocalizedResourceStatus::Missing,
            "missing-sticker",
        ),
        (
            "negative/sha256-mismatch.json",
            LocalizedResourceStatus::Sha256Mismatch,
            "font-sha-mismatch",
        ),
    ] {
        let bundle = read_bundle_fixture(fixture_path);
        let temp = temp_case_dir(expected_id);
        let source_root = temp.join("formula-bundle");
        let bundle_path = temp.join("localized.veproj");
        fs::create_dir_all(&source_root).expect("source root should create");
        seed_local_assets(&source_root, &bundle);
        fs::create_dir_all(&bundle_path).expect("project bundle dir should create");

        let result = ResourceLocalizer::default()
            .localize(ResourceLocalizationRequest {
                bundle_path,
                source_root,
                resources: bundle.resources.clone(),
                mode: ResourceLocalizationMode::CopyRenderableResources,
            })
            .expect("diagnostic localization should not fail");

        assert_eq!(result.manifest.resources.len(), 1);
        assert_eq!(result.manifest.resources[0].status, expected_status);
        assert!(result.manifest.resources[0].bundle_relative_uri.is_none());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].status,
            CompatibilityStatus::MissingResource
        );
        assert_eq!(
            result.diagnostics[0].external_id.as_deref(),
            Some(expected_id)
        );
    }
}

#[test]
fn resource_localizer_rejects_traversal_and_remote_render_urls_without_writes() {
    let traversal = read_bundle_fixture("negative/path-traversal-resource.json");
    let remote = patch(
        read_bundle_fixture("positive/resource-bundle-with-local-assets.json"),
        |value| {
            value["resources"] = json!([
                {
                    "resourceId": "remote-template-video",
                    "kind": "video",
                    "uri": "https://example.invalid/render/video.mp4",
                    "displayName": "remote-template-video.mp4"
                }
            ]);
        },
    );
    let remote_bundle: KaipaiFormulaBundle = serde_json::from_value(remote)
        .expect("remote case bypasses bundle sanitizer for localizer");

    for (bundle, expected_status, expected_id) in [
        (
            traversal,
            LocalizedResourceStatus::UnsafePath,
            "unsafe-sticker",
        ),
        (
            remote_bundle,
            LocalizedResourceStatus::RemoteRenderUrl,
            "remote-template-video",
        ),
    ] {
        let temp = temp_case_dir(expected_id);
        let source_root = temp.join("formula-bundle");
        let bundle_path = temp.join("localized.veproj");
        fs::create_dir_all(&source_root).expect("source root should create");
        seed_local_assets(&source_root, &bundle);
        fs::create_dir_all(&bundle_path).expect("project bundle dir should create");

        let result = ResourceLocalizer::default()
            .localize(ResourceLocalizationRequest {
                bundle_path: bundle_path.clone(),
                source_root,
                resources: bundle.resources.clone(),
                mode: ResourceLocalizationMode::CopyRenderableResources,
            })
            .expect("unsafe resources should report diagnostics");

        assert_eq!(result.manifest.resources[0].status, expected_status);
        assert!(result.manifest.resources[0].bundle_relative_uri.is_none());
        assert_eq!(
            result.diagnostics[0].status,
            CompatibilityStatus::MissingResource
        );
        assert_eq!(
            result.diagnostics[0].external_id.as_deref(),
            Some(expected_id)
        );
        assert!(
            !bundle_path.join("resources/stickers/escape.png").exists(),
            "unsafe traversal output must not be created"
        );
    }
}

#[test]
fn resource_localizer_mode_makes_source_media_handling_explicit() {
    let bundle = read_bundle_fixture("positive/resource-bundle-with-local-assets.json");
    let pip = bundle
        .resources
        .iter()
        .find(|resource| resource.kind == ResourceKind::Video)
        .expect("positive fixture should include a PIP video resource")
        .clone();
    let temp = temp_case_dir("source-mode");
    let source_root = temp.join("formula-bundle");
    let bundle_path = temp.join("localized.veproj");
    seed_local_assets(&source_root, &bundle);
    fs::create_dir_all(&bundle_path).expect("project bundle dir should create");

    let result = ResourceLocalizer::default()
        .localize(ResourceLocalizationRequest {
            bundle_path: bundle_path.clone(),
            source_root,
            resources: vec![pip],
            mode: ResourceLocalizationMode::PreserveExternalSourceMedia,
        })
        .expect("preserve mode should validate without copying");

    assert_eq!(
        result.manifest.resources[0].status,
        LocalizedResourceStatus::Available
    );
    assert_eq!(
        result.manifest.resources[0].bundle_relative_uri.as_deref(),
        Some("resources/videos/redacted-pip.mp4")
    );
    assert!(
        !bundle_path
            .join("resources/videos/redacted-pip.mp4")
            .exists(),
        "preserve mode should not copy source media"
    );
}

#[test]
#[cfg(unix)]
fn resource_localizer_rejects_source_symlink_escape() {
    use std::os::unix::fs::symlink;

    let temp = temp_case_dir("source-symlink");
    let source_root = temp.join("formula-bundle");
    let bundle_path = temp.join("localized.veproj");
    let outside_path = temp.join("outside-secret.ttf");
    fs::create_dir_all(source_root.join("resources/fonts")).expect("source dir should create");
    fs::create_dir_all(&bundle_path).expect("bundle dir should create");
    fs::write(&outside_path, b"must-not-copy").expect("outside file should write");
    symlink(
        &outside_path,
        source_root.join("resources/fonts/leaked-font.ttf"),
    )
    .expect("source symlink should create");

    let result = ResourceLocalizer::default()
        .localize(ResourceLocalizationRequest {
            bundle_path: bundle_path.clone(),
            source_root,
            resources: vec![resource_ref(
                "font-symlink",
                ResourceKind::Font,
                "resources/fonts/leaked-font.ttf",
            )],
            mode: ResourceLocalizationMode::CopyRenderableResources,
        })
        .expect("symlink source should report diagnostic");

    assert_eq!(
        result.manifest.resources[0].status,
        LocalizedResourceStatus::UnsafePath
    );
    assert_eq!(result.diagnostics.len(), 1);
    assert!(
        !bundle_path.join("resources/fonts/leaked-font.ttf").exists(),
        "source symlink target must not be copied"
    );
}

#[test]
#[cfg(unix)]
fn resource_localizer_rejects_destination_symlink_escape() {
    use std::os::unix::fs::symlink;

    let temp = temp_case_dir("destination-symlink");
    let source_root = temp.join("formula-bundle");
    let bundle_path = temp.join("localized.veproj");
    let outside_dir = temp.join("outside-bundle");
    fs::create_dir_all(source_root.join("template-a")).expect("source dir should create");
    fs::create_dir_all(&bundle_path).expect("bundle dir should create");
    fs::create_dir_all(&outside_dir).expect("outside dir should create");
    fs::write(source_root.join("template-a/overlay.png"), b"overlay")
        .expect("source resource should write");
    symlink(&outside_dir, bundle_path.join("resources"))
        .expect("destination symlink should create");

    let result = ResourceLocalizer::default()
        .localize(ResourceLocalizationRequest {
            bundle_path,
            source_root,
            resources: vec![resource_ref(
                "overlay-symlink",
                ResourceKind::Image,
                "template-a/overlay.png",
            )],
            mode: ResourceLocalizationMode::CopyRenderableResources,
        })
        .expect("destination symlink should report diagnostic");

    assert_eq!(
        result.manifest.resources[0].status,
        LocalizedResourceStatus::UnsafePath
    );
    assert!(
        !outside_dir.join("images/overlay-symlink").exists(),
        "destination symlink must not receive copied resources"
    );
}

#[test]
fn resource_localizer_preserves_non_kind_paths_and_rejects_duplicate_destinations() {
    let temp = temp_case_dir("destination-collisions");
    let source_root = temp.join("formula-bundle");
    let bundle_path = temp.join("localized.veproj");
    fs::create_dir_all(source_root.join("template-a")).expect("source dir should create");
    fs::create_dir_all(source_root.join("template-b")).expect("source dir should create");
    fs::create_dir_all(&bundle_path).expect("bundle dir should create");
    fs::write(source_root.join("template-a/overlay.png"), b"overlay-a")
        .expect("source a should write");
    fs::write(source_root.join("template-b/overlay.png"), b"overlay-b")
        .expect("source b should write");

    let result = ResourceLocalizer::default()
        .localize(ResourceLocalizationRequest {
            bundle_path: bundle_path.clone(),
            source_root: source_root.clone(),
            resources: vec![
                resource_ref("overlay-a", ResourceKind::Image, "template-a/overlay.png"),
                resource_ref("overlay-b", ResourceKind::Image, "template-b/overlay.png"),
            ],
            mode: ResourceLocalizationMode::CopyRenderableResources,
        })
        .expect("same file names in different template dirs should localize");

    assert!(result.diagnostics.is_empty());
    assert_eq!(
        result.manifest.resources[0].bundle_relative_uri.as_deref(),
        Some("resources/images/overlay-a/template-a/overlay.png")
    );
    assert_eq!(
        result.manifest.resources[1].bundle_relative_uri.as_deref(),
        Some("resources/images/overlay-b/template-b/overlay.png")
    );
    assert_eq!(
        fs::read(bundle_path.join("resources/images/overlay-a/template-a/overlay.png"))
            .expect("overlay a should read"),
        b"overlay-a"
    );
    assert_eq!(
        fs::read(bundle_path.join("resources/images/overlay-b/template-b/overlay.png"))
            .expect("overlay b should read"),
        b"overlay-b"
    );

    let duplicate_bundle_path = temp.join("duplicate.veproj");
    fs::create_dir_all(&duplicate_bundle_path).expect("duplicate bundle dir should create");
    let duplicate = ResourceLocalizer::default()
        .localize(ResourceLocalizationRequest {
            bundle_path: duplicate_bundle_path,
            source_root,
            resources: vec![
                resource_ref("overlay-a", ResourceKind::Image, "template-a/overlay.png"),
                resource_ref("overlay-a", ResourceKind::Image, "template-a/overlay.png"),
            ],
            mode: ResourceLocalizationMode::CopyRenderableResources,
        })
        .expect("duplicate destination should report diagnostic");

    assert_eq!(
        duplicate.manifest.resources[0].status,
        LocalizedResourceStatus::Available
    );
    assert_eq!(
        duplicate.manifest.resources[1].status,
        LocalizedResourceStatus::UnsafePath
    );
    assert_eq!(duplicate.diagnostics.len(), 1);
}

fn read_bundle_fixture(path: &str) -> KaipaiFormulaBundle {
    let value: Value = serde_json::from_slice(
        &fs::read(project_root().join("fixtures/kaipai").join(path))
            .unwrap_or_else(|error| panic!("fixture should be readable: {path}: {error}")),
    )
    .unwrap_or_else(|error| panic!("fixture should parse as JSON: {path}: {error}"));
    KaipaiFormulaBundle::from_json_value(value)
        .unwrap_or_else(|error| panic!("fixture should validate: {path}: {error}"))
}

fn resource_ref(resource_id: &str, kind: ResourceKind, uri: &str) -> FormulaResourceRef {
    FormulaResourceRef {
        resource_id: resource_id.to_owned(),
        kind,
        uri: uri.to_owned(),
        sha256: None,
        display_name: None,
    }
}

fn seed_local_assets(source_root: &Path, bundle: &KaipaiFormulaBundle) {
    for resource in &bundle.resources {
        if resource.resource_id.starts_with("missing-") || resource.uri.contains("../") {
            continue;
        }
        let path = source_root.join(&resource.uri);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("asset parent should create");
        }
        fs::write(path, fixture_bytes_for_resource(&resource.resource_id))
            .expect("asset fixture should write");
    }
}

fn fixture_bytes_for_resource(resource_id: &str) -> &'static [u8] {
    match resource_id {
        "font-redacted" | "font-sha-mismatch" => b"local-font-fixture",
        "sticker-redacted" => b"local-sticker-fixture",
        "pip-redacted" => b"local-pip-fixture",
        _ => b"local-resource-fixture",
    }
}

fn patch(bundle: KaipaiFormulaBundle, update: impl FnOnce(&mut Value)) -> Value {
    let mut value = serde_json::to_value(&bundle).expect("bundle should serialize");
    update(&mut value);
    value
}

fn temp_case_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("adapter-kaipai-resource-{name}-{nonce}"));
    if path.exists() {
        fs::remove_dir_all(&path).expect("old temp dir should remove");
    }
    fs::create_dir_all(&path).expect("temp dir should create");
    path
}

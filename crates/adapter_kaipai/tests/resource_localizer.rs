use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use adapter_kaipai::{
    CompatibilityStatus, KaipaiFormulaBundle, LocalizedResourceStatus, ResourceKind,
    ResourceLocalizationMode, ResourceLocalizationRequest, ResourceLocalizer,
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

fn read_bundle_fixture(path: &str) -> KaipaiFormulaBundle {
    let value: Value = serde_json::from_slice(
        &fs::read(project_root().join("fixtures/kaipai").join(path))
            .unwrap_or_else(|error| panic!("fixture should be readable: {path}: {error}")),
    )
    .unwrap_or_else(|error| panic!("fixture should parse as JSON: {path}: {error}"));
    KaipaiFormulaBundle::from_json_value(value)
        .unwrap_or_else(|error| panic!("fixture should validate: {path}: {error}"))
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

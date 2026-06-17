use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use adapter_kaipai::{
    CompatibilityStatus, FormulaResourceRef, LocalizedResourceStatus, ResourceKind,
    ResourceLocalizationMode, ResourceLocalizationRequest, ResourceLocalizer,
};

#[test]
fn resource_localizer_contract_localizes_renderable_resources_under_resources_dir() {
    let temp = temp_case_dir("contract-local");
    let source_root = temp.join("formula-bundle");
    let bundle_path = temp.join("draft.veproj");
    fs::create_dir_all(source_root.join("resources/fonts")).expect("source dir should create");
    fs::create_dir_all(&bundle_path).expect("bundle dir should create");
    fs::write(
        source_root.join("resources/fonts/redacted-font.ttf"),
        b"local-font-fixture",
    )
    .expect("source resource should write");

    let result = ResourceLocalizer::default()
        .localize(ResourceLocalizationRequest {
            bundle_path: bundle_path.clone(),
            source_root,
            resources: vec![FormulaResourceRef {
                resource_id: "font-main".to_owned(),
                kind: ResourceKind::Font,
                uri: "resources/fonts/redacted-font.ttf".to_owned(),
                sha256: None,
                display_name: Some("redacted-font.ttf".to_owned()),
            }],
            mode: ResourceLocalizationMode::CopyRenderableResources,
        })
        .expect("localization should succeed");

    assert!(result.diagnostics.is_empty());
    assert_eq!(result.manifest.resources.len(), 1);
    assert_eq!(
        result.manifest.resources[0].status,
        LocalizedResourceStatus::Available
    );
    assert_eq!(
        result.manifest.resources[0].bundle_relative_uri.as_deref(),
        Some("resources/fonts/redacted-font.ttf")
    );
    assert!(bundle_path.join("resources/fonts/redacted-font.ttf").exists());
}

#[test]
fn resource_localizer_contract_reports_missing_and_remote_resources() {
    let temp = temp_case_dir("contract-diagnostics");
    let source_root = temp.join("formula-bundle");
    let bundle_path = temp.join("draft.veproj");
    fs::create_dir_all(&source_root).expect("source root should create");
    fs::create_dir_all(&bundle_path).expect("bundle dir should create");

    let result = ResourceLocalizer::default()
        .localize(ResourceLocalizationRequest {
            bundle_path,
            source_root,
            resources: vec![
                FormulaResourceRef {
                    resource_id: "missing-sticker".to_owned(),
                    kind: ResourceKind::Sticker,
                    uri: "resources/stickers/missing.png".to_owned(),
                    sha256: None,
                    display_name: None,
                },
                FormulaResourceRef {
                    resource_id: "remote-video".to_owned(),
                    kind: ResourceKind::Video,
                    uri: "https://example.invalid/template/video.mp4".to_owned(),
                    sha256: None,
                    display_name: None,
                },
            ],
            mode: ResourceLocalizationMode::CopyRenderableResources,
        })
        .expect("diagnostic-only localization should not fail");

    assert_eq!(result.manifest.resources.len(), 2);
    assert!(result.manifest.resources.iter().all(|resource| {
        resource.bundle_relative_uri.is_none()
            && matches!(
                resource.status,
                LocalizedResourceStatus::Missing | LocalizedResourceStatus::RemoteRenderUrl
            )
    }));
    assert_eq!(result.diagnostics.len(), 2);
    assert!(result.diagnostics.iter().all(|item| {
        item.status == CompatibilityStatus::MissingResource
            && item.external_id.as_deref().is_some()
            && item.message.contains("resource")
    }));
}

fn temp_case_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("adapter-kaipai-{name}-{nonce}"));
    recreate_dir(&path);
    path
}

fn recreate_dir(path: &Path) {
    if path.exists() {
        fs::remove_dir_all(path).expect("old temp dir should remove");
    }
    fs::create_dir_all(path).expect("temp dir should create");
}

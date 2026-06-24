use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use draft_import::{
    AdaptationCategory, AdaptationStatus, AdaptationTargetKind, LocalizedResourceIndexKind,
    LocalizedResourceStatus, ResourceLocalizationMode, ResourceLocalizationRequest,
    TemplateResourceKind, TemplateResourceRef, localize_template_resources,
};

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("draft_import should live under crates/")
        .to_path_buf()
}

#[test]
fn resource_localizer_copies_assets_under_template_import_resources() {
    let temp = temp_case_dir("copy-assets");
    let source_root = temp.join("source-bundle");
    let bundle_path = temp.join("localized.veproj");
    fs::create_dir_all(source_root.join("assets/fonts")).expect("source font dir should create");
    fs::create_dir_all(source_root.join("assets/stickers")).expect("source sticker dir should create");
    fs::create_dir_all(&bundle_path).expect("bundle dir should create");
    fs::write(source_root.join("assets/fonts/main.ttf"), b"local-font-fixture")
        .expect("font fixture should write");
    fs::write(
        source_root.join("assets/stickers/overlay.png"),
        b"local-sticker-fixture",
    )
    .expect("sticker fixture should write");

    let result = localize_template_resources(ResourceLocalizationRequest {
        bundle_path: bundle_path.clone(),
        source_root,
        import_id: "template-alpha".to_owned(),
        resources: vec![
            template_resource(
                "font-main",
                TemplateResourceKind::Font,
                "assets/fonts/main.ttf",
                Some("001185e820f758e192bc6ba683933eb87756e4009b31422f41c2c6b848be0270"),
            ),
            template_resource(
                "sticker-overlay",
                TemplateResourceKind::Sticker,
                "assets/stickers/overlay.png",
                Some("1055b0032a851c06b42b46cd46a9f79472d35b095ee415afee271cc03df775b0"),
            ),
        ],
        mode: ResourceLocalizationMode::CopyRenderableResources,
    })
    .expect("local resources should localize");

    assert!(result.diagnostics.is_empty());
    assert_eq!(result.manifest.import_id, "template-alpha");
    assert_eq!(result.manifest.resources.len(), 2);

    let font = localized(&result.manifest.resources, "font-main");
    assert_eq!(font.status, LocalizedResourceStatus::Available);
    assert_eq!(
        font.project_relative_ref.as_deref(),
        Some("resources/template-import/template-alpha/fonts/font-main/assets/fonts/main.ttf")
    );
    assert_eq!(
        font.resource_index_ref.kind,
        LocalizedResourceIndexKind::Font
    );
    assert_eq!(
        font.resource_index_ref.project_relative_ref.as_deref(),
        font.project_relative_ref.as_deref()
    );
    assert!(bundle_path.join(font.project_relative_ref.as_ref().unwrap()).is_file());

    let sticker = localized(&result.manifest.resources, "sticker-overlay");
    assert_eq!(sticker.status, LocalizedResourceStatus::Available);
    assert_eq!(
        sticker.project_relative_ref.as_deref(),
        Some(
            "resources/template-import/template-alpha/stickers/sticker-overlay/assets/stickers/overlay.png"
        )
    );
    assert_eq!(
        sticker.resource_index_ref.kind,
        LocalizedResourceIndexKind::Material
    );
    assert!(bundle_path.join(sticker.project_relative_ref.as_ref().unwrap()).is_file());
}

#[test]
fn resource_localizer_reports_missing_and_sha256_mismatch_without_partial_output() {
    let temp = temp_case_dir("missing-and-mismatch");
    let source_root = temp.join("source-bundle");
    let bundle_path = temp.join("localized.veproj");
    fs::create_dir_all(source_root.join("assets/fonts")).expect("source font dir should create");
    fs::create_dir_all(&bundle_path).expect("bundle dir should create");
    fs::write(source_root.join("assets/fonts/main.ttf"), b"local-font-fixture")
        .expect("font fixture should write");

    let result = localize_template_resources(ResourceLocalizationRequest {
        bundle_path: bundle_path.clone(),
        source_root,
        import_id: "template-beta".to_owned(),
        resources: vec![
            template_resource(
                "missing-sticker",
                TemplateResourceKind::Sticker,
                "assets/stickers/missing.png",
                None,
            ),
            template_resource(
                "font-mismatch",
                TemplateResourceKind::Font,
                "assets/fonts/main.ttf",
                Some("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
            ),
        ],
        mode: ResourceLocalizationMode::CopyRenderableResources,
    })
    .expect("resource failures should be reportable diagnostics");

    assert_eq!(
        localized(&result.manifest.resources, "missing-sticker").status,
        LocalizedResourceStatus::Missing
    );
    assert_eq!(
        localized(&result.manifest.resources, "font-mismatch").status,
        LocalizedResourceStatus::Sha256Mismatch
    );
    assert_eq!(result.diagnostics.len(), 2);
    assert_resource_diagnostic(&result, "missing-sticker", "missing");
    assert_resource_diagnostic(&result, "font-mismatch", "sha256");
    assert!(
        !bundle_path
            .join("resources/template-import/template-beta/fonts/font-mismatch/assets/fonts/main.ttf")
            .exists(),
        "sha mismatch must not copy a partial resource"
    );
}

#[test]
fn resource_localizer_rejects_traversal_remote_urls_and_duplicate_destinations() {
    let temp = temp_case_dir("unsafe-inputs");
    let source_root = temp.join("source-bundle");
    let bundle_path = temp.join("localized.veproj");
    fs::create_dir_all(source_root.join("assets/video")).expect("source video dir should create");
    fs::create_dir_all(&bundle_path).expect("bundle dir should create");
    fs::write(source_root.join("assets/video/clip.mp4"), b"local-video-fixture")
        .expect("video fixture should write");

    let result = localize_template_resources(ResourceLocalizationRequest {
        bundle_path: bundle_path.clone(),
        source_root,
        import_id: "template-gamma".to_owned(),
        resources: vec![
            template_resource(
                "traversal-sticker",
                TemplateResourceKind::Sticker,
                "../outside.png",
                None,
            ),
            template_resource(
                "remote-video",
                TemplateResourceKind::Video,
                "https://example.invalid/render/video.mp4",
                None,
            ),
            template_resource(
                "duplicate-video",
                TemplateResourceKind::Video,
                "assets/video/clip.mp4",
                None,
            ),
            template_resource(
                "duplicate-video",
                TemplateResourceKind::Video,
                "assets/video/clip.mp4",
                None,
            ),
        ],
        mode: ResourceLocalizationMode::CopyRenderableResources,
    })
    .expect("unsafe resource references should produce diagnostics");

    assert_eq!(
        localized(&result.manifest.resources, "traversal-sticker").status,
        LocalizedResourceStatus::UnsafePath
    );
    assert_eq!(
        localized(&result.manifest.resources, "remote-video").status,
        LocalizedResourceStatus::RemoteRenderUrl
    );
    assert_eq!(
        result.manifest.resources[2].status,
        LocalizedResourceStatus::Available
    );
    assert_eq!(
        result.manifest.resources[3].status,
        LocalizedResourceStatus::DuplicateDestination
    );
    assert_resource_diagnostic(&result, "traversal-sticker", "unsafe");
    assert_resource_diagnostic(&result, "remote-video", "remote");
    assert_resource_diagnostic(&result, "duplicate-video", "duplicate");
    assert!(
        !bundle_path.join("resources/template-import/template-gamma/stickers/traversal-sticker/outside.png").exists(),
        "traversal output must not be created"
    );
    assert!(
        localized(&result.manifest.resources, "remote-video")
            .project_relative_ref
            .is_none(),
        "remote URLs must never remain runtime refs for preview/export"
    );
}

#[test]
#[cfg(unix)]
fn resource_localizer_rejects_source_symlink_escape() {
    use std::os::unix::fs::symlink;

    let temp = temp_case_dir("source-symlink");
    let source_root = temp.join("source-bundle");
    let bundle_path = temp.join("localized.veproj");
    let outside_path = temp.join("outside-font.ttf");
    fs::create_dir_all(source_root.join("assets/fonts")).expect("source font dir should create");
    fs::create_dir_all(&bundle_path).expect("bundle dir should create");
    fs::write(&outside_path, b"must-not-copy").expect("outside file should write");
    symlink(
        &outside_path,
        source_root.join("assets/fonts/leaked-font.ttf"),
    )
    .expect("source symlink should create");

    let result = localize_template_resources(ResourceLocalizationRequest {
        bundle_path: bundle_path.clone(),
        source_root,
        import_id: "template-delta".to_owned(),
        resources: vec![template_resource(
            "font-symlink",
            TemplateResourceKind::Font,
            "assets/fonts/leaked-font.ttf",
            None,
        )],
        mode: ResourceLocalizationMode::CopyRenderableResources,
    })
    .expect("symlink source should report diagnostic");

    assert_eq!(
        result.manifest.resources[0].status,
        LocalizedResourceStatus::UnsafePath
    );
    assert_resource_diagnostic(&result, "font-symlink", "symlink");
    assert!(
        !bundle_path
            .join("resources/template-import/template-delta/fonts/font-symlink/assets/fonts/leaked-font.ttf")
            .exists(),
        "source symlink target must not be copied"
    );
}

#[test]
#[cfg(not(unix))]
fn resource_localizer_rejects_source_symlink_escape() {
    eprintln!("skipping source symlink escape case: platform symlink setup is unsupported");
}

#[test]
#[cfg(unix)]
fn resource_localizer_rejects_destination_symlink_escape() {
    use std::os::unix::fs::symlink;

    let temp = temp_case_dir("destination-symlink");
    let source_root = temp.join("source-bundle");
    let bundle_path = temp.join("localized.veproj");
    let outside_dir = temp.join("outside-bundle");
    fs::create_dir_all(source_root.join("assets/images")).expect("source image dir should create");
    fs::create_dir_all(&bundle_path).expect("bundle dir should create");
    fs::create_dir_all(&outside_dir).expect("outside dir should create");
    fs::write(source_root.join("assets/images/overlay.png"), b"overlay")
        .expect("source resource should write");
    symlink(&outside_dir, bundle_path.join("resources"))
        .expect("destination symlink should create");

    let result = localize_template_resources(ResourceLocalizationRequest {
        bundle_path,
        source_root,
        import_id: "template-epsilon".to_owned(),
        resources: vec![template_resource(
            "overlay-symlink",
            TemplateResourceKind::Image,
            "assets/images/overlay.png",
            None,
        )],
        mode: ResourceLocalizationMode::CopyRenderableResources,
    })
    .expect("destination symlink should report diagnostic");

    assert_eq!(
        result.manifest.resources[0].status,
        LocalizedResourceStatus::UnsafePath
    );
    assert_resource_diagnostic(&result, "overlay-symlink", "symlink");
    assert!(
        directory_is_empty(&outside_dir).expect("outside dir should be readable"),
        "destination symlink must not receive copied resources"
    );
}

#[test]
#[cfg(not(unix))]
fn resource_localizer_rejects_destination_symlink_escape() {
    eprintln!("skipping destination symlink escape case: platform symlink setup is unsupported");
}

#[test]
fn resource_localizer_fixture_scan_rejects_credentials_and_signed_urls() {
    let fixture_root = project_root().join("fixtures/kaipai/resources");
    assert_resource_fixture_tree_is_sanitized(&fixture_root)
        .expect("committed resource fixtures should not contain secret-like evidence");

    for (name, bytes) in [
        ("credential-key.json", br#"{"access_token":"redacted"}"#.as_slice()),
        (
            "signed-url.json",
            br#"{"resource":"https://example.invalid/a.mp4?X-Amz-Signature=redacted"}"#.as_slice(),
        ),
        ("cookie-header.txt", b"Cookie: session=redacted".as_slice()),
    ] {
        assert!(
            scan_resource_fixture_bytes(name, bytes).is_err(),
            "fixture scanner should reject {name}"
        );
    }
}

fn template_resource(
    stable_id: &str,
    kind: TemplateResourceKind,
    source_uri: &str,
    sha256: Option<&str>,
) -> TemplateResourceRef {
    TemplateResourceRef {
        stable_id: stable_id.to_owned(),
        kind,
        source_uri: source_uri.to_owned(),
        sha256: sha256.map(str::to_owned),
        display_name: Some(stable_id.to_owned()),
    }
}

fn localized<'a>(
    resources: &'a [draft_import::LocalizedResource],
    stable_id: &str,
) -> &'a draft_import::LocalizedResource {
    resources
        .iter()
        .find(|resource| resource.stable_id == stable_id)
        .unwrap_or_else(|| panic!("localized resource should exist: {stable_id}"))
}

fn assert_resource_diagnostic(
    result: &draft_import::ResourceLocalizationResult,
    stable_id: &str,
    message_fragment: &str,
) {
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|item| {
            item.status == AdaptationStatus::MissingResource
                && item.category == AdaptationCategory::Resource
                && item
                    .target
                    .as_ref()
                    .is_some_and(|target| {
                        target.kind == AdaptationTargetKind::Resource
                            && target.id.as_deref() == Some(stable_id)
                    })
        })
        .unwrap_or_else(|| panic!("resource diagnostic should exist for {stable_id}"));

    assert!(
        diagnostic
            .message
            .to_ascii_lowercase()
            .contains(message_fragment),
        "diagnostic should mention {message_fragment}: {}",
        diagnostic.message
    );
}

fn assert_resource_fixture_tree_is_sanitized(root: &Path) -> Result<(), String> {
    if !root.exists() {
        return Err(format!("fixture root does not exist: {}", root.display()));
    }
    for file in resource_fixture_files(root).map_err(|error| error.to_string())? {
        if file.extension().and_then(|extension| extension.to_str()) == Some("md") {
            continue;
        }
        let bytes = fs::read(&file).map_err(|error| format!("{}: {error}", file.display()))?;
        scan_resource_fixture_bytes(&file.to_string_lossy(), &bytes)?;
    }
    Ok(())
}

fn resource_fixture_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(resource_fixture_files(&path)?);
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

fn scan_resource_fixture_bytes(name: &str, bytes: &[u8]) -> Result<(), String> {
    let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    let credential_keys = [
        "access_token",
        "authorization",
        "cookie:",
        "cookie_header",
        "secret",
        "sessionid",
        "account_id",
    ];
    if credential_keys.iter().any(|needle| text.contains(needle)) {
        return Err(format!("{name}: credential-like resource evidence"));
    }
    let signed_url_markers = [
        "x-amz-signature=",
        "x-oss-signature=",
        "signature=",
        "expires=",
        "x-security-token=",
    ];
    if text.contains("http") && signed_url_markers.iter().any(|needle| text.contains(needle)) {
        return Err(format!("{name}: signed resource URL shape"));
    }
    Ok(())
}

fn directory_is_empty(path: &Path) -> io::Result<bool> {
    Ok(fs::read_dir(path)?.next().is_none())
}

fn temp_case_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("draft-import-resource-{name}-{nonce}"));
    if path.exists() {
        fs::remove_dir_all(&path).expect("old temp dir should remove");
    }
    fs::create_dir_all(&path).expect("temp dir should create");
    path
}

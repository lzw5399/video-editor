use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use adapter_kaipai::{
    CompatibilityCanonicalTarget, CompatibilityCategory, CompatibilityReport,
    CompatibilityReportItem, CompatibilityReportSchemaVersion, CompatibilityReportSummary,
    CompatibilitySeverity, CompatibilityStatus, KaipaiFormulaBundle, ResourceLocalizationMode,
    ResourceLocalizationRequest, ResourceLocalizer, classify_formula_bundle_compatibility,
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
fn compatibility_report_contract_status_taxonomy_is_locked() {
    let statuses = [
        CompatibilityStatus::Supported,
        CompatibilityStatus::Degraded,
        CompatibilityStatus::Unsupported,
        CompatibilityStatus::MissingResource,
        CompatibilityStatus::NeedsNativeEffect,
    ];

    let serialized = statuses
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .expect("compatibility statuses should serialize");

    assert_eq!(
        serialized,
        vec![
            serde_json::json!("supported"),
            serde_json::json!("degraded"),
            serde_json::json!("unsupported"),
            serde_json::json!("missingResource"),
            serde_json::json!("needsNativeEffect"),
        ]
    );
}

#[test]
fn compatibility_report_contract_contains_stable_diagnostic_fields() {
    let report = CompatibilityReport {
        schema_version: CompatibilityReportSchemaVersion::current(),
        source_kind: "offlineFormulaBundle".to_owned(),
        source_id: "template:redacted-template-001".to_owned(),
        generated_at: "2026-06-17T00:00:00Z".to_owned(),
        summary: CompatibilityReportSummary {
            supported: 1,
            degraded: 1,
            unsupported: 1,
            missing_resource: 1,
            needs_native_effect: 1,
        },
        items: vec![CompatibilityReportItem {
            status: CompatibilityStatus::Supported,
            severity: CompatibilitySeverity::Info,
            category: CompatibilityCategory::Material,
            external_path: "sourceMedia".to_owned(),
            external_id: Some("material:source-video".to_owned()),
            canonical_target: Some(CompatibilityCanonicalTarget::Material),
            message: "Source media can map to a draft material.".to_owned(),
            details: None,
        }],
        provenance_digest: Some("sha256:redacted-fixture-digest".to_owned()),
    };

    let value = serde_json::to_value(report).expect("report should serialize");
    assert_eq!(value["schemaVersion"], 1);
    assert_eq!(value["sourceKind"], "offlineFormulaBundle");
    assert_eq!(value["sourceId"], "template:redacted-template-001");
    assert_eq!(value["generatedAt"], "2026-06-17T00:00:00Z");
    assert_eq!(value["items"][0]["externalPath"], "sourceMedia");
    assert_eq!(value["items"][0]["canonicalTarget"], "material");
    assert_eq!(value["provenanceDigest"], "sha256:redacted-fixture-digest");
}

#[test]
fn compatibility_report_snapshots_cover_locked_statuses() {
    let root = project_root();
    let report_dir = root.join("fixtures/kaipai/expected-reports");
    let schema = compatibility_report_schema_validator();

    if env::var_os("VE_UPDATE_COMPATIBILITY_REPORTS").as_deref() == Some(std::ffi::OsStr::new("1"))
    {
        write_expected_report_snapshots(&root, &report_dir);
    }

    let actual = report_snapshot_paths(&report_dir);
    let expected = expected_report_snapshots()
        .iter()
        .map(|case| case.path.to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "every Kaipai compatibility report snapshot must be explicitly classified"
    );

    let mut statuses = BTreeSet::new();
    for case in expected_report_snapshots() {
        let expected_report = case.report(&root);
        let expected_json = serde_json::to_string_pretty(&expected_report)
            .expect("expected report should serialize")
            + "\n";
        let actual_json = fs::read_to_string(report_dir.join(case.path)).unwrap_or_else(|error| {
            panic!("report snapshot should be readable: {}: {error}", case.path)
        });
        assert_eq!(
            actual_json, expected_json,
            "report snapshot drifted: {}",
            case.path
        );

        let report_value = read_report_snapshot(&report_dir, case.path);
        schema.validate(&report_value).unwrap_or_else(|error| {
            panic!(
                "report snapshot should validate against generated schema: {}: {error}",
                case.path
            )
        });
        let report: CompatibilityReport = serde_json::from_value(report_value.clone())
            .unwrap_or_else(|error| {
                panic!(
                    "report snapshot should deserialize through Rust contract: {}: {error}",
                    case.path
                )
            });

        let item = report_value["items"]
            .as_array()
            .and_then(|items| items.first())
            .unwrap_or_else(|| panic!("snapshot should contain at least one item: {}", case.path));
        assert_eq!(item["status"], Value::String(case.status.to_owned()));
        assert_eq!(report.items[0].status, case.expected_status);
        statuses.insert(case.status);
    }

    assert_eq!(
        statuses,
        BTreeSet::from([
            "supported",
            "degraded",
            "unsupported",
            "missingResource",
            "needsNativeEffect",
        ])
    );
}

struct ExpectedReportSnapshot {
    path: &'static str,
    status: &'static str,
    expected_status: CompatibilityStatus,
    build: fn(&Path) -> CompatibilityReport,
}

fn expected_report_snapshots() -> Vec<ExpectedReportSnapshot> {
    vec![
        ExpectedReportSnapshot {
            path: "supported-source-material.report.json",
            status: "supported",
            expected_status: CompatibilityStatus::Supported,
            build: supported_source_material_report,
        },
        ExpectedReportSnapshot {
            path: "degraded-text-style.report.json",
            status: "degraded",
            expected_status: CompatibilityStatus::Degraded,
            build: degraded_text_style_report,
        },
        ExpectedReportSnapshot {
            path: "unsupported-formula-block.report.json",
            status: "unsupported",
            expected_status: CompatibilityStatus::Unsupported,
            build: unsupported_formula_block_report,
        },
        ExpectedReportSnapshot {
            path: "missing-resource.report.json",
            status: "missingResource",
            expected_status: CompatibilityStatus::MissingResource,
            build: missing_resource_report,
        },
        ExpectedReportSnapshot {
            path: "native-effect-needs-native-effect.report.json",
            status: "needsNativeEffect",
            expected_status: CompatibilityStatus::NeedsNativeEffect,
            build: native_effect_report,
        },
    ]
}

impl ExpectedReportSnapshot {
    fn report(&self, root: &Path) -> CompatibilityReport {
        (self.build)(root)
    }
}

fn report_snapshot_paths(report_dir: &Path) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    if report_dir.exists() {
        for entry in fs::read_dir(report_dir).expect("report snapshot directory should be readable")
        {
            let entry = entry.expect("report snapshot directory entry should be readable");
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
                paths.insert(
                    path.file_name()
                        .expect("snapshot should have a file name")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    paths
}

fn read_report_snapshot(report_dir: &Path, snapshot_path: &str) -> Value {
    serde_json::from_slice(
        &fs::read(report_dir.join(snapshot_path)).unwrap_or_else(|error| {
            panic!("report snapshot should be readable: {snapshot_path}: {error}")
        }),
    )
    .unwrap_or_else(|error| panic!("report snapshot should parse: {snapshot_path}: {error}"))
}

fn write_expected_report_snapshots(root: &Path, report_dir: &Path) {
    fs::create_dir_all(report_dir).expect("report snapshot directory should be created");
    for case in expected_report_snapshots() {
        let report = case.report(root);
        let json = serde_json::to_string_pretty(&report)
            .expect("compatibility report should serialize")
            + "\n";
        fs::write(report_dir.join(case.path), json).unwrap_or_else(|error| {
            panic!("report snapshot should be written: {}: {error}", case.path)
        });
    }
}

fn supported_source_material_report(root: &Path) -> CompatibilityReport {
    classify_fixture(root, base_fixture_value(root))
}

fn degraded_text_style_report(root: &Path) -> CompatibilityReport {
    classify_fixture(
        root,
        patch(base_fixture_value(root), |value| {
            value["formula"]["textStyleFallback"] = json!({
                "source": "kaipaiNativeTextStyle",
                "fallback": "basicTextStyle"
            });
        }),
    )
}

fn unsupported_formula_block_report(root: &Path) -> CompatibilityReport {
    classify_fixture(
        root,
        patch(base_fixture_value(root), |value| {
            value["formula"]["unsupportedBlocks"] = json!(["smartBeatSync"]);
        }),
    )
}

fn missing_resource_report(root: &Path) -> CompatibilityReport {
    let value = patch(base_fixture_value(root), |value| {
        value["resources"] = json!([
            {
                "resourceId": "missing-font-default",
                "kind": "font",
                "uri": "resources/missing/redacted-default.ttf",
                "sha256": "4444444444444444444444444444444444444444444444444444444444444444",
                "displayName": "redacted-default.ttf"
            }
        ]);
    });
    let bundle = formula_bundle(value);
    let localization = localize_for_report(&bundle, "missing-resource-report");
    classify_formula_bundle_compatibility(&bundle, Some(&localization), "2026-06-17T00:00:00Z")
}

fn native_effect_report(root: &Path) -> CompatibilityReport {
    classify_fixture(
        root,
        patch(base_fixture_value(root), |value| {
            value["formula"]["effects"] = json!([
                {
                    "nativeEffectId": "kaipai-native-beauty-glow",
                    "name": "Kaipai native beauty glow",
                    "requiresNativeEffect": true
                }
            ]);
        }),
    )
}

fn classify_fixture(_root: &Path, value: Value) -> CompatibilityReport {
    let bundle = formula_bundle(value);
    classify_formula_bundle_compatibility(&bundle, None, "2026-06-17T00:00:00Z")
}

fn formula_bundle(value: Value) -> KaipaiFormulaBundle {
    KaipaiFormulaBundle::from_json_value(value)
        .expect("compatibility report fixture evidence should validate through formula bundle")
}

fn localize_for_report(
    bundle: &KaipaiFormulaBundle,
    case_name: &str,
) -> adapter_kaipai::ResourceLocalizationResult {
    let temp = temp_case_dir(case_name);
    let source_root = temp.join("formula-bundle");
    let bundle_path = temp.join("draft.veproj");
    fs::create_dir_all(&source_root).expect("source root should create");
    fs::create_dir_all(&bundle_path).expect("bundle dir should create");
    ResourceLocalizer::default()
        .localize(ResourceLocalizationRequest {
            bundle_path,
            source_root,
            resources: bundle.resources.clone(),
            mode: ResourceLocalizationMode::PreserveExternalSourceMedia,
        })
        .expect("compatibility report localization should complete")
}

fn base_fixture_value(root: &Path) -> Value {
    serde_json::from_slice(
        &fs::read(root.join("fixtures/kaipai/positive/sanitized-formula-bundle.json"))
            .expect("base formula fixture should be readable"),
    )
    .expect("base formula fixture should parse")
}

fn patch(mut value: Value, update: impl FnOnce(&mut Value)) -> Value {
    update(&mut value);
    value
}

fn compatibility_report_schema_validator() -> jsonschema::Validator {
    let schema_path = project_root().join("schemas/compatibility-report.schema.json");
    let schema_json: Value = serde_json::from_slice(
        &fs::read(&schema_path).expect("generated compatibility report schema should be readable"),
    )
    .expect("generated compatibility report schema should parse");
    jsonschema::validator_for(&schema_json)
        .expect("generated compatibility report schema should compile")
}

#[test]
fn compatibility_report_native_effects_are_not_smuggled_into_filter_parameters() {
    let root = project_root();
    let report = native_effect_report(&root);
    assert_eq!(
        report.items[0].status,
        CompatibilityStatus::NeedsNativeEffect
    );
    assert_eq!(report.items[0].external_path, "formula.effects[0]");
    assert_eq!(report.items[0].canonical_target, None);

    let serialized = serde_json::to_string(&report).expect("native effect report should serialize");
    let forbidden_filter_parameters = ["Filter", "parameters"].join(".");
    assert!(!serialized.contains(&forbidden_filter_parameters));
    assert!(!serialized.contains("\"parameters\""));
}

#[test]
fn compatibility_report_detects_nested_native_effects() {
    let root = project_root();
    let bundle = formula_bundle(patch(base_fixture_value(&root), |value| {
        value["formula"]["timeline"]["segments"][0]["effects"] = json!([
            {
                "nativeEffectId": "kaipai-native-segment-glow",
                "requiresNativeEffect": true
            }
        ]);
    }));

    let report = classify_formula_bundle_compatibility(&bundle, None, "2026-06-17T00:00:00Z");

    assert_eq!(report.items.len(), 1);
    assert_eq!(
        report.items[0].status,
        CompatibilityStatus::NeedsNativeEffect
    );
    assert_eq!(
        report.items[0].external_path,
        "formula.timeline.segments[0].effects[0]"
    );
}

#[test]
fn compatibility_report_rejects_unknown_nested_formula_blocks() {
    let root = project_root();
    let bundle = formula_bundle(patch(base_fixture_value(&root), |value| {
        value["formula"]["timeline"]["segments"][0]["providerSmartCut"] = json!({
            "mode": "redacted"
        });
    }));

    let report = classify_formula_bundle_compatibility(&bundle, None, "2026-06-17T00:00:00Z");

    assert_eq!(report.items.len(), 1);
    assert_eq!(report.items[0].status, CompatibilityStatus::Unsupported);
    assert_eq!(
        report.items[0].external_path,
        "formula.timeline.segments[0].providerSmartCut"
    );
}

#[test]
fn compatibility_report_uses_localization_diagnostics_for_missing_resources() {
    let root = project_root();
    let missing_bundle = formula_bundle(patch(base_fixture_value(&root), |value| {
        value["resources"] = json!([
            {
                "resourceId": "font-normal-id",
                "kind": "font",
                "uri": "resources/fonts/not-present.ttf",
                "displayName": "not-present.ttf"
            }
        ]);
    }));
    let missing_localization = localize_for_report(&missing_bundle, "normal-missing-resource");
    let missing_report = classify_formula_bundle_compatibility(
        &missing_bundle,
        Some(&missing_localization),
        "2026-06-17T00:00:00Z",
    );
    assert_eq!(
        missing_report.items[0].status,
        CompatibilityStatus::MissingResource
    );
    assert_eq!(
        missing_report.items[0].external_id.as_deref(),
        Some("font-normal-id")
    );

    let present_bundle = formula_bundle(patch(base_fixture_value(&root), |value| {
        value["resources"] = json!([
            {
                "resourceId": "font-present-id",
                "kind": "font",
                "uri": "resources/missing/actually-present.ttf",
                "displayName": "actually-present.ttf"
            }
        ]);
    }));
    let temp = temp_case_dir("present-resource-with-missing-path-name");
    let source_root = temp.join("formula-bundle");
    let bundle_path = temp.join("draft.veproj");
    fs::create_dir_all(source_root.join("resources/missing")).expect("source dir should create");
    fs::create_dir_all(&bundle_path).expect("bundle dir should create");
    fs::write(
        source_root.join("resources/missing/actually-present.ttf"),
        b"local-font-fixture",
    )
    .expect("source resource should write");
    let present_localization = ResourceLocalizer::default()
        .localize(ResourceLocalizationRequest {
            bundle_path,
            source_root,
            resources: present_bundle.resources.clone(),
            mode: ResourceLocalizationMode::PreserveExternalSourceMedia,
        })
        .expect("present resource should localize");
    let present_report = classify_formula_bundle_compatibility(
        &present_bundle,
        Some(&present_localization),
        "2026-06-17T00:00:00Z",
    );
    assert!(
        present_report
            .items
            .iter()
            .all(|item| item.status != CompatibilityStatus::MissingResource),
        "resource paths containing `/missing/` must not be classified by name heuristic"
    );
}

fn temp_case_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("adapter-kaipai-compat-{name}-{nonce}"));
    if path.exists() {
        fs::remove_dir_all(&path).expect("old temp dir should remove");
    }
    fs::create_dir_all(&path).expect("temp dir should create");
    path
}

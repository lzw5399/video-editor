use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use adapter_kaipai::{
    CompatibilityCanonicalTarget, CompatibilityCategory, CompatibilityReport,
    CompatibilityReportItem, CompatibilityReportSchemaVersion, CompatibilityReportSummary,
    CompatibilitySeverity, CompatibilityStatus,
};
use serde_json::Value;

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
        source_kind: "kaipaiFormulaBundle".to_owned(),
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
    assert_eq!(value["sourceKind"], "kaipaiFormulaBundle");
    assert_eq!(value["sourceId"], "template:redacted-template-001");
    assert_eq!(value["generatedAt"], "2026-06-17T00:00:00Z");
    assert_eq!(value["items"][0]["externalPath"], "sourceMedia");
    assert_eq!(value["items"][0]["canonicalTarget"], "material");
    assert_eq!(value["provenanceDigest"], "sha256:redacted-fixture-digest");
}

#[test]
fn compatibility_report_snapshots_cover_locked_statuses() {
    let report_dir = project_root().join("fixtures/kaipai/expected-reports");
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
        let report = read_report_snapshot(&report_dir, case.path);
        let item = report["items"]
            .as_array()
            .and_then(|items| items.first())
            .unwrap_or_else(|| panic!("snapshot should contain at least one item: {}", case.path));
        assert_eq!(item["status"], Value::String(case.status.to_owned()));
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
}

fn expected_report_snapshots() -> Vec<ExpectedReportSnapshot> {
    vec![
        ExpectedReportSnapshot {
            path: "supported-source-material.report.json",
            status: "supported",
        },
        ExpectedReportSnapshot {
            path: "degraded-text-style.report.json",
            status: "degraded",
        },
        ExpectedReportSnapshot {
            path: "unsupported-formula-block.report.json",
            status: "unsupported",
        },
        ExpectedReportSnapshot {
            path: "missing-resource.report.json",
            status: "missingResource",
        },
        ExpectedReportSnapshot {
            path: "native-effect-needs-native-effect.report.json",
            status: "needsNativeEffect",
        },
    ]
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

use adapter_kaipai::{
    CompatibilityCanonicalTarget, CompatibilityCategory, CompatibilityReport,
    CompatibilityReportItem, CompatibilityReportSchemaVersion, CompatibilityReportSummary,
    CompatibilitySeverity, CompatibilityStatus,
};

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

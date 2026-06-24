use draft_import::adaptation_report::{
    AdaptationCategory, AdaptationReportItem, AdaptationReportSummary, AdaptationSeverity,
    AdaptationStatus, AdaptationTargetKind, AdaptationTargetRef, ExternalProvenanceRef,
};
use serde_json::json;

#[test]
fn adaptation_report_summary_counts_every_status() {
    let items = report_fixture_items();

    let summary = AdaptationReportSummary::from_items(&items);

    assert_eq!(summary.supported, 1);
    assert_eq!(summary.approximated, 2);
    assert_eq!(summary.dropped, 1);
    assert_eq!(summary.missing_resource, 1);
    assert_eq!(summary.needs_native_effect, 1);
}

#[test]
fn adaptation_report_statuses_serialize_with_public_camel_case_names() {
    let cases = [
        (AdaptationStatus::Supported, "supported"),
        (AdaptationStatus::Approximated, "approximated"),
        (AdaptationStatus::Dropped, "dropped"),
        (AdaptationStatus::MissingResource, "missingResource"),
        (AdaptationStatus::NeedsNativeEffect, "needsNativeEffect"),
    ];

    for (status, expected) in cases {
        assert_eq!(serde_json::to_value(status).unwrap(), json!(expected));
    }
}

#[test]
fn adaptation_report_categories_cover_product_facing_import_concepts() {
    let categories = [
        (AdaptationCategory::SourceMedia, "sourceMedia"),
        (AdaptationCategory::Canvas, "canvas"),
        (AdaptationCategory::Material, "material"),
        (AdaptationCategory::Track, "track"),
        (AdaptationCategory::Segment, "segment"),
        (AdaptationCategory::Text, "text"),
        (AdaptationCategory::Sticker, "sticker"),
        (AdaptationCategory::Audio, "audio"),
        (AdaptationCategory::Animation, "animation"),
        (AdaptationCategory::Transition, "transition"),
        (AdaptationCategory::Resource, "resource"),
        (AdaptationCategory::Font, "font"),
        (AdaptationCategory::NativeEffect, "nativeEffect"),
    ];

    for (category, expected) in categories {
        assert_eq!(serde_json::to_value(category).unwrap(), json!(expected));
    }
}

#[test]
fn adaptation_report_provenance_preserves_external_references_without_canonical_render_fields() {
    let item = AdaptationReportItem {
        status: AdaptationStatus::Approximated,
        severity: AdaptationSeverity::Warning,
        category: AdaptationCategory::Animation,
        target: Some(AdaptationTargetRef {
            kind: AdaptationTargetKind::Keyframe,
            id: Some("segment-main-video".to_owned()),
        }),
        message: "Provider motion curve mapped to linear keyframes.".to_owned(),
        details: Some("External references remain report evidence only.".to_owned()),
        provenance: vec![ExternalProvenanceRef {
            source_kind: "offlineTemplateBundle".to_owned(),
            external_id: Some("template-42".to_owned()),
            external_path: Some("timeline.segments[0].animation.curve".to_owned()),
            note: Some("original curve preserved for diagnostics".to_owned()),
        }],
    };

    let serialized = serde_json::to_value(&item).unwrap();
    let object = serialized.as_object().unwrap();

    assert_eq!(
        serialized.pointer("/provenance/0/externalId"),
        Some(&json!("template-42"))
    );
    assert_eq!(
        serialized.pointer("/provenance/0/externalPath"),
        Some(&json!("timeline.segments[0].animation.curve"))
    );
    for forbidden_field in ["templateId", "recipeId", "formula", "safeArea"] {
        assert!(
            !object.contains_key(forbidden_field),
            "report item must not expose {forbidden_field} as canonical render semantics"
        );
    }
}

fn report_fixture_items() -> Vec<AdaptationReportItem> {
    vec![
        report_item(
            AdaptationStatus::Supported,
            AdaptationCategory::SourceMedia,
            AdaptationTargetKind::Material,
            "main video maps to a draft material",
        ),
        report_item(
            AdaptationStatus::Approximated,
            AdaptationCategory::Animation,
            AdaptationTargetKind::Keyframe,
            "transform animation maps to linear keyframes",
        ),
        report_item(
            AdaptationStatus::Approximated,
            AdaptationCategory::Text,
            AdaptationTargetKind::Text,
            "text styling maps to the supported draft subset",
        ),
        report_item(
            AdaptationStatus::Dropped,
            AdaptationCategory::Segment,
            AdaptationTargetKind::Segment,
            "unsupported provider block is dropped",
        ),
        report_item(
            AdaptationStatus::MissingResource,
            AdaptationCategory::Resource,
            AdaptationTargetKind::Material,
            "referenced resource is unavailable",
        ),
        report_item(
            AdaptationStatus::NeedsNativeEffect,
            AdaptationCategory::NativeEffect,
            AdaptationTargetKind::Effect,
            "native effect requires an explicit local implementation",
        ),
    ]
}

fn report_item(
    status: AdaptationStatus,
    category: AdaptationCategory,
    target_kind: AdaptationTargetKind,
    message: &str,
) -> AdaptationReportItem {
    AdaptationReportItem {
        status,
        severity: match status {
            AdaptationStatus::Supported => AdaptationSeverity::Info,
            AdaptationStatus::Approximated => AdaptationSeverity::Warning,
            AdaptationStatus::Dropped
            | AdaptationStatus::MissingResource
            | AdaptationStatus::NeedsNativeEffect => AdaptationSeverity::Error,
        },
        category,
        target: Some(AdaptationTargetRef {
            kind: target_kind,
            id: Some(format!("{category:?}-{status:?}")),
        }),
        message: message.to_owned(),
        details: None,
        provenance: vec![ExternalProvenanceRef {
            source_kind: "offlineTemplateBundle".to_owned(),
            external_id: Some(format!("external-{status:?}")),
            external_path: Some(format!("fixture.{category:?}.{status:?}")),
            note: None,
        }],
    }
}

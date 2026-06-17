use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{KaipaiFormulaBundle, resource_localizer::ResourceLocalizationResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CompatibilityReportSchemaVersion(pub u32);

impl CompatibilityReportSchemaVersion {
    pub const CURRENT_VALUE: u32 = 1;
    pub const CURRENT: Self = Self(Self::CURRENT_VALUE);

    pub fn current() -> Self {
        Self::CURRENT
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompatibilityReport {
    pub schema_version: CompatibilityReportSchemaVersion,
    pub source_kind: String,
    pub source_id: String,
    pub generated_at: String,
    pub summary: CompatibilityReportSummary,
    pub items: Vec<CompatibilityReportItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance_digest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompatibilityReportSummary {
    pub supported: u32,
    pub degraded: u32,
    pub unsupported: u32,
    pub missing_resource: u32,
    pub needs_native_effect: u32,
}

impl CompatibilityReportSummary {
    pub fn from_items(items: &[CompatibilityReportItem]) -> Self {
        let mut summary = Self {
            supported: 0,
            degraded: 0,
            unsupported: 0,
            missing_resource: 0,
            needs_native_effect: 0,
        };

        for item in items {
            match item.status {
                CompatibilityStatus::Supported => summary.supported += 1,
                CompatibilityStatus::Degraded => summary.degraded += 1,
                CompatibilityStatus::Unsupported => summary.unsupported += 1,
                CompatibilityStatus::MissingResource => summary.missing_resource += 1,
                CompatibilityStatus::NeedsNativeEffect => summary.needs_native_effect += 1,
            }
        }

        summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompatibilityReportItem {
    pub status: CompatibilityStatus,
    pub severity: CompatibilitySeverity,
    pub category: CompatibilityCategory,
    pub external_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_target: Option<CompatibilityCanonicalTarget>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CompatibilityStatus {
    Supported,
    Degraded,
    Unsupported,
    MissingResource,
    NeedsNativeEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CompatibilitySeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CompatibilityCategory {
    Source,
    Material,
    Track,
    Segment,
    Text,
    Sticker,
    Filter,
    Transition,
    Resource,
    NativeEffect,
    Formula,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CompatibilityCanonicalTarget {
    Draft,
    Material,
    Track,
    Segment,
    Text,
    Sticker,
    Keyframe,
    Filter,
    Transition,
}

pub fn classify_formula_bundle_compatibility(
    bundle: &KaipaiFormulaBundle,
    localization: Option<&ResourceLocalizationResult>,
    generated_at: impl Into<String>,
) -> CompatibilityReport {
    let mut items = Vec::new();

    if let Some(localization) = localization {
        items.extend(localization.diagnostics.iter().cloned());
    }
    collect_formula_semantic_items(&bundle.formula, "formula", &mut items);

    if items.is_empty() {
        items.push(CompatibilityReportItem {
            status: CompatibilityStatus::Supported,
            severity: CompatibilitySeverity::Info,
            category: CompatibilityCategory::Material,
            external_path: "sourceMedia".to_owned(),
            external_id: Some(bundle.source_media.uri.clone()),
            canonical_target: Some(CompatibilityCanonicalTarget::Material),
            message: "Source media can map to a Jianying-style draft material.".to_owned(),
            details: Some(format!(
                "{}x{}, durationMs={}",
                bundle.source_media.width,
                bundle.source_media.height,
                bundle.source_media.duration_ms
            )),
        });
    }

    CompatibilityReport {
        schema_version: CompatibilityReportSchemaVersion::current(),
        source_kind: "offlineFormulaBundle".to_owned(),
        source_id: format!("template:{}", bundle.provenance.template_id),
        generated_at: generated_at.into(),
        summary: CompatibilityReportSummary::from_items(&items),
        items,
        provenance_digest: Some("sha256:redacted-compatibility-fixture".to_owned()),
    }
}

fn collect_formula_semantic_items(
    value: &Value,
    path: &str,
    items: &mut Vec<CompatibilityReportItem>,
) {
    match value {
        Value::Object(object) => {
            if is_native_effect_evidence(value) {
                items.push(native_effect_item(path, value));
                return;
            }
            for (key, child) in object {
                let child_path = format!("{path}.{key}");
                if key == "textStyleFallback" {
                    items.push(degraded_text_style_item(&child_path));
                    continue;
                }
                if key == "unsupportedBlocks" {
                    collect_unsupported_formula_block_items(child, &child_path, items);
                    continue;
                }
                if !is_allowed_formula_key(path, key) {
                    items.push(unsupported_formula_item(&child_path, key));
                    continue;
                }
                collect_formula_semantic_items(child, &child_path, items);
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                collect_formula_semantic_items(child, &format!("{path}[{index}]"), items);
            }
        }
        _ => {}
    }
}

fn degraded_text_style_item(path: &str) -> CompatibilityReportItem {
    CompatibilityReportItem {
        status: CompatibilityStatus::Degraded,
        severity: CompatibilitySeverity::Warning,
        category: CompatibilityCategory::Text,
        external_path: path.to_owned(),
        external_id: None,
        canonical_target: Some(CompatibilityCanonicalTarget::Text),
        message: "Text style has unsupported provider-specific attributes and will use a simpler draft text style.".to_owned(),
        details: Some("Preserves text content and basic style only.".to_owned()),
    }
}

fn collect_unsupported_formula_block_items(
    value: &Value,
    path: &str,
    items: &mut Vec<CompatibilityReportItem>,
) {
    let Some(blocks) = value.as_array() else {
        items.push(unsupported_formula_item(path, "unsupportedBlocks"));
        return;
    };

    for (index, block) in blocks.iter().enumerate() {
        let name = block.as_str().unwrap_or("unknownProviderBlock");
        items.push(unsupported_formula_item(&format!("{path}[{index}]"), name));
    }
}

fn unsupported_formula_item(path: &str, external_id: &str) -> CompatibilityReportItem {
    CompatibilityReportItem {
        status: CompatibilityStatus::Unsupported,
        severity: CompatibilitySeverity::Error,
        category: CompatibilityCategory::Formula,
        external_path: path.to_owned(),
        external_id: Some(external_id.to_owned()),
        canonical_target: None,
        message: "Formula block has no supported Jianying-style draft semantic target yet."
            .to_owned(),
        details: Some("Mapper work must not claim support for this provider block.".to_owned()),
    }
}

fn native_effect_item(path: &str, effect: &Value) -> CompatibilityReportItem {
    let external_id = effect
        .get("nativeEffectId")
        .or_else(|| effect.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("nativeEffect");
    CompatibilityReportItem {
        status: CompatibilityStatus::NeedsNativeEffect,
        severity: CompatibilitySeverity::Warning,
        category: CompatibilityCategory::NativeEffect,
        external_path: path.to_owned(),
        external_id: Some(external_id.to_owned()),
        canonical_target: None,
        message: "Provider-native effect requires explicit compatibility handling before it can be represented locally.".to_owned(),
        details: Some("Do not smuggle native effect data into generic filter parameters.".to_owned()),
    }
}

fn is_allowed_formula_key(parent_path: &str, key: &str) -> bool {
    let normalized_parent = normalize_formula_path(parent_path);
    matches!(
        (normalized_parent.as_str(), key),
        (
            "formula",
            "effects"
                | "resourceUse"
                | "segments"
                | "template"
                | "timeline"
                | "tracks"
                | "unsupportedBlocks"
                | "textStyleFallback"
        ) | ("formula.template", "id" | "name")
            | ("formula.timeline", "segments" | "tracks")
            | ("formula.timeline.tracks[]", "id" | "type")
            | (
                "formula.timeline.segments[]",
                "durationMs"
                    | "effects"
                    | "id"
                    | "materialRef"
                    | "sourceDurationMs"
                    | "sourceStartMs"
                    | "targetStartMs"
                    | "trackId"
            )
            | ("formula.tracks[]", "id" | "type")
            | (
                "formula.segments[]",
                "durationMs"
                    | "effects"
                    | "id"
                    | "materialRef"
                    | "sourceDurationMs"
                    | "sourceStartMs"
                    | "targetStartMs"
                    | "trackId"
            )
    )
}

fn normalize_formula_path(path: &str) -> String {
    let mut normalized = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '[' {
            normalized.push_str("[]");
            for next in chars.by_ref() {
                if next == ']' {
                    break;
                }
            }
        } else {
            normalized.push(character);
        }
    }
    normalized
}

fn is_native_effect_evidence(effect: &Value) -> bool {
    effect
        .get("requiresNativeEffect")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || effect.get("nativeEffectId").is_some()
}

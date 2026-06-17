use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{FormulaResourceRef, KaipaiFormulaBundle};

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
    generated_at: impl Into<String>,
) -> CompatibilityReport {
    let mut items = Vec::new();

    collect_degraded_text_style_items(&bundle.formula, &mut items);
    collect_unsupported_formula_block_items(&bundle.formula, &mut items);
    collect_missing_resource_items(&bundle.resources, &mut items);
    collect_native_effect_items(&bundle.formula, &mut items);

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
        source_kind: "kaipaiFormulaBundle".to_owned(),
        source_id: format!("template:{}", bundle.provenance.template_id),
        generated_at: generated_at.into(),
        summary: CompatibilityReportSummary::from_items(&items),
        items,
        provenance_digest: Some("sha256:redacted-compatibility-fixture".to_owned()),
    }
}

fn collect_degraded_text_style_items(formula: &Value, items: &mut Vec<CompatibilityReportItem>) {
    if formula.get("textStyleFallback").is_some() {
        items.push(CompatibilityReportItem {
            status: CompatibilityStatus::Degraded,
            severity: CompatibilitySeverity::Warning,
            category: CompatibilityCategory::Text,
            external_path: "formula.textStyleFallback".to_owned(),
            external_id: None,
            canonical_target: Some(CompatibilityCanonicalTarget::Text),
            message: "Text style has unsupported provider-specific attributes and will use a simpler draft text style.".to_owned(),
            details: Some("Preserves text content and basic style only.".to_owned()),
        });
    }
}

fn collect_unsupported_formula_block_items(
    formula: &Value,
    items: &mut Vec<CompatibilityReportItem>,
) {
    let Some(blocks) = formula.get("unsupportedBlocks").and_then(Value::as_array) else {
        return;
    };

    for (index, block) in blocks.iter().enumerate() {
        let name = block.as_str().unwrap_or("unknownProviderBlock");
        items.push(CompatibilityReportItem {
            status: CompatibilityStatus::Unsupported,
            severity: CompatibilitySeverity::Error,
            category: CompatibilityCategory::Formula,
            external_path: format!("formula.unsupportedBlocks[{index}]"),
            external_id: Some(name.to_owned()),
            canonical_target: None,
            message: "Formula block has no supported Jianying-style draft semantic target yet."
                .to_owned(),
            details: Some("Mapper work must not claim support for this provider block.".to_owned()),
        });
    }
}

fn collect_missing_resource_items(
    resources: &[FormulaResourceRef],
    items: &mut Vec<CompatibilityReportItem>,
) {
    for (index, resource) in resources.iter().enumerate() {
        if !is_missing_resource_evidence(resource) {
            continue;
        }

        items.push(CompatibilityReportItem {
            status: CompatibilityStatus::MissingResource,
            severity: CompatibilitySeverity::Error,
            category: CompatibilityCategory::Resource,
            external_path: format!("resources[{index}]"),
            external_id: Some(resource.resource_id.clone()),
            canonical_target: None,
            message: "Referenced resource is not available in the offline formula bundle."
                .to_owned(),
            details: Some(resource.uri.clone()),
        });
    }
}

fn collect_native_effect_items(formula: &Value, items: &mut Vec<CompatibilityReportItem>) {
    let Some(effects) = formula.get("effects").and_then(Value::as_array) else {
        return;
    };

    for (index, effect) in effects.iter().enumerate() {
        if !is_native_effect_evidence(effect) {
            continue;
        }

        let external_id = effect
            .get("nativeEffectId")
            .or_else(|| effect.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("nativeEffect");
        items.push(CompatibilityReportItem {
            status: CompatibilityStatus::NeedsNativeEffect,
            severity: CompatibilitySeverity::Warning,
            category: CompatibilityCategory::NativeEffect,
            external_path: format!("formula.effects[{index}]"),
            external_id: Some(external_id.to_owned()),
            canonical_target: None,
            message: "Provider-native effect requires explicit compatibility handling before it can be represented locally.".to_owned(),
            details: Some("Do not smuggle native effect data into generic filter parameters.".to_owned()),
        });
    }
}

fn is_missing_resource_evidence(resource: &FormulaResourceRef) -> bool {
    resource.resource_id.starts_with("missing-") || resource.uri.contains("/missing/")
}

fn is_native_effect_evidence(effect: &Value) -> bool {
    effect
        .get("requiresNativeEffect")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || effect.get("nativeEffectId").is_some()
}

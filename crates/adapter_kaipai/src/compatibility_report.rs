use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

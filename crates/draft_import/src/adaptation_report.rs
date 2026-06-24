use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS,
)]
pub struct AdaptationReportSchemaVersion(pub u32);

impl AdaptationReportSchemaVersion {
    pub const CURRENT_VALUE: u32 = 1;
    pub const CURRENT: Self = Self(Self::CURRENT_VALUE);

    pub fn current() -> Self {
        Self::CURRENT
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdaptationReport {
    pub schema_version: AdaptationReportSchemaVersion,
    pub source_kind: String,
    pub generated_at: String,
    pub summary: AdaptationReportSummary,
    pub items: Vec<AdaptationReportItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub provenance_digest: Option<String>,
}

impl AdaptationReport {
    pub fn new(
        source_kind: impl Into<String>,
        generated_at: impl Into<String>,
        items: Vec<AdaptationReportItem>,
    ) -> Self {
        Self {
            schema_version: AdaptationReportSchemaVersion::current(),
            source_kind: source_kind.into(),
            generated_at: generated_at.into(),
            summary: AdaptationReportSummary::from_items(&items),
            items,
            provenance_digest: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdaptationReportSummary {
    pub supported: u32,
    pub approximated: u32,
    pub dropped: u32,
    pub missing_resource: u32,
    pub needs_native_effect: u32,
}

impl AdaptationReportSummary {
    pub fn from_items(items: &[AdaptationReportItem]) -> Self {
        let mut summary = Self {
            supported: 0,
            approximated: 0,
            dropped: 0,
            missing_resource: 0,
            needs_native_effect: 0,
        };

        for item in items {
            match item.status {
                AdaptationStatus::Supported => summary.supported += 1,
                AdaptationStatus::Approximated => summary.approximated += 1,
                AdaptationStatus::Dropped => summary.dropped += 1,
                AdaptationStatus::MissingResource => summary.missing_resource += 1,
                AdaptationStatus::NeedsNativeEffect => summary.needs_native_effect += 1,
            }
        }

        summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdaptationReportItem {
    pub status: AdaptationStatus,
    pub severity: AdaptationSeverity,
    pub category: AdaptationCategory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub target: Option<AdaptationTargetRef>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub details: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<ExternalProvenanceRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum AdaptationStatus {
    Supported,
    Approximated,
    Dropped,
    MissingResource,
    NeedsNativeEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum AdaptationSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum AdaptationCategory {
    SourceMedia,
    Canvas,
    Material,
    Track,
    Segment,
    Text,
    Sticker,
    Audio,
    Animation,
    Transition,
    Resource,
    Font,
    NativeEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdaptationTargetRef {
    pub kind: AdaptationTargetKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum AdaptationTargetKind {
    Draft,
    Canvas,
    Material,
    Track,
    Segment,
    Text,
    Sticker,
    Audio,
    Keyframe,
    Filter,
    Transition,
    Resource,
    Font,
    Effect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalProvenanceRef {
    pub source_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub external_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub external_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub note: Option<String>,
}

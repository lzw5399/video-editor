use draft_model::{Draft, DraftCanvasConfig, DraftId, Material, Track};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{AdaptationReport, AdaptationReportItem};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS,
)]
pub struct DraftImportPlanSchemaVersion(pub u32);

impl DraftImportPlanSchemaVersion {
    pub const CURRENT_VALUE: u32 = 1;
    pub const CURRENT: Self = Self(Self::CURRENT_VALUE);

    pub fn current() -> Self {
        Self::CURRENT
    }

    pub fn is_current(self) -> bool {
        self == Self::CURRENT
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftImportPlan {
    pub schema_version: DraftImportPlanSchemaVersion,
    pub import_id: String,
    pub draft_id: DraftId,
    pub draft_name: String,
    pub canvas_config: DraftCanvasConfig,
    pub materials: Vec<ImportMaterialPlan>,
    pub tracks: Vec<ImportTrackPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportMaterialPlan {
    pub material: Material,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportTrackPlan {
    pub z_order: i32,
    pub track: Track,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftImportApplicationInput {
    pub plan: DraftImportPlan,
    pub source_kind: String,
    pub generated_at: String,
    pub report_items: Vec<AdaptationReportItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftImportApplicationResult {
    pub draft: Draft,
    pub report: AdaptationReport,
}

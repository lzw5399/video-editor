use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{DraftCanvasConfig, DraftId, Material, Track};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
pub struct DraftSchemaVersion(pub u32);

impl DraftSchemaVersion {
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
pub struct DraftMetadata {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub description: Option<String>,
}

impl DraftMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub schema_version: DraftSchemaVersion,
    pub draft_id: DraftId,
    pub metadata: DraftMetadata,
    pub canvas_config: DraftCanvasConfig,
    pub materials: Vec<Material>,
    pub tracks: Vec<Track>,
}

impl Draft {
    pub fn new(draft_id: impl Into<DraftId>, name: impl Into<String>) -> Self {
        Self {
            schema_version: DraftSchemaVersion::current(),
            draft_id: draft_id.into(),
            metadata: DraftMetadata::new(name),
            canvas_config: DraftCanvasConfig::mvp_default(),
            materials: Vec::new(),
            tracks: Vec::new(),
        }
    }
}

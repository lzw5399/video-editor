use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Draft, DraftValidationError, MaterialId, Microseconds, validate_draft};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum MaterialKind {
    Video,
    Image,
    Audio,
    Text,
    Sticker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum MaterialStatus {
    Available,
    Missing,
    ProbeFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RationalFrameRate {
    pub numerator: u32,
    pub denominator: u32,
}

impl RationalFrameRate {
    pub fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MaterialMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub duration: Option<Microseconds>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub frame_rate: Option<RationalFrameRate>,
    pub has_video: bool,
    pub has_audio: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub audio_sample_rate: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub audio_channels: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub probe_error: Option<String>,
}

impl MaterialMetadata {
    pub fn empty() -> Self {
        Self {
            duration: None,
            width: None,
            height: None,
            frame_rate: None,
            has_video: false,
            has_audio: false,
            audio_sample_rate: None,
            audio_channels: None,
            probe_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Material {
    pub material_id: MaterialId,
    pub kind: MaterialKind,
    pub uri: String,
    pub display_name: String,
    pub metadata: MaterialMetadata,
    pub status: MaterialStatus,
}

impl Material {
    pub fn new(
        material_id: impl Into<MaterialId>,
        kind: MaterialKind,
        uri: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            material_id: material_id.into(),
            kind,
            uri: uri.into(),
            display_name: display_name.into(),
            metadata: MaterialMetadata::empty(),
            status: MaterialStatus::Available,
        }
    }
}

pub fn add_material(draft: &mut Draft, material: Material) -> Result<(), DraftValidationError> {
    let original_materials = draft.materials.clone();
    draft.materials.push(material);

    if let Err(error) = validate_draft(draft) {
        draft.materials = original_materials;
        return Err(error);
    }

    Ok(())
}

pub fn upsert_material(draft: &mut Draft, material: Material) -> Result<(), DraftValidationError> {
    let original_materials = draft.materials.clone();

    if let Some(existing) = draft
        .materials
        .iter_mut()
        .find(|existing| existing.material_id == material.material_id)
    {
        *existing = material;
    } else {
        draft.materials.push(material);
    }

    if let Err(error) = validate_draft(draft) {
        draft.materials = original_materials;
        return Err(error);
    }

    Ok(())
}

pub fn mark_material_missing(
    draft: &mut Draft,
    material_id: &MaterialId,
    probe_error: impl Into<String>,
) -> Result<(), DraftValidationError> {
    update_material_status(
        draft,
        material_id,
        MaterialStatus::Missing,
        Some(probe_error.into()),
    )
}

pub fn mark_material_probe_failed(
    draft: &mut Draft,
    material_id: &MaterialId,
    probe_error: impl Into<String>,
) -> Result<(), DraftValidationError> {
    update_material_status(
        draft,
        material_id,
        MaterialStatus::ProbeFailed,
        Some(probe_error.into()),
    )
}

pub fn mark_material_available(
    draft: &mut Draft,
    material_id: &MaterialId,
) -> Result<(), DraftValidationError> {
    update_material_status(draft, material_id, MaterialStatus::Available, None)
}

fn update_material_status(
    draft: &mut Draft,
    material_id: &MaterialId,
    status: MaterialStatus,
    probe_error: Option<String>,
) -> Result<(), DraftValidationError> {
    let original_materials = draft.materials.clone();
    let Some(material) = draft
        .materials
        .iter_mut()
        .find(|material| &material.material_id == material_id)
    else {
        return Err(DraftValidationError::MissingRequiredSemanticField {
            field: format!("materials[].materialId {}", material_id.as_str()),
        });
    };

    material.status = status;
    material.metadata.probe_error = probe_error;

    if let Err(error) = validate_draft(draft) {
        draft.materials = original_materials;
        return Err(error);
    }

    Ok(())
}

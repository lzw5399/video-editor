use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use draft_model::{
    AudioEffectSlotKind, Draft, DraftMetadata, DraftSchemaVersion, DraftValidationError, Filter,
    Segment, TextBubbleRef, TextEffectRef, TextSegment, Track, validate_draft,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    AdaptationReport, DraftImportApplicationInput, DraftImportApplicationResult, DraftImportPlan,
    DraftImportPlanSchemaVersion,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DraftImportPlanValidationError {
    InvalidSchemaVersion { found: String, expected: u32 },
    MissingRequiredSemanticField { field: String },
    RemoteRuntimeRef { field: String, value: String },
    ProviderSemanticLeakage { field: String, reason: String },
    InvalidTrackOrdering { field: String, reason: String },
    InvalidCanonicalDraft { source: DraftValidationError },
}

impl fmt::Display for DraftImportPlanValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSchemaVersion { found, expected } => {
                write!(
                    formatter,
                    "invalid draft import plan schema version {found}; expected {expected}"
                )
            }
            Self::MissingRequiredSemanticField { field } => {
                write!(formatter, "missing required import plan field {field}")
            }
            Self::RemoteRuntimeRef { field, value } => {
                write!(formatter, "remote runtime ref in {field}: {value}")
            }
            Self::ProviderSemanticLeakage { field, reason } => {
                write!(formatter, "provider semantic leakage in {field}: {reason}")
            }
            Self::InvalidTrackOrdering { field, reason } => {
                write!(formatter, "invalid import track ordering {field}: {reason}")
            }
            Self::InvalidCanonicalDraft { source } => {
                write!(
                    formatter,
                    "invalid canonical draft from import plan: {source}"
                )
            }
        }
    }
}

impl Error for DraftImportPlanValidationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidCanonicalDraft { source } => Some(source),
            _ => None,
        }
    }
}

pub fn validate_import_plan(plan: &DraftImportPlan) -> Result<(), DraftImportPlanValidationError> {
    if !plan.schema_version.is_current() {
        return Err(DraftImportPlanValidationError::InvalidSchemaVersion {
            found: plan.schema_version.0.to_string(),
            expected: DraftImportPlanSchemaVersion::CURRENT_VALUE,
        });
    }
    validate_required_text("importId", &plan.import_id)?;
    if plan.draft_id.is_empty() {
        return Err(missing_field("draftId"));
    }
    validate_required_text("draftName", &plan.draft_name)?;
    validate_track_order(plan)?;

    for (index, material_plan) in plan.materials.iter().enumerate() {
        let field = format!("materials[{index}].material.uri");
        reject_remote_runtime_ref(&field, &material_plan.material.uri)?;
        reject_provider_semantic_text(&field, &material_plan.material.uri)?;
        let display_name_field = format!("materials[{index}].material.displayName");
        reject_provider_semantic_text(&display_name_field, &material_plan.material.display_name)?;
    }

    for (track_index, track_plan) in plan.tracks.iter().enumerate() {
        validate_track_semantics(track_index, &track_plan.track)?;
    }

    let draft = draft_from_import_plan(plan);
    validate_draft(&draft)
        .map_err(|source| DraftImportPlanValidationError::InvalidCanonicalDraft { source })?;
    Ok(())
}

pub fn apply_import_plan_to_draft(
    input: DraftImportApplicationInput,
) -> Result<DraftImportApplicationResult, DraftImportPlanValidationError> {
    validate_import_plan(&input.plan)?;
    Ok(DraftImportApplicationResult {
        draft: draft_from_import_plan(&input.plan),
        report: AdaptationReport::new(input.source_kind, input.generated_at, input.report_items),
    })
}

fn draft_from_import_plan(plan: &DraftImportPlan) -> Draft {
    Draft {
        schema_version: DraftSchemaVersion::current(),
        draft_id: plan.draft_id.clone(),
        metadata: DraftMetadata::new(plan.draft_name.clone()),
        canvas_config: plan.canvas_config.clone(),
        materials: plan
            .materials
            .iter()
            .map(|material| material.material.clone())
            .collect(),
        tracks: plan
            .tracks
            .iter()
            .map(|track| track.track.clone())
            .collect(),
    }
}

fn validate_track_order(plan: &DraftImportPlan) -> Result<(), DraftImportPlanValidationError> {
    let mut seen = BTreeSet::new();
    let mut previous = None;
    for (index, track_plan) in plan.tracks.iter().enumerate() {
        if !seen.insert(track_plan.z_order) {
            return Err(DraftImportPlanValidationError::InvalidTrackOrdering {
                field: format!("tracks[{index}].zOrder"),
                reason: "z-order values must be unique".to_owned(),
            });
        }
        if let Some(previous) = previous
            && track_plan.z_order <= previous
        {
            return Err(DraftImportPlanValidationError::InvalidTrackOrdering {
                field: format!("tracks[{index}].zOrder"),
                reason: "tracks must be sorted by ascending z-order".to_owned(),
            });
        }
        previous = Some(track_plan.z_order);
    }
    Ok(())
}

fn validate_track_semantics(
    track_index: usize,
    track: &Track,
) -> Result<(), DraftImportPlanValidationError> {
    reject_provider_semantic_text(&format!("tracks[{track_index}].name"), &track.name)?;
    for (segment_index, segment) in track.segments.iter().enumerate() {
        validate_segment_semantics(track_index, segment_index, segment)?;
    }
    Ok(())
}

fn validate_segment_semantics(
    track_index: usize,
    segment_index: usize,
    segment: &Segment,
) -> Result<(), DraftImportPlanValidationError> {
    let prefix = format!("tracks[{track_index}].segments[{segment_index}]");
    for (filter_index, filter) in segment.filters.iter().enumerate() {
        validate_filter_semantics(&format!("{prefix}.filters[{filter_index}]"), filter)?;
    }
    if let Some(text) = &segment.text {
        validate_text_semantics(&format!("{prefix}.text"), text)?;
    }
    for (slot_index, slot) in segment.audio.effect_slots.iter().enumerate() {
        match &slot.kind {
            AudioEffectSlotKind::Unsupported { name, external_ref } => {
                reject_provider_semantic_text(
                    &format!("{prefix}.audio.effectSlots[{slot_index}].kind.name"),
                    name,
                )?;
                reject_external_ref(
                    &format!("{prefix}.audio.effectSlots[{slot_index}].kind.externalRef"),
                    external_ref,
                )?;
            }
        }
    }
    Ok(())
}

fn validate_filter_semantics(
    field: &str,
    filter: &Filter,
) -> Result<(), DraftImportPlanValidationError> {
    reject_provider_semantic_text(&format!("{field}.name"), &filter.name)?;
    for (key, value) in &filter.parameters {
        reject_provider_semantic_text(&format!("{field}.parameters.{key}"), key)?;
        reject_provider_semantic_text(&format!("{field}.parameters.{key}"), value)?;
    }
    Ok(())
}

fn validate_text_semantics(
    field: &str,
    text: &TextSegment,
) -> Result<(), DraftImportPlanValidationError> {
    if let Some(font_ref) = &text.style.font.font_ref {
        reject_remote_runtime_ref(&format!("{field}.style.font.fontRef"), font_ref)?;
        reject_provider_semantic_text(&format!("{field}.style.font.fontRef"), font_ref)?;
    }
    if let Some(bubble) = &text.bubble {
        match bubble {
            TextBubbleRef::Unsupported { name, external_ref } => {
                reject_provider_semantic_text(&format!("{field}.bubble.name"), name)?;
                reject_external_ref(&format!("{field}.bubble.externalRef"), external_ref)?;
            }
        }
    }
    if let Some(effect) = &text.effect {
        match effect {
            TextEffectRef::Unsupported { name, external_ref } => {
                reject_provider_semantic_text(&format!("{field}.effect.name"), name)?;
                reject_external_ref(&format!("{field}.effect.externalRef"), external_ref)?;
            }
        }
    }
    Ok(())
}

fn reject_external_ref(
    field: &str,
    external_ref: &Option<String>,
) -> Result<(), DraftImportPlanValidationError> {
    if let Some(external_ref) = external_ref {
        return Err(DraftImportPlanValidationError::ProviderSemanticLeakage {
            field: field.to_owned(),
            reason: format!(
                "external provider references belong in AdaptationReport provenance, not canonical import semantics: {external_ref}"
            ),
        });
    }
    Ok(())
}

fn reject_remote_runtime_ref(
    field: &str,
    value: &str,
) -> Result<(), DraftImportPlanValidationError> {
    let lower = value.trim().to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Err(DraftImportPlanValidationError::RemoteRuntimeRef {
            field: field.to_owned(),
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn reject_provider_semantic_text(
    field: &str,
    value: &str,
) -> Result<(), DraftImportPlanValidationError> {
    let normalized = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    for forbidden in [
        "templateid",
        "recipeid",
        "rawformula",
        "formula",
        "safearea",
        "provenance",
        "provider",
        "kaipai",
        "androidworker",
    ] {
        if normalized.contains(forbidden) {
            return Err(DraftImportPlanValidationError::ProviderSemanticLeakage {
                field: field.to_owned(),
                reason: format!("{forbidden} is adapter/report evidence, not draft semantics"),
            });
        }
    }
    Ok(())
}

fn validate_required_text(field: &str, value: &str) -> Result<(), DraftImportPlanValidationError> {
    if value.trim().is_empty() {
        return Err(missing_field(field));
    }
    Ok(())
}

fn missing_field(field: &str) -> DraftImportPlanValidationError {
    DraftImportPlanValidationError::MissingRequiredSemanticField {
        field: field.to_owned(),
    }
}

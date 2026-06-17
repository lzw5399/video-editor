use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    CanvasAspectRatio, CanvasBackground, Draft, DraftSchemaVersion, MAX_SEGMENT_VOLUME_MILLIS,
    MaterialKind, Microseconds, RationalFrameRate, SourceTimerange, TargetTimerange, TextSegment,
    reduce_ratio,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DraftValidationError {
    InvalidSchemaVersion { found: String, expected: u32 },
    MissingRequiredSemanticField { field: String },
    InvalidTimerange { field: String, reason: String },
    InvalidRationalFrameRate { field: String, reason: String },
    InvalidCanvasConfig { field: String, reason: String },
    DuplicateId { id_kind: String, id: String },
    DerivedArtifactLeakage { field: String },
    InvalidDraftJson { message: String },
}

impl fmt::Display for DraftValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSchemaVersion { found, expected } => {
                write!(
                    formatter,
                    "invalid draft schema version {found}; expected {expected}"
                )
            }
            Self::MissingRequiredSemanticField { field } => {
                write!(formatter, "missing required semantic field {field}")
            }
            Self::InvalidTimerange { field, reason } => {
                write!(formatter, "invalid timerange {field}: {reason}")
            }
            Self::InvalidRationalFrameRate { field, reason } => {
                write!(formatter, "invalid rational frame rate {field}: {reason}")
            }
            Self::InvalidCanvasConfig { field, reason } => {
                write!(formatter, "invalid canvas config {field}: {reason}")
            }
            Self::DuplicateId { id_kind, id } => {
                write!(formatter, "duplicate {id_kind} id {id}")
            }
            Self::DerivedArtifactLeakage { field } => {
                write!(
                    formatter,
                    "derived artifact field leaked into draft: {field}"
                )
            }
            Self::InvalidDraftJson { message } => {
                write!(formatter, "invalid draft JSON: {message}")
            }
        }
    }
}

impl Error for DraftValidationError {}

pub fn migrate_draft_json(value: serde_json::Value) -> Result<Draft, DraftValidationError> {
    reject_derived_artifact_fields(&value)?;

    let schema_version = value
        .get("schemaVersion")
        .ok_or_else(|| missing_field("schemaVersion"))?;
    let version = schema_version_u32(schema_version)?;
    if version != DraftSchemaVersion::CURRENT_VALUE {
        return Err(DraftValidationError::InvalidSchemaVersion {
            found: version.to_string(),
            expected: DraftSchemaVersion::CURRENT_VALUE,
        });
    }

    for field in ["draftId", "metadata", "canvasConfig", "materials", "tracks"] {
        if !value.get(field).is_some() {
            return Err(missing_field(field));
        }
    }

    let draft: Draft =
        serde_json::from_value(value).map_err(|error| DraftValidationError::InvalidDraftJson {
            message: error.to_string(),
        })?;
    validate_draft(&draft)?;
    Ok(draft)
}

pub fn validate_draft(draft: &Draft) -> Result<(), DraftValidationError> {
    if !draft.schema_version.is_current() {
        return Err(DraftValidationError::InvalidSchemaVersion {
            found: draft.schema_version.0.to_string(),
            expected: DraftSchemaVersion::CURRENT_VALUE,
        });
    }
    if draft.draft_id.is_empty() {
        return Err(missing_field("draftId"));
    }
    if draft.metadata.name.trim().is_empty() {
        return Err(missing_field("metadata.name"));
    }

    validate_canvas_config(draft)?;

    let mut material_ids = BTreeSet::new();
    for material in &draft.materials {
        if material.material_id.is_empty() {
            return Err(missing_field("materials[].materialId"));
        }
        if !material_ids.insert(material.material_id.as_str().to_owned()) {
            return Err(duplicate_id("materialId", material.material_id.as_str()));
        }
        if material.uri.trim().is_empty() {
            return Err(missing_field("materials[].uri"));
        }
        if material.display_name.trim().is_empty() {
            return Err(missing_field("materials[].displayName"));
        }
        if let Some(frame_rate) = &material.metadata.frame_rate {
            validate_frame_rate("materials[].metadata.frameRate", frame_rate)?;
        }
    }

    let mut track_ids = BTreeSet::new();
    let mut segment_ids = BTreeSet::new();
    for track in &draft.tracks {
        if track.track_id.is_empty() {
            return Err(missing_field("tracks[].trackId"));
        }
        if !track_ids.insert(track.track_id.as_str().to_owned()) {
            return Err(duplicate_id("trackId", track.track_id.as_str()));
        }
        if track.name.trim().is_empty() {
            return Err(missing_field("tracks[].name"));
        }

        for segment in &track.segments {
            if segment.segment_id.is_empty() {
                return Err(missing_field("tracks[].segments[].segmentId"));
            }
            if !segment_ids.insert(segment.segment_id.as_str().to_owned()) {
                return Err(duplicate_id("segmentId", segment.segment_id.as_str()));
            }
            if !material_ids.contains(segment.material_id.as_str()) {
                return Err(DraftValidationError::MissingRequiredSemanticField {
                    field: format!(
                        "tracks[].segments[].materialId references {}",
                        segment.material_id.as_str()
                    ),
                });
            }

            validate_source_timerange(
                "tracks[].segments[].sourceTimerange",
                &segment.source_timerange,
            )?;
            validate_target_timerange(
                "tracks[].segments[].targetTimerange",
                &segment.target_timerange,
            )?;

            for keyframe in &segment.keyframes {
                if keyframe.property.trim().is_empty() {
                    return Err(missing_field("tracks[].segments[].keyframes[].property"));
                }
                if keyframe.value.trim().is_empty() {
                    return Err(missing_field("tracks[].segments[].keyframes[].value"));
                }
            }
            for filter in &segment.filters {
                if filter.name.trim().is_empty() {
                    return Err(missing_field("tracks[].segments[].filters[].name"));
                }
            }
            if let Some(transition) = &segment.transition {
                if transition.name.trim().is_empty() {
                    return Err(missing_field("tracks[].segments[].transition.name"));
                }
                validate_duration(
                    "tracks[].segments[].transition.duration",
                    transition.duration,
                )?;
            }
            if let Some(text) = &segment.text {
                validate_text_segment("tracks[].segments[].text", text)?;
            }
            validate_segment_volume("tracks[].segments[].volume", segment.volume.level_millis)?;
        }
    }

    Ok(())
}

fn reject_derived_artifact_fields(value: &serde_json::Value) -> Result<(), DraftValidationError> {
    let Some(object) = value.as_object() else {
        return Err(DraftValidationError::InvalidDraftJson {
            message: "draft JSON must be an object".to_owned(),
        });
    };

    for field in [
        "thumbnails",
        "waveforms",
        "previewCaches",
        "renderGraph",
        "ffmpegScripts",
        "exports",
        "rawProbeJson",
    ] {
        if object.contains_key(field) {
            return Err(DraftValidationError::DerivedArtifactLeakage {
                field: field.to_owned(),
            });
        }
    }

    Ok(())
}

fn schema_version_u32(value: &serde_json::Value) -> Result<u32, DraftValidationError> {
    let Some(version) = value.as_u64() else {
        return Err(DraftValidationError::InvalidSchemaVersion {
            found: value.to_string(),
            expected: DraftSchemaVersion::CURRENT_VALUE,
        });
    };
    u32::try_from(version).map_err(|_| DraftValidationError::InvalidSchemaVersion {
        found: version.to_string(),
        expected: DraftSchemaVersion::CURRENT_VALUE,
    })
}

fn validate_source_timerange(
    field: &str,
    timerange: &SourceTimerange,
) -> Result<(), DraftValidationError> {
    validate_duration(&format!("{field}.duration"), timerange.duration)
}

fn validate_target_timerange(
    field: &str,
    timerange: &TargetTimerange,
) -> Result<(), DraftValidationError> {
    validate_duration(&format!("{field}.duration"), timerange.duration)
}

fn validate_duration(field: &str, duration: Microseconds) -> Result<(), DraftValidationError> {
    if duration.get() == 0 {
        return Err(DraftValidationError::InvalidTimerange {
            field: field.to_owned(),
            reason: "duration must be greater than zero microseconds".to_owned(),
        });
    }
    Ok(())
}

fn validate_frame_rate(
    field: &str,
    frame_rate: &RationalFrameRate,
) -> Result<(), DraftValidationError> {
    if frame_rate.denominator == 0 {
        return Err(DraftValidationError::InvalidRationalFrameRate {
            field: format!("{field}.denominator"),
            reason: "denominator must be greater than zero".to_owned(),
        });
    }
    if frame_rate.numerator == 0 {
        return Err(DraftValidationError::InvalidRationalFrameRate {
            field: format!("{field}.numerator"),
            reason: "numerator must be greater than zero".to_owned(),
        });
    }
    Ok(())
}

fn validate_canvas_config(draft: &Draft) -> Result<(), DraftValidationError> {
    let canvas = &draft.canvas_config;
    if canvas.width == 0 {
        return Err(invalid_canvas(
            "canvasConfig.width",
            "width must be greater than zero",
        ));
    }
    if canvas.height == 0 {
        return Err(invalid_canvas(
            "canvasConfig.height",
            "height must be greater than zero",
        ));
    }

    validate_frame_rate("canvasConfig.frameRate", &canvas.frame_rate)?;
    validate_canvas_aspect_ratio(
        "canvasConfig.aspectRatio",
        &canvas.aspect_ratio,
        canvas.width,
        canvas.height,
    )?;
    validate_canvas_background(draft, "canvasConfig.background", &canvas.background)
}

fn validate_canvas_aspect_ratio(
    field: &str,
    aspect_ratio: &CanvasAspectRatio,
    width: u32,
    height: u32,
) -> Result<(), DraftValidationError> {
    let Some(canvas_ratio) = reduce_ratio(width, height) else {
        return Err(invalid_canvas(
            field,
            "canvas dimensions must have a non-zero ratio",
        ));
    };
    let Some(expected_ratio) = aspect_ratio.ratio() else {
        return Err(invalid_canvas(
            field,
            "custom aspect ratio numerator and denominator must be greater than zero",
        ));
    };
    if canvas_ratio != expected_ratio {
        return Err(invalid_canvas(
            field,
            &format!(
                "aspect ratio {}:{} does not match canvas dimensions {}:{}",
                expected_ratio.0, expected_ratio.1, canvas_ratio.0, canvas_ratio.1
            ),
        ));
    }
    Ok(())
}

fn validate_canvas_background(
    draft: &Draft,
    field: &str,
    background: &CanvasBackground,
) -> Result<(), DraftValidationError> {
    match background {
        CanvasBackground::Black | CanvasBackground::BlurFill => Ok(()),
        CanvasBackground::SolidColor { color } => {
            validate_hex_color(&format!("{field}.color"), color)
        }
        CanvasBackground::Image { material_id } => {
            let Some(material_id) = material_id else {
                return Ok(());
            };
            let Some(material) = draft
                .materials
                .iter()
                .find(|material| &material.material_id == material_id)
            else {
                return Err(DraftValidationError::MissingRequiredSemanticField {
                    field: format!("{field}.materialId references {}", material_id.as_str()),
                });
            };
            if material.kind != MaterialKind::Image {
                return Err(invalid_canvas(
                    &format!("{field}.materialId"),
                    "image background material must reference an image material",
                ));
            }
            Ok(())
        }
    }
}

fn validate_hex_color(field: &str, color: &str) -> Result<(), DraftValidationError> {
    let color = color.trim();
    if color.len() != 7 || !color.starts_with('#') {
        return Err(invalid_canvas(field, "color must use #RRGGBB hex format"));
    }
    if !color[1..]
        .chars()
        .all(|character| character.is_ascii_hexdigit())
    {
        return Err(invalid_canvas(field, "color must contain only hex digits"));
    }
    Ok(())
}

fn validate_text_segment(field: &str, text: &TextSegment) -> Result<(), DraftValidationError> {
    if text.content.trim().is_empty() {
        return Err(missing_field(&format!("{field}.content")));
    }
    if text.style.font_size == 0 {
        return Err(missing_field(&format!("{field}.style.fontSize")));
    }
    validate_required_text(&format!("{field}.style.color"), &text.style.color)?;
    if let Some(stroke) = &text.style.stroke {
        validate_required_text(&format!("{field}.style.stroke.color"), &stroke.color)?;
        if stroke.width == 0 {
            return Err(missing_field(&format!("{field}.style.stroke.width")));
        }
    }
    if let Some(shadow) = &text.style.shadow {
        validate_required_text(&format!("{field}.style.shadow.color"), &shadow.color)?;
    }
    if let Some(background) = &text.style.background {
        validate_required_text(
            &format!("{field}.style.background.color"),
            &background.color,
        )?;
    }

    Ok(())
}

fn validate_required_text(field: &str, value: &str) -> Result<(), DraftValidationError> {
    if value.trim().is_empty() {
        return Err(missing_field(field));
    }
    Ok(())
}

fn validate_segment_volume(field: &str, level_millis: u32) -> Result<(), DraftValidationError> {
    if level_millis > MAX_SEGMENT_VOLUME_MILLIS {
        return Err(DraftValidationError::MissingRequiredSemanticField {
            field: format!("{field} must be <= {MAX_SEGMENT_VOLUME_MILLIS}"),
        });
    }
    Ok(())
}

fn missing_field(field: &str) -> DraftValidationError {
    DraftValidationError::MissingRequiredSemanticField {
        field: field.to_owned(),
    }
}

fn invalid_canvas(field: &str, reason: &str) -> DraftValidationError {
    DraftValidationError::InvalidCanvasConfig {
        field: field.to_owned(),
        reason: reason.to_owned(),
    }
}

fn duplicate_id(id_kind: &str, id: &str) -> DraftValidationError {
    DraftValidationError::DuplicateId {
        id_kind: id_kind.to_owned(),
        id: id.to_owned(),
    }
}

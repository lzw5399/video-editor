use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    AudioEffectSlotKind, CanvasAspectRatio, CanvasBackground, Draft, DraftSchemaVersion,
    FilterKind, Keyframe, KeyframeProperty, KeyframeValue, MAX_AUDIO_FADE_DURATION_MICROSECONDS,
    MAX_AUDIO_PAN_BALANCE_MILLIS, MAX_SEGMENT_CROP_MILLIS, MAX_SEGMENT_VOLUME_MILLIS,
    MIN_AUDIO_PAN_BALANCE_MILLIS, MaterialId, MaterialKind, Microseconds, RationalFrameRate,
    RetimeMode, SegmentAudio, SegmentBackgroundFilling, SegmentBlendMode, SegmentCrop, SegmentMask,
    SegmentRetiming, SegmentVisual, SourceTimerange, TargetTimerange, TextBox, TextBubbleRef,
    TextEffectRef, TextLayoutRegion, TextSegment, TextStyle, reduce_ratio,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DraftValidationError {
    InvalidSchemaVersion { found: String, expected: u32 },
    MissingRequiredSemanticField { field: String },
    InvalidTimerange { field: String, reason: String },
    InvalidRationalFrameRate { field: String, reason: String },
    InvalidCanvasConfig { field: String, reason: String },
    InvalidSegmentVisual { field: String, reason: String },
    InvalidSegmentAudio { field: String, reason: String },
    InvalidTextSegment { field: String, reason: String },
    InvalidKeyframe { field: String, reason: String },
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
            Self::InvalidSegmentVisual { field, reason } => {
                write!(formatter, "invalid segment visual {field}: {reason}")
            }
            Self::InvalidSegmentAudio { field, reason } => {
                write!(formatter, "invalid segment audio {field}: {reason}")
            }
            Self::InvalidTextSegment { field, reason } => {
                write!(formatter, "invalid text segment {field}: {reason}")
            }
            Self::InvalidKeyframe { field, reason } => {
                write!(formatter, "invalid keyframe {field}: {reason}")
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
            validate_segment_retiming("tracks[].segments[].retiming", &segment.retiming)?;

            validate_keyframes(
                "tracks[].segments[].keyframes",
                &segment.keyframes,
                segment.target_timerange.duration,
            )?;
            for filter in &segment.filters {
                if let FilterKind::ExternalReference { reference } = &filter.kind {
                    validate_required_text(
                        "tracks[].segments[].filters[].kind.reference.provider",
                        &reference.provider,
                    )?;
                    validate_required_text(
                        "tracks[].segments[].filters[].kind.reference.effectId",
                        &reference.effect_id,
                    )?;
                }
            }
            if let Some(transition) = &segment.transition {
                if let Some(reference) = transition.external() {
                    validate_required_text(
                        "tracks[].segments[].transition.reference.reference.provider",
                        &reference.provider,
                    )?;
                    validate_required_text(
                        "tracks[].segments[].transition.reference.reference.effectId",
                        &reference.effect_id,
                    )?;
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
            validate_segment_audio(
                "tracks[].segments[].audio",
                &segment.audio,
                segment.target_timerange.duration,
            )?;
            validate_segment_visual(draft, "tracks[].segments[].visual", &segment.visual)?;
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

fn validate_keyframes(
    field: &str,
    keyframes: &[Keyframe],
    target_duration: Microseconds,
) -> Result<(), DraftValidationError> {
    let mut seen = BTreeSet::new();

    for keyframe in keyframes {
        let keyframe_field = format!("{field}[]");
        if keyframe.at > target_duration {
            return Err(invalid_keyframe(
                &format!("{keyframe_field}.at"),
                "keyframe time must be within the segment target duration",
            ));
        }
        let key = (keyframe.property.clone(), keyframe.at);
        if !seen.insert(key) {
            return Err(invalid_keyframe(
                &keyframe_field,
                "duplicate keyframe for property and time",
            ));
        }
        validate_keyframe_property_value(&keyframe_field, &keyframe.property, &keyframe.value)?;
    }

    Ok(())
}

fn validate_keyframe_property_value(
    field: &str,
    property: &KeyframeProperty,
    value: &KeyframeValue,
) -> Result<(), DraftValidationError> {
    match property {
        KeyframeProperty::VisualPositionX
        | KeyframeProperty::VisualPositionY
        | KeyframeProperty::VisualRotation => {
            let KeyframeValue::Int { value } = value else {
                return Err(keyframe_type_mismatch(field, property, "int"));
            };
            if matches!(property, KeyframeProperty::VisualRotation)
                && (*value < -360 || *value > 360)
            {
                return Err(invalid_keyframe(
                    &format!("{field}.value"),
                    "visual rotation must be between -360 and 360 degrees",
                ));
            }
            Ok(())
        }
        KeyframeProperty::VisualScaleX | KeyframeProperty::VisualScaleY => {
            let KeyframeValue::Uint { value } = value else {
                return Err(keyframe_type_mismatch(field, property, "uint"));
            };
            if *value == 0 {
                return Err(invalid_keyframe(
                    &format!("{field}.value"),
                    "visual scale must be greater than zero",
                ));
            }
            Ok(())
        }
        KeyframeProperty::VisualOpacity => {
            let KeyframeValue::Uint { value } = value else {
                return Err(keyframe_type_mismatch(field, property, "uint"));
            };
            if *value > crate::MAX_SEGMENT_OPACITY_MILLIS {
                return Err(invalid_keyframe(
                    &format!("{field}.value"),
                    "visual opacity must be <= 1000",
                ));
            }
            Ok(())
        }
        KeyframeProperty::TextFontSize => {
            let KeyframeValue::Uint { value } = value else {
                return Err(keyframe_type_mismatch(field, property, "uint"));
            };
            if *value == 0 {
                return Err(invalid_keyframe(
                    &format!("{field}.value"),
                    "text font size must be greater than zero",
                ));
            }
            Ok(())
        }
        KeyframeProperty::TextColor => {
            let KeyframeValue::Color { value } = value else {
                return Err(keyframe_type_mismatch(field, property, "color"));
            };
            validate_keyframe_hex_color(&format!("{field}.value"), value)
        }
        KeyframeProperty::TextLineHeight => {
            let KeyframeValue::Uint { value } = value else {
                return Err(keyframe_type_mismatch(field, property, "uint"));
            };
            if *value < crate::MIN_TEXT_LINE_HEIGHT_MILLIS
                || *value > crate::MAX_TEXT_LINE_HEIGHT_MILLIS
            {
                return Err(invalid_keyframe(
                    &format!("{field}.value"),
                    "text line height must be between 500 and 3000 millis",
                ));
            }
            Ok(())
        }
        KeyframeProperty::TextLetterSpacing => validate_uint_keyframe_max(
            field,
            property,
            value,
            crate::MAX_TEXT_LETTER_SPACING_MILLIS,
            "text letter spacing must be between 0 and 2000 millis",
        ),
        KeyframeProperty::TextLayoutX | KeyframeProperty::TextLayoutY => {
            validate_uint_keyframe_max(
                field,
                property,
                value,
                crate::MAX_TEXT_LAYOUT_MILLIS,
                "text layout offset must be between 0 and 1000 millis",
            )
        }
        KeyframeProperty::TextLayoutWidth | KeyframeProperty::TextLayoutHeight => {
            let KeyframeValue::Uint { value } = value else {
                return Err(keyframe_type_mismatch(field, property, "uint"));
            };
            if *value == 0 || *value > crate::MAX_TEXT_LAYOUT_MILLIS {
                return Err(invalid_keyframe(
                    &format!("{field}.value"),
                    "text layout size must be between 1 and 1000 millis",
                ));
            }
            Ok(())
        }
        KeyframeProperty::Volume => validate_uint_keyframe_max(
            field,
            property,
            value,
            crate::MAX_SEGMENT_VOLUME_MILLIS,
            "volume must be between 0 and 4000 millis",
        ),
        KeyframeProperty::StickerPositionX
        | KeyframeProperty::StickerPositionY
        | KeyframeProperty::StickerScaleX
        | KeyframeProperty::StickerScaleY => Err(invalid_keyframe(
            &format!("{field}.property"),
            "sticker parameter animation is deferred until sticker semantics are implemented",
        )),
        KeyframeProperty::FilterParameterUnsupported => Err(invalid_keyframe(
            &format!("{field}.property"),
            "filter parameter animation is deferred until filter semantics are implemented",
        )),
    }
}

fn validate_uint_keyframe_max(
    field: &str,
    property: &KeyframeProperty,
    value: &KeyframeValue,
    max: u32,
    reason: &str,
) -> Result<(), DraftValidationError> {
    let KeyframeValue::Uint { value } = value else {
        return Err(keyframe_type_mismatch(field, property, "uint"));
    };
    if *value > max {
        return Err(invalid_keyframe(&format!("{field}.value"), reason));
    }
    Ok(())
}

fn validate_keyframe_hex_color(field: &str, color: &str) -> Result<(), DraftValidationError> {
    let color = color.trim();
    if color.len() != 7 || !color.starts_with('#') {
        return Err(invalid_keyframe(field, "color must use #RRGGBB hex format"));
    }
    if !color[1..]
        .chars()
        .all(|character| character.is_ascii_hexdigit())
    {
        return Err(invalid_keyframe(
            field,
            "color must contain only hex digits",
        ));
    }
    Ok(())
}

fn keyframe_type_mismatch(
    field: &str,
    property: &KeyframeProperty,
    expected: &str,
) -> DraftValidationError {
    invalid_keyframe(
        &format!("{field}.value"),
        &format!("{property:?} requires {expected} keyframe value"),
    )
}

fn validate_text_segment(field: &str, text: &TextSegment) -> Result<(), DraftValidationError> {
    if text.content.trim().is_empty() {
        return Err(missing_field(&format!("{field}.content")));
    }
    validate_text_style(&format!("{field}.style"), &text.style)?;
    validate_text_box(&format!("{field}.textBox"), &text.text_box)?;
    validate_text_layout_region(&format!("{field}.layoutRegion"), &text.layout_region)?;
    if let Some(bubble) = &text.bubble {
        validate_text_bubble_ref(&format!("{field}.bubble"), bubble)?;
    }
    if let Some(effect) = &text.effect {
        validate_text_effect_ref(&format!("{field}.effect"), effect)?;
    }

    Ok(())
}

fn validate_text_style(field: &str, style: &TextStyle) -> Result<(), DraftValidationError> {
    validate_required_text(&format!("{field}.font.family"), &style.font.family)?;
    if let Some(font_ref) = &style.font.font_ref {
        validate_required_text(&format!("{field}.font.fontRef"), font_ref)?;
        if font_ref.starts_with("font://bundled/")
            && crate::resolve_bundled_font(font_ref).is_none()
        {
            return Err(invalid_text_segment(
                &format!("{field}.font.fontRef"),
                "bundled fontRef is not registered",
            ));
        }
    }
    if style.font_size == 0 {
        return Err(invalid_text_segment(
            &format!("{field}.fontSize"),
            "font size must be greater than zero",
        ));
    }
    validate_text_hex_color(&format!("{field}.color"), &style.color)?;
    if style.line_height_millis < crate::MIN_TEXT_LINE_HEIGHT_MILLIS
        || style.line_height_millis > crate::MAX_TEXT_LINE_HEIGHT_MILLIS
    {
        return Err(invalid_text_segment(
            &format!("{field}.lineHeightMillis"),
            &format!(
                "line height must be between {} and {} millis",
                crate::MIN_TEXT_LINE_HEIGHT_MILLIS,
                crate::MAX_TEXT_LINE_HEIGHT_MILLIS
            ),
        ));
    }
    validate_text_millis_range(
        &format!("{field}.letterSpacingMillis"),
        style.letter_spacing_millis,
        crate::MAX_TEXT_LETTER_SPACING_MILLIS,
        "letter spacing must be between 0 and 2000 millis",
    )?;
    if let Some(stroke) = &style.stroke {
        validate_text_hex_color(&format!("{field}.stroke.color"), &stroke.color)?;
        if stroke.width == 0 {
            return Err(invalid_text_segment(
                &format!("{field}.stroke.width"),
                "stroke width must be greater than zero",
            ));
        }
    }
    if let Some(shadow) = &style.shadow {
        validate_text_hex_color(&format!("{field}.shadow.color"), &shadow.color)?;
    }
    if let Some(background) = &style.background {
        validate_text_hex_color(&format!("{field}.background.color"), &background.color)?;
    }

    Ok(())
}

fn validate_text_box(field: &str, text_box: &TextBox) -> Result<(), DraftValidationError> {
    validate_positive_text_millis(&format!("{field}.widthMillis"), text_box.width_millis)?;
    validate_positive_text_millis(&format!("{field}.heightMillis"), text_box.height_millis)
}

fn validate_text_layout_region(
    field: &str,
    layout_region: &TextLayoutRegion,
) -> Result<(), DraftValidationError> {
    for (name, value) in [
        ("xMillis", layout_region.x_millis),
        ("yMillis", layout_region.y_millis),
        ("widthMillis", layout_region.width_millis),
        ("heightMillis", layout_region.height_millis),
    ] {
        validate_text_millis_range(
            &format!("{field}.{name}"),
            value,
            crate::MAX_TEXT_LAYOUT_MILLIS,
            "layout region values must be between 0 and 1000 millis",
        )?;
    }
    validate_positive_text_millis(&format!("{field}.widthMillis"), layout_region.width_millis)?;
    validate_positive_text_millis(
        &format!("{field}.heightMillis"),
        layout_region.height_millis,
    )?;
    if layout_region.x_millis + layout_region.width_millis > crate::MAX_TEXT_LAYOUT_MILLIS {
        return Err(invalid_text_segment(
            field,
            "layout region x plus width must be <= 1000",
        ));
    }
    if layout_region.y_millis + layout_region.height_millis > crate::MAX_TEXT_LAYOUT_MILLIS {
        return Err(invalid_text_segment(
            field,
            "layout region y plus height must be <= 1000",
        ));
    }
    Ok(())
}

fn validate_text_bubble_ref(
    field: &str,
    bubble: &TextBubbleRef,
) -> Result<(), DraftValidationError> {
    match bubble {
        TextBubbleRef::Unsupported { name, external_ref } => {
            validate_required_text(&format!("{field}.name"), name)?;
            validate_optional_external_ref(&format!("{field}.externalRef"), external_ref)
        }
    }
}

fn validate_text_effect_ref(
    field: &str,
    effect: &TextEffectRef,
) -> Result<(), DraftValidationError> {
    match effect {
        TextEffectRef::Unsupported { name, external_ref } => {
            validate_required_text(&format!("{field}.name"), name)?;
            validate_optional_external_ref(&format!("{field}.externalRef"), external_ref)
        }
    }
}

fn validate_optional_external_ref(
    field: &str,
    external_ref: &Option<String>,
) -> Result<(), DraftValidationError> {
    if let Some(external_ref) = external_ref {
        validate_required_text(field, external_ref)?;
    }
    Ok(())
}

fn validate_required_text(field: &str, value: &str) -> Result<(), DraftValidationError> {
    if value.trim().is_empty() {
        return Err(missing_field(field));
    }
    Ok(())
}

fn validate_positive_text_millis(field: &str, value: u32) -> Result<(), DraftValidationError> {
    if value == 0 {
        return Err(invalid_text_segment(
            field,
            "value must be greater than zero millis",
        ));
    }
    validate_text_millis_range(
        field,
        value,
        crate::MAX_TEXT_LAYOUT_MILLIS,
        "value must be <= 1000 millis",
    )
}

fn validate_text_millis_range(
    field: &str,
    value: u32,
    max: u32,
    reason: &str,
) -> Result<(), DraftValidationError> {
    if value > max {
        return Err(invalid_text_segment(field, reason));
    }
    Ok(())
}

fn validate_text_hex_color(field: &str, color: &str) -> Result<(), DraftValidationError> {
    let color = color.trim();
    if color.len() != 7 || !color.starts_with('#') {
        return Err(invalid_text_segment(
            field,
            "color must use #RRGGBB hex format",
        ));
    }
    if !color[1..]
        .chars()
        .all(|character| character.is_ascii_hexdigit())
    {
        return Err(invalid_text_segment(
            field,
            "color must contain only hex digits",
        ));
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

fn validate_segment_audio(
    field: &str,
    audio: &SegmentAudio,
    target_duration: Microseconds,
) -> Result<(), DraftValidationError> {
    if audio.gain_millis > MAX_SEGMENT_VOLUME_MILLIS {
        return Err(invalid_segment_audio(
            &format!("{field}.gainMillis"),
            &format!("gain must be <= {MAX_SEGMENT_VOLUME_MILLIS} millis"),
        ));
    }
    if audio.pan_balance_millis.balance_millis < MIN_AUDIO_PAN_BALANCE_MILLIS
        || audio.pan_balance_millis.balance_millis > MAX_AUDIO_PAN_BALANCE_MILLIS
    {
        return Err(invalid_segment_audio(
            &format!("{field}.panBalanceMillis"),
            &format!(
                "pan balance must be between {MIN_AUDIO_PAN_BALANCE_MILLIS} and {MAX_AUDIO_PAN_BALANCE_MILLIS} millis"
            ),
        ));
    }
    validate_audio_fade(
        &format!("{field}.fadeInDuration"),
        audio.fade_in_duration.duration,
        target_duration,
    )?;
    validate_audio_fade(
        &format!("{field}.fadeOutDuration"),
        audio.fade_out_duration.duration,
        target_duration,
    )?;

    let mut seen_slot_ids = BTreeSet::new();
    for slot in &audio.effect_slots {
        if slot.slot_id.trim().is_empty() {
            return Err(invalid_segment_audio(
                &format!("{field}.effectSlots[].slotId"),
                "effect slot ID must not be empty",
            ));
        }
        if !seen_slot_ids.insert(slot.slot_id.as_str().to_owned()) {
            return Err(invalid_segment_audio(
                &format!("{field}.effectSlots[]"),
                "effect slot IDs must be unique",
            ));
        }
        match &slot.kind {
            AudioEffectSlotKind::Unsupported { name, external_ref } => {
                validate_required_audio_text(&format!("{field}.effectSlots[].kind.name"), name)?;
                validate_optional_audio_external_ref(
                    &format!("{field}.effectSlots[].kind.externalRef"),
                    external_ref,
                )?;
            }
        }
    }

    Ok(())
}

fn validate_audio_fade(
    field: &str,
    duration: Microseconds,
    target_duration: Microseconds,
) -> Result<(), DraftValidationError> {
    if duration.get() > MAX_AUDIO_FADE_DURATION_MICROSECONDS {
        return Err(invalid_segment_audio(
            field,
            &format!(
                "fade duration must be <= {MAX_AUDIO_FADE_DURATION_MICROSECONDS} microseconds"
            ),
        ));
    }
    if duration > target_duration {
        return Err(invalid_segment_audio(
            field,
            "fade duration must not exceed the segment target duration",
        ));
    }
    Ok(())
}

fn validate_required_audio_text(field: &str, value: &str) -> Result<(), DraftValidationError> {
    if value.trim().is_empty() {
        return Err(invalid_segment_audio(field, "value must not be empty"));
    }
    Ok(())
}

fn validate_optional_audio_external_ref(
    field: &str,
    external_ref: &Option<String>,
) -> Result<(), DraftValidationError> {
    if let Some(external_ref) = external_ref {
        validate_required_audio_text(field, external_ref)?;
    }
    Ok(())
}

fn validate_segment_visual(
    draft: &Draft,
    field: &str,
    visual: &SegmentVisual,
) -> Result<(), DraftValidationError> {
    validate_segment_transform(field, visual)?;
    validate_segment_background_filling(
        draft,
        &format!("{field}.backgroundFilling"),
        &visual.background_filling,
    )?;
    validate_segment_blend_mode(&format!("{field}.blendMode"), &visual.blend_mode)?;
    validate_segment_mask(&format!("{field}.mask"), &visual.mask)?;
    Ok(())
}

fn validate_segment_transform(
    field: &str,
    visual: &SegmentVisual,
) -> Result<(), DraftValidationError> {
    let transform = &visual.transform;
    if transform.scale.x_millis == 0 {
        return Err(invalid_segment_visual(
            &format!("{field}.transform.scale.xMillis"),
            "scale must be greater than zero",
        ));
    }
    if transform.scale.y_millis == 0 {
        return Err(invalid_segment_visual(
            &format!("{field}.transform.scale.yMillis"),
            "scale must be greater than zero",
        ));
    }
    if transform.rotation.degrees < -360 || transform.rotation.degrees > 360 {
        return Err(invalid_segment_visual(
            &format!("{field}.transform.rotation.degrees"),
            "rotation must be between -360 and 360 degrees",
        ));
    }
    if transform.opacity.value_millis > crate::MAX_SEGMENT_OPACITY_MILLIS {
        return Err(invalid_segment_visual(
            &format!("{field}.transform.opacity.valueMillis"),
            "opacity must be <= 1000",
        ));
    }
    validate_segment_crop(&format!("{field}.transform.crop"), &transform.crop)?;
    validate_millis_range(
        &format!("{field}.transform.anchor.xMillis"),
        transform.anchor.x_millis,
        crate::MAX_SEGMENT_ANCHOR_MILLIS,
        "anchor must be between 0 and 1000",
    )?;
    validate_millis_range(
        &format!("{field}.transform.anchor.yMillis"),
        transform.anchor.y_millis,
        crate::MAX_SEGMENT_ANCHOR_MILLIS,
        "anchor must be between 0 and 1000",
    )
}

fn validate_segment_crop(field: &str, crop: &SegmentCrop) -> Result<(), DraftValidationError> {
    for (name, value) in [
        ("leftMillis", crop.left_millis),
        ("rightMillis", crop.right_millis),
        ("topMillis", crop.top_millis),
        ("bottomMillis", crop.bottom_millis),
    ] {
        validate_millis_range(
            &format!("{field}.{name}"),
            value,
            crate::MAX_SEGMENT_CROP_MILLIS,
            "crop inset must be between 0 and 1000",
        )?;
    }

    if crop.left_millis + crop.right_millis >= crate::MAX_SEGMENT_CROP_MILLIS {
        return Err(invalid_segment_visual(
            field,
            "left and right crop insets must sum to less than 1000",
        ));
    }
    if crop.top_millis + crop.bottom_millis >= crate::MAX_SEGMENT_CROP_MILLIS {
        return Err(invalid_segment_visual(
            field,
            "top and bottom crop insets must sum to less than 1000",
        ));
    }

    Ok(())
}

fn validate_segment_background_filling(
    draft: &Draft,
    field: &str,
    background_filling: &SegmentBackgroundFilling,
) -> Result<(), DraftValidationError> {
    match background_filling {
        SegmentBackgroundFilling::None
        | SegmentBackgroundFilling::Black
        | SegmentBackgroundFilling::Blur => Ok(()),
        SegmentBackgroundFilling::SolidColor { color } => {
            validate_segment_hex_color(&format!("{field}.color"), color)
        }
        SegmentBackgroundFilling::Image { material_id } => {
            validate_optional_image_material(draft, &format!("{field}.materialId"), material_id)
        }
    }
}

fn validate_optional_image_material(
    draft: &Draft,
    field: &str,
    material_id: &Option<MaterialId>,
) -> Result<(), DraftValidationError> {
    let Some(material_id) = material_id else {
        return Ok(());
    };
    let Some(material) = draft
        .materials
        .iter()
        .find(|material| &material.material_id == material_id)
    else {
        return Err(DraftValidationError::MissingRequiredSemanticField {
            field: format!("{field} references {}", material_id.as_str()),
        });
    };
    if material.kind != MaterialKind::Image {
        return Err(invalid_segment_visual(
            field,
            "image background filling material must reference an image material",
        ));
    }
    Ok(())
}

fn validate_segment_blend_mode(
    field: &str,
    blend_mode: &SegmentBlendMode,
) -> Result<(), DraftValidationError> {
    match blend_mode {
        SegmentBlendMode::Normal | SegmentBlendMode::Multiply | SegmentBlendMode::Screen => Ok(()),
        SegmentBlendMode::ExternalReference { reference } => {
            validate_required_text(&format!("{field}.reference.provider"), &reference.provider)?;
            validate_required_text(&format!("{field}.reference.effectId"), &reference.effect_id)
        }
    }
}

fn validate_segment_mask(field: &str, mask: &SegmentMask) -> Result<(), DraftValidationError> {
    match mask {
        SegmentMask::None => Ok(()),
        SegmentMask::Rectangle {
            x_millis,
            y_millis,
            width_millis,
            height_millis,
            feather_millis,
            opacity_millis,
            ..
        }
        | SegmentMask::Ellipse {
            x_millis,
            y_millis,
            width_millis,
            height_millis,
            feather_millis,
            opacity_millis,
            ..
        } => {
            validate_millis_range(
                &format!("{field}.xMillis"),
                *x_millis,
                MAX_SEGMENT_CROP_MILLIS,
                "mask x must be expressed in normalized millis",
            )?;
            validate_millis_range(
                &format!("{field}.yMillis"),
                *y_millis,
                MAX_SEGMENT_CROP_MILLIS,
                "mask y must be expressed in normalized millis",
            )?;
            validate_millis_range(
                &format!("{field}.widthMillis"),
                *width_millis,
                MAX_SEGMENT_CROP_MILLIS,
                "mask width must be expressed in normalized millis",
            )?;
            validate_millis_range(
                &format!("{field}.heightMillis"),
                *height_millis,
                MAX_SEGMENT_CROP_MILLIS,
                "mask height must be expressed in normalized millis",
            )?;
            validate_millis_range(
                &format!("{field}.featherMillis"),
                *feather_millis,
                MAX_SEGMENT_CROP_MILLIS,
                "mask feather must be expressed in normalized millis",
            )?;
            validate_millis_range(
                &format!("{field}.opacityMillis"),
                *opacity_millis,
                crate::MAX_SEGMENT_OPACITY_MILLIS,
                "mask opacity must be expressed in normalized millis",
            )?;
            if *width_millis == 0 {
                return Err(invalid_segment_visual(
                    &format!("{field}.widthMillis"),
                    "mask width must be greater than zero",
                ));
            }
            if *height_millis == 0 {
                return Err(invalid_segment_visual(
                    &format!("{field}.heightMillis"),
                    "mask height must be greater than zero",
                ));
            }
            if *x_millis + *width_millis > MAX_SEGMENT_CROP_MILLIS {
                return Err(invalid_segment_visual(
                    field,
                    "mask x and width must stay inside normalized bounds",
                ));
            }
            if *y_millis + *height_millis > MAX_SEGMENT_CROP_MILLIS {
                return Err(invalid_segment_visual(
                    field,
                    "mask y and height must stay inside normalized bounds",
                ));
            }
            Ok(())
        }
        SegmentMask::ExternalReference { reference } => {
            validate_required_text(&format!("{field}.reference.provider"), &reference.provider)?;
            validate_required_text(&format!("{field}.reference.effectId"), &reference.effect_id)
        }
    }
}

fn validate_segment_retiming(
    field: &str,
    retiming: &SegmentRetiming,
) -> Result<(), DraftValidationError> {
    match &retiming.mode {
        RetimeMode::Constant { speed } => {
            validate_speed_ratio(&format!("{field}.mode.speed"), speed)
        }
        RetimeMode::SpeedCurve { points } => {
            if points.is_empty() {
                return Err(invalid_segment_visual(
                    field,
                    "speed curve retiming requires at least one typed point",
                ));
            }
            for (index, point) in points.iter().enumerate() {
                validate_speed_ratio(&format!("{field}.mode.points[{index}].speed"), &point.speed)?;
            }
            Ok(())
        }
    }
}

fn validate_speed_ratio(
    field: &str,
    speed: &crate::SpeedRatio,
) -> Result<(), DraftValidationError> {
    if speed.numerator == 0 || speed.denominator == 0 {
        return Err(invalid_segment_visual(
            field,
            "speed ratios must use nonzero integer numerators and denominators",
        ));
    }
    Ok(())
}

fn validate_millis_range(
    field: &str,
    value: u32,
    max: u32,
    reason: &str,
) -> Result<(), DraftValidationError> {
    if value > max {
        return Err(invalid_segment_visual(field, reason));
    }
    Ok(())
}

fn validate_segment_hex_color(field: &str, color: &str) -> Result<(), DraftValidationError> {
    let color = color.trim();
    if color.len() != 7 || !color.starts_with('#') {
        return Err(invalid_segment_visual(
            field,
            "color must use #RRGGBB hex format",
        ));
    }
    if !color[1..]
        .chars()
        .all(|character| character.is_ascii_hexdigit())
    {
        return Err(invalid_segment_visual(
            field,
            "color must contain only hex digits",
        ));
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

fn invalid_segment_visual(field: &str, reason: &str) -> DraftValidationError {
    DraftValidationError::InvalidSegmentVisual {
        field: field.to_owned(),
        reason: reason.to_owned(),
    }
}

fn invalid_segment_audio(field: &str, reason: &str) -> DraftValidationError {
    DraftValidationError::InvalidSegmentAudio {
        field: field.to_owned(),
        reason: reason.to_owned(),
    }
}

fn invalid_text_segment(field: &str, reason: &str) -> DraftValidationError {
    DraftValidationError::InvalidTextSegment {
        field: field.to_owned(),
        reason: reason.to_owned(),
    }
}

fn invalid_keyframe(field: &str, reason: &str) -> DraftValidationError {
    DraftValidationError::InvalidKeyframe {
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

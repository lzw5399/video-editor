use std::collections::BTreeMap;

use draft_model::{
    AudioEffectSlot, AudioEffectSlotKind, AudioFade, AudioPanBalance, Draft, DraftSchemaVersion,
    DraftValidationError, Filter, Keyframe, KeyframeEasing, KeyframeInterpolation,
    KeyframeProperty, KeyframeValue, MAX_AUDIO_FADE_DURATION_MICROSECONDS,
    MAX_AUDIO_PAN_BALANCE_MILLIS, MAX_SEGMENT_VOLUME_MILLIS, MIN_AUDIO_PAN_BALANCE_MILLIS,
    MainTrackMagnet, Material, MaterialKind, MaterialMetadata, MaterialStatus, Microseconds,
    RationalFrameRate, Segment, SegmentAnchor, SegmentAudio, SegmentBackgroundFilling,
    SegmentBlendMode, SegmentCrop, SegmentFitMode, SegmentMask, SegmentOpacity, SegmentScale,
    SegmentVisual, SourceTimerange, TargetTimerange, TextAlignment, TextBox, TextBubbleRef,
    TextEffectRef, TextFont, TextLayoutRegion, TextSegment, TextSegmentSource, TextStyle,
    TextWrapping, Track, TrackKind, Transition, add_material, mark_material_available,
    mark_material_missing, mark_material_probe_failed, migrate_draft_json, upsert_material,
    validate_draft,
};
use serde_json::json;

#[test]
fn draft_schema_creates_valid_empty_draft() {
    let draft = Draft::new("draft-001", "Untitled");

    assert_eq!(draft.schema_version, DraftSchemaVersion::CURRENT);
    assert_eq!(draft.draft_id.as_str(), "draft-001");
    assert_eq!(draft.metadata.name, "Untitled");
    assert!(draft.materials.is_empty());
    assert!(draft.tracks.is_empty());

    let serialized = serde_json::to_value(&draft).expect("draft should serialize");
    assert_eq!(serialized["schemaVersion"], json!(1));
    assert_eq!(serialized["draftId"], json!("draft-001"));
    assert_eq!(serialized["materials"], json!([]));
    assert_eq!(serialized["tracks"], json!([]));
}

#[test]
fn draft_schema_serializes_material_track_and_segment_records() {
    let material = Material {
        material_id: "material-video-001".into(),
        kind: MaterialKind::Video,
        uri: "media/video.mp4".to_owned(),
        display_name: "video.mp4".to_owned(),
        metadata: MaterialMetadata {
            duration: Some(Microseconds::new(1_500_000)),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(RationalFrameRate::new(30_000, 1_001)),
            has_video: true,
            has_audio: true,
            audio_sample_rate: Some(48_000),
            audio_channels: Some(2),
            probe_error: None,
        },
        status: MaterialStatus::Available,
    };

    let mut filter_parameters = BTreeMap::new();
    filter_parameters.insert("intensity".to_owned(), "0.75".to_owned());

    let segment = Segment {
        segment_id: "segment-001".into(),
        material_id: material.material_id.clone(),
        source_timerange: SourceTimerange::new(250_000, 1_000_000),
        target_timerange: TargetTimerange::new(0, 1_000_000),
        main_track_magnet: MainTrackMagnet::enabled(),
        keyframes: vec![Keyframe {
            at: Microseconds::new(500_000),
            property: KeyframeProperty::VisualOpacity,
            value: KeyframeValue::Uint { value: 500 },
            interpolation: KeyframeInterpolation::Linear,
            easing: KeyframeEasing::None,
        }],
        filters: vec![Filter {
            name: "brightness".to_owned(),
            parameters: filter_parameters,
        }],
        transition: Some(Transition {
            name: "fade".to_owned(),
            duration: Microseconds::new(100_000),
        }),
        text: None,
        volume: Default::default(),
        visual: SegmentVisual::default(),
    };

    let mut track = Track::new("track-video-001", TrackKind::Video, "Video 1");
    track.segments.push(segment);

    let mut draft = Draft::new("draft-001", "Timeline draft");
    draft.materials.push(material);
    draft.tracks.push(track);

    let serialized = serde_json::to_value(&draft).expect("draft should serialize");
    assert_eq!(
        serialized["materials"][0]["metadata"]["duration"],
        json!(1_500_000)
    );
    assert_eq!(
        serialized["materials"][0]["metadata"]["frameRate"],
        json!({ "numerator": 30000, "denominator": 1001 })
    );
    assert_eq!(
        serialized["tracks"][0]["segments"][0]["sourceTimerange"],
        json!({ "start": 250000, "duration": 1000000 })
    );
    assert_eq!(
        serialized["tracks"][0]["segments"][0]["targetTimerange"],
        json!({ "start": 0, "duration": 1000000 })
    );

    let round_tripped: Draft =
        serde_json::from_value(serialized).expect("serialized draft should deserialize");
    assert_eq!(round_tripped, draft);
}

#[test]
fn migration_loads_version_one_json_and_validates_draft() {
    let value = serde_json::to_value(valid_draft()).expect("draft should serialize");
    let migrated = migrate_draft_json(value).expect("version 1 draft should migrate");

    assert_eq!(migrated, valid_draft());
    validate_draft(&migrated).expect("migrated draft should validate");
}

#[test]
fn migration_rejects_unknown_future_schema_version() {
    let mut value = serde_json::to_value(valid_draft()).expect("draft should serialize");
    value["schemaVersion"] = json!(2);

    let error = migrate_draft_json(value).expect_err("future schema version should fail");
    assert_eq!(
        error,
        DraftValidationError::InvalidSchemaVersion {
            found: "2".to_owned(),
            expected: 1
        }
    );
}

#[test]
fn migration_rejects_missing_required_semantic_fields() {
    let error = migrate_draft_json(json!({
        "schemaVersion": 1,
        "metadata": { "name": "Missing ID" },
        "materials": [],
        "tracks": []
    }))
    .expect_err("missing draftId should fail");

    assert_eq!(
        error,
        DraftValidationError::MissingRequiredSemanticField {
            field: "draftId".to_owned()
        }
    );
}

#[test]
fn migration_rejects_derived_artifact_leakage() {
    let mut value = serde_json::to_value(valid_draft()).expect("draft should serialize");
    value["renderGraph"] = json!({ "nodes": [] });

    let error = migrate_draft_json(value).expect_err("derived fields should fail");
    assert_eq!(
        error,
        DraftValidationError::DerivedArtifactLeakage {
            field: "renderGraph".to_owned()
        }
    );
}

#[test]
fn migration_rejects_invalid_timeranges() {
    let mut draft = valid_draft();
    draft.tracks[0].segments[0].target_timerange.duration = Microseconds::ZERO;

    let error = validate_draft(&draft).expect_err("zero target duration should fail");
    assert_eq!(
        error,
        DraftValidationError::InvalidTimerange {
            field: "tracks[].segments[].targetTimerange.duration".to_owned(),
            reason: "duration must be greater than zero microseconds".to_owned()
        }
    );
}

#[test]
fn segment_visual_defaults_preserve_mvp_rendering_contract() {
    let draft = valid_draft();
    let visual = &draft.tracks[0].segments[0].visual;

    assert!(visual.visible);
    assert_eq!(visual.fit_mode, SegmentFitMode::Stretch);
    assert_eq!(visual.transform.position.x, 0);
    assert_eq!(visual.transform.position.y, 0);
    assert_eq!(visual.transform.scale.x_millis, 1_000);
    assert_eq!(visual.transform.scale.y_millis, 1_000);
    assert_eq!(visual.transform.rotation.degrees, 0);
    assert_eq!(visual.transform.opacity.value_millis, 1_000);
    assert_eq!(visual.transform.crop.left_millis, 0);
    assert_eq!(visual.transform.anchor.x_millis, 500);
    assert_eq!(visual.background_filling, SegmentBackgroundFilling::None);
    assert_eq!(visual.blend_mode, SegmentBlendMode::Normal);
    assert_eq!(visual.mask, SegmentMask::None);
}

#[test]
fn audio_semantics_defaults_preserve_legacy_segment_volume() {
    let mut legacy = serde_json::to_value(valid_draft()).expect("draft should serialize");
    let segment = legacy["tracks"][0]["segments"][0]
        .as_object_mut()
        .expect("fixture should expose segment object");
    segment.remove("audio");

    let migrated = migrate_draft_json(legacy).expect("legacy segment should load");
    let loaded_segment = &migrated.tracks[0].segments[0];

    assert_eq!(loaded_segment.volume.level_millis, 1_000);
    assert_eq!(loaded_segment.audio, SegmentAudio::default());
    assert_eq!(
        loaded_segment.audio.gain_millis,
        loaded_segment.volume.level_millis
    );
    assert_eq!(
        loaded_segment.audio.pan_balance_millis,
        AudioPanBalance::center().balance_millis
    );
    assert!(loaded_segment.audio.effect_slots.is_empty());
}

#[test]
fn audio_semantics_validate_gain_pan_fades_and_effect_slots() {
    let mut draft = valid_draft();
    draft.tracks[0].segments[0].audio = SegmentAudio {
        gain_millis: 1_250,
        pan_balance_millis: AudioPanBalance {
            balance_millis: -250,
        }
        .balance_millis,
        fade_in_duration: AudioFade {
            duration: Microseconds::new(100_000),
        },
        fade_out_duration: AudioFade {
            duration: Microseconds::new(150_000),
        },
        effect_slots: vec![AudioEffectSlot {
            slot_id: "slot-eq-1".to_owned(),
            kind: AudioEffectSlotKind::Unsupported {
                name: "future-eq".to_owned(),
                external_ref: Some("jianying://audio/effect/future-eq".to_owned()),
            },
            enabled: true,
        }],
    };
    validate_draft(&draft).expect("bounded audio semantics should validate");

    for (label, mutate, expected) in [
        (
            "gain overflow",
            set_audio_gain_overflow as fn(&mut SegmentAudio),
            "gainMillis",
        ),
        ("pan below min", set_audio_pan_below_min, "panBalanceMillis"),
        ("pan above max", set_audio_pan_above_max, "panBalanceMillis"),
        (
            "fade longer than segment",
            set_audio_fade_longer_than_segment,
            "fadeInDuration",
        ),
        (
            "fade over absolute max",
            set_audio_fade_over_absolute_max,
            "fadeOutDuration",
        ),
        (
            "empty effect slot",
            set_audio_empty_effect_slot,
            "effectSlots",
        ),
    ] {
        let mut invalid = draft.clone();
        mutate(&mut invalid.tracks[0].segments[0].audio);
        let error = validate_draft(&invalid).expect_err(label);
        assert!(
            error.to_string().contains(expected),
            "{label} should mention {expected}: {error}"
        );
    }
}

#[test]
fn audio_semantics_schema_excludes_derived_audio_artifacts() {
    let draft_schema = schemars::schema_for!(Draft);
    let schema_text = serde_json::to_string(&draft_schema).expect("schema should serialize");

    for expected in [
        "SegmentAudio",
        "AudioFade",
        "AudioPanBalance",
        "AudioEffectSlot",
    ] {
        assert!(
            schema_text.contains(expected),
            "draft schema should expose {expected}"
        );
    }

    for forbidden in [
        "waveformPath",
        "waveformBlob",
        "waveformPeaks",
        "sqlite",
        "cacheKey",
        "fingerprint",
        "outputDeviceHandle",
        "mixBuffer",
        "ringBuffer",
    ] {
        assert!(
            !schema_text.contains(forbidden),
            "draft schema must not expose derived audio artifact field {forbidden}"
        );
    }
}

#[test]
fn segment_visual_validation_rejects_invalid_transform_values() {
    for (label, mutate, expected_field) in [
        (
            "zero scale",
            set_zero_scale as fn(&mut SegmentVisual),
            "scale.xMillis",
        ),
        (
            "opacity overflow",
            set_opacity_overflow,
            "opacity.valueMillis",
        ),
        ("crop overflow", set_crop_overflow, "crop"),
        ("anchor overflow", set_anchor_overflow, "anchor.xMillis"),
        (
            "invalid color",
            set_invalid_background_color,
            "backgroundFilling.color",
        ),
        (
            "missing blend name",
            set_missing_blend_name,
            "blendMode.name",
        ),
        ("missing mask name", set_missing_mask_name, "mask.name"),
    ] {
        let mut draft = valid_draft();
        mutate(&mut draft.tracks[0].segments[0].visual);

        let error = validate_draft(&draft).expect_err(label);
        assert!(
            error.to_string().contains(expected_field),
            "{label} should mention {expected_field}: {error}"
        );
    }
}

#[test]
fn keyframes_serialize_as_typed_jianying_style_semantics() {
    let mut draft = valid_draft();
    draft.tracks[0].segments[0].keyframes = vec![
        Keyframe {
            at: Microseconds::new(250_000),
            property: KeyframeProperty::VisualOpacity,
            value: KeyframeValue::Uint { value: 760 },
            interpolation: KeyframeInterpolation::Linear,
            easing: KeyframeEasing::EaseInOut,
        },
        Keyframe {
            at: Microseconds::new(500_000),
            property: KeyframeProperty::TextColor,
            value: KeyframeValue::Color {
                value: "#ffcc00".to_owned(),
            },
            interpolation: KeyframeInterpolation::Hold,
            easing: KeyframeEasing::None,
        },
    ];

    validate_draft(&draft).expect("typed keyframes should validate");

    let serialized = serde_json::to_value(&draft).expect("draft should serialize");
    assert_eq!(
        serialized["tracks"][0]["segments"][0]["keyframes"][0],
        json!({
            "at": 250000,
            "property": "visualOpacity",
            "value": { "kind": "uint", "value": 760 },
            "interpolation": "linear",
            "easing": "easeInOut"
        })
    );
    assert_eq!(
        serialized["tracks"][0]["segments"][0]["keyframes"][1]["value"],
        json!({ "kind": "color", "value": "#ffcc00" })
    );

    let round_tripped: Draft =
        serde_json::from_value(serialized).expect("typed keyframes should deserialize");
    assert_eq!(round_tripped, draft);
}

#[test]
fn keyframe_validation_rejects_invalid_property_value_combinations() {
    let mut draft = valid_draft();
    draft.tracks[0].segments[0].keyframes = vec![Keyframe {
        at: Microseconds::new(250_000),
        property: KeyframeProperty::VisualOpacity,
        value: KeyframeValue::Color {
            value: "#ffffff".to_owned(),
        },
        interpolation: KeyframeInterpolation::Linear,
        easing: KeyframeEasing::None,
    }];

    let error = validate_draft(&draft).expect_err("opacity requires unsigned millis");
    assert!(
        error.to_string().contains("keyframes"),
        "error should mention keyframes: {error}"
    );
    assert!(
        error.to_string().contains("VisualOpacity") || error.to_string().contains("visualOpacity"),
        "error should mention visual opacity: {error}"
    );
}

#[test]
fn keyframe_validation_rejects_out_of_range_time_and_duplicate_property_time() {
    let mut draft = valid_draft();
    draft.tracks[0].segments[0].keyframes = vec![Keyframe {
        at: Microseconds::new(1_000_001),
        property: KeyframeProperty::VisualPositionX,
        value: KeyframeValue::Int { value: 120 },
        interpolation: KeyframeInterpolation::Linear,
        easing: KeyframeEasing::None,
    }];

    let error = validate_draft(&draft).expect_err("keyframe after segment end should fail");
    assert!(
        error.to_string().contains("at"),
        "error should mention keyframe time: {error}"
    );

    draft.tracks[0].segments[0].keyframes = vec![
        Keyframe {
            at: Microseconds::new(400_000),
            property: KeyframeProperty::VisualPositionX,
            value: KeyframeValue::Int { value: 120 },
            interpolation: KeyframeInterpolation::Linear,
            easing: KeyframeEasing::None,
        },
        Keyframe {
            at: Microseconds::new(400_000),
            property: KeyframeProperty::VisualPositionX,
            value: KeyframeValue::Int { value: 320 },
            interpolation: KeyframeInterpolation::Hold,
            easing: KeyframeEasing::None,
        },
    ];

    let error = validate_draft(&draft).expect_err("duplicate property/time should fail");
    assert!(
        error.to_string().contains("duplicate"),
        "error should mention duplicate keyframe: {error}"
    );
}

#[test]
fn keyframe_validation_preserves_existing_segments_with_empty_keyframes() {
    let draft = valid_draft();
    assert!(draft.tracks[0].segments[0].keyframes.is_empty());
    validate_draft(&draft).expect("empty keyframes remain valid");
}

#[test]
fn segment_visual_background_filling_references_image_materials() {
    let mut draft = valid_draft();
    draft.materials.push(Material::new(
        "material-image-001",
        MaterialKind::Image,
        "media/background.png",
        "background.png",
    ));
    draft.tracks[0].segments[0].visual.background_filling = SegmentBackgroundFilling::Image {
        material_id: Some("material-image-001".into()),
    };
    validate_draft(&draft).expect("image background filling may reference image materials");

    draft.tracks[0].segments[0].visual.background_filling = SegmentBackgroundFilling::Image {
        material_id: Some("material-video-001".into()),
    };
    let error = validate_draft(&draft).expect_err("video material should reject image filling");
    assert!(error.to_string().contains("backgroundFilling.materialId"));
}

#[test]
fn text_segment_deserializes_existing_text_with_phase9_defaults() {
    let text: TextSegment = serde_json::from_value(json!({
        "content": "旧文字片段",
        "style": {
            "fontSize": 36,
            "color": "#ffffff",
            "alignment": "center"
        }
    }))
    .expect("existing MVP text segment should deserialize");

    assert_eq!(text.source, TextSegmentSource::Text);
    assert_eq!(text.style.font, TextFont::default());
    assert_eq!(text.style.line_height_millis, 1_200);
    assert_eq!(text.style.letter_spacing_millis, 0);
    assert_eq!(text.text_box, TextBox::default());
    assert_eq!(text.layout_region, TextLayoutRegion::default());
    assert_eq!(text.wrapping, TextWrapping::Auto);
    assert_eq!(text.bubble, None);
    assert_eq!(text.effect, None);
}

#[test]
fn text_segment_validates_complete_subtitle_semantics() {
    let mut draft = valid_text_draft();
    draft.tracks[0].segments[0].text = Some(TextSegment {
        content: "完整字幕".to_owned(),
        source: TextSegmentSource::Subtitle,
        style: TextStyle {
            font: TextFont {
                family: "PingFang SC".to_owned(),
                font_ref: Some("font://system/pingfang-sc".to_owned()),
            },
            font_size: 42,
            color: "#ffeeaa".to_owned(),
            alignment: TextAlignment::Center,
            line_height_millis: 1_250,
            letter_spacing_millis: 40,
            stroke: None,
            shadow: None,
            background: None,
        },
        text_box: TextBox {
            width_millis: 700,
            height_millis: 180,
        },
        layout_region: TextLayoutRegion {
            x_millis: 100,
            y_millis: 650,
            width_millis: 800,
            height_millis: 250,
        },
        wrapping: TextWrapping::Auto,
        bubble: Some(TextBubbleRef::Unsupported {
            name: "jianying-bubble-a".to_owned(),
            external_ref: Some("jy://bubble/a".to_owned()),
        }),
        effect: Some(TextEffectRef::Unsupported {
            name: "jianying-huazi-a".to_owned(),
            external_ref: Some("jy://text-effect/a".to_owned()),
        }),
    });

    validate_draft(&draft).expect("complete text semantics should validate");
}

#[test]
fn text_segment_validation_rejects_invalid_phase9_fields() {
    for (label, mutate, expected_field) in [
        (
            "blank font family",
            blank_font_family as fn(&mut TextSegment),
            "font.family",
        ),
        ("blank font ref", blank_font_ref, "font.fontRef"),
        ("invalid fill color", invalid_text_color, "style.color"),
        ("zero font size", zero_font_size, "style.fontSize"),
        ("low line height", low_line_height, "style.lineHeightMillis"),
        (
            "large letter spacing",
            large_letter_spacing,
            "style.letterSpacingMillis",
        ),
        ("zero text box", zero_text_box_width, "textBox.widthMillis"),
        ("layout overflow", overflowing_layout_region, "layoutRegion"),
        ("blank bubble name", blank_bubble_name, "bubble.name"),
        (
            "blank effect external ref",
            blank_effect_external_ref,
            "effect.externalRef",
        ),
    ] {
        let mut draft = valid_text_draft();
        let text = draft.tracks[0].segments[0]
            .text
            .as_mut()
            .expect("test draft should contain text");
        mutate(text);

        let error = validate_draft(&draft).expect_err(label);
        assert!(
            error.to_string().contains(expected_field),
            "{label} should mention {expected_field}: {error}"
        );
    }
}

#[test]
fn migration_rejects_invalid_rational_frame_rate() {
    let mut draft = valid_draft();
    draft.materials[0].metadata.frame_rate = Some(RationalFrameRate::new(30, 0));

    let error = validate_draft(&draft).expect_err("zero frame-rate denominator should fail");
    assert_eq!(
        error,
        DraftValidationError::InvalidRationalFrameRate {
            field: "materials[].metadata.frameRate.denominator".to_owned(),
            reason: "denominator must be greater than zero".to_owned()
        }
    );
}

#[test]
fn migration_rejects_duplicate_ids() {
    let mut draft = valid_draft();
    draft.materials.push(Material::new(
        "material-video-001",
        MaterialKind::Video,
        "b.mp4",
        "b.mp4",
    ));

    let error = validate_draft(&draft).expect_err("duplicate material ID should fail");
    assert_eq!(
        error,
        DraftValidationError::DuplicateId {
            id_kind: "materialId".to_owned(),
            id: "material-video-001".to_owned()
        }
    );
}

#[test]
fn material_registry_helpers_add_upsert_and_mark_statuses() {
    let mut draft = Draft::new("draft-001", "Registry draft");
    let mut material = Material::new(
        "material-video-001",
        MaterialKind::Video,
        "media/video.mp4",
        "video.mp4",
    );
    material.metadata.duration = Some(Microseconds::new(1_000_000));
    material.metadata.has_video = true;

    add_material(&mut draft, material.clone()).expect("material should be added");
    assert_eq!(draft.materials, vec![material.clone()]);

    let mut updated = material.clone();
    updated.display_name = "renamed-video.mp4".to_owned();
    updated.metadata.width = Some(1280);
    upsert_material(&mut draft, updated.clone()).expect("material should be replaced");

    assert_eq!(draft.materials.len(), 1);
    assert_eq!(draft.materials[0], updated);

    mark_material_missing(
        &mut draft,
        &"material-video-001".into(),
        "material path is missing",
    )
    .expect("material should be marked missing");
    assert_eq!(draft.materials[0].status, MaterialStatus::Missing);
    assert_eq!(
        draft.materials[0].metadata.probe_error.as_deref(),
        Some("material path is missing")
    );

    mark_material_probe_failed(&mut draft, &"material-video-001".into(), "ffprobe failed")
        .expect("material should be marked probe failed");
    assert_eq!(draft.materials[0].status, MaterialStatus::ProbeFailed);
    assert_eq!(
        draft.materials[0].metadata.probe_error.as_deref(),
        Some("ffprobe failed")
    );

    mark_material_available(&mut draft, &"material-video-001".into())
        .expect("material should be marked available");
    assert_eq!(draft.materials[0].status, MaterialStatus::Available);
    assert_eq!(draft.materials[0].metadata.probe_error, None);
}

#[test]
fn material_registry_helpers_roll_back_invalid_mutations() {
    let mut draft = valid_draft();
    let original = draft.materials.clone();

    let duplicate = Material::new(
        "material-video-001",
        MaterialKind::Video,
        "media/duplicate.mp4",
        "duplicate.mp4",
    );
    let error = add_material(&mut draft, duplicate).expect_err("duplicate ID should fail");

    assert_eq!(
        error,
        DraftValidationError::DuplicateId {
            id_kind: "materialId".to_owned(),
            id: "material-video-001".to_owned()
        }
    );
    assert_eq!(draft.materials, original);

    let error = mark_material_missing(
        &mut draft,
        &"material-missing".into(),
        "missing material record",
    )
    .expect_err("unknown material ID should fail");
    assert_eq!(
        error,
        DraftValidationError::MissingRequiredSemanticField {
            field: "materials[].materialId material-missing".to_owned()
        }
    );
}

fn valid_draft() -> Draft {
    let material = Material {
        material_id: "material-video-001".into(),
        kind: MaterialKind::Video,
        uri: "media/video.mp4".to_owned(),
        display_name: "video.mp4".to_owned(),
        metadata: MaterialMetadata {
            duration: Some(Microseconds::new(1_500_000)),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(RationalFrameRate::new(30_000, 1_001)),
            has_video: true,
            has_audio: true,
            audio_sample_rate: Some(48_000),
            audio_channels: Some(2),
            probe_error: None,
        },
        status: MaterialStatus::Available,
    };

    let segment = Segment::new(
        "segment-001",
        material.material_id.clone(),
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    );
    let mut track = Track::new("track-video-001", TrackKind::Video, "Video 1");
    track.segments.push(segment);

    let mut draft = Draft::new("draft-001", "Valid draft");
    draft.materials.push(material);
    draft.tracks.push(track);
    draft
}

fn valid_text_draft() -> Draft {
    let material = Material::new(
        "material-text-001",
        MaterialKind::Text,
        "text://material-text-001",
        "字幕",
    );

    let mut segment = Segment::new(
        "segment-text-001",
        material.material_id.clone(),
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    );
    segment.text = Some(TextSegment {
        content: "默认文字".to_owned(),
        source: TextSegmentSource::Text,
        style: TextStyle::default(),
        text_box: TextBox::default(),
        layout_region: TextLayoutRegion::default(),
        wrapping: TextWrapping::Auto,
        bubble: None,
        effect: None,
    });

    let mut track = Track::new("track-text-001", TrackKind::Text, "字幕");
    track.segments.push(segment);

    let mut draft = Draft::new("draft-text-001", "Text draft");
    draft.materials.push(material);
    draft.tracks.push(track);
    draft
}

fn blank_font_family(text: &mut TextSegment) {
    text.style.font.family = " ".to_owned();
}

fn blank_font_ref(text: &mut TextSegment) {
    text.style.font.font_ref = Some(String::new());
}

fn invalid_text_color(text: &mut TextSegment) {
    text.style.color = "ffffff".to_owned();
}

fn zero_font_size(text: &mut TextSegment) {
    text.style.font_size = 0;
}

fn low_line_height(text: &mut TextSegment) {
    text.style.line_height_millis = 499;
}

fn large_letter_spacing(text: &mut TextSegment) {
    text.style.letter_spacing_millis = 2_001;
}

fn zero_text_box_width(text: &mut TextSegment) {
    text.text_box.width_millis = 0;
}

fn overflowing_layout_region(text: &mut TextSegment) {
    text.layout_region.x_millis = 300;
    text.layout_region.width_millis = 800;
}

fn blank_bubble_name(text: &mut TextSegment) {
    text.bubble = Some(TextBubbleRef::Unsupported {
        name: String::new(),
        external_ref: None,
    });
}

fn blank_effect_external_ref(text: &mut TextSegment) {
    text.effect = Some(TextEffectRef::Unsupported {
        name: "花字".to_owned(),
        external_ref: Some(" ".to_owned()),
    });
}

fn set_zero_scale(visual: &mut SegmentVisual) {
    visual.transform.scale = SegmentScale {
        x_millis: 0,
        y_millis: 1_000,
    };
}

fn set_opacity_overflow(visual: &mut SegmentVisual) {
    visual.transform.opacity = SegmentOpacity {
        value_millis: 1_001,
    };
}

fn set_crop_overflow(visual: &mut SegmentVisual) {
    visual.transform.crop = SegmentCrop {
        left_millis: 600,
        right_millis: 400,
        top_millis: 0,
        bottom_millis: 0,
    };
}

fn set_anchor_overflow(visual: &mut SegmentVisual) {
    visual.transform.anchor = SegmentAnchor {
        x_millis: 1_001,
        y_millis: 500,
    };
}

fn set_invalid_background_color(visual: &mut SegmentVisual) {
    visual.background_filling = SegmentBackgroundFilling::SolidColor {
        color: "ffffff".to_owned(),
    };
}

fn set_missing_blend_name(visual: &mut SegmentVisual) {
    visual.blend_mode = SegmentBlendMode::Unsupported {
        name: " ".to_owned(),
    };
}

fn set_missing_mask_name(visual: &mut SegmentVisual) {
    visual.mask = SegmentMask::Unsupported {
        name: String::new(),
    };
}

fn set_audio_gain_overflow(audio: &mut SegmentAudio) {
    audio.gain_millis = MAX_SEGMENT_VOLUME_MILLIS + 1;
}

fn set_audio_pan_below_min(audio: &mut SegmentAudio) {
    audio.pan_balance_millis = MIN_AUDIO_PAN_BALANCE_MILLIS - 1;
}

fn set_audio_pan_above_max(audio: &mut SegmentAudio) {
    audio.pan_balance_millis = MAX_AUDIO_PAN_BALANCE_MILLIS + 1;
}

fn set_audio_fade_longer_than_segment(audio: &mut SegmentAudio) {
    audio.fade_in_duration = AudioFade {
        duration: Microseconds::new(1_000_001),
    };
}

fn set_audio_fade_over_absolute_max(audio: &mut SegmentAudio) {
    audio.fade_out_duration = AudioFade {
        duration: Microseconds::new(MAX_AUDIO_FADE_DURATION_MICROSECONDS + 1),
    };
}

fn set_audio_empty_effect_slot(audio: &mut SegmentAudio) {
    audio.effect_slots = vec![AudioEffectSlot {
        slot_id: String::new(),
        kind: AudioEffectSlotKind::Unsupported {
            name: String::new(),
            external_ref: Some(" ".to_owned()),
        },
        enabled: true,
    }];
}

#[test]
fn draft_schema_rejects_unknown_fields() {
    let result = serde_json::from_value::<Draft>(json!({
        "schemaVersion": 1,
        "draftId": "draft-001",
        "metadata": { "name": "Unknown field draft" },
        "materials": [],
        "tracks": [],
        "previewCaches": []
    }));

    assert!(result.is_err(), "unknown draft fields must fail");

    let result = serde_json::from_value::<Material>(json!({
        "materialId": "material-001",
        "kind": "video",
        "uri": "media/video.mp4",
        "displayName": "video.mp4",
        "metadata": {
            "duration": 1000000,
            "hasVideo": true,
            "hasAudio": false
        },
        "status": "available",
        "thumbnailPath": "cache/thumb.jpg"
    }));

    assert!(result.is_err(), "unknown material fields must fail");
}

#[test]
fn draft_schema_serializes_integer_microseconds_and_rational_frame_rate() {
    let metadata = MaterialMetadata {
        duration: Some(Microseconds::new(3_333_333)),
        width: Some(1280),
        height: Some(720),
        frame_rate: Some(RationalFrameRate::new(24, 1)),
        has_video: true,
        has_audio: false,
        audio_sample_rate: None,
        audio_channels: None,
        probe_error: None,
    };

    let serialized = serde_json::to_value(metadata).expect("metadata should serialize");
    assert_eq!(serialized["duration"], json!(3_333_333));
    assert_eq!(
        serialized["frameRate"],
        json!({ "numerator": 24, "denominator": 1 })
    );
}

#[test]
fn draft_schema_excludes_derived_artifact_fields_from_draft() {
    let serialized = serde_json::to_value(Draft::new("draft-001", "Clean draft"))
        .expect("draft should serialize");
    let object = serialized
        .as_object()
        .expect("draft JSON should be an object");

    for forbidden_key in [
        "thumbnails",
        "waveforms",
        "previewCaches",
        "renderGraph",
        "ffmpegScripts",
        "exports",
        "rawProbeJson",
    ] {
        assert!(
            !object.contains_key(forbidden_key),
            "draft JSON must exclude derived artifact key {forbidden_key}"
        );
    }
}

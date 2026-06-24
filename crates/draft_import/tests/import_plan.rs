use draft_import::{
    AdaptationCategory, AdaptationReportItem, AdaptationSeverity, AdaptationStatus,
    AdaptationTargetKind, AdaptationTargetRef, DraftImportApplicationInput, DraftImportPlan,
    DraftImportPlanSchemaVersion, DraftImportPlanValidationError, ExternalProvenanceRef,
    ImportMaterialPlan, ImportTrackPlan, apply_import_plan_to_draft, validate_import_plan,
};
use draft_model::{
    AudioFade, CanvasAdaptationPolicy, CanvasAspectRatio, CanvasBackground, DraftCanvasConfig,
    Keyframe, KeyframeEasing, KeyframeInterpolation, KeyframeProperty, KeyframeValue,
    MainTrackMagnet, Material, MaterialKind, MaterialMetadata, MaterialStatus, Microseconds,
    RationalFrameRate, Segment, SegmentAudio, SegmentPosition, SegmentScale, SegmentTransform,
    SegmentVisual, SourceTimerange, TargetTimerange, TextAlignment, TextFont, TextLayoutRegion,
    TextSegment, TextStyle, TextWrapping, Track, TrackKind, Transition, validate_draft,
};
use serde_json::json;

#[test]
fn draft_import_plan_accepts_canonical_canvas_material_tracks_audio_text_and_keyframes() {
    let plan = valid_draft_import_plan();

    validate_import_plan(&plan).expect("valid import plan should pass before session mutation");
    let applied = apply_import_plan_to_draft(DraftImportApplicationInput {
        plan,
        source_kind: "kaipaiOfflineBundle".to_owned(),
        generated_at: "2026-06-24T00:00:00Z".to_owned(),
        report_items: vec![supported_report_item(
            AdaptationCategory::Canvas,
            AdaptationTargetKind::Canvas,
            "canvas-main",
        )],
    })
    .expect("valid import plan should apply to a canonical draft");

    validate_draft(&applied.draft).expect("applied import draft should pass draft validation");
    assert_eq!(applied.draft.canvas_config.width, 1080);
    assert_eq!(applied.draft.canvas_config.height, 1920);
    assert_eq!(applied.draft.materials.len(), 4);
    assert_eq!(
        applied.draft.materials[0].uri,
        "resources/template-import/template-alpha/videos/main-video/source.mp4"
    );
    assert_eq!(applied.draft.tracks.len(), 3);
    assert_eq!(applied.draft.tracks[0].kind, TrackKind::Video);
    assert_eq!(applied.draft.tracks[1].kind, TrackKind::Text);
    assert_eq!(applied.draft.tracks[2].kind, TrackKind::Audio);

    let visual_segment = &applied.draft.tracks[0].segments[0];
    assert_eq!(visual_segment.keyframes.len(), 2);
    assert_eq!(visual_segment.visual.transform.position.x, 120);
    assert_eq!(
        visual_segment
            .transition
            .as_ref()
            .map(|transition| transition.name.as_str()),
        Some("dissolve")
    );

    let text_segment = applied.draft.tracks[1].segments[0]
        .text
        .as_ref()
        .expect("text import should preserve canonical text segment");
    assert_eq!(text_segment.content, "导入标题");
    assert_eq!(
        text_segment.style.font.font_ref.as_deref(),
        Some(draft_model::BUNDLED_TEXT_FONT_REF)
    );

    let audio_segment = &applied.draft.tracks[2].segments[0];
    assert_eq!(audio_segment.audio.gain_millis, 850);
    assert_eq!(
        audio_segment.audio.fade_in_duration.duration,
        Microseconds::new(500_000)
    );
    assert_eq!(
        audio_segment.audio.fade_out_duration.duration,
        Microseconds::new(750_000)
    );
    assert_eq!(applied.report.summary.supported, 1);
    assert_eq!(applied.report.source_kind, "kaipaiOfflineBundle");
}

#[test]
fn draft_import_plan_rejects_remote_runtime_refs_and_raw_provider_semantics() {
    let mut remote_plan = valid_draft_import_plan();
    remote_plan.materials[0].material.uri =
        "https://example.invalid/render/main-video.mp4".to_owned();

    let remote_error = validate_import_plan(&remote_plan)
        .expect_err("remote material refs must be rejected before session mutation");
    assert!(
        matches!(
            remote_error,
            DraftImportPlanValidationError::RemoteRuntimeRef { .. }
        ),
        "expected remote runtime ref error, got {remote_error:?}"
    );

    for forbidden in [
        ("templateId", json!("template-42")),
        ("recipeId", json!("recipe-42")),
        ("formula", json!({"unsafe": true})),
        ("safeArea", json!({"x": 0, "y": 0})),
        ("provenance", json!({"externalPath": "formula.layers[0]"})),
    ] {
        let mut value = valid_draft_import_plan_json();
        value
            .as_object_mut()
            .expect("plan fixture should be an object")
            .insert(forbidden.0.to_owned(), forbidden.1);
        assert!(
            serde_json::from_value::<DraftImportPlan>(value).is_err(),
            "DraftImportPlan JSON must reject raw provider field {}",
            forbidden.0
        );
    }

    let mut formula_leak = valid_draft_import_plan();
    formula_leak.tracks[0].track.segments[0]
        .filters
        .push(draft_model::Filter {
            name: "provider-formula".to_owned(),
            parameters: [("rawFormula".to_owned(), "{\"kaipai\":true}".to_owned())].into(),
        });
    let formula_error = validate_import_plan(&formula_leak)
        .expect_err("raw provider formula fields must not enter canonical filters");
    assert!(
        matches!(
            formula_error,
            DraftImportPlanValidationError::ProviderSemanticLeakage { .. }
        ),
        "expected provider leakage error, got {formula_error:?}"
    );
}

#[test]
fn draft_import_plan_rejects_invalid_shape_before_project_session_mutation() {
    let mut zero_duration = valid_draft_import_plan();
    zero_duration.tracks[0].track.segments[0]
        .target_timerange
        .duration = Microseconds::ZERO;
    assert!(
        matches!(
            validate_import_plan(&zero_duration),
            Err(DraftImportPlanValidationError::InvalidCanonicalDraft { .. })
        ),
        "invalid timeranges should fail through canonical draft validation"
    );

    let mut missing_material = valid_draft_import_plan();
    missing_material.tracks[0].track.segments[0].material_id = "material-missing".into();
    assert!(
        matches!(
            validate_import_plan(&missing_material),
            Err(DraftImportPlanValidationError::InvalidCanonicalDraft { .. })
        ),
        "missing material references should fail before session mutation"
    );

    let mut unsorted_tracks = valid_draft_import_plan();
    unsorted_tracks.tracks[0].z_order = 20;
    unsorted_tracks.tracks[1].z_order = 10;
    assert!(
        matches!(
            validate_import_plan(&unsorted_tracks),
            Err(DraftImportPlanValidationError::InvalidTrackOrdering { .. })
        ),
        "import tracks should be validated as sorted z-order before application"
    );

    let mut duplicate_z_order = valid_draft_import_plan();
    duplicate_z_order.tracks[1].z_order = duplicate_z_order.tracks[0].z_order;
    assert!(
        matches!(
            validate_import_plan(&duplicate_z_order),
            Err(DraftImportPlanValidationError::InvalidTrackOrdering { .. })
        ),
        "duplicate z-order entries would make layer order ambiguous"
    );
}

fn valid_draft_import_plan_json() -> serde_json::Value {
    serde_json::to_value(valid_draft_import_plan()).expect("fixture plan should serialize")
}

fn valid_draft_import_plan() -> DraftImportPlan {
    DraftImportPlan {
        schema_version: DraftImportPlanSchemaVersion::current(),
        import_id: "template-alpha".to_owned(),
        draft_id: "draft-import-alpha".into(),
        draft_name: "导入模板 Alpha".to_owned(),
        canvas_config: DraftCanvasConfig {
            aspect_ratio: CanvasAspectRatio::preset(
                draft_model::CanvasAspectRatioPreset::Ratio9x16,
            ),
            width: 1080,
            height: 1920,
            frame_rate: RationalFrameRate::new(30, 1),
            background: CanvasBackground::SolidColor {
                color: "#101010".to_owned(),
            },
            adaptation_policy: CanvasAdaptationPolicy::Manual,
        },
        materials: vec![
            import_material(
                "material-main-video",
                MaterialKind::Video,
                "resources/template-import/template-alpha/videos/main-video/source.mp4",
                "main-video.mp4",
                Some(5_000_000),
                true,
                true,
            ),
            import_material(
                "material-title",
                MaterialKind::Text,
                "text://material-title",
                "导入标题",
                Some(5_000_000),
                false,
                false,
            ),
            import_material(
                "material-bgm",
                MaterialKind::Audio,
                "resources/template-import/template-alpha/audio/bgm/source.mp3",
                "bgm.mp3",
                Some(5_000_000),
                false,
                true,
            ),
            import_material(
                "material-sticker",
                MaterialKind::Sticker,
                "resources/template-import/template-alpha/stickers/sticker/source.png",
                "sticker.png",
                Some(5_000_000),
                true,
                false,
            ),
        ],
        tracks: vec![
            ImportTrackPlan {
                z_order: 0,
                track: visual_track(),
            },
            ImportTrackPlan {
                z_order: 10,
                track: text_track(),
            },
            ImportTrackPlan {
                z_order: 20,
                track: audio_track(),
            },
        ],
    }
}

fn import_material(
    material_id: &str,
    kind: MaterialKind,
    uri: &str,
    display_name: &str,
    duration: Option<u64>,
    has_video: bool,
    has_audio: bool,
) -> ImportMaterialPlan {
    ImportMaterialPlan {
        material: Material {
            material_id: material_id.into(),
            kind,
            uri: uri.to_owned(),
            display_name: display_name.to_owned(),
            metadata: MaterialMetadata {
                duration: duration.map(Microseconds::new),
                width: if has_video { Some(1080) } else { None },
                height: if has_video { Some(1920) } else { None },
                frame_rate: if has_video {
                    Some(RationalFrameRate::new(30, 1))
                } else {
                    None
                },
                has_video,
                has_audio,
                audio_sample_rate: if has_audio { Some(48_000) } else { None },
                audio_channels: if has_audio { Some(2) } else { None },
                probe_error: None,
            },
            status: MaterialStatus::Available,
        },
    }
}

fn visual_track() -> Track {
    let mut segment = Segment::new(
        "segment-main-video",
        "material-main-video",
        SourceTimerange::new(0, 5_000_000),
        TargetTimerange::new(0, 5_000_000),
    );
    segment.main_track_magnet = MainTrackMagnet::enabled();
    segment.visual = SegmentVisual {
        transform: SegmentTransform {
            position: SegmentPosition { x: 120, y: -80 },
            scale: SegmentScale {
                x_millis: 1_100,
                y_millis: 1_100,
            },
            ..SegmentTransform::default()
        },
        ..SegmentVisual::default()
    };
    segment.transition = Some(Transition {
        name: "dissolve".to_owned(),
        duration: Microseconds::new(300_000),
    });
    segment.keyframes = vec![
        Keyframe {
            at: Microseconds::ZERO,
            property: KeyframeProperty::VisualOpacity,
            value: KeyframeValue::Uint { value: 500 },
            interpolation: KeyframeInterpolation::Linear,
            easing: KeyframeEasing::None,
        },
        Keyframe {
            at: Microseconds::new(1_000_000),
            property: KeyframeProperty::VisualOpacity,
            value: KeyframeValue::Uint { value: 1_000 },
            interpolation: KeyframeInterpolation::Linear,
            easing: KeyframeEasing::EaseOut,
        },
    ];

    let mut track = Track::new("track-main-video", TrackKind::Video, "主视频");
    track.segments.push(segment);
    track
}

fn text_track() -> Track {
    let mut segment = Segment::new(
        "segment-title",
        "material-title",
        SourceTimerange::new(0, 5_000_000),
        TargetTimerange::new(500_000, 3_000_000),
    );
    segment.text = Some(TextSegment {
        content: "导入标题".to_owned(),
        style: TextStyle {
            font: TextFont::bundled_default(),
            font_size: 64,
            color: "#f8f8f8".to_owned(),
            alignment: TextAlignment::Center,
            ..TextStyle::default()
        },
        layout_region: TextLayoutRegion {
            x_millis: 150,
            y_millis: 120,
            width_millis: 700,
            height_millis: 180,
        },
        wrapping: TextWrapping::Auto,
        ..TextSegment {
            content: "unused".to_owned(),
            source: Default::default(),
            style: TextStyle::default(),
            text_box: Default::default(),
            layout_region: Default::default(),
            wrapping: TextWrapping::Auto,
            bubble: None,
            effect: None,
        }
    });

    let mut track = Track::new("track-title", TrackKind::Text, "文字");
    track.segments.push(segment);
    track
}

fn audio_track() -> Track {
    let mut segment = Segment::new(
        "segment-bgm",
        "material-bgm",
        SourceTimerange::new(0, 5_000_000),
        TargetTimerange::new(0, 5_000_000),
    );
    segment.audio = SegmentAudio {
        gain_millis: 850,
        fade_in_duration: AudioFade {
            duration: Microseconds::new(500_000),
        },
        fade_out_duration: AudioFade {
            duration: Microseconds::new(750_000),
        },
        ..SegmentAudio::default()
    };

    let mut track = Track::new("track-bgm", TrackKind::Audio, "背景音乐");
    track.segments.push(segment);
    track
}

fn supported_report_item(
    category: AdaptationCategory,
    target_kind: AdaptationTargetKind,
    id: &str,
) -> AdaptationReportItem {
    AdaptationReportItem {
        status: AdaptationStatus::Supported,
        severity: AdaptationSeverity::Info,
        category,
        target: Some(AdaptationTargetRef {
            kind: target_kind,
            id: Some(id.to_owned()),
        }),
        message: "Canonical import plan field mapped without provider runtime semantics."
            .to_owned(),
        details: None,
        provenance: vec![ExternalProvenanceRef {
            source_kind: "kaipaiOfflineBundle".to_owned(),
            external_id: Some("template-alpha".to_owned()),
            external_path: Some("formula.canvas".to_owned()),
            note: Some("kept in report evidence only".to_owned()),
        }],
    }
}

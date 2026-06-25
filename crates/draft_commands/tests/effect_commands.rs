use draft_commands::{
    TimelineCommandErrorKind,
    effects::{apply_segment_effect, remove_segment_effect, update_segment_effect_parameter},
    history::{redo_timeline_edit, undo_timeline_edit},
    timeline::execute_timeline_edit,
};
use draft_model::{
    ApplySegmentEffectCommandPayload, CommandState, DirtyDomain, Draft, EffectParameterUpdate,
    Filter, FilterKind, Material, MaterialKind, Microseconds, RemoveSegmentEffectCommandPayload,
    Segment, SourceTimerange, TargetTimerange, TimelineEditPayload, TimelineSelection, Track,
    TrackKind, UpdateSegmentEffectParameterCommandPayload,
};

#[test]
fn effect_commands_apply_gaussian_and_basic_color_through_rust_owned_payloads() {
    let draft = draft_with_video_segment();
    let state = CommandState::empty();
    let selection = selected_segment_context();

    let blurred = apply_segment_effect(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        Filter::gaussian_blur(750),
    )
    .expect("supported Gaussian blur should commit");

    assert_eq!(blurred.events[0].kind, "segmentEffectApplied");
    assert_eq!(blurred.command_state.undo_stack.len(), 1);
    assert_eq!(
        blurred.command_state.undo_stack[0].label.as_deref(),
        Some("applySegmentEffect")
    );
    assert!(blurred.delta.changed_domains.contains(&DirtyDomain::Effect));
    assert!(blurred.delta.changed_domains.contains(&DirtyDomain::Filter));
    assert!(matches!(
        blurred.draft.tracks[0].segments[0].filters[0].kind,
        FilterKind::GaussianBlur { radius_millis: 750 }
    ));
    assert!(
        draft.tracks[0].segments[0].filters.is_empty(),
        "input draft must remain unchanged"
    );

    let colored = execute_timeline_edit(TimelineEditPayload::ApplySegmentEffect(
        ApplySegmentEffectCommandPayload {
            draft: blurred.draft,
            command_state: blurred.command_state,
            selection: blurred.selection,
            segment_id: "video-segment".into(),
            effect: Filter::basic_color_adjustment(-250, 1_200, 800),
        },
    ))
    .expect("timeline dispatcher should route basic color effect application");

    assert_eq!(colored.events[0].kind, "segmentEffectApplied");
    assert_eq!(colored.command_state.undo_stack.len(), 2);
    assert!(matches!(
        colored.draft.tracks[0].segments[0].filters[1].kind,
        FilterKind::BasicColorAdjustment {
            brightness_millis: -250,
            contrast_millis: 1_200,
            saturation_millis: 800,
        }
    ));
}

#[test]
fn effect_commands_update_enable_disable_remove_and_undo_redo_once_per_commit() {
    let draft = draft_with_video_segment();
    let state = CommandState::empty();
    let selection = selected_segment_context();

    let applied = apply_segment_effect(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        Filter::opacity_adjustment(1_000),
    )
    .expect("supported opacity adjustment should commit");

    let updated = update_segment_effect_parameter(
        &applied.draft,
        &applied.command_state,
        &applied.selection,
        "video-segment".into(),
        0,
        EffectParameterUpdate::OpacityMillis {
            opacity_millis: 450,
        },
    )
    .expect("opacity strength update should commit");
    assert_eq!(updated.events[0].kind, "segmentEffectParameterUpdated");
    assert_eq!(updated.command_state.undo_stack.len(), 2);
    assert_eq!(
        updated.command_state.undo_stack[1].label.as_deref(),
        Some("updateSegmentEffectParameter")
    );
    assert!(matches!(
        updated.draft.tracks[0].segments[0].filters[0].kind,
        FilterKind::OpacityAdjustment {
            opacity_millis: 450,
        }
    ));

    let disabled = execute_timeline_edit(TimelineEditPayload::UpdateSegmentEffectParameter(
        UpdateSegmentEffectParameterCommandPayload {
            draft: updated.draft,
            command_state: updated.command_state,
            selection: updated.selection,
            segment_id: "video-segment".into(),
            effect_index: 0,
            parameter: EffectParameterUpdate::Enabled { enabled: false },
        },
    ))
    .expect("timeline dispatcher should route enable/disable updates");
    assert!(!disabled.draft.tracks[0].segments[0].filters[0].enabled);
    assert_eq!(disabled.command_state.undo_stack.len(), 3);

    let undone = undo_timeline_edit(&disabled.draft, &disabled.command_state, &disabled.selection)
        .expect("effect enable update should be undoable");
    assert!(undone.draft.tracks[0].segments[0].filters[0].enabled);
    let redone = redo_timeline_edit(&undone.draft, &undone.command_state, &undone.selection)
        .expect("effect enable update should be redoable");
    assert_eq!(redone.draft, disabled.draft);

    let removed = execute_timeline_edit(TimelineEditPayload::RemoveSegmentEffect(
        RemoveSegmentEffectCommandPayload {
            draft: redone.draft,
            command_state: redone.command_state,
            selection: redone.selection,
            segment_id: "video-segment".into(),
            effect_index: 0,
        },
    ))
    .expect("timeline dispatcher should route effect removal");
    assert_eq!(removed.events[0].kind, "segmentEffectRemoved");
    assert!(removed.draft.tracks[0].segments[0].filters.is_empty());
    assert_eq!(removed.command_state.undo_stack.len(), 4);
}

#[test]
fn effect_commands_invalid_parameters_reject_atomically() {
    let draft = draft_with_video_segment();
    let state = CommandState::empty();
    let selection = selected_segment_context();

    for (label, effect) in [
        ("blur radius overflow", Filter::gaussian_blur(100_001)),
        (
            "brightness below range",
            Filter::basic_color_adjustment(-1_001, 1_000, 1_000),
        ),
        (
            "contrast overflow",
            Filter::basic_color_adjustment(0, 5_001, 1_000),
        ),
        ("opacity overflow", Filter::opacity_adjustment(1_001)),
    ] {
        let error = apply_segment_effect(
            &draft,
            &state,
            &selection,
            "video-segment".into(),
            effect,
        )
        .expect_err(label);
        assert!(
            matches!(
                error.kind,
                TimelineCommandErrorKind::InvalidEffectParameter { .. }
            ),
            "{label} should surface as typed parameter validation"
        );
        assert_eq!(draft, draft_with_video_segment(), "{label} mutated draft");
        assert_eq!(state, CommandState::empty(), "{label} mutated state");
        assert_eq!(selection, selected_segment_context(), "{label} mutated selection");
    }
}

#[test]
fn effect_commands_unsupported_external_references_are_diagnostics_not_supported_effects() {
    let draft = draft_with_video_segment();
    let state = CommandState::empty();
    let selection = selected_segment_context();

    let error = apply_segment_effect(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        Filter::external_reference("jianying", "private-effect-id"),
    )
    .expect_err("external provider effect must not commit as supported semantics");

    match error.kind {
        TimelineCommandErrorKind::UnsupportedEffect {
            capability_id,
            reason,
            ..
        } => {
            assert_eq!(capability_id, "external:jianying:private-effect-id");
            assert!(reason.contains("external"));
        }
        other => panic!("expected unsupported effect diagnostic, got {other:?}"),
    }
    assert!(draft.tracks[0].segments[0].filters.is_empty());
    assert_eq!(state, CommandState::empty());
}

#[test]
fn effect_commands_update_rejects_wrong_parameter_or_missing_effect_atomically() {
    let draft = draft_with_video_segment();
    let state = CommandState::empty();
    let selection = selected_segment_context();
    let applied = apply_segment_effect(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        Filter::gaussian_blur(500),
    )
    .expect("supported blur should commit");

    let wrong_parameter = update_segment_effect_parameter(
        &applied.draft,
        &applied.command_state,
        &applied.selection,
        "video-segment".into(),
        0,
        EffectParameterUpdate::OpacityMillis {
            opacity_millis: 500,
        },
    )
    .expect_err("opacity parameter cannot update Gaussian blur");
    assert!(matches!(
        wrong_parameter.kind,
        TimelineCommandErrorKind::InvalidEffectParameter { .. }
    ));

    let missing = remove_segment_effect(
        &applied.draft,
        &applied.command_state,
        &applied.selection,
        "video-segment".into(),
        4,
    )
    .expect_err("missing effect index should reject");
    assert!(matches!(
        missing.kind,
        TimelineCommandErrorKind::EffectNotFound { .. }
    ));

    assert_eq!(applied.draft.tracks[0].segments[0].filters.len(), 1);
    assert_eq!(applied.command_state.undo_stack.len(), 1);
}

fn draft_with_video_segment() -> Draft {
    let mut draft = Draft::new("effect-command-draft", "Effect Command Draft");
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        "media/video.mp4",
        "video.mp4",
    );
    material.metadata.duration = Some(Microseconds::new(2_000_000));
    draft.materials.push(material);

    let mut track = Track::new("video-track", TrackKind::Video, "Video");
    track.segments.push(Segment::new(
        "video-segment",
        "video-material",
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    ));
    draft.tracks.push(track);
    draft
}

fn selected_segment_context() -> TimelineSelection {
    TimelineSelection {
        segment_ids: vec!["video-segment".into()],
        track_ids: vec!["video-track".into()],
    }
}

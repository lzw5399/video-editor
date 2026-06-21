use draft_commands::{
    TimelineCommandErrorKind,
    history::{redo_timeline_edit, undo_timeline_edit},
    timeline::execute_timeline_edit,
    visual::update_segment_visual,
};
use draft_model::{
    CommandState, Draft, Material, MaterialKind, Microseconds, Segment, SegmentBackgroundFilling,
    SegmentOpacity, SegmentPosition, SegmentScale, SegmentVisual, SourceTimerange, TargetTimerange,
    TimelineEditPayload, TimelineSelection, Track, TrackKind, UpdateSegmentVisualCommandPayload,
};

#[test]
fn visual_transform_update_commits_after_validation() {
    let draft = draft_with_video_segment();
    let state = CommandState::empty();
    let selection = selected_segment_context();
    let visual = edited_visual();

    let response = update_segment_visual(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        visual.clone(),
    )
    .expect("valid visual transform should commit");

    assert_eq!(response.draft.tracks[0].segments[0].visual, visual);
    assert_eq!(
        draft.tracks[0].segments[0].visual,
        SegmentVisual::default(),
        "input draft stays unchanged"
    );
    assert_eq!(response.selection, selection);
    assert_eq!(response.events[0].kind, "segmentVisualUpdated");
    assert_eq!(response.command_state.undo_stack.len(), 1);
    assert_eq!(response.command_state.undo_stack[0].draft, draft);
    assert_eq!(
        response.command_state.undo_stack[0].label.as_deref(),
        Some("updateSegmentVisual")
    );
    assert!(response.command_state.redo_stack.is_empty());
}

#[test]
fn visual_transform_update_undo_redo_uses_existing_history() {
    let draft = draft_with_video_segment();
    let state = CommandState::empty();
    let selection = selected_segment_context();

    let updated = update_segment_visual(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        edited_visual(),
    )
    .expect("valid visual transform should commit");
    let undone = undo_timeline_edit(&updated.draft, &updated.command_state, &updated.selection)
        .expect("visual update should be undoable");
    assert_eq!(undone.draft, draft);
    assert_eq!(undone.selection, selection);
    assert_eq!(undone.events[0].kind, "undoCommitted");
    assert_eq!(undone.command_state.redo_stack.len(), 1);

    let redone = redo_timeline_edit(&undone.draft, &undone.command_state, &undone.selection)
        .expect("visual update should be redoable");
    assert_eq!(redone.draft, updated.draft);
    assert_eq!(redone.selection, updated.selection);
    assert_eq!(redone.events[0].kind, "redoCommitted");
    assert_eq!(redone.command_state.undo_stack.len(), 1);
    assert!(redone.command_state.redo_stack.is_empty());
}

#[test]
fn invalid_visual_transform_updates_are_atomic() {
    let draft = draft_with_video_segment();
    let state = CommandState::empty();
    let selection = selected_segment_context();

    for (label, visual) in [
        ("zero scale", visual_with_zero_scale()),
        ("opacity overflow", visual_with_opacity_overflow()),
        (
            "bad background color",
            visual_with_invalid_background_color(),
        ),
    ] {
        let error =
            update_segment_visual(&draft, &state, &selection, "video-segment".into(), visual)
                .expect_err(label);
        assert!(
            matches!(
                error.kind,
                TimelineCommandErrorKind::DraftValidationFailed { .. }
            ),
            "{label} should surface as draft validation failure"
        );
        assert_eq!(
            draft,
            draft_with_video_segment(),
            "{label} mutated input draft"
        );
        assert_eq!(
            state,
            CommandState::empty(),
            "{label} mutated command state"
        );
        assert_eq!(
            selection,
            selected_segment_context(),
            "{label} mutated selection"
        );
    }
}

#[test]
fn execute_timeline_edit_routes_visual_transform_command() {
    let draft = draft_with_video_segment();
    let state = CommandState::empty();
    let selection = selected_segment_context();
    let visual = edited_visual();

    let response = execute_timeline_edit(TimelineEditPayload::UpdateSegmentVisual(
        UpdateSegmentVisualCommandPayload {
            draft,
            command_state: state,
            selection,
            segment_id: "video-segment".into(),
            visual: visual.clone(),
        },
    ))
    .expect("timeline dispatcher should route visual updates");

    assert_eq!(response.draft.tracks[0].segments[0].visual, visual);
    assert_eq!(response.events[0].kind, "segmentVisualUpdated");
}

fn draft_with_video_segment() -> Draft {
    let mut draft = Draft::new("visual-command-draft", "Visual Command Draft");
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

fn edited_visual() -> SegmentVisual {
    let mut visual = SegmentVisual::default();
    visual.transform.position = SegmentPosition { x: 120, y: -80 };
    visual.transform.scale = SegmentScale {
        x_millis: 1_250,
        y_millis: 1_250,
    };
    visual.transform.opacity = SegmentOpacity { value_millis: 760 };
    visual
}

fn visual_with_zero_scale() -> SegmentVisual {
    let mut visual = edited_visual();
    visual.transform.scale.x_millis = 0;
    visual
}

fn visual_with_opacity_overflow() -> SegmentVisual {
    let mut visual = edited_visual();
    visual.transform.opacity.value_millis = 1_001;
    visual
}

fn visual_with_invalid_background_color() -> SegmentVisual {
    let mut visual = edited_visual();
    visual.background_filling = SegmentBackgroundFilling::SolidColor {
        color: "101820".to_owned(),
    };
    visual
}

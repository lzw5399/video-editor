use draft_commands::{
    TimelineCommandErrorKind,
    history::{redo_timeline_edit, undo_timeline_edit},
    keyframe::{remove_segment_keyframe, set_segment_keyframe},
    timeline::execute_timeline_edit,
};
use draft_model::{
    CommandState, Draft, Keyframe, KeyframeEasing, KeyframeInterpolation, KeyframeProperty,
    KeyframeValue, Material, MaterialKind, Microseconds, RemoveSegmentKeyframeCommandPayload,
    Segment, SetSegmentKeyframeCommandPayload, SourceTimerange, TargetTimerange,
    TimelineEditPayload, TimelineSelection, Track, TrackKind,
};

#[test]
fn set_keyframe_adds_replaces_sorts_and_preserves_selection() {
    let draft = draft_with_video_segment();
    let state = CommandState::empty();
    let selection = selected_segment_context();

    let opacity_at_mid = keyframe(
        500_000,
        KeyframeProperty::VisualOpacity,
        KeyframeValue::Uint { value: 500 },
    );
    let added = set_segment_keyframe(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        opacity_at_mid.clone(),
    )
    .expect("valid keyframe should commit");

    assert_eq!(added.events[0].kind, "segmentKeyframeSet");
    assert_eq!(added.selection, selection);
    assert_eq!(added.command_state.undo_stack.len(), 1);
    assert_eq!(
        added.command_state.undo_stack[0].label.as_deref(),
        Some("setSegmentKeyframe")
    );
    assert_eq!(
        added.draft.tracks[0].segments[0].keyframes,
        vec![opacity_at_mid.clone()]
    );
    assert!(
        draft.tracks[0].segments[0].keyframes.is_empty(),
        "input draft stays unchanged"
    );

    let position_at_head = keyframe(
        0,
        KeyframeProperty::VisualPositionX,
        KeyframeValue::Int { value: -120 },
    );
    let sorted = set_segment_keyframe(
        &added.draft,
        &added.command_state,
        &added.selection,
        "video-segment".into(),
        position_at_head.clone(),
    )
    .expect("second keyframe should commit");
    assert_eq!(
        sorted.draft.tracks[0].segments[0].keyframes,
        vec![position_at_head.clone(), opacity_at_mid.clone()]
    );

    let opacity_replacement = keyframe(
        500_000,
        KeyframeProperty::VisualOpacity,
        KeyframeValue::Uint { value: 900 },
    );
    let replaced = set_segment_keyframe(
        &sorted.draft,
        &sorted.command_state,
        &sorted.selection,
        "video-segment".into(),
        opacity_replacement.clone(),
    )
    .expect("same property/time should replace");

    assert_eq!(
        replaced.draft.tracks[0].segments[0].keyframes,
        vec![position_at_head, opacity_replacement]
    );
    assert_eq!(replaced.command_state.undo_stack.len(), 3);
}

#[test]
fn remove_keyframe_removes_only_matching_property_and_time() {
    let (draft, state, selection) = draft_with_two_keyframes();

    let removed = remove_segment_keyframe(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        KeyframeProperty::VisualOpacity,
        Microseconds::new(500_000),
    )
    .expect("existing keyframe should be removed");

    assert_eq!(removed.events[0].kind, "segmentKeyframeRemoved");
    assert_eq!(removed.selection, selection);
    assert_eq!(removed.command_state.undo_stack.len(), 1);
    assert_eq!(
        removed.command_state.undo_stack[0].label.as_deref(),
        Some("removeSegmentKeyframe")
    );
    assert_eq!(removed.draft.tracks[0].segments[0].keyframes.len(), 1);
    assert_eq!(
        removed.draft.tracks[0].segments[0].keyframes[0].property,
        KeyframeProperty::VisualPositionX
    );
    assert_eq!(
        draft.tracks[0].segments[0].keyframes.len(),
        2,
        "input draft stays unchanged"
    );
}

#[test]
fn keyframe_commands_are_undoable_and_redoable() {
    let draft = draft_with_video_segment();
    let state = CommandState::empty();
    let selection = selected_segment_context();

    let added = set_segment_keyframe(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        keyframe(
            500_000,
            KeyframeProperty::VisualOpacity,
            KeyframeValue::Uint { value: 500 },
        ),
    )
    .expect("set keyframe should commit");

    let undone = undo_timeline_edit(&added.draft, &added.command_state, &added.selection)
        .expect("set keyframe should enter undo history");
    assert_eq!(undone.draft, draft);
    assert_eq!(undone.selection, selection);

    let redone = redo_timeline_edit(&undone.draft, &undone.command_state, &undone.selection)
        .expect("set keyframe should enter redo history");
    assert_eq!(redone.draft, added.draft);
    assert_eq!(redone.selection, added.selection);
}

#[test]
fn invalid_keyframe_commands_reject_without_partial_mutation() {
    let draft = draft_with_video_segment();
    let mut locked_draft = draft.clone();
    locked_draft.tracks[0].locked = true;
    let state = CommandState::empty();
    let selection = selected_segment_context();

    let locked = set_segment_keyframe(
        &locked_draft,
        &state,
        &selection,
        "video-segment".into(),
        keyframe(
            500_000,
            KeyframeProperty::VisualOpacity,
            KeyframeValue::Uint { value: 500 },
        ),
    )
    .expect_err("locked track should reject");
    assert!(matches!(
        locked.kind,
        TimelineCommandErrorKind::LockedTrack { .. }
    ));

    let invalid_value = set_segment_keyframe(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        keyframe(
            500_000,
            KeyframeProperty::VisualOpacity,
            KeyframeValue::Color {
                value: "#ffffff".to_owned(),
            },
        ),
    )
    .expect_err("invalid property/value pairing should reject");
    assert!(matches!(
        invalid_value.kind,
        TimelineCommandErrorKind::DraftValidationFailed { .. }
    ));

    let invalid_time = set_segment_keyframe(
        &draft,
        &state,
        &selection,
        "video-segment".into(),
        keyframe(
            2_000_000,
            KeyframeProperty::VisualOpacity,
            KeyframeValue::Uint { value: 500 },
        ),
    )
    .expect_err("keyframe outside segment duration should reject");
    assert!(matches!(
        invalid_time.kind,
        TimelineCommandErrorKind::DraftValidationFailed { .. }
    ));

    assert_eq!(draft, draft_with_video_segment());
    assert_eq!(state, CommandState::empty());
    assert_eq!(selection, selected_segment_context());
}

#[test]
fn execute_timeline_edit_routes_keyframe_commands() {
    let draft = draft_with_video_segment();
    let state = CommandState::empty();
    let selection = selected_segment_context();
    let keyframe = keyframe(
        500_000,
        KeyframeProperty::VisualOpacity,
        KeyframeValue::Uint { value: 500 },
    );

    let added = execute_timeline_edit(TimelineEditPayload::SetSegmentKeyframe(
        SetSegmentKeyframeCommandPayload {
            draft,
            command_state: state,
            selection,
            segment_id: "video-segment".into(),
            keyframe: keyframe.clone(),
        },
    ))
    .expect("timeline dispatcher should route set keyframe");

    assert_eq!(added.draft.tracks[0].segments[0].keyframes, vec![keyframe]);
    assert_eq!(added.events[0].kind, "segmentKeyframeSet");

    let removed = execute_timeline_edit(TimelineEditPayload::RemoveSegmentKeyframe(
        RemoveSegmentKeyframeCommandPayload {
            draft: added.draft,
            command_state: added.command_state,
            selection: added.selection,
            segment_id: "video-segment".into(),
            property: KeyframeProperty::VisualOpacity,
            at: Microseconds::new(500_000),
        },
    ))
    .expect("timeline dispatcher should route remove keyframe");

    assert!(removed.draft.tracks[0].segments[0].keyframes.is_empty());
    assert_eq!(removed.events[0].kind, "segmentKeyframeRemoved");
}

fn draft_with_two_keyframes() -> (Draft, CommandState, TimelineSelection) {
    let mut draft = draft_with_video_segment();
    draft.tracks[0].segments[0].keyframes = vec![
        keyframe(
            0,
            KeyframeProperty::VisualPositionX,
            KeyframeValue::Int { value: -120 },
        ),
        keyframe(
            500_000,
            KeyframeProperty::VisualOpacity,
            KeyframeValue::Uint { value: 500 },
        ),
    ];
    (draft, CommandState::empty(), selected_segment_context())
}

fn keyframe(at: u64, property: KeyframeProperty, value: KeyframeValue) -> Keyframe {
    Keyframe {
        at: Microseconds::new(at),
        property,
        value,
        interpolation: KeyframeInterpolation::Linear,
        easing: KeyframeEasing::None,
    }
}

fn draft_with_video_segment() -> Draft {
    let mut draft = Draft::new("keyframe-command-draft", "Keyframe Command Draft");
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

use draft_commands::{
    history::{redo_timeline_edit, undo_timeline_edit},
    text::{add_text_segment, edit_text_segment},
};
use draft_model::{
    CommandState, Draft, MaterialKind, SourceTimerange, TargetTimerange, TextAlignment,
    TextBackground, TextSegment, TextShadow, TextStroke, TextStyle, TimelineSelection, Track,
    TrackKind,
};

#[test]
fn text_commands() {
    let draft = draft_with_text_track();
    let selection = TimelineSelection::empty();
    let state = CommandState::empty();

    let added = add_text_segment(
        &draft,
        &state,
        &selection,
        "text-track".into(),
        "text-segment".into(),
        "text-material".into(),
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
        text_segment("Hello", 36, TextAlignment::Center),
    )
    .expect("text segment should be first-class semantic draft data");

    assert_eq!(added.events[0].kind, "textSegmentAdded");
    assert_eq!(added.command_state.undo_stack.len(), 1);
    assert_eq!(added.draft.materials.len(), 1);
    assert_eq!(added.draft.materials[0].kind, MaterialKind::Text);
    assert!(
        added.draft.materials[0].uri.starts_with("text://"),
        "text may use an internal material source, but content must live on Segment.text"
    );

    let text_segment = added.draft.tracks[0].segments[0]
        .text
        .as_ref()
        .expect("text content should be persisted on the segment");
    assert_eq!(text_segment.content, "Hello");
    assert_eq!(text_segment.style.font_size, 36);
    assert_eq!(text_segment.style.color, "#ffffff");
    assert_eq!(text_segment.style.alignment, TextAlignment::Center);
    assert_eq!(text_segment.style.stroke.as_ref().unwrap().color, "#101010");
    assert_eq!(text_segment.style.shadow.as_ref().unwrap().offset_x, 2);
    assert_eq!(
        text_segment.style.background.as_ref().unwrap().color,
        "#000000"
    );

    let edited = edit_text_segment(
        &added.draft,
        &added.command_state,
        &added.selection,
        "text-segment".into(),
        text_segment_with_color("Edited", 42, TextAlignment::Right, "#ff00aa"),
    )
    .expect("editing text should update only semantic text fields");

    assert_eq!(edited.events[0].kind, "textSegmentEdited");
    assert_eq!(edited.command_state.undo_stack.len(), 2);
    let edited_segment = &edited.draft.tracks[0].segments[0];
    assert_eq!(
        edited_segment.source_timerange,
        SourceTimerange::new(0, 1_000_000)
    );
    assert_eq!(
        edited_segment.target_timerange,
        TargetTimerange::new(0, 1_000_000)
    );
    assert_eq!(edited_segment.text.as_ref().unwrap().content, "Edited");
    assert_eq!(edited_segment.text.as_ref().unwrap().style.color, "#ff00aa");

    let undone = undo_timeline_edit(&edited.draft, &edited.command_state, &edited.selection)
        .expect("text edit should enter undo history");
    assert_eq!(undone.draft, added.draft);
    let redone = redo_timeline_edit(&undone.draft, &undone.command_state, &undone.selection)
        .expect("text edit should enter redo history");
    assert_eq!(redone.draft, edited.draft);

    let rejected = edit_text_segment(
        &edited.draft,
        &edited.command_state,
        &edited.selection,
        "text-segment".into(),
        text_segment("", 36, TextAlignment::Center),
    )
    .expect_err("empty text content should reject without committing history");
    assert!(rejected.to_string().contains("text"));
    assert_eq!(edited.command_state.undo_stack.len(), 2);
}

fn draft_with_text_track() -> Draft {
    let mut draft = Draft::new("text-command-draft", "Text Commands");
    draft
        .tracks
        .push(Track::new("text-track", TrackKind::Text, "Text"));
    draft
}

fn text_segment(content: &str, font_size: u32, alignment: TextAlignment) -> TextSegment {
    text_segment_with_color(content, font_size, alignment, "#ffffff")
}

fn text_segment_with_color(
    content: &str,
    font_size: u32,
    alignment: TextAlignment,
    color: &str,
) -> TextSegment {
    TextSegment {
        content: content.to_owned(),
        style: TextStyle {
            font_size,
            color: color.to_owned(),
            alignment,
            stroke: Some(TextStroke {
                color: "#101010".to_owned(),
                width: 2,
            }),
            shadow: Some(TextShadow {
                color: "#202020".to_owned(),
                offset_x: 2,
                offset_y: 3,
                blur: 4,
            }),
            background: Some(TextBackground {
                color: "#000000".to_owned(),
            }),
        },
    }
}

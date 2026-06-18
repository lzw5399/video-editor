use draft_commands::{
    history::{redo_timeline_edit, undo_timeline_edit},
    text::import_subtitle_srt,
};
use draft_model::{
    CommandPayload, CommandState, Draft, ImportSubtitleSrtCommandPayload, Microseconds,
    TextAlignment, TextBox, TextLayoutRegion, TextSegmentSource, TextStyle, TextWrapping,
    TimelineSelection, Track, TrackKind,
};

#[test]
fn subtitle_srt_import_creates_text_track_and_segments_atomically() {
    let draft = Draft::new("subtitle-import-draft", "Subtitle Import");
    let selection = TimelineSelection::empty();
    let state = CommandState::empty();
    let payload = subtitle_payload(draft.clone(), state.clone(), selection.clone());

    let imported =
        import_subtitle_srt(payload).expect("valid SRT should import as subtitle text segments");

    assert_eq!(imported.events[0].kind, "subtitleSrtImported");
    assert_eq!(imported.command_state.undo_stack.len(), 1);
    assert_eq!(
        imported.command_state.undo_stack[0].label.as_deref(),
        Some("importSubtitleSrt")
    );
    assert_eq!(imported.selection.track_ids, vec!["subtitle-track".into()]);
    assert_eq!(
        imported.selection.segment_ids,
        vec!["subtitle-segment-1".into(), "subtitle-segment-2".into()]
    );

    let track = imported
        .draft
        .tracks
        .iter()
        .find(|track| track.track_id.as_str() == "subtitle-track")
        .expect("import should create the target text track when missing");
    assert_eq!(track.kind, TrackKind::Text);
    assert_eq!(track.name, "字幕");
    assert_eq!(track.segments.len(), 2);

    let first = &track.segments[0];
    assert_eq!(first.material_id.as_str(), "subtitle-material-1");
    assert_eq!(first.source_timerange.start, Microseconds::new(0));
    assert_eq!(
        first.source_timerange.duration,
        Microseconds::new(1_000_000)
    );
    assert_eq!(first.target_timerange.start, Microseconds::new(500_000));
    assert_eq!(
        first.target_timerange.duration,
        Microseconds::new(1_000_000)
    );
    let first_text = first
        .text
        .as_ref()
        .expect("subtitle text should be on Segment.text");
    assert_eq!(first_text.content, "第一行\n继续第一行");
    assert_eq!(first_text.source, TextSegmentSource::Subtitle);
    assert_eq!(first_text.style.font_size, 32);
    assert_eq!(first_text.style.color, "#ffee00");
    assert_eq!(first_text.text_box.width_millis, 700);
    assert_eq!(first_text.layout_region.y_millis, 650);
    assert_eq!(first_text.wrapping, TextWrapping::Auto);

    let second = &track.segments[1];
    assert_eq!(second.segment_id.as_str(), "subtitle-segment-2");
    assert_eq!(second.material_id.as_str(), "subtitle-material-2");
    assert_eq!(second.target_timerange.start, Microseconds::new(2_000_000));
    assert_eq!(second.target_timerange.duration, Microseconds::new(750_000));
    assert_eq!(second.text.as_ref().unwrap().content, "第二行");

    let undone = undo_timeline_edit(
        &imported.draft,
        &imported.command_state,
        &imported.selection,
    )
    .expect("subtitle import should undo as one command");
    assert_eq!(undone.draft, draft);
    let redone = redo_timeline_edit(&undone.draft, &undone.command_state, &undone.selection)
        .expect("subtitle import should redo as one command");
    assert_eq!(redone.draft, imported.draft);
}

#[test]
fn subtitle_srt_import_targets_existing_text_track() {
    let mut draft = Draft::new("subtitle-existing-track-draft", "Subtitle Existing Track");
    draft
        .tracks
        .push(Track::new("subtitle-track", TrackKind::Text, "现有字幕"));

    let imported = import_subtitle_srt(subtitle_payload(
        draft,
        CommandState::empty(),
        TimelineSelection::empty(),
    ))
    .expect("valid SRT should target an existing text track");

    assert_eq!(imported.draft.tracks.len(), 1);
    assert_eq!(imported.draft.tracks[0].name, "现有字幕");
    assert_eq!(imported.draft.tracks[0].segments.len(), 2);
}

#[test]
fn malformed_subtitle_srt_rejects_without_mutating_history_or_draft() {
    let mut payload = subtitle_payload(
        Draft::new("bad-subtitle-import-draft", "Bad Subtitle Import"),
        CommandState::empty(),
        TimelineSelection::empty(),
    );
    let original_draft = payload.draft.clone();
    let original_state = payload.command_state.clone();
    payload.srt_content = "1\n00:00:02,000 --> 00:00:01,000\n反向时间\n".to_owned();

    let rejected = import_subtitle_srt(payload)
        .expect_err("malformed SRT should reject before draft mutation");

    assert!(rejected.to_string().contains("SRT"));
    assert_eq!(original_draft.tracks.len(), 0);
    assert_eq!(original_state.undo_stack.len(), 0);
}

#[test]
fn import_subtitle_srt_payload_routes_through_timeline_command_executor() {
    let payload = subtitle_payload(
        Draft::new("subtitle-route-draft", "Subtitle Route"),
        CommandState::empty(),
        TimelineSelection::empty(),
    );

    let routed =
        draft_commands::timeline::execute_timeline_edit(CommandPayload::ImportSubtitleSrt(payload))
            .expect("importSubtitleSrt should route through timeline command execution");

    assert_eq!(routed.events[0].kind, "subtitleSrtImported");
    assert_eq!(routed.draft.tracks[0].segments.len(), 2);
}

fn subtitle_payload(
    draft: Draft,
    command_state: CommandState,
    selection: TimelineSelection,
) -> ImportSubtitleSrtCommandPayload {
    ImportSubtitleSrtCommandPayload {
        draft,
        command_state,
        selection,
        track_id: "subtitle-track".into(),
        track_name: "字幕".to_owned(),
        srt_content: "1\n00:00:00,000 --> 00:00:01,000\n第一行\n继续第一行\n\n2\n00:00:01,500 --> 00:00:02,250\n第二行\n"
            .to_owned(),
        time_offset: Microseconds::new(500_000),
        segment_id_prefix: "subtitle-segment".to_owned(),
        material_id_prefix: "subtitle-material".to_owned(),
        style: TextStyle {
            font_size: 32,
            color: "#ffee00".to_owned(),
            alignment: TextAlignment::Center,
            ..TextStyle::default()
        },
        text_box: TextBox {
            width_millis: 700,
            height_millis: 180,
        },
        layout_region: TextLayoutRegion {
            x_millis: 150,
            y_millis: 650,
            width_millis: 700,
            height_millis: 250,
        },
        wrapping: TextWrapping::Auto,
    }
}

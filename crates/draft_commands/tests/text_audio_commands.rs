use draft_commands::{
    audio::{add_audio_segment, set_segment_volume, set_track_mute, update_segment_audio},
    delta::audio_property_delta,
    history::{redo_timeline_edit, undo_timeline_edit},
    keyframe::set_segment_keyframe,
    text::{add_text_segment, edit_text_segment},
};
use draft_model::{
    CommandDeltaName, CommandState, DirtyDomain, Draft, Keyframe, KeyframeEasing,
    KeyframeInterpolation, KeyframeProperty, KeyframeValue, MAX_SEGMENT_VOLUME_MILLIS, Material,
    MaterialKind, Microseconds, SegmentAudio, SegmentVolume, SourceTimerange, TargetTimerange,
    TextAlignment, TextBackground, TextBox, TextLayoutRegion, TextSegment, TextSegmentSource,
    TextShadow, TextStroke, TextStyle, TextWrapping, TimelineSelection, Track, TrackKind,
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

    let added_text = added.draft.tracks[0].segments[0]
        .text
        .as_ref()
        .expect("text content should be persisted on the segment");
    assert_eq!(added_text.content, "Hello");
    assert_eq!(added_text.style.font_size, 36);
    assert_eq!(added_text.style.color, "#ffffff");
    assert_eq!(added_text.style.alignment, TextAlignment::Center);
    assert_eq!(added_text.style.stroke.as_ref().unwrap().color, "#101010");
    assert_eq!(added_text.style.shadow.as_ref().unwrap().offset_x, 2);
    assert_eq!(
        added_text.style.background.as_ref().unwrap().color,
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

#[test]
fn audio_commands() {
    assert_eq!(MAX_SEGMENT_VOLUME_MILLIS, 4_000);

    let draft = draft_with_audio_track();
    let selection = TimelineSelection::empty();
    let state = CommandState::empty();

    let added = add_audio_segment(
        &draft,
        &state,
        &selection,
        "audio-track".into(),
        "audio-segment".into(),
        "audio-material".into(),
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    )
    .expect("audio materials should be accepted on audio tracks");

    assert_eq!(added.events[0].kind, "audioSegmentAdded");
    assert_eq!(added.command_state.undo_stack.len(), 1);
    assert_eq!(
        added.draft.tracks[0].segments[0].volume,
        SegmentVolume::unity()
    );

    let volume_changed = set_segment_volume(
        &added.draft,
        &added.command_state,
        &added.selection,
        "audio-segment".into(),
        SegmentVolume {
            level_millis: 1_500,
        },
    )
    .expect("segment volume should be integer millivolume semantics");

    assert_eq!(volume_changed.events[0].kind, "segmentVolumeChanged");
    assert_eq!(
        volume_changed.draft.tracks[0].segments[0]
            .volume
            .level_millis,
        1_500
    );
    assert_eq!(
        volume_changed.draft.tracks[0].segments[0].audio.gain_millis, 1_500,
        "legacy volume command should update the canonical audio gain path"
    );
    assert_eq!(
        volume_changed.draft.tracks[0].segments[0].target_timerange,
        TargetTimerange::new(0, 1_000_000)
    );

    let muted = set_track_mute(
        &volume_changed.draft,
        &volume_changed.command_state,
        &volume_changed.selection,
        "audio-track".into(),
        true,
    )
    .expect("track mute should be a Rust command semantic");

    assert_eq!(muted.events[0].kind, "trackMuteChanged");
    assert!(muted.draft.tracks[0].muted);
    assert_eq!(muted.command_state.undo_stack.len(), 3);

    let undone = undo_timeline_edit(&muted.draft, &muted.command_state, &muted.selection)
        .expect("track mute should enter undo history");
    assert!(!undone.draft.tracks[0].muted);
    let redone = redo_timeline_edit(&undone.draft, &undone.command_state, &undone.selection)
        .expect("track mute should enter redo history");
    assert!(redone.draft.tracks[0].muted);

    let invalid_volume = set_segment_volume(
        &muted.draft,
        &muted.command_state,
        &muted.selection,
        "audio-segment".into(),
        SegmentVolume {
            level_millis: MAX_SEGMENT_VOLUME_MILLIS + 1,
        },
    )
    .expect_err("volume above the max should reject");
    assert!(invalid_volume.to_string().contains("volume"));
    assert_eq!(muted.command_state.undo_stack.len(), 3);

    let incompatible = add_audio_segment(
        &draft_with_video_track_and_audio_material(),
        &CommandState::empty(),
        &TimelineSelection::empty(),
        "video-track".into(),
        "bad-audio-segment".into(),
        "audio-material".into(),
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    )
    .expect_err("audio material should reject video track targets");
    assert!(incompatible.to_string().contains("incompatible"));
}

#[test]
fn audio_commands_update_segment_audio_atomically() {
    let (draft, state, selection) = draft_with_audio_segment();

    let updated = update_segment_audio(
        &draft,
        &state,
        &selection,
        "audio-segment".into(),
        Some(1_400),
        Some(-375),
        Some(Microseconds::new(120_000)),
        Some(Microseconds::new(80_000)),
        None,
    )
    .expect("valid audio semantic update should commit");

    let segment = &updated.draft.tracks[0].segments[0];
    assert_eq!(segment.volume.level_millis, 1_400);
    assert_eq!(segment.audio.gain_millis, 1_400);
    assert_eq!(segment.audio.pan_balance_millis.balance_millis, -375);
    assert_eq!(
        segment.audio.fade_in_duration.duration,
        Microseconds::new(120_000)
    );
    assert_eq!(
        segment.audio.fade_out_duration.duration,
        Microseconds::new(80_000)
    );
    assert_eq!(updated.events[0].kind, "segmentAudioUpdated");
    assert_eq!(updated.command_state.undo_stack.len(), 1);
    assert_audio_dirty_domains(&updated.delta);

    let invalid = update_segment_audio(
        &updated.draft,
        &updated.command_state,
        &updated.selection,
        "audio-segment".into(),
        Some(MAX_SEGMENT_VOLUME_MILLIS + 1),
        None,
        None,
        None,
        None,
    )
    .expect_err("invalid gain should reject");
    assert!(invalid.to_string().contains("gain") || invalid.to_string().contains("volume"));
    assert_eq!(
        updated.draft.tracks[0].segments[0].audio, segment.audio,
        "invalid update must not mutate the accepted draft"
    );
    assert_eq!(updated.command_state.undo_stack.len(), 1);
}

#[test]
fn audio_commands_update_segment_audio_rejects_locked_tracks() {
    let (mut draft, state, selection) = draft_with_audio_segment();
    draft.tracks[0].locked = true;

    let rejected = update_segment_audio(
        &draft,
        &state,
        &selection,
        "audio-segment".into(),
        Some(1_250),
        Some(100),
        None,
        None,
        None,
    )
    .expect_err("locked audio track should reject semantic audio edits");

    assert!(rejected.to_string().contains("locked"));
    assert_eq!(draft.tracks[0].segments[0].audio, SegmentAudio::default());
    assert!(state.undo_stack.is_empty());
}

#[test]
fn audio_commands_dirty_domains_cover_gain_pan_fades_keyframes_and_effect_slots() {
    let (draft, state, selection) = draft_with_audio_segment();
    let update = update_segment_audio(
        &draft,
        &state,
        &selection,
        "audio-segment".into(),
        Some(1_100),
        Some(250),
        Some(Microseconds::new(90_000)),
        Some(Microseconds::new(70_000)),
        Some(vec![]),
    )
    .expect("audio semantic update should commit");
    assert_audio_dirty_domains(&update.delta);

    let keyframe = set_segment_keyframe(
        &update.draft,
        &update.command_state,
        &update.selection,
        "audio-segment".into(),
        Keyframe {
            at: Microseconds::new(250_000),
            property: KeyframeProperty::Volume,
            value: KeyframeValue::Uint { value: 900 },
            interpolation: KeyframeInterpolation::Linear,
            easing: KeyframeEasing::None,
        },
    )
    .expect("volume keyframe should commit");
    assert_audio_dirty_domains(&keyframe.delta);

    let direct_delta = audio_property_delta(
        CommandDeltaName::UpdateSegmentAudio,
        &"audio-track".into(),
        &keyframe.draft.tracks[0].segments[0],
        "effect slot classification changed",
    );
    assert_audio_dirty_domains(&direct_delta);
}

fn draft_with_text_track() -> Draft {
    let mut draft = Draft::new("text-command-draft", "Text Commands");
    draft
        .tracks
        .push(Track::new("text-track", TrackKind::Text, "Text"));
    draft
}

fn draft_with_audio_track() -> Draft {
    let mut draft = Draft::new("audio-command-draft", "Audio Commands");
    draft.materials.push(material_with_duration(
        "audio-material",
        MaterialKind::Audio,
        "media/audio.wav",
        2_000_000,
    ));
    draft
        .tracks
        .push(Track::new("audio-track", TrackKind::Audio, "Audio"));
    draft
}

fn draft_with_audio_segment() -> (Draft, CommandState, TimelineSelection) {
    let mut draft = draft_with_audio_track();
    draft.tracks[0].segments.push(draft_model::Segment::new(
        "audio-segment",
        "audio-material",
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    ));
    (
        draft,
        CommandState::empty(),
        TimelineSelection {
            segment_ids: vec!["audio-segment".into()],
            track_ids: vec!["audio-track".into()],
        },
    )
}

fn assert_audio_dirty_domains(delta: &draft_model::CommandDelta) {
    for domain in [
        DirtyDomain::Audio,
        DirtyDomain::ExportPrep,
        DirtyDomain::Waveform,
        DirtyDomain::GraphSnapshot,
        DirtyDomain::PreviewCache,
    ] {
        assert!(
            delta.changed_domains.contains(&domain),
            "changed domains should include {domain:?}: {:?}",
            delta.changed_domains
        );
        assert!(
            delta.invalidation.consumer_domains.contains(&domain),
            "consumer domains should include {domain:?}: {:?}",
            delta.invalidation.consumer_domains
        );
    }
}

fn draft_with_video_track_and_audio_material() -> Draft {
    let mut draft = Draft::new("bad-audio-command-draft", "Bad Audio Commands");
    draft.materials.push(material_with_duration(
        "audio-material",
        MaterialKind::Audio,
        "media/audio.wav",
        2_000_000,
    ));
    draft
        .tracks
        .push(Track::new("video-track", TrackKind::Video, "Video"));
    draft
}

fn material_with_duration(
    material_id: &str,
    kind: MaterialKind,
    uri: &str,
    duration: u64,
) -> Material {
    let mut material = Material::new(material_id, kind, uri, material_id);
    material.metadata.duration = Some(Microseconds::new(duration));
    material
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
        source: TextSegmentSource::Text,
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
            ..TextStyle::default()
        },
        text_box: TextBox::default(),
        layout_region: TextLayoutRegion::default(),
        wrapping: TextWrapping::default(),
        bubble: None,
        effect: None,
    }
}

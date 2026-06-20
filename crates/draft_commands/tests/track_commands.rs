use draft_commands::{
    TimelineCommandErrorKind,
    audio::set_track_mute,
    history::{redo_timeline_edit, undo_timeline_edit},
    timeline::{
        add_segment, add_track, audio_track_mix_order, rename_track, set_track_lock,
        set_track_visibility, visual_track_stack_order,
    },
};
use draft_model::{
    CommandName, CommandState, DirtyDomain, Draft, Material, MaterialKind, Microseconds,
    SourceTimerange, TargetTimerange, TimelineSelection, TrackKind,
};

#[test]
fn track_commands_create_rename_lock_visibility_mute_and_undo() {
    let draft = draft_with_materials();

    let added_video = add_track(
        &draft,
        &CommandState::empty(),
        &TimelineSelection::empty(),
        "video-main".into(),
        TrackKind::Video,
        "主视频".to_owned(),
    )
    .expect("valid video track should be created by Rust command semantics");
    assert_eq!(added_video.events[0].kind, "trackAdded");
    assert_eq!(added_video.delta.command, CommandName::AddTrack);
    assert!(
        added_video
            .delta
            .changed_domains
            .contains(&DirtyDomain::Track)
    );
    assert_eq!(added_video.selection.track_ids, vec!["video-main".into()]);
    assert_eq!(added_video.draft.tracks[0].kind, TrackKind::Video);
    assert_eq!(added_video.draft.tracks[0].name, "主视频");
    assert!(added_video.draft.tracks[0].visible);
    assert!(!added_video.draft.tracks[0].locked);
    assert!(!added_video.draft.tracks[0].muted);

    let added_audio = add_track(
        &added_video.draft,
        &added_video.command_state,
        &added_video.selection,
        "audio-bgm".into(),
        TrackKind::Audio,
        "背景音乐".to_owned(),
    )
    .expect("valid audio track should be created");
    let added_text = add_track(
        &added_audio.draft,
        &added_audio.command_state,
        &added_audio.selection,
        "text-title".into(),
        TrackKind::Text,
        "标题".to_owned(),
    )
    .expect("valid text track should be created");
    assert_eq!(
        visual_track_stack_order(&added_text.draft),
        vec!["video-main".into(), "text-title".into()]
    );
    assert_eq!(
        audio_track_mix_order(&added_text.draft),
        vec!["audio-bgm".into()]
    );

    let renamed = rename_track(
        &added_text.draft,
        &added_text.command_state,
        &added_text.selection,
        "video-main".into(),
        "主画面".to_owned(),
    )
    .expect("track rename should be a Rust command semantic");
    assert_eq!(renamed.events[0].kind, "trackRenamed");
    assert_eq!(renamed.draft.tracks[0].name, "主画面");
    assert_eq!(renamed.delta.command, CommandName::RenameTrack);

    let locked = set_track_lock(
        &renamed.draft,
        &renamed.command_state,
        &renamed.selection,
        "video-main".into(),
        true,
    )
    .expect("track lock should be command-owned");
    assert_eq!(locked.events[0].kind, "trackLockChanged");
    assert!(locked.draft.tracks[0].locked);

    let rejected = add_segment(
        &locked.draft,
        &locked.command_state,
        &locked.selection,
        "video-main".into(),
        "locked-segment".into(),
        "video-material".into(),
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    )
    .expect_err("locked tracks reject segment mutations");
    assert_eq!(
        rejected.kind,
        TimelineCommandErrorKind::LockedTrack {
            track_id: "video-main".into()
        }
    );

    let unlocked = set_track_lock(
        &locked.draft,
        &locked.command_state,
        &locked.selection,
        "video-main".into(),
        false,
    )
    .expect("unlock should also be command-owned");
    let with_segment = add_segment(
        &unlocked.draft,
        &unlocked.command_state,
        &unlocked.selection,
        "video-main".into(),
        "visible-segment".into(),
        "video-material".into(),
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    )
    .expect("unlocked visible track should accept compatible segment");

    let hidden = set_track_visibility(
        &with_segment.draft,
        &with_segment.command_state,
        &with_segment.selection,
        "video-main".into(),
        false,
    )
    .expect("visual track visibility should be command-owned");
    assert_eq!(hidden.events[0].kind, "trackVisibilityChanged");
    assert!(!hidden.draft.tracks[0].visible);
    assert_eq!(hidden.delta.command, CommandName::SetTrackVisibility);
    assert!(hidden.delta.changed_domains.contains(&DirtyDomain::Visual));
    assert_eq!(
        visual_track_stack_order(&hidden.draft),
        vec!["text-title".into()]
    );

    let visibility_undone =
        undo_timeline_edit(&hidden.draft, &hidden.command_state, &hidden.selection)
            .expect("track visibility should enter undo history");
    assert!(visibility_undone.draft.tracks[0].visible);
    let visibility_redone = redo_timeline_edit(
        &visibility_undone.draft,
        &visibility_undone.command_state,
        &visibility_undone.selection,
    )
    .expect("track visibility should enter redo history");
    assert!(!visibility_redone.draft.tracks[0].visible);

    let muted = set_track_mute(
        &visibility_redone.draft,
        &visibility_redone.command_state,
        &visibility_redone.selection,
        "audio-bgm".into(),
        true,
    )
    .expect("audio track mute should remain command-owned");
    assert_eq!(muted.events[0].kind, "trackMuteChanged");
    assert!(muted.draft.tracks[1].muted);
}

#[test]
fn target_track_segment_add_and_ordering_are_deterministic() {
    let draft = draft_with_materials();
    let state = CommandState::empty();
    let selection = TimelineSelection::empty();
    let draft = add_track(
        &draft,
        &state,
        &selection,
        "video-base".into(),
        TrackKind::Video,
        "底层视频".to_owned(),
    )
    .unwrap()
    .draft;
    let draft = add_track(
        &draft,
        &state,
        &selection,
        "video-overlay".into(),
        TrackKind::Video,
        "叠加视频".to_owned(),
    )
    .unwrap()
    .draft;
    let draft = add_track(
        &draft,
        &state,
        &selection,
        "text-title".into(),
        TrackKind::Text,
        "文字".to_owned(),
    )
    .unwrap()
    .draft;
    let draft = add_track(
        &draft,
        &state,
        &selection,
        "audio-a".into(),
        TrackKind::Audio,
        "人声".to_owned(),
    )
    .unwrap()
    .draft;
    let draft = add_track(
        &draft,
        &state,
        &selection,
        "audio-b".into(),
        TrackKind::Audio,
        "音乐".to_owned(),
    )
    .unwrap()
    .draft;

    let added_to_overlay = add_segment(
        &draft,
        &state,
        &TimelineSelection {
            segment_ids: Vec::new(),
            track_ids: vec!["video-overlay".into()],
        },
        "video-overlay".into(),
        "overlay-segment".into(),
        "video-material".into(),
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    )
    .expect("add segment targets the selected compatible track id");
    assert!(added_to_overlay.draft.tracks[0].segments.is_empty());
    assert_eq!(added_to_overlay.draft.tracks[1].segments.len(), 1);
    assert_eq!(
        added_to_overlay.selection.track_ids,
        vec!["video-overlay".into()]
    );
    assert_eq!(
        visual_track_stack_order(&added_to_overlay.draft),
        vec![
            "video-base".into(),
            "video-overlay".into(),
            "text-title".into()
        ]
    );
    assert_eq!(
        audio_track_mix_order(&added_to_overlay.draft),
        vec!["audio-a".into(), "audio-b".into()]
    );
}

fn draft_with_materials() -> Draft {
    let mut draft = Draft::new("track-command-draft", "Track Commands");
    let mut video = Material::new(
        "video-material",
        MaterialKind::Video,
        "media/video.mp4",
        "Video",
    );
    video.metadata.duration = Some(Microseconds::new(3_000_000));
    video.metadata.width = Some(1920);
    video.metadata.height = Some(1080);
    let mut audio = Material::new(
        "audio-material",
        MaterialKind::Audio,
        "media/audio.wav",
        "Audio",
    );
    audio.metadata.duration = Some(Microseconds::new(3_000_000));
    draft.materials.push(video);
    draft.materials.push(audio);
    draft
}

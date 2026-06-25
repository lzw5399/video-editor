use bindings_node::{
    close_project_session, close_realtime_preview_session, create_audio_preview_session,
    create_project_session, create_realtime_preview_session, execute_command,
    execute_project_intent, get_audio_preview_status, import_kaipai_formula_bundle,
    list_project_session_materials, list_project_session_missing_materials, open_project_session,
    seek_audio_preview, start_project_session_export, stop_audio_preview,
    update_realtime_preview_project_session_snapshot,
};
use draft_model::{
    Draft, Filter, Material, MaterialKind, Microseconds, Segment, SegmentId, SourceTimerange,
    TargetTimerange, TextSegmentSource, Track, TrackKind, TrackTransition,
};
use editor_runtime::project_session_node::{
    force_material_probe_enqueue_failure_for_tests,
    force_material_probe_worker_spawn_failure_for_tests,
};
use media_runtime::{
    discover_runtime_config, replace_configured_bundled_runtime_directory_for_tests,
};
use media_runtime_desktop::DesktopFfmpegExecutor;
use project_store::{StdPlatformFileSystem, open_project_bundle, save_project_bundle};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use testkit::generate_video_material_fixture;

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn project_session_binding_delegates_lifecycle_authority_to_editor_runtime() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(crate_dir.join("Cargo.toml"))
        .expect("bindings_node manifest should be readable");
    assert!(
        manifest.contains("editor_runtime"),
        "bindings_node must depend on editor_runtime instead of owning project-session semantics"
    );

    let adapter = fs::read_to_string(crate_dir.join("src/project_session_service.rs"))
        .expect("project session adapter source should be readable");
    assert!(
        adapter.contains("editor_runtime"),
        "project-session adapter should delegate to editor_runtime"
    );
    for forbidden in [
        "struct ProjectSessionRegistry",
        "struct ProjectSession {",
        "struct ActiveProjectInteraction",
        "draft_commands::timeline::execute_timeline_edit",
        "project_store::create_project_bundle",
        "project_store::open_project_bundle",
        "project_store::save_project_bundle",
    ] {
        assert!(
            !adapter.contains(forbidden),
            "bindings_node project session adapter still owns runtime semantics: {forbidden}"
        );
    }
}

#[test]
fn project_session_creates_project_without_renderer_draft() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-create.veproj");

    let created = create_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-create",
        "draftId": "session-created-draft",
        "draftName": "Session Created Project"
    }))
    .expect("createProjectSession should return an envelope");
    assert_eq!(created["ok"], true, "{created:#}");
    assert_eq!(created["data"]["sessionId"], "test-session-create");
    assert_eq!(created["data"]["revision"], 0);
    assert_no_renderer_project_state_payload(&created);
    assert_eq!(
        created["data"]["viewModel"]["timeline"]["rows"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        created["data"]["viewModel"]["project"]["draftName"],
        "Session Created Project"
    );
    assert_eq!(
        created["data"]["viewModel"]["project"]["canvasConfig"]["width"],
        1920
    );
    assert_eq!(created["data"]["viewModel"]["project"]["trackCount"], 3);
    assert_eq!(created["data"]["viewModel"]["project"]["materialCount"], 0);
    assert_eq!(
        created["data"]["viewModel"]["project"]["sequenceDuration"],
        0
    );
    assert_eq!(
        created["data"]["viewModel"]["project"]["frameDuration"],
        33333
    );
    assert_edit_controls(
        &created["data"]["viewModel"],
        false,
        false,
        true,
        false,
        false,
    );
    assert_eq!(
        created["data"]["viewModel"]["timeline"]["rows"][0]["selectionHandle"],
        "timeline-track:track-main-video"
    );
    assert_eq!(
        created["data"]["viewModel"]["timeline"]["rows"][0]["kindLabel"],
        "视频"
    );

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("created session should save canonical project.json");
    assert_eq!(
        reopened.bundle.draft.draft_id.as_str(),
        "session-created-draft"
    );
    assert!(reopened.bundle.draft.materials.is_empty());
    assert_eq!(reopened.bundle.draft.tracks.len(), 3);
    assert_eq!(
        reopened.bundle.draft.tracks[0].track_id.as_str(),
        "track-main-video"
    );
    assert_eq!(
        reopened.bundle.draft.tracks[1].track_id.as_str(),
        "track-bgm"
    );
    assert_eq!(
        reopened.bundle.draft.tracks[2].track_id.as_str(),
        "track-title"
    );

    close_project_session(json!({ "sessionId": "test-session-create" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_add_timeline_segment_intent_persists_without_renderer_draft() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-add.veproj");
    save_timeline_draft(&bundle_path);

    let opened = open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-add"
    }))
    .expect("openProjectSession should return an envelope");
    assert_eq!(opened["ok"], true, "{opened:#}");
    assert_eq!(opened["data"]["sessionId"], "test-session-add");
    assert_eq!(opened["data"]["revision"], 0);
    assert_no_renderer_project_state_payload(&opened);
    assert_eq!(
        opened["data"]["viewModel"]["timeline"]["rows"][0]["selectionHandle"],
        "timeline-track:video-track"
    );

    let added = execute_project_intent(json!({
        "sessionId": "test-session-add",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("executeProjectIntent should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");
    assert_eq!(added["data"]["revision"], 1);
    assert_eq!(added["data"]["events"][0]["kind"], "segmentAdded");
    assert_no_renderer_project_state_payload(&added);
    assert_eq!(
        added["data"]["viewModel"]["timeline"]["rows"][0]["segments"][0]["selectionHandle"],
        "timeline-segment:video-track:segment-1"
    );
    assert!(
        added["data"]["viewModel"]["timeline"]["rows"][0]
            .get("track")
            .is_none(),
        "timeline rows must not expose raw Track payloads: {added:#}"
    );
    assert_eq!(
        added["data"]["viewModel"]["timeline"]["rows"][0]["name"],
        "Video"
    );
    assert_eq!(
        added["data"]["viewModel"]["timeline"]["capabilities"]["hasAudioTrack"],
        false
    );
    assert!(
        added["data"]["viewModel"]["timeline"]["rows"][0]["segments"][0]
            .get("segment")
            .is_none(),
        "timeline segment views must not expose raw Segment payloads: {added:#}"
    );
    assert_eq!(
        added["data"]["viewModel"]["timeline"]["rows"][0]["segments"][0]["segmentKey"],
        "segment-1"
    );
    assert_eq!(
        added["data"]["viewModel"]["timeline"]["rows"][0]["segments"][0]["targetLabel"],
        "目标 00:00:00.000 / 00:00:01.000"
    );
    assert_eq!(
        added["data"]["viewModel"]["project"]["sequenceDuration"],
        1_000_000
    );
    assert_eq!(added["data"]["viewModel"]["project"]["trackCount"], 1);
    assert_eq!(added["data"]["viewModel"]["project"]["materialCount"], 1);
    assert_edit_controls(&added["data"]["viewModel"], true, false, true, true, true);
    assert!(
        added["data"]["viewModel"]["selectedSegment"]
            .get("segment")
            .is_none(),
        "selected segment view must not expose raw Segment payloads: {added:#}"
    );
    assert_eq!(
        added["data"]["viewModel"]["selectedSegment"]["segmentKey"],
        "segment-1"
    );

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("session intent should save canonical project.json");
    assert_eq!(reopened.bundle.draft.tracks[0].segments.len(), 1);
    assert_eq!(
        reopened.bundle.draft.tracks[0].segments[0]
            .segment_id
            .as_str(),
        "segment-1"
    );

    close_project_session(json!({ "sessionId": "test-session-add" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_add_timeline_segment_uses_session_playhead() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-add-playhead.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-add-playhead"
    }))
    .expect("openProjectSession should return an envelope");

    let positioned = execute_project_intent(json!({
        "sessionId": "test-session-add-playhead",
        "expectedRevision": 0,
        "intent": {
            "kind": "setSessionPlayhead",
            "playhead": 450_000
        }
    }))
    .expect("session playhead intent should return an envelope");
    assert_eq!(positioned["ok"], true, "{positioned:#}");
    assert_eq!(positioned["data"]["revision"], 0);

    let added = execute_project_intent(json!({
        "sessionId": "test-session-add-playhead",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("add intent should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");
    assert_eq!(added["data"]["revision"], 1);
    assert_no_renderer_project_state_payload(&added);
    assert_eq!(
        added["data"]["viewModel"]["selectedSegment"]["targetTimerange"]["start"],
        450_000
    );
    assert_eq!(
        added["data"]["viewModel"]["selectedSegment"]["targetTimerange"]["duration"],
        1_000_000
    );

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("session add at playhead should save canonical project.json");
    assert_eq!(
        reopened.bundle.draft.tracks[0].segments[0]
            .target_timerange
            .start
            .get(),
        450_000
    );

    close_project_session(json!({ "sessionId": "test-session-add-playhead" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_add_intent_accepts_atomic_drop_placement_handle() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-add-drop-placement.veproj");
    save_multimedia_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-add-drop-placement"
    }))
    .expect("openProjectSession should return an envelope");

    let added = execute_project_intent(json!({
        "sessionId": "test-session-add-drop-placement",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material",
            "targetStart": 450_000,
            "targetTrackHandle": "timeline-track:video-track"
        }
    }))
    .expect("add drop placement payload should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");
    assert_no_renderer_project_state_payload(&added);
    assert_eq!(
        added["data"]["viewModel"]["selectedSegment"]["targetTimerange"]["start"],
        450_000
    );

    let rejected = execute_project_intent(json!({
        "sessionId": "test-session-add-drop-placement",
        "expectedRevision": 1,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material",
            "targetStart": 550_000,
            "targetTrackHandle": "timeline-track:audio-track"
        }
    }))
    .expect("wrong track add drop placement payload should return an envelope");
    assert_eq!(rejected["ok"], false, "{rejected:#}");
    assert_eq!(rejected["data"], Value::Null);
    assert_eq!(rejected["error"]["kind"], "invalidTimelineEdit");

    close_project_session(json!({ "sessionId": "test-session-add-drop-placement" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_add_text_audio_subtitle_use_session_playhead_and_core_timing() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-add-media-timing.veproj");
    save_multimedia_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-add-media-timing"
    }))
    .expect("openProjectSession should return an envelope");

    let positioned_text = execute_project_intent(json!({
        "sessionId": "test-session-add-media-timing",
        "expectedRevision": 0,
        "intent": {
            "kind": "setSessionPlayhead",
            "playhead": 450_000
        }
    }))
    .expect("session playhead intent should return an envelope");
    assert_eq!(positioned_text["ok"], true, "{positioned_text:#}");
    assert_eq!(positioned_text["data"]["revision"], 0);

    let text_added = execute_project_intent(json!({
        "sessionId": "test-session-add-media-timing",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTextSegmentIntent",
            "content": "播放头文字"
        }
    }))
    .expect("text add intent should return an envelope");
    assert_eq!(text_added["ok"], true, "{text_added:#}");
    assert_eq!(text_added["data"]["revision"], 1);
    assert_no_renderer_project_state_payload(&text_added);
    assert_eq!(
        text_added["data"]["viewModel"]["selectedSegment"]["targetTimerange"]["start"],
        450_000
    );
    assert_eq!(
        text_added["data"]["viewModel"]["selectedSegment"]["targetTimerange"]["duration"],
        3_000_000
    );

    let positioned_audio = execute_project_intent(json!({
        "sessionId": "test-session-add-media-timing",
        "expectedRevision": 1,
        "intent": {
            "kind": "setSessionPlayhead",
            "playhead": 550_000
        }
    }))
    .expect("session playhead intent should return an envelope");
    assert_eq!(positioned_audio["ok"], true, "{positioned_audio:#}");
    assert_eq!(positioned_audio["data"]["revision"], 1);

    let audio_added = execute_project_intent(json!({
        "sessionId": "test-session-add-media-timing",
        "expectedRevision": 1,
        "intent": {
            "kind": "addAudioSegmentIntent",
            "materialId": "audio-material"
        }
    }))
    .expect("audio add intent should return an envelope");
    assert_eq!(audio_added["ok"], true, "{audio_added:#}");
    assert_eq!(audio_added["data"]["revision"], 2);
    assert_no_renderer_project_state_payload(&audio_added);
    assert_eq!(
        audio_added["data"]["viewModel"]["selectedSegment"]["targetTimerange"]["start"],
        550_000
    );
    assert_eq!(
        audio_added["data"]["viewModel"]["selectedSegment"]["targetTimerange"]["duration"],
        2_000_000
    );

    let positioned_subtitle = execute_project_intent(json!({
        "sessionId": "test-session-add-media-timing",
        "expectedRevision": 2,
        "intent": {
            "kind": "setSessionPlayhead",
            "playhead": 650_000
        }
    }))
    .expect("session playhead intent should return an envelope");
    assert_eq!(positioned_subtitle["ok"], true, "{positioned_subtitle:#}");
    assert_eq!(positioned_subtitle["data"]["revision"], 2);

    let subtitle_added = execute_project_intent(json!({
        "sessionId": "test-session-add-media-timing",
        "expectedRevision": 2,
        "intent": {
            "kind": "importSubtitleSrtIntent",
            "srtContent": "1\n00:00:00,000 --> 00:00:01,000\n播放头字幕\n"
        }
    }))
    .expect("subtitle import intent should return an envelope");
    assert_eq!(subtitle_added["ok"], true, "{subtitle_added:#}");
    assert_eq!(subtitle_added["data"]["revision"], 3);
    assert_no_renderer_project_state_payload(&subtitle_added);

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("session add media timing should save canonical project.json");
    let text_track = reopened
        .bundle
        .draft
        .tracks
        .iter()
        .find(|track| track.track_id.as_str() == "text-track")
        .expect("text track should exist");
    assert_eq!(text_track.segments.len(), 1);
    assert_eq!(text_track.segments[0].target_timerange.start.get(), 450_000);
    assert_eq!(
        text_track.segments[0].target_timerange.duration.get(),
        3_000_000
    );
    let text = text_track.segments[0]
        .text
        .as_ref()
        .expect("text segment should include Rust-owned text payload");
    assert_eq!(text.content, "播放头文字");
    assert_eq!(text.source, TextSegmentSource::Text);
    assert_eq!(text.style.font_size, 36);
    assert_eq!(text.style.color, "#ffffff");
    assert_eq!(text.style.stroke.as_ref().unwrap().width, 2);
    assert_eq!(text.text_box.width_millis, 800);
    assert_eq!(text.layout_region.y_millis, 100);

    let audio_track = reopened
        .bundle
        .draft
        .tracks
        .iter()
        .find(|track| track.track_id.as_str() == "audio-track")
        .expect("audio track should exist");
    assert_eq!(audio_track.segments.len(), 1);
    assert_eq!(
        audio_track.segments[0].target_timerange.start.get(),
        550_000
    );
    assert_eq!(
        audio_track.segments[0].target_timerange.duration.get(),
        2_000_000
    );

    let subtitle_track = reopened
        .bundle
        .draft
        .tracks
        .iter()
        .find(|track| track.name == "字幕")
        .expect("subtitle track should be created");
    assert_eq!(subtitle_track.segments.len(), 1);
    assert_eq!(
        subtitle_track.segments[0].target_timerange.start.get(),
        650_000
    );
    assert_eq!(
        subtitle_track.segments[0].target_timerange.duration.get(),
        1_000_000
    );
    let subtitle = subtitle_track.segments[0]
        .text
        .as_ref()
        .expect("subtitle segment should include Rust-owned text payload");
    assert_eq!(subtitle.content, "播放头字幕");
    assert_eq!(subtitle.source, TextSegmentSource::Subtitle);
    assert_eq!(subtitle.style.font_size, 36);
    assert_eq!(subtitle.style.color, "#ffffff");
    assert_eq!(subtitle.style.stroke.as_ref().unwrap().color, "#000000");
    assert_eq!(subtitle.style.stroke.as_ref().unwrap().width, 2);
    assert_eq!(subtitle.style.shadow.as_ref().unwrap().color, "#222222");
    assert_eq!(subtitle.style.shadow.as_ref().unwrap().offset_x, 2);
    assert_eq!(subtitle.style.shadow.as_ref().unwrap().offset_y, 2);
    assert_eq!(subtitle.style.shadow.as_ref().unwrap().blur, 4);
    assert_eq!(subtitle.text_box.height_millis, 180);
    assert_eq!(subtitle.layout_region.y_millis, 720);
    assert!(
        subtitle.layout_region.y_millis > text.layout_region.y_millis,
        "project-session default subtitles must render below title text instead of overlapping it"
    );

    close_project_session(json!({ "sessionId": "test-session-add-media-timing" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_add_text_segment_accepts_atomic_drop_placement_handle() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-text-drop-placement.veproj");
    save_multimedia_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-text-drop-placement"
    }))
    .expect("openProjectSession should return an envelope");

    let text_added = execute_project_intent(json!({
        "sessionId": "test-session-text-drop-placement",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTextSegmentIntent",
            "content": "拖入时间线文字",
            "targetStart": 900_000,
            "targetTrackHandle": "timeline-track:text-track"
        }
    }))
    .expect("text drop placement intent should return an envelope");
    assert_eq!(text_added["ok"], true, "{text_added:#}");
    assert_no_renderer_project_state_payload(&text_added);
    assert_eq!(
        text_added["data"]["viewModel"]["selectedSegment"]["selectionHandle"],
        "timeline-segment:text-track:text-segment-1"
    );
    assert_eq!(
        text_added["data"]["viewModel"]["selectedSegment"]["targetTimerange"]["start"],
        900_000
    );

    let rejected = execute_project_intent(json!({
        "sessionId": "test-session-text-drop-placement",
        "expectedRevision": 1,
        "intent": {
            "kind": "addTextSegmentIntent",
            "content": "错误轨道文字",
            "targetStart": 1_100_000,
            "targetTrackHandle": "timeline-track:audio-track"
        }
    }))
    .expect("wrong track text drop placement intent should return an envelope");
    assert_eq!(rejected["ok"], false, "{rejected:#}");
    assert_eq!(rejected["error"]["kind"], "invalidTimelineEdit");

    close_project_session(json!({ "sessionId": "test-session-text-drop-placement" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_add_media_intents_reject_renderer_timing_fields() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir
        .path()
        .join("session-add-media-timing-reject.veproj");
    save_multimedia_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-add-media-timing-reject"
    }))
    .expect("openProjectSession should return an envelope");

    let subtitle_template = text_segment_json("字幕", "subtitle");
    let cases = [
        json!({
            "kind": "addTextSegmentIntent",
            "text": text_segment_json("旧完整文字模板", "text")
        }),
        json!({
            "kind": "addTextSegmentIntent",
            "content": "旧文字时长",
            "duration": 1_000_000
        }),
        json!({
            "kind": "addAudioSegmentIntent",
            "materialId": "audio-material",
            "duration": 1_000_000
        }),
        json!({
            "kind": "addAudioSegmentIntent",
            "materialId": "audio-material",
            "targetStart": 1_000_000
        }),
        json!({
            "kind": "importSubtitleSrtIntent",
            "srtContent": "1\n00:00:00,000 --> 00:00:01,000\n旧偏移\n",
            "timeOffset": 1_000_000
        }),
        json!({
            "kind": "importSubtitleSrtIntent",
            "srtContent": "1\n00:00:00,000 --> 00:00:01,000\n旧样式\n",
            "style": subtitle_template["style"].clone(),
            "textBox": subtitle_template["textBox"].clone(),
            "layoutRegion": subtitle_template["layoutRegion"].clone(),
            "wrapping": subtitle_template["wrapping"].clone()
        }),
    ];

    for intent in cases {
        let rejected = execute_project_intent(json!({
            "sessionId": "test-session-add-media-timing-reject",
            "expectedRevision": 0,
            "intent": intent
        }))
        .expect("legacy media timing payload should return an envelope");
        assert_eq!(rejected["ok"], false, "{rejected:#}");
        assert_eq!(rejected["data"], Value::Null);
        assert_eq!(rejected["error"]["kind"], "invalidPayload");
    }

    close_project_session(json!({ "sessionId": "test-session-add-media-timing-reject" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_move_selected_segment_uses_target_start() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-move.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-move"
    }))
    .expect("openProjectSession should return an envelope");

    let added = execute_project_intent(json!({
        "sessionId": "test-session-move",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("add intent should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");

    let moved = execute_project_intent(json!({
        "sessionId": "test-session-move",
        "expectedRevision": 1,
        "intent": {
            "kind": "moveSelectedSegmentIntent",
            "startAt": 200_000
        }
    }))
    .expect("move intent should return an envelope");
    assert_eq!(moved["ok"], true, "{moved:#}");
    assert_eq!(moved["data"]["revision"], 2);
    assert_eq!(moved["data"]["events"][0]["kind"], "segmentMoved");
    assert_no_renderer_project_state_payload(&moved);
    assert_eq!(
        moved["data"]["viewModel"]["selectedSegment"]["targetTimerange"]["start"],
        200_000
    );
    assert_eq!(
        moved["data"]["viewModel"]["selectedSegment"]["targetTimerange"]["duration"],
        1_000_000
    );

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("session move should save canonical project.json");
    assert_eq!(reopened.bundle.draft.tracks[0].segments.len(), 1);
    assert_eq!(
        reopened.bundle.draft.tracks[0].segments[0]
            .target_timerange
            .start
            .get(),
        200_000
    );

    close_project_session(json!({ "sessionId": "test-session-move" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_move_intent_rejects_renderer_built_delta() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-move-reject.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-move-reject"
    }))
    .expect("openProjectSession should return an envelope");

    let rejected = execute_project_intent(json!({
        "sessionId": "test-session-move-reject",
        "expectedRevision": 0,
        "intent": {
            "kind": "moveSelectedSegmentIntent",
            "delta": 200_000
        }
    }))
    .expect("legacy move delta payload should return an envelope");
    assert_eq!(rejected["ok"], false, "{rejected:#}");
    assert_eq!(rejected["data"], Value::Null);
    assert_eq!(rejected["error"]["kind"], "invalidPayload");

    close_project_session(json!({ "sessionId": "test-session-move-reject" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_move_selected_text_segment_accepts_only_track_selection_handle() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-text-cross-track-move.veproj");
    save_multimedia_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-text-cross-track-move"
    }))
    .expect("openProjectSession should return an envelope");

    let text_added = execute_project_intent(json!({
        "sessionId": "test-session-text-cross-track-move",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTextSegmentIntent",
            "content": "跨轨文字"
        }
    }))
    .expect("text add intent should return an envelope");
    assert_eq!(text_added["ok"], true, "{text_added:#}");
    let mut revision = text_added["data"]["revision"].as_u64().unwrap();

    let track_added = execute_project_intent(json!({
        "sessionId": "test-session-text-cross-track-move",
        "expectedRevision": revision,
        "intent": {
            "kind": "addTrackIntent",
            "trackKind": "text"
        }
    }))
    .expect("text track add intent should return an envelope");
    assert_eq!(track_added["ok"], true, "{track_added:#}");
    revision = track_added["data"]["revision"].as_u64().unwrap();
    assert_eq!(
        track_added["data"]["viewModel"]["timeline"]["rows"][3]["selectionHandle"],
        "timeline-track:track-text-4"
    );

    let selected = execute_project_intent(json!({
        "sessionId": "test-session-text-cross-track-move",
        "expectedRevision": revision,
        "intent": {
            "kind": "selectTimelineItemIntent",
            "itemHandle": "timeline-segment:text-track:text-segment-1"
        }
    }))
    .expect("select text segment intent should return an envelope");
    assert_eq!(selected["ok"], true, "{selected:#}");
    revision = selected["data"]["revision"].as_u64().unwrap();

    let moved = execute_project_intent(json!({
        "sessionId": "test-session-text-cross-track-move",
        "expectedRevision": revision,
        "intent": {
            "kind": "moveSelectedSegmentIntent",
            "startAt": 800_000,
            "targetTrackHandle": "timeline-track:track-text-4"
        }
    }))
    .expect("cross-track text move intent should return an envelope");
    assert_eq!(moved["ok"], true, "{moved:#}");
    assert_eq!(moved["data"]["events"][0]["kind"], "segmentMoved");
    assert_no_renderer_project_state_payload(&moved);
    assert_eq!(
        moved["data"]["viewModel"]["selectedSegment"]["selectionHandle"],
        "timeline-segment:track-text-4:text-segment-1"
    );
    assert_eq!(
        moved["data"]["viewModel"]["selectedSegment"]["targetTimerange"]["start"],
        800_000
    );
    revision = moved["data"]["revision"].as_u64().unwrap();

    let segment_target_rejected = execute_project_intent(json!({
        "sessionId": "test-session-text-cross-track-move",
        "expectedRevision": revision,
        "intent": {
            "kind": "moveSelectedSegmentIntent",
            "startAt": 900_000,
            "targetTrackHandle": "timeline-segment:track-text-4:text-segment-1"
        }
    }))
    .expect("segment target handle move intent should return an envelope");
    assert_eq!(
        segment_target_rejected["ok"], false,
        "{segment_target_rejected:#}"
    );
    assert_eq!(
        segment_target_rejected["error"]["kind"],
        "invalidTimelineEdit"
    );

    let raw_track_rejected = execute_project_intent(json!({
        "sessionId": "test-session-text-cross-track-move",
        "expectedRevision": revision,
        "intent": {
            "kind": "moveSelectedSegmentIntent",
            "startAt": 900_000,
            "trackId": "track-text-4"
        }
    }))
    .expect("renderer raw track id move intent should return an envelope");
    assert_eq!(raw_track_rejected["ok"], false, "{raw_track_rejected:#}");
    assert_eq!(raw_track_rejected["error"]["kind"], "invalidPayload");

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("cross-track text move should save canonical project.json");
    let source_text_track = reopened
        .bundle
        .draft
        .tracks
        .iter()
        .find(|track| track.track_id.as_str() == "text-track")
        .expect("source text track should exist");
    let target_text_track = reopened
        .bundle
        .draft
        .tracks
        .iter()
        .find(|track| track.track_id.as_str() == "track-text-4")
        .expect("target text track should exist");
    assert!(source_text_track.segments.is_empty());
    assert_eq!(target_text_track.segments.len(), 1);
    assert_eq!(
        target_text_track.segments[0].segment_id.as_str(),
        "text-segment-1"
    );
    assert_eq!(
        target_text_track.segments[0].target_timerange.start.get(),
        800_000
    );

    close_project_session(json!({ "sessionId": "test-session-text-cross-track-move" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_text_and_visual_edits_are_patch_owned() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-text-visual-patch.veproj");
    save_multimedia_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-text-visual-patch"
    }))
    .expect("openProjectSession should return an envelope");

    let subtitle_added = execute_project_intent(json!({
        "sessionId": "test-session-text-visual-patch",
        "expectedRevision": 0,
        "intent": {
            "kind": "importSubtitleSrtIntent",
            "srtContent": "1\n00:00:00,000 --> 00:00:01,000\n旧字幕\n"
        }
    }))
    .expect("subtitle import intent should return an envelope");
    assert_eq!(subtitle_added["ok"], true, "{subtitle_added:#}");
    let mut revision = subtitle_added["data"]["revision"].as_u64().unwrap();

    let full_text_rejected = execute_project_intent(json!({
        "sessionId": "test-session-text-visual-patch",
        "expectedRevision": revision,
        "intent": {
            "kind": "editSelectedText",
            "text": text_segment_json("renderer full replacement", "text")
        }
    }))
    .expect("full text replacement should return an envelope");
    assert_eq!(full_text_rejected["ok"], false, "{full_text_rejected:#}");
    assert_eq!(full_text_rejected["error"]["kind"], "invalidPayload");

    let text_patched = execute_project_intent(json!({
        "sessionId": "test-session-text-visual-patch",
        "expectedRevision": revision,
        "intent": {
            "kind": "editSelectedText",
            "patch": {
                "content": "Rust patch 字幕",
                "fontFamily": "Noto Serif CJK SC",
                "fontRef": "font://bundled/noto-serif-cjk-sc-regular",
                "fontSize": 42,
                "color": "#ffeeaa",
                "alignment": "center",
                "lineHeightMillis": 1300,
                "letterSpacingMillis": 40,
                "strokeEnabled": false,
                "backgroundEnabled": true,
                "backgroundColor": "#101010",
                "textBoxWidthMillis": 700,
                "textBoxHeightMillis": 180,
                "layoutXMillis": 120,
                "layoutYMillis": 700,
                "layoutWidthMillis": 760,
                "layoutHeightMillis": 200,
                "wrapping": "auto"
            }
        }
    }))
    .expect("text patch intent should return an envelope");
    assert_eq!(text_patched["ok"], true, "{text_patched:#}");
    assert_no_renderer_project_state_payload(&text_patched);
    revision = text_patched["data"]["revision"].as_u64().unwrap();

    let full_visual_rejected = execute_project_intent(json!({
        "sessionId": "test-session-text-visual-patch",
        "expectedRevision": revision,
        "intent": {
            "kind": "updateSelectedSegmentVisual",
            "visual": text_patched["data"]["viewModel"]["selectedSegment"]["visual"].clone()
        }
    }))
    .expect("full visual replacement should return an envelope");
    assert_eq!(
        full_visual_rejected["ok"], false,
        "{full_visual_rejected:#}"
    );
    assert_eq!(full_visual_rejected["error"]["kind"], "invalidPayload");

    let visual_patched = execute_project_intent(json!({
        "sessionId": "test-session-text-visual-patch",
        "expectedRevision": revision,
        "intent": {
            "kind": "updateSelectedSegmentVisual",
            "patch": {
                "positionDeltaX": 140,
                "positionDeltaY": -80,
                "rotationDeltaDegrees": 25,
                "opacityMillis": 830,
                "fitMode": "fill"
            }
        }
    }))
    .expect("visual patch intent should return an envelope");
    assert_eq!(visual_patched["ok"], true, "{visual_patched:#}");
    assert_no_renderer_project_state_payload(&visual_patched);
    revision = visual_patched["data"]["revision"].as_u64().unwrap();

    let invalid_visual_patch = execute_project_intent(json!({
        "sessionId": "test-session-text-visual-patch",
        "expectedRevision": revision,
        "intent": {
            "kind": "updateSelectedSegmentVisual",
            "patch": {
                "opacityMillis": 1_001
            }
        }
    }))
    .expect("invalid visual patch should return an envelope");
    assert_eq!(
        invalid_visual_patch["ok"], false,
        "{invalid_visual_patch:#}"
    );
    assert_eq!(invalid_visual_patch["error"]["kind"], "invalidTimelineEdit");

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("text and visual patch should save canonical project.json");
    let subtitle = reopened
        .bundle
        .draft
        .tracks
        .iter()
        .flat_map(|track| track.segments.iter())
        .find(|segment| segment.segment_id.as_str() == "subtitle-segment-1")
        .expect("subtitle segment should exist");
    let text = subtitle
        .text
        .as_ref()
        .expect("subtitle should keep text data");
    assert_eq!(text.content, "Rust patch 字幕");
    assert_eq!(text.source, TextSegmentSource::Subtitle);
    assert_eq!(text.style.font.family, "Noto Serif CJK SC");
    assert_eq!(
        text.style.font.font_ref.as_deref(),
        Some("font://bundled/noto-serif-cjk-sc-regular")
    );
    assert!(text.style.stroke.is_none());
    assert_eq!(text.style.background.as_ref().unwrap().color, "#101010");
    assert_eq!(subtitle.visual.transform.position.x, 140);
    assert_eq!(subtitle.visual.transform.position.y, -80);
    assert_eq!(subtitle.visual.transform.rotation.degrees, 25);
    assert_eq!(subtitle.visual.transform.opacity.value_millis, 830);

    close_project_session(json!({ "sessionId": "test-session-text-visual-patch" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_split_selected_segment_uses_session_playhead() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-split.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-split"
    }))
    .expect("openProjectSession should return an envelope");

    let added = execute_project_intent(json!({
        "sessionId": "test-session-split",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("add intent should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");

    let moved = execute_project_intent(json!({
        "sessionId": "test-session-split",
        "expectedRevision": 1,
        "intent": {
            "kind": "moveSelectedSegmentIntent",
            "startAt": 200_000
        }
    }))
    .expect("move intent should return an envelope");
    assert_eq!(moved["ok"], true, "{moved:#}");

    let positioned = execute_project_intent(json!({
        "sessionId": "test-session-split",
        "expectedRevision": 2,
        "intent": {
            "kind": "setSessionPlayhead",
            "playhead": 450_000
        }
    }))
    .expect("session playhead intent should return an envelope");
    assert_eq!(positioned["ok"], true, "{positioned:#}");
    assert_eq!(positioned["data"]["revision"], 2);
    assert_no_renderer_project_state_payload(&positioned);

    let split = execute_project_intent(json!({
        "sessionId": "test-session-split",
        "expectedRevision": 2,
        "intent": {
            "kind": "splitSelectedSegmentIntent"
        }
    }))
    .expect("split intent should return an envelope");
    assert_eq!(split["ok"], true, "{split:#}");
    assert_eq!(split["data"]["revision"], 3);
    assert_eq!(split["data"]["events"][0]["kind"], "segmentSplit");
    assert_no_renderer_project_state_payload(&split);

    let rows = split["data"]["viewModel"]["timeline"]["rows"]
        .as_array()
        .expect("timeline rows should be an array");
    let segments = rows[0]["segments"]
        .as_array()
        .expect("timeline row segments should be an array");
    assert_eq!(segments.len(), 2, "{split:#}");
    assert_eq!(segments[0]["start"], 200_000);
    assert_eq!(segments[0]["duration"], 250_000);
    assert_eq!(segments[1]["start"], 450_000);
    assert_eq!(segments[1]["duration"], 750_000);

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("session split should save canonical project.json");
    assert_eq!(reopened.bundle.draft.tracks[0].segments.len(), 2);
    assert_eq!(
        reopened.bundle.draft.tracks[0].segments[0]
            .target_timerange
            .start
            .get(),
        200_000
    );
    assert_eq!(
        reopened.bundle.draft.tracks[0].segments[0]
            .target_timerange
            .duration
            .get(),
        250_000
    );
    assert_eq!(
        reopened.bundle.draft.tracks[0].segments[1]
            .target_timerange
            .start
            .get(),
        450_000
    );
    assert_eq!(
        reopened.bundle.draft.tracks[0].segments[1]
            .target_timerange
            .duration
            .get(),
        750_000
    );

    close_project_session(json!({ "sessionId": "test-session-split" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_split_intent_rejects_renderer_built_split_at() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-split-reject.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-split-reject"
    }))
    .expect("openProjectSession should return an envelope");

    let rejected = execute_project_intent(json!({
        "sessionId": "test-session-split-reject",
        "expectedRevision": 0,
        "intent": {
            "kind": "splitSelectedSegmentIntent",
            "splitAt": 450_000
        }
    }))
    .expect("legacy splitAt payload should return an envelope");
    assert_eq!(rejected["ok"], false, "{rejected:#}");
    assert_eq!(rejected["data"], Value::Null);
    assert_eq!(rejected["error"]["kind"], "invalidPayload");

    close_project_session(json!({ "sessionId": "test-session-split-reject" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_trim_selected_segment_uses_trim_boundary() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-trim.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-trim"
    }))
    .expect("openProjectSession should return an envelope");

    let added = execute_project_intent(json!({
        "sessionId": "test-session-trim",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("add intent should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");

    let moved = execute_project_intent(json!({
        "sessionId": "test-session-trim",
        "expectedRevision": 1,
        "intent": {
            "kind": "moveSelectedSegmentIntent",
            "startAt": 200_000
        }
    }))
    .expect("move intent should return an envelope");
    assert_eq!(moved["ok"], true, "{moved:#}");

    let left_trimmed = execute_project_intent(json!({
        "sessionId": "test-session-trim",
        "expectedRevision": 2,
        "intent": {
            "kind": "trimSelectedSegmentIntent",
            "direction": "left",
            "trimAt": 450_000
        }
    }))
    .expect("left trim intent should return an envelope");
    assert_eq!(left_trimmed["ok"], true, "{left_trimmed:#}");
    assert_eq!(left_trimmed["data"]["revision"], 3);
    assert_eq!(left_trimmed["data"]["events"][0]["kind"], "segmentTrimmed");
    assert_no_renderer_project_state_payload(&left_trimmed);

    let right_trimmed = execute_project_intent(json!({
        "sessionId": "test-session-trim",
        "expectedRevision": 3,
        "intent": {
            "kind": "trimSelectedSegmentIntent",
            "direction": "right",
            "trimAt": 900_000
        }
    }))
    .expect("right trim intent should return an envelope");
    assert_eq!(right_trimmed["ok"], true, "{right_trimmed:#}");
    assert_eq!(right_trimmed["data"]["revision"], 4);
    assert_eq!(right_trimmed["data"]["events"][0]["kind"], "segmentTrimmed");
    assert_no_renderer_project_state_payload(&right_trimmed);

    let selected = &right_trimmed["data"]["viewModel"]["selectedSegment"];
    assert_eq!(selected["targetTimerange"]["start"], 450_000);
    assert_eq!(selected["targetTimerange"]["duration"], 450_000);
    assert_eq!(selected["sourceTimerange"]["start"], 250_000);
    assert_eq!(selected["sourceTimerange"]["duration"], 450_000);

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("session trim should save canonical project.json");
    let segment = &reopened.bundle.draft.tracks[0].segments[0];
    assert_eq!(segment.target_timerange.start.get(), 450_000);
    assert_eq!(segment.target_timerange.duration.get(), 450_000);
    assert_eq!(segment.source_timerange.start.get(), 250_000);
    assert_eq!(segment.source_timerange.duration.get(), 450_000);

    close_project_session(json!({ "sessionId": "test-session-trim" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_trim_intent_rejects_renderer_built_delta() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-trim-reject.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-trim-reject"
    }))
    .expect("openProjectSession should return an envelope");

    let rejected = execute_project_intent(json!({
        "sessionId": "test-session-trim-reject",
        "expectedRevision": 0,
        "intent": {
            "kind": "trimSelectedSegmentIntent",
            "direction": "left",
            "delta": 250_000
        }
    }))
    .expect("legacy trim delta payload should return an envelope");
    assert_eq!(rejected["ok"], false, "{rejected:#}");
    assert_eq!(rejected["data"], Value::Null);
    assert_eq!(rejected["error"]["kind"], "invalidPayload");

    close_project_session(json!({ "sessionId": "test-session-trim-reject" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_imports_material_then_adds_segment_without_renderer_draft() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let runtime = discover_runtime_config().expect("ffmpeg runtime should be discoverable");
    let executor = DesktopFfmpegExecutor::default();
    let video = generate_video_material_fixture(&executor, &runtime)
        .expect("video material fixture should be generated");
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-import-add.veproj");
    save_empty_timeline_draft(&bundle_path);

    let opened = open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-import-add"
    }))
    .expect("openProjectSession should return an envelope");
    assert_eq!(opened["ok"], true, "{opened:#}");

    let imported = execute_project_intent(json!({
        "sessionId": "test-session-import-add",
        "expectedRevision": 0,
        "intent": {
            "kind": "importMaterial",
            "materialPath": video.path().display().to_string(),
            "materialId": "session-video-material",
            "displayName": "session-video.mp4"
        }
    }))
    .expect("session importMaterial intent should return an envelope");
    assert_eq!(imported["ok"], true, "{imported:#}");
    assert_eq!(imported["data"]["revision"], 1);
    assert_eq!(
        imported["data"]["material"]["materialId"],
        "session-video-material"
    );
    assert_eq!(imported["data"]["material"]["status"], "available");
    assert_eq!(
        imported["data"]["materials"][0]["materialId"],
        "session-video-material"
    );
    assert_eq!(
        imported["data"]["materials"].as_array().unwrap().len(),
        1,
        "{imported:#}"
    );
    assert_no_renderer_project_state_payload(&imported);
    assert_eq!(
        imported["data"]["viewModel"]["project"]["materialCount"], 1,
        "{imported:#}"
    );
    assert_eq!(imported["data"]["delta"]["command"], "importMaterial");
    assert_eq!(
        imported["data"]["delta"]["changedEntities"][0],
        json!({ "kind": "material", "materialId": "session-video-material" })
    );
    assert_eq!(
        imported["data"]["delta"]["invalidation"]["materialIds"][0],
        "session-video-material"
    );

    let added = execute_project_intent(json!({
        "sessionId": "test-session-import-add",
        "expectedRevision": 1,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "session-video-material"
        }
    }))
    .expect("session addTimelineSegmentIntent should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");
    assert_eq!(added["data"]["revision"], 2);
    assert_eq!(added["data"]["events"][0]["kind"], "segmentAdded");
    assert_no_renderer_project_state_payload(&added);

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("session import and add should save canonical project.json");
    assert_eq!(reopened.bundle.draft.materials.len(), 1);
    assert_eq!(reopened.bundle.draft.tracks[0].segments.len(), 1);
    assert_eq!(
        reopened.bundle.draft.tracks[0].segments[0]
            .material_id
            .as_str(),
        "session-video-material"
    );

    close_project_session(json!({ "sessionId": "test-session-import-add" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_template_import_response_includes_events_and_delta() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-template-import.veproj");
    let source_root =
        seed_template_import_fixture_resources(temp_dir.path(), "positive/main-video.json");

    let created = create_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-template-import",
        "draftId": "template-import-before-draft",
        "draftName": "Before Template Import"
    }))
    .expect("createProjectSession should return an envelope");
    assert_eq!(created["ok"], true, "{created:#}");

    let imported = import_kaipai_formula_bundle(json!({
        "sessionId": "test-session-template-import",
        "expectedRevision": 0,
        "bundlePath": template_import_fixture_path("positive/main-video.json"),
        "resourceRoot": source_root,
        "importId": "binding-template-import",
        "generatedAt": "2026-06-24T00:00:00Z",
        "verifyResourceSha256": false
    }))
    .expect("importKaipaiFormulaBundle should return an envelope");
    assert_eq!(imported["ok"], true, "{imported:#}");
    assert_eq!(imported["data"]["revision"], 1, "{imported:#}");
    assert_eq!(
        imported["data"]["events"][0]["kind"], "templateImported",
        "template import must emit command events like other project-session mutations: {imported:#}"
    );
    assert_eq!(
        imported["data"]["delta"]["command"], "importTemplate",
        "template import must return a provider-neutral CommandDelta: {imported:#}"
    );
    assert_eq!(
        imported["data"]["delta"]["changedEntities"][0]["kind"], "draft",
        "template import delta must identify the imported draft: {imported:#}"
    );
    assert!(
        imported["data"]["delta"]["changedEntities"][0]["draftId"]
            .as_str()
            .is_some_and(|draft_id| draft_id.contains("binding-template-import")),
        "template import delta must identify the imported draft id: {imported:#}"
    );
    for domain in ["track", "timing", "visual", "material", "canvas"] {
        assert!(
            imported["data"]["delta"]["changedDomains"]
                .as_array()
                .expect("changedDomains should be an array")
                .contains(&json!(domain)),
            "template import delta should mark changed domain {domain}: {imported:#}"
        );
    }
    assert!(
        imported["data"]["delta"]["changedRanges"]
            .as_array()
            .expect("changedRanges should be an array")
            .iter()
            .any(|range| range["source"] == "fullDraft"),
        "template import should dirty the full imported draft range: {imported:#}"
    );
    assert_eq!(
        imported["data"]["delta"]["invalidation"]["fullDraft"], true,
        "template import must invalidate full draft consumers: {imported:#}"
    );
    for consumer_domain in [
        "preview",
        "exportPrep",
        "audio",
        "thumbnail",
        "waveform",
        "proxy",
        "graphSnapshot",
        "previewCache",
    ] {
        assert!(
            imported["data"]["delta"]["invalidation"]["consumerDomains"]
                .as_array()
                .expect("consumerDomains should be an array")
                .contains(&json!(consumer_domain)),
            "template import must invalidate {consumer_domain}: {imported:#}"
        );
    }
    assert_no_renderer_project_state_payload(&imported);

    close_project_session(json!({ "sessionId": "test-session-template-import" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_material_reads_use_canonical_session_draft() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let runtime = discover_runtime_config().expect("ffmpeg runtime should be discoverable");
    let executor = DesktopFfmpegExecutor::default();
    let video = generate_video_material_fixture(&executor, &runtime)
        .expect("video material fixture should be generated");
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-material-read.veproj");
    save_empty_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-material-read"
    }))
    .expect("openProjectSession should return an envelope");

    let imported = execute_project_intent(json!({
        "sessionId": "test-session-material-read",
        "expectedRevision": 0,
        "intent": {
            "kind": "importMaterial",
            "materialPath": video.path().display().to_string(),
            "materialId": "session-read-material",
            "displayName": "session-read.mp4"
        }
    }))
    .expect("session importMaterial intent should return an envelope");
    assert_eq!(imported["ok"], true, "{imported:#}");
    assert_eq!(imported["data"]["revision"], 1);

    let listed = list_project_session_materials(json!({
        "sessionId": "test-session-material-read",
        "expectedRevision": 1
    }))
    .expect("listProjectSessionMaterials should return an envelope");
    assert_eq!(listed["ok"], true, "{listed:#}");
    assert!(
        listed["data"]["revision"].as_u64().unwrap() >= 1,
        "material read should return the current session revision: {listed:#}"
    );
    assert_eq!(
        listed["data"]["materials"][0]["materialId"],
        "session-read-material"
    );
    assert_eq!(
        listed["data"]["bundlePath"],
        bundle_path.canonicalize().unwrap().display().to_string()
    );

    let stale = list_project_session_materials(json!({
        "sessionId": "test-session-material-read",
        "expectedRevision": 0
    }))
    .expect("outdated listProjectSessionMaterials should return current session state");
    assert_eq!(stale["ok"], true, "{stale:#}");
    assert!(
        stale["data"]["revision"].as_u64().unwrap() >= 1,
        "material read with an outdated expected revision should sync to current revision: {stale:#}"
    );
    assert_eq!(
        stale["data"]["materials"][0]["materialId"],
        "session-read-material"
    );

    let rejected = list_project_session_materials(json!({
        "sessionId": "test-session-material-read",
        "expectedRevision": 1,
        "draft": timeline_draft_json()
    }))
    .expect("draft-bearing material read should return an envelope");
    assert_eq!(rejected["ok"], false, "{rejected:#}");
    assert_eq!(rejected["error"]["kind"], "invalidPayload");

    close_project_session(json!({ "sessionId": "test-session-material-read" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_material_probe_queue_preserves_work_for_burst_imports() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = ExportSandbox::new("session-material-probe-queue");
    let _ffmpeg = sandbox.ffmpeg_complete();
    let _ffprobe = sandbox.ffprobe_success_slow(160, 90, false, 300);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-probe-queue.veproj");
    save_empty_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-probe-queue"
    }))
    .expect("openProjectSession should return an envelope");

    for (index, material_id) in [
        "session-queue-material-a",
        "session-queue-material-b",
        "session-queue-material-c",
    ]
    .iter()
    .enumerate()
    {
        let material_path = temp_dir.path().join(format!("queue-{index}.mp4"));
        fs::write(&material_path, format!("queued material {index}"))
            .expect("queued material fixture should be written");
        let imported = execute_project_intent(json!({
            "sessionId": "test-session-probe-queue",
            "expectedRevision": index,
            "intent": {
                "kind": "importMaterial",
                "materialPath": material_path.display().to_string(),
                "materialId": material_id,
                "displayName": format!("queue-{index}.mp4")
            }
        }))
        .expect("queued importMaterial intent should return an envelope");
        assert_eq!(imported["ok"], true, "{imported:#}");
        assert_eq!(imported["data"]["revision"], index as u64 + 1);
        assert_eq!(imported["data"]["probeStatus"], "queued");
    }

    let listed = wait_for_material_probe_metadata(
        "test-session-probe-queue",
        &[
            "session-queue-material-a",
            "session-queue-material-b",
            "session-queue-material-c",
        ],
    );
    assert!(
        listed["data"]["revision"].as_u64().unwrap() >= 6,
        "three queued probe completions should advance the canonical session revision: {listed:#}"
    );

    close_project_session(json!({ "sessionId": "test-session-probe-queue" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_import_material_reports_probe_schedule_failure_after_commit_as_success() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let _probe_failure = force_material_probe_enqueue_failure_for_tests(
        "material probe scheduler rejected: scheduler queue for MediaProbe is full at 8 jobs",
    );
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir
        .path()
        .join("session-probe-schedule-failure.veproj");
    save_empty_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-probe-schedule-failure"
    }))
    .expect("openProjectSession should return an envelope");

    let material_path = temp_dir.path().join("probe-schedule-failure.mp4");
    fs::write(&material_path, "probe schedule failure material")
        .expect("material fixture should be written");
    let imported = execute_project_intent(json!({
        "sessionId": "test-session-probe-schedule-failure",
        "expectedRevision": 0,
        "intent": {
            "kind": "importMaterial",
            "materialPath": material_path.display().to_string(),
            "materialId": "probe-schedule-failure-material",
            "displayName": "probe-schedule-failure.mp4"
        }
    }))
    .expect("importMaterial intent should return an envelope");

    assert_eq!(imported["ok"], true, "{imported:#}");
    assert_eq!(imported["data"]["revision"], 1);
    assert_eq!(imported["data"]["probeStatus"], "failed");
    assert!(
        imported["data"].get("probeJobId").is_none(),
        "failed probe scheduling must not fabricate a job id: {imported:#}"
    );
    assert_eq!(
        imported["data"]["diagnostic"]["kind"], "probeFailed",
        "{imported:#}"
    );
    assert_eq!(
        imported["data"]["diagnostic"]["materialId"], "probe-schedule-failure-material",
        "{imported:#}"
    );
    assert!(
        imported["data"]["diagnostic"]["message"]
            .as_str()
            .unwrap()
            .contains("could not be scheduled"),
        "{imported:#}"
    );
    assert_eq!(
        imported["data"]["materials"][0]["materialId"], "probe-schedule-failure-material",
        "{imported:#}"
    );

    let listed = list_project_session_materials(json!({
        "sessionId": "test-session-probe-schedule-failure",
        "expectedRevision": 1
    }))
    .expect("listProjectSessionMaterials should return an envelope");
    assert_eq!(listed["ok"], true, "{listed:#}");
    assert_eq!(listed["data"]["revision"], 1);
    assert!(
        listed["data"]["materials"]
            .as_array()
            .unwrap()
            .iter()
            .any(|material| material["materialId"] == "probe-schedule-failure-material"),
        "committed import must remain visible after probe scheduling failure: {listed:#}"
    );

    let project_json = fs::read_to_string(imported["data"]["projectJsonPath"].as_str().unwrap())
        .expect("project.json should be readable after committed import");
    assert!(
        project_json.contains("probe-schedule-failure-material"),
        "committed import must be persisted in project.json"
    );

    close_project_session(json!({ "sessionId": "test-session-probe-schedule-failure" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_material_probe_worker_spawn_failure_releases_scheduler_capacity() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = ExportSandbox::new("session-material-probe-spawn-failure");
    let _ffmpeg = sandbox.ffmpeg_complete();
    let _ffprobe = sandbox.ffprobe_success_slow(160, 90, false, 50);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-probe-spawn-failure.veproj");
    save_empty_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-probe-spawn-failure"
    }))
    .expect("openProjectSession should return an envelope");

    {
        let _spawn_failure =
            force_material_probe_worker_spawn_failure_for_tests("test worker spawn failure");
        let failed_material_path = temp_dir.path().join("spawn-failure-a.mp4");
        fs::write(&failed_material_path, "spawn failure material")
            .expect("spawn failure material fixture should be written");
        let imported = execute_project_intent(json!({
            "sessionId": "test-session-probe-spawn-failure",
            "expectedRevision": 0,
            "intent": {
                "kind": "importMaterial",
                "materialPath": failed_material_path.display().to_string(),
                "materialId": "spawn-failure-material-a",
                "displayName": "spawn-failure-a.mp4"
            }
        }))
        .expect("spawn failure importMaterial intent should return an envelope");
        assert_eq!(imported["ok"], true, "{imported:#}");
        assert_eq!(imported["data"]["revision"], 1);
        assert_eq!(imported["data"]["probeStatus"], "failed");
        assert!(
            imported["data"].get("probeJobId").is_none(),
            "worker spawn failure must not expose a running job id: {imported:#}"
        );
    }

    let recover_material_path = temp_dir.path().join("spawn-failure-b.mp4");
    fs::write(&recover_material_path, "recover material")
        .expect("recover material fixture should be written");
    let recovered = execute_project_intent(json!({
        "sessionId": "test-session-probe-spawn-failure",
        "expectedRevision": 1,
        "intent": {
            "kind": "importMaterial",
            "materialPath": recover_material_path.display().to_string(),
            "materialId": "spawn-failure-material-b",
            "displayName": "spawn-failure-b.mp4"
        }
    }))
    .expect("recover importMaterial intent should return an envelope");
    assert_eq!(recovered["ok"], true, "{recovered:#}");
    assert_eq!(recovered["data"]["revision"], 2);
    assert_eq!(recovered["data"]["probeStatus"], "queued");

    let listed = wait_for_material_probe_metadata(
        "test-session-probe-spawn-failure",
        &["spawn-failure-material-b"],
    );
    assert!(
        listed["data"]["revision"].as_u64().unwrap() >= 3,
        "successful follow-up probe proves the failed spawn released scheduler capacity: {listed:#}"
    );

    close_project_session(json!({ "sessionId": "test-session-probe-spawn-failure" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_material_probe_jobs_are_unique_across_sessions() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = ExportSandbox::new("session-material-probe-cross-session");
    let _ffmpeg = sandbox.ffmpeg_complete();
    let _ffprobe = sandbox.ffprobe_success_slow(160, 90, false, 300);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_a = temp_dir.path().join("session-probe-a.veproj");
    let bundle_b = temp_dir.path().join("session-probe-b.veproj");
    save_empty_timeline_draft(&bundle_a);
    save_empty_timeline_draft(&bundle_b);
    open_project_session(json!({
        "bundlePath": bundle_a.display().to_string(),
        "sessionId": "test-session-probe-a"
    }))
    .expect("openProjectSession A should return an envelope");
    open_project_session(json!({
        "bundlePath": bundle_b.display().to_string(),
        "sessionId": "test-session-probe-b"
    }))
    .expect("openProjectSession B should return an envelope");

    let material_path_a = temp_dir.path().join("shared-a.mp4");
    let material_path_b = temp_dir.path().join("shared-b.mp4");
    fs::write(&material_path_a, "shared material a").expect("material A should be written");
    fs::write(&material_path_b, "shared material b").expect("material B should be written");
    for (session_id, material_path) in [
        ("test-session-probe-a", &material_path_a),
        ("test-session-probe-b", &material_path_b),
    ] {
        let imported = execute_project_intent(json!({
            "sessionId": session_id,
            "expectedRevision": 0,
            "intent": {
                "kind": "importMaterial",
                "materialPath": material_path.display().to_string(),
                "materialId": "shared-session-material",
                "displayName": "shared-session-material.mp4"
            }
        }))
        .expect("cross-session importMaterial intent should return an envelope");
        assert_eq!(imported["ok"], true, "{imported:#}");
        assert_eq!(imported["data"]["revision"], 1);
        assert_eq!(imported["data"]["probeStatus"], "queued");
    }

    let listed_a =
        wait_for_material_probe_metadata("test-session-probe-a", &["shared-session-material"]);
    let listed_b =
        wait_for_material_probe_metadata("test-session-probe-b", &["shared-session-material"]);
    assert!(
        listed_a["data"]["revision"].as_u64().unwrap() >= 2,
        "session A probe should advance independently: {listed_a:#}"
    );
    assert!(
        listed_b["data"]["revision"].as_u64().unwrap() >= 2,
        "session B probe should advance independently: {listed_b:#}"
    );

    close_project_session(json!({ "sessionId": "test-session-probe-a" }))
        .expect("closeProjectSession A should return an envelope");
    close_project_session(json!({ "sessionId": "test-session-probe-b" }))
        .expect("closeProjectSession B should return an envelope");
}

#[test]
fn project_session_missing_material_reads_use_session_bundle_path() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-missing-read.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-missing-read"
    }))
    .expect("openProjectSession should return an envelope");

    let listed = list_project_session_missing_materials(json!({
        "sessionId": "test-session-missing-read",
        "expectedRevision": 0
    }))
    .expect("listProjectSessionMissingMaterials should return an envelope");
    assert_eq!(listed["ok"], true, "{listed:#}");
    assert_eq!(listed["data"]["revision"], 0);
    assert_eq!(
        listed["data"]["diagnostics"][0]["materialId"],
        "video-material"
    );
    assert_eq!(listed["data"]["diagnostics"][0]["kind"], "missingFile");
    let resolved_path = listed["data"]["diagnostics"][0]["lastKnownResolvedPath"]
        .as_str()
        .expect("missing material should include resolved path");
    assert!(
        resolved_path.contains("session-missing-read.veproj"),
        "missing diagnostics should resolve against the session bundle path: {resolved_path}"
    );

    let unknown = list_project_session_missing_materials(json!({
        "sessionId": "missing-session",
        "expectedRevision": 0
    }))
    .expect("unknown session missing material read should return an envelope");
    assert_eq!(unknown["ok"], false, "{unknown:#}");
    assert_eq!(unknown["error"]["kind"], "invalidProject");

    let rejected = list_project_session_missing_materials(json!({
        "sessionId": "test-session-missing-read",
        "expectedRevision": 0,
        "bundlePath": "/tmp/renderer-owned-path"
    }))
    .expect("bundlePath-bearing missing material read should return an envelope");
    assert_eq!(rejected["ok"], false, "{rejected:#}");
    assert_eq!(rejected["error"]["kind"], "invalidPayload");

    close_project_session(json!({ "sessionId": "test-session-missing-read" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_rejects_renderer_owned_state_fields_before_execution() {
    for (label, extra_field) in [
        ("draft", json!({ "draft": timeline_draft_json() })),
        (
            "commandState",
            json!({ "commandState": { "undoStack": [], "redoStack": [] } }),
        ),
        (
            "selection",
            json!({ "selection": { "segmentIds": [], "trackIds": [] } }),
        ),
    ] {
        let mut request = json!({
            "sessionId": "test-session-reject-draft",
            "expectedRevision": 0,
            "intent": {
                "kind": "addTimelineSegmentIntent",
                "materialId": "video-material"
            }
        });
        request
            .as_object_mut()
            .expect("request should be an object")
            .extend(
                extra_field
                    .as_object()
                    .expect("extra field should be an object")
                    .clone(),
            );

        let envelope = execute_project_intent(request)
            .unwrap_or_else(|_| panic!("{label} should return a structured envelope"));

        assert_eq!(envelope["ok"], false, "{label}: {envelope:#}");
        assert_eq!(envelope["data"], Value::Null, "{label}: {envelope:#}");
        assert_eq!(
            envelope["error"]["kind"], "invalidPayload",
            "{label}: {envelope:#}"
        );
        assert_eq!(
            envelope["error"]["command"], "executeProjectIntent",
            "{label}: {envelope:#}"
        );
    }
}

#[test]
fn project_session_stale_revision_is_rejected_without_persisting() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-stale.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-stale"
    }))
    .expect("openProjectSession should return an envelope");

    let first = execute_project_intent(json!({
        "sessionId": "test-session-stale",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("first executeProjectIntent should return an envelope");
    assert_eq!(first["ok"], true, "{first:#}");
    assert_eq!(first["data"]["revision"], 1);

    let stale = execute_project_intent(json!({
        "sessionId": "test-session-stale",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("stale executeProjectIntent should return an envelope");
    assert_eq!(stale["ok"], false, "{stale:#}");
    assert_eq!(stale["error"]["kind"], "invalidPayload");
    assert!(
        stale["error"]["message"]
            .as_str()
            .expect("stale error should have a message")
            .contains("Stale project session revision")
    );

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("stale command must not mutate project.json");
    assert_eq!(reopened.bundle.draft.tracks[0].segments.len(), 1);

    close_project_session(json!({ "sessionId": "test-session-stale" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_undo_and_redo_use_rust_owned_command_state() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-undo-redo.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-undo-redo"
    }))
    .expect("openProjectSession should return an envelope");

    let added = execute_project_intent(json!({
        "sessionId": "test-session-undo-redo",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("add intent should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");

    let undone = execute_project_intent(json!({
        "sessionId": "test-session-undo-redo",
        "expectedRevision": 1,
        "intent": { "kind": "undoTimelineEdit" }
    }))
    .expect("undo intent should return an envelope");
    assert_eq!(undone["ok"], true, "{undone:#}");
    assert_eq!(undone["data"]["revision"], 2);
    assert_eq!(undone["data"]["events"][0]["kind"], "undoCommitted");
    assert_edit_controls(
        &undone["data"]["viewModel"],
        false,
        true,
        true,
        false,
        false,
    );
    let reopened_after_undo = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("undo should save canonical project.json");
    assert_eq!(reopened_after_undo.bundle.draft.tracks[0].segments.len(), 0);

    let redone = execute_project_intent(json!({
        "sessionId": "test-session-undo-redo",
        "expectedRevision": 2,
        "intent": { "kind": "redoTimelineEdit" }
    }))
    .expect("redo intent should return an envelope");
    assert_eq!(redone["ok"], true, "{redone:#}");
    assert_eq!(redone["data"]["revision"], 3);
    assert_eq!(redone["data"]["events"][0]["kind"], "redoCommitted");
    assert_edit_controls(&redone["data"]["viewModel"], true, false, true, true, true);
    let reopened_after_redo = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("redo should save canonical project.json");
    assert_eq!(reopened_after_redo.bundle.draft.tracks[0].segments.len(), 1);

    close_project_session(json!({ "sessionId": "test-session-undo-redo" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_opening_same_bundle_invalidates_previous_session() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-single-owner.veproj");
    save_timeline_draft(&bundle_path);

    let first = open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-single-owner-a"
    }))
    .expect("first openProjectSession should return an envelope");
    assert_eq!(first["ok"], true, "{first:#}");

    let second = open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-single-owner-b"
    }))
    .expect("second openProjectSession should return an envelope");
    assert_eq!(second["ok"], true, "{second:#}");

    let stale_owner = execute_project_intent(json!({
        "sessionId": "test-session-single-owner-a",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("old owner executeProjectIntent should return an envelope");
    assert_eq!(stale_owner["ok"], false, "{stale_owner:#}");
    assert_eq!(stale_owner["error"]["kind"], "invalidProject");

    let current_owner = execute_project_intent(json!({
        "sessionId": "test-session-single-owner-b",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("current owner executeProjectIntent should return an envelope");
    assert_eq!(current_owner["ok"], true, "{current_owner:#}");
    assert_eq!(current_owner["data"]["revision"], 1);

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("current owner should save canonical project.json");
    assert_eq!(reopened.bundle.draft.tracks[0].segments.len(), 1);

    close_project_session(json!({ "sessionId": "test-session-single-owner-b" }))
        .expect("closeProjectSession should return an envelope");
}

#[cfg(unix)]
#[test]
fn project_session_opening_same_bundle_through_symlink_invalidates_previous_session() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-single-owner-real.veproj");
    let symlink_path = temp_dir.path().join("session-single-owner-link.veproj");
    save_timeline_draft(&bundle_path);
    symlink(&bundle_path, &symlink_path).expect("bundle symlink should be created");

    let first = open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-single-owner-real"
    }))
    .expect("first openProjectSession should return an envelope");
    assert_eq!(first["ok"], true, "{first:#}");

    let second = open_project_session(json!({
        "bundlePath": symlink_path.display().to_string(),
        "sessionId": "test-session-single-owner-link"
    }))
    .expect("second openProjectSession should return an envelope");
    assert_eq!(second["ok"], true, "{second:#}");
    assert_eq!(
        second["data"]["bundlePath"],
        std::fs::canonicalize(&bundle_path)
            .expect("bundle should canonicalize")
            .display()
            .to_string()
    );

    let stale_owner = execute_project_intent(json!({
        "sessionId": "test-session-single-owner-real",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("old owner executeProjectIntent should return an envelope");
    assert_eq!(stale_owner["ok"], false, "{stale_owner:#}");
    assert_eq!(stale_owner["error"]["kind"], "invalidProject");

    let current_owner = execute_project_intent(json!({
        "sessionId": "test-session-single-owner-link",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("current owner executeProjectIntent should return an envelope");
    assert_eq!(current_owner["ok"], true, "{current_owner:#}");
    assert_eq!(current_owner["data"]["revision"], 1);

    close_project_session(json!({ "sessionId": "test-session-single-owner-link" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_selection_intent_does_not_persist_or_advance_revision() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-selection.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-selection"
    }))
    .expect("openProjectSession should return an envelope");

    let added = execute_project_intent(json!({
        "sessionId": "test-session-selection",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("add intent should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");
    assert_eq!(added["data"]["revision"], 1);

    let selected = execute_project_intent(json!({
        "sessionId": "test-session-selection",
        "expectedRevision": 1,
        "intent": {
            "kind": "selectTimelineItemIntent",
            "itemHandle": "timeline-segment:video-track:segment-1"
        }
    }))
    .expect("selection intent should return an envelope");
    assert_eq!(selected["ok"], true, "{selected:#}");
    assert_eq!(selected["data"]["revision"], 1);
    assert_no_renderer_project_state_payload(&selected);
    assert_eq!(
        selected["data"]["viewModel"]["selectedSegment"]["segmentKey"],
        "segment-1"
    );
    assert_eq!(
        selected["data"]["viewModel"]["selectedSegment"]["track"]["selectionHandle"],
        "timeline-track:video-track"
    );
    assert_edit_controls(
        &selected["data"]["viewModel"],
        true,
        false,
        true,
        true,
        true,
    );

    let follow_up = execute_project_intent(json!({
        "sessionId": "test-session-selection",
        "expectedRevision": 1,
        "intent": {
            "kind": "setSelectedSegmentVolume",
            "volume": { "levelMillis": 750 }
        }
    }))
    .expect("follow-up edit should use unchanged revision after selection");
    assert_eq!(follow_up["ok"], true, "{follow_up:#}");
    assert_eq!(follow_up["data"]["revision"], 2);

    close_project_session(json!({ "sessionId": "test-session-selection" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_rejects_legacy_and_invalid_selection_intents() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-selection-rejections.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-selection-rejections"
    }))
    .expect("openProjectSession should return an envelope");

    let added = execute_project_intent(json!({
        "sessionId": "test-session-selection-rejections",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("add intent should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");
    assert_eq!(added["data"]["revision"], 1);

    for (label, intent) in [
        (
            "legacy selectTimelineSegments payload",
            json!({
                "kind": "selectTimelineSegments",
                "segmentIds": ["segment-1"],
                "trackIds": ["video-track"]
            }),
        ),
        (
            "extra legacy fields on item handle selection",
            json!({
                "kind": "selectTimelineItemIntent",
                "itemHandle": "timeline-track:video-track",
                "trackIds": ["video-track"]
            }),
        ),
        (
            "unknown handle prefix",
            json!({
                "kind": "selectTimelineItemIntent",
                "itemHandle": "timeline-clip:video-track:segment-1"
            }),
        ),
        (
            "bad percent encoding",
            json!({
                "kind": "selectTimelineItemIntent",
                "itemHandle": "timeline-track:%ZZ"
            }),
        ),
        (
            "unknown track handle",
            json!({
                "kind": "selectTimelineItemIntent",
                "itemHandle": "timeline-track:missing-track"
            }),
        ),
        (
            "unknown segment handle",
            json!({
                "kind": "selectTimelineItemIntent",
                "itemHandle": "timeline-segment:video-track:missing-segment"
            }),
        ),
        (
            "malformed segment handle",
            json!({
                "kind": "selectTimelineItemIntent",
                "itemHandle": "timeline-segment:video-track:segment-1:extra"
            }),
        ),
    ] {
        let rejected = execute_project_intent(json!({
            "sessionId": "test-session-selection-rejections",
            "expectedRevision": 1,
            "intent": intent
        }))
        .unwrap_or_else(|_| panic!("{label} should return an error envelope"));
        assert_eq!(rejected["ok"], false, "{label}: {rejected:#}");
    }

    let valid = execute_project_intent(json!({
        "sessionId": "test-session-selection-rejections",
        "expectedRevision": 1,
        "intent": {
            "kind": "selectTimelineItemIntent",
            "itemHandle": "timeline-segment:video-track:segment-1"
        }
    }))
    .expect("valid selection should still use unchanged revision");
    assert_eq!(valid["ok"], true, "{valid:#}");
    assert_eq!(valid["data"]["revision"], 1);
    assert_no_renderer_project_state_payload(&valid);
    assert_eq!(
        valid["data"]["viewModel"]["selectedSegment"]["segmentKey"],
        "segment-1"
    );

    close_project_session(json!({ "sessionId": "test-session-selection-rejections" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_view_model_encodes_timeline_item_handles() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-encoded-handles.veproj");
    let mut draft_json = timeline_draft_json();
    draft_json["tracks"][0]["trackId"] = json!("video:track");
    draft_json["tracks"][0]["segments"] = json!([{
        "segmentId": "segment:1",
        "materialId": "video-material",
        "sourceTimerange": { "start": 0, "duration": 1_000_000 },
        "targetTimerange": { "start": 0, "duration": 1_000_000 },
        "mainTrackMagnet": { "enabled": false },
        "keyframes": [],
        "filters": [],
        "transition": null
    }]);
    let draft: Draft =
        serde_json::from_value(draft_json).expect("encoded handle draft should parse");
    save_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
        .expect("encoded handle draft should be saved");

    let opened = open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-encoded-handles"
    }))
    .expect("openProjectSession should return an envelope");
    assert_eq!(opened["ok"], true, "{opened:#}");
    assert_eq!(
        opened["data"]["viewModel"]["timeline"]["rows"][0]["selectionHandle"],
        "timeline-track:video%3Atrack"
    );
    assert_eq!(
        opened["data"]["viewModel"]["timeline"]["rows"][0]["segments"][0]["selectionHandle"],
        "timeline-segment:video%3Atrack:segment%3A1"
    );

    let selected = execute_project_intent(json!({
        "sessionId": "test-session-encoded-handles",
        "expectedRevision": 0,
        "intent": {
            "kind": "selectTimelineItemIntent",
            "itemHandle": opened["data"]["viewModel"]["timeline"]["rows"][0]["segments"][0]["selectionHandle"]
        }
    }))
    .expect("encoded handle selection should return an envelope");
    assert_eq!(selected["ok"], true, "{selected:#}");
    assert_eq!(
        selected["data"]["viewModel"]["selectedSegment"]["segmentKey"],
        "segment:1"
    );
    assert_eq!(selected["data"]["revision"], 0);

    close_project_session(json!({ "sessionId": "test-session-encoded-handles" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_phase19_intents_delegate_to_rust_commands() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-phase19-intents.veproj");
    save_phase19_project_intent_draft(&bundle_path);

    let opened = open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-phase19-intents"
    }))
    .expect("openProjectSession should return an envelope");
    assert_eq!(opened["ok"], true, "{opened:#}");

    let selected = execute_project_intent(json!({
        "sessionId": "test-session-phase19-intents",
        "expectedRevision": 0,
        "intent": {
            "kind": "selectTimelineItemIntent",
            "itemHandle": "timeline-segment:video-track:left-segment"
        }
    }))
    .expect("selectTimelineItemIntent should return an envelope");
    assert_eq!(selected["ok"], true, "{selected:#}");
    let mut revision = selected["data"]["revision"]
        .as_u64()
        .expect("selection should return revision");

    let stale = execute_project_intent(json!({
        "sessionId": "test-session-phase19-intents",
        "expectedRevision": revision + 99,
        "intent": {
            "kind": "setSelectedSegmentRetime",
            "retiming": {
                "mode": {
                    "kind": "constant",
                    "speed": { "numerator": 1, "denominator": 2 }
                },
                "audioPolicy": "followVideoSpeed"
            }
        }
    }))
    .expect("stale phase19 retime intent should return an envelope");
    assert_eq!(stale["ok"], false, "{stale:#}");
    assert!(
        stale["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Stale project session revision")
    );

    let retimed = execute_project_intent(json!({
        "sessionId": "test-session-phase19-intents",
        "expectedRevision": revision,
        "intent": {
            "kind": "setSelectedSegmentRetime",
            "retiming": {
                "mode": {
                    "kind": "constant",
                    "speed": { "numerator": 1, "denominator": 2 }
                },
                "audioPolicy": "followVideoSpeed"
            }
        }
    }))
    .expect("setSelectedSegmentRetime should return an envelope");
    assert_eq!(retimed["ok"], true, "{retimed:#}");
    revision += 1;
    assert_eq!(retimed["data"]["revision"], revision);
    assert_eq!(retimed["data"]["delta"]["command"], "setSegmentRetime");
    assert_no_renderer_project_state_payload(&retimed);

    let applied = execute_project_intent(json!({
        "sessionId": "test-session-phase19-intents",
        "expectedRevision": revision,
        "intent": {
            "kind": "applySelectedSegmentEffect",
            "effect": {
                "kind": { "kind": "opacityAdjustment", "opacityMillis": 900 },
                "enabled": true
            }
        }
    }))
    .expect("applySelectedSegmentEffect should return an envelope");
    assert_eq!(applied["ok"], true, "{applied:#}");
    revision += 1;
    assert_eq!(applied["data"]["revision"], revision);
    assert_eq!(applied["data"]["delta"]["command"], "applySegmentEffect");

    let effected = execute_project_intent(json!({
        "sessionId": "test-session-phase19-intents",
        "expectedRevision": revision,
        "intent": {
            "kind": "updateSelectedSegmentEffectParameter",
            "effectIndex": 0,
            "parameter": {
                "parameter": "gaussianBlurRadiusMillis",
                "radiusMillis": 750
            }
        }
    }))
    .expect("updateSelectedSegmentEffectParameter should return an envelope");
    assert_eq!(effected["ok"], true, "{effected:#}");
    revision += 1;
    assert_eq!(effected["data"]["revision"], revision);
    assert_eq!(
        effected["data"]["delta"]["command"],
        "updateSegmentEffectParameter"
    );

    let removed_effect = execute_project_intent(json!({
        "sessionId": "test-session-phase19-intents",
        "expectedRevision": revision,
        "intent": {
            "kind": "removeSelectedSegmentEffect",
            "effectIndex": 1
        }
    }))
    .expect("removeSelectedSegmentEffect should return an envelope");
    assert_eq!(removed_effect["ok"], true, "{removed_effect:#}");
    revision += 1;
    assert_eq!(removed_effect["data"]["revision"], revision);
    assert_eq!(
        removed_effect["data"]["delta"]["command"],
        "removeSegmentEffect"
    );

    let external_mask = execute_project_intent(json!({
        "sessionId": "test-session-phase19-intents",
        "expectedRevision": revision,
        "intent": {
            "kind": "setSelectedSegmentMask",
            "mask": {
                "kind": "externalReference",
                "reference": {
                    "provider": "jianying",
                    "effectId": "private-mask"
                }
            }
        }
    }))
    .expect("external mask intent should return an envelope");
    assert_eq!(external_mask["ok"], false, "{external_mask:#}");
    assert!(
        external_mask["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("external")
    );

    let masked = execute_project_intent(json!({
        "sessionId": "test-session-phase19-intents",
        "expectedRevision": revision,
        "intent": {
            "kind": "setSelectedSegmentMask",
            "mask": {
                "kind": "rectangle",
                "xMillis": 100,
                "yMillis": 120,
                "widthMillis": 500,
                "heightMillis": 400,
                "featherMillis": 40,
                "opacityMillis": 900,
                "inverted": false
            }
        }
    }))
    .expect("setSelectedSegmentMask should return an envelope");
    assert_eq!(masked["ok"], true, "{masked:#}");
    revision += 1;
    assert_eq!(masked["data"]["revision"], revision);
    assert_eq!(masked["data"]["events"][0]["kind"], "segmentMaskSet");
    assert_eq!(masked["data"]["delta"]["command"], "setSegmentMask");

    let blended = execute_project_intent(json!({
        "sessionId": "test-session-phase19-intents",
        "expectedRevision": revision,
        "intent": {
            "kind": "setSelectedSegmentBlendMode",
            "blendMode": { "kind": "multiply" }
        }
    }))
    .expect("setSelectedSegmentBlendMode should return an envelope");
    assert_eq!(blended["ok"], true, "{blended:#}");
    revision += 1;
    assert_eq!(blended["data"]["revision"], revision);
    assert_eq!(blended["data"]["events"][0]["kind"], "segmentBlendModeSet");
    assert_eq!(blended["data"]["delta"]["command"], "setSegmentBlendMode");

    let transition_updated = execute_project_intent(json!({
        "sessionId": "test-session-phase19-intents",
        "expectedRevision": revision,
        "intent": {
            "kind": "updateSelectedTransitionDuration",
            "fromSegmentId": "left-segment",
            "toSegmentId": "right-segment",
            "duration": 250_000
        }
    }))
    .expect("updateSelectedTransitionDuration should return an envelope");
    assert_eq!(transition_updated["ok"], true, "{transition_updated:#}");
    revision += 1;
    assert_eq!(transition_updated["data"]["revision"], revision);
    assert_eq!(
        transition_updated["data"]["delta"]["command"],
        "updateTransitionDuration"
    );

    let transition_removed = execute_project_intent(json!({
        "sessionId": "test-session-phase19-intents",
        "expectedRevision": revision,
        "intent": {
            "kind": "removeSelectedTransition",
            "fromSegmentId": "left-segment",
            "toSegmentId": "right-segment"
        }
    }))
    .expect("removeSelectedTransition should return an envelope");
    assert_eq!(transition_removed["ok"], true, "{transition_removed:#}");
    revision += 1;
    assert_eq!(transition_removed["data"]["revision"], revision);
    assert_eq!(
        transition_removed["data"]["delta"]["command"],
        "removeTransition"
    );

    let transition_added = execute_project_intent(json!({
        "sessionId": "test-session-phase19-intents",
        "expectedRevision": revision,
        "intent": {
            "kind": "addTransitionAtBoundary",
            "fromSegmentId": "left-segment",
            "toSegmentId": "right-segment",
            "reference": {
                "kind": "firstParty",
                "transition": "dissolve"
            },
            "duration": 200_000
        }
    }))
    .expect("addTransitionAtBoundary should return an envelope");
    assert_eq!(transition_added["ok"], true, "{transition_added:#}");
    revision += 1;
    assert_eq!(transition_added["data"]["revision"], revision);
    assert_eq!(
        transition_added["data"]["delta"]["command"],
        "addTransition"
    );

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("phase19 project intent commits should save canonical project.json");
    let segment = &reopened.bundle.draft.tracks[0].segments[0];
    assert_eq!(
        serde_json::to_value(&segment.retiming.mode).expect("retime mode should serialize"),
        json!({
            "kind": "constant",
            "speed": { "numerator": 1, "denominator": 2 }
        })
    );
    assert_eq!(
        serde_json::to_value(&segment.filters[0]).expect("filter should serialize")["kind"]["radiusMillis"],
        750
    );
    assert_eq!(
        serde_json::to_value(&segment.visual.mask).expect("mask should serialize")["kind"],
        "rectangle"
    );
    assert_eq!(
        serde_json::to_value(&segment.visual.blend_mode).expect("blend should serialize")["kind"],
        "multiply"
    );
    assert_eq!(reopened.bundle.draft.tracks[0].transitions.len(), 1);
    assert_eq!(
        reopened.bundle.draft.tracks[0].transitions[0].duration,
        Microseconds::new(200_000)
    );

    close_project_session(json!({ "sessionId": "test-session-phase19-intents" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_track_mutation_intents_use_selected_track() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-selected-track.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-selected-track"
    }))
    .expect("openProjectSession should return an envelope");

    let selected = execute_project_intent(json!({
        "sessionId": "test-session-selected-track",
        "expectedRevision": 0,
        "intent": {
            "kind": "selectTimelineItemIntent",
            "itemHandle": "timeline-track:video-track"
        }
    }))
    .expect("track selection intent should return an envelope");
    assert_eq!(selected["ok"], true, "{selected:#}");
    assert_eq!(selected["data"]["revision"], 0);

    let renamed = execute_project_intent(json!({
        "sessionId": "test-session-selected-track",
        "expectedRevision": 0,
        "intent": {
            "kind": "renameSelectedTrack",
            "name": "Primary Video"
        }
    }))
    .expect("rename selected track should return an envelope");
    assert_eq!(renamed["ok"], true, "{renamed:#}");
    assert_eq!(renamed["data"]["revision"], 1);
    assert_no_renderer_project_state_payload(&renamed);
    assert_eq!(
        renamed["data"]["viewModel"]["selectedTrack"]["name"],
        "Primary Video"
    );

    let locked = execute_project_intent(json!({
        "sessionId": "test-session-selected-track",
        "expectedRevision": 1,
        "intent": {
            "kind": "setSelectedTrackLock",
            "locked": true
        }
    }))
    .expect("lock selected track should return an envelope");
    assert_eq!(locked["ok"], true, "{locked:#}");
    assert_eq!(locked["data"]["revision"], 2);
    assert_no_renderer_project_state_payload(&locked);
    assert_eq!(locked["data"]["viewModel"]["selectedTrack"]["locked"], true);

    let unlocked = execute_project_intent(json!({
        "sessionId": "test-session-selected-track",
        "expectedRevision": 2,
        "intent": {
            "kind": "setSelectedTrackLock",
            "locked": false
        }
    }))
    .expect("unlock selected track should return an envelope");
    assert_eq!(unlocked["ok"], true, "{unlocked:#}");
    assert_eq!(unlocked["data"]["revision"], 3);

    let hidden = execute_project_intent(json!({
        "sessionId": "test-session-selected-track",
        "expectedRevision": 3,
        "intent": {
            "kind": "setSelectedTrackVisibility",
            "visible": false
        }
    }))
    .expect("hide selected track should return an envelope");
    assert_eq!(hidden["ok"], true, "{hidden:#}");
    assert_eq!(hidden["data"]["revision"], 4);
    assert_no_renderer_project_state_payload(&hidden);
    assert_eq!(
        hidden["data"]["viewModel"]["selectedTrack"]["visible"],
        false
    );

    let muted = execute_project_intent(json!({
        "sessionId": "test-session-selected-track",
        "expectedRevision": 4,
        "intent": {
            "kind": "setSelectedTrackMute",
            "muted": true
        }
    }))
    .expect("mute selected track should return an envelope");
    assert_eq!(muted["ok"], true, "{muted:#}");
    assert_eq!(muted["data"]["revision"], 5);
    assert_no_renderer_project_state_payload(&muted);
    assert_eq!(muted["data"]["viewModel"]["selectedTrack"]["muted"], true);

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("selected-track intents should persist canonical project.json");
    assert_eq!(reopened.bundle.draft.tracks[0].name, "Primary Video");
    assert!(!reopened.bundle.draft.tracks[0].visible);
    assert!(reopened.bundle.draft.tracks[0].muted);

    close_project_session(json!({ "sessionId": "test-session-selected-track" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_keyframe_intent_derives_keyframe_from_selected_segment() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-keyframe.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-keyframe"
    }))
    .expect("openProjectSession should return an envelope");

    let added = execute_project_intent(json!({
        "sessionId": "test-session-keyframe",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("add intent should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");

    let moved = execute_project_intent(json!({
        "sessionId": "test-session-keyframe",
        "expectedRevision": 1,
        "intent": {
            "kind": "moveSelectedSegmentIntent",
            "startAt": 200_000
        }
    }))
    .expect("move intent should return an envelope");
    assert_eq!(moved["ok"], true, "{moved:#}");

    let volume = execute_project_intent(json!({
        "sessionId": "test-session-keyframe",
        "expectedRevision": 2,
        "intent": {
            "kind": "setSelectedSegmentVolume",
            "volume": { "levelMillis": 750 }
        }
    }))
    .expect("volume intent should return an envelope");
    assert_eq!(volume["ok"], true, "{volume:#}");

    let positioned = execute_project_intent(json!({
        "sessionId": "test-session-keyframe",
        "expectedRevision": 3,
        "intent": {
            "kind": "setSessionPlayhead",
            "playhead": 450_000
        }
    }))
    .expect("session playhead intent should return an envelope");
    assert_eq!(positioned["ok"], true, "{positioned:#}");
    assert_eq!(positioned["data"]["revision"], 3);
    assert_no_renderer_project_state_payload(&positioned);

    let keyed = execute_project_intent(json!({
        "sessionId": "test-session-keyframe",
        "expectedRevision": 3,
        "intent": {
            "kind": "setSelectedSegmentKeyframe",
            "property": "volume",
            "interpolation": "hold",
            "easing": "easeIn"
        }
    }))
    .expect("keyframe intent should return an envelope");
    assert_eq!(keyed["ok"], true, "{keyed:#}");
    assert_eq!(keyed["data"]["revision"], 4);
    assert_no_renderer_project_state_payload(&keyed);

    let keyframe = &keyed["data"]["viewModel"]["selectedSegment"]["keyframes"][0];
    assert_eq!(keyframe["property"], "volume");
    assert_eq!(keyframe["at"], 250_000);
    assert_eq!(keyframe["value"], json!({ "kind": "uint", "value": 750 }));
    assert_eq!(keyframe["interpolation"], "hold");
    assert_eq!(keyframe["easing"], "easeIn");

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("session keyframe should save canonical project.json");
    let saved_segment = &reopened.bundle.draft.tracks[0].segments[0];
    assert_eq!(saved_segment.target_timerange.start.get(), 200_000);
    assert_eq!(saved_segment.keyframes[0].at.get(), 250_000);

    let remove_positioned = execute_project_intent(json!({
        "sessionId": "test-session-keyframe",
        "expectedRevision": 4,
        "intent": {
            "kind": "setSessionPlayhead",
            "playhead": 450_000
        }
    }))
    .expect("remove session playhead intent should return an envelope");
    assert_eq!(remove_positioned["ok"], true, "{remove_positioned:#}");
    assert_eq!(remove_positioned["data"]["revision"], 4);
    assert_no_renderer_project_state_payload(&remove_positioned);

    let removed = execute_project_intent(json!({
        "sessionId": "test-session-keyframe",
        "expectedRevision": 4,
        "intent": {
            "kind": "removeSelectedSegmentKeyframe",
            "property": "volume"
        }
    }))
    .expect("remove keyframe intent should return an envelope");
    assert_eq!(removed["ok"], true, "{removed:#}");
    assert_eq!(removed["data"]["revision"], 5);
    assert_no_renderer_project_state_payload(&removed);
    assert_eq!(
        removed["data"]["viewModel"]["selectedSegment"]["keyframes"]
            .as_array()
            .expect("keyframes should be an array")
            .len(),
        0
    );

    close_project_session(json!({ "sessionId": "test-session-keyframe" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_keyframe_intent_rejects_renderer_built_keyframe_payload() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-keyframe-reject.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-keyframe-reject"
    }))
    .expect("openProjectSession should return an envelope");

    let rejected = execute_project_intent(json!({
        "sessionId": "test-session-keyframe-reject",
        "expectedRevision": 0,
        "intent": {
            "kind": "setSelectedSegmentKeyframe",
            "keyframe": {
                "at": 0,
                "property": "visualPositionX",
                "value": { "kind": "int", "value": 0 },
                "interpolation": "linear",
                "easing": "none"
            }
        }
    }))
    .expect("old keyframe payload should return an envelope");
    assert_eq!(rejected["ok"], false, "{rejected:#}");
    assert_eq!(rejected["data"], Value::Null);
    assert_eq!(rejected["error"]["kind"], "invalidPayload");

    let rejected_set_at = execute_project_intent(json!({
        "sessionId": "test-session-keyframe-reject",
        "expectedRevision": 0,
        "intent": {
            "kind": "setSelectedSegmentKeyframe",
            "property": "visualPositionX",
            "at": 0,
            "interpolation": "linear",
            "easing": "none"
        }
    }))
    .expect("legacy keyframe at payload should return an envelope");
    assert_eq!(rejected_set_at["ok"], false, "{rejected_set_at:#}");
    assert_eq!(rejected_set_at["data"], Value::Null);
    assert_eq!(rejected_set_at["error"]["kind"], "invalidPayload");

    let rejected_remove_at = execute_project_intent(json!({
        "sessionId": "test-session-keyframe-reject",
        "expectedRevision": 0,
        "intent": {
            "kind": "removeSelectedSegmentKeyframe",
            "property": "visualPositionX",
            "at": 0
        }
    }))
    .expect("legacy remove keyframe at payload should return an envelope");
    assert_eq!(rejected_remove_at["ok"], false, "{rejected_remove_at:#}");
    assert_eq!(rejected_remove_at["data"], Value::Null);
    assert_eq!(rejected_remove_at["error"]["kind"], "invalidPayload");

    close_project_session(json!({ "sessionId": "test-session-keyframe-reject" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_export_starts_from_session_snapshot_without_renderer_draft() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = ExportSandbox::new("session-export-start");
    let _ffmpeg = sandbox.ffmpeg_complete();
    let _ffprobe = sandbox.ffprobe_success(160, 90, false);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-export.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-export"
    }))
    .expect("openProjectSession should return an envelope");
    let added = execute_project_intent(json!({
        "sessionId": "test-session-export",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("add segment before export should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");

    let output_path = sandbox.root.join("session-export.mp4");
    let started = start_project_session_export(json!({
        "sessionId": "test-session-export",
        "expectedRevision": 1,
        "outputPath": output_path.display().to_string(),
        "preset": "h264AacBalanced"
    }))
    .expect("startProjectSessionExport should return an envelope");

    assert_eq!(started["ok"], true, "{started:#}");
    assert_eq!(started["data"]["phase"], "running");
    assert_eq!(
        started["data"]["outputPath"],
        output_path.display().to_string()
    );
    let job_id = started["data"]["jobId"]
        .as_str()
        .expect("export job id should be present")
        .to_owned();
    let completed = wait_for_export_phase(&job_id);
    assert_eq!(completed["ok"], true, "{completed:#}");
    assert_eq!(completed["data"]["phase"], "completed");

    close_project_session(json!({ "sessionId": "test-session-export" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_export_rejects_stale_and_unknown_sessions() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-export-stale.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-export-stale"
    }))
    .expect("openProjectSession should return an envelope");

    let stale = start_project_session_export(json!({
        "sessionId": "test-session-export-stale",
        "expectedRevision": 1,
        "outputPath": temp_dir.path().join("stale.mp4").display().to_string(),
        "preset": "h264AacBalanced"
    }))
    .expect("stale export should return an envelope");
    assert_eq!(stale["ok"], false, "{stale:#}");
    assert_eq!(stale["error"]["kind"], "invalidPayload");
    assert_eq!(stale["error"]["command"], "startProjectSessionExport");

    let unknown = start_project_session_export(json!({
        "sessionId": "missing-session-export",
        "expectedRevision": 0,
        "outputPath": temp_dir.path().join("unknown.mp4").display().to_string(),
        "preset": "h264AacBalanced"
    }))
    .expect("unknown export should return an envelope");
    assert_eq!(unknown["ok"], false, "{unknown:#}");
    assert_eq!(unknown["error"]["kind"], "invalidProject");
    assert_eq!(unknown["error"]["command"], "startProjectSessionExport");

    close_project_session(json!({ "sessionId": "test-session-export-stale" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_session_export_rejects_renderer_draft_payload() {
    let rejected = start_project_session_export(json!({
        "sessionId": "test-session-export",
        "expectedRevision": 0,
        "outputPath": "/tmp/renderer-draft-export.mp4",
        "preset": "h264AacBalanced",
        "draft": timeline_draft_json()
    }))
    .expect("draft-bearing export should return an envelope");

    assert_eq!(rejected["ok"], false, "{rejected:#}");
    assert_eq!(rejected["data"], Value::Null);
    assert_eq!(rejected["error"]["kind"], "invalidPayload");
}

#[test]
fn realtime_preview_updates_from_project_session_snapshot() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-preview.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-preview"
    }))
    .expect("openProjectSession should return an envelope");
    let preview = create_preview_session("project-session-preview-success");
    let preview_session_id = preview["sessionId"]
        .as_str()
        .expect("preview session id should be returned");

    let snapshot = update_realtime_preview_project_session_snapshot(json!({
        "sessionId": preview_session_id,
        "projectSessionId": "test-session-preview",
        "expectedRevision": 0
    }))
    .expect("project-session snapshot should update realtime preview");

    assert!(
        snapshot["playbackGeneration"].as_u64().unwrap_or_default() > 0,
        "{snapshot:#}"
    );

    close_realtime_preview_session(json!({ "sessionId": preview_session_id }))
        .expect("closeRealtimePreviewSession should return a response");
    close_project_session(json!({ "sessionId": "test-session-preview" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn realtime_preview_project_session_snapshot_rejects_stale_revision() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-preview-stale.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-preview-stale"
    }))
    .expect("openProjectSession should return an envelope");
    let preview = create_preview_session("project-session-preview-stale");
    let preview_session_id = preview["sessionId"]
        .as_str()
        .expect("preview session id should be returned");

    let stale = update_realtime_preview_project_session_snapshot(json!({
        "sessionId": preview_session_id,
        "projectSessionId": "test-session-preview-stale",
        "expectedRevision": 1
    }));

    assert!(stale.is_err(), "stale project session revision should fail");

    close_realtime_preview_session(json!({ "sessionId": preview_session_id }))
        .expect("closeRealtimePreviewSession should return a response");
    close_project_session(json!({ "sessionId": "test-session-preview-stale" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn realtime_preview_project_session_snapshot_rejects_unknown_project_session() {
    let preview = create_preview_session("project-session-preview-unknown");
    let preview_session_id = preview["sessionId"]
        .as_str()
        .expect("preview session id should be returned");

    let unknown = update_realtime_preview_project_session_snapshot(json!({
        "sessionId": preview_session_id,
        "projectSessionId": "missing-project-session",
        "expectedRevision": 0
    }));

    assert!(unknown.is_err(), "unknown project session should fail");

    close_realtime_preview_session(json!({ "sessionId": preview_session_id }))
        .expect("closeRealtimePreviewSession should return a response");
}

#[test]
fn realtime_preview_project_session_snapshot_rejects_renderer_draft_payload() {
    let rejected = update_realtime_preview_project_session_snapshot(json!({
        "sessionId": "rtprev-session-0000000000000001",
        "projectSessionId": "test-session-preview",
        "expectedRevision": 0,
        "draft": timeline_draft_json()
    }));

    assert!(
        rejected.is_err(),
        "draft field must not be accepted on preview snapshot sync"
    );
}

#[test]
fn audio_preview_commands_use_project_session_snapshot_without_renderer_draft() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-audio-preview.veproj");
    save_timeline_draft(&bundle_path);

    let opened = open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-audio-preview"
    }))
    .expect("openProjectSession should return an envelope");
    assert_eq!(opened["ok"], true, "{opened:#}");

    let create = create_audio_preview_session(json!({
        "projectSessionId": "test-session-audio-preview",
        "expectedRevision": 0,
        "targetTime": 0
    }))
    .expect("audio preview create command should return an envelope");
    assert_eq!(create["ok"], true, "{create:#}");
    let audio_session_id = create["data"]["sessionId"]
        .as_str()
        .expect("audio session id should be returned");
    assert!(audio_session_id.starts_with("audio-session-"));

    let status = get_audio_preview_status(json!({
        "projectSessionId": "test-session-audio-preview",
        "expectedRevision": 0,
        "sessionId": audio_session_id,
        "targetTime": 0
    }))
    .expect("audio preview status command should return an envelope");
    assert_eq!(status["ok"], true, "{status:#}");
    assert_eq!(status["data"]["sessionId"], audio_session_id);

    let seek = seek_audio_preview(json!({
        "projectSessionId": "test-session-audio-preview",
        "expectedRevision": 0,
        "sessionId": audio_session_id,
        "targetTime": 500000,
        "playbackGeneration": create["data"]["generation"]
    }))
    .expect("audio preview seek command should return an envelope");
    assert_eq!(seek["ok"], true, "{seek:#}");
    assert_eq!(seek["data"]["targetTime"], 500000);

    let renderer_draft_payload = get_audio_preview_status(json!({
        "draft": timeline_draft_json(),
        "sessionId": audio_session_id
    }))
    .expect("audio preview renderer draft payload should return an error envelope");
    assert_eq!(
        renderer_draft_payload["ok"], false,
        "{renderer_draft_payload:#}"
    );

    let missing_identity = stop_audio_preview(json!({
        "sessionId": audio_session_id
    }))
    .expect("audio preview missing project identity should return an error envelope");
    assert_eq!(missing_identity["ok"], false, "{missing_identity:#}");
    assert!(
        missing_identity["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("projectSessionId")
    );

    let legacy_envelope = execute_command(json!({
        "command": "getAudioPreviewStatus",
        "payload": {
            "kind": "getAudioPreviewStatus",
            "projectSessionId": "test-session-audio-preview",
            "expectedRevision": 0,
            "sessionId": audio_session_id
        },
        "requestId": "req-audio-legacy"
    }))
    .expect("legacy audio preview envelope should return an error envelope");
    assert_eq!(legacy_envelope["ok"], false, "{legacy_envelope:#}");
    assert_eq!(legacy_envelope["error"]["kind"], "unsupportedCommand");

    close_project_session(json!({ "sessionId": "test-session-audio-preview" }))
        .expect("closeProjectSession should return an envelope");
}

fn create_preview_session(label: &str) -> Value {
    create_realtime_preview_session(json!({
        "sessionLabel": label,
        "frameRateNumerator": 30,
        "frameRateDenominator": 1,
        "playbackRateNumerator": 1,
        "playbackRateDenominator": 1
    }))
    .expect("createRealtimePreviewSession should return a response")
}

fn assert_edit_controls(
    view_model: &Value,
    can_undo: bool,
    can_redo: bool,
    snapping_enabled: bool,
    has_selected_segment: bool,
    has_selected_track: bool,
) {
    let edit_controls = &view_model["editControls"];
    assert_eq!(edit_controls["canUndo"], can_undo, "{view_model:#}");
    assert_eq!(edit_controls["canRedo"], can_redo, "{view_model:#}");
    assert_eq!(
        edit_controls["snappingEnabled"], snapping_enabled,
        "{view_model:#}"
    );
    assert_eq!(
        edit_controls["snappingLabel"],
        if snapping_enabled {
            "吸附 开"
        } else {
            "吸附 关"
        },
        "{view_model:#}"
    );
    assert_eq!(
        edit_controls["hasSelectedSegment"], has_selected_segment,
        "{view_model:#}"
    );
    assert_eq!(
        edit_controls["hasSelectedTrack"], has_selected_track,
        "{view_model:#}"
    );
}

fn assert_no_renderer_project_state_payload(envelope: &Value) {
    assert!(
        envelope["data"].get("draft").is_none(),
        "session response must not expose renderer-owned draft payloads: {envelope:#}"
    );
    assert!(
        envelope["data"].get("commandState").is_none(),
        "session response must not expose renderer-owned command state: {envelope:#}"
    );
    assert!(
        envelope["data"].get("selection").is_none(),
        "session response must not expose renderer-owned selection: {envelope:#}"
    );
}

fn save_timeline_draft(bundle_path: &std::path::Path) {
    let draft: Draft =
        serde_json::from_value(timeline_draft_json()).expect("timeline draft fixture should parse");
    save_project_bundle(&StdPlatformFileSystem, bundle_path, &draft)
        .expect("timeline draft fixture should be saved");
}

fn save_empty_timeline_draft(bundle_path: &std::path::Path) {
    let mut draft: Draft =
        serde_json::from_value(timeline_draft_json()).expect("timeline draft fixture should parse");
    draft.materials.clear();
    save_project_bundle(&StdPlatformFileSystem, bundle_path, &draft)
        .expect("empty timeline draft fixture should be saved");
}

fn save_multimedia_timeline_draft(bundle_path: &std::path::Path) {
    let draft: Draft = serde_json::from_value(multimedia_timeline_draft_json())
        .expect("multimedia timeline draft fixture should parse");
    save_project_bundle(&StdPlatformFileSystem, bundle_path, &draft)
        .expect("multimedia timeline draft fixture should be saved");
}

fn save_phase19_project_intent_draft(bundle_path: &std::path::Path) {
    let mut draft = Draft::new(
        "phase19-project-intent-draft",
        "Phase 19 Project Intent Draft",
    );
    draft.materials.push(Material::new(
        "video-material",
        MaterialKind::Video,
        "file://video.mp4",
        "video.mp4",
    ));

    let mut left_segment = Segment::new(
        "left-segment",
        "video-material",
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    );
    left_segment.filters.push(Filter::gaussian_blur(500));
    let right_segment = Segment::new(
        "right-segment",
        "video-material",
        SourceTimerange::new(1_000_000, 1_000_000),
        TargetTimerange::new(1_000_000, 1_000_000),
    );
    let mut track = Track::new("video-track", TrackKind::Video, "Video");
    track.segments.push(left_segment);
    track.segments.push(right_segment);
    track.transitions.push(TrackTransition::dissolve(
        SegmentId::from("left-segment"),
        SegmentId::from("right-segment"),
        Microseconds::new(300_000),
    ));
    draft.tracks.push(track);

    save_project_bundle(&StdPlatformFileSystem, bundle_path, &draft)
        .expect("phase19 project intent draft fixture should be saved");
}

fn multimedia_timeline_draft_json() -> Value {
    let mut draft = timeline_draft_json();
    draft["materials"]
        .as_array_mut()
        .expect("materials should be an array")
        .push(json!({
            "materialId": "audio-material",
            "kind": "audio",
            "uri": "media/bgm.wav",
            "displayName": "bgm.wav",
            "metadata": {
                "duration": 2_000_000,
                "hasVideo": false,
                "hasAudio": true,
                "audioSampleRate": 48_000,
                "audioChannels": 2
            },
            "status": "available"
        }));
    let tracks = draft["tracks"]
        .as_array_mut()
        .expect("tracks should be an array");
    tracks.push(json!({
        "trackId": "audio-track",
        "kind": "audio",
        "name": "Audio",
        "muted": false,
        "locked": false,
        "segments": []
    }));
    tracks.push(json!({
        "trackId": "text-track",
        "kind": "text",
        "name": "Title",
        "muted": false,
        "locked": false,
        "segments": []
    }));
    draft
}

fn timeline_draft_json() -> Value {
    json!({
        "schemaVersion": 1,
        "draftId": "session-timeline-draft",
        "metadata": { "name": "Session Timeline Draft" },
        "canvasConfig": {
            "width": 1920,
            "height": 1080,
            "frameRate": { "numerator": 30, "denominator": 1 },
            "aspectRatio": { "kind": "preset", "preset": "ratio16x9" },
            "background": { "kind": "black" }
        },
        "materials": [{
            "materialId": "video-material",
            "kind": "video",
            "uri": "media/video.mp4",
            "displayName": "video.mp4",
            "metadata": {
                "duration": 1_000_000,
                "width": 160,
                "height": 90,
                "frameRate": { "numerator": 30, "denominator": 1 },
                "hasVideo": true,
                "hasAudio": false
            },
            "status": "available"
        }],
        "tracks": [{
            "trackId": "video-track",
            "kind": "video",
            "name": "Video",
            "muted": false,
            "locked": false,
            "segments": []
        }]
    })
}

fn text_segment_json(content: &str, source: &str) -> Value {
    json!({
        "content": content,
        "source": source,
        "style": {
            "font": {
                "family": "Noto Sans CJK SC",
                "fontRef": "font://bundled/noto-sans-cjk-sc-regular"
            },
            "fontSize": 36,
            "color": "#ffffff",
            "alignment": "center",
            "lineHeightMillis": 1200,
            "letterSpacingMillis": 0,
            "stroke": { "color": "#000000", "width": 2 },
            "shadow": { "color": "#222222", "offsetX": 2, "offsetY": 2, "blur": 4 },
            "background": null
        },
        "textBox": {
            "widthMillis": 800,
            "heightMillis": 200
        },
        "layoutRegion": {
            "xMillis": 100,
            "yMillis": 100,
            "widthMillis": 800,
            "heightMillis": 800
        },
        "wrapping": "auto",
        "bubble": null,
        "effect": null
    })
}

fn template_import_fixture_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/kaipai")
        .join(path)
}

fn seed_template_import_fixture_resources(root: &Path, fixture: &str) -> PathBuf {
    let source_root = root.join("template-source");
    fs::create_dir_all(&source_root).expect("template import source root should create");
    let value: Value = serde_json::from_str(
        &fs::read_to_string(template_import_fixture_path(fixture))
            .expect("template import fixture should be readable"),
    )
    .expect("template import fixture should parse");
    for resource in value["resources"].as_array().into_iter().flatten() {
        let uri = resource["uri"]
            .as_str()
            .expect("template import fixture resource should have uri");
        let resource_id = resource["resourceId"]
            .as_str()
            .expect("template import fixture resource should have resourceId");
        let path = source_root.join(uri);
        fs::create_dir_all(
            path.parent()
                .expect("template resource path should have parent"),
        )
        .expect("template resource directory should create");
        fs::write(
            &path,
            format!("project session template import fixture {resource_id}"),
        )
        .expect("template resource fixture should write");
    }
    source_root
}

fn wait_for_export_phase(job_id: &str) -> Value {
    let mut last = Value::Null;
    for _ in 0..20 {
        let status = execute_command(json!({
            "command": "getExportJobStatus",
            "payload": {
                "kind": "getExportJobStatus",
                "jobId": job_id
            },
            "requestId": "req-session-export-status"
        }))
        .expect("status command should return envelope");
        last = status.clone();
        if status["data"]["phase"] == "completed" {
            return status;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("export job did not complete; last={last:#}");
}

fn wait_for_material_probe_metadata(session_id: &str, material_ids: &[&str]) -> Value {
    let mut last = Value::Null;
    for _ in 0..40 {
        let listed = list_project_session_materials(json!({
            "sessionId": session_id,
            "expectedRevision": 0
        }))
        .expect("listProjectSessionMaterials should return an envelope");
        last = listed.clone();
        let materials = listed["data"]["materials"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let all_probed = material_ids.iter().all(|material_id| {
            materials.iter().any(|material| {
                material["materialId"] == *material_id
                    && material["metadata"]["width"] == 160
                    && material["metadata"]["height"] == 90
                    && material["metadata"]["duration"] == 1_000_000
            })
        });
        if all_probed {
            return listed;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("material probes did not complete; last={last:#}");
}

struct ExportSandbox {
    root: PathBuf,
}

impl ExportSandbox {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "video-editor-binding-session-export-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn ffmpeg_complete(&self) -> PathBuf {
        self.script(
            "ffmpeg",
            r#"#!/bin/sh
if [ "$1" = "-version" ]; then
  printf 'ffmpeg version session-export-test\n'
  exit 0
fi
for arg in "$@"; do
  if [ "$arg" = "-encoders" ]; then
    printf ' V..... libx264 H.264 encoder\n'
    printf ' A..... aac AAC encoder\n'
    exit 0
  fi
  if [ "$arg" = "-filters" ]; then
    printf ' ... ass Render ASS subtitles\n'
    printf ' ... subtitles Render text subtitles\n'
    exit 0
  fi
done
last=""
for arg in "$@"; do
  last="$arg"
done
mkdir -p "$(dirname "$last")"
printf 'out_time_us=500000\n'
printf 'progress=continue\n'
printf 'fake mp4\n' > "$last"
printf 'out_time_us=1000000\n'
printf 'progress=end\n'
"#,
        )
    }

    fn ffprobe_success(&self, width: u32, height: u32, has_audio: bool) -> PathBuf {
        self.ffprobe_success_with_delay(width, height, has_audio, 0)
    }

    fn ffprobe_success_slow(
        &self,
        width: u32,
        height: u32,
        has_audio: bool,
        delay_ms: u64,
    ) -> PathBuf {
        self.ffprobe_success_with_delay(width, height, has_audio, delay_ms)
    }

    fn ffprobe_success_with_delay(
        &self,
        width: u32,
        height: u32,
        has_audio: bool,
        delay_ms: u64,
    ) -> PathBuf {
        let audio_stream = if has_audio {
            r#",{"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2,"duration":"1.000000"}"#
        } else {
            ""
        };
        let delay = if delay_ms > 0 {
            format!("sleep {}\n", delay_ms as f64 / 1000.0)
        } else {
            String::new()
        };
        self.script(
            "ffprobe",
            &format!(
                r#"#!/bin/sh
if [ "$1" = "-version" ]; then
  printf 'ffprobe version session-export-test\n'
  exit 0
fi
{delay}
cat <<'JSON'
{{"streams":[{{"codec_type":"video","codec_name":"h264","width":{width},"height":{height},"r_frame_rate":"30/1","duration":"1.000000"}}{audio_stream}],"format":{{"duration":"1.000000"}}}}
JSON
"#
            ),
        )
    }

    fn script(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, contents).unwrap();
        make_executable(&path);
        path
    }
}

impl Drop for ExportSandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

struct RuntimeDirectoryGuard {
    previous: Option<PathBuf>,
}

impl RuntimeDirectoryGuard {
    fn set(directory: impl AsRef<Path>) -> Self {
        Self {
            previous: replace_configured_bundled_runtime_directory_for_tests(Some(
                directory.as_ref().to_path_buf(),
            )),
        }
    }
}

impl Drop for RuntimeDirectoryGuard {
    fn drop(&mut self) {
        replace_configured_bundled_runtime_directory_for_tests(self.previous.take());
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

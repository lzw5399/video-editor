use bindings_node::{
    close_project_session, close_realtime_preview_session, create_audio_preview_session,
    create_project_session, create_realtime_preview_session, execute_command,
    execute_project_intent, get_audio_preview_status, list_project_session_materials,
    list_project_session_missing_materials, open_project_session, seek_audio_preview,
    start_project_session_export, stop_audio_preview,
    update_realtime_preview_project_session_snapshot,
};
use draft_model::{Draft, TextSegmentSource};
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
fn project_session_add_intent_rejects_renderer_placement_fields() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-add-reject-placement.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-add-reject-placement"
    }))
    .expect("openProjectSession should return an envelope");

    let rejected = execute_project_intent(json!({
        "sessionId": "test-session-add-reject-placement",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material",
            "targetStart": 450_000
        }
    }))
    .expect("legacy add placement payload should return an envelope");
    assert_eq!(rejected["ok"], false, "{rejected:#}");
    assert_eq!(rejected["data"], Value::Null);
    assert_eq!(rejected["error"]["kind"], "invalidPayload");

    close_project_session(json!({ "sessionId": "test-session-add-reject-placement" }))
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
            "kind": "addTextSegmentIntent",
            "content": "旧文字位置",
            "targetStart": 1_000_000
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
    assert_eq!(listed["data"]["revision"], 1);
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
    .expect("stale listProjectSessionMaterials should return an envelope");
    assert_eq!(stale["ok"], false, "{stale:#}");
    assert_eq!(stale["error"]["kind"], "invalidPayload");

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
        let audio_stream = if has_audio {
            r#",{"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2,"duration":"1.000000"}"#
        } else {
            ""
        };
        self.script(
            "ffprobe",
            &format!(
                r#"#!/bin/sh
if [ "$1" = "-version" ]; then
  printf 'ffprobe version session-export-test\n'
  exit 0
fi
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

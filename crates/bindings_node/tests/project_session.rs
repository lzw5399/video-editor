use bindings_node::{close_project_session, execute_project_intent, open_project_session};
use draft_model::Draft;
use project_store::{StdPlatformFileSystem, open_project_bundle, save_project_bundle};
use serde_json::{Value, json};

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
    assert_eq!(
        added["data"]["draft"]["tracks"][0]["segments"][0]["segmentId"],
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
fn project_session_rejects_renderer_draft_field_before_execution() {
    let envelope = execute_project_intent(json!({
        "sessionId": "test-session-reject-draft",
        "expectedRevision": 0,
        "draft": timeline_draft_json(),
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("executeProjectIntent should return a structured envelope");

    assert_eq!(envelope["ok"], false, "{envelope:#}");
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(envelope["error"]["kind"], "invalidPayload");
    assert_eq!(envelope["error"]["command"], "executeProjectIntent");
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
    let reopened_after_redo = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("redo should save canonical project.json");
    assert_eq!(reopened_after_redo.bundle.draft.tracks[0].segments.len(), 1);

    close_project_session(json!({ "sessionId": "test-session-undo-redo" }))
        .expect("closeProjectSession should return an envelope");
}

fn save_timeline_draft(bundle_path: &std::path::Path) {
    let draft: Draft =
        serde_json::from_value(timeline_draft_json()).expect("timeline draft fixture should parse");
    save_project_bundle(&StdPlatformFileSystem, bundle_path, &draft)
        .expect("timeline draft fixture should be saved");
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

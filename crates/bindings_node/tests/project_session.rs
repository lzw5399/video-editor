use bindings_node::{
    close_project_session, create_project_session, execute_project_intent,
    list_project_session_materials, list_project_session_missing_materials, open_project_session,
};
use draft_model::Draft;
use media_runtime::discover_runtime_config;
use media_runtime_desktop::DesktopFfmpegExecutor;
use project_store::{StdPlatformFileSystem, open_project_bundle, save_project_bundle};
use serde_json::{Value, json};
use std::sync::{Mutex, OnceLock};
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
    assert_eq!(created["data"]["draft"]["draftId"], "session-created-draft");
    assert_eq!(
        created["data"]["draft"]["metadata"]["name"],
        "Session Created Project"
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
        imported["data"]["draft"]["materials"]
            .as_array()
            .unwrap()
            .len(),
        1
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
    assert_eq!(
        added["data"]["draft"]["tracks"][0]["segments"][0]["materialId"],
        "session-video-material"
    );

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
            "kind": "selectTimelineSegments",
            "segmentIds": ["segment-1"],
            "trackIds": ["video-track"]
        }
    }))
    .expect("selection intent should return an envelope");
    assert_eq!(selected["ok"], true, "{selected:#}");
    assert_eq!(selected["data"]["revision"], 1);
    assert_eq!(selected["data"]["selection"]["segmentIds"][0], "segment-1");

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
            "kind": "selectTimelineSegments",
            "segmentIds": [],
            "trackIds": ["video-track"]
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
    assert_eq!(
        renamed["data"]["draft"]["tracks"][0]["name"],
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
    assert_eq!(locked["data"]["draft"]["tracks"][0]["locked"], true);

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
    assert_eq!(hidden["data"]["draft"]["tracks"][0]["visible"], false);

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
    assert_eq!(muted["data"]["draft"]["tracks"][0]["muted"], true);

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
            "delta": 200_000
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

    let keyed = execute_project_intent(json!({
        "sessionId": "test-session-keyframe",
        "expectedRevision": 3,
        "intent": {
            "kind": "setSelectedSegmentKeyframe",
            "property": "volume",
            "at": 450_000,
            "interpolation": "hold",
            "easing": "easeIn"
        }
    }))
    .expect("keyframe intent should return an envelope");
    assert_eq!(keyed["ok"], true, "{keyed:#}");
    assert_eq!(keyed["data"]["revision"], 4);

    let keyframe = &keyed["data"]["draft"]["tracks"][0]["segments"][0]["keyframes"][0];
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

    let removed = execute_project_intent(json!({
        "sessionId": "test-session-keyframe",
        "expectedRevision": 4,
        "intent": {
            "kind": "removeSelectedSegmentKeyframe",
            "property": "volume",
            "at": 450_000
        }
    }))
    .expect("remove keyframe intent should return an envelope");
    assert_eq!(removed["ok"], true, "{removed:#}");
    assert_eq!(removed["data"]["revision"], 5);
    assert_eq!(
        removed["data"]["draft"]["tracks"][0]["segments"][0]["keyframes"]
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

    close_project_session(json!({ "sessionId": "test-session-keyframe-reject" }))
        .expect("closeProjectSession should return an envelope");
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

use bindings_node::{
    begin_project_interaction, cancel_project_interaction, close_project_session,
    commit_project_interaction, execute_project_intent, open_project_session,
    update_project_interaction,
};
use draft_model::Draft;
use project_store::{StdPlatformFileSystem, open_project_bundle, save_project_bundle};
use serde_json::{Value, json};
use std::fs;

#[test]
fn project_interaction_session_update_and_cancel_do_not_mutate_canonical_project() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("interaction-update-cancel.veproj");
    let revision = open_session_with_selected_segment(&bundle_path, "interaction-update-cancel");

    let begin = begin_visual_interaction("interaction-update-cancel", revision);
    let interaction_id = begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return an interaction id")
        .to_owned();
    assert_eq!(begin["data"]["baseRevision"], revision);
    assert_eq!(begin["data"]["revision"], revision);
    assert_eq!(begin["data"]["acceptedSequence"], 0);

    let before_update = project_json_bytes(&bundle_path);
    let updated = update_project_interaction(json!({
        "sessionId": "interaction-update-cancel",
        "expectedRevision": revision,
        "interactionId": interaction_id,
        "sequence": 1,
        "payload": {
            "kind": "selectedSegmentVisual",
            "patch": { "positionDeltaX": 120 }
        }
    }))
    .expect("updateProjectInteraction should return an envelope");
    assert_eq!(updated["ok"], true, "{updated:#}");
    assert_eq!(updated["data"]["baseRevision"], revision);
    assert_eq!(updated["data"]["revision"], revision);
    assert_eq!(updated["data"]["revisionUnchanged"], true);
    assert_eq!(updated["data"]["acceptedSequence"], 1);
    assert_eq!(updated["data"]["coalescedThrough"], 1);
    assert_eq!(
        updated["data"]["provisionalViewModel"]["selectedSegment"]["visual"]["transform"]["position"]
            ["x"],
        120
    );
    assert_eq!(
        project_json_bytes(&bundle_path),
        before_update,
        "live update must not save project.json"
    );

    let duplicate = update_project_interaction(json!({
        "sessionId": "interaction-update-cancel",
        "expectedRevision": revision,
        "interactionId": updated["data"]["interactionId"],
        "sequence": 1,
        "payload": {
            "kind": "selectedSegmentVisual",
            "patch": { "positionDeltaX": 130 }
        }
    }))
    .expect("duplicate update should return an envelope");
    assert_eq!(duplicate["ok"], false, "{duplicate:#}");
    assert!(
        duplicate["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Stale project interaction sequence")
    );

    let sequence_two = update_project_interaction(json!({
        "sessionId": "interaction-update-cancel",
        "expectedRevision": revision,
        "interactionId": updated["data"]["interactionId"],
        "sequence": 2,
        "payload": {
            "kind": "selectedSegmentVisual",
            "patch": { "positionDeltaX": 140 }
        }
    }))
    .expect("second update should return an envelope");
    assert_eq!(sequence_two["ok"], true, "{sequence_two:#}");
    assert_eq!(sequence_two["data"]["acceptedSequence"], 2);

    let out_of_order = update_project_interaction(json!({
        "sessionId": "interaction-update-cancel",
        "expectedRevision": revision,
        "interactionId": sequence_two["data"]["interactionId"],
        "sequence": 1,
        "payload": {
            "kind": "selectedSegmentVisual",
            "patch": { "positionDeltaX": 150 }
        }
    }))
    .expect("out-of-order update should return an envelope");
    assert_eq!(out_of_order["ok"], false, "{out_of_order:#}");

    let canceled = cancel_project_interaction(json!({
        "sessionId": "interaction-update-cancel",
        "expectedRevision": revision,
        "interactionId": sequence_two["data"]["interactionId"]
    }))
    .expect("cancelProjectInteraction should return an envelope");
    assert_eq!(canceled["ok"], true, "{canceled:#}");
    assert_eq!(canceled["data"]["revision"], revision);
    assert_eq!(canceled["data"]["revisionUnchanged"], true);
    assert_eq!(canceled["data"]["canceled"], true);
    assert_eq!(
        project_json_bytes(&bundle_path),
        before_update,
        "cancel must not save project.json"
    );

    let stale_id = update_project_interaction(json!({
        "sessionId": "interaction-update-cancel",
        "expectedRevision": revision,
        "interactionId": canceled["data"]["interactionId"],
        "sequence": 3,
        "payload": {
            "kind": "selectedSegmentVisual",
            "patch": { "positionDeltaX": 160 }
        }
    }))
    .expect("stale interaction id should return an envelope");
    assert_eq!(stale_id["ok"], false, "{stale_id:#}");
    assert!(
        stale_id["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("not found")
    );

    close_project_session(json!({ "sessionId": "interaction-update-cancel" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_interaction_session_commit_revalidates_and_saves_once() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("interaction-commit.veproj");
    let revision = open_session_with_selected_segment(&bundle_path, "interaction-commit");

    let begin = begin_visual_interaction("interaction-commit", revision);
    let interaction_id = begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return an interaction id")
        .to_owned();
    let before_update = project_json_bytes(&bundle_path);

    let updated = update_project_interaction(json!({
        "sessionId": "interaction-commit",
        "expectedRevision": revision,
        "interactionId": interaction_id,
        "sequence": 1,
        "payload": {
            "kind": "selectedSegmentVisual",
            "patch": { "positionDeltaX": 90 }
        }
    }))
    .expect("updateProjectInteraction should return an envelope");
    assert_eq!(updated["ok"], true, "{updated:#}");
    assert_eq!(project_json_bytes(&bundle_path), before_update);

    let committed = commit_project_interaction(json!({
        "sessionId": "interaction-commit",
        "expectedRevision": revision,
        "interactionId": updated["data"]["interactionId"]
    }))
    .expect("commitProjectInteraction should return an envelope");
    assert_eq!(committed["ok"], true, "{committed:#}");
    assert_eq!(committed["data"]["baseRevision"], revision);
    assert_eq!(committed["data"]["revision"], revision + 1);
    assert_eq!(committed["data"]["acceptedSequence"], 1);
    assert_eq!(committed["data"]["delta"]["command"], "updateSegmentVisual");
    assert_eq!(
        committed["data"]["viewModel"]["selectedSegment"]["visual"]["transform"]["position"]["x"],
        90
    );
    assert_eq!(
        committed["data"]["viewModel"]["editControls"]["canUndo"], true,
        "one committed interaction should create one undo item"
    );

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("committed interaction should save canonical project.json");
    assert_eq!(
        reopened.bundle.draft.tracks[0].segments[0]
            .visual
            .transform
            .position
            .x,
        90
    );

    let stale_id = commit_project_interaction(json!({
        "sessionId": "interaction-commit",
        "expectedRevision": revision + 1,
        "interactionId": committed["data"]["interactionId"]
    }))
    .expect("committed interaction id should be stale");
    assert_eq!(stale_id["ok"], false, "{stale_id:#}");

    close_project_session(json!({ "sessionId": "interaction-commit" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_interaction_session_rejects_stale_base_revision() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("interaction-stale-revision.veproj");
    let revision = open_session_with_selected_segment(&bundle_path, "interaction-stale-revision");

    let begin = begin_visual_interaction("interaction-stale-revision", revision);
    let interaction_id = begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return an interaction id")
        .to_owned();

    let canonical = execute_project_intent(json!({
        "sessionId": "interaction-stale-revision",
        "expectedRevision": revision,
        "intent": {
            "kind": "updateSelectedSegmentVisual",
            "patch": { "positionDeltaX": 10 }
        }
    }))
    .expect("canonical intent should return an envelope");
    assert_eq!(canonical["ok"], true, "{canonical:#}");
    assert_eq!(canonical["data"]["revision"], revision + 1);

    let stale = update_project_interaction(json!({
        "sessionId": "interaction-stale-revision",
        "expectedRevision": revision,
        "interactionId": interaction_id,
        "sequence": 1,
        "payload": {
            "kind": "selectedSegmentVisual",
            "patch": { "positionDeltaX": 20 }
        }
    }))
    .expect("stale update should return an envelope");
    assert_eq!(stale["ok"], false, "{stale:#}");
    assert!(
        stale["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Stale project session revision")
    );

    let stale_current_revision = cancel_project_interaction(json!({
        "sessionId": "interaction-stale-revision",
        "expectedRevision": revision + 1,
        "interactionId": interaction_id
    }))
    .expect("stale cancel should return an envelope");
    assert_eq!(
        stale_current_revision["ok"], false,
        "{stale_current_revision:#}"
    );
    assert!(
        stale_current_revision["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("not found")
    );

    close_project_session(json!({ "sessionId": "interaction-stale-revision" }))
        .expect("closeProjectSession should return an envelope");
}

fn open_session_with_selected_segment(bundle_path: &std::path::Path, session_id: &str) -> u64 {
    save_timeline_draft(bundle_path);
    let opened = open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": session_id
    }))
    .expect("openProjectSession should return an envelope");
    assert_eq!(opened["ok"], true, "{opened:#}");

    let added = execute_project_intent(json!({
        "sessionId": session_id,
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("add segment should return an envelope");
    assert_eq!(added["ok"], true, "{added:#}");
    added["data"]["revision"]
        .as_u64()
        .expect("add segment should return revision")
}

fn begin_visual_interaction(session_id: &str, revision: u64) -> Value {
    let begin = begin_project_interaction(json!({
        "sessionId": session_id,
        "expectedRevision": revision,
        "kind": "selectedSegmentVisual"
    }))
    .expect("beginProjectInteraction should return an envelope");
    assert_eq!(begin["ok"], true, "{begin:#}");
    begin
}

fn project_json_bytes(bundle_path: &std::path::Path) -> Vec<u8> {
    fs::read(bundle_path.join("project.json")).expect("project.json should be readable")
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
        "draftId": "interaction-session-draft",
        "metadata": { "name": "Interaction Session Draft" },
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

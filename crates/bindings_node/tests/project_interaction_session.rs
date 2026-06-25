use bindings_node::{
    begin_project_interaction, cancel_project_interaction, close_project_session,
    commit_project_interaction, execute_project_intent, open_project_session,
    update_project_interaction,
};
use draft_model::{
    Draft, Filter, Material, MaterialKind, Microseconds, Segment, SegmentId, SourceTimerange,
    TargetTimerange, Track, TrackKind, TrackTransition,
};
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
fn project_interaction_session_timeline_move_trim_validates_and_commits_once() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir
        .path()
        .join("interaction-timeline-move-trim.veproj");
    let revision =
        open_session_with_selected_segment(&bundle_path, "interaction-timeline-move-trim");

    let invalid_begin = begin_project_interaction(json!({
        "sessionId": "interaction-timeline-move-trim",
        "expectedRevision": revision,
        "kind": "timelineMoveTrim"
    }))
    .expect("begin timeline interaction should return an envelope");
    assert_eq!(invalid_begin["ok"], true, "{invalid_begin:#}");
    let invalid_interaction_id = invalid_begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return an interaction id")
        .to_owned();
    let invalid_update = update_project_interaction(json!({
        "sessionId": "interaction-timeline-move-trim",
        "expectedRevision": revision,
        "interactionId": invalid_interaction_id,
        "sequence": 1,
        "payload": {
            "kind": "timelineMoveTrim",
            "mode": "move",
            "startAt": 250_000,
            "targetTrackHandle": "timeline-track:audio-track"
        }
    }))
    .expect("invalid cross-track update should return an envelope");
    assert_eq!(invalid_update["ok"], false, "{invalid_update:#}");
    assert!(
        invalid_update["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("incompatible")
    );
    let canceled_invalid = cancel_project_interaction(json!({
        "sessionId": "interaction-timeline-move-trim",
        "expectedRevision": revision,
        "interactionId": invalid_interaction_id
    }))
    .expect("cancel invalid interaction should return an envelope");
    assert_eq!(canceled_invalid["ok"], true, "{canceled_invalid:#}");

    let begin = begin_project_interaction(json!({
        "sessionId": "interaction-timeline-move-trim",
        "expectedRevision": revision,
        "kind": "timelineMoveTrim"
    }))
    .expect("beginProjectInteraction should return an envelope");
    assert_eq!(begin["ok"], true, "{begin:#}");
    let interaction_id = begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return an interaction id")
        .to_owned();
    let before_update = project_json_bytes(&bundle_path);

    let moved = update_project_interaction(json!({
        "sessionId": "interaction-timeline-move-trim",
        "expectedRevision": revision,
        "interactionId": interaction_id,
        "sequence": 1,
        "payload": {
            "kind": "timelineMoveTrim",
            "mode": "move",
            "startAt": 250_000,
            "targetTrackHandle": "timeline-track:video-track-2"
        }
    }))
    .expect("timeline move update should return an envelope");
    assert_eq!(moved["ok"], true, "{moved:#}");
    assert_eq!(moved["data"]["kind"], "timelineMoveTrim");
    assert_eq!(moved["data"]["revision"], revision);
    assert_eq!(moved["data"]["revisionUnchanged"], true);
    assert_eq!(moved["data"]["provisionalDelta"]["command"], "moveSegment");
    assert_eq!(
        project_json_bytes(&bundle_path),
        before_update,
        "timeline move update must not save project.json"
    );

    let committed = commit_project_interaction(json!({
        "sessionId": "interaction-timeline-move-trim",
        "expectedRevision": revision,
        "interactionId": moved["data"]["interactionId"]
    }))
    .expect("timeline move commit should return an envelope");
    assert_eq!(committed["ok"], true, "{committed:#}");
    assert_eq!(committed["data"]["revision"], revision + 1);
    assert_eq!(committed["data"]["delta"]["command"], "moveSegment");
    assert_eq!(
        committed["data"]["viewModel"]["editControls"]["canUndo"],
        true
    );
    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("committed move should save canonical project.json");
    assert_eq!(reopened.bundle.draft.tracks[0].segments.len(), 0);
    assert_eq!(reopened.bundle.draft.tracks[1].segments.len(), 1);
    assert_eq!(
        reopened.bundle.draft.tracks[1].segments[0]
            .target_timerange
            .start
            .get(),
        250_000
    );

    let trim_revision = committed["data"]["revision"]
        .as_u64()
        .expect("commit should return revision");
    let trim_begin = begin_project_interaction(json!({
        "sessionId": "interaction-timeline-move-trim",
        "expectedRevision": trim_revision,
        "kind": "timelineMoveTrim"
    }))
    .expect("begin trim interaction should return an envelope");
    assert_eq!(trim_begin["ok"], true, "{trim_begin:#}");
    let trim_interaction_id = trim_begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return trim interaction id")
        .to_owned();
    let before_trim_update = project_json_bytes(&bundle_path);
    let trimmed = update_project_interaction(json!({
        "sessionId": "interaction-timeline-move-trim",
        "expectedRevision": trim_revision,
        "interactionId": trim_interaction_id,
        "sequence": 1,
        "payload": {
            "kind": "timelineMoveTrim",
            "mode": "trimLeft",
            "trimAt": 300_000
        }
    }))
    .expect("timeline trim update should return an envelope");
    assert_eq!(trimmed["ok"], true, "{trimmed:#}");
    assert_eq!(trimmed["data"]["revision"], trim_revision);
    assert_eq!(trimmed["data"]["revisionUnchanged"], true);
    assert_eq!(
        trimmed["data"]["provisionalDelta"]["command"],
        "trimSegment"
    );
    assert_eq!(project_json_bytes(&bundle_path), before_trim_update);

    let trim_commit = commit_project_interaction(json!({
        "sessionId": "interaction-timeline-move-trim",
        "expectedRevision": trim_revision,
        "interactionId": trimmed["data"]["interactionId"]
    }))
    .expect("timeline trim commit should return an envelope");
    assert_eq!(trim_commit["ok"], true, "{trim_commit:#}");
    assert_eq!(trim_commit["data"]["revision"], trim_revision + 1);
    assert_eq!(trim_commit["data"]["delta"]["command"], "trimSegment");
    let reopened_trim = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("committed trim should save canonical project.json");
    assert_eq!(
        reopened_trim.bundle.draft.tracks[1].segments[0]
            .target_timerange
            .start
            .get(),
        300_000
    );
    assert_eq!(
        reopened_trim.bundle.draft.tracks[1].segments[0]
            .target_timerange
            .duration
            .get(),
        950_000
    );

    close_project_session(json!({ "sessionId": "interaction-timeline-move-trim" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_interaction_session_keyframe_edit_updates_and_moves_segment_relative_keyframes() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("interaction-keyframe-edit.veproj");
    let revision = open_session_with_selected_segment(&bundle_path, "interaction-keyframe-edit");

    let begin = begin_keyframe_interaction("interaction-keyframe-edit", revision);
    let interaction_id = begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return an interaction id")
        .to_owned();
    let before_update = project_json_bytes(&bundle_path);

    let updated = update_project_interaction(json!({
        "sessionId": "interaction-keyframe-edit",
        "expectedRevision": revision,
        "interactionId": interaction_id,
        "sequence": 1,
        "payload": {
            "kind": "keyframeEdit",
            "property": "visualPositionX",
            "at": 400_000,
            "value": { "kind": "int", "value": 120 },
            "interpolation": "linear",
            "easing": "none"
        }
    }))
    .expect("keyframe update should return an envelope");
    assert_eq!(updated["ok"], true, "{updated:#}");
    assert_eq!(updated["data"]["kind"], "keyframeEdit");
    assert_eq!(updated["data"]["revision"], revision);
    assert_eq!(updated["data"]["revisionUnchanged"], true);
    assert_eq!(
        updated["data"]["provisionalDelta"]["command"],
        "setSegmentKeyframe"
    );
    assert_eq!(
        updated["data"]["provisionalViewModel"]["selectedSegment"]["keyframes"][0]["at"],
        400_000
    );
    assert_eq!(
        project_json_bytes(&bundle_path),
        before_update,
        "keyframe interaction update must not save project.json"
    );

    let committed = commit_project_interaction(json!({
        "sessionId": "interaction-keyframe-edit",
        "expectedRevision": revision,
        "interactionId": updated["data"]["interactionId"]
    }))
    .expect("keyframe commit should return an envelope");
    assert_eq!(committed["ok"], true, "{committed:#}");
    assert_eq!(committed["data"]["revision"], revision + 1);
    assert_eq!(committed["data"]["acceptedSequence"], 1);
    assert_eq!(committed["data"]["delta"]["command"], "setSegmentKeyframe");
    assert_eq!(
        committed["data"]["viewModel"]["editControls"]["canUndo"],
        true
    );

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("committed keyframe should save canonical project.json");
    let keyframes = &reopened.bundle.draft.tracks[0].segments[0].keyframes;
    assert_eq!(keyframes.len(), 1);
    assert_eq!(keyframes[0].at.get(), 400_000);
    assert_eq!(
        serde_json::to_value(&keyframes[0].value).expect("keyframe value should serialize"),
        json!({ "kind": "int", "value": 120 })
    );

    let move_revision = committed["data"]["revision"]
        .as_u64()
        .expect("commit should return revision");
    let move_begin = begin_keyframe_interaction("interaction-keyframe-edit", move_revision);
    let move_interaction_id = move_begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return an interaction id")
        .to_owned();
    let moved = update_project_interaction(json!({
        "sessionId": "interaction-keyframe-edit",
        "expectedRevision": move_revision,
        "interactionId": move_interaction_id,
        "sequence": 1,
        "payload": {
            "kind": "keyframeEdit",
            "property": "visualPositionX",
            "fromAt": 400_000,
            "at": 700_000
        }
    }))
    .expect("keyframe marker move should return an envelope");
    assert_eq!(moved["ok"], true, "{moved:#}");
    assert_eq!(moved["data"]["revisionUnchanged"], true);
    let move_commit = commit_project_interaction(json!({
        "sessionId": "interaction-keyframe-edit",
        "expectedRevision": move_revision,
        "interactionId": moved["data"]["interactionId"]
    }))
    .expect("keyframe marker move commit should return an envelope");
    assert_eq!(move_commit["ok"], true, "{move_commit:#}");
    let reopened_move = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("moved keyframe should save canonical project.json");
    let moved_keyframes = &reopened_move.bundle.draft.tracks[0].segments[0].keyframes;
    assert_eq!(
        moved_keyframes.len(),
        1,
        "marker drag should move, not copy"
    );
    assert_eq!(moved_keyframes[0].at.get(), 700_000);
    assert_eq!(
        serde_json::to_value(&moved_keyframes[0].value).expect("keyframe value should serialize"),
        json!({ "kind": "int", "value": 120 })
    );

    let duplicate_revision = move_commit["data"]["revision"]
        .as_u64()
        .expect("move commit should return revision");
    add_keyframe_via_interaction(
        "interaction-keyframe-edit",
        duplicate_revision,
        "visualPositionX",
        800_000,
        json!({ "kind": "int", "value": 240 }),
    );
    let duplicate_revision = duplicate_revision + 1;
    let duplicate_begin =
        begin_keyframe_interaction("interaction-keyframe-edit", duplicate_revision);
    let duplicate_interaction_id = duplicate_begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return an interaction id")
        .to_owned();
    let duplicate = update_project_interaction(json!({
        "sessionId": "interaction-keyframe-edit",
        "expectedRevision": duplicate_revision,
        "interactionId": duplicate_interaction_id,
        "sequence": 1,
        "payload": {
            "kind": "keyframeEdit",
            "property": "visualPositionX",
            "fromAt": 700_000,
            "at": 800_000
        }
    }))
    .expect("duplicate keyframe marker move should return an envelope");
    assert_eq!(duplicate["ok"], false, "{duplicate:#}");
    assert!(
        duplicate["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("duplicate keyframe")
    );

    close_project_session(json!({ "sessionId": "interaction-keyframe-edit" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn project_interaction_session_removes_nearest_keyframe_without_exact_playhead_match() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir
        .path()
        .join("interaction-keyframe-nearest-delete.veproj");
    let revision =
        open_session_with_selected_segment(&bundle_path, "interaction-keyframe-nearest-delete");
    add_keyframe_via_interaction(
        "interaction-keyframe-nearest-delete",
        revision,
        "visualOpacity",
        0,
        json!({ "kind": "uint", "value": 700 }),
    );
    let revision = revision + 1;

    let playhead = execute_project_intent(json!({
        "sessionId": "interaction-keyframe-nearest-delete",
        "expectedRevision": revision,
        "intent": {
            "kind": "setSessionPlayhead",
            "playhead": 120_000
        }
    }))
    .expect("setSessionPlayhead should return an envelope");
    assert_eq!(playhead["ok"], true, "{playhead:#}");
    assert_eq!(playhead["data"]["revision"], revision);

    let removed = execute_project_intent(json!({
        "sessionId": "interaction-keyframe-nearest-delete",
        "expectedRevision": revision,
        "intent": {
            "kind": "removeSelectedSegmentKeyframe",
            "property": "visualOpacity"
        }
    }))
    .expect("nearest keyframe remove should return an envelope");
    assert_eq!(removed["ok"], true, "{removed:#}");
    assert_eq!(removed["data"]["revision"], revision + 1);
    assert_eq!(removed["data"]["delta"]["command"], "removeSegmentKeyframe");

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("removed keyframe should save canonical project.json");
    assert!(
        reopened.bundle.draft.tracks[0].segments[0]
            .keyframes
            .is_empty(),
        "near-but-not-exact delete should remove the focused property keyframe"
    );

    close_project_session(json!({ "sessionId": "interaction-keyframe-nearest-delete" }))
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

#[test]
fn phase19_retime_effect_mask_blend_interactions_update_provisionally_and_commit_once() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("interaction-phase19-segment.veproj");
    let revision =
        open_phase19_session_with_selected_segment(&bundle_path, "interaction-phase19-segment");

    let retime_begin = begin_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "kind": "selectedSegmentRetime"
    }))
    .expect("begin retime interaction should return an envelope");
    assert_eq!(retime_begin["ok"], true, "{retime_begin:#}");
    let retime_id = retime_begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return retime interaction id")
        .to_owned();
    let before_retime = project_json_bytes(&bundle_path);
    let retimed = update_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "interactionId": retime_id,
        "sequence": 1,
        "payload": {
            "kind": "selectedSegmentRetime",
            "retiming": {
                "mode": {
                    "kind": "constant",
                    "speed": { "numerator": 1, "denominator": 2 }
                },
                "audioPolicy": "followVideoSpeed"
            }
        }
    }))
    .expect("retime update should return an envelope");
    assert_eq!(retimed["ok"], true, "{retimed:#}");
    assert_eq!(retimed["data"]["kind"], "selectedSegmentRetime");
    assert_eq!(retimed["data"]["revision"], revision);
    assert_eq!(retimed["data"]["revisionUnchanged"], true);
    assert_eq!(retimed["data"]["acceptedSequence"], 1);
    assert_eq!(retimed["data"]["coalescedThrough"], 1);
    assert_eq!(
        retimed["data"]["provisionalDelta"]["command"],
        "setSegmentRetime"
    );
    assert_eq!(project_json_bytes(&bundle_path), before_retime);

    let stale_retime = update_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "interactionId": retimed["data"]["interactionId"],
        "sequence": 1,
        "payload": {
            "kind": "selectedSegmentRetime",
            "retiming": {
                "mode": {
                    "kind": "constant",
                    "speed": { "numerator": 1, "denominator": 1 }
                },
                "audioPolicy": "followVideoSpeed"
            }
        }
    }))
    .expect("stale retime update should return an envelope");
    assert_eq!(stale_retime["ok"], false, "{stale_retime:#}");

    let retime_commit = commit_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "interactionId": retimed["data"]["interactionId"]
    }))
    .expect("retime commit should return an envelope");
    assert_eq!(retime_commit["ok"], true, "{retime_commit:#}");
    assert_eq!(retime_commit["data"]["revision"], revision + 1);
    assert_eq!(retime_commit["data"]["delta"]["command"], "setSegmentRetime");
    assert_eq!(
        retime_commit["data"]["viewModel"]["editControls"]["canUndo"],
        true
    );
    let reopened_retime = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("retime commit should save canonical project.json once");
    assert_eq!(
        reopened_retime.bundle.draft.tracks[0].segments[0]
            .retiming
            .mode,
        serde_json::from_value(json!({
            "kind": "constant",
            "speed": { "numerator": 1, "denominator": 2 }
        }))
        .expect("retime mode should parse")
    );

    let revision = revision + 1;
    let effect_begin = begin_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "kind": "selectedSegmentEffect"
    }))
    .expect("begin effect interaction should return an envelope");
    assert_eq!(effect_begin["ok"], true, "{effect_begin:#}");
    let effect_id = effect_begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return effect interaction id")
        .to_owned();
    let before_effect = project_json_bytes(&bundle_path);
    let effected = update_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "interactionId": effect_id,
        "sequence": 1,
        "payload": {
            "kind": "selectedSegmentEffect",
            "effectIndex": 0,
            "parameter": {
                "parameter": "gaussianBlurRadiusMillis",
                "radiusMillis": 750
            }
        }
    }))
    .expect("effect update should return an envelope");
    assert_eq!(effected["ok"], true, "{effected:#}");
    assert_eq!(effected["data"]["revision"], revision);
    assert_eq!(effected["data"]["revisionUnchanged"], true);
    assert_eq!(
        effected["data"]["provisionalDelta"]["command"],
        "updateSegmentEffectParameter"
    );
    assert_eq!(project_json_bytes(&bundle_path), before_effect);
    let effect_commit = commit_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "interactionId": effected["data"]["interactionId"]
    }))
    .expect("effect commit should return an envelope");
    assert_eq!(effect_commit["ok"], true, "{effect_commit:#}");
    assert_eq!(effect_commit["data"]["revision"], revision + 1);
    assert_eq!(
        effect_commit["data"]["delta"]["command"],
        "updateSegmentEffectParameter"
    );
    let reopened_effect = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("effect commit should save canonical project.json once");
    assert_eq!(
        serde_json::to_value(&reopened_effect.bundle.draft.tracks[0].segments[0].filters[0])
            .expect("filter should serialize")["kind"]["radiusMillis"],
        750
    );

    let revision = revision + 1;
    let mask_begin = begin_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "kind": "selectedSegmentMask"
    }))
    .expect("begin mask interaction should return an envelope");
    assert_eq!(mask_begin["ok"], true, "{mask_begin:#}");
    let mask_id = mask_begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return mask interaction id")
        .to_owned();
    let before_mask = project_json_bytes(&bundle_path);
    let masked = update_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "interactionId": mask_id,
        "sequence": 1,
        "payload": {
            "kind": "selectedSegmentMask",
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
    .expect("mask update should return an envelope");
    assert_eq!(masked["ok"], true, "{masked:#}");
    assert_eq!(masked["data"]["revision"], revision);
    assert_eq!(masked["data"]["revisionUnchanged"], true);
    assert_eq!(
        masked["data"]["provisionalDelta"]["command"],
        "updateSegmentVisual"
    );
    assert_eq!(project_json_bytes(&bundle_path), before_mask);
    let mask_commit = commit_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "interactionId": masked["data"]["interactionId"]
    }))
    .expect("mask commit should return an envelope");
    assert_eq!(mask_commit["ok"], true, "{mask_commit:#}");
    assert_eq!(mask_commit["data"]["revision"], revision + 1);
    let reopened_mask = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("mask commit should save canonical project.json once");
    assert_eq!(
        serde_json::to_value(&reopened_mask.bundle.draft.tracks[0].segments[0].visual.mask)
            .expect("mask should serialize")["kind"],
        "rectangle"
    );

    let revision = revision + 1;
    let blend_begin = begin_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "kind": "selectedSegmentBlend"
    }))
    .expect("begin blend interaction should return an envelope");
    assert_eq!(blend_begin["ok"], true, "{blend_begin:#}");
    let blend_id = blend_begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return blend interaction id")
        .to_owned();
    let before_blend = project_json_bytes(&bundle_path);
    let blended = update_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "interactionId": blend_id,
        "sequence": 1,
        "payload": {
            "kind": "selectedSegmentBlend",
            "opacityMillis": 650
        }
    }))
    .expect("blend opacity update should return an envelope");
    assert_eq!(blended["ok"], true, "{blended:#}");
    assert_eq!(blended["data"]["revision"], revision);
    assert_eq!(blended["data"]["revisionUnchanged"], true);
    assert_eq!(
        blended["data"]["provisionalDelta"]["command"],
        "updateSegmentVisual"
    );
    assert_eq!(project_json_bytes(&bundle_path), before_blend);
    let blend_commit = commit_project_interaction(json!({
        "sessionId": "interaction-phase19-segment",
        "expectedRevision": revision,
        "interactionId": blended["data"]["interactionId"]
    }))
    .expect("blend commit should return an envelope");
    assert_eq!(blend_commit["ok"], true, "{blend_commit:#}");
    assert_eq!(blend_commit["data"]["revision"], revision + 1);
    let reopened_blend = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("blend commit should save canonical project.json once");
    assert_eq!(
        reopened_blend.bundle.draft.tracks[0].segments[0]
            .visual
            .transform
            .opacity
            .value_millis,
        650
    );

    close_project_session(json!({ "sessionId": "interaction-phase19-segment" }))
        .expect("closeProjectSession should return an envelope");
}

#[test]
fn phase19_transition_duration_interaction_updates_provisionally_and_commits_once() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("interaction-phase19-transition.veproj");
    let revision =
        open_phase19_session_with_selected_segment(&bundle_path, "interaction-phase19-transition");

    let begin = begin_project_interaction(json!({
        "sessionId": "interaction-phase19-transition",
        "expectedRevision": revision,
        "kind": "selectedTransitionDuration"
    }))
    .expect("begin transition duration interaction should return an envelope");
    assert_eq!(begin["ok"], true, "{begin:#}");
    let interaction_id = begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return transition duration interaction id")
        .to_owned();
    let before_update = project_json_bytes(&bundle_path);

    let updated = update_project_interaction(json!({
        "sessionId": "interaction-phase19-transition",
        "expectedRevision": revision,
        "interactionId": interaction_id,
        "sequence": 1,
        "payload": {
            "kind": "selectedTransitionDuration",
            "fromSegmentId": "left-segment",
            "toSegmentId": "right-segment",
            "duration": 250_000
        }
    }))
    .expect("transition duration update should return an envelope");
    assert_eq!(updated["ok"], true, "{updated:#}");
    assert_eq!(updated["data"]["kind"], "selectedTransitionDuration");
    assert_eq!(updated["data"]["revision"], revision);
    assert_eq!(updated["data"]["revisionUnchanged"], true);
    assert_eq!(
        updated["data"]["provisionalDelta"]["command"],
        "updateTransitionDuration"
    );
    assert_eq!(
        project_json_bytes(&bundle_path),
        before_update,
        "transition duration update must not save project.json"
    );

    let committed = commit_project_interaction(json!({
        "sessionId": "interaction-phase19-transition",
        "expectedRevision": revision,
        "interactionId": updated["data"]["interactionId"]
    }))
    .expect("transition duration commit should return an envelope");
    assert_eq!(committed["ok"], true, "{committed:#}");
    assert_eq!(committed["data"]["revision"], revision + 1);
    assert_eq!(
        committed["data"]["delta"]["command"],
        "updateTransitionDuration"
    );
    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("transition duration commit should save canonical project.json once");
    assert_eq!(
        reopened.bundle.draft.tracks[0].transitions[0].duration,
        Microseconds::new(250_000)
    );

    close_project_session(json!({ "sessionId": "interaction-phase19-transition" }))
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

fn open_phase19_session_with_selected_segment(
    bundle_path: &std::path::Path,
    session_id: &str,
) -> u64 {
    save_phase19_interaction_draft(bundle_path);
    let opened = open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": session_id
    }))
    .expect("openProjectSession should return an envelope");
    assert_eq!(opened["ok"], true, "{opened:#}");

    let selected = execute_project_intent(json!({
        "sessionId": session_id,
        "expectedRevision": 0,
        "intent": {
            "kind": "selectTimelineItemIntent",
            "itemHandle": "timeline-segment:video-track:left-segment"
        }
    }))
    .expect("selectTimelineItemIntent should return an envelope");
    assert_eq!(selected["ok"], true, "{selected:#}");
    selected["data"]["revision"]
        .as_u64()
        .expect("selection should return revision")
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

fn begin_keyframe_interaction(session_id: &str, revision: u64) -> Value {
    let begin = begin_project_interaction(json!({
        "sessionId": session_id,
        "expectedRevision": revision,
        "kind": "keyframeEdit"
    }))
    .expect("beginProjectInteraction should return an envelope");
    assert_eq!(begin["ok"], true, "{begin:#}");
    begin
}

fn add_keyframe_via_interaction(
    session_id: &str,
    revision: u64,
    property: &str,
    at: u64,
    value: Value,
) -> Value {
    let begin = begin_keyframe_interaction(session_id, revision);
    let interaction_id = begin["data"]["interactionId"]
        .as_str()
        .expect("begin should return an interaction id")
        .to_owned();
    let updated = update_project_interaction(json!({
        "sessionId": session_id,
        "expectedRevision": revision,
        "interactionId": interaction_id,
        "sequence": 1,
        "payload": {
            "kind": "keyframeEdit",
            "property": property,
            "at": at,
            "value": value,
            "interpolation": "linear",
            "easing": "none"
        }
    }))
    .expect("keyframe update should return an envelope");
    assert_eq!(updated["ok"], true, "{updated:#}");
    let committed = commit_project_interaction(json!({
        "sessionId": session_id,
        "expectedRevision": revision,
        "interactionId": updated["data"]["interactionId"]
    }))
    .expect("keyframe commit should return an envelope");
    assert_eq!(committed["ok"], true, "{committed:#}");
    committed
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

fn save_phase19_interaction_draft(bundle_path: &std::path::Path) {
    let mut draft = Draft::new("phase19-interaction-draft", "Phase 19 Interaction Draft");
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
        .expect("phase19 interaction draft fixture should be saved");
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
        }, {
            "trackId": "video-track-2",
            "kind": "video",
            "name": "Video 2",
            "muted": false,
            "locked": false,
            "segments": []
        }, {
            "trackId": "audio-track",
            "kind": "audio",
            "name": "Audio",
            "muted": false,
            "locked": false,
            "segments": []
        }]
    })
}

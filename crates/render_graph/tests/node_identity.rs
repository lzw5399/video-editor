use draft_model::{
    Draft, Material, MaterialKind, Microseconds, RationalFrameRate, Segment, SourceTimerange,
    TargetTimerange, Track, TrackKind,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use render_graph::{
    OutputDimensions, RenderGraphNodeRole, RenderGraphSnapshot, RenderOutputProfile,
    build_render_graph,
};

#[test]
fn stable_node_ids_cover_render_graph_entries_with_semantic_keys() {
    let graph = graph_for(&phase13_graph_draft());

    assert_eq!(graph.canvas.node_id.role, RenderGraphNodeRole::Canvas);
    assert_eq!(
        graph.canvas.node_id.stable_key(),
        "draft:phase13-node-identity-draft:canvas"
    );
    assert_eq!(
        graph.materials[0].node_id.stable_key(),
        "draft:phase13-node-identity-draft:material:video-material"
    );
    assert_eq!(
        graph.video_layers[0].node_id.stable_key(),
        "draft:phase13-node-identity-draft:track:video-track:segment:segment-a:video"
    );
    assert_eq!(
        graph.audio_mixes[0].node_id.stable_key(),
        "draft:phase13-node-identity-draft:track:video-track:segment:segment-a:audio"
    );
    assert_eq!(
        graph.sampled_frames[0].node_id.stable_key(),
        "draft:phase13-node-identity-draft:frame:0:at:0"
    );
}

#[test]
fn stable_node_ids_survive_content_timing_and_material_metadata_changes() {
    let before = phase13_graph_draft();
    let mut after = before.clone();
    after.materials[0].display_name = "Renamed Video Material".to_owned();
    after.tracks[0].segments[0].target_timerange = TargetTimerange::new(0, 700_000);
    after.tracks[0].segments[0].source_timerange = SourceTimerange::new(100_000, 700_000);

    let before_graph = graph_for(&before);
    let after_graph = graph_for(&after);

    assert_eq!(
        before_graph.video_layers[0].node_id, after_graph.video_layers[0].node_id,
        "move/trim/content changes must keep the semantic segment node identity"
    );
    assert_eq!(
        before_graph.audio_mixes[0].node_id, after_graph.audio_mixes[0].node_id,
        "audio identity must also remain semantic for the same material-backed segment"
    );
    assert_ne!(
        before_graph.video_layers[0].target_timerange, after_graph.video_layers[0].target_timerange,
        "timing changes should affect graph content, not the stable node ID"
    );
}

#[test]
fn fingerprints_change_without_changing_node_identity() {
    let before = phase13_graph_draft();
    let mut semantic_edit = before.clone();
    semantic_edit.tracks[0].segments[0].visual.transform.opacity.value_millis = 750;
    let mut input_edit = before.clone();
    input_edit.materials[0].uri = "file://relinked-video.mp4".to_owned();

    let before_snapshot = snapshot_for(&before, output_profile(960, 540), "runtime:software:v1");
    let semantic_snapshot =
        snapshot_for(&semantic_edit, output_profile(960, 540), "runtime:software:v1");
    let input_snapshot = snapshot_for(&input_edit, output_profile(960, 540), "runtime:software:v1");
    let output_snapshot = snapshot_for(&before, output_profile(1280, 720), "runtime:software:v1");
    let runtime_snapshot = snapshot_for(&before, output_profile(960, 540), "runtime:hardware:v2");

    let video_key =
        "draft:phase13-node-identity-draft:track:video-track:segment:segment-a:video";
    let material_key = "draft:phase13-node-identity-draft:material:video-material";
    let before_video = before_snapshot
        .node_fingerprint_by_key(video_key)
        .expect("video fingerprint should exist");
    let semantic_video = semantic_snapshot
        .node_fingerprint_by_key(video_key)
        .expect("semantic edit fingerprint should exist");
    let input_material = input_snapshot
        .node_fingerprint_by_key(material_key)
        .expect("input edit fingerprint should exist");
    let before_material = before_snapshot
        .node_fingerprint_by_key(material_key)
        .expect("material fingerprint should exist");
    let output_video = output_snapshot
        .node_fingerprint_by_key(video_key)
        .expect("output fingerprint should exist");
    let runtime_video = runtime_snapshot
        .node_fingerprint_by_key(video_key)
        .expect("runtime fingerprint should exist");

    assert_eq!(before_video.node_id, semantic_video.node_id);
    assert_ne!(
        before_video.semantic_fingerprint,
        semantic_video.semantic_fingerprint
    );
    assert_ne!(
        before_material.input_fingerprint,
        input_material.input_fingerprint
    );
    assert_ne!(
        before_video.output_profile_fingerprint,
        output_video.output_profile_fingerprint
    );
    assert_ne!(
        before_video.runtime_capability_fingerprint,
        runtime_video.runtime_capability_fingerprint
    );
    assert_eq!(
        before_video.graph_schema_version,
        render_graph::GRAPH_SCHEMA_VERSION
    );
    assert_eq!(
        before_video.generator_version,
        render_graph::GRAPH_GENERATOR_VERSION
    );
}

#[test]
fn node_identity_target_keeps_semantic_segment_anchors_stable_across_content_changes() {
    let before = phase13_graph_draft();
    let mut after = before.clone();
    after.materials[0].display_name = "Renamed Video Material".to_owned();

    let before_graph = graph_for(&before);
    let after_graph = graph_for(&after);

    assert_eq!(
        before_graph
            .video_layers
            .iter()
            .map(|layer| (
                layer.track_id.as_str(),
                layer.segment_id.as_str(),
                layer.material_id.as_str()
            ))
            .collect::<Vec<_>>(),
        after_graph
            .video_layers
            .iter()
            .map(|layer| (
                layer.track_id.as_str(),
                layer.segment_id.as_str(),
                layer.material_id.as_str()
            ))
            .collect::<Vec<_>>(),
        "semantic track/segment/material anchors should remain stable while fingerprints later change"
    );
    assert_ne!(
        before_graph.materials[0].display_name, after_graph.materials[0].display_name,
        "content changes should be fingerprint inputs, not node identity inputs"
    );
}

#[test]
fn node_identity_target_preserves_integer_sampled_frame_anchors() {
    let graph = graph_for(&phase13_graph_draft());

    assert_eq!(
        graph
            .sampled_frames
            .iter()
            .map(|frame| (frame.frame_index, frame.at.get()))
            .collect::<Vec<_>>(),
        vec![(0, 0), (1, 33_333), (2, 66_666)]
    );
}

fn graph_for(draft: &Draft) -> render_graph::RenderGraph {
    let profile = EngineProfile::from_draft_canvas(draft).expect("canvas profile should resolve");
    let normalized = normalize_draft(draft, &profile).expect("draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::new(0), Microseconds::new(100_000)),
    )
    .expect("range should resolve");
    build_render_graph(&normalized, &range).expect("graph should build")
}

fn snapshot_for(
    draft: &Draft,
    output_profile: RenderOutputProfile,
    runtime_capability_fingerprint: &str,
) -> RenderGraphSnapshot {
    let graph = graph_for(draft);
    RenderGraphSnapshot::from_graph(&graph, &output_profile, runtime_capability_fingerprint)
}

fn output_profile(width: u32, height: u32) -> RenderOutputProfile {
    RenderOutputProfile::preview_frame_png(
        OutputDimensions::new(width, height),
        RationalFrameRate::new(30, 1),
        TargetTimerange::new(Microseconds::new(0), Microseconds::new(100_000)),
    )
}

fn phase13_graph_draft() -> Draft {
    let mut draft = Draft::new("phase13-node-identity-draft", "Phase 13 Node Identity");
    draft.materials.push(video_material());
    let mut track = Track::new("video-track", TrackKind::Video, "Video");
    track.segments.push(Segment::new(
        "segment-a",
        "video-material",
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    ));
    draft.tracks.push(track);
    draft
}

fn video_material() -> Material {
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        "file://video.mp4",
        "Video Material",
    );
    material.metadata.duration = Some(Microseconds::new(1_000_000));
    material.metadata.width = Some(1920);
    material.metadata.height = Some(1080);
    material.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
    material.metadata.has_video = true;
    material.metadata.has_audio = true;
    material
}

use draft_model::{
    Draft, Material, MaterialKind, Microseconds, RationalFrameRate, Segment, SourceTimerange,
    TargetTimerange, Track, TrackKind,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use render_graph::build_render_graph;

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

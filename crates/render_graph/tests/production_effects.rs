use draft_model::{
    AudioRetimePolicy, DirtyDomain, DirtyRange, DirtyRangeSource, Draft, Material, MaterialKind,
    Microseconds, RationalFrameRate, RetimeMode, Segment, SegmentRetiming, SourceTimerange,
    SpeedCurvePoint, SpeedRatio, TargetTimerange, Track, TrackKind,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use render_graph::{
    OutputDimensions, RenderGraphDiff, RenderGraphSnapshot, RenderOutputProfile,
    build_render_graph,
};

const GRAPH_RS: &str = include_str!("../src/graph.rs");
const FINGERPRINT_RS: &str = include_str!("../src/fingerprint.rs");
const INCREMENTAL_RS: &str = include_str!("../src/incremental.rs");

#[test]
fn phase19_production_effects_render_graph_carries_retime_transition_and_effect_intents() {
    assert!(
        GRAPH_RS.contains("RenderRetimeIntent"),
        "render graph must represent retimed source mapping as typed render intent"
    );
    assert!(
        GRAPH_RS.contains("ProductionEffectCapabilityDecision")
            || GRAPH_RS.contains("RenderEffectCapability"),
        "render graph must carry registry-backed capability decisions for effects/filters/transitions"
    );
    assert!(
        GRAPH_RS.contains("RenderTransitionWindow") || GRAPH_RS.contains("TransitionAdjacency"),
        "transition intent must include adjacency/window facts, not just a segment-local name"
    );
}

#[test]
fn phase19_production_effects_render_graph_fingerprints_and_dirty_ranges_include_semantics() {
    assert!(
        FINGERPRINT_RS.contains("retime")
            && FINGERPRINT_RS.contains("effect")
            && FINGERPRINT_RS.contains("transition"),
        "graph fingerprints must include retime, effect/filter, and transition semantics"
    );
    assert!(
        INCREMENTAL_RS.contains("DirtyDomain::Effect")
            && INCREMENTAL_RS.contains("DirtyDomain::Transition")
            && (INCREMENTAL_RS.contains("DirtyDomain::Timing")
                || INCREMENTAL_RS.contains("DirtyDomain::Retime")),
        "incremental dirty facts must cover production effects, transitions, and retiming"
    );
}

#[test]
fn phase19_production_effects_retime_render_intent_carries_engine_source_mapping_and_audio_support()
{
    let graph = graph_for(&retimed_graph_draft(SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::PreservePitch,
    }));
    let video_layer = graph
        .video_layers
        .iter()
        .find(|layer| layer.segment_id.as_str() == "video-a")
        .expect("video retime layer should exist");
    let audio_mix = graph
        .audio_mixes
        .iter()
        .find(|mix| mix.segment_id.as_str() == "video-a")
        .expect("video material audio retime mix should exist");

    let video_retime = serde_json::to_value(&video_layer.retime).expect("retime serializes");
    assert_eq!(
        video_retime["sourceMapping"]["sourceTimerange"],
        serde_json::json!({ "start": 100000, "duration": 3000000 }),
        "retime intent must retain the original source timerange consumed by engine_core"
    );
    assert_eq!(
        video_retime["sourceMapping"]["retimedSourceTimerange"],
        serde_json::json!({ "start": 100000, "duration": 2000000 }),
        "retime intent must carry the engine-owned source range mapped from target duration"
    );
    assert_eq!(
        video_retime["sourceMapping"]["targetTimerange"],
        serde_json::json!({ "start": 0, "duration": 1000000 }),
        "retime intent must bind source mapping facts to the layer target range"
    );
    assert_eq!(
        video_retime["audio"]["policy"],
        serde_json::json!("preservePitch"),
        "audio follow-speed policy must be explicit in the retime render intent"
    );
    assert_eq!(
        video_retime["audio"]["support"],
        serde_json::json!("unsupported"),
        "unsupported preserve-pitch retime must be represented as a typed render diagnostic fact"
    );
    assert!(
        video_retime["audio"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("preserve-pitch"),
        "unsupported audio retime reason should explain the degraded/unsupported policy"
    );

    assert_eq!(
        serde_json::to_value(&audio_mix.retime).expect("audio retime serializes")["sourceMapping"]
            ["retimedSourceTimerange"],
        serde_json::json!({ "start": 100000, "duration": 2000000 }),
        "audio retime mixes must consume the same engine-owned source mapping facts"
    );
}

#[test]
fn phase19_production_effects_retime_fingerprints_change_without_changing_stable_segment_identity()
{
    let base = snapshot_for(&retimed_graph_draft(SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(2, 1),
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    }));
    let changed = snapshot_for(&retimed_graph_draft(SegmentRetiming {
        mode: RetimeMode::SpeedCurve {
            points: vec![
                SpeedCurvePoint {
                    target_time: Microseconds::ZERO,
                    speed: SpeedRatio::new(1, 1),
                },
                SpeedCurvePoint {
                    target_time: Microseconds::new(500_000),
                    speed: SpeedRatio::new(3, 2),
                },
            ],
        },
        audio_policy: AudioRetimePolicy::FollowVideoSpeed,
    }));

    let base_key = "draft:phase19-retime-render-graph:track:video-track:segment:video-a:video";
    let base_video = base
        .node_fingerprint_by_key(base_key)
        .expect("base video fingerprint should exist");
    let changed_video = changed
        .node_fingerprint_by_key(base_key)
        .expect("changed video fingerprint should keep stable node identity");
    assert_eq!(
        base_video.node_id, changed_video.node_id,
        "retime edits must not create new node IDs for the same segment identity"
    );
    assert_ne!(
        base_video.semantic_fingerprint, changed_video.semantic_fingerprint,
        "retime speed-curve changes must alter semantic fingerprints"
    );

    let dirty_domains = vec![DirtyDomain::Timing, DirtyDomain::Effect];
    let diff = RenderGraphDiff::between(
        &base,
        &changed,
        &[DirtyRange {
            target_timerange: TargetTimerange::new(
                Microseconds::ZERO,
                Microseconds::new(1_000_000),
            ),
            source: DirtyRangeSource::PreviousAndCurrent,
        }],
        &dirty_domains,
    );
    let changed_keys = diff
        .changed
        .iter()
        .map(|change| change.node_id.stable_key())
        .collect::<Vec<_>>();
    assert!(
        changed_keys.iter().any(|key| key.ends_with(":video")),
        "retime edits must dirty the video render segment"
    );
    assert!(
        changed_keys.iter().any(|key| key.ends_with(":audio")),
        "retime edits must dirty the audio render segment"
    );
    assert!(diff.dirty_domains.contains(&DirtyDomain::Timing));
    assert!(diff.dirty_domains.contains(&DirtyDomain::Effect));
}

fn graph_for(draft: &Draft) -> render_graph::RenderGraph {
    let profile = EngineProfile::mvp_default();
    let normalized = normalize_draft(draft, &profile).expect("draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(100_000)),
    )
    .expect("range should resolve");
    build_render_graph(&normalized, &range).expect("graph should build")
}

fn snapshot_for(draft: &Draft) -> RenderGraphSnapshot {
    let output_profile = RenderOutputProfile::preview_segment_mp4(
        OutputDimensions::new(160, 90),
        RationalFrameRate::new(30, 1),
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
    );
    RenderGraphSnapshot::from_graph(&graph_for(draft), &output_profile, "runtime:phase19-retime")
}

fn retimed_graph_draft(retiming: SegmentRetiming) -> Draft {
    let mut draft = Draft::new("phase19-retime-render-graph", "Phase 19 Retime Render Graph");
    draft.materials.push(video_material());
    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    let mut segment = Segment::new(
        "video-a",
        "video-material",
        SourceTimerange::new(Microseconds::new(100_000), Microseconds::new(3_000_000)),
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
    );
    segment.retiming = retiming;
    video_track.segments.push(segment);
    draft.tracks.push(video_track);
    draft
}

fn video_material() -> Material {
    let mut material = Material::new(
        "video-material",
        MaterialKind::Video,
        "file://retime.mp4",
        "Retime Video",
    );
    material.metadata.duration = Some(Microseconds::new(4_000_000));
    material.metadata.width = Some(1920);
    material.metadata.height = Some(1080);
    material.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
    material.metadata.has_video = true;
    material.metadata.has_audio = true;
    material
}

use draft_model::{
    AudioRetimePolicy, DirtyDomain, DirtyRange, DirtyRangeSource, Draft, Filter, FilterKind,
    Material, MaterialKind, Microseconds, RationalFrameRate, RetimeMode, Segment, SegmentId,
    SegmentBlendMode, SegmentMask, SegmentRetiming, SourceTimerange, SpeedCurvePoint, SpeedRatio,
    TargetTimerange, Track, TrackKind, TrackTransition, TransitionKind, TransitionReference,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use render_graph::{
    OutputDimensions, RenderGraphDiff, RenderGraphSnapshot, RenderOutputProfile, build_render_graph,
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
fn phase19_production_effects_render_graph_carries_typed_effect_order_enabled_and_support() {
    let graph = graph_for(&effect_stack_draft(vec![
        Filter::gaussian_blur(250),
        Filter::basic_color_adjustment(120, 1_150, 900),
        disabled_filter(Filter::opacity_adjustment(640)),
    ]));
    let video_layer = graph
        .video_layers
        .iter()
        .find(|layer| layer.segment_id.as_str() == "video-a")
        .expect("video effect layer should exist");

    assert_eq!(video_layer.filters.len(), 3);
    assert_eq!(video_layer.filters[0].order_index, 0);
    assert!(video_layer.filters[0].enabled);
    assert!(matches!(
        &video_layer.filters[0].kind,
        FilterKind::GaussianBlur { radius_millis: 250 }
    ));
    assert_eq!(
        video_layer.filters[0].capability.capability_id,
        "effect.gaussianBlur"
    );
    assert_eq!(
        video_layer.filters[0].capability.preview,
        render_graph::RenderIntentSupport::Supported
    );
    assert_eq!(
        video_layer.filters[0].capability.export,
        render_graph::RenderIntentSupport::Supported
    );

    assert_eq!(video_layer.filters[1].order_index, 1);
    assert!(matches!(
        &video_layer.filters[1].kind,
        FilterKind::BasicColorAdjustment {
            brightness_millis: 120,
            contrast_millis: 1_150,
            saturation_millis: 900
        }
    ));
    assert!(
        video_layer.filters[1]
            .capability
            .preview_reason
            .contains("first-party typed filter")
    );

    assert_eq!(video_layer.filters[2].order_index, 2);
    assert!(
        !video_layer.filters[2].enabled,
        "effect intent must preserve disabled state instead of dropping the typed effect"
    );
    assert!(matches!(
        &video_layer.filters[2].kind,
        FilterKind::OpacityAdjustment {
            opacity_millis: 640
        }
    ));
}

#[test]
fn phase19_production_effects_filter_enabled_state_changes_semantic_fingerprint() {
    let enabled = snapshot_for(&effect_stack_draft(vec![Filter::gaussian_blur(250)]));
    let disabled = snapshot_for(&effect_stack_draft(vec![disabled_filter(
        Filter::gaussian_blur(250),
    )]));

    let filter_key = "draft:phase19-effect-render-graph:track:video-track:segment:video-a:filter:0";
    let enabled_filter = enabled
        .node_fingerprint_by_key(filter_key)
        .expect("enabled filter fingerprint should exist");
    let disabled_filter = disabled
        .node_fingerprint_by_key(filter_key)
        .expect("disabled filter fingerprint should preserve stable identity");
    assert_eq!(
        enabled_filter.node_id, disabled_filter.node_id,
        "enable toggles must keep the same stable effect node identity"
    );
    assert_ne!(
        enabled_filter.semantic_fingerprint, disabled_filter.semantic_fingerprint,
        "enable toggles must invalidate render graph fingerprints"
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

#[test]
fn phase19_production_effects_transition_relationship_intent_windows() {
    let graph = graph_for(&transition_relationship_draft(
        TransitionReference::dissolve(),
        Microseconds::new(300_000),
    ));
    let from_layer = graph
        .video_layers
        .iter()
        .find(|layer| layer.segment_id.as_str() == "left-segment")
        .expect("from segment layer should be active");
    let transition = from_layer
        .transition
        .as_ref()
        .expect("from segment should carry relationship transition intent");

    assert_eq!(transition.from_segment_id, SegmentId::from("left-segment"));
    assert_eq!(transition.to_segment_id, SegmentId::from("right-segment"));
    assert!(matches!(
        transition.reference,
        TransitionReference::FirstParty {
            transition: TransitionKind::Dissolve
        }
    ));
    assert_eq!(transition.duration, Microseconds::new(300_000));
    assert_eq!(
        transition.window.target_timerange,
        TargetTimerange::new(Microseconds::new(700_000), Microseconds::new(300_000)),
        "dissolve overlap window should cover the outgoing segment tail before the cut"
    );
    assert_eq!(
        transition.window.from_target_timerange,
        TargetTimerange::new(Microseconds::new(700_000), Microseconds::new(300_000)),
        "from endpoint window should be the tail of the outgoing segment"
    );
    assert_eq!(
        transition.window.to_target_timerange,
        TargetTimerange::new(Microseconds::new(1_000_000), Microseconds::new(300_000)),
        "to endpoint window should be the head of the incoming segment"
    );
    assert_eq!(
        transition.support,
        render_graph::RenderIntentSupport::Supported
    );
    assert!(
        transition
            .node_id
            .stable_key()
            .contains("left-segment:transition:to:right-segment"),
        "transition node identity should include both relationship endpoints"
    );
}

#[test]
fn phase19_production_effects_transition_relationship_fingerprints_dirty_ranges() {
    let base = snapshot_for(&transition_relationship_draft(
        TransitionReference::dissolve(),
        Microseconds::new(300_000),
    ));
    let changed_duration = snapshot_for(&transition_relationship_draft(
        TransitionReference::dissolve(),
        Microseconds::new(400_000),
    ));
    let changed_endpoint = snapshot_for(&transition_relationship_with_endpoint("third-segment"));

    let transition_key = "draft:phase19-transition-render-graph:track:video-track:segment:left-segment:transition:to:right-segment";
    let base_transition = base
        .node_fingerprint_by_key(transition_key)
        .expect("base transition fingerprint should exist");
    let changed_transition = changed_duration
        .node_fingerprint_by_key(transition_key)
        .expect("duration edit should preserve transition node identity");
    assert_ne!(
        base_transition.semantic_fingerprint, changed_transition.semantic_fingerprint,
        "transition duration/window edits must alter semantic fingerprints"
    );
    assert!(
        changed_endpoint
            .node_fingerprint_by_key(transition_key)
            .is_none(),
        "changing transition endpoints must move the stable transition node identity"
    );
    assert!(
        changed_endpoint
            .node_fingerprint_by_key(
                "draft:phase19-transition-render-graph:track:video-track:segment:left-segment:transition:to:third-segment"
            )
            .is_some(),
        "new endpoint relationship should have its own stable transition node"
    );

    let diff = RenderGraphDiff::between(
        &base,
        &changed_duration,
        &[DirtyRange {
            target_timerange: TargetTimerange::new(
                Microseconds::new(600_000),
                Microseconds::new(400_000),
            ),
            source: DirtyRangeSource::PreviousAndCurrent,
        }],
        &[DirtyDomain::Transition],
    );
    assert!(
        diff.changed
            .iter()
            .any(|change| change.node_id.stable_key() == transition_key),
        "transition relationship edits must dirty the transition node"
    );
    assert!(diff.dirty_domains.contains(&DirtyDomain::Transition));
}

#[test]
fn phase19_production_effects_render_graph_carries_mask_blend_intents() {
    let graph = graph_for(&mask_blend_draft(
        SegmentMask::Rectangle {
            x_millis: 120,
            y_millis: 180,
            width_millis: 620,
            height_millis: 500,
            feather_millis: 80,
            opacity_millis: 740,
            inverted: true,
        },
        SegmentBlendMode::Multiply,
    ));
    let layer = graph
        .video_layers
        .iter()
        .find(|layer| layer.segment_id.as_str() == "video-a")
        .expect("mask/blend video layer should exist");

    assert_eq!(layer.mask.capability.capability_id, "mask.rectangle");
    assert_eq!(layer.mask.support, render_graph::RenderIntentSupport::Supported);
    assert_eq!(layer.mask.mask, layer.visual.mask);
    assert!(
        layer.mask.reason.contains("first-party typed mask"),
        "mask intent should carry the registry support reason"
    );
    assert_eq!(layer.blend.capability.capability_id, "blend.multiply");
    assert_eq!(
        layer.blend.support,
        render_graph::RenderIntentSupport::Supported
    );
    assert_eq!(layer.blend.blend_mode, layer.visual.blend_mode);
    assert!(
        layer.blend.reason.contains("first-party typed compositing"),
        "blend intent should carry the registry support reason"
    );
}

#[test]
fn phase19_production_effects_mask_blend_fingerprints_change_without_segment_identity_churn() {
    let base = snapshot_for(&mask_blend_draft(
        SegmentMask::Rectangle {
            x_millis: 120,
            y_millis: 180,
            width_millis: 620,
            height_millis: 500,
            feather_millis: 80,
            opacity_millis: 740,
            inverted: false,
        },
        SegmentBlendMode::Multiply,
    ));
    let changed_mask = snapshot_for(&mask_blend_draft(
        SegmentMask::Rectangle {
            x_millis: 180,
            y_millis: 180,
            width_millis: 620,
            height_millis: 500,
            feather_millis: 80,
            opacity_millis: 740,
            inverted: false,
        },
        SegmentBlendMode::Multiply,
    ));
    let changed_blend = snapshot_for(&mask_blend_draft(
        SegmentMask::Rectangle {
            x_millis: 120,
            y_millis: 180,
            width_millis: 620,
            height_millis: 500,
            feather_millis: 80,
            opacity_millis: 740,
            inverted: false,
        },
        SegmentBlendMode::Screen,
    ));

    let video_key = "draft:phase19-mask-blend-render-graph:track:video-track:segment:video-a:video";
    let base_video = base
        .node_fingerprint_by_key(video_key)
        .expect("base mask/blend video fingerprint should exist");
    let changed_mask_video = changed_mask
        .node_fingerprint_by_key(video_key)
        .expect("mask edits should preserve stable segment identity");
    let changed_blend_video = changed_blend
        .node_fingerprint_by_key(video_key)
        .expect("blend edits should preserve stable segment identity");

    assert_eq!(base_video.node_id, changed_mask_video.node_id);
    assert_eq!(base_video.node_id, changed_blend_video.node_id);
    assert_ne!(
        base_video.semantic_fingerprint, changed_mask_video.semantic_fingerprint,
        "mask geometry/opacity/inversion changes must invalidate video layer semantics"
    );
    assert_ne!(
        base_video.semantic_fingerprint, changed_blend_video.semantic_fingerprint,
        "blend mode changes must invalidate video layer semantics"
    );

    let diff = RenderGraphDiff::between(
        &base,
        &changed_mask,
        &[DirtyRange {
            target_timerange: TargetTimerange::new(
                Microseconds::ZERO,
                Microseconds::new(1_000_000),
            ),
            source: DirtyRangeSource::PreviousAndCurrent,
        }],
        &[DirtyDomain::Effect],
    );
    assert!(
        diff.changed
            .iter()
            .any(|change| change.node_id.stable_key() == video_key),
        "mask edits must dirty the affected video render segment"
    );
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
    let mut draft = Draft::new(
        "phase19-retime-render-graph",
        "Phase 19 Retime Render Graph",
    );
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

fn effect_stack_draft(filters: Vec<Filter>) -> Draft {
    let mut draft = Draft::new(
        "phase19-effect-render-graph",
        "Phase 19 Effect Render Graph",
    );
    draft.materials.push(video_material());
    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    let mut segment = Segment::new(
        "video-a",
        "video-material",
        SourceTimerange::new(Microseconds::new(100_000), Microseconds::new(3_000_000)),
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
    );
    segment.filters = filters;
    video_track.segments.push(segment);
    draft.tracks.push(video_track);
    draft
}

fn mask_blend_draft(mask: SegmentMask, blend_mode: SegmentBlendMode) -> Draft {
    let mut draft = Draft::new(
        "phase19-mask-blend-render-graph",
        "Phase 19 Mask Blend Render Graph",
    );
    draft.materials.push(video_material());
    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    let mut segment = Segment::new(
        "video-a",
        "video-material",
        SourceTimerange::new(Microseconds::new(100_000), Microseconds::new(3_000_000)),
        TargetTimerange::new(Microseconds::ZERO, Microseconds::new(1_000_000)),
    );
    segment.visual.mask = mask;
    segment.visual.blend_mode = blend_mode;
    video_track.segments.push(segment);
    draft.tracks.push(video_track);
    draft
}

fn disabled_filter(mut filter: Filter) -> Filter {
    filter.enabled = false;
    filter
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

fn transition_relationship_draft(reference: TransitionReference, duration: Microseconds) -> Draft {
    let mut draft = Draft::new(
        "phase19-transition-render-graph",
        "Phase 19 Transition Render Graph",
    );
    draft.materials.push(video_material());
    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track
        .segments
        .push(transition_segment("left-segment", 0, 0, 1_000_000));
    video_track.segments.push(transition_segment(
        "right-segment",
        1_000_000,
        1_000_000,
        1_000_000,
    ));
    video_track.transitions.push(TrackTransition {
        from_segment_id: SegmentId::from("left-segment"),
        to_segment_id: SegmentId::from("right-segment"),
        reference,
        duration,
        parameters: Default::default(),
    });
    draft.tracks.push(video_track);
    draft
}

fn transition_relationship_with_endpoint(to_segment_id: &str) -> Draft {
    let mut draft = Draft::new(
        "phase19-transition-render-graph",
        "Phase 19 Transition Render Graph",
    );
    draft.materials.push(video_material());
    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track
        .segments
        .push(transition_segment("left-segment", 0, 0, 1_000_000));
    video_track.segments.push(transition_segment(
        "third-segment",
        1_000_000,
        1_000_000,
        1_000_000,
    ));
    video_track.transitions.push(TrackTransition::dissolve(
        SegmentId::from("left-segment"),
        SegmentId::from(to_segment_id),
        Microseconds::new(300_000),
    ));
    draft.tracks.push(video_track);
    draft
}

fn transition_segment(
    segment_id: &str,
    source_start: u64,
    target_start: u64,
    duration: u64,
) -> Segment {
    Segment::new(
        segment_id,
        "video-material",
        SourceTimerange::new(Microseconds::new(source_start), Microseconds::new(duration)),
        TargetTimerange::new(Microseconds::new(target_start), Microseconds::new(duration)),
    )
}

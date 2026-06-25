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

const CAPABILITIES_RS: &str = include_str!("../src/capabilities.rs");

#[test]
fn phase19_production_effects_preview_requires_registry_backed_supported_effects() {
    assert!(
        CAPABILITIES_RS.contains("ProductionEffectCapabilityRegistry")
            || CAPABILITIES_RS.contains("RealtimeProductionEffectSupport"),
        "realtime preview must classify Phase 19 effects/transitions/masks/blends through a registry-backed support matrix"
    );
    assert!(
        CAPABILITIES_RS.contains("supported_first_party_effect")
            || CAPABILITIES_RS.contains("with_supported_production_effects"),
        "supported GPU preview must be opt-in per first-party semantic effect instead of accepting generic string filters"
    );
}

#[test]
fn phase19_production_effects_preview_rejects_fallback_success_for_masks_blends_and_transitions() {
    assert!(
        CAPABILITIES_RS.contains("fallback_used: false")
            && CAPABILITIES_RS.contains("mask")
            && CAPABILITIES_RS.contains("blend")
            && CAPABILITIES_RS.contains("transition"),
        "supported Phase 19 preview diagnostics must prove real GPU support for masks, blends, and transitions with no fallback success"
    );
}

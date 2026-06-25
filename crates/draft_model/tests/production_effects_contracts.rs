const TIMELINE_RS: &str = include_str!("../src/timeline.rs");
const LIB_RS: &str = include_str!("../src/lib.rs");

#[test]
fn phase19_production_effects_contracts_capability_registry_requires_typed_support_states() {
    assert!(
        LIB_RS.contains("effects") && TIMELINE_RS.contains("ProductionEffectCapabilityRegistry"),
        "Phase 19 requires a first-party capability registry exported from draft_model before UI/backend implementation"
    );
    assert!(
        TIMELINE_RS.contains("EffectSupportState")
            || TIMELINE_RS.contains("CapabilitySupportState"),
        "Phase 19 support must be typed as supported/degraded/unsupported/external rather than inferred from strings"
    );
}

#[test]
fn phase19_production_effects_contracts_first_party_filter_transition_and_retime_are_not_stringly()
{
    assert!(
        !TIMELINE_RS.contains("pub struct Filter {\n    pub name: String"),
        "first-party filters/effects must no longer persist only name + string parameters"
    );
    assert!(
        !TIMELINE_RS.contains("pub struct Transition {\n    pub name: String"),
        "first-party transitions must no longer persist only name + duration"
    );
    assert!(
        TIMELINE_RS.contains("SegmentRetiming") || TIMELINE_RS.contains("RetimeCurve"),
        "segments must carry typed retiming/speed semantics in draft_model"
    );
}

#[test]
fn phase19_production_effects_contracts_external_provider_ids_remain_compatibility_references_only()
{
    assert!(
        TIMELINE_RS.contains("ExternalEffectReference")
            || TIMELINE_RS.contains("ExternalProviderReference"),
        "external provider IDs should be explicit compatibility references, not internal render semantics"
    );
    assert!(
        !TIMELINE_RS.contains("kaipaiEffectId") && !TIMELINE_RS.contains("jianyingEffectId"),
        "provider-native effect IDs must not appear as first-party draft fields"
    );
}

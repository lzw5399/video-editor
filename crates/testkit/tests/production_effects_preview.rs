const TEMPLATE_IMPORT_PREVIEW_RS: &str = include_str!("template_import_preview.rs");

#[test]
fn phase19_production_effects_preview_fixtures_cover_retime_effect_transition_parity() {
    assert!(
        TEMPLATE_IMPORT_PREVIEW_RS.contains("production-effects")
            || TEMPLATE_IMPORT_PREVIEW_RS.contains("phase19"),
        "testkit preview fixtures must add a Phase 19 production-effects case family before implementation is accepted"
    );
    assert!(
        TEMPLATE_IMPORT_PREVIEW_RS.contains("retime")
            && TEMPLATE_IMPORT_PREVIEW_RS.contains("transition")
            && TEMPLATE_IMPORT_PREVIEW_RS.contains("fallback"),
        "production preview fixtures must cover retime, transition/effect semantics, and fallback reports"
    );
}

#[test]
fn phase19_production_effects_preview_fixtures_record_performance_evidence() {
    assert!(
        TEMPLATE_IMPORT_PREVIEW_RS.contains("queueLatencyUs")
            || TEMPLATE_IMPORT_PREVIEW_RS.contains("performance_budget"),
        "Phase 19 preview fixtures must record performance evidence for complex template-like edits"
    );
}

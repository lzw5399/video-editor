const TEMPLATE_IMPORT_EXPORTS_RS: &str = include_str!("template_import_exports.rs");

#[test]
fn phase19_production_effects_export_fixtures_cover_preview_export_parity() {
    assert!(
        TEMPLATE_IMPORT_EXPORTS_RS.contains("production-effects")
            || TEMPLATE_IMPORT_EXPORTS_RS.contains("phase19"),
        "testkit export fixtures must add a Phase 19 production-effects case family before implementation is accepted"
    );
    assert!(
        TEMPLATE_IMPORT_EXPORTS_RS.contains("preview_export_parity")
            || TEMPLATE_IMPORT_EXPORTS_RS.contains("retime")
            || TEMPLATE_IMPORT_EXPORTS_RS.contains("transition"),
        "production exports must verify retime/effect/transition preview-export parity, not just output existence"
    );
}

#[test]
fn phase19_production_effects_export_fixtures_reject_fallback_reports_as_success() {
    assert!(
        TEMPLATE_IMPORT_EXPORTS_RS.contains("fallback")
            && TEMPLATE_IMPORT_EXPORTS_RS.contains("NeedsNativeEffect"),
        "Phase 19 export fixtures must keep fallback/degraded reports explicit instead of treating them as product success"
    );
}

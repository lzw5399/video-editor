const LIB_RS: &str = include_str!("../src/lib.rs");
const FILTERS_RS: &str = include_str!("../src/filters.rs");
const JOB_RS: &str = include_str!("../src/job.rs");

#[test]
fn phase19_production_effects_compiler_owns_filtergraph_output_from_render_intent() {
    assert!(
        LIB_RS.contains("production_effects") || FILTERS_RS.contains("compile_production_effect"),
        "ffmpeg_compiler must expose compiler-owned production effect filtergraph compilation"
    );
    assert!(
        FILTERS_RS.contains("RenderRetimeIntent")
            && FILTERS_RS.contains("RenderTransitionWindow")
            && FILTERS_RS.contains("ProductionEffectCapabilityDecision"),
        "compiler output must be derived from typed render graph retime, transition, and effect intents"
    );
}

#[test]
fn phase19_production_effects_compiler_classifies_unsupported_export_paths() {
    assert!(
        JOB_RS.contains("UnsupportedProductionEffect")
            || FILTERS_RS.contains("UnsupportedProductionEffect"),
        "unsupported Phase 19 export semantics must be classified instead of silently compiling fallback filters"
    );
    assert!(
        FILTERS_RS.contains("setpts") && FILTERS_RS.contains("xfade"),
        "retime and transition compiler support must be explicit FFmpeg compiler output, never renderer strings"
    );
}

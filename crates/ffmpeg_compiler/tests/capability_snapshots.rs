mod common;

use ffmpeg_compiler::{FfmpegCompileErrorKind, compile_ffmpeg_job};

#[test]
fn capability_classifies_missing_text_font_without_silent_fallback() {
    let error = compile_ffmpeg_job(&common::export_plan(), &common::no_font_context())
        .expect_err("missing font should be classified");

    assert_eq!(error.kind, FfmpegCompileErrorKind::MissingTextFont);
    assert!(error.message.contains("VE_TEXT_FONT_PATH"));
}

#[test]
fn capability_classifies_missing_ass_or_subtitle_filter_support() {
    let error = compile_ffmpeg_job(
        &common::export_plan(),
        &common::no_subtitle_filter_context(),
    )
    .expect_err("missing ASS filter support should be classified");

    assert_eq!(error.kind, FfmpegCompileErrorKind::MissingTextFilterSupport);
    assert!(error.remediation.contains("ASS"));
}

#[test]
fn capability_classifies_unsupported_text_bubble_effect_and_font_refs() {
    let error = compile_ffmpeg_job(
        &common::export_plan_with_unsupported_text_resources(),
        &common::compile_context(),
    )
    .expect_err("unsupported text resources should be classified before ASS output");

    assert_eq!(error.kind, FfmpegCompileErrorKind::UnsupportedTextResource);
    assert!(error.message.contains("fontRef vendor-font-ref"));
    assert!(error.message.contains("bubble"));
    assert!(error.message.contains("effect"));
    assert!(error.remediation.contains("Remove or replace"));
}

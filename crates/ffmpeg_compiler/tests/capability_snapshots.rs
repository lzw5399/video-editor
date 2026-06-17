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

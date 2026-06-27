mod common;

use draft_model::{BUNDLED_TEXT_FONT_FAMILY, BUNDLED_TEXT_FONT_RELATIVE_PATH};
use ffmpeg_compiler::{FfmpegSidecarKind, compile_ffmpeg_job};

#[test]
fn ass_text_sidecar_snapshot_contains_deterministic_style_timing_and_escaping() {
    let job = compile_ffmpeg_job(&common::export_plan(), &common::compile_context())
        .expect("export with text should compile");
    let ass = job
        .sidecars
        .iter()
        .find(|sidecar| sidecar.kind == FfmpegSidecarKind::AssSubtitle)
        .expect("text overlay should generate an ASS sidecar");

    assert_eq!(
        ass.path,
        "/derived/draft-compiler-export-mp4-h264-balanced-text.ass"
    );
    assert!(ass.contents.contains("PlayResX: 1920"));
    assert!(
        ass.contents
            .contains("Style: Style0_text-a,PingFang SC,48,&H00FFCC33,&H00101010,&H80202020,0,0,0,0,100,100,6,0,3,2,4,2,192,192,108,1")
    );
    assert!(ass.contents.contains(
        "Dialogue: 2,0:00:00.00,0:00:00.10,Style0_text-a,text-a,192,192,108,,标题 \\{一\\}\\N第二行"
    ));
    assert!(ass.contents.contains("FontPath: /fonts/PingFang.ttc"));
    assert!(ass.contents.contains("; TextBox: 1152x280"));
    assert!(ass.contents.contains("; LayoutRegion: 192,756 1536x216"));
    assert!(ass.contents.contains("; LineHeightMillis: 1500"));
}

#[test]
fn ass_text_sidecar_resolves_bundled_font_ref_through_registry() {
    let job = compile_ffmpeg_job(
        &common::export_plan_with_bundled_font_ref(),
        &common::compile_context(),
    )
    .expect("export with bundled text font should compile");
    let ass = job
        .sidecars
        .iter()
        .find(|sidecar| sidecar.kind == FfmpegSidecarKind::AssSubtitle)
        .expect("text overlay should generate an ASS sidecar");

    assert!(ass.contents.contains(&format!(
        "Style: Style0_text-a,{BUNDLED_TEXT_FONT_FAMILY},48"
    )));
    assert!(
        ass.contents
            .contains(&format!("FontPath: {BUNDLED_TEXT_FONT_RELATIVE_PATH}"))
    );
}

#[test]
fn ass_text_sidecar_uses_engine_resolved_auto_wrapping() {
    let job = compile_ffmpeg_job(
        &common::export_plan_with_wrapped_text(),
        &common::compile_context(),
    )
    .expect("export with wrapped text should compile");
    let ass = job
        .sidecars
        .iter()
        .find(|sidecar| sidecar.kind == FfmpegSidecarKind::AssSubtitle)
        .expect("text overlay should generate an ASS sidecar");

    assert!(ass.contents.contains(
        "Dialogue: 2,0:00:00.00,0:00:00.10,Style0_text-a,text-a,192,192,108,,abcde\\Nfghij"
    ));
}

#[test]
fn ass_text_sidecar_batches_same_font_directory_overlays() {
    let job = compile_ffmpeg_job(
        &common::export_plan_with_overlapping_text_overlays_same_font(),
        &common::compile_context(),
    )
    .expect("export with multiple bundled text overlays should compile");
    let ass_sidecars = job
        .sidecars
        .iter()
        .filter(|sidecar| sidecar.kind == FfmpegSidecarKind::AssSubtitle)
        .collect::<Vec<_>>();

    assert_eq!(ass_sidecars.len(), 1);
    assert!(job.filter_script.matches("subtitles=").count() == 1);
    assert!(
        ass_sidecars[0]
            .contents
            .contains("Dialogue: 2,0:00:00.00,0:00:00.10,Style0_text-a,text-a")
    );
    assert!(
        ass_sidecars[0]
            .contents
            .contains("Dialogue: 3,0:00:00.00,0:00:00.10,Style1_text-b,text-b")
    );
}

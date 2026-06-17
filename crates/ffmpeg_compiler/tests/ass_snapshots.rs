mod common;

use ffmpeg_compiler::{FfmpegSidecarKind, compile_ffmpeg_job};

#[test]
fn ass_sidecar_snapshot_contains_deterministic_style_timing_and_escaping() {
    let job = compile_ffmpeg_job(&common::export_plan(), &common::compile_context())
        .expect("export with text should compile");
    let ass = job
        .sidecars
        .iter()
        .find(|sidecar| sidecar.kind == FfmpegSidecarKind::AssSubtitle)
        .expect("text overlay should generate an ASS sidecar");

    assert_eq!(
        ass.path,
        "/derived/draft-compiler-export-mp4-h264-balanced-text-text-a.ass"
    );
    assert!(ass.contents.contains("PlayResX: 1920"));
    assert!(
        ass.contents
            .contains("Style: Default,PingFang SC,48,&H00FFCC33,&H00101010,&H80202020")
    );
    assert!(ass.contents.contains(
        "Dialogue: 2,0:00:00.000,0:00:00.100,Default,text-a,96,96,54,,标题 \\\\{一\\\\}\\\\N第二行"
    ));
    assert!(ass.contents.contains("FontPath: /fonts/PingFang.ttc"));
}

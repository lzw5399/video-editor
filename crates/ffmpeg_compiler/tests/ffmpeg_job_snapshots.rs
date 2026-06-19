mod common;

use ffmpeg_compiler::{FfmpegCompileErrorKind, FfmpegOutputKind, compile_ffmpeg_job};

#[test]
fn ffmpeg_job_preview_frame_snapshot_contains_structured_args_and_filter_sidecars() {
    let job = compile_ffmpeg_job(
        &common::preview_frame_plan(),
        &common::preview_frame_context(),
    )
    .expect("preview frame job should compile");

    assert_eq!(job.output_kind, FfmpegOutputKind::PreviewFramePng);
    assert_eq!(job.output_path, "/derived/preview.png");
    assert_eq!(
        job.args_as_strings(),
        vec![
            "-y",
            "-i",
            "/media/audio.wav",
            "-i",
            "/media/overlay.png",
            "-i",
            "/media/video.mp4",
            "-filter_complex_script",
            "/derived/draft-compiler-preview-frame-png-filter.ffscript",
            "-map",
            "[vout]",
            "-frames:v",
            "1",
            "-f",
            "image2",
            "-c:v",
            "png",
            "/derived/preview.png",
        ]
    );
    assert_eq!(job.inputs.len(), 3);
    assert_eq!(
        job.sidecars[0].path,
        "/derived/draft-compiler-preview-frame-png-filter.ffscript"
    );
    assert!(
        job.sidecars[0]
            .contents
            .contains("subtitles='/derived/draft-compiler-preview-frame-png-text-text-a.ass'")
    );
    assert_eq!(job.validation.expect_audio_stream, false);
}

#[test]
fn ffmpeg_job_preview_segment_and_export_share_graph_compiler_path() {
    let preview = compile_ffmpeg_job(&common::preview_segment_plan(), &common::compile_context())
        .expect("preview segment job should compile");
    let export = compile_ffmpeg_job(&common::export_plan(), &common::compile_context())
        .expect("export job should compile");

    assert_eq!(preview.output_kind, FfmpegOutputKind::PreviewSegmentMp4);
    assert_eq!(export.output_kind, FfmpegOutputKind::ExportMp4);
    assert!(preview.args_as_strings().contains(&"libx264".to_owned()));
    assert!(preview.args_as_strings().contains(&"aac".to_owned()));
    assert!(export.args_as_strings().contains(&"libx264".to_owned()));
    assert!(export.args_as_strings().contains(&"192k".to_owned()));
    for expected in ["overlay=x=0:y=0", "subtitles=", "amix=inputs=2"] {
        assert!(preview.filter_script.contains(expected));
        assert!(export.filter_script.contains(expected));
    }
    assert_eq!(preview.validation.expect_audio_stream, true);
    assert_eq!(export.validation.expect_audio_stream, true);
}

#[test]
fn filters_snapshot_uses_stable_labels_for_video_audio_text_outputs() {
    let job = compile_ffmpeg_job(&common::export_plan(), &common::compile_context())
        .expect("export job should compile");

    assert_eq!(
        job.filter_script,
        [
            "[2:v]trim=start=0.700000:duration=0.100000,setpts=PTS-STARTPTS,scale=1920:1080[v0]",
            "[1:v]trim=start=0.600000:duration=0.100000,setpts=PTS-STARTPTS,scale=1920:1080[v1]",
            "[v0][v1]overlay=x=0:y=0:shortest=1[vbase1]",
            "[vbase1]subtitles='/derived/draft-compiler-export-mp4-h264-balanced-text-text-a.ass'[vtext0]",
            "[vtext0]format=yuv420p[vout]",
            "[0:a]atrim=start=0.600000:duration=0.100000,asetpts=PTS-STARTPTS,volume=1.000[a0]",
            "[2:a]atrim=start=0.700000:duration=0.100000,asetpts=PTS-STARTPTS,volume=1.000[a1]",
            "[a0][a1]amix=inputs=2:duration=longest:normalize=0[aout]",
        ]
        .join(";\n")
    );
}

#[test]
fn audio_filters_compile_gain_pan_fades_and_classify_unsupported_effect_slots() {
    let job = compile_ffmpeg_job(
        &common::export_plan_with_audio_mix_intent(),
        &common::compile_context(),
    )
    .expect("audio mix intent export should compile");

    assert!(
        job.filter_script
            .contains("atrim=start=0.600000:duration=0.100000")
    );
    assert!(job.filter_script.contains("volume=0.750"));
    assert!(
        job.filter_script
            .contains("pan=stereo|c0=1.000*c0|c1=0.500*c1")
    );
    assert!(job.filter_script.contains("afade=t=in:st=0:d=0.100000"));
    assert!(
        job.filter_script
            .contains("afade=t=out:st=0.000000:d=0.200000")
    );
    assert_eq!(job.filter_script_diagnostics.len(), 1);
    assert_eq!(
        job.filter_script_diagnostics[0].reason,
        "unsupported audio effect slot external-space is preserved for diagnostics"
    );
    assert_eq!(job.validation.expect_audio_stream, true);
}

#[test]
fn ffmpeg_job_classifies_missing_encoder_and_output_path_preconditions() {
    let encoder_error = compile_ffmpeg_job(&common::export_plan(), &common::no_h264_context())
        .expect_err("missing h264 encoder should be classified");
    assert_eq!(
        encoder_error.kind,
        FfmpegCompileErrorKind::UnsupportedEncoder
    );

    let missing_output = compile_ffmpeg_job(
        &common::export_plan(),
        &common::compile_context().with_output_path(""),
    )
    .expect_err("empty output path should be classified");
    assert_eq!(
        missing_output.kind,
        FfmpegCompileErrorKind::MissingOutputPath
    );
}

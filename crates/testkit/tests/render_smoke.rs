use testkit::{
    assert_tiny_smoke_metadata, probe_media_metadata, run_tiny_render_smoke, SmokeMetadata,
};

#[test]
fn render_smoke_asserts_generated_output_metadata() {
    let smoke = run_tiny_render_smoke().expect(
        "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
    );

    assert!(smoke.output_path().is_file());
    assert_tiny_smoke_metadata(smoke.metadata())
        .expect("tiny render smoke metadata should match the Phase 1 harness contract");
}

#[test]
fn render_smoke_probe_metadata_reports_video_and_audio_streams() {
    let media = testkit::generate_tiny_lavfi_media().expect(
        "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
    );

    let metadata: SmokeMetadata = probe_media_metadata(media.output_path())
        .expect("ffprobe should return metadata for generated media");

    assert_tiny_smoke_metadata(&metadata)
        .expect("generated lavfi media should have expected smoke metadata");
}

use testkit::{
    SmokeMetadata, assert_tiny_smoke_metadata, probe_media_metadata, run_tiny_render_smoke,
};

#[test]
fn render_smoke_asserts_generated_output_metadata() {
    let smoke = run_tiny_render_smoke().expect(
        "ffmpeg and ffprobe must be available in the bundled runtime directory; run pnpm --dir apps/desktop-electron run provision:ffmpeg-runtime or set VE_BUNDLED_FFMPEG_DIR",
    );

    assert!(smoke.output_path().is_file());
    assert_tiny_smoke_metadata(smoke.metadata())
        .expect("tiny render smoke metadata should match the Phase 1 harness contract");
}

#[test]
fn render_smoke_probe_metadata_reports_video_and_audio_streams() {
    let media = testkit::generate_tiny_lavfi_media().expect(
        "ffmpeg and ffprobe must be available in the bundled runtime directory; run pnpm --dir apps/desktop-electron run provision:ffmpeg-runtime or set VE_BUNDLED_FFMPEG_DIR",
    );

    let metadata: SmokeMetadata = probe_media_metadata(media.output_path())
        .expect("ffprobe should return metadata for generated media");

    assert_tiny_smoke_metadata(&metadata)
        .expect("generated lavfi media should have expected smoke metadata");
}

use media_runtime::{discover_runtime_config, probe_material_metadata};
use media_runtime_desktop::DesktopFfmpegExecutor;
use testkit::generate_material_fixtures;

#[test]
fn material_fixtures_generate_temp_media_for_all_supported_kinds() {
    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
    );
    let executor = DesktopFfmpegExecutor::default();
    let fixtures = generate_material_fixtures(&executor, &runtime)
        .expect("video image and audio fixtures should generate");

    assert_eq!(fixtures.len(), 3);

    for fixture in fixtures {
        assert!(fixture.path().is_file());
        assert!(
            fixture
                .path()
                .ancestors()
                .any(|path| path.file_name().and_then(|value| value.to_str())
                    == Some("media-generated")),
            "generated material should live under a temp media-generated directory"
        );

        let metadata = probe_material_metadata(&executor, &runtime, fixture.path())
            .expect("generated material should probe");
        fixture
            .assert_probe_metadata(&metadata)
            .expect("probe metadata should match generated fixture expectations");
    }
}

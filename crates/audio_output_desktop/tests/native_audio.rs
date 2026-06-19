use audio_output_desktop::native_audio_probe;

#[test]
fn native_audio_proof_is_explicitly_env_gated() {
    if std::env::var_os("VIDEO_EDITOR_TEST_NATIVE_AUDIO").is_none() {
        let diagnostic = native_audio_probe();
        eprintln!("{}", diagnostic.message);
        assert!(diagnostic.skipped);
        assert!(
            diagnostic
                .message
                .contains("VIDEO_EDITOR_TEST_NATIVE_AUDIO=1")
        );
        return;
    }

    let diagnostic = native_audio_probe();
    assert!(
        !diagnostic.skipped,
        "native proof should run instead of skip when env var is set"
    );
    assert!(
        diagnostic.ready || diagnostic.message.contains("no output device"),
        "native proof must return safe readiness or no-device diagnostics"
    );
}

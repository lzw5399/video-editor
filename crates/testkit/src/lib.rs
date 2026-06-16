//! Test harness shell for fixtures, goldens, and render smoke checks.
//!
//! Later Phase 1 plans add deterministic schema fixtures, tiny media generation,
//! and FFmpeg render smoke helpers here. This shell exists so downstream plans
//! can depend on a stable testkit crate without introducing media behavior early.

/// Boundary marker for Phase 1 test harness helpers.
pub const TESTKIT_BOUNDARY: &str = "fixtures-goldens-render-smoke-shell";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_tiny_lavfi_media_creates_temporary_mp4() {
        let media = generate_tiny_lavfi_media().expect(
            "ffmpeg and ffprobe must be available; set VE_FFMPEG_PATH/VE_FFPROBE_PATH or install them on PATH",
        );

        assert!(
            media.output_path().is_file(),
            "tiny lavfi smoke output should exist"
        );
        assert_eq!(media.output_path().extension().and_then(|value| value.to_str()), Some("mp4"));
        assert!(
            media.output_path()
                .ancestors()
                .any(|path| path.file_name().and_then(|value| value.to_str()) == Some("media-generated")),
            "generated media should live under a media-generated temp directory"
        );
    }
}

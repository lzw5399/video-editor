#[cfg(test)]
mod tests {
    use draft_model::{Microseconds, RationalFrameRate};

    use super::{PlaybackGeneration, PlaybackRate, PlaybackState, TimelineClock, TimelineFreshness};

    #[test]
    fn freshness_generation_increments_for_timeline_invalidations() {
        let mut clock = TimelineClock::new(
            Microseconds::new(0),
            RationalFrameRate::new(30_000, 1_001),
            PlaybackRate::normal(),
        );

        assert_eq!(clock.generation(), PlaybackGeneration::initial());
        assert_eq!(clock.state(), PlaybackState::Stopped);

        clock.seek(Microseconds::new(1_000_000));
        clock.start_scrub();
        clock.commit_scrub(Microseconds::new(2_000_000));
        clock.play();
        clock.pause();
        clock.resume();
        clock.stop();
        clock.accepted_edit();
        clock.draft_reloaded();
        clock.material_relinked();
        clock.surface_detached();
        clock.reset_runtime();

        assert_eq!(clock.generation().get(), 12);
        assert_eq!(clock.position(), Microseconds::ZERO);
        assert_eq!(clock.state(), PlaybackState::Stopped);
    }

    #[test]
    fn freshness_serializes_integer_microseconds_and_rational_rates() {
        let clock = TimelineClock::new(
            Microseconds::new(12_345_678),
            RationalFrameRate::new(24_000, 1_001),
            PlaybackRate::new(3, 2).expect("valid rational playback rate"),
        );

        let json = serde_json::to_value(&clock).expect("clock serializes");

        assert_eq!(json["position"], 12_345_678);
        assert_eq!(json["frameRate"]["numerator"], 24_000);
        assert_eq!(json["frameRate"]["denominator"], 1_001);
        assert_eq!(json["playbackRate"]["numerator"], 3);
        assert_eq!(json["playbackRate"]["denominator"], 2);
        assert!(json["position"].is_u64());

        let round_tripped: TimelineClock = serde_json::from_value(json).expect("clock deserializes");
        assert_eq!(round_tripped.position(), Microseconds::new(12_345_678));
        assert_eq!(
            round_tripped.playback_rate(),
            PlaybackRate::new(3, 2).unwrap()
        );
    }

    #[test]
    fn target_timeline_freshness_matches_preview_request_field_names() {
        let freshness = TimelineFreshness::new(
            Microseconds::new(1_234_567),
            PlaybackGeneration::new(42),
        )
        .with_project_session("session-a", 7);

        let json = serde_json::to_value(&freshness).expect("freshness serializes");

        assert_eq!(json["targetTime"], 1_234_567);
        assert_eq!(json["playbackGeneration"], 42);
        assert_eq!(json["projectSessionId"], "session-a");
        assert_eq!(json["expectedRevision"], 7);
        assert!(json.get("ffmpegPath").is_none());
        assert!(json.get("workerName").is_none());
    }

    #[test]
    fn playback_rate_rejects_non_rational_values() {
        assert!(PlaybackRate::new(1, 0).is_err());
        assert!(PlaybackRate::new(0, 1).is_err());
    }
}

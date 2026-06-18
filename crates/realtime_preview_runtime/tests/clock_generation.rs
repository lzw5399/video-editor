use draft_model::{Microseconds, RationalFrameRate};
use realtime_preview_runtime::{
    PlaybackGeneration, PlaybackRate, PlaybackState, TimelineClock,
};

#[test]
fn clock_generation_increments_for_all_invalidating_operations() {
    let mut clock = TimelineClock::new(
        Microseconds::new(0),
        RationalFrameRate::new(30_000, 1_001),
        PlaybackRate::normal(),
    );

    assert_eq!(clock.generation(), PlaybackGeneration::initial());
    assert_eq!(clock.position(), Microseconds::new(0));
    assert_eq!(clock.state(), PlaybackState::Stopped);

    let operations: [fn(&mut TimelineClock); 11] = [
        |clock| clock.seek(Microseconds::new(1_000_000)),
        TimelineClock::start_scrub,
        |clock| clock.commit_scrub(Microseconds::new(2_000_000)),
        TimelineClock::play,
        TimelineClock::pause,
        TimelineClock::resume,
        TimelineClock::stop,
        TimelineClock::accepted_edit,
        TimelineClock::draft_reloaded,
        TimelineClock::material_relinked,
        TimelineClock::surface_detached,
    ];

    for (index, operation) in operations.into_iter().enumerate() {
        operation(&mut clock);
        assert_eq!(clock.generation().get(), (index as u64) + 1);
    }

    clock.reset_runtime();
    assert_eq!(clock.generation().get(), 12);
    assert_eq!(clock.position(), Microseconds::ZERO);
    assert_eq!(clock.state(), PlaybackState::Stopped);
}

#[test]
fn clock_generation_serializes_integer_microseconds_and_rational_rates() {
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

    let round_tripped: TimelineClock =
        serde_json::from_value(json).expect("clock deserializes");
    assert_eq!(round_tripped.position(), Microseconds::new(12_345_678));
    assert_eq!(round_tripped.playback_rate(), PlaybackRate::new(3, 2).unwrap());
}

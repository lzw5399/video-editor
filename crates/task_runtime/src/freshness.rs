use std::error::Error;
use std::fmt;

use draft_model::{Microseconds, RationalFrameRate};
use serde::{Deserialize, Serialize};

/// Monotonic generation for stale-sensitive playback and timeline work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlaybackGeneration(u64);

impl PlaybackGeneration {
    pub const fn initial() -> Self {
        Self(0)
    }

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl Default for PlaybackGeneration {
    fn default() -> Self {
        Self::initial()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackState {
    Stopped,
    Paused,
    Playing,
    Scrubbing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackRate {
    pub numerator: i32,
    pub denominator: u32,
}

impl PlaybackRate {
    pub fn new(numerator: i32, denominator: u32) -> Result<Self, PlaybackRateError> {
        if denominator == 0 {
            return Err(PlaybackRateError::ZeroDenominator);
        }
        if numerator == 0 {
            return Err(PlaybackRateError::ZeroNumerator);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    pub const fn normal() -> Self {
        Self {
            numerator: 1,
            denominator: 1,
        }
    }
}

impl Default for PlaybackRate {
    fn default() -> Self {
        Self::normal()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackRateError {
    ZeroDenominator,
    ZeroNumerator,
}

impl fmt::Display for PlaybackRateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDenominator => write!(formatter, "playback rate denominator must be nonzero"),
            Self::ZeroNumerator => write!(formatter, "playback rate numerator must be nonzero"),
        }
    }
}

impl Error for PlaybackRateError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineClock {
    position: Microseconds,
    frame_rate: RationalFrameRate,
    playback_rate: PlaybackRate,
    state: PlaybackState,
    generation: PlaybackGeneration,
}

impl TimelineClock {
    pub fn new(
        position: Microseconds,
        frame_rate: RationalFrameRate,
        playback_rate: PlaybackRate,
    ) -> Self {
        Self {
            position,
            frame_rate,
            playback_rate,
            state: PlaybackState::Stopped,
            generation: PlaybackGeneration::initial(),
        }
    }

    pub fn position(&self) -> Microseconds {
        self.position
    }

    pub fn frame_rate(&self) -> &RationalFrameRate {
        &self.frame_rate
    }

    pub fn playback_rate(&self) -> PlaybackRate {
        self.playback_rate
    }

    pub fn state(&self) -> PlaybackState {
        self.state
    }

    pub fn generation(&self) -> PlaybackGeneration {
        self.generation
    }

    pub fn seek(&mut self, target_time: Microseconds) {
        self.position = target_time;
        self.state = PlaybackState::Paused;
        self.advance_generation();
    }

    pub fn start_scrub(&mut self) {
        self.state = PlaybackState::Scrubbing;
        self.advance_generation();
    }

    pub fn commit_scrub(&mut self, target_time: Microseconds) {
        self.position = target_time;
        self.state = PlaybackState::Paused;
        self.advance_generation();
    }

    pub fn play(&mut self) {
        self.state = PlaybackState::Playing;
        self.advance_generation();
    }

    pub fn record_playback_position(&mut self, target_time: Microseconds) {
        self.position = target_time;
    }

    pub fn pause(&mut self) {
        self.state = PlaybackState::Paused;
        self.advance_generation();
    }

    pub fn resume(&mut self) {
        self.state = PlaybackState::Playing;
        self.advance_generation();
    }

    pub fn stop(&mut self) {
        self.position = Microseconds::ZERO;
        self.state = PlaybackState::Stopped;
        self.advance_generation();
    }

    pub fn accepted_edit(&mut self) {
        self.advance_generation();
    }

    pub fn draft_reloaded(&mut self) {
        self.position = Microseconds::ZERO;
        self.state = PlaybackState::Stopped;
        self.advance_generation();
    }

    pub fn material_relinked(&mut self) {
        self.advance_generation();
    }

    pub fn surface_detached(&mut self) {
        self.state = PlaybackState::Paused;
        self.advance_generation();
    }

    pub fn reset_runtime(&mut self) {
        self.position = Microseconds::ZERO;
        self.state = PlaybackState::Stopped;
        self.advance_generation();
    }

    pub fn advance_generation(&mut self) -> PlaybackGeneration {
        self.generation = self.generation.next();
        self.generation
    }
}

/// Freshness contract shared by stale-sensitive scheduler jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineFreshness {
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<u64>,
}

impl TimelineFreshness {
    pub fn new(target_time: Microseconds, playback_generation: PlaybackGeneration) -> Self {
        Self {
            target_time,
            playback_generation,
            project_session_id: None,
            expected_revision: None,
        }
    }

    pub fn with_project_session(
        mut self,
        project_session_id: impl Into<String>,
        expected_revision: u64,
    ) -> Self {
        self.project_session_id = Some(project_session_id.into());
        self.expected_revision = Some(expected_revision);
        self
    }
}

#[cfg(test)]
mod tests {
    use draft_model::{Microseconds, RationalFrameRate};

    use super::{
        PlaybackGeneration, PlaybackRate, PlaybackState, TimelineClock, TimelineFreshness,
    };

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

        let round_tripped: TimelineClock =
            serde_json::from_value(json).expect("clock deserializes");
        assert_eq!(round_tripped.position(), Microseconds::new(12_345_678));
        assert_eq!(
            round_tripped.playback_rate(),
            PlaybackRate::new(3, 2).unwrap()
        );
    }

    #[test]
    fn target_timeline_freshness_matches_preview_request_field_names() {
        let freshness =
            TimelineFreshness::new(Microseconds::new(1_234_567), PlaybackGeneration::new(42))
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

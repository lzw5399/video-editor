use std::error::Error;
use std::fmt;

use draft_model::{Microseconds, RationalFrameRate};
use serde::{Deserialize, Serialize};

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

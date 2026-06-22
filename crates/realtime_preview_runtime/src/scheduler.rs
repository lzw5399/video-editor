use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};

use draft_model::{Draft, Microseconds, TextSegmentSource};
use render_graph::{OutputDimensions, RenderGraph};
use serde::{Deserialize, Serialize};

use crate::{
    PlaybackGeneration, PlaybackRate, RealtimePreviewGraphInput, RealtimePreviewGraphPrepareError,
    prepare_realtime_preview_graph,
};

pub const REALTIME_PLAYBACK_IDLE_POLL_INTERVAL: Duration = Duration::from_millis(4);
pub const REALTIME_PLAYBACK_MAX_IN_FLIGHT_SURFACE_PRESENTATIONS: usize = 4;
pub const REALTIME_PLAYBACK_SURFACE_PRESENT_BACKPRESSURE_TIMEOUT: Duration =
    Duration::from_millis(250);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePlaybackSchedulerConfig {
    pub preview_dimensions: OutputDimensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePlaybackSchedulerEvidence {
    pub source: RealtimePlaybackSchedulerEvidenceSource,
    pub digest: String,
    pub width: u32,
    pub height: u32,
    pub byte_count: usize,
    pub target_time_microseconds: u64,
    pub presented_frames: u32,
    pub submitted_draws: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_text_overlays: Vec<RealtimePlaybackTextOverlayEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RealtimePlaybackSchedulerEvidenceSource {
    RenderGraphGpuComposited,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePlaybackTextOverlayEvidence {
    pub source: TextSegmentSource,
    pub content: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimePlaybackSchedulerPresentation {
    pub width: u32,
    pub height: u32,
    pub byte_count: usize,
    pub presented_frames: u32,
    pub submitted_draws: u32,
    pub digest: String,
}

pub trait RealtimePlaybackSchedulerPresenter {
    fn present_render_graph(
        &mut self,
        graph: &RenderGraph,
        target_time: Microseconds,
        playback_generation: PlaybackGeneration,
    ) -> Result<RealtimePlaybackSchedulerPresentation, RealtimePlaybackSchedulerError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealtimePlaybackCadence {
    frame_duration_us: u64,
    playback_rate: PlaybackRate,
}

impl RealtimePlaybackCadence {
    pub fn new(
        frame_rate: &draft_model::RationalFrameRate,
        playback_rate: PlaybackRate,
    ) -> Result<Self, RealtimePlaybackCadenceError> {
        if frame_rate.numerator == 0 || frame_rate.denominator == 0 {
            return Err(RealtimePlaybackCadenceError::InvalidFrameRate);
        }
        if playback_rate.numerator <= 0 {
            return Err(RealtimePlaybackCadenceError::UnsupportedReversePlayback);
        }
        let frame_duration_us = u128::from(frame_rate.denominator)
            .checked_mul(1_000_000)
            .ok_or(RealtimePlaybackCadenceError::FrameDurationOverflow)?
            / u128::from(frame_rate.numerator);
        let frame_duration_us = u64::try_from(frame_duration_us)
            .map_err(|_| RealtimePlaybackCadenceError::FrameDurationOverflow)?;
        if frame_duration_us == 0 {
            return Err(RealtimePlaybackCadenceError::InvalidFrameRate);
        }
        Ok(Self {
            frame_duration_us,
            playback_rate,
        })
    }

    pub const fn frame_duration_us(self) -> u64 {
        self.frame_duration_us
    }

    fn media_elapsed_us(self, wall_elapsed_us: u64) -> u64 {
        let scaled = u128::from(wall_elapsed_us)
            .saturating_mul(self.playback_rate.numerator as u128)
            / u128::from(self.playback_rate.denominator);
        u64::try_from(scaled).unwrap_or(u64::MAX)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimePlaybackCadenceError {
    InvalidFrameRate,
    UnsupportedReversePlayback,
    FrameDurationOverflow,
}

impl fmt::Display for RealtimePlaybackCadenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFrameRate => formatter.write_str(
                "realtime playback frame rate numerator and denominator must be greater than zero",
            ),
            Self::UnsupportedReversePlayback => formatter.write_str(
                "realtime playback scheduler does not support reverse playback cadence yet",
            ),
            Self::FrameDurationOverflow => {
                formatter.write_str("realtime playback frame duration overflowed")
            }
        }
    }
}

impl Error for RealtimePlaybackCadenceError {}

#[derive(Debug, Clone)]
pub struct RealtimePlaybackTimeline {
    next_tick_time: Microseconds,
    anchor: Option<RealtimePlaybackAnchor>,
}

impl RealtimePlaybackTimeline {
    pub fn new() -> Self {
        Self {
            next_tick_time: Microseconds::ZERO,
            anchor: None,
        }
    }

    pub fn seek(&mut self, target_time: Microseconds) {
        self.next_tick_time = target_time;
        self.anchor = None;
    }

    pub fn reset(&mut self) {
        self.next_tick_time = Microseconds::ZERO;
        self.anchor = None;
    }

    pub fn pause(&mut self) {
        self.anchor = None;
    }

    pub fn stop(&mut self) {
        self.reset();
    }

    pub fn start_after_prewarm(
        &mut self,
        start_time: Microseconds,
        playback_generation: PlaybackGeneration,
        sequence_duration: Microseconds,
        cadence: RealtimePlaybackCadence,
    ) {
        self.start_after_prewarm_at(
            start_time,
            playback_generation,
            sequence_duration,
            cadence,
            Instant::now(),
        );
    }

    pub fn start_after_prewarm_at(
        &mut self,
        start_time: Microseconds,
        playback_generation: PlaybackGeneration,
        sequence_duration: Microseconds,
        cadence: RealtimePlaybackCadence,
        started_at: Instant,
    ) {
        self.anchor = Some(RealtimePlaybackAnchor {
            started_at,
            start_time,
            playback_generation,
            sequence_duration,
            cadence,
        });
        self.next_tick_time = Microseconds::new(
            start_time
                .get()
                .saturating_add(cadence.frame_duration_us())
                .min(sequence_duration.get()),
        );
    }

    pub fn due_tick(
        &self,
        playback_generation: PlaybackGeneration,
    ) -> Option<RealtimePlaybackDueTick> {
        self.due_tick_at(playback_generation, Instant::now())
    }

    pub fn advance_after_presented(&mut self, presented_time: Microseconds) {
        let Some(anchor) = self.anchor.as_ref() else {
            return;
        };
        self.next_tick_time = Microseconds::new(
            presented_time
                .get()
                .saturating_add(anchor.cadence.frame_duration_us())
                .min(anchor.sequence_duration.get()),
        );
    }

    #[cfg(test)]
    fn start_for_test(
        &mut self,
        start_time: Microseconds,
        playback_generation: PlaybackGeneration,
        sequence_duration: Microseconds,
        cadence: RealtimePlaybackCadence,
        started_at: Instant,
    ) {
        self.anchor = Some(RealtimePlaybackAnchor {
            started_at,
            start_time,
            playback_generation,
            sequence_duration,
            cadence,
        });
        self.next_tick_time = start_time;
    }

    fn due_tick_at(
        &self,
        playback_generation: PlaybackGeneration,
        now: Instant,
    ) -> Option<RealtimePlaybackDueTick> {
        let Some(anchor) = self.anchor.as_ref() else {
            return None;
        };
        if anchor.playback_generation != playback_generation {
            return None;
        }

        let elapsed_us =
            u64::try_from(now.duration_since(anchor.started_at).as_micros()).unwrap_or(u64::MAX);
        let wall_media_time = anchor
            .start_time
            .get()
            .saturating_add(anchor.cadence.media_elapsed_us(elapsed_us))
            .min(anchor.sequence_duration.get());
        if wall_media_time < self.next_tick_time.get()
            && self.next_tick_time.get() < anchor.sequence_duration.get()
        {
            return None;
        }

        let elapsed_frames = wall_media_time.saturating_sub(anchor.start_time.get())
            / anchor.cadence.frame_duration_us();
        let wall_aligned_target = anchor
            .start_time
            .get()
            .saturating_add(elapsed_frames.saturating_mul(anchor.cadence.frame_duration_us()))
            .min(anchor.sequence_duration.get());
        let target_time = self
            .next_tick_time
            .get()
            .max(wall_aligned_target)
            .min(anchor.sequence_duration.get());
        let dropped_frames = target_time.saturating_sub(self.next_tick_time.get())
            / anchor.cadence.frame_duration_us();
        let schedule_lateness_ms = wall_media_time.saturating_sub(target_time) / 1_000;
        Some(RealtimePlaybackDueTick {
            target_time: Microseconds::new(target_time),
            dropped_frames,
            schedule_lateness_ms,
        })
    }
}

impl Default for RealtimePlaybackTimeline {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct RealtimePlaybackAnchor {
    started_at: Instant,
    start_time: Microseconds,
    playback_generation: PlaybackGeneration,
    sequence_duration: Microseconds,
    cadence: RealtimePlaybackCadence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealtimePlaybackDueTick {
    pub target_time: Microseconds,
    pub dropped_frames: u64,
    pub schedule_lateness_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimePlaybackPresentedFrame {
    pub evidence: RealtimePlaybackSchedulerEvidence,
    pub dropped_frames: u64,
    pub schedule_lateness_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealtimePlaybackPresentationQueuePolicy {
    pub max_in_flight_presentations: usize,
    pub backpressure_timeout: Duration,
}

impl RealtimePlaybackPresentationQueuePolicy {
    pub const fn production() -> Self {
        Self {
            max_in_flight_presentations: REALTIME_PLAYBACK_MAX_IN_FLIGHT_SURFACE_PRESENTATIONS,
            backpressure_timeout: REALTIME_PLAYBACK_SURFACE_PRESENT_BACKPRESSURE_TIMEOUT,
        }
    }

    pub const fn has_capacity(self, in_flight_count: usize) -> bool {
        in_flight_count < self.max_in_flight_presentations
    }
}

impl Default for RealtimePlaybackPresentationQueuePolicy {
    fn default() -> Self {
        Self::production()
    }
}

#[derive(Debug, Clone)]
pub struct RealtimePlaybackScheduler {
    config: RealtimePlaybackSchedulerConfig,
    draft_snapshot: Option<Draft>,
    last_evidence: Option<RealtimePlaybackSchedulerEvidence>,
}

impl RealtimePlaybackScheduler {
    pub fn new(config: RealtimePlaybackSchedulerConfig) -> Self {
        Self {
            config,
            draft_snapshot: None,
            last_evidence: None,
        }
    }

    pub fn update_preview_dimensions(&mut self, dimensions: OutputDimensions) {
        self.config.preview_dimensions = dimensions;
        self.last_evidence = None;
    }

    pub fn update_draft_snapshot(&mut self, draft: Draft) {
        self.draft_snapshot = Some(draft);
        self.last_evidence = None;
    }

    pub fn last_evidence(&self) -> Option<&RealtimePlaybackSchedulerEvidence> {
        self.last_evidence.as_ref()
    }

    pub fn present_tick(
        &mut self,
        target_time: Microseconds,
        playback_generation: PlaybackGeneration,
        presenter: &mut impl RealtimePlaybackSchedulerPresenter,
    ) -> Result<RealtimePlaybackSchedulerEvidence, RealtimePlaybackSchedulerError> {
        let draft = self.draft_snapshot.clone().ok_or(
            RealtimePlaybackSchedulerError::MissingPrerequisite {
                reason: "accepted draft snapshot is required before scheduler playback".to_owned(),
            },
        )?;
        let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
            draft,
            target_time,
            preview_dimensions: self.config.preview_dimensions,
        })
        .map_err(RealtimePlaybackSchedulerError::GraphPrepare)?;
        let active_text_overlays =
            active_text_overlay_evidence(&prepared.graph, self.config.preview_dimensions);
        let presentation =
            presenter.present_render_graph(&prepared.graph, target_time, playback_generation)?;
        if presentation.presented_frames == 0 {
            return Err(RealtimePlaybackSchedulerError::MissingPrerequisite {
                reason: "render graph GPU compositor did not present a surface frame".to_owned(),
            });
        }
        let evidence = RealtimePlaybackSchedulerEvidence {
            source: RealtimePlaybackSchedulerEvidenceSource::RenderGraphGpuComposited,
            digest: presentation.digest,
            width: presentation.width,
            height: presentation.height,
            byte_count: presentation.byte_count,
            target_time_microseconds: target_time.get(),
            presented_frames: presentation.presented_frames,
            submitted_draws: presentation.submitted_draws,
            active_text_overlays,
        };
        self.last_evidence = Some(evidence.clone());
        Ok(evidence)
    }
}

fn active_text_overlay_evidence(
    graph: &RenderGraph,
    target: OutputDimensions,
) -> Vec<RealtimePlaybackTextOverlayEvidence> {
    graph
        .text_overlays
        .iter()
        .map(|text| {
            let rect = graph_canvas_rect_to_target(
                graph.canvas.width,
                graph.canvas.height,
                target.width,
                target.height,
                text.overlay.layout_region.x,
                text.overlay.layout_region.y,
                text.overlay.layout_width,
                text.overlay.layout_height,
            );
            RealtimePlaybackTextOverlayEvidence {
                source: text.overlay.source,
                content: text.overlay.content.clone(),
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct TargetRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn graph_canvas_rect_to_target(
    canvas_width: u32,
    canvas_height: u32,
    target_width: u32,
    target_height: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> TargetRect {
    let canvas_width = canvas_width.max(1);
    let canvas_height = canvas_height.max(1);
    let target_width = target_width.max(1);
    let target_height = target_height.max(1);
    let (fitted_width, fitted_height) =
        fit_canvas_to_target(canvas_width, canvas_height, target_width, target_height);
    let offset_x = (target_width.saturating_sub(fitted_width)) / 2;
    let offset_y = (target_height.saturating_sub(fitted_height)) / 2;
    TargetRect {
        x: offset_x.saturating_add(scale_canvas_distance(x, fitted_width, canvas_width)),
        y: offset_y.saturating_add(scale_canvas_distance(y, fitted_height, canvas_height)),
        width: scale_canvas_span(width, fitted_width, canvas_width).max(1),
        height: scale_canvas_span(height, fitted_height, canvas_height).max(1),
    }
}

fn fit_canvas_to_target(
    canvas_width: u32,
    canvas_height: u32,
    target_width: u32,
    target_height: u32,
) -> (u32, u32) {
    if u64::from(target_width) * u64::from(canvas_height)
        <= u64::from(target_height) * u64::from(canvas_width)
    {
        (
            target_width,
            scale_canvas_span(canvas_height, target_width, canvas_width),
        )
    } else {
        (
            scale_canvas_span(canvas_width, target_height, canvas_height),
            target_height,
        )
    }
}

fn scale_canvas_span(span: u32, target_span: u32, canvas_span: u32) -> u32 {
    scale_canvas_distance(span, target_span, canvas_span).max(1)
}

fn scale_canvas_distance(span: u32, target_span: u32, canvas_span: u32) -> u32 {
    ((u64::from(span) * u64::from(target_span) + u64::from(canvas_span.max(1)) / 2)
        / u64::from(canvas_span.max(1)))
    .min(u64::from(u32::MAX)) as u32
}

#[derive(Debug)]
pub enum RealtimePlaybackSchedulerError {
    MissingPrerequisite { reason: String },
    GraphPrepare(RealtimePreviewGraphPrepareError),
    Presentation { reason: String },
}

impl fmt::Display for RealtimePlaybackSchedulerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPrerequisite { reason } => formatter.write_str(reason),
            Self::GraphPrepare(error) => write!(formatter, "{error}"),
            Self::Presentation { reason } => formatter.write_str(reason),
        }
    }
}

impl Error for RealtimePlaybackSchedulerError {}

#[cfg(test)]
mod tests {
    use super::{
        RealtimePlaybackCadence, RealtimePlaybackPresentationQueuePolicy,
        RealtimePlaybackScheduler, RealtimePlaybackSchedulerConfig,
        RealtimePlaybackSchedulerPresentation, RealtimePlaybackSchedulerPresenter,
        RealtimePlaybackTimeline,
    };
    use draft_model::{
        Draft, DraftCanvasConfig, Material, MaterialKind, MaterialMetadata, Microseconds,
        RationalFrameRate, Segment, SourceTimerange, TargetTimerange, TextBox, TextLayoutRegion,
        TextSegment, TextSegmentSource, TextStyle, TextWrapping, Track, TrackKind,
    };
    use render_graph::{OutputDimensions, RenderGraph};
    use std::time::{Duration, Instant};

    use crate::{PlaybackGeneration, PlaybackRate};

    #[test]
    fn scheduler_builds_render_graph_before_presenting_surface() {
        let mut scheduler = RealtimePlaybackScheduler::new(RealtimePlaybackSchedulerConfig {
            preview_dimensions: OutputDimensions {
                width: 640,
                height: 360,
            },
        });
        scheduler.update_draft_snapshot(video_draft());
        let mut presenter = RecordingPresenter::default();

        let evidence = scheduler
            .present_tick(
                Microseconds::new(500_000),
                PlaybackGeneration::new(7),
                &mut presenter,
            )
            .expect("scheduler presents render graph frame");

        assert_eq!(presenter.present_count, 1);
        assert_eq!(presenter.last_layer_count, 1);
        assert_eq!(evidence.target_time_microseconds, 500_000);
        assert_eq!(evidence.presented_frames, 1);
        assert_eq!(evidence.digest, "presented-500000-7");
    }

    #[test]
    fn scheduler_fails_closed_without_accepted_draft() {
        let mut scheduler = RealtimePlaybackScheduler::new(RealtimePlaybackSchedulerConfig {
            preview_dimensions: OutputDimensions {
                width: 640,
                height: 360,
            },
        });
        let mut presenter = RecordingPresenter::default();

        let error = scheduler
            .present_tick(
                Microseconds::new(0),
                PlaybackGeneration::new(1),
                &mut presenter,
            )
            .expect_err("missing draft fails closed");

        assert!(error.to_string().contains("accepted draft snapshot"));
        assert_eq!(presenter.present_count, 0);
    }

    #[test]
    fn scheduler_text_overlay_evidence_uses_presentation_target_coordinates() {
        let mut scheduler = RealtimePlaybackScheduler::new(RealtimePlaybackSchedulerConfig {
            preview_dimensions: OutputDimensions {
                width: 1280,
                height: 720,
            },
        });
        scheduler.update_draft_snapshot(text_draft_with_low_resolution_canvas());
        let mut presenter = RecordingPresenter::default();

        let evidence = scheduler
            .present_tick(
                Microseconds::new(500_000),
                PlaybackGeneration::new(7),
                &mut presenter,
            )
            .expect("scheduler presents text render graph frame");
        let text = evidence
            .active_text_overlays
            .first()
            .expect("active text evidence is present");

        assert_eq!(text.source, TextSegmentSource::Subtitle);
        assert_eq!(text.content, "字幕位置证据");
        assert!(
            text.y >= 396,
            "subtitle evidence must be scaled into the lower presentation target: {text:?}"
        );
        assert!(
            text.width >= 1_000,
            "text width must be scaled from the 320px draft canvas into the 1280px target: {text:?}"
        );
    }

    #[test]
    fn playback_cadence_uses_rational_frame_rate_not_fixed_30fps() {
        let twenty_four =
            RealtimePlaybackCadence::new(&RationalFrameRate::new(24, 1), PlaybackRate::normal())
                .expect("24fps cadence is valid");
        let ntsc = RealtimePlaybackCadence::new(
            &RationalFrameRate::new(30_000, 1_001),
            PlaybackRate::normal(),
        )
        .expect("29.97fps cadence is valid");
        let thirty =
            RealtimePlaybackCadence::new(&RationalFrameRate::new(30, 1), PlaybackRate::normal())
                .expect("30fps cadence is valid");

        assert_eq!(twenty_four.frame_duration_us(), 41_666);
        assert_eq!(ntsc.frame_duration_us(), 33_366);
        assert_eq!(thirty.frame_duration_us(), 33_333);
    }

    #[test]
    fn playback_timeline_skips_late_frames_without_slowing_media_clock() {
        let cadence =
            RealtimePlaybackCadence::new(&RationalFrameRate::new(30, 1), PlaybackRate::normal())
                .expect("30fps cadence is valid");
        let generation = PlaybackGeneration::new(7);
        let start_time = Microseconds::new(100_000);
        let mut timeline = RealtimePlaybackTimeline::new();
        timeline.start_for_test(
            start_time,
            generation,
            Microseconds::new(2_000_000),
            cadence,
            Instant::now()
                .checked_sub(Duration::from_millis(500))
                .expect("500ms before now is representable"),
        );

        let due_tick = timeline
            .due_tick(generation)
            .expect("late playback should immediately present the wall-clock frame");
        let expected_target_time = start_time
            .get()
            .saturating_add(15 * cadence.frame_duration_us());

        assert_eq!(due_tick.target_time.get(), expected_target_time);
        assert_eq!(due_tick.dropped_frames, 15);
    }

    #[test]
    fn playback_timeline_keeps_media_clock_running_during_prewarm() {
        let cadence =
            RealtimePlaybackCadence::new(&RationalFrameRate::new(30, 1), PlaybackRate::normal())
                .expect("30fps cadence is valid");
        let generation = PlaybackGeneration::new(7);
        let mut timeline = RealtimePlaybackTimeline::new();
        timeline.start_after_prewarm_at(
            Microseconds::ZERO,
            generation,
            Microseconds::new(2_000_000),
            cadence,
            Instant::now()
                .checked_sub(Duration::from_millis(120))
                .expect("120ms before now is representable"),
        );

        let due_tick = timeline
            .due_tick(generation)
            .expect("prewarm elapsed time should be reflected in the media clock");

        assert_eq!(due_tick.target_time.get(), 3 * cadence.frame_duration_us());
        assert_eq!(due_tick.dropped_frames, 2);
    }

    #[test]
    fn playback_timeline_respects_non_unit_playback_rate() {
        let cadence = RealtimePlaybackCadence::new(
            &RationalFrameRate::new(30, 1),
            PlaybackRate::new(2, 1).expect("2x playback rate is valid"),
        )
        .expect("2x cadence is valid");
        let generation = PlaybackGeneration::new(11);
        let start_time = Microseconds::ZERO;
        let mut timeline = RealtimePlaybackTimeline::new();
        timeline.start_for_test(
            start_time,
            generation,
            Microseconds::new(2_000_000),
            cadence,
            Instant::now()
                .checked_sub(Duration::from_millis(500))
                .expect("500ms before now is representable"),
        );

        let due_tick = timeline
            .due_tick(generation)
            .expect("2x playback should choose the media frame for 1s elapsed media time");

        assert_eq!(due_tick.target_time.get(), 30 * cadence.frame_duration_us());
        assert_eq!(due_tick.dropped_frames, 30);
    }

    #[test]
    fn playback_timeline_rejects_stale_generation_ticks() {
        let cadence =
            RealtimePlaybackCadence::new(&RationalFrameRate::new(24, 1), PlaybackRate::normal())
                .expect("cadence is valid");
        let mut timeline = RealtimePlaybackTimeline::new();
        timeline.start_for_test(
            Microseconds::ZERO,
            PlaybackGeneration::new(3),
            Microseconds::new(1_000_000),
            cadence,
            Instant::now()
                .checked_sub(Duration::from_millis(100))
                .expect("100ms before now is representable"),
        );

        assert_eq!(timeline.due_tick(PlaybackGeneration::new(4)), None);
    }

    #[test]
    fn playback_queue_policy_is_runtime_owned() {
        let policy = RealtimePlaybackPresentationQueuePolicy::production();

        assert_eq!(policy.max_in_flight_presentations, 4);
        assert!(policy.has_capacity(3));
        assert!(!policy.has_capacity(4));
        assert_eq!(policy.backpressure_timeout, Duration::from_millis(250));
    }

    #[derive(Default)]
    struct RecordingPresenter {
        present_count: u32,
        last_layer_count: usize,
    }

    impl RealtimePlaybackSchedulerPresenter for RecordingPresenter {
        fn present_render_graph(
            &mut self,
            graph: &RenderGraph,
            target_time: Microseconds,
            playback_generation: PlaybackGeneration,
        ) -> Result<RealtimePlaybackSchedulerPresentation, super::RealtimePlaybackSchedulerError>
        {
            self.present_count = self.present_count.saturating_add(1);
            self.last_layer_count = graph.video_layers.len();
            Ok(RealtimePlaybackSchedulerPresentation {
                width: 640,
                height: 360,
                byte_count: 0,
                presented_frames: 1,
                submitted_draws: graph.video_layers.len() as u32,
                digest: format!(
                    "presented-{}-{}",
                    target_time.get(),
                    playback_generation.get()
                ),
            })
        }
    }

    fn video_draft() -> Draft {
        let mut draft = Draft::new("draft-scheduler-runtime-001", "Scheduler runtime");
        let mut material = Material::new(
            "material-video-001",
            MaterialKind::Video,
            "file:///repo-owned-fixture/p0-moving-testsrc.mp4",
            "p0-moving-testsrc.mp4",
        );
        material.metadata = MaterialMetadata {
            duration: Some(Microseconds::new(2_000_000)),
            width: Some(640),
            height: Some(360),
            frame_rate: Some(RationalFrameRate::new(30, 1)),
            has_video: true,
            has_audio: false,
            audio_sample_rate: None,
            audio_channels: None,
            probe_error: None,
        };
        draft.materials.push(material);
        let segment = Segment::new(
            "segment-video-001",
            "material-video-001",
            SourceTimerange::new(0, 2_000_000),
            TargetTimerange::new(0, 2_000_000),
        );
        let mut track = Track::new("track-video-001", TrackKind::Video, "视频");
        track.segments.push(segment);
        draft.tracks.push(track);
        draft
    }

    fn text_draft_with_low_resolution_canvas() -> Draft {
        let mut draft = Draft::new("draft-scheduler-text-001", "Scheduler text");
        draft.canvas_config = DraftCanvasConfig {
            width: 320,
            height: 180,
            ..DraftCanvasConfig::mvp_default()
        };
        let material = Material::new(
            "material-text-001",
            MaterialKind::Text,
            "text://subtitle",
            "字幕位置证据",
        );
        draft.materials.push(material);
        let mut segment = Segment::new(
            "segment-text-001",
            "material-text-001",
            SourceTimerange::new(0, 2_000_000),
            TargetTimerange::new(0, 2_000_000),
        );
        segment.text = Some(TextSegment {
            content: "字幕位置证据".to_owned(),
            source: TextSegmentSource::Subtitle,
            style: TextStyle::default(),
            text_box: TextBox {
                width_millis: 800,
                height_millis: 180,
            },
            layout_region: TextLayoutRegion {
                x_millis: 100,
                y_millis: 720,
                width_millis: 800,
                height_millis: 180,
            },
            wrapping: TextWrapping::default(),
            bubble: None,
            effect: None,
        });
        let mut track = Track::new("track-text-001", TrackKind::Text, "字幕");
        track.segments.push(segment);
        draft.tracks.push(track);
        draft
    }
}

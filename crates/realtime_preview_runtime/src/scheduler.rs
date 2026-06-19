use std::error::Error;
use std::fmt;

use draft_model::{Draft, Microseconds};
use render_graph::{OutputDimensions, RenderGraph};
use serde::{Deserialize, Serialize};

use crate::{
    PlaybackGeneration, RealtimePreviewGraphInput, RealtimePreviewGraphPrepareError,
    prepare_realtime_preview_graph,
};

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RealtimePlaybackSchedulerEvidenceSource {
    RenderGraphGpuComposited,
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
        };
        self.last_evidence = Some(evidence.clone());
        Ok(evidence)
    }
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
        RealtimePlaybackScheduler, RealtimePlaybackSchedulerConfig,
        RealtimePlaybackSchedulerPresentation, RealtimePlaybackSchedulerPresenter,
    };
    use draft_model::{
        Draft, Material, MaterialKind, MaterialMetadata, Microseconds, RationalFrameRate, Segment,
        SourceTimerange, TargetTimerange, Track, TrackKind,
    };
    use render_graph::{OutputDimensions, RenderGraph};

    use crate::PlaybackGeneration;

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
}

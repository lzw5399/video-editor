use std::fmt;

use draft_model::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasBackground, Draft, DraftCanvasConfig,
    Material, MaterialId, MaterialKind, Microseconds, RationalFrameRate, Segment, SourceTimerange,
    TargetTimerange, TextSegment, TextSegmentSource, TextStyle, TextWrapping, Track, TrackId,
    TrackKind, validate_draft,
};

pub const MAX_SEGMENTS_PER_TRACK: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LargeTimelineConfig {
    pub segments_per_track: usize,
    pub include_video: bool,
    pub include_audio: bool,
    pub include_text: bool,
    pub segment_duration: Microseconds,
    pub target_stride: Microseconds,
    pub localized_edit_index: usize,
    pub canvas_config: DraftCanvasConfig,
}

impl LargeTimelineConfig {
    pub fn new(segments_per_track: usize) -> Self {
        Self {
            segments_per_track,
            ..Self::default()
        }
    }

    pub fn with_track_mix(
        mut self,
        include_video: bool,
        include_audio: bool,
        include_text: bool,
    ) -> Self {
        self.include_video = include_video;
        self.include_audio = include_audio;
        self.include_text = include_text;
        self
    }

    pub fn with_localized_edit_index(mut self, index: usize) -> Self {
        self.localized_edit_index = index;
        self
    }

    pub fn with_segment_duration(mut self, duration: Microseconds) -> Self {
        self.segment_duration = duration;
        self.target_stride = duration;
        self
    }

    pub fn with_target_stride(mut self, stride: Microseconds) -> Self {
        self.target_stride = stride;
        self
    }

    pub fn with_canvas_config(mut self, canvas_config: DraftCanvasConfig) -> Self {
        self.canvas_config = canvas_config;
        self
    }

    pub fn track_count(&self) -> usize {
        usize::from(self.include_video)
            + usize::from(self.include_audio)
            + usize::from(self.include_text)
    }

    pub fn total_segment_count(&self) -> usize {
        self.segments_per_track * self.track_count()
    }
}

impl Default for LargeTimelineConfig {
    fn default() -> Self {
        Self {
            segments_per_track: 300,
            include_video: true,
            include_audio: true,
            include_text: true,
            segment_duration: Microseconds::new(100_000),
            target_stride: Microseconds::new(100_000),
            localized_edit_index: 150,
            canvas_config: DraftCanvasConfig {
                aspect_ratio: CanvasAspectRatio::custom(16, 9),
                width: 1920,
                height: 1080,
                frame_rate: RationalFrameRate::new(30, 1),
                background: CanvasBackground::Black,
                adaptation_policy: CanvasAdaptationPolicy::Auto,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LargeTimelineDraft {
    pub draft: Draft,
    pub localized_edit: LocalizedEditTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedEditTarget {
    pub track_id: TrackId,
    pub segment_id: draft_model::SegmentId,
    pub material_id: MaterialId,
    pub track_kind: TrackKind,
    pub segment_index: usize,
    pub target_timerange: TargetTimerange,
    pub source_timerange: SourceTimerange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LargeTimelineError {
    message: String,
}

impl LargeTimelineError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for LargeTimelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for LargeTimelineError {}

pub fn build_large_timeline(
    config: LargeTimelineConfig,
) -> Result<LargeTimelineDraft, LargeTimelineError> {
    validate_config(&config)?;

    let mut draft = Draft::new("phase13-large-timeline-draft", "Phase 13 Large Timeline");
    draft.canvas_config = config.canvas_config.clone();

    if config.include_video {
        push_track_with_segments(&mut draft, &config, TrackKind::Video)?;
    }
    if config.include_audio {
        push_track_with_segments(&mut draft, &config, TrackKind::Audio)?;
    }
    if config.include_text {
        push_track_with_segments(&mut draft, &config, TrackKind::Text)?;
    }

    validate_draft(&draft).map_err(|error| LargeTimelineError::new(error.to_string()))?;
    let localized_edit = localized_edit_target(&draft, &config)?;

    Ok(LargeTimelineDraft {
        draft,
        localized_edit,
    })
}

pub fn assert_no_track_overlaps(draft: &Draft) -> Result<(), LargeTimelineError> {
    for track in &draft.tracks {
        let mut ranges = track
            .segments
            .iter()
            .map(|segment| &segment.target_timerange)
            .collect::<Vec<_>>();
        ranges.sort_by_key(|range| range.start.get());

        for pair in ranges.windows(2) {
            let previous = pair[0];
            let current = pair[1];
            let previous_end = checked_end(previous)?;
            if previous_end > current.start.get() {
                return Err(LargeTimelineError::new(format!(
                    "track {} has overlapping segments at {} and {}",
                    track.track_id.as_str(),
                    previous.start.get(),
                    current.start.get()
                )));
            }
        }
    }

    Ok(())
}

fn validate_config(config: &LargeTimelineConfig) -> Result<(), LargeTimelineError> {
    if config.segments_per_track == 0 {
        return Err(LargeTimelineError::new(
            "segments_per_track must be greater than zero",
        ));
    }
    if config.segments_per_track > MAX_SEGMENTS_PER_TRACK {
        return Err(LargeTimelineError::new(format!(
            "segments_per_track must be <= {MAX_SEGMENTS_PER_TRACK}"
        )));
    }
    if config.track_count() == 0 {
        return Err(LargeTimelineError::new(
            "at least one video, audio, or text track must be enabled",
        ));
    }
    if config.segment_duration.get() == 0 {
        return Err(LargeTimelineError::new(
            "segment_duration must be greater than zero microseconds",
        ));
    }
    if config.target_stride.get() < config.segment_duration.get() {
        return Err(LargeTimelineError::new(
            "target_stride must be greater than or equal to segment_duration",
        ));
    }
    let last_index = config.segments_per_track - 1;
    checked_target_range(last_index, config.segment_duration, config.target_stride)?;
    Ok(())
}

fn push_track_with_segments(
    draft: &mut Draft,
    config: &LargeTimelineConfig,
    kind: TrackKind,
) -> Result<(), LargeTimelineError> {
    let prefix = track_prefix(kind);
    let mut track = Track::new(
        format!("{prefix}-track-000"),
        kind,
        format!("{prefix} track 000"),
    );

    for index in 0..config.segments_per_track {
        let material_id = format!("{prefix}-material-{index:06}");
        draft
            .materials
            .push(material(&material_id, kind, index, config.segment_duration));

        let mut segment = Segment::new(
            format!("{prefix}-segment-{index:06}"),
            material_id,
            SourceTimerange::new(Microseconds::ZERO, config.segment_duration),
            checked_target_range(index, config.segment_duration, config.target_stride)?,
        );
        if kind == TrackKind::Text {
            segment.text = Some(text_segment(index));
        }
        track.segments.push(segment);
    }

    draft.tracks.push(track);
    Ok(())
}

fn material(id: &str, track_kind: TrackKind, index: usize, duration: Microseconds) -> Material {
    let material_kind = match track_kind {
        TrackKind::Video => MaterialKind::Video,
        TrackKind::Audio => MaterialKind::Audio,
        TrackKind::Text => MaterialKind::Text,
        TrackKind::Sticker | TrackKind::Filter => MaterialKind::Sticker,
    };
    let mut material = Material::new(
        id,
        material_kind,
        format!(
            "{}://phase13/{index:06}",
            material_uri_scheme(material_kind)
        ),
        format!("{} material {index:06}", track_prefix(track_kind)),
    );
    material.metadata.duration = Some(duration);
    match material_kind {
        MaterialKind::Video => {
            material.metadata.width = Some(1920);
            material.metadata.height = Some(1080);
            material.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
            material.metadata.has_video = true;
            material.metadata.has_audio = true;
            material.metadata.audio_sample_rate = Some(48_000);
            material.metadata.audio_channels = Some(2);
        }
        MaterialKind::Audio => {
            material.metadata.has_audio = true;
            material.metadata.audio_sample_rate = Some(48_000);
            material.metadata.audio_channels = Some(2);
        }
        MaterialKind::Text | MaterialKind::Image | MaterialKind::Sticker => {}
    }
    material
}

fn text_segment(index: usize) -> TextSegment {
    TextSegment {
        content: format!("大型时间线文字 {index:06}"),
        source: TextSegmentSource::Text,
        style: TextStyle::default(),
        text_box: Default::default(),
        layout_region: Default::default(),
        wrapping: TextWrapping::Auto,
        bubble: None,
        effect: None,
    }
}

fn localized_edit_target(
    draft: &Draft,
    config: &LargeTimelineConfig,
) -> Result<LocalizedEditTarget, LargeTimelineError> {
    let segment_index = config
        .localized_edit_index
        .min(config.segments_per_track.saturating_sub(1));
    let track = draft
        .tracks
        .iter()
        .find(|track| track.kind == TrackKind::Video)
        .or_else(|| {
            draft
                .tracks
                .iter()
                .find(|track| track.kind == TrackKind::Audio)
        })
        .or_else(|| {
            draft
                .tracks
                .iter()
                .find(|track| track.kind == TrackKind::Text)
        })
        .ok_or_else(|| LargeTimelineError::new("large timeline has no editable track"))?;
    let segment = track
        .segments
        .get(segment_index)
        .ok_or_else(|| LargeTimelineError::new("localized edit segment is missing"))?;

    Ok(LocalizedEditTarget {
        track_id: track.track_id.clone(),
        segment_id: segment.segment_id.clone(),
        material_id: segment.material_id.clone(),
        track_kind: track.kind,
        segment_index,
        target_timerange: segment.target_timerange.clone(),
        source_timerange: segment.source_timerange.clone(),
    })
}

fn checked_target_range(
    index: usize,
    duration: Microseconds,
    stride: Microseconds,
) -> Result<TargetTimerange, LargeTimelineError> {
    let index = u64::try_from(index)
        .map_err(|_| LargeTimelineError::new("segment index exceeds u64 range"))?;
    let start = index
        .checked_mul(stride.get())
        .ok_or_else(|| LargeTimelineError::new("large timeline target start overflowed"))?;
    Ok(TargetTimerange::new(Microseconds::new(start), duration))
}

fn checked_end(range: &TargetTimerange) -> Result<u64, LargeTimelineError> {
    range
        .start
        .get()
        .checked_add(range.duration.get())
        .ok_or_else(|| LargeTimelineError::new("target range end overflowed"))
}

fn material_uri_scheme(kind: MaterialKind) -> &'static str {
    match kind {
        MaterialKind::Video => "video",
        MaterialKind::Image => "image",
        MaterialKind::Audio => "audio",
        MaterialKind::Text => "text",
        MaterialKind::Sticker => "sticker",
    }
}

fn track_prefix(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Video => "video",
        TrackKind::Audio => "audio",
        TrackKind::Text => "text",
        TrackKind::Sticker => "sticker",
        TrackKind::Filter => "filter",
    }
}

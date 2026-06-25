use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use draft_model::{
    CanvasBackground, Draft, DraftId, DraftValidationError, Filter, Keyframe, Material, MaterialId,
    MaterialKind, MaterialStatus, Microseconds, RationalFrameRate, Segment, SegmentAudio,
    SegmentBackgroundFilling, SegmentBlendMode, SegmentId, SegmentMask, SegmentVisual,
    SourceTimerange, TargetTimerange, TextSegment, TrackId, TrackKind, Transition, validate_draft,
};
use serde::{Deserialize, Serialize};

use crate::{TextLayoutProfile, TextSafeArea};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineProfile {
    pub frame_rate: RationalFrameRate,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub canvas_background: CanvasBackground,
    pub text_layout: Option<TextLayoutProfile>,
}

impl EngineProfile {
    /// Intentional MVP fallback for tests and fixtures. Production preview/export
    /// callers should resolve profiles from `Draft.canvas_config`.
    pub fn mvp_default() -> Self {
        Self {
            frame_rate: RationalFrameRate::new(30, 1),
            canvas_width: 1920,
            canvas_height: 1080,
            canvas_background: CanvasBackground::Black,
            text_layout: Some(TextLayoutProfile::mvp_default()),
        }
    }

    pub fn from_draft_canvas(draft: &Draft) -> Result<Self, EngineError> {
        validate_draft(draft)?;
        let canvas = &draft.canvas_config;
        let profile = Self {
            frame_rate: canvas.frame_rate.clone(),
            canvas_width: canvas.width,
            canvas_height: canvas.height,
            canvas_background: canvas.background.clone(),
            text_layout: Some(text_layout_for_canvas(canvas.width, canvas.height)),
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<(), EngineError> {
        if self.frame_rate.numerator == 0 || self.frame_rate.denominator == 0 {
            return Err(EngineError::new(
                EngineErrorKind::InvalidFrameRate,
                "engine profile frameRate numerator and denominator must be greater than zero",
            ));
        }
        if self.canvas_width == 0 || self.canvas_height == 0 {
            return Err(EngineError::new(
                EngineErrorKind::InvalidEngineProfile,
                "engine profile canvas dimensions must be greater than zero",
            ));
        }
        if let Some(text_layout) = &self.text_layout {
            text_layout.validate(self.canvas_width, self.canvas_height)?;
        }
        Ok(())
    }
}

fn text_layout_for_canvas(canvas_width: u32, canvas_height: u32) -> TextLayoutProfile {
    let mut profile = TextLayoutProfile::mvp_default();
    profile.safe_area = TextSafeArea {
        left: canvas_width / 20,
        right: canvas_width / 20,
        top: canvas_height / 20,
        bottom: canvas_height / 20,
    };
    profile
}

impl Default for EngineProfile {
    fn default() -> Self {
        Self::mvp_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NormalizedDraft {
    pub draft_id: DraftId,
    pub profile: EngineProfile,
    pub duration: Microseconds,
    pub tracks: Vec<NormalizedTrack>,
    pub diagnostics: Vec<EngineDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NormalizedTrack {
    pub track_id: TrackId,
    pub kind: TrackKind,
    pub name: String,
    pub muted: bool,
    pub stack_index: Option<u32>,
    pub segments: Vec<NormalizedSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NormalizedSegment {
    pub segment_id: SegmentId,
    pub material: NormalizedMaterialRef,
    pub source_timerange: SourceTimerange,
    pub source_end: Microseconds,
    pub target_timerange: TargetTimerange,
    pub target_end: Microseconds,
    pub retiming: draft_model::SegmentRetiming,
    pub renderable: MaterialRenderableState,
    pub keyframes: Vec<Keyframe>,
    pub filters: Vec<Filter>,
    pub transition: Option<Transition>,
    pub text: Option<TextSegment>,
    pub audio: SegmentAudio,
    pub volume_level_millis: u32,
    pub visual: SegmentVisual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NormalizedMaterialRef {
    pub material_id: MaterialId,
    pub kind: MaterialKind,
    pub uri: String,
    pub display_name: String,
    pub status: MaterialStatus,
    pub duration: Option<Microseconds>,
    pub frame_rate: Option<RationalFrameRate>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub has_video: bool,
    pub has_audio: bool,
    pub audio_sample_rate: Option<u32>,
    pub audio_channels: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MaterialRenderableState {
    Renderable,
    MutedTrack,
    UnavailableMaterial,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineDiagnostic {
    pub kind: EngineErrorKind,
    pub track_id: Option<TrackId>,
    pub segment_id: Option<SegmentId>,
    pub material_id: Option<MaterialId>,
    pub message: String,
}

impl EngineDiagnostic {
    fn new(
        kind: EngineErrorKind,
        track_id: Option<TrackId>,
        segment_id: Option<SegmentId>,
        material_id: Option<MaterialId>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            track_id,
            segment_id,
            material_id,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineError {
    pub kind: EngineErrorKind,
    pub track_id: Option<TrackId>,
    pub segment_id: Option<SegmentId>,
    pub material_id: Option<MaterialId>,
    pub message: String,
}

impl EngineError {
    pub fn new(kind: EngineErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            track_id: None,
            segment_id: None,
            material_id: None,
            message: message.into(),
        }
    }

    pub(crate) fn with_track_id(mut self, track_id: TrackId) -> Self {
        self.track_id = Some(track_id);
        self
    }

    pub(crate) fn with_segment_id(mut self, segment_id: SegmentId) -> Self {
        self.segment_id = Some(segment_id);
        self
    }

    pub(crate) fn with_material_id(mut self, material_id: MaterialId) -> Self {
        self.material_id = Some(material_id);
        self
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for EngineError {}

impl From<DraftValidationError> for EngineError {
    fn from(error: DraftValidationError) -> Self {
        Self::new(EngineErrorKind::DraftValidationFailed, error.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EngineErrorKind {
    DraftValidationFailed,
    InvalidEngineProfile,
    InvalidFrameRate,
    MissingMaterial,
    TimerangeOverflow,
    SourceRangeExceedsMaterialDuration,
    MutedTrack,
    UnavailableMaterial,
    MissingTextLayoutProfile,
    InvalidTextLayoutProfile,
    DegradedVisualIntent,
    UnsupportedVisualIntent,
}

pub fn normalize_draft(
    draft: &Draft,
    profile: &EngineProfile,
) -> Result<NormalizedDraft, EngineError> {
    profile.validate()?;
    validate_draft(draft)?;

    let materials = draft
        .materials
        .iter()
        .map(|material| (material.material_id.clone(), material))
        .collect::<BTreeMap<_, _>>();

    let mut diagnostics = Vec::new();
    let mut duration = Microseconds::ZERO;
    let mut visual_stack_index = 0_u32;
    let mut tracks = Vec::with_capacity(draft.tracks.len());

    for track in &draft.tracks {
        if is_visual_track(track.kind) && !track.visible {
            continue;
        }
        let stack_index = if is_visual_track(track.kind) {
            let index = visual_stack_index;
            visual_stack_index = visual_stack_index.checked_add(1).ok_or_else(|| {
                EngineError::new(
                    EngineErrorKind::TimerangeOverflow,
                    "visual track stack index overflowed",
                )
            })?;
            Some(index)
        } else {
            None
        };

        let mut segments = track
            .segments
            .iter()
            .map(|segment| {
                normalize_segment(
                    track.track_id.clone(),
                    track.kind,
                    track.muted,
                    segment,
                    &materials,
                    &mut diagnostics,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        segments.sort_by(|first, second| {
            first
                .target_timerange
                .start
                .cmp(&second.target_timerange.start)
                .then_with(|| first.segment_id.cmp(&second.segment_id))
        });

        for segment in &segments {
            duration = max_microseconds(duration, segment.target_end);
        }

        tracks.push(NormalizedTrack {
            track_id: track.track_id.clone(),
            kind: track.kind,
            name: track.name.clone(),
            muted: track.muted,
            stack_index,
            segments,
        });
    }

    Ok(NormalizedDraft {
        draft_id: draft.draft_id.clone(),
        profile: profile.clone(),
        duration,
        tracks,
        diagnostics,
    })
}

fn normalize_segment(
    track_id: TrackId,
    track_kind: TrackKind,
    track_muted: bool,
    segment: &Segment,
    materials: &BTreeMap<MaterialId, &Material>,
    diagnostics: &mut Vec<EngineDiagnostic>,
) -> Result<NormalizedSegment, EngineError> {
    let material = materials.get(&segment.material_id).ok_or_else(|| {
        EngineError::new(
            EngineErrorKind::MissingMaterial,
            format!(
                "segment {} references missing material {}",
                segment.segment_id.as_str(),
                segment.material_id.as_str()
            ),
        )
        .with_track_id(track_id.clone())
        .with_segment_id(segment.segment_id.clone())
        .with_material_id(segment.material_id.clone())
    })?;

    let source_end = checked_timerange_end(
        "sourceTimerange",
        segment.source_timerange.start,
        segment.source_timerange.duration,
        &track_id,
        segment,
    )?;
    let target_end = checked_timerange_end(
        "targetTimerange",
        segment.target_timerange.start,
        segment.target_timerange.duration,
        &track_id,
        segment,
    )?;

    if let Some(material_duration) = material.metadata.duration {
        if source_end.get() > material_duration.get() {
            return Err(EngineError::new(
                EngineErrorKind::SourceRangeExceedsMaterialDuration,
                format!(
                    "segment {} sourceTimerange ends at {} but material {} duration is {}",
                    segment.segment_id.as_str(),
                    source_end.get(),
                    material.material_id.as_str(),
                    material_duration.get()
                ),
            )
            .with_track_id(track_id.clone())
            .with_segment_id(segment.segment_id.clone())
            .with_material_id(material.material_id.clone()));
        }
    }

    let renderable = renderable_state(track_muted, material.status);
    if renderable == MaterialRenderableState::MutedTrack {
        diagnostics.push(EngineDiagnostic::new(
            EngineErrorKind::MutedTrack,
            Some(track_id.clone()),
            Some(segment.segment_id.clone()),
            Some(material.material_id.clone()),
            format!(
                "track {} is muted; segment {} is non-renderable",
                track_id.as_str(),
                segment.segment_id.as_str()
            ),
        ));
    }
    if renderable == MaterialRenderableState::UnavailableMaterial {
        diagnostics.push(EngineDiagnostic::new(
            EngineErrorKind::UnavailableMaterial,
            Some(track_id.clone()),
            Some(segment.segment_id.clone()),
            Some(material.material_id.clone()),
            format!(
                "material {} is {:?}; segment {} is non-renderable",
                material.material_id.as_str(),
                material.status,
                segment.segment_id.as_str()
            ),
        ));
    }
    if is_visual_track(track_kind) {
        collect_visual_diagnostics(
            &track_id,
            &segment.segment_id,
            &material.material_id,
            &segment.visual,
            diagnostics,
        );
    }

    Ok(NormalizedSegment {
        segment_id: segment.segment_id.clone(),
        material: normalized_material_ref(material),
        source_timerange: segment.source_timerange.clone(),
        source_end,
        target_timerange: segment.target_timerange.clone(),
        target_end,
        retiming: segment.retiming.clone(),
        renderable,
        keyframes: segment.keyframes.clone(),
        filters: segment.filters.clone(),
        transition: segment.transition.clone(),
        text: segment.text.clone(),
        audio: segment.audio.clone(),
        volume_level_millis: segment.volume.level_millis,
        visual: segment.visual.clone(),
    })
}

fn collect_visual_diagnostics(
    track_id: &TrackId,
    segment_id: &SegmentId,
    material_id: &MaterialId,
    visual: &SegmentVisual,
    diagnostics: &mut Vec<EngineDiagnostic>,
) {
    match &visual.background_filling {
        SegmentBackgroundFilling::None
        | SegmentBackgroundFilling::Black
        | SegmentBackgroundFilling::SolidColor { .. } => {}
        SegmentBackgroundFilling::Blur => diagnostics.push(EngineDiagnostic::new(
            EngineErrorKind::DegradedVisualIntent,
            Some(track_id.clone()),
            Some(segment_id.clone()),
            Some(material_id.clone()),
            "segment backgroundFilling blur is preserved as degraded until render support is implemented",
        )),
        SegmentBackgroundFilling::Image { .. } => diagnostics.push(EngineDiagnostic::new(
            EngineErrorKind::UnsupportedVisualIntent,
            Some(track_id.clone()),
            Some(segment_id.clone()),
            Some(material_id.clone()),
            "segment backgroundFilling image is unsupported until segment background material rendering is implemented",
        )),
    }

    if let SegmentBlendMode::ExternalReference { reference } = &visual.blend_mode {
        diagnostics.push(EngineDiagnostic::new(
            EngineErrorKind::UnsupportedVisualIntent,
            Some(track_id.clone()),
            Some(segment_id.clone()),
            Some(material_id.clone()),
            format!(
                "segment blendMode external reference {}:{} is unsupported",
                reference.provider, reference.effect_id
            ),
        ));
    }
    if let SegmentMask::ExternalReference { reference } = &visual.mask {
        diagnostics.push(EngineDiagnostic::new(
            EngineErrorKind::UnsupportedVisualIntent,
            Some(track_id.clone()),
            Some(segment_id.clone()),
            Some(material_id.clone()),
            format!(
                "segment mask external reference {}:{} is unsupported",
                reference.provider, reference.effect_id
            ),
        ));
    }
}

fn checked_timerange_end(
    field: &str,
    start: Microseconds,
    duration: Microseconds,
    track_id: &TrackId,
    segment: &Segment,
) -> Result<Microseconds, EngineError> {
    start
        .get()
        .checked_add(duration.get())
        .map(Microseconds::new)
        .ok_or_else(|| {
            EngineError::new(
                EngineErrorKind::TimerangeOverflow,
                format!(
                    "segment {} {field} start + duration overflowed u64 microseconds",
                    segment.segment_id.as_str()
                ),
            )
            .with_track_id(track_id.clone())
            .with_segment_id(segment.segment_id.clone())
            .with_material_id(segment.material_id.clone())
        })
}

fn normalized_material_ref(material: &Material) -> NormalizedMaterialRef {
    NormalizedMaterialRef {
        material_id: material.material_id.clone(),
        kind: material.kind,
        uri: material.uri.clone(),
        display_name: material.display_name.clone(),
        status: material.status,
        duration: material.metadata.duration,
        frame_rate: material.metadata.frame_rate.clone(),
        width: material.metadata.width,
        height: material.metadata.height,
        has_video: material.metadata.has_video,
        has_audio: material.metadata.has_audio,
        audio_sample_rate: material.metadata.audio_sample_rate,
        audio_channels: material.metadata.audio_channels,
    }
}

fn renderable_state(track_muted: bool, material_status: MaterialStatus) -> MaterialRenderableState {
    if track_muted {
        return MaterialRenderableState::MutedTrack;
    }
    if material_status != MaterialStatus::Available {
        return MaterialRenderableState::UnavailableMaterial;
    }
    MaterialRenderableState::Renderable
}

fn is_visual_track(kind: TrackKind) -> bool {
    matches!(
        kind,
        TrackKind::Video | TrackKind::Text | TrackKind::Sticker
    )
}

fn max_microseconds(first: Microseconds, second: Microseconds) -> Microseconds {
    if first >= second { first } else { second }
}

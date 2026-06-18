use std::error::Error;
use std::fmt;

use draft_model::{Draft, Microseconds, RationalFrameRate, TargetTimerange};
use engine_core::{EngineProfile, RenderRangeState, normalize_draft, resolve_render_range};
use render_graph::{
    OutputDimensions, RenderGraph, RenderGraphError, RenderIntentSupport, build_render_graph,
};
use serde::{Deserialize, Serialize};

use crate::{RealtimePreviewDiagnostic, RealtimePreviewDiagnosticDomain, RealtimePreviewSupport};

const MICROSECONDS_PER_SECOND: u128 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewGraphInput {
    pub draft: Draft,
    pub target_time: Microseconds,
    pub preview_dimensions: OutputDimensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedRealtimePreviewGraph {
    pub profile: EngineProfile,
    pub target_time: Microseconds,
    pub frame_rate: RationalFrameRate,
    pub preview_dimensions: OutputDimensions,
    pub render_range: RenderRangeState,
    pub graph: RenderGraph,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewGraphPrepareError {
    pub kind: RealtimePreviewGraphPrepareErrorKind,
    pub message: String,
}

impl RealtimePreviewGraphPrepareError {
    fn new(kind: RealtimePreviewGraphPrepareErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for RealtimePreviewGraphPrepareError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for RealtimePreviewGraphPrepareError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RealtimePreviewGraphPrepareErrorKind {
    InvalidPreviewProfile,
    EngineFailed,
    RenderGraphFailed,
}

pub fn prepare_realtime_preview_graph(
    input: RealtimePreviewGraphInput,
) -> Result<PreparedRealtimePreviewGraph, RealtimePreviewGraphPrepareError> {
    validate_preview_dimensions(input.preview_dimensions)?;

    let profile = EngineProfile::from_draft_canvas(&input.draft).map_err(|error| {
        RealtimePreviewGraphPrepareError::new(
            RealtimePreviewGraphPrepareErrorKind::EngineFailed,
            format!("realtime preview engine profile resolution failed: {error}"),
        )
    })?;
    let frame_duration = single_frame_duration(&profile.frame_rate)?;
    let normalized = normalize_draft(&input.draft, &profile).map_err(|error| {
        RealtimePreviewGraphPrepareError::new(
            RealtimePreviewGraphPrepareErrorKind::EngineFailed,
            format!("realtime preview draft normalization failed: {error}"),
        )
    })?;
    let target_timerange = TargetTimerange::new(input.target_time, frame_duration);
    let render_range = resolve_render_range(&normalized, target_timerange).map_err(|error| {
        RealtimePreviewGraphPrepareError::new(
            RealtimePreviewGraphPrepareErrorKind::EngineFailed,
            format!("realtime preview render range resolution failed: {error}"),
        )
    })?;
    let graph = build_render_graph(&normalized, &render_range).map_err(render_graph_error)?;
    let diagnostics = graph_diagnostics(&graph);

    Ok(PreparedRealtimePreviewGraph {
        target_time: input.target_time,
        frame_rate: profile.frame_rate.clone(),
        preview_dimensions: input.preview_dimensions,
        profile,
        render_range,
        graph,
        diagnostics,
    })
}

fn validate_preview_dimensions(
    dimensions: OutputDimensions,
) -> Result<(), RealtimePreviewGraphPrepareError> {
    if dimensions.width == 0 || dimensions.height == 0 {
        return Err(RealtimePreviewGraphPrepareError::new(
            RealtimePreviewGraphPrepareErrorKind::InvalidPreviewProfile,
            "preview dimensions width and height must be greater than zero",
        ));
    }
    Ok(())
}

fn single_frame_duration(
    frame_rate: &RationalFrameRate,
) -> Result<Microseconds, RealtimePreviewGraphPrepareError> {
    if frame_rate.numerator == 0 || frame_rate.denominator == 0 {
        return Err(RealtimePreviewGraphPrepareError::new(
            RealtimePreviewGraphPrepareErrorKind::EngineFailed,
            "realtime preview frameRate numerator and denominator must be greater than zero",
        ));
    }

    let value = u128::from(frame_rate.denominator)
        .checked_mul(MICROSECONDS_PER_SECOND)
        .ok_or_else(|| {
            RealtimePreviewGraphPrepareError::new(
                RealtimePreviewGraphPrepareErrorKind::EngineFailed,
                "realtime preview single-frame duration overflowed",
            )
        })?
        / u128::from(frame_rate.numerator);

    u64::try_from(value).map(Microseconds::new).map_err(|_| {
        RealtimePreviewGraphPrepareError::new(
            RealtimePreviewGraphPrepareErrorKind::EngineFailed,
            "realtime preview single-frame duration exceeded u64 microseconds",
        )
    })
}

fn render_graph_error(error: RenderGraphError) -> RealtimePreviewGraphPrepareError {
    RealtimePreviewGraphPrepareError::new(
        RealtimePreviewGraphPrepareErrorKind::RenderGraphFailed,
        format!("realtime preview render graph failed: {error}"),
    )
}

fn graph_diagnostics(graph: &RenderGraph) -> Vec<RealtimePreviewDiagnostic> {
    let canvas = graph.canvas.diagnostics.iter().map(|diagnostic| {
        RealtimePreviewDiagnostic::new(
            None,
            RealtimePreviewDiagnosticDomain::Canvas,
            support_from_render_intent(diagnostic.support, &diagnostic.reason),
            diagnostic.reason.clone(),
            None,
            false,
        )
    });
    let visual = graph.visual_diagnostics.iter().map(|diagnostic| {
        RealtimePreviewDiagnostic::new(
            Some(diagnostic.segment_id.as_str().to_owned()),
            diagnostic_domain_for_property(&diagnostic.property),
            support_from_render_intent(diagnostic.support, &diagnostic.reason),
            diagnostic.reason.clone(),
            None,
            false,
        )
    });
    canvas.chain(visual).collect()
}

fn support_from_render_intent(
    support: RenderIntentSupport,
    reason: &str,
) -> RealtimePreviewSupport {
    match support {
        RenderIntentSupport::Supported => RealtimePreviewSupport::Supported,
        RenderIntentSupport::Degraded => RealtimePreviewSupport::Degraded {
            reason: reason.to_owned(),
        },
        RenderIntentSupport::Unsupported => RealtimePreviewSupport::Unsupported {
            reason: reason.to_owned(),
        },
    }
}

fn diagnostic_domain_for_property(property: &str) -> RealtimePreviewDiagnosticDomain {
    match property {
        "rotation" | "backgroundFilling" | "blendMode" | "mask" => {
            RealtimePreviewDiagnosticDomain::Transform
        }
        "visualPositionX" | "visualPositionY" | "visualScaleX" | "visualScaleY"
        | "visualOpacity" | "visualRotation" | "textFontSize" | "textColor" | "textLineHeight"
        | "textLetterSpacing" | "textLayoutX" | "textLayoutY" | "textLayoutWidth"
        | "textLayoutHeight" | "volume" => RealtimePreviewDiagnosticDomain::Keyframe,
        _ => RealtimePreviewDiagnosticDomain::VisualLayer,
    }
}

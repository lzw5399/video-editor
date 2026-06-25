//! Typed render intent graph.
//!
//! This crate will translate resolved draft frame state into a renderer-neutral
//! graph of materials, tracks, segments, filters, transitions, and text intents.
//! It does not execute FFmpeg jobs or decide editing behavior.

pub mod effects;
pub mod fingerprint;
pub mod graph;
pub mod incremental;
pub mod profile;

pub use effects::{
    ProductionEffectCapabilityDecision, RenderEffectCapability, render_blend_capability,
    render_effect_capability, render_mask_capability, render_retime_capability,
    render_transition_capability,
};
pub use fingerprint::{
    GRAPH_GENERATOR_VERSION, GRAPH_SCHEMA_VERSION, RenderGraphNodeFingerprint, RenderGraphSnapshot,
    deterministic_fingerprint,
};
pub use graph::{
    RenderAudioEffectSlot, RenderAudioEffectSlotSupport, RenderAudioMix,
    RenderAudioMixClassification, RenderAudioMixDiagnostic, RenderAudioVolumeKeyframe,
    RenderCanvas, RenderCanvasBackground, RenderCanvasBackgroundMode, RenderCanvasDiagnostic,
    RenderFilterIntent, RenderGraph, RenderGraphError, RenderGraphErrorKind, RenderIntentSupport,
    RenderMaterial, RenderRetimeIntent, RenderSampledFrame, RenderTextOverlay,
    RenderTransitionIntent, RenderTransitionWindow, RenderVideoLayer, RenderVisualDiagnostic,
    build_render_graph,
};
pub use incremental::{
    RenderGraphDiff, RenderGraphNodeChange, RenderGraphNodeId, RenderGraphNodeRole,
};
pub use profile::{
    ExportMp4Preset, OutputDimensions, PreviewFrameFormat, RenderAudioCodec, RenderContainer,
    RenderGraphPlan, RenderOutputProfile, RenderVideoCodec,
};

/// Boundary marker for render intent graph types.
pub const RENDER_GRAPH_BOUNDARY: &str = "semantic-render-intents";

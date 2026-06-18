use render_graph::{RenderGraph, RenderIntentSupport};
use serde::{Deserialize, Serialize};

use crate::{
    RealtimePreviewCapabilityReport, RealtimePreviewDiagnosticDomain, RealtimePreviewSupport,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewParityDiagnostic {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    pub domain: RealtimePreviewDiagnosticDomain,
    pub preview_support: RealtimePreviewSupport,
    pub export_support: RenderIntentSupport,
    pub reason: String,
    pub fallback_used: bool,
}

pub fn realtime_preview_parity_diagnostics(
    graph: &RenderGraph,
    report: &RealtimePreviewCapabilityReport,
) -> Vec<RealtimePreviewParityDiagnostic> {
    report
        .diagnostics
        .iter()
        .filter(|diagnostic| parity_domain(diagnostic.domain))
        .filter_map(|diagnostic| {
            let export_support =
                export_support_for(graph, diagnostic.domain, diagnostic.entity_id.as_deref())?;
            if equivalent_support(&diagnostic.support, export_support) {
                return None;
            }
            Some(RealtimePreviewParityDiagnostic {
                entity_id: diagnostic.entity_id.clone(),
                domain: diagnostic.domain,
                preview_support: diagnostic.support.clone(),
                export_support,
                reason: parity_reason(diagnostic.domain),
                fallback_used: diagnostic.fallback_used,
            })
        })
        .collect()
}

fn parity_domain(domain: RealtimePreviewDiagnosticDomain) -> bool {
    matches!(
        domain,
        RealtimePreviewDiagnosticDomain::Canvas
            | RealtimePreviewDiagnosticDomain::MaterialFrame
            | RealtimePreviewDiagnosticDomain::VisualLayer
            | RealtimePreviewDiagnosticDomain::Transform
            | RealtimePreviewDiagnosticDomain::Text
            | RealtimePreviewDiagnosticDomain::Keyframe
            | RealtimePreviewDiagnosticDomain::Effect
    )
}

fn export_support_for(
    graph: &RenderGraph,
    domain: RealtimePreviewDiagnosticDomain,
    entity_id: Option<&str>,
) -> Option<RenderIntentSupport> {
    match domain {
        RealtimePreviewDiagnosticDomain::Canvas => Some(graph.canvas.background.support),
        RealtimePreviewDiagnosticDomain::MaterialFrame => entity_id
            .and_then(|entity_id| {
                graph
                    .materials
                    .iter()
                    .find(|material| material.material_id.as_str() == entity_id)
            })
            .map(|_| RenderIntentSupport::Supported),
        RealtimePreviewDiagnosticDomain::Text => entity_id
            .and_then(|entity_id| {
                graph
                    .text_overlays
                    .iter()
                    .find(|text| text.overlay.segment_id.as_str() == entity_id)
            })
            .map(|text| {
                if text
                    .overlay
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.support == "unsupported")
                {
                    RenderIntentSupport::Unsupported
                } else {
                    RenderIntentSupport::Supported
                }
            }),
        RealtimePreviewDiagnosticDomain::Effect => entity_id
            .and_then(|entity_id| {
                graph
                    .video_layers
                    .iter()
                    .find(|layer| layer.segment_id.as_str() == entity_id)
            })
            .map(|layer| {
                layer
                    .filters
                    .first()
                    .map(|filter| filter.support)
                    .or_else(|| {
                        layer
                            .transition
                            .as_ref()
                            .map(|transition| transition.support)
                    })
                    .unwrap_or(RenderIntentSupport::Supported)
            }),
        RealtimePreviewDiagnosticDomain::VisualLayer
        | RealtimePreviewDiagnosticDomain::Transform
        | RealtimePreviewDiagnosticDomain::Keyframe => entity_id
            .and_then(|entity_id| {
                graph
                    .visual_diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic.segment_id.as_str() == entity_id)
            })
            .map(|diagnostic| diagnostic.support)
            .or(Some(RenderIntentSupport::Supported)),
        RealtimePreviewDiagnosticDomain::Surface
        | RealtimePreviewDiagnosticDomain::Runtime
        | RealtimePreviewDiagnosticDomain::Cancellation => None,
    }
}

fn equivalent_support(
    preview_support: &RealtimePreviewSupport,
    export_support: RenderIntentSupport,
) -> bool {
    matches!(
        (preview_support, export_support),
        (
            RealtimePreviewSupport::Supported,
            RenderIntentSupport::Supported
        ) | (
            RealtimePreviewSupport::Degraded { .. },
            RenderIntentSupport::Degraded
        ) | (
            RealtimePreviewSupport::Unsupported { .. },
            RenderIntentSupport::Unsupported
        )
    )
}

fn parity_reason(domain: RealtimePreviewDiagnosticDomain) -> String {
    let domain_name = match domain {
        RealtimePreviewDiagnosticDomain::Canvas => "canvas",
        RealtimePreviewDiagnosticDomain::MaterialFrame => "material frame",
        RealtimePreviewDiagnosticDomain::VisualLayer => "visual layer",
        RealtimePreviewDiagnosticDomain::Transform => "transform",
        RealtimePreviewDiagnosticDomain::Text => "text",
        RealtimePreviewDiagnosticDomain::Keyframe => "keyframe",
        RealtimePreviewDiagnosticDomain::Effect => "effect",
        RealtimePreviewDiagnosticDomain::Surface => "surface",
        RealtimePreviewDiagnosticDomain::Runtime => "runtime",
        RealtimePreviewDiagnosticDomain::Cancellation => "cancellation",
    };
    format!("realtime preview {domain_name} support diverges from export graph intent")
}

use draft_model::resolve_bundled_font;
use render_graph::RenderGraph;
use serde::{Deserialize, Serialize};

use crate::{
    RealtimePreviewCapabilityClassifier, RealtimePreviewDiagnostic,
    RealtimePreviewDiagnosticDomain, RealtimePreviewFallbackReason, RealtimePreviewGraphSupport,
    RealtimePreviewSupport,
};

pub const TEXT_PARITY_UNPROVEN_REASON: &str = "gpu text parity has not been proven with repository fonts; realtime preview must use fallback text rasterization";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextPreviewOutcome {
    pub support: RealtimePreviewGraphSupport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<RealtimePreviewFallbackReason>,
    pub diagnostics: Vec<RealtimePreviewDiagnostic>,
}

pub fn classify_text_preview_outcome(
    graph: &RenderGraph,
    classifier: &RealtimePreviewCapabilityClassifier,
) -> TextPreviewOutcome {
    let diagnostics = graph
        .text_overlays
        .iter()
        .map(|text| {
            text_preview_diagnostic(
                text,
                classifier.gpu_text_parity,
                classifier.bundled_text_font_registry_available,
            )
        })
        .collect::<Vec<_>>();
    let support = summarize_text_support(&diagnostics);
    let fallback_reason = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.fallback_used)
        .then_some(RealtimePreviewFallbackReason::TextParityUnsupported);

    TextPreviewOutcome {
        support,
        fallback_reason,
        diagnostics,
    }
}

pub(crate) fn text_preview_diagnostic(
    text: &render_graph::RenderTextOverlay,
    gpu_text_parity: bool,
    bundled_text_font_registry_available: bool,
) -> RealtimePreviewDiagnostic {
    if let Some(unsupported) = text
        .overlay
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.support == "unsupported")
    {
        RealtimePreviewDiagnostic::new(
            Some(text.overlay.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Text,
            RealtimePreviewSupport::Unsupported {
                reason: unsupported.reason.clone(),
            },
            unsupported.reason.clone(),
            Some(RealtimePreviewFallbackReason::UnsupportedGraphIntent),
            true,
        )
    } else if let Some(font_ref) = text.overlay.font_ref.as_deref() {
        if resolve_bundled_font(font_ref).is_none() {
            RealtimePreviewDiagnostic::new(
                Some(text.overlay.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::Text,
                RealtimePreviewSupport::Unsupported {
                    reason: format!("text fontRef {font_ref} is not available in realtime preview"),
                },
                format!("text fontRef {font_ref} is not available in realtime preview"),
                Some(RealtimePreviewFallbackReason::TextParityUnsupported),
                true,
            )
        } else if bundled_text_font_registry_available {
            RealtimePreviewDiagnostic::new(
                Some(text.overlay.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::Text,
                RealtimePreviewSupport::Supported,
                format!(
                    "text fontRef {font_ref} is resolved through bundled realtime font registry"
                ),
                None,
                false,
            )
        } else if gpu_text_parity {
            RealtimePreviewDiagnostic::new(
                Some(text.overlay.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::Text,
                RealtimePreviewSupport::Supported,
                "text intent is realtime supported by proven GPU text parity",
                None,
                false,
            )
        } else {
            RealtimePreviewDiagnostic::new(
                Some(text.overlay.segment_id.as_str().to_owned()),
                RealtimePreviewDiagnosticDomain::Text,
                RealtimePreviewSupport::Unsupported {
                    reason: TEXT_PARITY_UNPROVEN_REASON.to_owned(),
                },
                TEXT_PARITY_UNPROVEN_REASON,
                Some(RealtimePreviewFallbackReason::TextParityUnsupported),
                true,
            )
        }
    } else if gpu_text_parity {
        RealtimePreviewDiagnostic::new(
            Some(text.overlay.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Text,
            RealtimePreviewSupport::Supported,
            "text intent is realtime supported by proven GPU text parity",
            None,
            false,
        )
    } else {
        RealtimePreviewDiagnostic::new(
            Some(text.overlay.segment_id.as_str().to_owned()),
            RealtimePreviewDiagnosticDomain::Text,
            RealtimePreviewSupport::Unsupported {
                reason: TEXT_PARITY_UNPROVEN_REASON.to_owned(),
            },
            TEXT_PARITY_UNPROVEN_REASON,
            Some(RealtimePreviewFallbackReason::TextParityUnsupported),
            true,
        )
    }
}

fn summarize_text_support(
    diagnostics: &[RealtimePreviewDiagnostic],
) -> RealtimePreviewGraphSupport {
    if diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.support,
            RealtimePreviewSupport::Unsupported { .. }
        )
    }) {
        RealtimePreviewGraphSupport::Unsupported
    } else if diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.support, RealtimePreviewSupport::Degraded { .. }))
    {
        RealtimePreviewGraphSupport::Degraded
    } else {
        RealtimePreviewGraphSupport::Supported
    }
}

use serde::{Deserialize, Serialize};

use crate::{PreviewCancellationToken, RealtimePreviewFallbackReason};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewDiagnostic {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    pub domain: RealtimePreviewDiagnosticDomain,
    pub support: RealtimePreviewSupport,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<RealtimePreviewFallbackReason>,
    pub fallback_used: bool,
    pub canceled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_token: Option<PreviewCancellationToken>,
}

impl RealtimePreviewDiagnostic {
    pub fn new(
        entity_id: Option<String>,
        domain: RealtimePreviewDiagnosticDomain,
        support: RealtimePreviewSupport,
        reason: impl Into<String>,
        fallback: Option<RealtimePreviewFallbackReason>,
        fallback_used: bool,
    ) -> Self {
        Self {
            entity_id,
            domain,
            support,
            reason: reason.into(),
            fallback,
            fallback_used,
            canceled: false,
            cancellation_token: None,
        }
    }

    pub fn runtime(
        reason: impl Into<String>,
        support: RealtimePreviewSupport,
        fallback: Option<RealtimePreviewFallbackReason>,
        fallback_used: bool,
        canceled: bool,
        cancellation_token: Option<PreviewCancellationToken>,
    ) -> Self {
        Self {
            entity_id: None,
            domain: RealtimePreviewDiagnosticDomain::Runtime,
            support,
            reason: reason.into(),
            fallback,
            fallback_used,
            canceled,
            cancellation_token,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RealtimePreviewDiagnosticDomain {
    Canvas,
    MaterialFrame,
    VisualLayer,
    Transform,
    Text,
    Audio,
    Keyframe,
    Effect,
    Surface,
    Runtime,
    Cancellation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RealtimePreviewSupport {
    Supported,
    Degraded { reason: String },
    Unsupported { reason: String },
}

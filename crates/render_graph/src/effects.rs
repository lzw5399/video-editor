use draft_model::{
    BlendModeKind, CapabilitySupport, CapabilitySurface, EffectCapabilityRegistry, Filter,
    SegmentBlendMode, SegmentMask, SegmentRetiming, TrackTransition, Transition,
};
use serde::{Deserialize, Serialize};

use crate::RenderIntentSupport;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderEffectCapability {
    pub capability_id: String,
    pub preview: RenderIntentSupport,
    pub preview_reason: String,
    pub export: RenderIntentSupport,
    pub export_reason: String,
    pub report_only_external: bool,
}

pub type ProductionEffectCapabilityDecision = RenderEffectCapability;

pub fn render_effect_capability(filter: &Filter) -> RenderEffectCapability {
    let registry = EffectCapabilityRegistry::phase19_first_party();
    decision_from_registry(
        &registry,
        &filter.capability_id(),
        filter.display_name(),
        filter.external().is_some(),
    )
}

pub fn render_transition_capability(transition: &Transition) -> RenderEffectCapability {
    let registry = EffectCapabilityRegistry::phase19_first_party();
    decision_from_registry(
        &registry,
        &transition.capability_id(),
        transition.display_name(),
        transition.external().is_some(),
    )
}

pub fn render_track_transition_capability(transition: &TrackTransition) -> RenderEffectCapability {
    if transition.external().is_some() {
        return external_decision(
            transition.capability_id(),
            format!(
                "external transition reference {} is report-only and unsupported",
                transition.capability_id()
            ),
        );
    }

    let registry = EffectCapabilityRegistry::phase19_first_party();
    decision_from_registry(
        &registry,
        &transition.capability_id(),
        transition.display_name(),
        transition.external().is_some(),
    )
}

pub fn render_retime_capability(retiming: &SegmentRetiming) -> RenderEffectCapability {
    let registry = EffectCapabilityRegistry::phase19_first_party();
    decision_from_registry(
        &registry,
        retiming.mode.capability_id(),
        "segment retiming",
        false,
    )
}

pub fn render_mask_capability(mask: &SegmentMask) -> RenderEffectCapability {
    let registry = EffectCapabilityRegistry::phase19_first_party();
    match mask.mask_kind() {
        Some(kind) => {
            decision_from_registry(&registry, kind.capability_id(), "segment mask", false)
        }
        None => match mask {
            SegmentMask::None => RenderEffectCapability {
                capability_id: "mask.none".to_owned(),
                preview: RenderIntentSupport::Supported,
                preview_reason: "no segment mask is directly supported".to_owned(),
                export: RenderIntentSupport::Supported,
                export_reason: "no segment mask requires compiler work".to_owned(),
                report_only_external: false,
            },
            SegmentMask::ExternalReference { reference } => external_decision(
                format!("external:{}:{}", reference.provider, reference.effect_id),
                "external mask reference is report-only and unsupported",
            ),
            _ => unreachable!("mask_kind handles first-party masks"),
        },
    }
}

pub fn render_blend_capability(blend_mode: &SegmentBlendMode) -> RenderEffectCapability {
    let registry = EffectCapabilityRegistry::phase19_first_party();
    match blend_mode.kind() {
        Some(BlendModeKind::Normal) => decision_from_registry(
            &registry,
            BlendModeKind::Normal.capability_id(),
            "normal blend",
            false,
        ),
        Some(BlendModeKind::Multiply) => decision_from_registry(
            &registry,
            BlendModeKind::Multiply.capability_id(),
            "multiply blend",
            false,
        ),
        Some(BlendModeKind::Screen) => decision_from_registry(
            &registry,
            BlendModeKind::Screen.capability_id(),
            "screen blend",
            false,
        ),
        None => match blend_mode {
            SegmentBlendMode::ExternalReference { reference } => external_decision(
                format!("external:{}:{}", reference.provider, reference.effect_id),
                "external blend reference is report-only and unsupported",
            ),
            _ => unreachable!("blend kind handles first-party blend modes"),
        },
    }
}

fn decision_from_registry(
    registry: &EffectCapabilityRegistry,
    capability_id: &str,
    display_name: impl AsRef<str>,
    report_only_external: bool,
) -> RenderEffectCapability {
    if let Some(entry) = registry.entry(capability_id) {
        let preview = entry.support_for(CapabilitySurface::Preview);
        let export = entry.support_for(CapabilitySurface::Export);
        return RenderEffectCapability {
            capability_id: entry.capability_id.clone(),
            preview: support_from_capability(preview),
            preview_reason: preview.reason().to_owned(),
            export: support_from_capability(export),
            export_reason: export.reason().to_owned(),
            report_only_external,
        };
    }

    external_decision(
        capability_id.to_owned(),
        format!(
            "{} has no first-party Phase 19 capability entry",
            display_name.as_ref()
        ),
    )
}

fn external_decision(capability_id: String, reason: impl Into<String>) -> RenderEffectCapability {
    let reason = reason.into();
    RenderEffectCapability {
        capability_id,
        preview: RenderIntentSupport::Unsupported,
        preview_reason: reason.clone(),
        export: RenderIntentSupport::Unsupported,
        export_reason: reason,
        report_only_external: true,
    }
}

fn support_from_capability(support: &CapabilitySupport) -> RenderIntentSupport {
    match support {
        CapabilitySupport::Supported { .. } => RenderIntentSupport::Supported,
        CapabilitySupport::Degraded { .. } => RenderIntentSupport::Degraded,
        CapabilitySupport::Unsupported { .. } | CapabilitySupport::ExternalReference { .. } => {
            RenderIntentSupport::Unsupported
        }
    }
}

//! Draft-level canvas command semantics.

use draft_model::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasAspectRatioPreset, CommandEvent,
    CommandState, Draft, DraftCanvasConfig, Material, MaterialKind, TimelineCommandResponse,
    TimelineSelection, reduce_ratio, validate_draft,
};

use crate::{TimelineCommandError, delta::canvas_delta, history::push_undo_snapshot};

pub fn update_draft_canvas_config(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    canvas_config: DraftCanvasConfig,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let mut canvas_config = canvas_config;
    canvas_config.adaptation_policy = CanvasAdaptationPolicy::Manual;
    next_draft.canvas_config = canvas_config;
    validate_draft(&next_draft)?;
    let delta = canvas_delta(&next_draft);

    let (command_state, pruned) =
        push_undo_snapshot(command_state, draft, selection, "updateDraftCanvasConfig");
    let mut events = vec![CommandEvent {
        kind: "draftCanvasConfigUpdated".to_owned(),
        message: None,
    }];
    if pruned {
        events.push(CommandEvent {
            kind: "historyLimitPruned".to_owned(),
            message: None,
        });
    }

    Ok(TimelineCommandResponse {
        draft: next_draft,
        command_state,
        selection: selection.clone(),
        events,
        delta,
    })
}

pub fn first_visual_material_canvas_config(
    current: &DraftCanvasConfig,
    material: &Material,
) -> Option<DraftCanvasConfig> {
    if current.adaptation_policy != CanvasAdaptationPolicy::Auto {
        return None;
    }
    if !matches!(material.kind, MaterialKind::Video | MaterialKind::Image) {
        return None;
    }

    let width = material.metadata.width?;
    let height = material.metadata.height?;
    if width == 0 || height == 0 {
        return None;
    }

    let frame_rate = if material.kind == MaterialKind::Video {
        material
            .metadata
            .frame_rate
            .clone()
            .unwrap_or_else(|| current.frame_rate.clone())
    } else {
        current.frame_rate.clone()
    };

    Some(DraftCanvasConfig {
        aspect_ratio: aspect_ratio_for_dimensions(width, height),
        width,
        height,
        frame_rate,
        background: current.background.clone(),
        adaptation_policy: CanvasAdaptationPolicy::Auto,
    })
}

fn aspect_ratio_for_dimensions(width: u32, height: u32) -> CanvasAspectRatio {
    match reduce_ratio(width, height) {
        Some((16, 9)) => CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9),
        Some((9, 16)) => CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio9x16),
        Some((1, 1)) => CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio1x1),
        Some((4, 3)) => CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio4x3),
        Some((3, 4)) => CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio3x4),
        Some((numerator, denominator)) => CanvasAspectRatio::custom(numerator, denominator),
        None => CanvasAspectRatio::custom(width, height),
    }
}

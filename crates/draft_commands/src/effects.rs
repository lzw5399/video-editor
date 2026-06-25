//! Rust-owned segment effect/filter command semantics.

use draft_model::{
    CapabilityCategory, CommandDeltaName, CommandEvent, CommandState, Draft,
    EffectCapabilityRegistry, EffectParameterUpdate, Filter, FilterKind, SegmentId,
    TimelineCommandResponse, TimelineSelection, TrackKind,
};

use crate::{
    TimelineCommandError, TimelineCommandErrorKind,
    delta::effect_segment_delta,
    history::push_undo_snapshot,
    timeline::{find_segment_location, validate_timeline_rules, validate_track_unlocked},
};

const MAX_GAUSSIAN_BLUR_RADIUS_MILLIS: u32 = 100_000;
const MIN_BRIGHTNESS_MILLIS: i32 = -1_000;
const MAX_BRIGHTNESS_MILLIS: i32 = 1_000;
const MAX_COLOR_MULTIPLIER_MILLIS: u32 = 5_000;
const MAX_OPACITY_MILLIS: u32 = 1_000;

pub fn apply_segment_effect(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    effect: Filter,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;
    validate_effect_track(next_draft.tracks[track_index].kind, &segment_id)?;
    validate_supported_effect(&segment_id, &effect)?;

    next_draft.tracks[track_index].segments[segment_index]
        .filters
        .push(effect);
    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();
    let segment = &next_draft.tracks[track_index].segments[segment_index];
    let delta = effect_segment_delta(
        CommandDeltaName::ApplySegmentEffect,
        &track_id,
        segment,
        "segment effect applied",
    );

    Ok(response(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "applySegmentEffect"),
        selection.clone(),
        "segmentEffectApplied",
        delta,
    ))
}

pub fn update_segment_effect_parameter(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    effect_index: u32,
    parameter: EffectParameterUpdate,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;
    validate_effect_track(next_draft.tracks[track_index].kind, &segment_id)?;
    let filter_index = effect_index as usize;
    let original = next_draft.tracks[track_index].segments[segment_index]
        .filters
        .get(filter_index)
        .cloned()
        .ok_or_else(|| effect_not_found(&segment_id, effect_index))?;
    let updated = apply_parameter_update(&segment_id, original, parameter)?;
    validate_supported_effect(&segment_id, &updated)?;

    next_draft.tracks[track_index].segments[segment_index].filters[filter_index] = updated;
    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();
    let segment = &next_draft.tracks[track_index].segments[segment_index];
    let delta = effect_segment_delta(
        CommandDeltaName::UpdateSegmentEffectParameter,
        &track_id,
        segment,
        "segment effect parameter updated",
    );

    Ok(response(
        next_draft,
        command_state_after_commit(
            command_state,
            draft,
            selection,
            "updateSegmentEffectParameter",
        ),
        selection.clone(),
        "segmentEffectParameterUpdated",
        delta,
    ))
}

pub fn remove_segment_effect(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    effect_index: u32,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;
    validate_effect_track(next_draft.tracks[track_index].kind, &segment_id)?;
    let filter_index = effect_index as usize;
    if filter_index
        >= next_draft.tracks[track_index].segments[segment_index]
            .filters
            .len()
    {
        return Err(effect_not_found(&segment_id, effect_index));
    }

    next_draft.tracks[track_index].segments[segment_index]
        .filters
        .remove(filter_index);
    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();
    let segment = &next_draft.tracks[track_index].segments[segment_index];
    let delta = effect_segment_delta(
        CommandDeltaName::RemoveSegmentEffect,
        &track_id,
        segment,
        "segment effect removed",
    );

    Ok(response(
        next_draft,
        command_state_after_commit(command_state, draft, selection, "removeSegmentEffect"),
        selection.clone(),
        "segmentEffectRemoved",
        delta,
    ))
}

fn validate_supported_effect(
    segment_id: &SegmentId,
    effect: &Filter,
) -> Result<(), TimelineCommandError> {
    validate_effect_parameters(segment_id, effect)?;
    let registry = EffectCapabilityRegistry::phase19_first_party();
    let capability_id = effect.capability_id();
    let Some(entry) = registry.entry(&capability_id) else {
        return unsupported_effect(
            segment_id,
            capability_id,
            "effect capability is not registered for Phase 19 first-party semantics",
        );
    };

    if effect.external().is_some() {
        return unsupported_effect(
            segment_id,
            capability_id,
            "external provider effects are report-only diagnostics and cannot commit as supported first-party effects",
        );
    }

    if !matches!(
        entry.category,
        CapabilityCategory::Effect | CapabilityCategory::Filter
    ) {
        return unsupported_effect(
            segment_id,
            capability_id,
            "capability is not an effect/filter command target",
        );
    }

    if !entry.preview.is_supported() || !entry.export.is_supported() {
        return unsupported_effect(
            segment_id,
            capability_id,
            format!(
                "preview/export support is not fully supported: preview={}, export={}",
                entry.preview.reason(),
                entry.export.reason()
            ),
        );
    }

    Ok(())
}

fn validate_effect_parameters(
    segment_id: &SegmentId,
    effect: &Filter,
) -> Result<(), TimelineCommandError> {
    match &effect.kind {
        FilterKind::GaussianBlur { radius_millis } => {
            if *radius_millis > MAX_GAUSSIAN_BLUR_RADIUS_MILLIS {
                return invalid_parameter(
                    segment_id,
                    effect.capability_id(),
                    "radiusMillis",
                    format!("radius_millis must be <= {MAX_GAUSSIAN_BLUR_RADIUS_MILLIS}"),
                );
            }
        }
        FilterKind::BasicColorAdjustment {
            brightness_millis,
            contrast_millis,
            saturation_millis,
        } => {
            if !(MIN_BRIGHTNESS_MILLIS..=MAX_BRIGHTNESS_MILLIS).contains(brightness_millis) {
                return invalid_parameter(
                    segment_id,
                    effect.capability_id(),
                    "brightnessMillis",
                    format!(
                        "brightness_millis must be between {MIN_BRIGHTNESS_MILLIS} and {MAX_BRIGHTNESS_MILLIS}"
                    ),
                );
            }
            if *contrast_millis > MAX_COLOR_MULTIPLIER_MILLIS {
                return invalid_parameter(
                    segment_id,
                    effect.capability_id(),
                    "contrastMillis",
                    format!("contrast_millis must be <= {MAX_COLOR_MULTIPLIER_MILLIS}"),
                );
            }
            if *saturation_millis > MAX_COLOR_MULTIPLIER_MILLIS {
                return invalid_parameter(
                    segment_id,
                    effect.capability_id(),
                    "saturationMillis",
                    format!("saturation_millis must be <= {MAX_COLOR_MULTIPLIER_MILLIS}"),
                );
            }
        }
        FilterKind::OpacityAdjustment { opacity_millis } => {
            if *opacity_millis > MAX_OPACITY_MILLIS {
                return invalid_parameter(
                    segment_id,
                    effect.capability_id(),
                    "opacityMillis",
                    format!("opacity_millis must be <= {MAX_OPACITY_MILLIS}"),
                );
            }
        }
        FilterKind::ExternalReference { .. } => {}
    }

    Ok(())
}

fn apply_parameter_update(
    segment_id: &SegmentId,
    mut effect: Filter,
    parameter: EffectParameterUpdate,
) -> Result<Filter, TimelineCommandError> {
    match (&mut effect.kind, parameter) {
        (_, EffectParameterUpdate::Enabled { enabled }) => {
            effect.enabled = enabled;
        }
        (
            FilterKind::GaussianBlur { radius_millis },
            EffectParameterUpdate::GaussianBlurRadiusMillis {
                radius_millis: next,
            },
        ) => {
            *radius_millis = next;
        }
        (
            FilterKind::BasicColorAdjustment {
                brightness_millis, ..
            },
            EffectParameterUpdate::BasicColorBrightnessMillis {
                brightness_millis: next,
            },
        ) => {
            *brightness_millis = next;
        }
        (
            FilterKind::BasicColorAdjustment {
                contrast_millis, ..
            },
            EffectParameterUpdate::BasicColorContrastMillis {
                contrast_millis: next,
            },
        ) => {
            *contrast_millis = next;
        }
        (
            FilterKind::BasicColorAdjustment {
                saturation_millis, ..
            },
            EffectParameterUpdate::BasicColorSaturationMillis {
                saturation_millis: next,
            },
        ) => {
            *saturation_millis = next;
        }
        (
            FilterKind::OpacityAdjustment { opacity_millis },
            EffectParameterUpdate::OpacityMillis {
                opacity_millis: next,
            },
        ) => {
            *opacity_millis = next;
        }
        (kind, parameter) => {
            let current = Filter {
                kind: kind.clone(),
                enabled: effect.enabled,
            };
            return invalid_parameter(
                segment_id,
                current.capability_id(),
                parameter_name(&parameter),
                "parameter does not apply to this effect kind",
            );
        }
    }

    Ok(effect)
}

fn validate_effect_track(
    track_kind: TrackKind,
    segment_id: &SegmentId,
) -> Result<(), TimelineCommandError> {
    if matches!(
        track_kind,
        TrackKind::Video | TrackKind::Text | TrackKind::Sticker | TrackKind::Filter
    ) {
        return Ok(());
    }

    Err(TimelineCommandError::new(
        TimelineCommandErrorKind::InvalidTrackOperation {
            track_id: "".into(),
            reason: format!(
                "segment {} is on a non-visual track; effects require a visual track",
                segment_id.as_str()
            ),
        },
    ))
}

fn parameter_name(parameter: &EffectParameterUpdate) -> &'static str {
    match parameter {
        EffectParameterUpdate::Enabled { .. } => "enabled",
        EffectParameterUpdate::GaussianBlurRadiusMillis { .. } => "radiusMillis",
        EffectParameterUpdate::BasicColorBrightnessMillis { .. } => "brightnessMillis",
        EffectParameterUpdate::BasicColorContrastMillis { .. } => "contrastMillis",
        EffectParameterUpdate::BasicColorSaturationMillis { .. } => "saturationMillis",
        EffectParameterUpdate::OpacityMillis { .. } => "opacityMillis",
    }
}

fn invalid_parameter<T>(
    segment_id: &SegmentId,
    capability_id: impl Into<String>,
    parameter: impl Into<String>,
    reason: impl Into<String>,
) -> Result<T, TimelineCommandError> {
    Err(TimelineCommandError::new(
        TimelineCommandErrorKind::InvalidEffectParameter {
            segment_id: segment_id.clone(),
            capability_id: capability_id.into(),
            parameter: parameter.into(),
            reason: reason.into(),
        },
    ))
}

fn unsupported_effect<T>(
    segment_id: &SegmentId,
    capability_id: impl Into<String>,
    reason: impl Into<String>,
) -> Result<T, TimelineCommandError> {
    Err(TimelineCommandError::new(
        TimelineCommandErrorKind::UnsupportedEffect {
            segment_id: segment_id.clone(),
            capability_id: capability_id.into(),
            reason: reason.into(),
        },
    ))
}

fn effect_not_found(segment_id: &SegmentId, effect_index: u32) -> TimelineCommandError {
    TimelineCommandError::new(TimelineCommandErrorKind::EffectNotFound {
        segment_id: segment_id.clone(),
        effect_index,
    })
}

fn response(
    draft: Draft,
    command_state: CommandStateWithEvents,
    selection: TimelineSelection,
    event_kind: &str,
    delta: draft_model::CommandDelta,
) -> TimelineCommandResponse {
    let mut events = vec![CommandEvent {
        kind: event_kind.to_owned(),
        message: None,
    }];
    events.extend(command_state.events);

    TimelineCommandResponse {
        draft,
        command_state: command_state.state,
        selection,
        events,
        delta,
    }
}

struct CommandStateWithEvents {
    state: CommandState,
    events: Vec<CommandEvent>,
}

fn command_state_after_commit(
    command_state: &CommandState,
    draft: &Draft,
    selection: &TimelineSelection,
    label: &str,
) -> CommandStateWithEvents {
    let (state, pruned) = push_undo_snapshot(command_state, draft, selection, label);
    let events = if pruned {
        vec![CommandEvent {
            kind: "historyLimitPruned".to_owned(),
            message: None,
        }]
    } else {
        Vec::new()
    };
    CommandStateWithEvents { state, events }
}

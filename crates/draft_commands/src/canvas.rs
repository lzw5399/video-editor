//! Draft-level canvas command semantics.

use draft_model::{
    CommandDelta, CommandEvent, CommandName, CommandState, Draft, DraftCanvasConfig,
    TimelineCommandResponse, TimelineSelection, validate_draft,
};

use crate::{TimelineCommandError, history::push_undo_snapshot};

pub fn update_draft_canvas_config(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    canvas_config: DraftCanvasConfig,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    next_draft.canvas_config = canvas_config;
    validate_draft(&next_draft)?;

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
        delta: CommandDelta::none(
            CommandName::UpdateDraftCanvasConfig,
            "delta pending command-specific builder",
        ),
    })
}

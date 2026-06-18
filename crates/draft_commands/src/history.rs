//! Session-only bounded undo/redo history for timeline commands.

use draft_model::{
    CommandEvent, CommandHistorySnapshot, CommandName, CommandState, Draft,
    TimelineCommandResponse, TimelineSelection,
};

use crate::{TimelineCommandError, TimelineCommandErrorKind, delta::restored_draft_delta};

pub const DEFAULT_HISTORY_LIMIT: u32 = 100;

pub fn push_undo_snapshot(
    command_state: &CommandState,
    draft: &Draft,
    selection: &TimelineSelection,
    label: impl Into<String>,
) -> (CommandState, bool) {
    let mut next_state = command_state.clone();
    if next_state.max_history_entries == 0 {
        next_state.max_history_entries = DEFAULT_HISTORY_LIMIT;
    }
    next_state.undo_stack.push(CommandHistorySnapshot {
        draft: draft.clone(),
        selection: selection.clone(),
        label: Some(label.into()),
    });
    clear_redo_after_commit(&mut next_state);
    let pruned = prune_history_to_limit(&mut next_state);
    (next_state, pruned)
}

pub fn clear_redo_after_commit(command_state: &mut CommandState) {
    command_state.redo_stack.clear();
}

pub fn prune_history_to_limit(command_state: &mut CommandState) -> bool {
    let limit = if command_state.max_history_entries == 0 {
        DEFAULT_HISTORY_LIMIT
    } else {
        command_state.max_history_entries
    } as usize;

    if command_state.undo_stack.len() <= limit {
        return false;
    }

    let remove_count = command_state.undo_stack.len() - limit;
    command_state.undo_stack.drain(0..remove_count);
    true
}

pub fn undo_timeline_edit(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_state = command_state.clone();
    let Some(snapshot) = next_state.undo_stack.pop() else {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::HistoryEmpty {
                direction: "undo".to_owned(),
            },
        ));
    };

    next_state.redo_stack.push(CommandHistorySnapshot {
        draft: draft.clone(),
        selection: selection.clone(),
        label: Some("redo snapshot".to_owned()),
    });

    let restored_draft = snapshot.draft;
    let restored_selection = snapshot.selection;
    let delta = restored_draft_delta(
        CommandName::UndoTimelineEdit,
        draft,
        &restored_draft,
        "undo restored a prior semantic snapshot",
    );

    Ok(TimelineCommandResponse {
        draft: restored_draft,
        command_state: next_state,
        selection: restored_selection,
        events: vec![event("undoCommitted")],
        delta,
    })
}

pub fn redo_timeline_edit(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_state = command_state.clone();
    let Some(snapshot) = next_state.redo_stack.pop() else {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::HistoryEmpty {
                direction: "redo".to_owned(),
            },
        ));
    };

    next_state.undo_stack.push(CommandHistorySnapshot {
        draft: draft.clone(),
        selection: selection.clone(),
        label: Some("undo snapshot".to_owned()),
    });
    let pruned = prune_history_to_limit(&mut next_state);

    let mut events = vec![event("redoCommitted")];
    if pruned {
        events.push(event("historyLimitPruned"));
    }

    let restored_draft = snapshot.draft;
    let restored_selection = snapshot.selection;
    let delta = restored_draft_delta(
        CommandName::RedoTimelineEdit,
        draft,
        &restored_draft,
        "redo restored a later semantic snapshot",
    );

    Ok(TimelineCommandResponse {
        draft: restored_draft,
        command_state: next_state,
        selection: restored_selection,
        events,
        delta,
    })
}

fn event(kind: &str) -> CommandEvent {
    CommandEvent {
        kind: kind.to_owned(),
        message: None,
    }
}

use draft_commands::{
    TimelineCommandErrorKind,
    canvas::update_draft_canvas_config,
    history::{redo_timeline_edit, undo_timeline_edit},
    timeline::execute_timeline_edit,
};
use draft_model::{
    CanvasAdaptationPolicy, CanvasAspectRatio, CanvasAspectRatioPreset, CanvasBackground,
    CommandState, Draft, DraftCanvasConfig, Material, MaterialKind, RationalFrameRate,
    TimelineEditPayload, TimelineSelection, UpdateDraftCanvasConfigCommandPayload,
};

#[test]
fn canvas_update_commits_after_validation() {
    let draft = draft_with_image_material();
    let state = CommandState::empty();
    let selection = selected_canvas_context();
    let canvas_config = vertical_canvas_config();

    let response = update_draft_canvas_config(&draft, &state, &selection, canvas_config.clone())
        .expect("valid canvas config should commit");

    assert_eq!(
        response.draft.canvas_config,
        accepted_manual_canvas_config(canvas_config)
    );
    assert_eq!(
        draft.canvas_config,
        DraftCanvasConfig::mvp_default(),
        "input draft stays unchanged"
    );
    assert_eq!(response.selection, selection);
    assert_eq!(response.events[0].kind, "draftCanvasConfigUpdated");
    assert_eq!(response.command_state.undo_stack.len(), 1);
    assert_eq!(response.command_state.undo_stack[0].draft, draft);
    assert_eq!(
        response.command_state.undo_stack[0].label.as_deref(),
        Some("updateDraftCanvasConfig")
    );
    assert!(response.command_state.redo_stack.is_empty());
}

#[test]
fn canvas_update_undo_redo_uses_existing_history() {
    let draft = draft_with_image_material();
    let state = CommandState::empty();
    let selection = selected_canvas_context();

    let updated = update_draft_canvas_config(&draft, &state, &selection, vertical_canvas_config())
        .expect("valid canvas config should commit");
    let undone = undo_timeline_edit(&updated.draft, &updated.command_state, &updated.selection)
        .expect("canvas update should be undoable");
    assert_eq!(undone.draft, draft);
    assert_eq!(undone.selection, selection);
    assert_eq!(undone.events[0].kind, "undoCommitted");
    assert_eq!(undone.command_state.redo_stack.len(), 1);

    let redone = redo_timeline_edit(&undone.draft, &undone.command_state, &undone.selection)
        .expect("canvas update should be redoable");
    assert_eq!(redone.draft, updated.draft);
    assert_eq!(redone.selection, updated.selection);
    assert_eq!(redone.events[0].kind, "redoCommitted");
    assert_eq!(redone.command_state.undo_stack.len(), 1);
    assert!(redone.command_state.redo_stack.is_empty());
}

#[test]
fn invalid_canvas_updates_are_atomic() {
    let draft = draft_with_image_material();
    let state = CommandState::empty();
    let selection = selected_canvas_context();

    for (label, canvas_config) in [
        ("aspect ratio mismatch", aspect_ratio_mismatch_config()),
        ("zero width", zero_width_config()),
        ("zero frame rate", zero_frame_rate_config()),
        ("invalid solid color", invalid_solid_color_config()),
        ("missing image material", missing_image_material_config()),
        ("video image material", video_image_material_config()),
    ] {
        let error = update_draft_canvas_config(&draft, &state, &selection, canvas_config.clone())
            .expect_err(label);
        assert!(
            matches!(
                error.kind,
                TimelineCommandErrorKind::DraftValidationFailed { .. }
            ),
            "{label} should surface as draft validation failure"
        );
        assert_eq!(
            draft,
            draft_with_image_material(),
            "{label} mutated input draft"
        );
        assert_eq!(
            state,
            CommandState::empty(),
            "{label} mutated command state"
        );
        assert_eq!(
            selection,
            selected_canvas_context(),
            "{label} mutated selection"
        );
    }
}

#[test]
fn execute_timeline_edit_routes_canvas_command() {
    let draft = draft_with_image_material();
    let state = CommandState::empty();
    let selection = selected_canvas_context();
    let canvas_config = square_canvas_config();

    let response = execute_timeline_edit(TimelineEditPayload::UpdateDraftCanvasConfig(
        UpdateDraftCanvasConfigCommandPayload {
            draft,
            command_state: state,
            selection,
            canvas_config: canvas_config.clone(),
        },
    ))
    .expect("timeline dispatcher should route canvas updates");

    assert_eq!(
        response.draft.canvas_config,
        accepted_manual_canvas_config(canvas_config)
    );
    assert_eq!(response.events[0].kind, "draftCanvasConfigUpdated");
}

fn draft_with_image_material() -> Draft {
    let mut draft = Draft::new("canvas-command-draft", "Canvas Command Draft");
    draft.materials.push(Material::new(
        "image-material",
        MaterialKind::Image,
        "media/background.png",
        "background.png",
    ));
    draft.materials.push(Material::new(
        "video-material",
        MaterialKind::Video,
        "media/video.mp4",
        "video.mp4",
    ));
    draft
}

fn selected_canvas_context() -> TimelineSelection {
    TimelineSelection {
        segment_ids: vec!["selected-segment".into()],
        track_ids: vec!["selected-track".into()],
    }
}

fn vertical_canvas_config() -> DraftCanvasConfig {
    DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio9x16),
        width: 1080,
        height: 1920,
        frame_rate: RationalFrameRate::new(25, 1),
        background: CanvasBackground::SolidColor {
            color: "#101820".to_owned(),
        },
        adaptation_policy: CanvasAdaptationPolicy::Auto,
    }
}

fn square_canvas_config() -> DraftCanvasConfig {
    DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio1x1),
        width: 1080,
        height: 1080,
        frame_rate: RationalFrameRate::new(30, 1),
        background: CanvasBackground::Black,
        adaptation_policy: CanvasAdaptationPolicy::Auto,
    }
}

fn aspect_ratio_mismatch_config() -> DraftCanvasConfig {
    DraftCanvasConfig {
        aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9),
        width: 1080,
        height: 1920,
        frame_rate: RationalFrameRate::new(25, 1),
        background: CanvasBackground::Black,
        adaptation_policy: CanvasAdaptationPolicy::Auto,
    }
}

fn zero_width_config() -> DraftCanvasConfig {
    DraftCanvasConfig {
        width: 0,
        ..vertical_canvas_config()
    }
}

fn zero_frame_rate_config() -> DraftCanvasConfig {
    DraftCanvasConfig {
        frame_rate: RationalFrameRate::new(0, 1),
        ..vertical_canvas_config()
    }
}

fn invalid_solid_color_config() -> DraftCanvasConfig {
    DraftCanvasConfig {
        background: CanvasBackground::SolidColor {
            color: "101820".to_owned(),
        },
        ..vertical_canvas_config()
    }
}

fn missing_image_material_config() -> DraftCanvasConfig {
    DraftCanvasConfig {
        background: CanvasBackground::Image {
            material_id: Some("missing-image".into()),
        },
        ..vertical_canvas_config()
    }
}

fn video_image_material_config() -> DraftCanvasConfig {
    DraftCanvasConfig {
        background: CanvasBackground::Image {
            material_id: Some("video-material".into()),
        },
        ..vertical_canvas_config()
    }
}

fn accepted_manual_canvas_config(mut canvas_config: DraftCanvasConfig) -> DraftCanvasConfig {
    canvas_config.adaptation_policy = CanvasAdaptationPolicy::Manual;
    canvas_config
}

use draft_model::{
    AddAudioSegmentIntentCommandPayload, AddTextSegmentIntentCommandPayload,
    AddTimelineSegmentIntentCommandPayload, AddTrackIntentCommandPayload, AudioEffectSlot,
    AudioFade, AudioPanBalance, CommandDelta, CommandErrorKind, CommandState,
    DeleteSegmentCommandPayload, Draft, DraftCanvasConfig, EditTextSegmentCommandPayload,
    ImportSubtitleSrtIntentCommandPayload, Keyframe, KeyframeEasing, KeyframeInterpolation,
    KeyframeProperty, KeyframeValue, MaterialId, MaterialKind, Microseconds,
    MissingMaterialCommandDiagnostic, MoveSelectedSegmentIntentCommandPayload,
    RemoveSegmentKeyframeCommandPayload, RenameTrackCommandPayload, Segment, SegmentId,
    SegmentVisual, SegmentVolume, SelectTimelineSegmentsCommandPayload,
    SetSegmentKeyframeCommandPayload, SetSegmentVolumeCommandPayload, SetTrackLockCommandPayload,
    SetTrackMuteCommandPayload, SetTrackVisibilityCommandPayload,
    SplitSelectedSegmentIntentCommandPayload, TextBox, TextLayoutRegion, TextSegment, TextStyle,
    TextWrapping, TimelineCommandResponse, TimelineEditPayload, TimelineSelection, Track, TrackId,
    TrackKind, TrimSegmentDirection, TrimSelectedSegmentIntentCommandPayload,
    UpdateDraftCanvasConfigCommandPayload, UpdateSegmentAudioCommandPayload,
    UpdateSegmentVisualCommandPayload,
};
use media_runtime::discover_runtime_config;
use media_runtime_desktop::DesktopFfmpegExecutor;
use napi::bindgen_prelude::Result;
use project_store::{
    ProjectStoreError, StdPlatformFileSystem, create_project_bundle, open_project_bundle,
    save_project_bundle,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OpenProjectSessionRequest {
    bundle_path: String,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateProjectSessionRequest {
    bundle_path: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    draft_id: Option<String>,
    #[serde(default)]
    draft_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSessionOpenResponse {
    session_id: String,
    revision: u64,
    draft: Draft,
    bundle_path: String,
    project_json_path: String,
    warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectSessionRequest {
    session_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectSessionReadRequest {
    session_id: String,
    expected_revision: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExecuteProjectIntentRequest {
    session_id: String,
    expected_revision: u64,
    intent: ProjectIntent,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum ProjectIntent {
    ImportMaterial {
        #[serde(rename = "materialPath")]
        material_path: String,
        #[serde(default, rename = "materialId")]
        material_id: Option<MaterialId>,
        #[serde(default, rename = "displayName")]
        display_name: Option<String>,
        #[serde(default, rename = "materialKindHint")]
        material_kind_hint: Option<MaterialKind>,
    },
    AddTimelineSegmentIntent {
        #[serde(rename = "materialId")]
        material_id: MaterialId,
    },
    SelectTimelineSegments {
        #[serde(default, rename = "segmentIds")]
        segment_ids: Vec<SegmentId>,
        #[serde(default, rename = "trackIds")]
        track_ids: Vec<TrackId>,
    },
    MoveSelectedSegmentIntent {
        delta: Microseconds,
    },
    SplitSelectedSegmentIntent {
        #[serde(rename = "splitAt")]
        split_at: Microseconds,
    },
    TrimSelectedSegmentIntent {
        direction: TrimSegmentDirection,
        delta: Microseconds,
    },
    DeleteSelectedSegment {},
    AddTextSegmentIntent {
        text: TextSegment,
        duration: Microseconds,
    },
    EditSelectedText {
        text: TextSegment,
    },
    ImportSubtitleSrtIntent {
        #[serde(rename = "srtContent")]
        srt_content: String,
        #[serde(rename = "timeOffset")]
        time_offset: Microseconds,
        style: TextStyle,
        #[serde(rename = "textBox")]
        text_box: TextBox,
        #[serde(rename = "layoutRegion")]
        layout_region: TextLayoutRegion,
        wrapping: TextWrapping,
    },
    AddAudioSegmentIntent {
        #[serde(default, rename = "materialId")]
        material_id: Option<MaterialId>,
        #[serde(default)]
        duration: Option<Microseconds>,
    },
    SetSelectedSegmentVolume {
        volume: SegmentVolume,
    },
    UpdateSelectedSegmentAudio {
        #[serde(default, rename = "gainMillis")]
        gain_millis: Option<u32>,
        #[serde(default, rename = "panBalanceMillis")]
        pan_balance_millis: Option<AudioPanBalance>,
        #[serde(default, rename = "fadeInDuration")]
        fade_in_duration: Option<AudioFade>,
        #[serde(default, rename = "fadeOutDuration")]
        fade_out_duration: Option<AudioFade>,
        #[serde(default, rename = "effectSlots")]
        effect_slots: Option<Vec<AudioEffectSlot>>,
    },
    AddTrackIntent {
        #[serde(rename = "trackKind")]
        track_kind: TrackKind,
    },
    RenameSelectedTrack {
        name: String,
    },
    SetSelectedTrackLock {
        locked: bool,
    },
    SetSelectedTrackVisibility {
        visible: bool,
    },
    SetSelectedTrackMute {
        muted: bool,
    },
    UpdateDraftCanvasConfig {
        #[serde(rename = "canvasConfig")]
        canvas_config: DraftCanvasConfig,
    },
    UpdateSelectedSegmentVisual {
        visual: SegmentVisual,
    },
    SetSelectedSegmentKeyframe {
        property: KeyframeProperty,
        at: Microseconds,
        interpolation: KeyframeInterpolation,
        easing: KeyframeEasing,
    },
    RemoveSelectedSegmentKeyframe {
        property: KeyframeProperty,
        at: Microseconds,
    },
    UndoTimelineEdit {},
    RedoTimelineEdit {},
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSessionClosedResponse {
    session_id: String,
    closed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSessionIntentResponse {
    session_id: String,
    revision: u64,
    draft: Draft,
    command_state: CommandState,
    selection: TimelineSelection,
    events: Vec<draft_model::CommandEvent>,
    delta: draft_model::CommandDelta,
    bundle_path: String,
    project_json_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSessionImportMaterialResponse {
    session_id: String,
    revision: u64,
    draft: Draft,
    material: draft_model::Material,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    diagnostic: Option<MissingMaterialCommandDiagnostic>,
    bundle_path: String,
    project_json_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSessionMaterialsResponse {
    session_id: String,
    revision: u64,
    bundle_path: String,
    project_json_path: String,
    materials: Vec<draft_model::Material>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSessionMissingMaterialsResponse {
    session_id: String,
    revision: u64,
    bundle_path: String,
    project_json_path: String,
    diagnostics: Vec<MissingMaterialCommandDiagnostic>,
}

#[derive(Debug, Default)]
struct ProjectSessionRegistry {
    sessions: HashMap<String, ProjectSession>,
    next_session_id: u64,
}

#[derive(Debug)]
struct ProjectSession {
    session_id: String,
    revision: u64,
    bundle_path: PathBuf,
    project_json_path: PathBuf,
    draft: Draft,
    command_state: CommandState,
    selection: TimelineSelection,
}

pub fn create_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<CreateProjectSessionRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid createProjectSession payload: {error}"),
                Some("createProjectSession".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.create_session(request))
}

pub fn open_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<OpenProjectSessionRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid openProjectSession payload: {error}"),
                Some("openProjectSession".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.open_session(request))
}

pub fn close_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<ProjectSessionRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid closeProjectSession payload: {error}"),
                Some("closeProjectSession".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.close_session(request))
}

pub fn execute_project_intent(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<ExecuteProjectIntentRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid executeProjectIntent payload: {error}"),
                Some("executeProjectIntent".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.execute_intent(request))
}

pub fn list_project_session_materials(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<ProjectSessionReadRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid listProjectSessionMaterials payload: {error}"),
                Some("listProjectSessionMaterials".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.list_materials(request))
}

pub fn list_project_session_missing_materials(
    request: serde_json::Value,
) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<ProjectSessionReadRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid listProjectSessionMissingMaterials payload: {error}"),
                Some("listProjectSessionMissingMaterials".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.list_missing_materials(request))
}

fn with_project_session_registry(
    f: impl FnOnce(&mut ProjectSessionRegistry) -> Result<serde_json::Value>,
) -> Result<serde_json::Value> {
    static REGISTRY: OnceLock<Mutex<ProjectSessionRegistry>> = OnceLock::new();
    let registry = REGISTRY.get_or_init(|| Mutex::new(ProjectSessionRegistry::default()));
    let mut registry = registry.lock().map_err(|_| {
        napi::Error::from_reason("project session registry lock poisoned".to_string())
    })?;
    f(&mut registry)
}

impl ProjectSessionRegistry {
    fn create_session(
        &mut self,
        request: CreateProjectSessionRequest,
    ) -> Result<serde_json::Value> {
        let fs = StdPlatformFileSystem;
        let bundle_path = PathBuf::from(&request.bundle_path);
        let draft_id = request.draft_id.unwrap_or_else(default_draft_id);
        let draft_name = request
            .draft_name
            .unwrap_or_else(|| default_draft_name(&bundle_path));
        let draft = product_default_draft(draft_id, draft_name);
        let bundle = match create_project_bundle(&fs, &bundle_path, &draft) {
            Ok(bundle) => bundle,
            Err(error) => {
                return project_session_store_error("createProjectSession", error);
            }
        };
        let (bundle_path, project_json_path) = match canonical_project_session_paths(
            "createProjectSession",
            &bundle.bundle_path,
            &bundle.project_json_path,
        ) {
            Ok(paths) => paths,
            Err(error) => {
                return project_session_store_error("createProjectSession", error);
            }
        };
        let session_id = request.session_id.unwrap_or_else(|| self.next_session_id());
        self.close_sessions_for_bundle(&bundle_path, Some(&session_id));
        let session = ProjectSession {
            session_id: session_id.clone(),
            revision: 0,
            bundle_path: bundle_path.clone(),
            project_json_path: project_json_path.clone(),
            draft: bundle.draft.clone(),
            command_state: CommandState::empty(),
            selection: TimelineSelection::empty(),
        };
        self.sessions.insert(session_id.clone(), session);

        crate::to_js_value(crate::ok_envelope(ProjectSessionOpenResponse {
            session_id,
            revision: 0,
            draft: bundle.draft,
            bundle_path: bundle_path.display().to_string(),
            project_json_path: project_json_path.display().to_string(),
            warnings: Vec::new(),
        }))
    }

    fn open_session(&mut self, request: OpenProjectSessionRequest) -> Result<serde_json::Value> {
        let fs = StdPlatformFileSystem;
        let opened = match open_project_bundle(&fs, PathBuf::from(&request.bundle_path)) {
            Ok(opened) => opened,
            Err(error) => {
                return project_session_store_error("openProjectSession", error);
            }
        };
        let (bundle_path, project_json_path) = match canonical_project_session_paths(
            "openProjectSession",
            &opened.bundle.bundle_path,
            &opened.bundle.project_json_path,
        ) {
            Ok(paths) => paths,
            Err(error) => {
                return project_session_store_error("openProjectSession", error);
            }
        };
        let session_id = request.session_id.unwrap_or_else(|| self.next_session_id());
        self.close_sessions_for_bundle(&bundle_path, Some(&session_id));
        let session = ProjectSession {
            session_id: session_id.clone(),
            revision: 0,
            bundle_path: bundle_path.clone(),
            project_json_path: project_json_path.clone(),
            draft: opened.bundle.draft.clone(),
            command_state: CommandState::empty(),
            selection: TimelineSelection::empty(),
        };
        self.sessions.insert(session_id.clone(), session);

        crate::to_js_value(crate::ok_envelope(ProjectSessionOpenResponse {
            session_id,
            revision: 0,
            draft: opened.bundle.draft,
            bundle_path: bundle_path.display().to_string(),
            project_json_path: project_json_path.display().to_string(),
            warnings: opened
                .warnings
                .into_iter()
                .map(crate::project_store_warning_message)
                .collect(),
        }))
    }

    fn close_session(&mut self, request: ProjectSessionRequest) -> Result<serde_json::Value> {
        let closed = self.sessions.remove(&request.session_id).is_some();
        crate::to_js_value(crate::ok_envelope(ProjectSessionClosedResponse {
            session_id: request.session_id,
            closed,
        }))
    }

    fn list_materials(&self, request: ProjectSessionReadRequest) -> Result<serde_json::Value> {
        let Some(session) = self.sessions.get(&request.session_id) else {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidProject,
                format!("Project session not found: {}", request.session_id),
                Some("listProjectSessionMaterials".to_string()),
            ));
        };
        if request.expected_revision != session.revision {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!(
                    "Stale project session revision: expected {}, current {}",
                    request.expected_revision, session.revision
                ),
                Some("listProjectSessionMaterials".to_string()),
            ));
        }

        crate::to_js_value(crate::ok_envelope(ProjectSessionMaterialsResponse {
            session_id: session.session_id.clone(),
            revision: session.revision,
            bundle_path: session.bundle_path.display().to_string(),
            project_json_path: session.project_json_path.display().to_string(),
            materials: crate::material_service::list_materials(&session.draft),
        }))
    }

    fn list_missing_materials(
        &self,
        request: ProjectSessionReadRequest,
    ) -> Result<serde_json::Value> {
        let Some(session) = self.sessions.get(&request.session_id) else {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidProject,
                format!("Project session not found: {}", request.session_id),
                Some("listProjectSessionMissingMaterials".to_string()),
            ));
        };
        if request.expected_revision != session.revision {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!(
                    "Stale project session revision: expected {}, current {}",
                    request.expected_revision, session.revision
                ),
                Some("listProjectSessionMissingMaterials".to_string()),
            ));
        }

        let fs = StdPlatformFileSystem;
        match crate::material_service::list_missing_materials(
            &session.draft,
            &fs,
            &session.bundle_path,
        ) {
            Ok(diagnostics) => {
                crate::to_js_value(crate::ok_envelope(ProjectSessionMissingMaterialsResponse {
                    session_id: session.session_id.clone(),
                    revision: session.revision,
                    bundle_path: session.bundle_path.display().to_string(),
                    project_json_path: session.project_json_path.display().to_string(),
                    diagnostics: diagnostics
                        .into_iter()
                        .map(crate::command_diagnostic)
                        .collect(),
                }))
            }
            Err(error) => crate::to_js_value(crate::material_service_error_envelope(
                "listProjectSessionMissingMaterials",
                error,
            )),
        }
    }

    fn execute_intent(
        &mut self,
        request: ExecuteProjectIntentRequest,
    ) -> Result<serde_json::Value> {
        let Some(session) = self.sessions.get_mut(&request.session_id) else {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidProject,
                format!("Project session not found: {}", request.session_id),
                Some("executeProjectIntent".to_string()),
            ));
        };
        if request.expected_revision != session.revision {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!(
                    "Stale project session revision: expected {}, current {}",
                    request.expected_revision, session.revision
                ),
                Some("executeProjectIntent".to_string()),
            ));
        }

        match request.intent {
            ProjectIntent::ImportMaterial {
                material_path,
                material_id,
                display_name,
                material_kind_hint,
            } => session.import_material(
                material_path,
                material_id,
                display_name,
                material_kind_hint,
            ),
            intent => {
                let payload = match session.intent_payload(intent) {
                    Ok(payload) => payload,
                    Err(message) => {
                        return crate::to_js_value(crate::error_envelope(
                            CommandErrorKind::InvalidTimelineEdit,
                            message,
                            Some("executeProjectIntent".to_string()),
                        ));
                    }
                };
                let response = match draft_commands::timeline::execute_timeline_edit(payload) {
                    Ok(response) => response,
                    Err(error) => {
                        return crate::to_js_value(crate::error_envelope(
                            CommandErrorKind::InvalidTimelineEdit,
                            error.to_string(),
                            Some("executeProjectIntent".to_string()),
                        ));
                    }
                };
                session.apply_response(response)
            }
        }
    }

    fn next_session_id(&mut self) -> String {
        self.next_session_id = self.next_session_id.saturating_add(1);
        format!("project-session-{}", self.next_session_id)
    }

    fn close_sessions_for_bundle(&mut self, bundle_path: &Path, except_session_id: Option<&str>) {
        self.sessions.retain(|session_id, session| {
            if except_session_id == Some(session_id.as_str()) {
                return true;
            }
            session.bundle_path != bundle_path
        });
    }
}

impl ProjectSession {
    fn intent_payload(
        &self,
        intent: ProjectIntent,
    ) -> std::result::Result<TimelineEditPayload, String> {
        match intent {
            ProjectIntent::ImportMaterial { .. } => {
                unreachable!("importMaterial is handled before timeline payload conversion")
            }
            ProjectIntent::AddTimelineSegmentIntent { material_id } => {
                Ok(TimelineEditPayload::AddTimelineSegmentIntent(
                    AddTimelineSegmentIntentCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        material_id,
                    },
                ))
            }
            ProjectIntent::SelectTimelineSegments {
                segment_ids,
                track_ids,
            } => Ok(TimelineEditPayload::SelectTimelineSegments(
                SelectTimelineSegmentsCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    segment_ids,
                    track_ids,
                },
            )),
            ProjectIntent::MoveSelectedSegmentIntent { delta } => {
                Ok(TimelineEditPayload::MoveSelectedSegmentIntent(
                    MoveSelectedSegmentIntentCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        delta,
                    },
                ))
            }
            ProjectIntent::SplitSelectedSegmentIntent { split_at } => {
                Ok(TimelineEditPayload::SplitSelectedSegmentIntent(
                    SplitSelectedSegmentIntentCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        split_at,
                    },
                ))
            }
            ProjectIntent::TrimSelectedSegmentIntent { direction, delta } => {
                Ok(TimelineEditPayload::TrimSelectedSegmentIntent(
                    TrimSelectedSegmentIntentCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        direction,
                        delta,
                    },
                ))
            }
            ProjectIntent::DeleteSelectedSegment {} => Ok(TimelineEditPayload::DeleteSegment(
                DeleteSegmentCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    segment_id: self.selected_segment_id("删除片段")?,
                },
            )),
            ProjectIntent::AddTextSegmentIntent { text, duration } => Ok(
                TimelineEditPayload::AddTextSegmentIntent(AddTextSegmentIntentCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    text,
                    duration,
                }),
            ),
            ProjectIntent::EditSelectedText { text } => Ok(TimelineEditPayload::EditTextSegment(
                EditTextSegmentCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    segment_id: self.selected_segment_id("编辑文字")?,
                    text,
                },
            )),
            ProjectIntent::ImportSubtitleSrtIntent {
                srt_content,
                time_offset,
                style,
                text_box,
                layout_region,
                wrapping,
            } => Ok(TimelineEditPayload::ImportSubtitleSrtIntent(
                ImportSubtitleSrtIntentCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    srt_content,
                    time_offset,
                    style,
                    text_box,
                    layout_region,
                    wrapping,
                },
            )),
            ProjectIntent::AddAudioSegmentIntent {
                material_id,
                duration,
            } => Ok(TimelineEditPayload::AddAudioSegmentIntent(
                AddAudioSegmentIntentCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    material_id,
                    duration,
                },
            )),
            ProjectIntent::SetSelectedSegmentVolume { volume } => Ok(
                TimelineEditPayload::SetSegmentVolume(SetSegmentVolumeCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    segment_id: self.selected_segment_id("调整音量")?,
                    volume,
                }),
            ),
            ProjectIntent::UpdateSelectedSegmentAudio {
                gain_millis,
                pan_balance_millis,
                fade_in_duration,
                fade_out_duration,
                effect_slots,
            } => Ok(TimelineEditPayload::UpdateSegmentAudio(
                UpdateSegmentAudioCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    segment_id: self.selected_segment_id("应用音频")?,
                    gain_millis,
                    pan_balance_millis,
                    fade_in_duration,
                    fade_out_duration,
                    effect_slots,
                },
            )),
            ProjectIntent::AddTrackIntent { track_kind } => Ok(
                TimelineEditPayload::AddTrackIntent(AddTrackIntentCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    track_kind,
                }),
            ),
            ProjectIntent::RenameSelectedTrack { name } => Ok(TimelineEditPayload::RenameTrack(
                RenameTrackCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    track_id: self.selected_track_id("重命名轨道")?,
                    name,
                },
            )),
            ProjectIntent::SetSelectedTrackLock { locked } => Ok(
                TimelineEditPayload::SetTrackLock(SetTrackLockCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    track_id: self.selected_track_id("切换轨道锁定")?,
                    locked,
                }),
            ),
            ProjectIntent::SetSelectedTrackVisibility { visible } => Ok(
                TimelineEditPayload::SetTrackVisibility(SetTrackVisibilityCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    track_id: self.selected_track_id("切换轨道显示")?,
                    visible,
                }),
            ),
            ProjectIntent::SetSelectedTrackMute { muted } => Ok(TimelineEditPayload::SetTrackMute(
                SetTrackMuteCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    track_id: self.selected_track_id("切换轨道静音")?,
                    muted,
                },
            )),
            ProjectIntent::UpdateDraftCanvasConfig { canvas_config } => {
                Ok(TimelineEditPayload::UpdateDraftCanvasConfig(
                    UpdateDraftCanvasConfigCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        canvas_config,
                    },
                ))
            }
            ProjectIntent::UpdateSelectedSegmentVisual { visual } => Ok(
                TimelineEditPayload::UpdateSegmentVisual(UpdateSegmentVisualCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    segment_id: self.selected_segment_id("应用画面")?,
                    visual,
                }),
            ),
            ProjectIntent::SetSelectedSegmentKeyframe {
                property,
                at,
                interpolation,
                easing,
            } => {
                let (segment_id, keyframe) =
                    self.keyframe_for_selected_segment(property, at, interpolation, easing)?;
                Ok(TimelineEditPayload::SetSegmentKeyframe(
                    SetSegmentKeyframeCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_id,
                        keyframe,
                    },
                ))
            }
            ProjectIntent::RemoveSelectedSegmentKeyframe { property, at } => {
                let segment = self.selected_segment("删除关键帧")?;
                let relative_at = relative_keyframe_time(segment, at);
                Ok(TimelineEditPayload::RemoveSegmentKeyframe(
                    RemoveSegmentKeyframeCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_id: segment.segment_id.clone(),
                        property,
                        at: relative_at,
                    },
                ))
            }
            ProjectIntent::UndoTimelineEdit {} => Ok(TimelineEditPayload::UndoTimelineEdit(
                draft_model::UndoTimelineEditCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                },
            )),
            ProjectIntent::RedoTimelineEdit {} => Ok(TimelineEditPayload::RedoTimelineEdit(
                draft_model::RedoTimelineEditCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                },
            )),
        }
    }

    fn selected_segment_id(&self, action: &str) -> std::result::Result<SegmentId, String> {
        let Some(segment_id) = self.selection.segment_ids.first() else {
            return Err(format!("{action}失败：请先选择一个片段"));
        };
        let exists = self.draft.tracks.iter().any(|track| {
            track
                .segments
                .iter()
                .any(|segment| &segment.segment_id == segment_id)
        });
        if exists {
            Ok(segment_id.clone())
        } else {
            Err(format!("{action}失败：选中的片段不存在"))
        }
    }

    fn selected_track_id(&self, action: &str) -> std::result::Result<TrackId, String> {
        let Some(track_id) = self.selection.track_ids.first() else {
            return Err(format!("{action}失败：请先选择一个轨道"));
        };
        if self
            .draft
            .tracks
            .iter()
            .any(|track| &track.track_id == track_id)
        {
            Ok(track_id.clone())
        } else {
            Err(format!("{action}失败：选中的轨道不存在"))
        }
    }

    fn selected_segment(&self, action: &str) -> std::result::Result<&Segment, String> {
        let Some(segment_id) = self.selection.segment_ids.first() else {
            return Err(format!("{action}失败：请先选择一个片段"));
        };
        self.draft
            .tracks
            .iter()
            .flat_map(|track| track.segments.iter())
            .find(|segment| &segment.segment_id == segment_id)
            .ok_or_else(|| format!("{action}失败：选中的片段不存在"))
    }

    fn keyframe_for_selected_segment(
        &self,
        property: KeyframeProperty,
        timeline_at: Microseconds,
        interpolation: KeyframeInterpolation,
        easing: KeyframeEasing,
    ) -> std::result::Result<(SegmentId, Keyframe), String> {
        let segment = self.selected_segment("设置关键帧")?;
        let at = relative_keyframe_time(segment, timeline_at);
        let value = keyframe_value_for_segment(segment, &property)?;
        Ok((
            segment.segment_id.clone(),
            Keyframe {
                at,
                property,
                value,
                interpolation,
                easing,
            },
        ))
    }

    fn import_material(
        &mut self,
        material_path: String,
        material_id: Option<MaterialId>,
        display_name: Option<String>,
        material_kind_hint: Option<MaterialKind>,
    ) -> Result<serde_json::Value> {
        let runtime = match discover_runtime_config() {
            Ok(runtime) => runtime,
            Err(error) => {
                return crate::to_js_value(crate::error_envelope(
                    CommandErrorKind::MaterialProbeFailed,
                    crate::runtime_discovery_message(error),
                    Some("executeProjectIntent".to_string()),
                ));
            }
        };
        let fs = StdPlatformFileSystem;
        let executor = DesktopFfmpegExecutor::default();
        let mut request =
            crate::material_service::ImportMaterialRequest::new(PathBuf::from(material_path));
        if let Some(material_id) = material_id {
            request = request.with_material_id(material_id);
        }
        if let Some(display_name) = display_name {
            request = request.with_display_name(display_name);
        }
        if let Some(kind) = material_kind_hint {
            request = request.with_material_kind_hint(kind);
        }

        let mut next_draft = self.draft.clone();
        let imported = match crate::material_service::import_material_and_save(
            &mut next_draft,
            request,
            &fs,
            &executor,
            &runtime,
            &self.bundle_path,
        ) {
            Ok(imported) => imported,
            Err(error) => {
                return crate::to_js_value(crate::material_service_error_envelope(
                    "executeProjectIntent",
                    error,
                ));
            }
        };

        self.revision = self.revision.saturating_add(1);
        self.draft = imported.draft;
        self.bundle_path = imported.bundle_path;
        self.project_json_path = imported.project_json_path;

        crate::to_js_value(crate::ok_envelope(ProjectSessionImportMaterialResponse {
            session_id: self.session_id.clone(),
            revision: self.revision,
            draft: self.draft.clone(),
            material: imported.material,
            diagnostic: imported.diagnostic.map(crate::command_diagnostic),
            bundle_path: self.bundle_path.display().to_string(),
            project_json_path: self.project_json_path.display().to_string(),
        }))
    }

    fn apply_response(&mut self, response: TimelineCommandResponse) -> Result<serde_json::Value> {
        if is_selection_only_delta(&response.delta) {
            self.command_state = response.command_state;
            self.selection = response.selection;
            return crate::to_js_value(crate::ok_envelope(ProjectSessionIntentResponse {
                session_id: self.session_id.clone(),
                revision: self.revision,
                draft: self.draft.clone(),
                command_state: self.command_state.clone(),
                selection: self.selection.clone(),
                events: response.events,
                delta: response.delta,
                bundle_path: self.bundle_path.display().to_string(),
                project_json_path: self.project_json_path.display().to_string(),
            }));
        }

        let fs = StdPlatformFileSystem;
        let saved = match save_project_bundle(&fs, &self.bundle_path, &response.draft) {
            Ok(saved) => saved,
            Err(error) => {
                return project_session_store_error("executeProjectIntent", error);
            }
        };
        self.revision = self.revision.saturating_add(1);
        self.draft = saved.draft;
        self.bundle_path = saved.bundle_path;
        self.project_json_path = saved.project_json_path;
        self.command_state = response.command_state;
        self.selection = response.selection;

        crate::to_js_value(crate::ok_envelope(ProjectSessionIntentResponse {
            session_id: self.session_id.clone(),
            revision: self.revision,
            draft: self.draft.clone(),
            command_state: self.command_state.clone(),
            selection: self.selection.clone(),
            events: response.events,
            delta: response.delta,
            bundle_path: self.bundle_path.display().to_string(),
            project_json_path: self.project_json_path.display().to_string(),
        }))
    }
}

fn relative_keyframe_time(segment: &Segment, timeline_at: Microseconds) -> Microseconds {
    let segment_start = segment.target_timerange.start.get();
    let segment_duration = segment.target_timerange.duration.get();
    let relative = timeline_at.get().saturating_sub(segment_start);
    Microseconds::new(relative.min(segment_duration))
}

fn keyframe_value_for_segment(
    segment: &Segment,
    property: &KeyframeProperty,
) -> std::result::Result<KeyframeValue, String> {
    match property {
        KeyframeProperty::VisualPositionX => Ok(KeyframeValue::Int {
            value: segment.visual.transform.position.x,
        }),
        KeyframeProperty::VisualPositionY => Ok(KeyframeValue::Int {
            value: segment.visual.transform.position.y,
        }),
        KeyframeProperty::VisualScaleX => Ok(KeyframeValue::Uint {
            value: segment.visual.transform.scale.x_millis,
        }),
        KeyframeProperty::VisualScaleY => Ok(KeyframeValue::Uint {
            value: segment.visual.transform.scale.y_millis,
        }),
        KeyframeProperty::VisualRotation => Ok(KeyframeValue::Int {
            value: segment.visual.transform.rotation.degrees,
        }),
        KeyframeProperty::VisualOpacity => Ok(KeyframeValue::Uint {
            value: segment.visual.transform.opacity.value_millis,
        }),
        KeyframeProperty::Volume => Ok(KeyframeValue::Uint {
            value: segment.volume.level_millis,
        }),
        KeyframeProperty::TextFontSize => Ok(KeyframeValue::Uint {
            value: text_for_keyframe(segment, property)?.style.font_size,
        }),
        KeyframeProperty::TextColor => Ok(KeyframeValue::Color {
            value: text_for_keyframe(segment, property)?.style.color.clone(),
        }),
        KeyframeProperty::TextLineHeight => Ok(KeyframeValue::Uint {
            value: text_for_keyframe(segment, property)?
                .style
                .line_height_millis,
        }),
        KeyframeProperty::TextLetterSpacing => Ok(KeyframeValue::Uint {
            value: text_for_keyframe(segment, property)?
                .style
                .letter_spacing_millis,
        }),
        KeyframeProperty::TextLayoutX => Ok(KeyframeValue::Uint {
            value: text_for_keyframe(segment, property)?.layout_region.x_millis,
        }),
        KeyframeProperty::TextLayoutY => Ok(KeyframeValue::Uint {
            value: text_for_keyframe(segment, property)?.layout_region.y_millis,
        }),
        KeyframeProperty::TextLayoutWidth => Ok(KeyframeValue::Uint {
            value: text_for_keyframe(segment, property)?
                .layout_region
                .width_millis,
        }),
        KeyframeProperty::TextLayoutHeight => Ok(KeyframeValue::Uint {
            value: text_for_keyframe(segment, property)?
                .layout_region
                .height_millis,
        }),
        KeyframeProperty::StickerPositionX
        | KeyframeProperty::StickerPositionY
        | KeyframeProperty::StickerScaleX
        | KeyframeProperty::StickerScaleY
        | KeyframeProperty::FilterParameterUnsupported => {
            Err("当前阶段暂不支持该参数动画".to_string())
        }
    }
}

fn text_for_keyframe<'a>(
    segment: &'a Segment,
    property: &KeyframeProperty,
) -> std::result::Result<&'a TextSegment, String> {
    segment
        .text
        .as_ref()
        .ok_or_else(|| format!("当前片段没有可用于 {property:?} 的文字参数"))
}

fn canonical_project_session_paths(
    command: &str,
    bundle_path: &Path,
    project_json_path: &Path,
) -> std::result::Result<(PathBuf, PathBuf), ProjectStoreError> {
    let canonical_bundle_path =
        std::fs::canonicalize(bundle_path).map_err(|source| ProjectStoreError::Io {
            path: bundle_path.to_path_buf(),
            source,
        })?;
    let canonical_project_json_path =
        std::fs::canonicalize(project_json_path).map_err(|source| ProjectStoreError::Io {
            path: project_json_path.to_path_buf(),
            source,
        })?;
    if canonical_project_json_path.parent() != Some(canonical_bundle_path.as_path()) {
        return Err(ProjectStoreError::InvalidProjectJson {
            path: canonical_project_json_path,
            message: format!(
                "{command} resolved project.json outside the canonical project bundle"
            ),
        });
    }
    Ok((canonical_bundle_path, canonical_project_json_path))
}

fn project_session_store_error(
    command: &str,
    error: ProjectStoreError,
) -> Result<serde_json::Value> {
    crate::to_js_value(crate::project_store_error_envelope(command, error))
}

fn is_selection_only_delta(delta: &CommandDelta) -> bool {
    delta.changed_entities.is_empty()
        && delta.changed_domains.is_empty()
        && delta.changed_ranges.is_empty()
        && !delta.invalidation.full_draft
        && delta.invalidation.material_ids.is_empty()
        && delta.invalidation.graph_node_ids.is_empty()
        && delta.invalidation.consumer_domains.is_empty()
}

fn default_draft_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("draft-{millis}")
}

fn default_draft_name(bundle_path: &std::path::Path) -> String {
    bundle_path
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("未命名项目")
        .to_string()
}

fn product_default_draft(
    draft_id: impl Into<draft_model::DraftId>,
    name: impl Into<String>,
) -> Draft {
    let mut draft = Draft::new(draft_id, name);
    draft.tracks = vec![
        Track::new("track-main-video", TrackKind::Video, "视频轨道 1"),
        Track::new("track-bgm", TrackKind::Audio, "音频轨道 1"),
        Track::new("track-title", TrackKind::Text, "文字轨道 1"),
    ];
    draft
}

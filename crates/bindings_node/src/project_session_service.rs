use draft_commands::delta::material_dependency_delta;
use draft_model::{
    AddAudioSegmentIntentCommandPayload, AddTextSegmentIntentCommandPayload,
    AddTimelineSegmentIntentCommandPayload, AddTrackIntentCommandPayload, AudioEffectSlot,
    AudioFade, AudioPanBalance, CommandDelta, CommandDeltaName, CommandErrorKind, CommandState,
    DeleteSegmentCommandPayload, Draft, DraftCanvasConfig, EditTextSegmentCommandPayload,
    ImportSubtitleSrtIntentCommandPayload, Keyframe, KeyframeEasing, KeyframeInterpolation,
    KeyframeProperty, KeyframeValue, MainTrackMagnet, Material, MaterialId, MaterialKind,
    MaterialStatus, Microseconds, MissingMaterialCommandDiagnostic,
    MoveSelectedSegmentIntentCommandPayload, RationalFrameRate,
    RemoveSegmentKeyframeCommandPayload, RenameTrackCommandPayload, Segment, SegmentAudio,
    SegmentId, SegmentVisual, SegmentVolume, SelectTimelineSegmentsCommandPayload,
    SetSegmentKeyframeCommandPayload, SetSegmentVolumeCommandPayload, SetTrackLockCommandPayload,
    SetTrackMuteCommandPayload, SetTrackVisibilityCommandPayload, SourceTimerange,
    SplitSelectedSegmentIntentCommandPayload, TargetTimerange, TextBox, TextLayoutRegion,
    TextSegment, TextStyle, TextWrapping, TimelineCommandResponse, TimelineEditPayload,
    TimelineSelection, Track, TrackId, TrackKind, TrimSegmentDirection,
    TrimSelectedSegmentIntentCommandPayload, UpdateDraftCanvasConfigCommandPayload,
    UpdateSegmentAudioCommandPayload, UpdateSegmentVisualCommandPayload,
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
    #[serde(default)]
    fixture: Option<ProjectSessionFixture>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ProjectSessionFixture {
    Demo,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSessionOpenResponse {
    session_id: String,
    revision: u64,
    #[serde(rename = "viewModel")]
    view_model: ProjectSessionViewModel,
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
    SelectTimelineItemIntent {
        #[serde(rename = "itemHandle")]
        item_handle: String,
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
    #[serde(rename = "viewModel")]
    view_model: ProjectSessionViewModel,
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
    material: draft_model::Material,
    materials: Vec<draft_model::Material>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    diagnostic: Option<MissingMaterialCommandDiagnostic>,
    #[serde(rename = "viewModel")]
    view_model: ProjectSessionViewModel,
    events: Vec<draft_model::CommandEvent>,
    delta: draft_model::CommandDelta,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSessionViewModel {
    project: ProjectSummaryViewModel,
    edit_controls: EditControlsViewModel,
    timeline: TimelineViewModel,
    selected_track: Option<SelectedTrackViewModel>,
    selected_segment: Option<SelectedSegmentViewModel>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EditControlsViewModel {
    can_undo: bool,
    can_redo: bool,
    snapping_enabled: bool,
    snapping_label: String,
    has_selected_segment: bool,
    has_selected_track: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSummaryViewModel {
    draft_name: String,
    canvas_config: DraftCanvasConfig,
    sequence_duration: Microseconds,
    frame_duration: Microseconds,
    track_count: usize,
    material_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectedTrackViewModel {
    track_id: TrackId,
    selection_handle: String,
    name: String,
    kind_label: String,
    muted: bool,
    locked: bool,
    visible: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectedSegmentViewModel {
    segment_key: String,
    selection_handle: String,
    track: SelectedTrackViewModel,
    material: Option<Material>,
    source_timerange: SourceTimerange,
    target_timerange: TargetTimerange,
    source_label: String,
    target_label: String,
    visual: SegmentVisual,
    volume: SegmentVolume,
    audio: SegmentAudio,
    text: Option<TextSegment>,
    keyframes: Vec<Keyframe>,
    has_text: bool,
    has_audio_controls: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelineViewModel {
    rows: Vec<TimelineTrackRowViewModel>,
    duration: Microseconds,
    ruler_ticks: Vec<Microseconds>,
    capabilities: TimelineCapabilitiesViewModel,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelineCapabilitiesViewModel {
    has_text_track: bool,
    has_audio_track: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelineTrackRowViewModel {
    row_key: String,
    selection_handle: String,
    name: String,
    symbol: String,
    kind_label: String,
    status_label: String,
    lock_label: String,
    visibility_label: String,
    mute_label: String,
    row_class_name: String,
    selected: bool,
    lock_active: bool,
    visibility_active: bool,
    mute_active: bool,
    can_toggle_visibility: bool,
    can_toggle_mute: bool,
    next_locked: bool,
    next_visible: bool,
    next_muted: bool,
    visibility_symbol: String,
    segments: Vec<TimelineSegmentViewModel>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelineSegmentViewModel {
    segment_key: String,
    selection_handle: String,
    waveform_material_id: Option<MaterialId>,
    material: Option<Material>,
    label: String,
    source_label: String,
    target_label: String,
    visual_kind: TimelineSegmentVisualKind,
    start: Microseconds,
    duration: Microseconds,
    selected: bool,
    keyframe_markers: Vec<TimelineKeyframeMarkerViewModel>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelineKeyframeMarkerViewModel {
    marker_key: String,
    position_per_mille: u32,
    title: String,
    aria_label: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum TimelineSegmentVisualKind {
    Video,
    Image,
    Audio,
    Text,
    Sticker,
    Filter,
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

#[derive(Debug, Clone)]
pub(crate) struct ProjectSessionPreviewSnapshot {
    pub draft: Draft,
    pub bundle_path: PathBuf,
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

pub(crate) fn realtime_preview_snapshot(
    session_id: &str,
    expected_revision: u64,
) -> std::result::Result<ProjectSessionPreviewSnapshot, String> {
    project_session_snapshot(session_id, expected_revision)
}

pub(crate) fn project_session_snapshot(
    session_id: &str,
    expected_revision: u64,
) -> std::result::Result<ProjectSessionPreviewSnapshot, String> {
    let registry = global_project_session_registry();
    let registry = registry
        .lock()
        .map_err(|_| "project session registry lock poisoned".to_string())?;
    let session = registry
        .sessions
        .get(session_id)
        .ok_or_else(|| format!("Project session not found: {session_id}"))?;
    if expected_revision != session.revision {
        return Err(format!(
            "Stale project session revision: expected {}, current {}",
            expected_revision, session.revision
        ));
    }
    Ok(ProjectSessionPreviewSnapshot {
        draft: session.draft.clone(),
        bundle_path: session.bundle_path.clone(),
    })
}

fn global_project_session_registry() -> &'static Mutex<ProjectSessionRegistry> {
    static REGISTRY: OnceLock<Mutex<ProjectSessionRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(ProjectSessionRegistry::default()))
}

fn with_project_session_registry(
    f: impl FnOnce(&mut ProjectSessionRegistry) -> Result<serde_json::Value>,
) -> Result<serde_json::Value> {
    let mut registry = global_project_session_registry().lock().map_err(|_| {
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
        let draft = match request.fixture {
            Some(ProjectSessionFixture::Demo) => product_demo_fixture_draft(),
            None => product_default_draft(draft_id, draft_name),
        };
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
            view_model: project_session_view_model(
                &bundle.draft,
                &CommandState::empty(),
                &TimelineSelection::empty(),
            ),
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
            view_model: project_session_view_model(
                &opened.bundle.draft,
                &CommandState::empty(),
                &TimelineSelection::empty(),
            ),
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
            ProjectIntent::SelectTimelineItemIntent { item_handle } => {
                let selection = self.selection_from_timeline_item_handle(&item_handle)?;
                Ok(TimelineEditPayload::SelectTimelineSegments(
                    SelectTimelineSegmentsCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_ids: selection.segment_ids,
                        track_ids: selection.track_ids,
                    },
                ))
            }
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

    fn selection_from_timeline_item_handle(
        &self,
        item_handle: &str,
    ) -> std::result::Result<TimelineSelection, String> {
        if let Some(encoded_track_id) = item_handle.strip_prefix("timeline-track:") {
            let raw_track_id = percent_decode_timeline_handle_component(encoded_track_id)
                .map_err(|_| "Invalid timeline track selection handle".to_owned())?;
            if raw_track_id.trim().is_empty() {
                return Err("Invalid timeline track selection handle".to_owned());
            }
            let track_id = TrackId::new(raw_track_id);
            self.draft
                .tracks
                .iter()
                .find(|track| track.track_id == track_id)
                .ok_or_else(|| "Timeline track selection handle no longer resolves".to_owned())?;
            return Ok(TimelineSelection {
                segment_ids: Vec::new(),
                track_ids: vec![track_id],
            });
        }

        if let Some(raw) = item_handle.strip_prefix("timeline-segment:") {
            let Some((encoded_track_id, encoded_segment_id)) = raw.split_once(':') else {
                return Err("Invalid timeline segment selection handle".to_owned());
            };
            if encoded_segment_id.contains(':') {
                return Err("Invalid timeline segment selection handle".to_owned());
            }
            let raw_track_id = percent_decode_timeline_handle_component(encoded_track_id)
                .map_err(|_| "Invalid timeline segment selection handle".to_owned())?;
            let raw_segment_id = percent_decode_timeline_handle_component(encoded_segment_id)
                .map_err(|_| "Invalid timeline segment selection handle".to_owned())?;
            if raw_track_id.trim().is_empty() || raw_segment_id.trim().is_empty() {
                return Err("Invalid timeline segment selection handle".to_owned());
            }
            let track_id = TrackId::new(raw_track_id);
            let segment_id = SegmentId::new(raw_segment_id);
            let track = self
                .draft
                .tracks
                .iter()
                .find(|track| track.track_id == track_id)
                .ok_or_else(|| "Timeline segment selection track no longer resolves".to_owned())?;
            track
                .segments
                .iter()
                .find(|segment| segment.segment_id == segment_id)
                .ok_or_else(|| "Timeline segment selection handle no longer resolves".to_owned())?;
            return Ok(TimelineSelection {
                segment_ids: vec![segment_id],
                track_ids: vec![track_id],
            });
        }

        Err("Unknown timeline item selection handle".to_owned())
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
        let material = imported.material;
        let delta = material_dependency_delta(
            CommandDeltaName::ImportMaterial,
            &self.draft,
            std::slice::from_ref(&material.material_id),
            "material imported",
        );

        crate::to_js_value(crate::ok_envelope(ProjectSessionImportMaterialResponse {
            session_id: self.session_id.clone(),
            revision: self.revision,
            material,
            materials: crate::material_service::list_materials(&self.draft),
            diagnostic: imported.diagnostic.map(crate::command_diagnostic),
            view_model: project_session_view_model(
                &self.draft,
                &self.command_state,
                &self.selection,
            ),
            events: Vec::new(),
            delta,
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
                view_model: project_session_view_model(
                    &self.draft,
                    &self.command_state,
                    &self.selection,
                ),
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
            view_model: project_session_view_model(
                &self.draft,
                &self.command_state,
                &self.selection,
            ),
            events: response.events,
            delta: response.delta,
            bundle_path: self.bundle_path.display().to_string(),
            project_json_path: self.project_json_path.display().to_string(),
        }))
    }
}

fn project_session_view_model(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
) -> ProjectSessionViewModel {
    let project = project_summary_view_model(draft);
    let edit_controls = edit_controls_view_model(command_state, selection);
    let timeline = timeline_view_model(draft, selection);
    let selected_track = selected_track_view_model(draft, selection);
    let selected_segment = selected_segment_view_model(draft, selection);

    ProjectSessionViewModel {
        project,
        edit_controls,
        timeline,
        selected_track,
        selected_segment,
    }
}

fn edit_controls_view_model(
    command_state: &CommandState,
    selection: &TimelineSelection,
) -> EditControlsViewModel {
    EditControlsViewModel {
        can_undo: !command_state.undo_stack.is_empty(),
        can_redo: !command_state.redo_stack.is_empty(),
        snapping_enabled: command_state.snapping.enabled,
        snapping_label: if command_state.snapping.enabled {
            "吸附 开"
        } else {
            "吸附 关"
        }
        .to_owned(),
        has_selected_segment: !selection.segment_ids.is_empty(),
        has_selected_track: !selection.track_ids.is_empty(),
    }
}

fn project_summary_view_model(draft: &Draft) -> ProjectSummaryViewModel {
    ProjectSummaryViewModel {
        draft_name: draft.metadata.name.clone(),
        canvas_config: draft.canvas_config.clone(),
        sequence_duration: Microseconds::new(sequence_duration(draft)),
        frame_duration: Microseconds::new(frame_duration_us(&draft.canvas_config)),
        track_count: draft.tracks.len(),
        material_count: draft.materials.len(),
    }
}

fn selected_track_view_model(
    draft: &Draft,
    selection: &TimelineSelection,
) -> Option<SelectedTrackViewModel> {
    let selected_track_id = selection.track_ids.first();
    let selected_segment_id = selection.segment_ids.first();
    let track = draft
        .tracks
        .iter()
        .find(|track| Some(&track.track_id) == selected_track_id)
        .or_else(|| {
            draft.tracks.iter().find(|track| {
                track
                    .segments
                    .iter()
                    .any(|segment| Some(&segment.segment_id) == selected_segment_id)
            })
        })?;

    Some(selected_track_view_model_from_track(track))
}

fn selected_segment_view_model(
    draft: &Draft,
    selection: &TimelineSelection,
) -> Option<SelectedSegmentViewModel> {
    let selected_segment_id = selection.segment_ids.first()?;

    for track in &draft.tracks {
        if let Some(segment) = track
            .segments
            .iter()
            .find(|candidate| &candidate.segment_id == selected_segment_id)
        {
            let material = draft
                .materials
                .iter()
                .find(|candidate| candidate.material_id == segment.material_id)
                .cloned();
            let source_start = format_timeline_time(segment.source_timerange.start.get());
            let source_duration = format_timeline_time(segment.source_timerange.duration.get());
            let target_start = format_timeline_time(segment.target_timerange.start.get());
            let target_duration = format_timeline_time(segment.target_timerange.duration.get());
            let has_text = segment.text.is_some();
            let has_audio_controls = track.kind == TrackKind::Audio
                || material
                    .as_ref()
                    .is_some_and(|material| match material.kind {
                        MaterialKind::Audio => true,
                        MaterialKind::Video => material.metadata.has_audio,
                        _ => false,
                    });
            return Some(SelectedSegmentViewModel {
                segment_key: segment.segment_id.as_str().to_owned(),
                selection_handle: timeline_segment_selection_handle(
                    &track.track_id,
                    &segment.segment_id,
                ),
                track: selected_track_view_model_from_track(track),
                material,
                source_timerange: segment.source_timerange.clone(),
                target_timerange: segment.target_timerange.clone(),
                source_label: format!("源 {source_start} / {source_duration}"),
                target_label: format!("目标 {target_start} / {target_duration}"),
                visual: segment.visual.clone(),
                volume: segment.volume,
                audio: segment.audio.clone(),
                text: segment.text.clone(),
                keyframes: segment.keyframes.clone(),
                has_text,
                has_audio_controls,
            });
        }
    }

    None
}

fn selected_track_view_model_from_track(track: &Track) -> SelectedTrackViewModel {
    SelectedTrackViewModel {
        track_id: track.track_id.clone(),
        selection_handle: timeline_track_selection_handle(&track.track_id),
        name: track.name.clone(),
        kind_label: format_track_kind(track.kind).to_owned(),
        muted: track.muted,
        locked: track.locked,
        visible: track.visible,
    }
}

fn timeline_view_model(draft: &Draft, selection: &TimelineSelection) -> TimelineViewModel {
    let duration = timeline_duration(draft);
    let rows = draft
        .tracks
        .iter()
        .map(|track| timeline_track_row_view_model(draft, selection, track))
        .collect();

    TimelineViewModel {
        rows,
        duration: Microseconds::new(duration),
        ruler_ticks: build_ruler_ticks(duration)
            .into_iter()
            .map(Microseconds::new)
            .collect(),
        capabilities: timeline_capabilities_view_model(draft),
    }
}

fn timeline_capabilities_view_model(draft: &Draft) -> TimelineCapabilitiesViewModel {
    TimelineCapabilitiesViewModel {
        has_text_track: draft
            .tracks
            .iter()
            .any(|track| track.kind == TrackKind::Text),
        has_audio_track: draft
            .tracks
            .iter()
            .any(|track| track.kind == TrackKind::Audio),
    }
}

fn timeline_track_row_view_model(
    draft: &Draft,
    selection: &TimelineSelection,
    track: &Track,
) -> TimelineTrackRowViewModel {
    let kind_label = format_track_kind(track.kind);
    let selection_handle = timeline_track_selection_handle(&track.track_id);
    let selected = selection.track_ids.contains(&track.track_id);
    let can_toggle_visibility = track.kind != TrackKind::Audio;
    let can_toggle_mute = track.kind == TrackKind::Audio;
    let segments = track
        .segments
        .iter()
        .map(|segment| timeline_segment_view_model(draft, selection, track, segment))
        .collect();

    TimelineTrackRowViewModel {
        row_key: selection_handle.clone(),
        selection_handle,
        name: track.name.clone(),
        symbol: timeline_track_symbol(track.kind).to_owned(),
        kind_label: kind_label.to_owned(),
        status_label: format!("{kind_label} · {} 片段", track.segments.len()),
        lock_label: if track.locked {
            "已锁定"
        } else {
            "未锁定"
        }
        .to_owned(),
        visibility_label: if track.kind == TrackKind::Audio {
            "听觉开启".to_owned()
        } else if track.visible {
            "画面可见".to_owned()
        } else {
            "画面隐藏".to_owned()
        },
        mute_label: if track.muted {
            "已静音"
        } else {
            "未静音"
        }
        .to_owned(),
        row_class_name: timeline_track_row_class_name(track, selection),
        selected,
        lock_active: track.locked,
        visibility_active: if track.kind == TrackKind::Audio {
            !track.muted
        } else {
            track.visible
        },
        mute_active: track.muted,
        can_toggle_visibility,
        can_toggle_mute,
        next_locked: !track.locked,
        next_visible: !track.visible,
        next_muted: !track.muted,
        visibility_symbol: if track.kind == TrackKind::Audio {
            "听"
        } else {
            "眼"
        }
        .to_owned(),
        segments,
    }
}

fn timeline_segment_view_model(
    draft: &Draft,
    selection: &TimelineSelection,
    track: &Track,
    segment: &Segment,
) -> TimelineSegmentViewModel {
    let material = draft
        .materials
        .iter()
        .find(|candidate| candidate.material_id == segment.material_id)
        .cloned();
    let selected = selection.segment_ids.contains(&segment.segment_id);
    let source_start = format_timeline_time(segment.source_timerange.start.get());
    let source_duration = format_timeline_time(segment.source_timerange.duration.get());
    let target_start = format_timeline_time(segment.target_timerange.start.get());
    let target_duration = format_timeline_time(segment.target_timerange.duration.get());
    let label = material
        .as_ref()
        .map(|material| material.display_name.clone())
        .unwrap_or_else(|| format!("片段 {}", segment.segment_id.as_str()));

    TimelineSegmentViewModel {
        segment_key: segment.segment_id.as_str().to_owned(),
        selection_handle: timeline_segment_selection_handle(&track.track_id, &segment.segment_id),
        waveform_material_id: Some(segment.material_id.clone()),
        label: label.clone(),
        source_label: format!("源 {source_start} / {source_duration}"),
        target_label: format!("目标 {target_start} / {target_duration}"),
        visual_kind: timeline_segment_visual_kind(track.kind, material.as_ref()),
        start: segment.target_timerange.start,
        duration: segment.target_timerange.duration,
        selected,
        keyframe_markers: timeline_keyframe_markers(segment, &label),
        material,
    }
}

fn timeline_keyframe_markers(
    segment: &Segment,
    segment_label: &str,
) -> Vec<TimelineKeyframeMarkerViewModel> {
    let duration = segment.target_timerange.duration.get().max(1);
    segment
        .keyframes
        .iter()
        .map(|keyframe| {
            let clamped_at = keyframe.at.get().min(duration);
            let position_per_mille = ((clamped_at.saturating_mul(1_000)) / duration) as u32;
            let property_label = format_keyframe_property(&keyframe.property);
            let time_label = format_timeline_time(keyframe.at.get());
            let easing_label = format_keyframe_easing(keyframe.easing);
            TimelineKeyframeMarkerViewModel {
                marker_key: format!("{:?}-{}", keyframe.property, keyframe.at.get()),
                position_per_mille,
                title: format!("{property_label}关键帧 {time_label} · {easing_label}"),
                aria_label: format!("{segment_label} {property_label}关键帧 {time_label}"),
            }
        })
        .collect()
}

fn timeline_duration(draft: &Draft) -> u64 {
    sequence_duration(draft).max(10_000_000)
}

fn sequence_duration(draft: &Draft) -> u64 {
    draft
        .tracks
        .iter()
        .flat_map(|track| track.segments.iter())
        .fold(0, |duration, segment| {
            duration.max(
                segment
                    .target_timerange
                    .start
                    .get()
                    .saturating_add(segment.target_timerange.duration.get()),
            )
        })
}

fn frame_duration_us(canvas_config: &DraftCanvasConfig) -> u64 {
    let numerator = u64::from(canvas_config.frame_rate.numerator).max(1);
    let denominator = u64::from(canvas_config.frame_rate.denominator).max(1);
    ((denominator * 1_000_000) / numerator).max(1)
}

fn build_ruler_ticks(duration: u64) -> Vec<u64> {
    let tick_count = 6;
    let interval = (duration / tick_count).max(1);
    (0..=tick_count)
        .map(|index| (index as u64).saturating_mul(interval).min(duration))
        .collect()
}

fn timeline_track_row_class_name(track: &Track, selection: &TimelineSelection) -> String {
    let mut classes = vec![
        "track-row".to_owned(),
        track_kind_class(track.kind).to_owned(),
    ];
    if !track.visible {
        classes.push("hidden".to_owned());
    }
    if selection.track_ids.contains(&track.track_id) {
        classes.push("selected-track".to_owned());
    }
    classes.join(" ")
}

fn timeline_segment_visual_kind(
    track_kind: TrackKind,
    material: Option<&Material>,
) -> TimelineSegmentVisualKind {
    match material.map(|material| material.kind) {
        Some(MaterialKind::Video) => TimelineSegmentVisualKind::Video,
        Some(MaterialKind::Image) => TimelineSegmentVisualKind::Image,
        Some(MaterialKind::Audio) => TimelineSegmentVisualKind::Audio,
        Some(MaterialKind::Text) => TimelineSegmentVisualKind::Text,
        _ => match track_kind {
            TrackKind::Video => TimelineSegmentVisualKind::Video,
            TrackKind::Audio => TimelineSegmentVisualKind::Audio,
            TrackKind::Text => TimelineSegmentVisualKind::Text,
            TrackKind::Sticker => TimelineSegmentVisualKind::Sticker,
            TrackKind::Filter => TimelineSegmentVisualKind::Filter,
        },
    }
}

fn format_track_kind(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Video => "视频",
        TrackKind::Audio => "音频",
        TrackKind::Text => "文字",
        TrackKind::Sticker => "贴纸",
        TrackKind::Filter => "滤镜",
    }
}

fn timeline_track_symbol(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Video => "▣",
        TrackKind::Audio => "♪",
        TrackKind::Text => "T",
        TrackKind::Sticker => "◇",
        TrackKind::Filter => "◐",
    }
}

fn format_keyframe_property(property: &KeyframeProperty) -> &'static str {
    match property {
        KeyframeProperty::VisualPositionX => "位置 X",
        KeyframeProperty::VisualPositionY => "位置 Y",
        KeyframeProperty::VisualScaleX => "缩放 X",
        KeyframeProperty::VisualScaleY => "缩放 Y",
        KeyframeProperty::VisualRotation => "旋转",
        KeyframeProperty::VisualOpacity => "不透明度",
        KeyframeProperty::TextFontSize => "字号",
        KeyframeProperty::TextColor => "颜色",
        KeyframeProperty::TextLineHeight => "行高",
        KeyframeProperty::TextLetterSpacing => "字间距",
        KeyframeProperty::TextLayoutX => "布局 X",
        KeyframeProperty::TextLayoutY => "布局 Y",
        KeyframeProperty::TextLayoutWidth => "布局宽",
        KeyframeProperty::TextLayoutHeight => "布局高",
        KeyframeProperty::Volume => "音量",
        KeyframeProperty::StickerPositionX => "贴纸位置 X",
        KeyframeProperty::StickerPositionY => "贴纸位置 Y",
        KeyframeProperty::StickerScaleX => "贴纸缩放 X",
        KeyframeProperty::StickerScaleY => "贴纸缩放 Y",
        KeyframeProperty::FilterParameterUnsupported => "滤镜参数",
    }
}

fn format_keyframe_easing(easing: KeyframeEasing) -> &'static str {
    match easing {
        KeyframeEasing::None => "无",
        KeyframeEasing::EaseIn => "缓入",
        KeyframeEasing::EaseOut => "缓出",
        KeyframeEasing::EaseInOut => "缓入缓出",
    }
}

fn track_kind_class(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Video => "video",
        TrackKind::Audio => "audio",
        TrackKind::Text => "text",
        TrackKind::Sticker => "sticker",
        TrackKind::Filter => "filter",
    }
}

fn format_timeline_time(time_us: u64) -> String {
    let total_millis = time_us / 1_000;
    let millis = total_millis % 1_000;
    let total_seconds = total_millis / 1_000;
    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

fn timeline_track_selection_handle(track_id: &TrackId) -> String {
    format!(
        "timeline-track:{}",
        percent_encode_timeline_handle_component(track_id.as_str())
    )
}

fn timeline_segment_selection_handle(track_id: &TrackId, segment_id: &SegmentId) -> String {
    format!(
        "timeline-segment:{}:{}",
        percent_encode_timeline_handle_component(track_id.as_str()),
        percent_encode_timeline_handle_component(segment_id.as_str())
    )
}

fn percent_encode_timeline_handle_component(raw: &str) -> String {
    let mut encoded = String::with_capacity(raw.len());
    for byte in raw.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn relative_keyframe_time(segment: &Segment, timeline_at: Microseconds) -> Microseconds {
    let segment_start = segment.target_timerange.start.get();
    let segment_duration = segment.target_timerange.duration.get();
    let relative = timeline_at.get().saturating_sub(segment_start);
    Microseconds::new(relative.min(segment_duration))
}

fn percent_decode_timeline_handle_component(encoded: &str) -> std::result::Result<String, ()> {
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }

        if index + 2 >= bytes.len() {
            return Err(());
        }

        let high = percent_hex_nibble(bytes[index + 1]).ok_or(())?;
        let low = percent_hex_nibble(bytes[index + 2]).ok_or(())?;
        decoded.push((high << 4) | low);
        index += 3;
    }

    String::from_utf8(decoded).map_err(|_| ())
}

fn percent_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
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

fn product_demo_fixture_draft() -> Draft {
    let mut draft = product_default_draft("draft-phase-04-workspace", "未命名草稿");
    draft.metadata.description = Some("阶段四桌面工作区展示草稿".to_owned());

    let mut video = Material::new(
        "material-workspace-video",
        MaterialKind::Video,
        "media/workspace-video.mp4",
        "城市街景.mp4",
    );
    video.metadata.duration = Some(Microseconds::new(12_000_000));
    video.metadata.width = Some(1920);
    video.metadata.height = Some(1080);
    video.metadata.frame_rate = Some(RationalFrameRate::new(30, 1));
    video.metadata.has_video = true;
    video.metadata.has_audio = true;
    video.metadata.audio_sample_rate = Some(48_000);
    video.metadata.audio_channels = Some(2);

    let mut audio = Material::new(
        "material-workspace-audio",
        MaterialKind::Audio,
        "media/bgm.wav",
        "背景音乐.wav",
    );
    audio.metadata.duration = Some(Microseconds::new(18_000_000));
    audio.metadata.has_audio = true;
    audio.metadata.audio_sample_rate = Some(44_100);
    audio.metadata.audio_channels = Some(2);

    let mut missing = Material::new(
        "material-workspace-missing",
        MaterialKind::Image,
        "media/missing-cover.png",
        "封面图.png",
    );
    missing.metadata.duration = Some(Microseconds::new(3_000_000));
    missing.metadata.width = Some(1280);
    missing.metadata.height = Some(720);
    missing.metadata.has_video = true;
    missing.status = MaterialStatus::Missing;

    let mut sticker = Material::new(
        "material-workspace-sticker-failed",
        MaterialKind::Sticker,
        "media/sticker.webp",
        "贴纸素材.webp",
    );
    sticker.metadata.has_video = true;
    sticker.metadata.probe_error = Some("无法读取素材头信息".to_owned());
    sticker.status = MaterialStatus::ProbeFailed;

    let text = Material::new(
        "material-workspace-title",
        MaterialKind::Text,
        "text://material-workspace-title",
        "标题文字",
    );

    draft.materials = vec![video, audio, missing, sticker, text];

    let mut main_segment = Segment::new(
        "segment-main-video",
        "material-workspace-video",
        SourceTimerange::new(0, 8_000_000),
        TargetTimerange::new(0, 8_000_000),
    );
    main_segment.main_track_magnet = MainTrackMagnet::enabled();

    let mut audio_segment = Segment::new(
        "segment-bgm",
        "material-workspace-audio",
        SourceTimerange::new(0, 8_000_000),
        TargetTimerange::new(0, 8_000_000),
    );
    audio_segment.volume = SegmentVolume { level_millis: 800 };
    audio_segment.audio = SegmentAudio {
        gain_millis: 800,
        ..SegmentAudio::default()
    };

    draft.tracks = vec![
        {
            let mut track = Track::new("track-main-video", TrackKind::Video, "视频轨道 1");
            track.segments.push(main_segment);
            track
        },
        {
            let mut track = Track::new("track-bgm", TrackKind::Audio, "音频轨道 1");
            track.segments.push(audio_segment);
            track
        },
        Track::new("track-title", TrackKind::Text, "文字轨道 1"),
    ];

    draft
}

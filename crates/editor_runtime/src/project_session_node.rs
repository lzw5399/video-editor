use adapter_kaipai::{KaipaiFormulaBundle, KaipaiImportOptions, map_kaipai_bundle_to_import_plan};
use artifact_store::{
    ArtifactStoreError,
    resource_index::{
        ResourceKind, ResourceRef, index_draft_resources, index_draft_resources_with_extra_refs,
    },
};
use draft_commands::delta::material_dependency_delta;
use draft_import::{
    DraftImportApplicationInput, LocalizedResourceIndexKind, LocalizedResourceManifest,
    LocalizedResourceStatus, apply_import_plan_to_draft,
};
use draft_model::{
    AddAudioSegmentIntentCommandPayload, AddTextSegmentIntentCommandPayload,
    AddTimelineSegmentIntentCommandPayload, AddTrackIntentCommandPayload,
    AddTransitionCommandPayload, ApplySegmentEffectCommandPayload, AudioEffectSlot, AudioFade,
    AudioPanBalance, BlendModeKind, CapabilityReportItem, CapabilitySupport, ChangedEntity,
    CommandDelta, CommandDeltaName, CommandError, CommandErrorKind, CommandEvent,
    CommandResultEnvelope, CommandState, DeleteSegmentCommandPayload, DirtyDomain, Draft,
    DraftCanvasConfig, EditTextSegmentCommandPayload, EffectCapabilityRegistry,
    EffectParameterUpdate, Filter, ImportSubtitleSrtIntentCommandPayload, Keyframe, KeyframeEasing,
    KeyframeInterpolation, KeyframeProperty, KeyframeValue, MainTrackMagnet, Material, MaterialId,
    MaterialKind, MaterialStatus, Microseconds, MissingMaterialCommandDiagnostic,
    MoveSegmentCommandPayload, ProjectInteractionKind, ProjectInteractionSequenceError,
    ProjectInteractionSession as DraftProjectInteractionSession, RationalFrameRate,
    RemoveSegmentEffectCommandPayload, RemoveSegmentKeyframeCommandPayload,
    RemoveTransitionCommandPayload, RenameTrackCommandPayload, Segment, SegmentAudio,
    SegmentBackgroundFilling, SegmentBlendMode, SegmentFitMode, SegmentId, SegmentMask,
    SegmentPosition, SegmentRetiming, SegmentVisual, SegmentVolume,
    SelectTimelineSegmentsCommandPayload, SetSegmentBlendModeCommandPayload,
    SetSegmentKeyframeCommandPayload, SetSegmentMaskCommandPayload, SetSegmentRetimeCommandPayload,
    SetSegmentVolumeCommandPayload, SetTrackLockCommandPayload, SetTrackMuteCommandPayload,
    SetTrackVisibilityCommandPayload, SourceTimerange, SplitSelectedSegmentIntentCommandPayload,
    TargetTimerange, TextAlignment, TextBackground, TextBox, TextLayoutRegion, TextSegment,
    TextSegmentSource, TextShadow, TextStroke, TextStyle, TextWrapping, TimelineCommandResponse,
    TimelineEditPayload, TimelineSelection, Track, TrackId, TrackKind, Transition,
    TransitionReference, TrimSegmentCommandPayload, TrimSegmentDirection,
    UpdateDraftCanvasConfigCommandPayload, UpdateSegmentAudioCommandPayload,
    UpdateSegmentEffectParameterCommandPayload, UpdateSegmentVisualCommandPayload,
    UpdateTransitionDurationCommandPayload,
};
use media_runtime::{DiscoveryError, discover_runtime_config, run_scheduled_material_probe};
use media_runtime_desktop::DesktopFfmpegExecutor;
use project_store::{
    ProjectStoreError, ProjectStoreWarning, StdPlatformFileSystem, create_project_bundle,
    open_project_bundle, project_io_scheduler_envelope, save_project_bundle,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use task_runtime::{
    CompletionFreshness, JobDomain, JobEnvelope, JobFreshness, JobId, JobPriority, JobResult,
    PlaybackGeneration, ResourceClass, SchedulerTelemetrySnapshot, TaskCancellationToken,
    TaskRuntimeConfig,
};

use crate::timeline_selection::{
    percent_decode_timeline_handle_component, timeline_segment_selection_handle,
    timeline_track_selection_handle,
};
use crate::{RuntimeError, RuntimeErrorKind};

type Result<T> = std::result::Result<T, RuntimeError>;

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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BeginProjectInteractionRequest {
    session_id: String,
    expected_revision: u64,
    kind: ProjectInteractionKind,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateProjectInteractionRequest {
    session_id: String,
    expected_revision: u64,
    interaction_id: String,
    sequence: u64,
    payload: ProjectInteractionPayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CommitProjectInteractionRequest {
    session_id: String,
    expected_revision: u64,
    interaction_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CancelProjectInteractionRequest {
    session_id: String,
    expected_revision: u64,
    interaction_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportKaipaiFormulaBundleRequest {
    session_id: String,
    expected_revision: u64,
    bundle_path: String,
    resource_root: String,
    #[serde(default)]
    import_id: Option<String>,
    #[serde(default)]
    generated_at: Option<String>,
    #[serde(default)]
    verify_resource_sha256: Option<bool>,
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
        #[serde(default, rename = "targetStart")]
        target_start: Option<Microseconds>,
        #[serde(default, rename = "targetTrackHandle")]
        target_track_handle: Option<String>,
    },
    SelectTimelineItemIntent {
        #[serde(rename = "itemHandle")]
        item_handle: String,
    },
    MoveSelectedSegmentIntent {
        #[serde(rename = "startAt")]
        start_at: Microseconds,
        #[serde(default, rename = "targetTrackHandle")]
        target_track_handle: Option<String>,
    },
    SplitSelectedSegmentIntent {},
    TrimSelectedSegmentIntent {
        direction: TrimSegmentDirection,
        #[serde(rename = "trimAt")]
        trim_at: Microseconds,
    },
    DeleteSelectedSegment {},
    AddTextSegmentIntent {
        content: String,
        #[serde(default, rename = "targetStart")]
        target_start: Option<Microseconds>,
        #[serde(default, rename = "targetTrackHandle")]
        target_track_handle: Option<String>,
    },
    EditSelectedText {
        patch: TextSegmentPatch,
    },
    ImportSubtitleSrtIntent {
        #[serde(rename = "srtContent")]
        srt_content: String,
    },
    AddAudioSegmentIntent {
        #[serde(default, rename = "materialId")]
        material_id: Option<MaterialId>,
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
    SetSessionPlayhead {
        playhead: Microseconds,
    },
    UpdateDraftCanvasConfig {
        #[serde(rename = "canvasConfig")]
        canvas_config: DraftCanvasConfig,
    },
    UpdateSelectedSegmentVisual {
        patch: SegmentVisualPatch,
    },
    SetSelectedSegmentRetime {
        retiming: SegmentRetiming,
    },
    ApplySelectedSegmentEffect {
        effect: Filter,
    },
    UpdateSelectedSegmentEffectParameter {
        #[serde(rename = "effectIndex")]
        effect_index: u32,
        parameter: EffectParameterUpdate,
    },
    RemoveSelectedSegmentEffect {
        #[serde(rename = "effectIndex")]
        effect_index: u32,
    },
    SetSelectedSegmentMask {
        mask: SegmentMask,
    },
    SetSelectedSegmentBlendMode {
        #[serde(rename = "blendMode")]
        blend_mode: SegmentBlendMode,
    },
    AddTransitionAtBoundary {
        #[serde(rename = "fromSegmentId")]
        from_segment_id: SegmentId,
        #[serde(rename = "toSegmentId")]
        to_segment_id: SegmentId,
        reference: TransitionReference,
        duration: Microseconds,
        #[serde(default)]
        parameters: BTreeMap<String, String>,
    },
    UpdateSelectedTransitionDuration {
        #[serde(rename = "fromSegmentId")]
        from_segment_id: SegmentId,
        #[serde(rename = "toSegmentId")]
        to_segment_id: SegmentId,
        duration: Microseconds,
    },
    RemoveSelectedTransition {
        #[serde(rename = "fromSegmentId")]
        from_segment_id: SegmentId,
        #[serde(rename = "toSegmentId")]
        to_segment_id: SegmentId,
    },
    SetSelectedSegmentKeyframe {
        property: KeyframeProperty,
        interpolation: KeyframeInterpolation,
        easing: KeyframeEasing,
    },
    EditSelectedSegmentKeyframe {
        property: KeyframeProperty,
        at: Microseconds,
        from_at: Option<Microseconds>,
        value: Option<KeyframeValue>,
        interpolation: Option<KeyframeInterpolation>,
        easing: Option<KeyframeEasing>,
    },
    RemoveSelectedSegmentKeyframe {
        property: KeyframeProperty,
    },
    UndoTimelineEdit {},
    RedoTimelineEdit {},
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum ProjectInteractionPayload {
    SelectedSegmentVisual {
        patch: SegmentVisualPatch,
    },
    SelectedSegmentRetime {
        retiming: SegmentRetiming,
    },
    SelectedSegmentEffect {
        #[serde(rename = "effectIndex")]
        effect_index: u32,
        parameter: EffectParameterUpdate,
    },
    SelectedSegmentMask {
        mask: SegmentMask,
    },
    SelectedSegmentBlend {
        #[serde(rename = "opacityMillis")]
        opacity_millis: u32,
    },
    SelectedText {
        patch: TextSegmentPatch,
    },
    SelectedSegmentAudio {
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
    PlayheadScrub {
        playhead: Microseconds,
    },
    TimelineMoveTrim {
        mode: TimelineMoveTrimInteractionMode,
        #[serde(default, rename = "startAt")]
        start_at: Option<Microseconds>,
        #[serde(default, rename = "trimAt")]
        trim_at: Option<Microseconds>,
        #[serde(default, rename = "targetTrackHandle")]
        target_track_handle: Option<String>,
    },
    KeyframeEdit {
        property: KeyframeProperty,
        at: Microseconds,
        #[serde(default, rename = "fromAt")]
        from_at: Option<Microseconds>,
        #[serde(default)]
        value: Option<KeyframeValue>,
        #[serde(default)]
        interpolation: Option<KeyframeInterpolation>,
        #[serde(default)]
        easing: Option<KeyframeEasing>,
    },
    SelectedTransitionDuration {
        #[serde(rename = "fromSegmentId")]
        from_segment_id: SegmentId,
        #[serde(rename = "toSegmentId")]
        to_segment_id: SegmentId,
        duration: Microseconds,
    },
}

impl ProjectInteractionPayload {
    fn interaction_kind(&self) -> ProjectInteractionKind {
        match self {
            ProjectInteractionPayload::SelectedSegmentVisual { .. } => {
                ProjectInteractionKind::SelectedSegmentVisual
            }
            ProjectInteractionPayload::SelectedSegmentRetime { .. } => {
                ProjectInteractionKind::SelectedSegmentRetime
            }
            ProjectInteractionPayload::SelectedSegmentEffect { .. } => {
                ProjectInteractionKind::SelectedSegmentEffect
            }
            ProjectInteractionPayload::SelectedSegmentMask { .. } => {
                ProjectInteractionKind::SelectedSegmentMask
            }
            ProjectInteractionPayload::SelectedSegmentBlend { .. } => {
                ProjectInteractionKind::SelectedSegmentBlend
            }
            ProjectInteractionPayload::SelectedText { .. } => ProjectInteractionKind::SelectedText,
            ProjectInteractionPayload::SelectedSegmentAudio { .. } => {
                ProjectInteractionKind::SelectedSegmentAudio
            }
            ProjectInteractionPayload::PlayheadScrub { .. } => {
                ProjectInteractionKind::PlayheadScrub
            }
            ProjectInteractionPayload::TimelineMoveTrim { .. } => {
                ProjectInteractionKind::TimelineMoveTrim
            }
            ProjectInteractionPayload::KeyframeEdit { .. } => ProjectInteractionKind::KeyframeEdit,
            ProjectInteractionPayload::SelectedTransitionDuration { .. } => {
                ProjectInteractionKind::SelectedTransitionDuration
            }
        }
    }

    fn into_project_intent(self) -> std::result::Result<ProjectIntent, String> {
        match self {
            ProjectInteractionPayload::SelectedSegmentVisual { patch } => {
                Ok(ProjectIntent::UpdateSelectedSegmentVisual { patch })
            }
            ProjectInteractionPayload::SelectedSegmentRetime { .. }
            | ProjectInteractionPayload::SelectedSegmentEffect { .. }
            | ProjectInteractionPayload::SelectedSegmentMask { .. }
            | ProjectInteractionPayload::SelectedSegmentBlend { .. }
            | ProjectInteractionPayload::SelectedTransitionDuration { .. } => Err(
                "Phase 19 interaction payloads resolve through Rust command modules directly"
                    .to_owned(),
            ),
            ProjectInteractionPayload::SelectedText { patch } => {
                Ok(ProjectIntent::EditSelectedText { patch })
            }
            ProjectInteractionPayload::SelectedSegmentAudio {
                gain_millis,
                pan_balance_millis,
                fade_in_duration,
                fade_out_duration,
                effect_slots,
            } => Ok(ProjectIntent::UpdateSelectedSegmentAudio {
                gain_millis,
                pan_balance_millis,
                fade_in_duration,
                fade_out_duration,
                effect_slots,
            }),
            ProjectInteractionPayload::PlayheadScrub { playhead } => {
                Ok(ProjectIntent::SetSessionPlayhead { playhead })
            }
            ProjectInteractionPayload::TimelineMoveTrim {
                mode,
                start_at,
                trim_at,
                target_track_handle,
            } => match mode {
                TimelineMoveTrimInteractionMode::Move => {
                    let start_at =
                        start_at.ok_or_else(|| "时间线移动交互缺少 startAt".to_owned())?;
                    Ok(ProjectIntent::MoveSelectedSegmentIntent {
                        start_at,
                        target_track_handle,
                    })
                }
                TimelineMoveTrimInteractionMode::TrimLeft => {
                    let trim_at =
                        trim_at.ok_or_else(|| "时间线左裁剪交互缺少 trimAt".to_owned())?;
                    Ok(ProjectIntent::TrimSelectedSegmentIntent {
                        direction: TrimSegmentDirection::Left,
                        trim_at,
                    })
                }
                TimelineMoveTrimInteractionMode::TrimRight => {
                    let trim_at =
                        trim_at.ok_or_else(|| "时间线右裁剪交互缺少 trimAt".to_owned())?;
                    Ok(ProjectIntent::TrimSelectedSegmentIntent {
                        direction: TrimSegmentDirection::Right,
                        trim_at,
                    })
                }
            },
            ProjectInteractionPayload::KeyframeEdit {
                property,
                at,
                from_at,
                value,
                interpolation,
                easing,
            } => Ok(ProjectIntent::EditSelectedSegmentKeyframe {
                property,
                at,
                from_at,
                value,
                interpolation,
                easing,
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum TimelineMoveTrimInteractionMode {
    Move,
    TrimLeft,
    TrimRight,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TextSegmentPatch {
    #[serde(default)]
    content: Option<String>,
    #[serde(default, rename = "fontFamily")]
    font_family: Option<String>,
    #[serde(default, rename = "fontRef")]
    font_ref: Option<String>,
    #[serde(default, rename = "fontSize")]
    font_size: Option<u32>,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    alignment: Option<TextAlignment>,
    #[serde(default, rename = "lineHeightMillis")]
    line_height_millis: Option<u32>,
    #[serde(default, rename = "letterSpacingMillis")]
    letter_spacing_millis: Option<u32>,
    #[serde(default, rename = "strokeEnabled")]
    stroke_enabled: Option<bool>,
    #[serde(default, rename = "strokeColor")]
    stroke_color: Option<String>,
    #[serde(default, rename = "strokeWidth")]
    stroke_width: Option<u32>,
    #[serde(default, rename = "shadowEnabled")]
    shadow_enabled: Option<bool>,
    #[serde(default, rename = "shadowColor")]
    shadow_color: Option<String>,
    #[serde(default, rename = "backgroundEnabled")]
    background_enabled: Option<bool>,
    #[serde(default, rename = "backgroundColor")]
    background_color: Option<String>,
    #[serde(default, rename = "textBoxWidthMillis")]
    text_box_width_millis: Option<u32>,
    #[serde(default, rename = "textBoxHeightMillis")]
    text_box_height_millis: Option<u32>,
    #[serde(default, rename = "layoutXMillis")]
    layout_x_millis: Option<u32>,
    #[serde(default, rename = "layoutYMillis")]
    layout_y_millis: Option<u32>,
    #[serde(default, rename = "layoutWidthMillis")]
    layout_width_millis: Option<u32>,
    #[serde(default, rename = "layoutHeightMillis")]
    layout_height_millis: Option<u32>,
    #[serde(default)]
    wrapping: Option<TextWrapping>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SegmentVisualPatch {
    #[serde(default)]
    visible: Option<bool>,
    #[serde(default, rename = "positionX")]
    position_x: Option<i32>,
    #[serde(default, rename = "positionY")]
    position_y: Option<i32>,
    #[serde(default, rename = "positionDeltaX")]
    position_delta_x: Option<i32>,
    #[serde(default, rename = "positionDeltaY")]
    position_delta_y: Option<i32>,
    #[serde(default, rename = "scaleXMillis")]
    scale_x_millis: Option<u32>,
    #[serde(default, rename = "scaleYMillis")]
    scale_y_millis: Option<u32>,
    #[serde(default, rename = "rotationDegrees")]
    rotation_degrees: Option<i32>,
    #[serde(default, rename = "rotationDeltaDegrees")]
    rotation_delta_degrees: Option<i32>,
    #[serde(default, rename = "opacityMillis")]
    opacity_millis: Option<u32>,
    #[serde(default, rename = "cropLeftMillis")]
    crop_left_millis: Option<u32>,
    #[serde(default, rename = "cropRightMillis")]
    crop_right_millis: Option<u32>,
    #[serde(default, rename = "cropTopMillis")]
    crop_top_millis: Option<u32>,
    #[serde(default, rename = "cropBottomMillis")]
    crop_bottom_millis: Option<u32>,
    #[serde(default, rename = "fitMode")]
    fit_mode: Option<SegmentFitMode>,
    #[serde(default, rename = "backgroundKind")]
    background_kind: Option<SegmentBackgroundFillingPatchKind>,
    #[serde(default, rename = "backgroundColor")]
    background_color: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum SegmentBackgroundFillingPatchKind {
    None,
    Black,
    SolidColor,
    Blur,
    Image,
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
struct ProjectInteractionBeginResponse {
    session_id: String,
    interaction_id: String,
    kind: ProjectInteractionKind,
    base_revision: u64,
    revision: u64,
    generation: u64,
    accepted_sequence: u64,
    coalesced_through: u64,
    #[serde(rename = "viewModel")]
    view_model: ProjectSessionViewModel,
    bundle_path: String,
    project_json_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectInteractionUpdateResponse {
    session_id: String,
    interaction_id: String,
    kind: ProjectInteractionKind,
    base_revision: u64,
    revision: u64,
    generation: u64,
    accepted_sequence: u64,
    coalesced_through: u64,
    revision_unchanged: bool,
    #[serde(rename = "provisionalViewModel")]
    provisional_view_model: ProjectSessionViewModel,
    #[serde(rename = "provisionalDelta")]
    provisional_delta: CommandDelta,
    bundle_path: String,
    project_json_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectInteractionCommitResponse {
    session_id: String,
    interaction_id: String,
    kind: ProjectInteractionKind,
    base_revision: u64,
    revision: u64,
    generation: u64,
    accepted_sequence: u64,
    coalesced_through: u64,
    #[serde(rename = "viewModel")]
    view_model: ProjectSessionViewModel,
    events: Vec<draft_model::CommandEvent>,
    delta: draft_model::CommandDelta,
    bundle_path: String,
    project_json_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectInteractionCancelResponse {
    session_id: String,
    interaction_id: String,
    kind: ProjectInteractionKind,
    base_revision: u64,
    revision: u64,
    generation: u64,
    accepted_sequence: u64,
    coalesced_through: u64,
    revision_unchanged: bool,
    canceled: bool,
    #[serde(rename = "viewModel")]
    view_model: ProjectSessionViewModel,
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
    probe_status: ProjectSessionProbeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    probe_job_id: Option<String>,
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
struct ProjectSessionTemplateImportResponse {
    session_id: String,
    revision: u64,
    #[serde(rename = "viewModel")]
    view_model: ProjectSessionViewModel,
    events: Vec<draft_model::CommandEvent>,
    delta: draft_model::CommandDelta,
    adaptation_report: draft_import::AdaptationReport,
    bundle_path: String,
    project_json_path: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
enum ProjectSessionProbeStatus {
    Queued,
    Running,
    Probed,
    Failed,
    Stale,
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
    production_effect_capabilities: EffectCapabilityRegistry,
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
    retiming: SegmentRetiming,
    filters: Vec<Filter>,
    transition: Option<Transition>,
    visual: SegmentVisual,
    volume: SegmentVolume,
    audio: SegmentAudio,
    text: Option<TextSegment>,
    keyframes: Vec<Keyframe>,
    has_text: bool,
    has_audio_controls: bool,
    phase19: SelectedSegmentPhase19ViewModel,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectedSegmentPhase19ViewModel {
    retime_label: String,
    audio_retime_label: String,
    effect_count: usize,
    mask_label: String,
    blend_label: String,
    transition_label: Option<String>,
    support_chips: Vec<ProductionCapabilityChipViewModel>,
    transition_boundary: Option<SelectedSegmentTransitionBoundaryViewModel>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductionCapabilityChipViewModel {
    capability_id: String,
    label: String,
    preview_label: String,
    export_label: String,
    tone: ProductionCapabilityTone,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum ProductionCapabilityTone {
    Ready,
    Warning,
    Error,
    Muted,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectedSegmentTransitionBoundaryViewModel {
    from_segment_id: SegmentId,
    to_segment_id: SegmentId,
    label: String,
    duration: Microseconds,
    has_transition: bool,
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
    kind: TrackKind,
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
    retime_label: String,
    speed_adjusted: bool,
    effect_count: usize,
    mask_label: Option<String>,
    blend_label: String,
    transition_label: Option<String>,
    transition_duration: Option<Microseconds>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelineKeyframeMarkerViewModel {
    marker_key: String,
    property: KeyframeProperty,
    at: Microseconds,
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

struct MaterialProbeSchedulerState {
    scheduler: task_runtime::JobScheduler,
    pending: BTreeMap<JobId, ScheduledMaterialProbe>,
    started_at: Instant,
    next_token_id: u64,
}

struct ProjectIoSchedulerState {
    scheduler: task_runtime::JobScheduler,
    started_at: Instant,
    next_token_id: u64,
}

#[derive(Clone)]
struct ScheduledMaterialProbe {
    session_id: String,
    expected_revision: u64,
    material_id: MaterialId,
    material_uri: String,
    material_path: PathBuf,
    task_token: TaskCancellationToken,
}

#[derive(Debug)]
enum ProjectSessionImportPersistError {
    ProjectStore(ProjectStoreError),
    ArtifactStore(ArtifactStoreError),
}

impl Default for MaterialProbeSchedulerState {
    fn default() -> Self {
        Self {
            scheduler: task_runtime::JobScheduler::new(TaskRuntimeConfig::portable_default()),
            pending: BTreeMap::new(),
            started_at: Instant::now(),
            next_token_id: 1,
        }
    }
}

impl Default for ProjectIoSchedulerState {
    fn default() -> Self {
        Self {
            scheduler: task_runtime::JobScheduler::new(TaskRuntimeConfig::portable_default()),
            started_at: Instant::now(),
            next_token_id: 1,
        }
    }
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
    playhead: Microseconds,
    active_interactions: HashMap<String, ActiveProjectInteraction>,
    next_interaction_id: u64,
    next_interaction_generation: u64,
}

#[derive(Debug, Clone)]
struct ActiveProjectInteraction {
    session: DraftProjectInteractionSession,
    latest_payload: Option<ProjectInteractionPayload>,
    provisional_view_model: Option<ProjectSessionViewModel>,
    provisional_delta: Option<CommandDelta>,
    provisional_draft: Option<Draft>,
    provisional_selection: Option<TimelineSelection>,
}

#[derive(Debug, Clone)]
struct ProjectInteractionProvisionalResult {
    view_model: ProjectSessionViewModel,
    delta: CommandDelta,
    draft: Draft,
    selection: TimelineSelection,
}

enum ResolvedProjectInteraction {
    Playhead(Microseconds),
    Timeline(TimelineEditPayload),
}

#[derive(Debug, Clone)]
pub struct ProjectSessionPreviewSnapshot {
    pub draft: Draft,
    pub bundle_path: PathBuf,
    pub selected_segment: Option<ProjectSessionPreviewSelectedSegment>,
}

#[derive(Debug, Clone)]
pub struct ProjectSessionPreviewSelectedSegment {
    pub track_id: TrackId,
    pub segment_id: SegmentId,
}

#[derive(Debug, Clone)]
pub struct ProjectSessionArtifactSnapshot {
    pub draft: Draft,
    pub bundle_path: PathBuf,
}

pub fn create_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<CreateProjectSessionRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_runtime_value(error_envelope(
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
            return to_runtime_value(error_envelope(
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
            return to_runtime_value(error_envelope(
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
            return to_runtime_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid executeProjectIntent payload: {error}"),
                Some("executeProjectIntent".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.execute_intent(request))
}

pub fn begin_project_interaction(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<BeginProjectInteractionRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_runtime_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid beginProjectInteraction payload: {error}"),
                Some("beginProjectInteraction".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.begin_interaction(request))
}

pub fn update_project_interaction(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<UpdateProjectInteractionRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_runtime_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid updateProjectInteraction payload: {error}"),
                Some("updateProjectInteraction".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.update_interaction(request))
}

pub fn commit_project_interaction(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<CommitProjectInteractionRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_runtime_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid commitProjectInteraction payload: {error}"),
                Some("commitProjectInteraction".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.commit_interaction(request))
}

pub fn cancel_project_interaction(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<CancelProjectInteractionRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_runtime_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid cancelProjectInteraction payload: {error}"),
                Some("cancelProjectInteraction".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.cancel_interaction(request))
}

pub fn import_kaipai_formula_bundle(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<ImportKaipaiFormulaBundleRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_runtime_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid importKaipaiFormulaBundle payload: {error}"),
                Some("importKaipaiFormulaBundle".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.import_kaipai_formula_bundle(request))
}

pub fn list_project_session_materials(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<ProjectSessionReadRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_runtime_value(error_envelope(
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
            return to_runtime_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid listProjectSessionMissingMaterials payload: {error}"),
                Some("listProjectSessionMissingMaterials".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.list_missing_materials(request))
}

pub fn realtime_preview_snapshot(
    session_id: &str,
    expected_revision: u64,
    interaction_id: Option<&str>,
) -> std::result::Result<ProjectSessionPreviewSnapshot, String> {
    project_session_snapshot_for_preview(session_id, expected_revision, interaction_id)
}

pub fn project_session_snapshot(
    session_id: &str,
    expected_revision: u64,
) -> std::result::Result<ProjectSessionPreviewSnapshot, String> {
    project_session_snapshot_for_preview(session_id, expected_revision, None)
}

fn project_session_snapshot_for_preview(
    session_id: &str,
    expected_revision: u64,
    interaction_id: Option<&str>,
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
    if let Some(interaction_id) = interaction_id {
        let active = session
            .active_interactions
            .get(interaction_id)
            .ok_or_else(|| {
                format!("Project interaction not found for preview snapshot: {interaction_id}")
            })?;
        let draft = active.provisional_draft.clone().ok_or_else(|| {
            format!("Project interaction {interaction_id} has no provisional draft snapshot")
        })?;
        let selection = active
            .provisional_selection
            .clone()
            .unwrap_or_else(|| session.selection.clone());
        return Ok(ProjectSessionPreviewSnapshot {
            selected_segment: selected_segment_for_preview(&draft, &selection),
            draft,
            bundle_path: session.bundle_path.clone(),
        });
    }
    Ok(ProjectSessionPreviewSnapshot {
        draft: session.draft.clone(),
        bundle_path: session.bundle_path.clone(),
        selected_segment: selected_segment_for_preview(&session.draft, &session.selection),
    })
}

fn selected_segment_for_preview(
    draft: &Draft,
    selection: &TimelineSelection,
) -> Option<ProjectSessionPreviewSelectedSegment> {
    let selected_segment_id = selection.segment_ids.first()?;
    draft.tracks.iter().find_map(|track| {
        track
            .segments
            .iter()
            .find(|segment| &segment.segment_id == selected_segment_id)
            .map(|segment| ProjectSessionPreviewSelectedSegment {
                track_id: track.track_id.clone(),
                segment_id: segment.segment_id.clone(),
            })
    })
}

pub fn project_session_artifact_snapshot(
    session_id: &str,
    bundle_path: &Path,
) -> std::result::Result<Option<ProjectSessionArtifactSnapshot>, String> {
    let registry = global_project_session_registry();
    let registry = registry
        .lock()
        .map_err(|_| "project session registry lock poisoned".to_string())?;
    let Some(session) = registry.sessions.get(session_id) else {
        return Ok(None);
    };
    if session.bundle_path != bundle_path {
        return Err(format!(
            "Artifact session bundle mismatch: session {} is bound to {} but request used {}",
            session_id,
            session.bundle_path.display(),
            bundle_path.display()
        ));
    }
    Ok(Some(ProjectSessionArtifactSnapshot {
        draft: session.draft.clone(),
        bundle_path: session.bundle_path.clone(),
    }))
}

pub fn project_session_current_revision(session_id: &str) -> Option<u64> {
    let registry = global_project_session_registry();
    let registry = registry.lock().ok()?;
    registry
        .sessions
        .get(session_id)
        .map(|session| session.revision)
}

fn global_project_session_registry() -> &'static Mutex<ProjectSessionRegistry> {
    static REGISTRY: OnceLock<Mutex<ProjectSessionRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(ProjectSessionRegistry::default()))
}

fn with_project_session_registry(
    f: impl FnOnce(&mut ProjectSessionRegistry) -> Result<serde_json::Value>,
) -> Result<serde_json::Value> {
    let mut registry = global_project_session_registry().lock().map_err(|_| {
        RuntimeError::new(
            RuntimeErrorKind::ProjectStore,
            "project session registry lock poisoned",
        )
    })?;
    f(&mut registry)
}

fn global_material_probe_scheduler() -> &'static Mutex<MaterialProbeSchedulerState> {
    static SCHEDULER: OnceLock<Mutex<MaterialProbeSchedulerState>> = OnceLock::new();
    SCHEDULER.get_or_init(|| Mutex::new(MaterialProbeSchedulerState::default()))
}

#[cfg(feature = "test-hooks")]
fn material_probe_enqueue_failure_for_tests() -> &'static Mutex<Option<String>> {
    static FAILURE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    FAILURE.get_or_init(|| Mutex::new(None))
}

#[cfg(feature = "test-hooks")]
pub struct MaterialProbeEnqueueFailureGuard {
    previous: Option<String>,
}

#[cfg(feature = "test-hooks")]
pub fn force_material_probe_enqueue_failure_for_tests(
    message: impl Into<String>,
) -> MaterialProbeEnqueueFailureGuard {
    let mut failure = material_probe_enqueue_failure_for_tests()
        .lock()
        .expect("material probe enqueue failure hook lock should not be poisoned");
    let previous = failure.replace(message.into());
    MaterialProbeEnqueueFailureGuard { previous }
}

#[cfg(feature = "test-hooks")]
impl Drop for MaterialProbeEnqueueFailureGuard {
    fn drop(&mut self) {
        if let Ok(mut failure) = material_probe_enqueue_failure_for_tests().lock() {
            *failure = self.previous.take();
        }
    }
}

#[cfg(feature = "test-hooks")]
fn material_probe_worker_spawn_failure_for_tests() -> &'static Mutex<Option<String>> {
    static FAILURE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    FAILURE.get_or_init(|| Mutex::new(None))
}

#[cfg(feature = "test-hooks")]
pub struct MaterialProbeWorkerSpawnFailureGuard {
    previous: Option<String>,
}

#[cfg(feature = "test-hooks")]
pub fn force_material_probe_worker_spawn_failure_for_tests(
    message: impl Into<String>,
) -> MaterialProbeWorkerSpawnFailureGuard {
    let mut failure = material_probe_worker_spawn_failure_for_tests()
        .lock()
        .expect("material probe worker spawn failure hook lock should not be poisoned");
    let previous = failure.replace(message.into());
    MaterialProbeWorkerSpawnFailureGuard { previous }
}

#[cfg(feature = "test-hooks")]
impl Drop for MaterialProbeWorkerSpawnFailureGuard {
    fn drop(&mut self) {
        if let Ok(mut failure) = material_probe_worker_spawn_failure_for_tests().lock() {
            *failure = self.previous.take();
        }
    }
}

fn global_project_io_scheduler() -> &'static Mutex<ProjectIoSchedulerState> {
    static SCHEDULER: OnceLock<Mutex<ProjectIoSchedulerState>> = OnceLock::new();
    SCHEDULER.get_or_init(|| Mutex::new(ProjectIoSchedulerState::default()))
}

pub fn record_project_session_task_runtime_telemetry_snapshots() {
    if let Ok(state) = global_material_probe_scheduler().lock() {
        state.record_task_runtime_telemetry();
    }
    if let Ok(state) = global_project_io_scheduler().lock() {
        state.record_task_runtime_telemetry();
    }
}

impl MaterialProbeSchedulerState {
    fn now_us(&self) -> u64 {
        u64::try_from(self.started_at.elapsed().as_micros()).unwrap_or(u64::MAX)
    }

    fn next_task_token(&mut self) -> TaskCancellationToken {
        let token = TaskCancellationToken::new(self.next_token_id);
        self.next_token_id = self.next_token_id.saturating_add(1);
        token
    }

    fn record_task_runtime_telemetry(&self) {
        record_task_runtime_scheduler_snapshot(
            ProjectSessionTelemetrySource::MediaProbe,
            &self.scheduler.telemetry_snapshot(),
        );
    }
}

impl ProjectIoSchedulerState {
    fn now_us(&self) -> u64 {
        u64::try_from(self.started_at.elapsed().as_micros()).unwrap_or(u64::MAX)
    }

    fn next_task_token(&mut self) -> TaskCancellationToken {
        let token = TaskCancellationToken::new(self.next_token_id);
        self.next_token_id = self.next_token_id.saturating_add(1);
        token
    }

    fn record_task_runtime_telemetry(&self) {
        record_task_runtime_scheduler_snapshot(
            ProjectSessionTelemetrySource::ProjectIo,
            &self.scheduler.telemetry_snapshot(),
        );
    }
}

fn enqueue_material_probe(work: ScheduledMaterialProbe) -> std::result::Result<String, String> {
    #[cfg(feature = "test-hooks")]
    if let Some(message) = material_probe_enqueue_failure_for_tests()
        .lock()
        .map_err(|_| "material probe enqueue failure hook lock poisoned".to_owned())?
        .clone()
    {
        return Err(message);
    }

    let (job_id_string, started) = {
        let scheduler = global_material_probe_scheduler();
        let mut state = scheduler
            .lock()
            .map_err(|_| "material probe scheduler lock poisoned".to_owned())?;
        let submitted_at_us = state.now_us();
        let token = state.next_task_token();
        let job_id = JobId::new(format!(
            "material-probe-{}-{}-{}-{}",
            work.session_id,
            work.material_id.as_str(),
            work.expected_revision,
            token.id()
        ));
        let envelope = JobEnvelope::new(
            job_id.clone(),
            JobDomain::MediaProbe,
            JobPriority::UserVisible,
            ResourceClass::ValidationProbe,
            token.clone(),
            submitted_at_us,
        );
        let mut work = work;
        work.task_token = token;
        state
            .scheduler
            .submit(envelope)
            .map_err(|error| format!("material probe scheduler rejected: {error}"))?;
        let job_id_string = job_id.as_str().to_owned();
        state.pending.insert(job_id.clone(), work);
        (job_id_string, start_ready_material_probe_jobs(&mut state)?)
    };

    spawn_material_probe_jobs_with_failure_policy(started, false)?;

    Ok(job_id_string)
}

fn start_ready_material_probe_jobs(
    state: &mut MaterialProbeSchedulerState,
) -> std::result::Result<Vec<(JobId, ScheduledMaterialProbe)>, String> {
    let mut started = Vec::new();
    loop {
        let start_at_us = state.now_us();
        let Some(envelope) = state
            .scheduler
            .start_next(start_at_us)
            .map_err(|error| format!("material probe scheduler start failed: {error}"))?
        else {
            break;
        };
        let Some(work) = state.pending.remove(&envelope.job_id) else {
            return Err(format!(
                "material probe work missing for scheduled job {}",
                envelope.job_id.as_str()
            ));
        };
        started.push((envelope.job_id, work));
    }
    state.record_task_runtime_telemetry();
    Ok(started)
}

fn spawn_material_probe_jobs(
    jobs: Vec<(JobId, ScheduledMaterialProbe)>,
) -> std::result::Result<(), String> {
    spawn_material_probe_jobs_with_failure_policy(jobs, true)
}

fn spawn_material_probe_jobs_with_failure_policy(
    jobs: Vec<(JobId, ScheduledMaterialProbe)>,
    apply_spawn_failure_to_material: bool,
) -> std::result::Result<(), String> {
    let mut first_error = None;
    for (job_id, work) in jobs {
        #[cfg(feature = "test-hooks")]
        if let Some(message) = material_probe_worker_spawn_failure_for_tests()
            .lock()
            .map_err(|_| "material probe worker spawn failure hook lock poisoned".to_owned())?
            .clone()
        {
            fail_started_material_probe_job(
                job_id,
                work,
                message.clone(),
                apply_spawn_failure_to_material,
            );
            first_error.get_or_insert(message);
            continue;
        }

        let thread_job_id = job_id.clone();
        let thread_work = work.clone();
        if let Err(error) = thread::Builder::new()
            .name("task-runtime-media-probe".to_owned())
            .spawn(move || run_material_probe_worker(thread_job_id, thread_work))
        {
            let message = format!("material probe worker failed to start: {error}");
            fail_started_material_probe_job(
                job_id,
                work,
                message.clone(),
                apply_spawn_failure_to_material,
            );
            first_error.get_or_insert(message);
        }
    }
    if let Some(error) = first_error {
        Err(error)
    } else {
        Ok(())
    }
}

fn fail_started_material_probe_job(
    job_id: JobId,
    work: ScheduledMaterialProbe,
    message: String,
    apply_material_failure: bool,
) {
    let mut accepted = false;
    let mut next_jobs = Vec::new();
    let scheduler = global_material_probe_scheduler();
    if let Ok(mut state) = scheduler.lock() {
        let completed_at_us = state.now_us();
        let _ = state.scheduler.complete_with_commit(
            &job_id,
            JobResult::new(job_id.clone(), task_runtime::JobResultKind::Failed),
            completed_at_us,
            CompletionFreshness::none(),
            |_| accepted = true,
        );
        if let Ok(started) = start_ready_material_probe_jobs(&mut state) {
            next_jobs = started;
        }
        state.record_task_runtime_telemetry();
    }

    if accepted && apply_material_failure {
        let _ = complete_material_probe_job_error(&work, message);
    }
    let _ = spawn_material_probe_jobs_with_failure_policy(next_jobs, apply_material_failure);
}

fn run_material_probe_worker(job_id: JobId, work: ScheduledMaterialProbe) {
    let result = discover_runtime_config()
        .map_err(|error| runtime_discovery_message(error))
        .and_then(|runtime| {
            let executor = DesktopFfmpegExecutor::default();
            run_scheduled_material_probe(&executor, &runtime, &work.material_path)
                .map_err(|error| error.message)
        });

    let job_result = match &result {
        Ok(_) => JobResult::completed(job_id.clone()),
        Err(_) => JobResult::new(job_id.clone(), task_runtime::JobResultKind::Failed),
    };

    let mut accepted = false;
    let mut next_jobs = Vec::new();
    let scheduler = global_material_probe_scheduler();
    if let Ok(mut state) = scheduler.lock() {
        let completed_at_us = state.now_us();
        let _ = state.scheduler.complete_with_commit(
            &job_id,
            job_result,
            completed_at_us,
            CompletionFreshness::none(),
            |_| accepted = true,
        );
        if let Ok(started) = start_ready_material_probe_jobs(&mut state) {
            next_jobs = started;
        }
        state.record_task_runtime_telemetry();
    }

    if accepted {
        match result {
            Ok(metadata) => {
                let _ = complete_material_probe_job(&work, Some(metadata), None);
            }
            Err(message) => {
                let _ = complete_material_probe_job_error(&work, message);
            }
        }
    }
    let _ = spawn_material_probe_jobs(next_jobs);
}

fn complete_material_probe_job(
    work: &ScheduledMaterialProbe,
    metadata: Option<media_runtime::MaterialProbeMetadata>,
    message: Option<String>,
) -> bool {
    let registry = global_project_session_registry();
    let Ok(mut registry) = registry.lock() else {
        return false;
    };
    let Some(session) = registry.sessions.get_mut(&work.session_id) else {
        return false;
    };
    let material_still_matches = session.draft.materials.iter().any(|material| {
        material.material_id == work.material_id && material.uri == work.material_uri
    });
    if !material_still_matches {
        return false;
    }
    let mut next_draft = session.draft.clone();
    let material_result = if let Some(metadata) = metadata {
        crate::material_service::apply_probe_metadata_to_material(
            &mut next_draft,
            &work.material_id,
            metadata,
        )
    } else {
        let runtime = media_runtime::RuntimeConfig {
            ffmpeg: media_runtime::DiscoveredBinary {
                kind: media_runtime::BinaryKind::Ffmpeg,
                path: PathBuf::from("scheduled-probe"),
                source: media_runtime::DiscoverySource::Bundled {
                    directory: PathBuf::from("scheduled-probe"),
                },
                version: "scheduled-probe".to_owned(),
            },
            ffprobe: media_runtime::DiscoveredBinary {
                kind: media_runtime::BinaryKind::Ffprobe,
                path: PathBuf::from("scheduled-probe"),
                source: media_runtime::DiscoverySource::Bundled {
                    directory: PathBuf::from("scheduled-probe"),
                },
                version: "scheduled-probe".to_owned(),
            },
        };
        let error = media_runtime::MaterialProbeError {
            kind: media_runtime::MaterialProbeErrorKind::ProbeFailed,
            path: work.material_path.clone(),
            ffprobe_path: runtime.ffprobe.path,
            executor: "task-runtime-media-probe".to_owned(),
            stdout_summary: None,
            stderr_summary: None,
            message: message.unwrap_or_else(|| "material probe failed".to_owned()),
        };
        crate::material_service::apply_probe_error_to_material(
            &mut next_draft,
            &work.material_id,
            &error,
        )
    };
    if material_result.is_err() {
        return false;
    }
    let fs = StdPlatformFileSystem;
    let Ok(saved) = save_project_bundle(&fs, &session.bundle_path, &next_draft) else {
        return false;
    };
    session.revision = session.revision.saturating_add(1);
    session.draft = saved.draft;
    session.bundle_path = saved.bundle_path;
    session.project_json_path = saved.project_json_path;
    session.active_interactions.clear();
    true
}

fn complete_material_probe_job_error(work: &ScheduledMaterialProbe, message: String) -> bool {
    complete_material_probe_job(work, None, Some(message))
}

fn run_project_io_job<T>(label: &str, expected_revision: u64, operation: impl FnOnce() -> T) -> T {
    let job_id = JobId::new(format!("project-io-{label}-{expected_revision}"));
    {
        let scheduler = global_project_io_scheduler();
        if let Ok(mut state) = scheduler.lock() {
            let submitted_at_us = state.now_us();
            let token = state.next_task_token();
            let envelope = project_io_scheduler_envelope(job_id.clone(), token, submitted_at_us)
                .with_freshness(
                    JobFreshness::timeline(Microseconds::ZERO, PlaybackGeneration::new(0))
                        .with_project_session(label.to_owned(), expected_revision),
                );
            let _ = state.scheduler.submit(envelope);
            let now_us = state.now_us();
            let _ = state.scheduler.start_next(now_us);
            state.record_task_runtime_telemetry();
        }
    }
    let output = operation();
    complete_project_io_job(job_id, expected_revision);
    output
}

fn complete_project_io_job(job_id: JobId, expected_revision: u64) {
    // stale project session revision completions are rejected before session-visible mutation.
    let scheduler = global_project_io_scheduler();
    if let Ok(mut state) = scheduler.lock() {
        let completed_at_us = state.now_us();
        let _ = state.scheduler.complete_with_commit(
            &job_id,
            JobResult::completed(job_id.clone()),
            completed_at_us,
            CompletionFreshness::none().with_expected_revision(expected_revision),
            |_| {},
        );
        state.record_task_runtime_telemetry();
    }
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
        let bundle = match run_project_io_job("create-project", 0, || {
            create_project_bundle(&fs, &bundle_path, &draft)
        }) {
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
            playhead: Microseconds::new(0),
            active_interactions: HashMap::new(),
            next_interaction_id: 0,
            next_interaction_generation: 0,
        };
        self.sessions.insert(session_id.clone(), session);

        to_runtime_value(ok_envelope(ProjectSessionOpenResponse {
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
        let opened = match run_project_io_job("open-project", 0, || {
            open_project_bundle(&fs, PathBuf::from(&request.bundle_path))
        }) {
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
            playhead: Microseconds::new(0),
            active_interactions: HashMap::new(),
            next_interaction_id: 0,
            next_interaction_generation: 0,
        };
        self.sessions.insert(session_id.clone(), session);

        to_runtime_value(ok_envelope(ProjectSessionOpenResponse {
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
                .map(project_store_warning_message)
                .collect(),
        }))
    }

    fn close_session(&mut self, request: ProjectSessionRequest) -> Result<serde_json::Value> {
        let closed = self.sessions.remove(&request.session_id).is_some();
        to_runtime_value(ok_envelope(ProjectSessionClosedResponse {
            session_id: request.session_id,
            closed,
        }))
    }

    fn list_materials(&self, request: ProjectSessionReadRequest) -> Result<serde_json::Value> {
        let _known_revision = request.expected_revision;
        let Some(session) = self.sessions.get(&request.session_id) else {
            return to_runtime_value(error_envelope(
                CommandErrorKind::InvalidProject,
                format!("Project session not found: {}", request.session_id),
                Some("listProjectSessionMaterials".to_string()),
            ));
        };

        to_runtime_value(ok_envelope(ProjectSessionMaterialsResponse {
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
        let _known_revision = request.expected_revision;
        let Some(session) = self.sessions.get(&request.session_id) else {
            return to_runtime_value(error_envelope(
                CommandErrorKind::InvalidProject,
                format!("Project session not found: {}", request.session_id),
                Some("listProjectSessionMissingMaterials".to_string()),
            ));
        };

        let fs = StdPlatformFileSystem;
        match crate::material_service::list_missing_materials(
            &session.draft,
            &fs,
            &session.bundle_path,
        ) {
            Ok(diagnostics) => {
                to_runtime_value(ok_envelope(ProjectSessionMissingMaterialsResponse {
                    session_id: session.session_id.clone(),
                    revision: session.revision,
                    bundle_path: session.bundle_path.display().to_string(),
                    project_json_path: session.project_json_path.display().to_string(),
                    diagnostics: diagnostics.into_iter().map(command_diagnostic).collect(),
                }))
            }
            Err(error) => to_runtime_value(material_service_error_envelope(
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
            return to_runtime_value(error_envelope(
                CommandErrorKind::InvalidProject,
                format!("Project session not found: {}", request.session_id),
                Some("executeProjectIntent".to_string()),
            ));
        };
        if request.expected_revision != session.revision {
            return to_runtime_value(error_envelope(
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
            ProjectIntent::SetSessionPlayhead { playhead } => {
                session.apply_session_playhead(playhead)
            }
            intent => {
                let payload = match session.intent_payload(intent) {
                    Ok(payload) => payload,
                    Err(message) => {
                        return to_runtime_value(error_envelope(
                            CommandErrorKind::InvalidTimelineEdit,
                            message,
                            Some("executeProjectIntent".to_string()),
                        ));
                    }
                };
                let response = match draft_commands::timeline::execute_timeline_edit(payload) {
                    Ok(response) => response,
                    Err(error) => {
                        return to_runtime_value(error_envelope(
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

    fn begin_interaction(
        &mut self,
        request: BeginProjectInteractionRequest,
    ) -> Result<serde_json::Value> {
        let Some(session) = self.sessions.get_mut(&request.session_id) else {
            return project_interaction_error(
                "beginProjectInteraction",
                CommandErrorKind::InvalidProject,
                format!("Project session not found: {}", request.session_id),
            );
        };
        if request.expected_revision != session.revision {
            return stale_interaction_revision_error(
                "beginProjectInteraction",
                request.expected_revision,
                session.revision,
            );
        }

        let interaction_id = session.next_project_interaction_id();
        let generation = session.next_project_interaction_generation();
        let interaction = DraftProjectInteractionSession::new(
            interaction_id.clone(),
            request.kind,
            session.revision,
            generation,
        );
        let response = ProjectInteractionBeginResponse {
            session_id: session.session_id.clone(),
            interaction_id: interaction_id.clone(),
            kind: request.kind,
            base_revision: interaction.base_revision,
            revision: session.revision,
            generation: interaction.generation,
            accepted_sequence: interaction.accepted_sequence,
            coalesced_through: interaction.coalesced_through,
            view_model: project_session_view_model(
                &session.draft,
                &session.command_state,
                &session.selection,
            ),
            bundle_path: session.bundle_path.display().to_string(),
            project_json_path: session.project_json_path.display().to_string(),
        };
        session.active_interactions.insert(
            interaction_id,
            ActiveProjectInteraction {
                session: interaction,
                latest_payload: None,
                provisional_view_model: None,
                provisional_delta: None,
                provisional_draft: None,
                provisional_selection: None,
            },
        );

        to_runtime_value(ok_envelope(response))
    }

    fn update_interaction(
        &mut self,
        request: UpdateProjectInteractionRequest,
    ) -> Result<serde_json::Value> {
        let Some(session) = self.sessions.get_mut(&request.session_id) else {
            return project_interaction_error(
                "updateProjectInteraction",
                CommandErrorKind::InvalidProject,
                format!("Project session not found: {}", request.session_id),
            );
        };
        if request.expected_revision != session.revision {
            return stale_interaction_revision_error(
                "updateProjectInteraction",
                request.expected_revision,
                session.revision,
            );
        }

        let Some(active) = session.active_interactions.get(&request.interaction_id) else {
            return missing_interaction_error("updateProjectInteraction", &request.interaction_id);
        };
        if let Some(error) = validate_interaction_revision(
            "updateProjectInteraction",
            request.expected_revision,
            active.session.base_revision,
        ) {
            return error;
        }
        if let Some(error) = validate_interaction_kind(
            "updateProjectInteraction",
            active.session.kind,
            request.payload.interaction_kind(),
        ) {
            return error;
        }
        let mut accepted = active.session.clone();
        if let Some(error) =
            accept_interaction_sequence("updateProjectInteraction", &mut accepted, request.sequence)
        {
            return error;
        }

        let provisional = match session.provisional_interaction_payload(&request.payload) {
            Ok(response) => response,
            Err(message) => {
                return project_interaction_error(
                    "updateProjectInteraction",
                    CommandErrorKind::InvalidTimelineEdit,
                    message,
                );
            }
        };
        let provisional_view_model = provisional.view_model.clone();
        let provisional_delta = provisional.delta.clone();
        let Some(active) = session.active_interactions.get_mut(&request.interaction_id) else {
            return missing_interaction_error("updateProjectInteraction", &request.interaction_id);
        };
        active.session = accepted.clone();
        active.latest_payload = Some(request.payload);
        active.provisional_view_model = Some(provisional_view_model.clone());
        active.provisional_delta = Some(provisional_delta.clone());
        active.provisional_draft = Some(provisional.draft);
        active.provisional_selection = Some(provisional.selection);

        to_runtime_value(ok_envelope(ProjectInteractionUpdateResponse {
            session_id: session.session_id.clone(),
            interaction_id: accepted.interaction_id,
            kind: accepted.kind,
            base_revision: accepted.base_revision,
            revision: session.revision,
            generation: accepted.generation,
            accepted_sequence: accepted.accepted_sequence,
            coalesced_through: accepted.coalesced_through,
            revision_unchanged: true,
            provisional_view_model,
            provisional_delta,
            bundle_path: session.bundle_path.display().to_string(),
            project_json_path: session.project_json_path.display().to_string(),
        }))
    }

    fn commit_interaction(
        &mut self,
        request: CommitProjectInteractionRequest,
    ) -> Result<serde_json::Value> {
        let Some(session) = self.sessions.get_mut(&request.session_id) else {
            return project_interaction_error(
                "commitProjectInteraction",
                CommandErrorKind::InvalidProject,
                format!("Project session not found: {}", request.session_id),
            );
        };
        if request.expected_revision != session.revision {
            return stale_interaction_revision_error(
                "commitProjectInteraction",
                request.expected_revision,
                session.revision,
            );
        }

        let Some(active) = session.active_interactions.get(&request.interaction_id) else {
            return missing_interaction_error("commitProjectInteraction", &request.interaction_id);
        };
        if let Some(error) = validate_interaction_revision(
            "commitProjectInteraction",
            request.expected_revision,
            active.session.base_revision,
        ) {
            return error;
        }
        let interaction = active.session.clone();
        let Some(payload) = active.latest_payload.clone() else {
            return project_interaction_error(
                "commitProjectInteraction",
                CommandErrorKind::InvalidPayload,
                format!(
                    "Project interaction {} has no accepted update to commit",
                    request.interaction_id
                ),
            );
        };
        if let Some(error) = validate_interaction_kind(
            "commitProjectInteraction",
            interaction.kind,
            payload.interaction_kind(),
        ) {
            return error;
        }

        let response = session.commit_interaction_payload(payload, &interaction)?;
        session.active_interactions.remove(&request.interaction_id);
        Ok(response)
    }

    fn cancel_interaction(
        &mut self,
        request: CancelProjectInteractionRequest,
    ) -> Result<serde_json::Value> {
        let Some(session) = self.sessions.get_mut(&request.session_id) else {
            return project_interaction_error(
                "cancelProjectInteraction",
                CommandErrorKind::InvalidProject,
                format!("Project session not found: {}", request.session_id),
            );
        };
        if request.expected_revision != session.revision {
            return stale_interaction_revision_error(
                "cancelProjectInteraction",
                request.expected_revision,
                session.revision,
            );
        }

        let Some(active) = session.active_interactions.get(&request.interaction_id) else {
            return missing_interaction_error("cancelProjectInteraction", &request.interaction_id);
        };
        if let Some(error) = validate_interaction_revision(
            "cancelProjectInteraction",
            request.expected_revision,
            active.session.base_revision,
        ) {
            return error;
        }
        let active = session
            .active_interactions
            .remove(&request.interaction_id)
            .expect("validated interaction should still be active");

        to_runtime_value(ok_envelope(ProjectInteractionCancelResponse {
            session_id: session.session_id.clone(),
            interaction_id: active.session.interaction_id,
            kind: active.session.kind,
            base_revision: active.session.base_revision,
            revision: session.revision,
            generation: active.session.generation,
            accepted_sequence: active.session.accepted_sequence,
            coalesced_through: active.session.coalesced_through,
            revision_unchanged: true,
            canceled: true,
            view_model: project_session_view_model(
                &session.draft,
                &session.command_state,
                &session.selection,
            ),
            bundle_path: session.bundle_path.display().to_string(),
            project_json_path: session.project_json_path.display().to_string(),
        }))
    }

    fn import_kaipai_formula_bundle(
        &mut self,
        request: ImportKaipaiFormulaBundleRequest,
    ) -> Result<serde_json::Value> {
        let Some(session) = self.sessions.get_mut(&request.session_id) else {
            return to_runtime_value(error_envelope(
                CommandErrorKind::InvalidProject,
                format!("Project session not found: {}", request.session_id),
                Some("importKaipaiFormulaBundle".to_string()),
            ));
        };
        if request.expected_revision != session.revision {
            return to_runtime_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!(
                    "Stale project session revision: expected {}, current {}",
                    request.expected_revision, session.revision
                ),
                Some("importKaipaiFormulaBundle".to_string()),
            ));
        }

        session.import_kaipai_formula_bundle(request)
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

fn project_interaction_error(
    command: &str,
    kind: CommandErrorKind,
    message: String,
) -> Result<serde_json::Value> {
    to_runtime_value(error_envelope(kind, message, Some(command.to_owned())))
}

fn stale_interaction_revision_error(
    command: &str,
    expected_revision: u64,
    current_revision: u64,
) -> Result<serde_json::Value> {
    project_interaction_error(
        command,
        CommandErrorKind::InvalidPayload,
        format!(
            "Stale project session revision: expected {}, current {}",
            expected_revision, current_revision
        ),
    )
}

fn missing_interaction_error(command: &str, interaction_id: &str) -> Result<serde_json::Value> {
    project_interaction_error(
        command,
        CommandErrorKind::InvalidPayload,
        format!("Project interaction not found or no longer active: {interaction_id}"),
    )
}

fn validate_interaction_revision(
    command: &str,
    expected_revision: u64,
    base_revision: u64,
) -> Option<Result<serde_json::Value>> {
    if expected_revision == base_revision {
        return None;
    }
    Some(project_interaction_error(
        command,
        CommandErrorKind::InvalidPayload,
        format!(
            "Stale project interaction base revision: expected {}, interaction base {}",
            expected_revision, base_revision
        ),
    ))
}

fn validate_interaction_kind(
    command: &str,
    expected: ProjectInteractionKind,
    received: ProjectInteractionKind,
) -> Option<Result<serde_json::Value>> {
    if expected == received {
        return None;
    }
    Some(project_interaction_error(
        command,
        CommandErrorKind::InvalidPayload,
        format!(
            "Project interaction kind mismatch: expected {:?}, received {:?}",
            expected, received
        ),
    ))
}

fn accept_interaction_sequence(
    command: &str,
    interaction: &mut DraftProjectInteractionSession,
    sequence: u64,
) -> Option<Result<serde_json::Value>> {
    match interaction.accept_sequence(sequence) {
        Ok(()) => None,
        Err(ProjectInteractionSequenceError::Zero) => Some(project_interaction_error(
            command,
            CommandErrorKind::InvalidPayload,
            "Project interaction sequence must start at 1".to_owned(),
        )),
        Err(ProjectInteractionSequenceError::Stale {
            accepted_sequence,
            received_sequence,
        }) => Some(project_interaction_error(
            command,
            CommandErrorKind::InvalidPayload,
            format!(
                "Stale project interaction sequence: received {}, accepted {}",
                received_sequence, accepted_sequence
            ),
        )),
    }
}

impl ProjectSession {
    fn next_project_interaction_id(&mut self) -> String {
        self.next_interaction_id = self.next_interaction_id.saturating_add(1);
        format!(
            "{}-interaction-{}",
            self.session_id, self.next_interaction_id
        )
    }

    fn next_project_interaction_generation(&mut self) -> u64 {
        self.next_interaction_generation = self.next_interaction_generation.saturating_add(1);
        self.next_interaction_generation
    }

    fn provisional_interaction_payload(
        &self,
        payload: &ProjectInteractionPayload,
    ) -> std::result::Result<ProjectInteractionProvisionalResult, String> {
        match self.resolve_interaction_payload(payload.clone())? {
            ResolvedProjectInteraction::Playhead(_) => Ok(ProjectInteractionProvisionalResult {
                view_model: project_session_view_model(
                    &self.draft,
                    &self.command_state,
                    &self.selection,
                ),
                delta: CommandDelta::none(
                    CommandDeltaName::SeekAudioPreview,
                    "playhead scrub update",
                ),
                draft: self.draft.clone(),
                selection: self.selection.clone(),
            }),
            ResolvedProjectInteraction::Timeline(payload) => {
                let response = draft_commands::timeline::execute_timeline_edit(payload)
                    .map_err(|error| error.to_string())?;
                Ok(ProjectInteractionProvisionalResult {
                    view_model: project_session_view_model(
                        &response.draft,
                        &response.command_state,
                        &response.selection,
                    ),
                    delta: response.delta,
                    draft: response.draft,
                    selection: response.selection,
                })
            }
        }
    }

    fn commit_interaction_payload(
        &mut self,
        payload: ProjectInteractionPayload,
        interaction: &DraftProjectInteractionSession,
    ) -> Result<serde_json::Value> {
        let resolved = match self.resolve_interaction_payload(payload) {
            Ok(resolved) => resolved,
            Err(message) => {
                return project_interaction_error(
                    "commitProjectInteraction",
                    CommandErrorKind::InvalidPayload,
                    message,
                );
            }
        };
        match resolved {
            ResolvedProjectInteraction::Playhead(playhead) => {
                self.playhead = playhead;
                to_runtime_value(ok_envelope(ProjectInteractionCommitResponse {
                    session_id: self.session_id.clone(),
                    interaction_id: interaction.interaction_id.clone(),
                    kind: interaction.kind,
                    base_revision: interaction.base_revision,
                    revision: self.revision,
                    generation: interaction.generation,
                    accepted_sequence: interaction.accepted_sequence,
                    coalesced_through: interaction.coalesced_through,
                    view_model: project_session_view_model(
                        &self.draft,
                        &self.command_state,
                        &self.selection,
                    ),
                    events: Vec::new(),
                    delta: CommandDelta::none(
                        CommandDeltaName::SeekAudioPreview,
                        "playhead scrub committed",
                    ),
                    bundle_path: self.bundle_path.display().to_string(),
                    project_json_path: self.project_json_path.display().to_string(),
                }))
            }
            ResolvedProjectInteraction::Timeline(payload) => {
                let response = match draft_commands::timeline::execute_timeline_edit(payload) {
                    Ok(response) => response,
                    Err(error) => {
                        return project_interaction_error(
                            "commitProjectInteraction",
                            CommandErrorKind::InvalidTimelineEdit,
                            error.to_string(),
                        );
                    }
                };
                self.apply_interaction_response(response, interaction)
            }
        }
    }

    fn resolve_interaction_payload(
        &self,
        payload: ProjectInteractionPayload,
    ) -> std::result::Result<ResolvedProjectInteraction, String> {
        match payload {
            ProjectInteractionPayload::PlayheadScrub { playhead } => {
                Ok(ResolvedProjectInteraction::Playhead(playhead))
            }
            ProjectInteractionPayload::SelectedSegmentRetime { retiming } => {
                Ok(ResolvedProjectInteraction::Timeline(
                    TimelineEditPayload::SetSegmentRetime(SetSegmentRetimeCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_id: self.selected_segment_id("调整速度")?,
                        retiming,
                    }),
                ))
            }
            ProjectInteractionPayload::SelectedSegmentEffect {
                effect_index,
                parameter,
            } => Ok(ResolvedProjectInteraction::Timeline(
                TimelineEditPayload::UpdateSegmentEffectParameter(
                    UpdateSegmentEffectParameterCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_id: self.selected_segment_id("调整效果")?,
                        effect_index,
                        parameter,
                    },
                ),
            )),
            ProjectInteractionPayload::SelectedSegmentMask { mask } => {
                Ok(ResolvedProjectInteraction::Timeline(
                    TimelineEditPayload::SetSegmentMask(SetSegmentMaskCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_id: self.selected_segment_id("调整遮罩")?,
                        mask,
                    }),
                ))
            }
            ProjectInteractionPayload::SelectedSegmentBlend { opacity_millis } => {
                let segment = self.selected_segment("调整混合")?;
                let segment_id = segment.segment_id.clone();
                let mut visual = segment.visual.clone();
                visual.transform.opacity.value_millis =
                    checked_u32("混合不透明度", opacity_millis, 0, 1_000)?;
                Ok(ResolvedProjectInteraction::Timeline(
                    TimelineEditPayload::UpdateSegmentVisual(UpdateSegmentVisualCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_id,
                        visual,
                    }),
                ))
            }
            ProjectInteractionPayload::SelectedTransitionDuration {
                from_segment_id,
                to_segment_id,
                duration,
            } => Ok(ResolvedProjectInteraction::Timeline(
                TimelineEditPayload::UpdateTransitionDuration(
                    UpdateTransitionDurationCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        from_segment_id,
                        to_segment_id,
                        duration,
                    },
                ),
            )),
            payload => {
                let intent = payload.into_project_intent()?;
                Ok(ResolvedProjectInteraction::Timeline(
                    self.intent_payload(intent)?,
                ))
            }
        }
    }

    fn apply_interaction_response(
        &mut self,
        response: TimelineCommandResponse,
        interaction: &DraftProjectInteractionSession,
    ) -> Result<serde_json::Value> {
        if is_selection_only_delta(&response.delta) {
            self.command_state = response.command_state;
            self.selection = response.selection;
            return to_runtime_value(ok_envelope(ProjectInteractionCommitResponse {
                session_id: self.session_id.clone(),
                interaction_id: interaction.interaction_id.clone(),
                kind: interaction.kind,
                base_revision: interaction.base_revision,
                revision: self.revision,
                generation: interaction.generation,
                accepted_sequence: interaction.accepted_sequence,
                coalesced_through: interaction.coalesced_through,
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
        let saved = match run_project_io_job("interaction-commit", self.revision, || {
            save_project_bundle(&fs, &self.bundle_path, &response.draft)
        }) {
            Ok(saved) => saved,
            Err(error) => {
                return project_session_store_error("commitProjectInteraction", error);
            }
        };
        self.revision = self.revision.saturating_add(1);
        self.draft = saved.draft;
        self.bundle_path = saved.bundle_path;
        self.project_json_path = saved.project_json_path;
        self.active_interactions.clear();
        self.command_state = response.command_state;
        self.selection = response.selection;

        to_runtime_value(ok_envelope(ProjectInteractionCommitResponse {
            session_id: self.session_id.clone(),
            interaction_id: interaction.interaction_id.clone(),
            kind: interaction.kind,
            base_revision: interaction.base_revision,
            revision: self.revision,
            generation: interaction.generation,
            accepted_sequence: interaction.accepted_sequence,
            coalesced_through: interaction.coalesced_through,
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

    fn intent_payload(
        &self,
        intent: ProjectIntent,
    ) -> std::result::Result<TimelineEditPayload, String> {
        match intent {
            ProjectIntent::ImportMaterial { .. } => {
                unreachable!("importMaterial is handled before timeline payload conversion")
            }
            ProjectIntent::AddTimelineSegmentIntent {
                material_id,
                target_start,
                target_track_handle,
            } => {
                let selection = self.selection_for_material_target_track(
                    target_track_handle.as_deref(),
                    &material_id,
                    "添加片段",
                )?;
                Ok(TimelineEditPayload::AddTimelineSegmentIntent(
                    AddTimelineSegmentIntentCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection,
                        material_id,
                        target_start: target_start.or(Some(self.playhead)),
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
            ProjectIntent::MoveSelectedSegmentIntent {
                start_at,
                target_track_handle,
            } => {
                let (segment_id, track_id) = self.selected_segment_location("移动片段")?;
                let target_track_id = match target_track_handle {
                    Some(handle) => self.track_id_from_timeline_item_handle(&handle)?,
                    None => track_id,
                };
                Ok(TimelineEditPayload::MoveSegment(
                    MoveSegmentCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_id,
                        target_track_id,
                        target_start: start_at,
                    },
                ))
            }
            ProjectIntent::SplitSelectedSegmentIntent {} => {
                Ok(TimelineEditPayload::SplitSelectedSegmentIntent(
                    SplitSelectedSegmentIntentCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        split_at: self.playhead,
                    },
                ))
            }
            ProjectIntent::TrimSelectedSegmentIntent { direction, trim_at } => {
                let segment = self.selected_segment("裁剪片段")?;
                let target_timerange = trim_target_timerange(segment, direction, trim_at)?;
                Ok(TimelineEditPayload::TrimSegment(
                    TrimSegmentCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_id: segment.segment_id.clone(),
                        direction,
                        target_timerange,
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
            ProjectIntent::AddTextSegmentIntent {
                content,
                target_start,
                target_track_handle,
            } => {
                let selection = self.selection_for_kind_target_track(
                    target_track_handle.as_deref(),
                    TrackKind::Text,
                    "添加文字",
                )?;
                Ok(TimelineEditPayload::AddTextSegmentIntent(
                    AddTextSegmentIntentCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection,
                        text: default_text_segment(content, TextSegmentSource::Text),
                        duration: None,
                        target_start: target_start.or(Some(self.playhead)),
                    },
                ))
            }
            ProjectIntent::EditSelectedText { patch } => {
                let text = self.apply_text_segment_patch(patch)?;
                Ok(TimelineEditPayload::EditTextSegment(
                    EditTextSegmentCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_id: self.selected_segment_id("编辑文字")?,
                        text,
                    },
                ))
            }
            ProjectIntent::ImportSubtitleSrtIntent { srt_content } => {
                Ok(TimelineEditPayload::ImportSubtitleSrtIntent(
                    ImportSubtitleSrtIntentCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        srt_content,
                        time_offset: Some(self.playhead),
                        style: default_text_style(),
                        text_box: default_subtitle_text_box(),
                        layout_region: default_subtitle_layout_region(),
                        wrapping: TextWrapping::default(),
                    },
                ))
            }
            ProjectIntent::AddAudioSegmentIntent { material_id } => Ok(
                TimelineEditPayload::AddAudioSegmentIntent(AddAudioSegmentIntentCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    material_id,
                    duration: None,
                    target_start: Some(self.playhead),
                }),
            ),
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
                    segment_id: self.selected_segment_id("编辑音频")?,
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
            ProjectIntent::SetSessionPlayhead { .. } => {
                unreachable!("setSessionPlayhead is handled before timeline payload conversion")
            }
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
            ProjectIntent::UpdateSelectedSegmentVisual { patch } => {
                let visual = self.apply_segment_visual_patch(patch)?;
                Ok(TimelineEditPayload::UpdateSegmentVisual(
                    UpdateSegmentVisualCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_id: self.selected_segment_id("编辑画面")?,
                        visual,
                    },
                ))
            }
            ProjectIntent::SetSelectedSegmentRetime { retiming } => Ok(
                TimelineEditPayload::SetSegmentRetime(SetSegmentRetimeCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    segment_id: self.selected_segment_id("调整速度")?,
                    retiming,
                }),
            ),
            ProjectIntent::ApplySelectedSegmentEffect { effect } => Ok(
                TimelineEditPayload::ApplySegmentEffect(ApplySegmentEffectCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    segment_id: self.selected_segment_id("应用效果")?,
                    effect,
                }),
            ),
            ProjectIntent::UpdateSelectedSegmentEffectParameter {
                effect_index,
                parameter,
            } => Ok(TimelineEditPayload::UpdateSegmentEffectParameter(
                UpdateSegmentEffectParameterCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    segment_id: self.selected_segment_id("调整效果")?,
                    effect_index,
                    parameter,
                },
            )),
            ProjectIntent::RemoveSelectedSegmentEffect { effect_index } => Ok(
                TimelineEditPayload::RemoveSegmentEffect(RemoveSegmentEffectCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    segment_id: self.selected_segment_id("移除效果")?,
                    effect_index,
                }),
            ),
            ProjectIntent::SetSelectedSegmentMask { mask } => Ok(
                TimelineEditPayload::SetSegmentMask(SetSegmentMaskCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    segment_id: self.selected_segment_id("设置遮罩")?,
                    mask,
                }),
            ),
            ProjectIntent::SetSelectedSegmentBlendMode { blend_mode } => Ok(
                TimelineEditPayload::SetSegmentBlendMode(SetSegmentBlendModeCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    segment_id: self.selected_segment_id("设置混合模式")?,
                    blend_mode,
                }),
            ),
            ProjectIntent::AddTransitionAtBoundary {
                from_segment_id,
                to_segment_id,
                reference,
                duration,
                parameters,
            } => Ok(TimelineEditPayload::AddTransition(
                AddTransitionCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    from_segment_id,
                    to_segment_id,
                    reference,
                    duration,
                    parameters,
                },
            )),
            ProjectIntent::UpdateSelectedTransitionDuration {
                from_segment_id,
                to_segment_id,
                duration,
            } => Ok(TimelineEditPayload::UpdateTransitionDuration(
                UpdateTransitionDurationCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    from_segment_id,
                    to_segment_id,
                    duration,
                },
            )),
            ProjectIntent::RemoveSelectedTransition {
                from_segment_id,
                to_segment_id,
            } => Ok(TimelineEditPayload::RemoveTransition(
                RemoveTransitionCommandPayload {
                    draft: self.draft.clone(),
                    command_state: self.command_state.clone(),
                    selection: self.selection.clone(),
                    from_segment_id,
                    to_segment_id,
                },
            )),
            ProjectIntent::SetSelectedSegmentKeyframe {
                property,
                interpolation,
                easing,
            } => {
                let (segment_id, keyframe) = self.keyframe_for_selected_segment(
                    property,
                    self.playhead,
                    interpolation,
                    easing,
                )?;
                Ok(TimelineEditPayload::SetSegmentKeyframe(
                    SetSegmentKeyframeCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_id,
                        replace_at: None,
                        keyframe,
                    },
                ))
            }
            ProjectIntent::EditSelectedSegmentKeyframe {
                property,
                at,
                from_at,
                value,
                interpolation,
                easing,
            } => {
                let (segment_id, replace_at, keyframe) = self.keyframe_edit_for_selected_segment(
                    property,
                    at,
                    from_at,
                    value,
                    interpolation,
                    easing,
                )?;
                Ok(TimelineEditPayload::SetSegmentKeyframe(
                    SetSegmentKeyframeCommandPayload {
                        draft: self.draft.clone(),
                        command_state: self.command_state.clone(),
                        selection: self.selection.clone(),
                        segment_id,
                        replace_at,
                        keyframe,
                    },
                ))
            }
            ProjectIntent::RemoveSelectedSegmentKeyframe { property } => {
                let segment = self.selected_segment("删除关键帧")?;
                let relative_at = nearest_keyframe_time(
                    segment,
                    &property,
                    relative_keyframe_time(segment, self.playhead),
                )?;
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

    fn apply_text_segment_patch(
        &self,
        patch: TextSegmentPatch,
    ) -> std::result::Result<TextSegment, String> {
        let mut text = self
            .selected_segment("编辑文字")?
            .text
            .clone()
            .ok_or_else(|| "编辑文字失败：选中的片段不是文字片段".to_owned())?;

        if let Some(content) = patch.content {
            if content.trim().is_empty() {
                return Err("编辑文字失败：文字内容不能为空".to_owned());
            }
            text.content = content;
        }
        if let Some(font_family) = patch.font_family {
            if font_family.trim().is_empty() {
                return Err("编辑文字失败：字体名称不能为空".to_owned());
            }
            text.style.font.family = font_family;
        }
        if let Some(font_ref) = patch.font_ref {
            if font_ref.trim().is_empty() {
                return Err("编辑文字失败：字体引用不能为空".to_owned());
            }
            text.style.font.font_ref = Some(font_ref);
        }
        if let Some(font_size) = patch.font_size {
            text.style.font_size = checked_u32("字号", font_size, 1, 400)?;
        }
        if let Some(color) = patch.color {
            text.style.color = color;
        }
        if let Some(alignment) = patch.alignment {
            text.style.alignment = alignment;
        }
        if let Some(line_height_millis) = patch.line_height_millis {
            text.style.line_height_millis = checked_u32("行高", line_height_millis, 500, 3_000)?;
        }
        if let Some(letter_spacing_millis) = patch.letter_spacing_millis {
            text.style.letter_spacing_millis =
                checked_u32("字间距", letter_spacing_millis, 0, 2_000)?;
        }
        text.style.stroke = apply_text_stroke_patch(
            text.style.stroke,
            patch.stroke_enabled,
            patch.stroke_color,
            patch.stroke_width,
        )?;
        text.style.shadow =
            apply_text_shadow_patch(text.style.shadow, patch.shadow_enabled, patch.shadow_color);
        text.style.background = apply_text_background_patch(
            text.style.background,
            patch.background_enabled,
            patch.background_color,
        );
        if let Some(width_millis) = patch.text_box_width_millis {
            text.text_box.width_millis = checked_u32("文本框宽", width_millis, 1, 1_000)?;
        }
        if let Some(height_millis) = patch.text_box_height_millis {
            text.text_box.height_millis = checked_u32("文本框高", height_millis, 1, 1_000)?;
        }
        if let Some(x_millis) = patch.layout_x_millis {
            text.layout_region.x_millis = checked_u32("布局 X", x_millis, 0, 1_000)?;
        }
        if let Some(y_millis) = patch.layout_y_millis {
            text.layout_region.y_millis = checked_u32("布局 Y", y_millis, 0, 1_000)?;
        }
        if let Some(width_millis) = patch.layout_width_millis {
            text.layout_region.width_millis = checked_u32("布局宽", width_millis, 1, 1_000)?;
        }
        if let Some(height_millis) = patch.layout_height_millis {
            text.layout_region.height_millis = checked_u32("布局高", height_millis, 1, 1_000)?;
        }
        if text.layout_region.x_millis + text.layout_region.width_millis > 1_000
            || text.layout_region.y_millis + text.layout_region.height_millis > 1_000
        {
            return Err("编辑文字失败：布局安全区域不能超出画布范围".to_owned());
        }
        if let Some(wrapping) = patch.wrapping {
            text.wrapping = wrapping;
        }

        Ok(text)
    }

    fn apply_segment_visual_patch(
        &self,
        patch: SegmentVisualPatch,
    ) -> std::result::Result<SegmentVisual, String> {
        let mut visual = self.selected_segment("编辑画面")?.visual.clone();

        if let Some(visible) = patch.visible {
            visual.visible = visible;
        }

        let mut position_x = visual.transform.position.x;
        let mut position_y = visual.transform.position.y;
        if let Some(x) = patch.position_x {
            position_x = checked_i32("位置 X", x, -1_000, 1_000)?;
        }
        if let Some(y) = patch.position_y {
            position_y = checked_i32("位置 Y", y, -1_000, 1_000)?;
        }
        if let Some(delta_x) = patch.position_delta_x {
            position_x = clamp_i32(position_x.saturating_add(delta_x), -1_000, 1_000);
        }
        if let Some(delta_y) = patch.position_delta_y {
            position_y = clamp_i32(position_y.saturating_add(delta_y), -1_000, 1_000);
        }
        visual.transform.position = SegmentPosition {
            x: position_x,
            y: position_y,
        };

        if let Some(x_millis) = patch.scale_x_millis {
            visual.transform.scale.x_millis = checked_u32("缩放 X", x_millis, 1, 3_000)?;
        }
        if let Some(y_millis) = patch.scale_y_millis {
            visual.transform.scale.y_millis = checked_u32("缩放 Y", y_millis, 1, 3_000)?;
        }
        if let Some(rotation_degrees) = patch.rotation_degrees {
            visual.transform.rotation.degrees = checked_i32("旋转", rotation_degrees, -360, 360)?;
        }
        if let Some(delta_degrees) = patch.rotation_delta_degrees {
            visual.transform.rotation.degrees =
                normalize_rotation_degrees(visual.transform.rotation.degrees + delta_degrees);
        }
        if let Some(opacity_millis) = patch.opacity_millis {
            visual.transform.opacity.value_millis =
                checked_u32("不透明度", opacity_millis, 0, 1_000)?;
        }
        if let Some(left_millis) = patch.crop_left_millis {
            visual.transform.crop.left_millis = checked_u32("左裁剪", left_millis, 0, 999)?;
        }
        if let Some(right_millis) = patch.crop_right_millis {
            visual.transform.crop.right_millis = checked_u32("右裁剪", right_millis, 0, 999)?;
        }
        if let Some(top_millis) = patch.crop_top_millis {
            visual.transform.crop.top_millis = checked_u32("上裁剪", top_millis, 0, 999)?;
        }
        if let Some(bottom_millis) = patch.crop_bottom_millis {
            visual.transform.crop.bottom_millis = checked_u32("下裁剪", bottom_millis, 0, 999)?;
        }
        if visual.transform.crop.left_millis + visual.transform.crop.right_millis >= 1_000
            || visual.transform.crop.top_millis + visual.transform.crop.bottom_millis >= 1_000
        {
            return Err("编辑画面失败：左右或上下裁剪总和必须小于 1000".to_owned());
        }
        if let Some(fit_mode) = patch.fit_mode {
            visual.fit_mode = fit_mode;
        }
        if let Some(background_kind) = patch.background_kind {
            visual.background_filling = match background_kind {
                SegmentBackgroundFillingPatchKind::None => SegmentBackgroundFilling::None,
                SegmentBackgroundFillingPatchKind::Black => SegmentBackgroundFilling::Black,
                SegmentBackgroundFillingPatchKind::Blur => SegmentBackgroundFilling::Blur,
                SegmentBackgroundFillingPatchKind::SolidColor => {
                    SegmentBackgroundFilling::SolidColor {
                        color: patch.background_color.clone().unwrap_or_else(|| {
                            match &visual.background_filling {
                                SegmentBackgroundFilling::SolidColor { color } => color.clone(),
                                _ => "#000000".to_owned(),
                            }
                        }),
                    }
                }
                SegmentBackgroundFillingPatchKind::Image => match &visual.background_filling {
                    SegmentBackgroundFilling::Image { material_id } => {
                        SegmentBackgroundFilling::Image {
                            material_id: material_id.clone(),
                        }
                    }
                    _ => SegmentBackgroundFilling::Image { material_id: None },
                },
            };
        } else if let Some(background_color) = patch.background_color {
            if matches!(
                visual.background_filling,
                SegmentBackgroundFilling::SolidColor { .. }
            ) {
                visual.background_filling = SegmentBackgroundFilling::SolidColor {
                    color: background_color,
                };
            }
        }

        Ok(visual)
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

    fn track_id_from_timeline_item_handle(
        &self,
        item_handle: &str,
    ) -> std::result::Result<TrackId, String> {
        let selection = self.selection_from_timeline_item_handle(item_handle)?;
        if selection.segment_ids.is_empty() && selection.track_ids.len() == 1 {
            return Ok(selection.track_ids[0].clone());
        }
        Err("Move target must be a timeline track selection handle".to_owned())
    }

    fn selection_for_kind_target_track(
        &self,
        target_track_handle: Option<&str>,
        expected_kind: TrackKind,
        action: &str,
    ) -> std::result::Result<TimelineSelection, String> {
        let Some(handle) = target_track_handle else {
            return Ok(self.selection.clone());
        };
        let track_id = self.track_id_from_timeline_item_handle(handle)?;
        let track = self
            .draft
            .tracks
            .iter()
            .find(|track| track.track_id == track_id)
            .ok_or_else(|| format!("{action}失败：目标轨道不存在"))?;
        if track.kind != expected_kind {
            return Err(format!(
                "{action}失败：目标轨道 {} 不是 {:?} 轨道",
                track.name, expected_kind
            ));
        }
        Ok(TimelineSelection {
            segment_ids: Vec::new(),
            track_ids: vec![track_id],
        })
    }

    fn selection_for_material_target_track(
        &self,
        target_track_handle: Option<&str>,
        material_id: &MaterialId,
        action: &str,
    ) -> std::result::Result<TimelineSelection, String> {
        let Some(handle) = target_track_handle else {
            return Ok(self.selection.clone());
        };
        let track_id = self.track_id_from_timeline_item_handle(handle)?;
        let track = self
            .draft
            .tracks
            .iter()
            .find(|track| track.track_id == track_id)
            .ok_or_else(|| format!("{action}失败：目标轨道不存在"))?;
        let material = self
            .draft
            .materials
            .iter()
            .find(|material| &material.material_id == material_id)
            .ok_or_else(|| format!("{action}失败：素材不存在 {}", material_id.as_str()))?;
        if !track_accepts_material_kind(track.kind, material.kind) {
            return Err(format!(
                "{action}失败：素材 {:?} 不能添加到 {:?} 轨道",
                material.kind, track.kind
            ));
        }
        Ok(TimelineSelection {
            segment_ids: Vec::new(),
            track_ids: vec![track_id],
        })
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

    fn selected_segment_location(
        &self,
        action: &str,
    ) -> std::result::Result<(SegmentId, TrackId), String> {
        let Some(segment_id) = self.selection.segment_ids.first() else {
            return Err(format!("{action}失败：请先选择一个片段"));
        };
        self.draft
            .tracks
            .iter()
            .find_map(|track| {
                track
                    .segments
                    .iter()
                    .any(|segment| &segment.segment_id == segment_id)
                    .then(|| (segment_id.clone(), track.track_id.clone()))
            })
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

    fn keyframe_edit_for_selected_segment(
        &self,
        property: KeyframeProperty,
        at: Microseconds,
        from_at: Option<Microseconds>,
        value: Option<KeyframeValue>,
        interpolation: Option<KeyframeInterpolation>,
        easing: Option<KeyframeEasing>,
    ) -> std::result::Result<(SegmentId, Option<Microseconds>, Keyframe), String> {
        let segment = self.selected_segment("编辑关键帧")?;
        if at > segment.target_timerange.duration {
            return Err(format!(
                "编辑关键帧失败：关键帧时间 {} 超出片段时长 {}",
                at.get(),
                segment.target_timerange.duration.get()
            ));
        }
        if let Some(source_at) = from_at {
            if source_at > segment.target_timerange.duration {
                return Err(format!(
                    "编辑关键帧失败：源关键帧时间 {} 超出片段时长 {}",
                    source_at.get(),
                    segment.target_timerange.duration.get()
                ));
            }
        }

        let source_keyframe = from_at.and_then(|source_at| {
            segment
                .keyframes
                .iter()
                .find(|keyframe| keyframe.property == property && keyframe.at == source_at)
        });
        let resolved_value = if let Some(value) = value {
            value
        } else if let Some(source_keyframe) = source_keyframe {
            source_keyframe.value.clone()
        } else {
            keyframe_value_for_segment(segment, &property)?
        };
        let resolved_interpolation = interpolation
            .or_else(|| source_keyframe.map(|keyframe| keyframe.interpolation))
            .unwrap_or(KeyframeInterpolation::Linear);
        let resolved_easing = easing
            .or_else(|| source_keyframe.map(|keyframe| keyframe.easing))
            .unwrap_or(KeyframeEasing::None);

        Ok((
            segment.segment_id.clone(),
            from_at,
            Keyframe {
                at,
                property,
                value: resolved_value,
                interpolation: resolved_interpolation,
                easing: resolved_easing,
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
        let fs = StdPlatformFileSystem;
        let material_path = PathBuf::from(material_path);
        let mut request =
            crate::material_service::ImportMaterialRequest::new(material_path.clone());
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
        let imported = match crate::material_service::queue_material_import(
            &mut next_draft,
            request,
            &fs,
            &self.bundle_path,
        ) {
            Ok(imported) => imported,
            Err(error) => {
                return to_runtime_value(material_service_error_envelope(
                    "executeProjectIntent",
                    error,
                ));
            }
        };

        let saved = match run_project_io_job("import-material", self.revision, || {
            save_project_bundle(&fs, &self.bundle_path, &next_draft)
        }) {
            Ok(saved) => saved,
            Err(error) => {
                return project_session_store_error("executeProjectIntent", error);
            }
        };

        self.revision = self.revision.saturating_add(1);
        self.draft = saved.draft;
        self.bundle_path = saved.bundle_path;
        self.project_json_path = saved.project_json_path;
        self.active_interactions.clear();
        let material = imported.material;
        let probe_result = enqueue_material_probe(ScheduledMaterialProbe {
            session_id: self.session_id.clone(),
            expected_revision: self.revision,
            material_id: material.material_id.clone(),
            material_uri: material.uri.clone(),
            material_path: material_path.clone(),
            task_token: TaskCancellationToken::new(0),
        });
        let (probe_status, probe_job_id, diagnostic) = match probe_result {
            Ok(job_id) => (
                ProjectSessionProbeStatus::Queued,
                Some(job_id),
                imported.diagnostic.map(command_diagnostic),
            ),
            Err(error) => (
                ProjectSessionProbeStatus::Failed,
                None,
                Some(material_probe_schedule_diagnostic(
                    &material,
                    &material_path,
                    error,
                )),
            ),
        };
        let delta = material_dependency_delta(
            CommandDeltaName::ImportMaterial,
            &self.draft,
            std::slice::from_ref(&material.material_id),
            "material imported",
        );

        to_runtime_value(ok_envelope(ProjectSessionImportMaterialResponse {
            session_id: self.session_id.clone(),
            revision: self.revision,
            material,
            materials: crate::material_service::list_materials(&self.draft),
            probe_status,
            probe_job_id,
            diagnostic,
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

    fn import_kaipai_formula_bundle(
        &mut self,
        request: ImportKaipaiFormulaBundleRequest,
    ) -> Result<serde_json::Value> {
        let command = "importKaipaiFormulaBundle";
        let formula_bundle_path = PathBuf::from(&request.bundle_path);
        let formula_json = match fs::read_to_string(&formula_bundle_path) {
            Ok(contents) => contents,
            Err(error) => {
                return to_runtime_value(error_envelope(
                    CommandErrorKind::InvalidPayload,
                    format!(
                        "Unable to read Kaipai formula bundle {}: {error}",
                        formula_bundle_path.display()
                    ),
                    Some(command.to_string()),
                ));
            }
        };
        let formula_bundle = match KaipaiFormulaBundle::from_json_str(&formula_json) {
            Ok(bundle) => bundle,
            Err(error) => {
                return to_runtime_value(error_envelope(
                    CommandErrorKind::InvalidPayload,
                    format!("Invalid Kaipai formula bundle: {error}"),
                    Some(command.to_string()),
                ));
            }
        };

        let import_id = request
            .import_id
            .clone()
            .unwrap_or_else(|| default_template_import_id(&formula_bundle_path));
        let mut options = KaipaiImportOptions::new(
            self.bundle_path.clone(),
            PathBuf::from(&request.resource_root),
            import_id,
        );
        options.generated_at = request.generated_at.clone();
        options.verify_resource_sha256 = request.verify_resource_sha256.unwrap_or(true);

        let mapped = match map_kaipai_bundle_to_import_plan(&formula_bundle, options) {
            Ok(mapped) => mapped,
            Err(error) => {
                return to_runtime_value(error_envelope(
                    CommandErrorKind::InvalidPayload,
                    format!("Kaipai formula bundle mapping failed: {error}"),
                    Some(command.to_string()),
                ));
            }
        };
        let localized_resource_files = localized_resource_file_refs(&mapped.localized_resources);
        let localized_resources = localized_resource_refs(&mapped.localized_resources);
        let applied = match apply_import_plan_to_draft(DraftImportApplicationInput {
            plan: mapped.plan,
            source_kind: mapped.report.source_kind,
            generated_at: mapped.report.generated_at,
            report_items: mapped.report.items,
        }) {
            Ok(applied) => applied,
            Err(error) => {
                cleanup_localized_resource_files(&self.bundle_path, &localized_resource_files);
                return to_runtime_value(error_envelope(
                    CommandErrorKind::InvalidPayload,
                    format!("Draft import plan validation failed: {error}"),
                    Some(command.to_string()),
                ));
            }
        };

        let fs = StdPlatformFileSystem;
        let previous_draft = self.draft.clone();
        let target_bundle_path = self.bundle_path.clone();
        let next_draft = applied.draft;
        let saved = match run_project_io_job("import-kaipai-formula", self.revision, || {
            let saved = save_project_bundle(&fs, &target_bundle_path, &next_draft)
                .map_err(ProjectSessionImportPersistError::ProjectStore)?;
            match index_draft_resources_with_extra_refs(
                &saved.bundle_path,
                &saved.draft,
                localized_resources,
            ) {
                Ok(_) => Ok(saved),
                Err(error) => {
                    let _ = save_project_bundle(&fs, &target_bundle_path, &previous_draft);
                    let _ = index_draft_resources(&target_bundle_path, &previous_draft);
                    cleanup_localized_resource_files(
                        &target_bundle_path,
                        &localized_resource_files,
                    );
                    Err(ProjectSessionImportPersistError::ArtifactStore(error))
                }
            }
        }) {
            Ok(saved) => saved,
            Err(ProjectSessionImportPersistError::ProjectStore(error)) => {
                cleanup_localized_resource_files(&self.bundle_path, &localized_resource_files);
                return project_session_store_error(command, error);
            }
            Err(ProjectSessionImportPersistError::ArtifactStore(error)) => {
                return to_runtime_value(error_envelope(
                    CommandErrorKind::ProjectIoFailed,
                    format!("Failed to persist Kaipai import resource index: {error}"),
                    Some(command.to_string()),
                ));
            }
        };

        self.revision = self.revision.saturating_add(1);
        self.draft = saved.draft;
        self.bundle_path = saved.bundle_path;
        self.project_json_path = saved.project_json_path;
        self.active_interactions.clear();
        self.command_state = CommandState::empty();
        self.selection = TimelineSelection::empty();
        self.playhead = Microseconds::ZERO;
        let delta = template_import_delta(&self.draft);
        let events = vec![CommandEvent {
            kind: "templateImported".to_owned(),
            message: None,
        }];

        to_runtime_value(ok_envelope(ProjectSessionTemplateImportResponse {
            session_id: self.session_id.clone(),
            revision: self.revision,
            view_model: project_session_view_model(
                &self.draft,
                &self.command_state,
                &self.selection,
            ),
            events,
            delta,
            adaptation_report: applied.report,
            bundle_path: self.bundle_path.display().to_string(),
            project_json_path: self.project_json_path.display().to_string(),
        }))
    }

    fn apply_response(&mut self, response: TimelineCommandResponse) -> Result<serde_json::Value> {
        if is_selection_only_delta(&response.delta) {
            self.command_state = response.command_state;
            self.selection = response.selection;
            return to_runtime_value(ok_envelope(ProjectSessionIntentResponse {
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
        let saved = match run_project_io_job("timeline-save", self.revision, || {
            save_project_bundle(&fs, &self.bundle_path, &response.draft)
        }) {
            Ok(saved) => saved,
            Err(error) => {
                return project_session_store_error("executeProjectIntent", error);
            }
        };
        self.revision = self.revision.saturating_add(1);
        self.draft = saved.draft;
        self.bundle_path = saved.bundle_path;
        self.project_json_path = saved.project_json_path;
        self.active_interactions.clear();
        self.command_state = response.command_state;
        self.selection = response.selection;

        to_runtime_value(ok_envelope(ProjectSessionIntentResponse {
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

    fn apply_session_playhead(&mut self, playhead: Microseconds) -> Result<serde_json::Value> {
        self.playhead = playhead;
        to_runtime_value(ok_envelope(ProjectSessionIntentResponse {
            session_id: self.session_id.clone(),
            revision: self.revision,
            view_model: project_session_view_model(
                &self.draft,
                &self.command_state,
                &self.selection,
            ),
            events: Vec::new(),
            delta: CommandDelta::none(
                CommandDeltaName::SelectTimelineSegments,
                "session playhead changed",
            ),
            bundle_path: self.bundle_path.display().to_string(),
            project_json_path: self.project_json_path.display().to_string(),
        }))
    }
}

fn track_accepts_material_kind(track_kind: TrackKind, material_kind: MaterialKind) -> bool {
    match track_kind {
        TrackKind::Video => matches!(material_kind, MaterialKind::Video | MaterialKind::Image),
        TrackKind::Audio => material_kind == MaterialKind::Audio,
        TrackKind::Text => material_kind == MaterialKind::Text,
        TrackKind::Sticker => material_kind == MaterialKind::Sticker,
        TrackKind::Filter => matches!(material_kind, MaterialKind::Video | MaterialKind::Image),
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
    let production_effect_capabilities = EffectCapabilityRegistry::phase19_first_party();
    let selected_track = selected_track_view_model(draft, selection);
    let selected_segment = selected_segment_view_model(draft, selection);

    ProjectSessionViewModel {
        project,
        edit_controls,
        timeline,
        production_effect_capabilities,
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
            let registry = EffectCapabilityRegistry::phase19_first_party();
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
                retiming: segment.retiming.clone(),
                filters: segment.filters.clone(),
                transition: segment.transition.clone(),
                visual: segment.visual.clone(),
                volume: segment.volume,
                audio: segment.audio.clone(),
                text: segment.text.clone(),
                keyframes: segment.keyframes.clone(),
                has_text,
                has_audio_controls,
                phase19: selected_segment_phase19_view_model(track, segment, &registry),
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
        kind: track.kind,
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
        retime_label: format_segment_retime_label(&segment.retiming),
        speed_adjusted: is_segment_speed_adjusted(&segment.retiming),
        effect_count: segment.filters.len(),
        mask_label: segment
            .visual
            .mask
            .display_name()
            .map(|label| localize_mask_label(&label)),
        blend_label: localize_blend_mode_label(&segment.visual.blend_mode),
        transition_label: segment
            .transition
            .as_ref()
            .map(|transition| localize_transition_label(&transition.reference)),
        transition_duration: segment
            .transition
            .as_ref()
            .map(|transition| transition.duration),
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
                property: keyframe.property.clone(),
                at: keyframe.at,
                position_per_mille,
                title: format!("{property_label}关键帧 {time_label} · {easing_label}"),
                aria_label: format!("{segment_label} {property_label}关键帧 {time_label}"),
            }
        })
        .collect()
}

fn selected_segment_phase19_view_model(
    track: &Track,
    segment: &Segment,
    registry: &EffectCapabilityRegistry,
) -> SelectedSegmentPhase19ViewModel {
    let mut support_chips = Vec::new();
    push_capability_chip(
        &mut support_chips,
        registry,
        segment.retiming.mode.capability_id(),
    );
    for filter in &segment.filters {
        push_capability_chip(&mut support_chips, registry, &filter.capability_id());
    }
    if let Some(kind) = segment.visual.mask.mask_kind() {
        push_capability_chip(&mut support_chips, registry, kind.capability_id());
    }
    if let Some(kind) = segment.visual.blend_mode.kind() {
        push_capability_chip(&mut support_chips, registry, kind.capability_id());
    }
    if let Some(transition) = &segment.transition {
        push_capability_chip(&mut support_chips, registry, &transition.capability_id());
    }
    support_chips.sort_by(|left, right| left.capability_id.cmp(&right.capability_id));
    support_chips.dedup_by(|left, right| left.capability_id == right.capability_id);

    SelectedSegmentPhase19ViewModel {
        retime_label: format_segment_retime_label(&segment.retiming),
        audio_retime_label: localize_audio_retime_policy(segment.retiming.audio_policy),
        effect_count: segment.filters.len(),
        mask_label: segment
            .visual
            .mask
            .display_name()
            .map(|label| localize_mask_label(&label))
            .unwrap_or_else(|| "无蒙版".to_owned()),
        blend_label: localize_blend_mode_label(&segment.visual.blend_mode),
        transition_label: segment
            .transition
            .as_ref()
            .map(|transition| localize_transition_label(&transition.reference)),
        support_chips,
        transition_boundary: selected_segment_transition_boundary_view_model(track, segment),
    }
}

fn push_capability_chip(
    chips: &mut Vec<ProductionCapabilityChipViewModel>,
    registry: &EffectCapabilityRegistry,
    capability_id: &str,
) {
    if let Some(entry) = registry.entry(capability_id) {
        chips.push(production_capability_chip_view_model(entry));
    }
}

fn production_capability_chip_view_model(
    entry: &CapabilityReportItem,
) -> ProductionCapabilityChipViewModel {
    ProductionCapabilityChipViewModel {
        capability_id: entry.capability_id.clone(),
        label: localize_capability_display_name(entry),
        preview_label: capability_support_label("预览", &entry.preview),
        export_label: capability_support_label("导出", &entry.export),
        tone: capability_tone(&entry.preview, &entry.export),
    }
}

fn capability_support_label(surface_label: &str, support: &CapabilitySupport) -> String {
    match support {
        CapabilitySupport::Supported { .. } => format!("{surface_label}支持"),
        CapabilitySupport::Degraded { .. } => format!("{surface_label}降级"),
        CapabilitySupport::Unsupported { .. } => "暂不支持".to_owned(),
        CapabilitySupport::ExternalReference { .. } => "外部参考".to_owned(),
    }
}

fn capability_tone(
    preview: &CapabilitySupport,
    export: &CapabilitySupport,
) -> ProductionCapabilityTone {
    if matches!(preview, CapabilitySupport::ExternalReference { .. })
        || matches!(export, CapabilitySupport::ExternalReference { .. })
    {
        return ProductionCapabilityTone::Muted;
    }
    if preview.is_supported() && export.is_supported() {
        return ProductionCapabilityTone::Ready;
    }
    if matches!(preview, CapabilitySupport::Unsupported { .. })
        || matches!(export, CapabilitySupport::Unsupported { .. })
    {
        return ProductionCapabilityTone::Error;
    }
    ProductionCapabilityTone::Warning
}

fn localize_capability_display_name(entry: &CapabilityReportItem) -> String {
    match entry.capability_id.as_str() {
        "retime.constantSpeed" => "常规变速".to_owned(),
        "retime.speedCurve" => "曲线变速".to_owned(),
        "transition.dissolve" => "叠化".to_owned(),
        "effect.gaussianBlur" => "高斯模糊".to_owned(),
        "effect.basicColorAdjustment" => "基础调色".to_owned(),
        "effect.opacityAdjustment" => "不透明度".to_owned(),
        "mask.rectangle" => "矩形蒙版".to_owned(),
        "mask.ellipse" => "椭圆蒙版".to_owned(),
        "blend.normal" => "正常混合".to_owned(),
        "blend.multiply" => "正片叠底".to_owned(),
        "blend.screen" => "滤色".to_owned(),
        _ => entry.display_name.clone(),
    }
}

fn selected_segment_transition_boundary_view_model(
    track: &Track,
    segment: &Segment,
) -> Option<SelectedSegmentTransitionBoundaryViewModel> {
    if !matches!(track.kind, TrackKind::Video | TrackKind::Filter) {
        return None;
    }
    let segment_index = track
        .segments
        .iter()
        .position(|candidate| candidate.segment_id == segment.segment_id)?;
    let (from_segment, to_segment) =
        if let Some(next_segment) = track.segments.get(segment_index + 1) {
            (segment, next_segment)
        } else if segment_index > 0 {
            (track.segments.get(segment_index - 1)?, segment)
        } else {
            return None;
        };
    let transition = transition_for_boundary(track, from_segment, to_segment);
    let duration = transition
        .as_ref()
        .map(|candidate| candidate.duration)
        .unwrap_or_else(|| Microseconds::new(500_000));
    Some(SelectedSegmentTransitionBoundaryViewModel {
        from_segment_id: from_segment.segment_id.clone(),
        to_segment_id: to_segment.segment_id.clone(),
        label: format!(
            "{} → {}",
            from_segment.segment_id.as_str(),
            to_segment.segment_id.as_str()
        ),
        duration,
        has_transition: transition.is_some(),
    })
}

fn transition_for_boundary(
    track: &Track,
    from_segment: &Segment,
    to_segment: &Segment,
) -> Option<Transition> {
    if from_segment
        .transition
        .as_ref()
        .is_some_and(|transition| transition.capability_id() == "transition.dissolve")
    {
        return from_segment.transition.clone();
    }
    track
        .transitions
        .iter()
        .find(|candidate| {
            candidate.from_segment_id == from_segment.segment_id
                && candidate.to_segment_id == to_segment.segment_id
        })
        .map(|candidate| Transition {
            reference: candidate.reference.clone(),
            duration: candidate.duration,
        })
}

fn format_segment_retime_label(retiming: &SegmentRetiming) -> String {
    match &retiming.mode {
        draft_model::RetimeMode::Constant { speed } => {
            if speed.denominator == 0 {
                return "常规变速".to_owned();
            }
            let per_mille = (u64::from(speed.numerator) * 1_000) / u64::from(speed.denominator);
            format!("{}.{:01}x", per_mille / 1_000, (per_mille % 1_000) / 100)
        }
        draft_model::RetimeMode::SpeedCurve { .. } => "曲线变速".to_owned(),
    }
}

fn is_segment_speed_adjusted(retiming: &SegmentRetiming) -> bool {
    match &retiming.mode {
        draft_model::RetimeMode::Constant { speed } => speed.numerator != speed.denominator,
        draft_model::RetimeMode::SpeedCurve { .. } => true,
    }
}

fn localize_audio_retime_policy(policy: draft_model::AudioRetimePolicy) -> String {
    match policy {
        draft_model::AudioRetimePolicy::FollowVideoSpeed => "音频跟随变速".to_owned(),
        draft_model::AudioRetimePolicy::PreservePitch => "保持音调暂不支持".to_owned(),
        draft_model::AudioRetimePolicy::MuteUnsupported => "不支持时静音".to_owned(),
    }
}

fn localize_mask_label(label: &str) -> String {
    match label {
        "rectangle" => "矩形蒙版".to_owned(),
        "ellipse" => "椭圆蒙版".to_owned(),
        _ => label.to_owned(),
    }
}

fn localize_blend_mode_label(blend_mode: &SegmentBlendMode) -> String {
    match blend_mode.kind() {
        Some(BlendModeKind::Normal) => "正常".to_owned(),
        Some(BlendModeKind::Multiply) => "正片叠底".to_owned(),
        Some(BlendModeKind::Screen) => "滤色".to_owned(),
        None => blend_mode.display_name(),
    }
}

fn localize_transition_label(reference: &TransitionReference) -> String {
    match reference {
        TransitionReference::FirstParty { .. } => "叠化".to_owned(),
        TransitionReference::ExternalReference { reference } => reference
            .display_name
            .clone()
            .unwrap_or_else(|| format!("{}:{}", reference.provider, reference.effect_id)),
    }
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

fn relative_keyframe_time(segment: &Segment, timeline_at: Microseconds) -> Microseconds {
    let segment_start = segment.target_timerange.start.get();
    let segment_duration = segment.target_timerange.duration.get();
    let relative = timeline_at.get().saturating_sub(segment_start);
    Microseconds::new(relative.min(segment_duration))
}

fn nearest_keyframe_time(
    segment: &Segment,
    property: &KeyframeProperty,
    relative_at: Microseconds,
) -> std::result::Result<Microseconds, String> {
    segment
        .keyframes
        .iter()
        .filter(|keyframe| &keyframe.property == property)
        .min_by_key(|keyframe| keyframe.at.get().abs_diff(relative_at.get()))
        .map(|keyframe| keyframe.at)
        .ok_or_else(|| format!("删除关键帧失败：当前属性 {:?} 没有关键帧", property))
}

fn trim_target_timerange(
    segment: &Segment,
    direction: TrimSegmentDirection,
    trim_at: Microseconds,
) -> std::result::Result<TargetTimerange, String> {
    let old_start = segment.target_timerange.start.get();
    let old_duration = segment.target_timerange.duration.get();
    if old_duration == 0 {
        return Err("裁剪片段失败：片段时长无效".to_owned());
    }
    let old_end = old_start
        .checked_add(old_duration)
        .ok_or_else(|| "裁剪片段失败：片段时间范围溢出".to_owned())?;

    match direction {
        TrimSegmentDirection::Left => {
            let new_start = trim_at.get().clamp(old_start, old_end.saturating_sub(1));
            Ok(TargetTimerange {
                start: Microseconds::new(new_start),
                duration: Microseconds::new(old_end.saturating_sub(new_start).max(1)),
            })
        }
        TrimSegmentDirection::Right => {
            let new_end = trim_at.get().clamp(old_start.saturating_add(1), old_end);
            Ok(TargetTimerange {
                start: Microseconds::new(old_start),
                duration: Microseconds::new(new_end.saturating_sub(old_start).max(1)),
            })
        }
    }
}

fn apply_text_stroke_patch(
    current: Option<TextStroke>,
    enabled: Option<bool>,
    color: Option<String>,
    width: Option<u32>,
) -> std::result::Result<Option<TextStroke>, String> {
    match enabled {
        Some(false) => Ok(None),
        Some(true) => Ok(Some(TextStroke {
            color: color
                .or_else(|| current.as_ref().map(|stroke| stroke.color.clone()))
                .unwrap_or_else(|| "#000000".to_owned()),
            width: checked_u32(
                "描边宽度",
                width
                    .or_else(|| current.as_ref().map(|stroke| stroke.width))
                    .unwrap_or(2),
                1,
                120,
            )?,
        })),
        None => {
            if color.is_none() && width.is_none() {
                return Ok(current);
            }
            Ok(Some(TextStroke {
                color: color
                    .or_else(|| current.as_ref().map(|stroke| stroke.color.clone()))
                    .unwrap_or_else(|| "#000000".to_owned()),
                width: checked_u32(
                    "描边宽度",
                    width
                        .or_else(|| current.as_ref().map(|stroke| stroke.width))
                        .unwrap_or(2),
                    1,
                    120,
                )?,
            }))
        }
    }
}

fn apply_text_shadow_patch(
    current: Option<TextShadow>,
    enabled: Option<bool>,
    color: Option<String>,
) -> Option<TextShadow> {
    match enabled {
        Some(false) => None,
        Some(true) => Some(TextShadow {
            color: color
                .or_else(|| current.as_ref().map(|shadow| shadow.color.clone()))
                .unwrap_or_else(|| "#222222".to_owned()),
            offset_x: current.as_ref().map(|shadow| shadow.offset_x).unwrap_or(2),
            offset_y: current.as_ref().map(|shadow| shadow.offset_y).unwrap_or(2),
            blur: current.as_ref().map(|shadow| shadow.blur).unwrap_or(4),
        }),
        None => color
            .map(|color| TextShadow {
                color,
                offset_x: current.as_ref().map(|shadow| shadow.offset_x).unwrap_or(2),
                offset_y: current.as_ref().map(|shadow| shadow.offset_y).unwrap_or(2),
                blur: current.as_ref().map(|shadow| shadow.blur).unwrap_or(4),
            })
            .or(current),
    }
}

fn apply_text_background_patch(
    current: Option<TextBackground>,
    enabled: Option<bool>,
    color: Option<String>,
) -> Option<TextBackground> {
    match enabled {
        Some(false) => None,
        Some(true) => Some(TextBackground {
            color: color
                .or_else(|| current.as_ref().map(|background| background.color.clone()))
                .unwrap_or_else(|| "#000000".to_owned()),
        }),
        None => color.map(|color| TextBackground { color }).or(current),
    }
}

fn checked_u32(label: &str, value: u32, min: u32, max: u32) -> std::result::Result<u32, String> {
    if value < min || value > max {
        return Err(format!("{label}必须在 {min} 到 {max} 之间"));
    }
    Ok(value)
}

fn checked_i32(label: &str, value: i32, min: i32, max: i32) -> std::result::Result<i32, String> {
    if value < min || value > max {
        return Err(format!("{label}必须在 {min} 到 {max} 之间"));
    }
    Ok(value)
}

fn clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}

fn normalize_rotation_degrees(value: i32) -> i32 {
    let mut normalized = value;
    while normalized > 180 {
        normalized -= 360;
    }
    while normalized < -180 {
        normalized += 360;
    }
    normalized
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

fn default_text_segment(content: String, source: TextSegmentSource) -> TextSegment {
    TextSegment {
        content,
        source,
        style: default_text_style(),
        text_box: TextBox::default(),
        layout_region: default_text_layout_region(source),
        wrapping: TextWrapping::default(),
        bubble: None,
        effect: None,
    }
}

fn default_text_style() -> TextStyle {
    TextStyle {
        font_size: 36,
        color: "#ffffff".to_owned(),
        alignment: TextAlignment::Center,
        line_height_millis: 1_200,
        letter_spacing_millis: 0,
        stroke: Some(TextStroke {
            color: "#000000".to_owned(),
            width: 2,
        }),
        shadow: Some(TextShadow {
            color: "#222222".to_owned(),
            offset_x: 2,
            offset_y: 2,
            blur: 4,
        }),
        ..TextStyle::default()
    }
}

fn default_text_layout_region(source: TextSegmentSource) -> TextLayoutRegion {
    match source {
        TextSegmentSource::Text => TextLayoutRegion::safe_area(),
        TextSegmentSource::Subtitle => default_subtitle_layout_region(),
    }
}

fn default_subtitle_text_box() -> TextBox {
    TextBox {
        width_millis: 800,
        height_millis: 180,
    }
}

fn default_subtitle_layout_region() -> TextLayoutRegion {
    TextLayoutRegion {
        x_millis: 100,
        y_millis: 720,
        width_millis: 800,
        height_millis: 180,
    }
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

fn localized_resource_refs(
    manifest: &LocalizedResourceManifest,
) -> Vec<(ResourceRef, Option<String>)> {
    manifest
        .resources
        .iter()
        .filter(|resource| resource.status == LocalizedResourceStatus::Available)
        .map(|resource| {
            (
                ResourceRef::new(
                    localized_resource_kind(resource.resource_index_ref.kind),
                    resource.resource_index_ref.resource_id.clone(),
                    resource.resource_index_ref.stable_key.clone(),
                ),
                resource.project_relative_ref.clone(),
            )
        })
        .collect()
}

fn localized_resource_file_refs(manifest: &LocalizedResourceManifest) -> Vec<String> {
    manifest
        .resources
        .iter()
        .filter(|resource| resource.status == LocalizedResourceStatus::Available)
        .filter_map(|resource| resource.project_relative_ref.clone())
        .filter(|project_relative_ref| is_template_import_resource_ref(project_relative_ref))
        .collect()
}

fn cleanup_localized_resource_files(bundle_path: &Path, project_relative_refs: &[String]) {
    for project_relative_ref in project_relative_refs {
        if !is_template_import_resource_ref(project_relative_ref) {
            continue;
        }
        let path = bundle_path.join(project_relative_ref);
        if path.is_file() {
            let _ = fs::remove_file(&path);
            prune_empty_template_import_dirs(bundle_path, path.parent());
        }
    }
}

fn prune_empty_template_import_dirs(bundle_path: &Path, start: Option<&Path>) {
    let stop = bundle_path.join("resources").join("template-import");
    let mut current = start;
    while let Some(path) = current {
        if path == stop || !path.starts_with(&stop) {
            break;
        }
        if fs::remove_dir(path).is_err() {
            break;
        }
        current = path.parent();
    }
}

fn is_template_import_resource_ref(project_relative_ref: &str) -> bool {
    if !project_relative_ref.starts_with("resources/template-import/") {
        return false;
    }
    let path = Path::new(project_relative_ref);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

fn localized_resource_kind(kind: LocalizedResourceIndexKind) -> ResourceKind {
    match kind {
        LocalizedResourceIndexKind::Material => ResourceKind::Material,
        LocalizedResourceIndexKind::Font => ResourceKind::Font,
        LocalizedResourceIndexKind::Effect => ResourceKind::Effect,
        LocalizedResourceIndexKind::Filter => ResourceKind::Filter,
        LocalizedResourceIndexKind::Transition => ResourceKind::Transition,
    }
}

fn default_template_import_id(formula_bundle_path: &Path) -> String {
    formula_bundle_path
        .file_stem()
        .and_then(|name| name.to_str())
        .map(sanitize_template_import_id)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "offline-template-import".to_owned())
}

fn sanitize_template_import_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn template_import_delta(draft: &Draft) -> CommandDelta {
    let mut changed_entities = vec![
        ChangedEntity::Draft {
            draft_id: draft.draft_id.clone(),
        },
        ChangedEntity::Canvas {
            draft_id: draft.draft_id.clone(),
        },
    ];
    for material in &draft.materials {
        changed_entities.push(ChangedEntity::Material {
            material_id: material.material_id.clone(),
        });
    }
    for track in &draft.tracks {
        changed_entities.push(ChangedEntity::Track {
            track_id: track.track_id.clone(),
        });
        for segment in &track.segments {
            changed_entities.push(ChangedEntity::Segment {
                track_id: track.track_id.clone(),
                segment_id: segment.segment_id.clone(),
            });
        }
    }

    CommandDelta::full_draft(
        CommandDeltaName::ImportTemplate,
        changed_entities,
        vec![
            DirtyDomain::Track,
            DirtyDomain::Timing,
            DirtyDomain::Visual,
            DirtyDomain::Text,
            DirtyDomain::Audio,
            DirtyDomain::Material,
            DirtyDomain::Canvas,
            DirtyDomain::OutputProfile,
        ],
        vec![
            DirtyDomain::Preview,
            DirtyDomain::ExportPrep,
            DirtyDomain::Audio,
            DirtyDomain::Thumbnail,
            DirtyDomain::Waveform,
            DirtyDomain::Proxy,
            DirtyDomain::GraphSnapshot,
            DirtyDomain::PreviewCache,
        ],
        "template imported",
    )
}

fn project_session_store_error(
    command: &str,
    error: ProjectStoreError,
) -> Result<serde_json::Value> {
    to_runtime_value(project_store_error_envelope(command, error))
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

#[derive(Debug, Clone, Copy)]
enum ProjectSessionTelemetrySource {
    MediaProbe,
    ProjectIo,
}

fn record_task_runtime_scheduler_snapshot(
    _source: ProjectSessionTelemetrySource,
    _snapshot: &SchedulerTelemetrySnapshot,
) {
}

fn ok_envelope<T>(data: T) -> CommandResultEnvelope<T> {
    CommandResultEnvelope {
        ok: true,
        data: Some(data),
        error: None,
        events: Vec::new(),
    }
}

fn error_envelope(
    kind: CommandErrorKind,
    message: String,
    command: Option<String>,
) -> CommandResultEnvelope<serde_json::Value> {
    CommandResultEnvelope {
        ok: false,
        data: None,
        error: Some(CommandError {
            kind,
            message,
            command,
        }),
        events: Vec::new(),
    }
}

fn to_runtime_value<T: serde::Serialize>(
    value: CommandResultEnvelope<T>,
) -> Result<serde_json::Value> {
    serde_json::to_value(value).map_err(|error| {
        RuntimeError::new(
            RuntimeErrorKind::InvalidRequest,
            format!("project-session response serialization failed: {error}"),
        )
    })
}

fn command_diagnostic(
    diagnostic: crate::material_service::MissingMaterialDiagnostic,
) -> MissingMaterialCommandDiagnostic {
    MissingMaterialCommandDiagnostic {
        material_id: diagnostic.material_id,
        kind: command_diagnostic_kind(diagnostic.kind),
        original_uri: diagnostic.original_uri,
        last_known_resolved_path: diagnostic
            .last_known_resolved_path
            .map(|path| path.display().to_string()),
        status: diagnostic.status,
        message: diagnostic.message,
    }
}

fn material_probe_schedule_diagnostic(
    material: &Material,
    material_path: &Path,
    message: String,
) -> MissingMaterialCommandDiagnostic {
    MissingMaterialCommandDiagnostic {
        material_id: material.material_id.clone(),
        kind: draft_model::MissingMaterialCommandDiagnosticKind::ProbeFailed,
        original_uri: material.uri.clone(),
        last_known_resolved_path: Some(material_path.display().to_string()),
        status: material.status,
        message: format!("Material imported, but metadata probe could not be scheduled: {message}"),
    }
}

fn command_diagnostic_kind(
    kind: crate::material_service::MissingMaterialDiagnosticKind,
) -> draft_model::MissingMaterialCommandDiagnosticKind {
    match kind {
        crate::material_service::MissingMaterialDiagnosticKind::MissingFile => {
            draft_model::MissingMaterialCommandDiagnosticKind::MissingFile
        }
        crate::material_service::MissingMaterialDiagnosticKind::MarkedMissing => {
            draft_model::MissingMaterialCommandDiagnosticKind::MarkedMissing
        }
        crate::material_service::MissingMaterialDiagnosticKind::ProbeFailed => {
            draft_model::MissingMaterialCommandDiagnosticKind::ProbeFailed
        }
        crate::material_service::MissingMaterialDiagnosticKind::UnresolvedExternalUri => {
            draft_model::MissingMaterialCommandDiagnosticKind::UnresolvedExternalUri
        }
    }
}

fn material_service_error_envelope(
    command: &str,
    error: crate::material_service::MaterialServiceError,
) -> CommandResultEnvelope<serde_json::Value> {
    let kind = match &error {
        crate::material_service::MaterialServiceError::ProjectStore(ProjectStoreError::Io {
            ..
        }) => CommandErrorKind::ProjectIoFailed,
        crate::material_service::MaterialServiceError::ProjectStore(_)
        | crate::material_service::MaterialServiceError::Draft(_) => {
            CommandErrorKind::InvalidProject
        }
    };

    error_envelope(kind, error.to_string(), Some(command.to_string()))
}

fn project_store_error_envelope(
    command: &str,
    error: ProjectStoreError,
) -> CommandResultEnvelope<serde_json::Value> {
    let kind = match &error {
        ProjectStoreError::Io { .. } => CommandErrorKind::ProjectIoFailed,
        _ => CommandErrorKind::InvalidProject,
    };

    error_envelope(kind, error.to_string(), Some(command.to_string()))
}

fn project_store_warning_message(warning: ProjectStoreWarning) -> String {
    match warning {
        ProjectStoreWarning::MissingMaterial {
            material_id,
            uri,
            resolved_path,
        } => {
            let resolved = resolved_path
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "unresolved".to_owned());
            format!("missing material {material_id}: {uri} -> {resolved}")
        }
    }
}

fn runtime_discovery_message(error: DiscoveryError) -> String {
    let kind = serde_json::to_value(error.kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{:?}", error.kind));
    let checked_paths = error
        .checked_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let mut message = format!(
        "Media runtime discovery failed ({kind}) for {}. {}",
        error.binary.binary_name(),
        error.remediation
    );

    if !checked_paths.is_empty() {
        message.push_str(" Checked paths: ");
        message.push_str(&checked_paths);
        message.push('.');
    }
    if let Some(stdout) = error.stdout_summary {
        message.push_str(" stdout: ");
        message.push_str(&stdout);
    }
    if let Some(stderr) = error.stderr_summary {
        message.push_str(" stderr: ");
        message.push_str(&stderr);
    }

    message
}

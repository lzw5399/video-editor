# Phase 19: Production Effects, Retiming, And Transition Semantics - Pattern Map

**Mapped:** 2026-06-25  
**Files analyzed:** 57  
**Analogs found:** 57 / 57  
**Architecture posture:** Production refactor. Rust owns semantics, preview/export capability, render graph, FFmpeg compilation, and committed project state. Renderer surfaces issue commands/interactions and may display ghost/proxy feedback only.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/draft_model/src/effects.rs` | model | transform | `crates/draft_model/src/timeline.rs` | role-match |
| `crates/draft_model/src/timeline.rs` | model | transform | `crates/draft_model/src/timeline.rs` | exact |
| `crates/draft_model/src/interaction.rs` | model | event-driven | `crates/draft_model/src/interaction.rs` | exact |
| `crates/draft_model/tests/schema_exports.rs` | test | batch | `crates/draft_model/tests/schema_exports.rs` | exact |
| `apps/desktop-electron/src/generated/Draft.ts` | generated contract | batch | `crates/draft_model/tests/schema_exports.rs` | role-match |
| `crates/draft_commands/src/effects.rs` | service/command | CRUD | `crates/draft_commands/src/visual.rs` | role-match |
| `crates/draft_commands/src/retiming.rs` | service/command | CRUD | `crates/draft_commands/src/timeline.rs` | role-match |
| `crates/draft_commands/src/transition.rs` | service/command | CRUD | `crates/draft_commands/src/timeline.rs` | role-match |
| `crates/draft_commands/src/timeline.rs` | service/command | CRUD | `crates/draft_commands/src/timeline.rs` | exact |
| `crates/draft_commands/src/error.rs` | utility | request-response | `crates/draft_commands/src/error.rs` | exact |
| `crates/draft_commands/tests/effect_commands.rs` | test | batch | `crates/draft_commands/tests/visual_transform_commands.rs` | role-match |
| `crates/draft_commands/tests/retiming_commands.rs` | test | batch | `crates/draft_commands/tests/keyframe_commands.rs` | role-match |
| `crates/draft_commands/tests/transition_commands.rs` | test | batch | `crates/draft_commands/tests/visual_transform_commands.rs` | role-match |
| `crates/engine_core/src/time_mapping.rs` | service/utility | transform | `crates/engine_core/src/frame_state.rs` | role-match |
| `crates/engine_core/src/frame_state.rs` | service | transform | `crates/engine_core/src/frame_state.rs` | exact |
| `crates/engine_core/tests/retiming.rs` | test | batch | `crates/engine_core/src/frame_state.rs` tests | role-match |
| `crates/render_graph/src/effects.rs` | service/model | transform | `crates/render_graph/src/graph.rs` | role-match |
| `crates/render_graph/src/graph.rs` | service/model | transform | `crates/render_graph/src/graph.rs` | exact |
| `crates/render_graph/src/fingerprint.rs` | utility | transform | `crates/render_graph/src/fingerprint.rs` | exact |
| `crates/render_graph/src/incremental.rs` | utility | transform | `crates/render_graph/src/incremental.rs` | exact |
| `crates/render_graph/tests/render_graph_snapshots.rs` | test | batch | `crates/render_graph/tests/render_graph_snapshots.rs` | exact |
| `crates/realtime_preview_runtime/src/effects.rs` | service | streaming | `crates/realtime_preview_runtime/src/capabilities.rs` | role-match |
| `crates/realtime_preview_runtime/src/capabilities.rs` | service | transform | `crates/realtime_preview_runtime/src/capabilities.rs` | exact |
| `crates/realtime_preview_runtime/src/parity.rs` | service | transform | `crates/realtime_preview_runtime/src/parity.rs` | exact |
| `crates/realtime_preview_runtime/src/gpu/compositor.rs` | service | streaming | `crates/realtime_preview_runtime/src/gpu/compositor.rs` | exact |
| `crates/realtime_preview_runtime/src/gpu/pipelines.rs` | config/provider | streaming | `crates/realtime_preview_runtime/src/gpu/pipelines.rs` | exact |
| `crates/realtime_preview_runtime/tests/capability_matrix.rs` | test | batch | `crates/realtime_preview_runtime/tests/capability_matrix.rs` | exact |
| `crates/ffmpeg_compiler/src/effects.rs` | service | transform | `crates/ffmpeg_compiler/src/filters.rs` | role-match |
| `crates/ffmpeg_compiler/src/filters.rs` | service | transform | `crates/ffmpeg_compiler/src/filters.rs` | exact |
| `crates/ffmpeg_compiler/tests/capability_snapshots.rs` | test | batch | `crates/ffmpeg_compiler/tests/capability_snapshots.rs` | exact |
| `crates/editor_runtime/src/project_session_node.rs` | service/controller | request-response + event-driven | `crates/editor_runtime/src/project_session_node.rs` | exact |
| `crates/editor_runtime/src/project_session.rs` | service | request-response | `crates/editor_runtime/src/project_session.rs` | exact |
| `crates/bindings_node/src/project_session_service.rs` | binding/controller | request-response | `crates/bindings_node/src/project_session_service.rs` | exact |
| `crates/bindings_node/tests/project_interaction_session.rs` | test | batch | `crates/bindings_node/tests/project_interaction_session.rs` | exact |
| `apps/desktop-electron/src/main/nativeBinding.ts` | binding facade | request-response | `apps/desktop-electron/src/main/nativeBinding.ts` | exact |
| `apps/desktop-electron/src/preload/index.ts` | binding facade | request-response | `apps/desktop-electron/src/main/nativeBinding.ts` | role-match |
| `apps/desktop-electron/src/renderer/App.tsx` | provider/store | event-driven | `apps/desktop-electron/src/renderer/App.tsx` | exact |
| `apps/desktop-electron/src/renderer/viewModel.ts` | model/view-model | transform | `apps/desktop-electron/src/renderer/viewModel.ts` | exact |
| `apps/desktop-electron/src/renderer/workspace/projectInteraction.ts` | provider | event-driven | `apps/desktop-electron/src/renderer/workspace/projectInteraction.ts` | exact |
| `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx` | component | request-response | `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx` | exact |
| `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` | component | event-driven | `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` | exact |
| `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` | component | streaming + event-driven | `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` | exact |
| `apps/desktop-electron/src/renderer/workspace/Timeline.tsx` | component | event-driven | `apps/desktop-electron/src/renderer/workspace/Timeline.tsx` | exact |
| `apps/desktop-electron/src/renderer/styles.css` | config/style | transform | `apps/desktop-electron/src/renderer/styles.css` | exact |
| `apps/desktop-electron/src/renderer/workspace/preview-inspector.css` | config/style | transform | `apps/desktop-electron/src/renderer/workspace/preview-inspector.css` | exact |
| `apps/desktop-electron/src/renderer/workspace/timeline.css` | config/style | transform | `apps/desktop-electron/src/renderer/workspace/timeline.css` | exact |
| `apps/desktop-electron/src/renderer/assets/icons/index.ts` | config/provider | transform | `apps/desktop-electron/src/renderer/assets/icons/index.ts` | exact |
| `apps/desktop-electron/src/renderer/assets/icons/manifest.json` | config | batch | `apps/desktop-electron/src/renderer/assets/icons/manifest.json` | exact |
| `crates/adapter_kaipai/src/mapper.rs` | adapter/service | transform | `crates/adapter_kaipai/src/mapper.rs` | exact |
| `crates/testkit/tests/template_import_preview.rs` | test | batch | `crates/testkit/tests/template_import_preview.rs` | exact |
| `crates/testkit/tests/template_import_exports.rs` | test | batch | `crates/testkit/tests/template_import_exports.rs` | exact |
| `crates/testkit/tests/production_effects_preview.rs` | test | batch | `crates/testkit/tests/template_import_preview.rs` | role-match |
| `crates/testkit/tests/production_effects_exports.rs` | test | batch | `crates/testkit/tests/template_import_exports.rs` | role-match |
| `scripts/phase19-source-guards.sh` | utility/test | batch | `scripts/phase17-1-source-guards.sh` | role-match |
| `package.json` | config | batch | `package.json` | exact |
| `apps/desktop-electron/tests/production-effects.spec.ts` | test | event-driven | `apps/desktop-electron/tests/interaction-preview-inspector.spec.ts` | role-match |
| `apps/desktop-electron/tests/template-import.spec.ts` | test | event-driven | `apps/desktop-electron/tests/template-import.spec.ts` | exact |

## Pattern Assignments

### Rust Semantic Contracts

**Apply to:** `crates/draft_model/src/effects.rs`, `crates/draft_model/src/timeline.rs`, `crates/draft_model/src/interaction.rs`, generated TS contracts.

**Analog:** `crates/draft_model/src/time.rs`, `crates/draft_model/src/timeline.rs`, `crates/draft_model/src/interaction.rs`.

**Imports and integer-time pattern** (`crates/draft_model/src/time.rs` lines 1-18):
```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS,
)]
pub struct Microseconds(pub u64);
```

**Tagged enum/schema pattern** (`crates/draft_model/src/timeline.rs` lines 120-161):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Keyframe {
    pub at: Microseconds,
    pub property: KeyframeProperty,
    pub value: KeyframeValue,
    pub interpolation: KeyframeInterpolation,
    pub easing: KeyframeEasing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum KeyframeValue {
    Int { value: i32 },
    Uint { value: u32 },
    Color { value: String },
}
```

**Current placeholders to replace/upgrade** (`crates/draft_model/src/timeline.rs` lines 179-190):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Filter {
    pub name: String,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Transition {
    pub name: String,
    pub duration: Microseconds,
}
```

**Mask/blend shape already in persisted visual state** (`crates/draft_model/src/timeline.rs` lines 763-795):
```rust
pub enum SegmentBlendMode {
    Normal,
    Unsupported { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SegmentMask {
    None,
    Unsupported { name: String },
}

pub struct SegmentVisual {
    pub blend_mode: SegmentBlendMode,
    pub mask: SegmentMask,
}
```

**Interaction session semantic contract** (`crates/draft_model/src/interaction.rs` lines 5-27 and 46-61):
```rust
pub enum ProjectInteractionKind {
    SelectedSegmentVisual,
    SelectedText,
    SelectedSegmentAudio,
    PlayheadScrub,
    TimelineMoveTrim,
    KeyframeEdit,
}

pub struct ProjectInteractionSession {
    pub interaction_id: String,
    pub kind: ProjectInteractionKind,
    pub base_revision: u64,
    pub generation: u64,
    pub accepted_sequence: u64,
    pub coalesced_through: u64,
}

pub fn accept_sequence(&mut self, sequence: u64) -> Result<(), ProjectInteractionSequenceError> {
    if sequence == 0 { return Err(ProjectInteractionSequenceError::Zero); }
    if sequence <= self.accepted_sequence { /* stale */ }
    self.accepted_sequence = sequence;
    self.coalesced_through = sequence;
    Ok(())
}
```

**Planner instruction:** Add typed first-party effect/filter/transition/retime/capability variants before exposing UI controls. Keep external provider IDs as compatibility references or report items, not render semantics. Use `Microseconds`, integer millis, frame indices, or rational rates; do not persist floating seconds.

### Draft Command Semantics

**Apply to:** `crates/draft_commands/src/effects.rs`, `crates/draft_commands/src/retiming.rs`, `crates/draft_commands/src/transition.rs`, `crates/draft_commands/src/timeline.rs`, `crates/draft_commands/src/error.rs`.

**Analog:** `crates/draft_commands/src/visual.rs`, `crates/draft_commands/src/timeline.rs`, `crates/draft_commands/src/error.rs`.

**Command import/core pattern** (`crates/draft_commands/src/visual.rs` lines 3-13 and 15-55):
```rust
use draft_model::{
    CommandDeltaName, CommandEvent, CommandState, Draft, SegmentId, SegmentVisual,
    TimelineCommandResponse, TimelineSelection,
};

pub fn update_segment_visual(...) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    next_draft.tracks[track_index].segments[segment_index].visual = visual;
    validate_timeline_rules(&next_draft)?;
    let delta = visual_segment_delta(...);
    let (command_state, pruned) =
        push_undo_snapshot(command_state, draft, selection, "updateSegmentVisual");

    Ok(TimelineCommandResponse { draft: next_draft, command_state, selection: selection.clone(), events, delta })
}
```

**Timeline validation/snapping/magnet pattern** (`crates/draft_commands/src/timeline.rs` lines 735-807):
```rust
pub fn move_segment(...) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (source_track_index, source_segment_index) =
        find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[source_track_index])?;

    let (snapped_start, snap_event) = apply_snapping(...)?;
    segment.target_timerange.start = snapped_start;
    if let Some(event) = apply_main_track_magnet(&mut next_draft, &source_track_id)? {
        extra_events.push(event);
    }
    validate_timeline_rules(&next_draft)?;

    Ok(response_with_events(...).with_selection_fallback(selection))
}
```

**Structured error pattern** (`crates/draft_commands/src/error.rs` lines 10-75):
```rust
pub struct TimelineCommandError {
    pub kind: TimelineCommandErrorKind,
}

pub enum TimelineCommandErrorKind {
    TrackNotFound { track_id: TrackId },
    SegmentNotFound { segment_id: SegmentId },
    LockedTrack { track_id: TrackId },
    OverlappingSegment { track_id: TrackId, first_segment_id: SegmentId, second_segment_id: SegmentId },
    UnsupportedCommand { command: String },
    DraftValidationFailed { message: String },
}
```

**Planner instruction:** Retime, transition, and effect commands must clone the draft, validate locked tracks/material compatibility/transition windows, mutate only the Rust draft, call `validate_timeline_rules`, return `CommandDelta`, and push one undo snapshot on commit. Failed commands must leave draft, command state, and selection unchanged.

### Engine Time Mapping And Evaluation

**Apply to:** `crates/engine_core/src/time_mapping.rs`, `crates/engine_core/src/frame_state.rs`, `crates/engine_core/tests/retiming.rs`.

**Analog:** `crates/engine_core/src/frame_state.rs`.

**Current linear source mapping to replace/extend** (`crates/engine_core/src/frame_state.rs` lines 82-84 and 264-282):
```rust
let segment_time = segment_relative_time_at(segment, at)?;
let source_position = source_position_at(segment, at)?;

fn source_position_at(segment: &NormalizedSegment, at: Microseconds) -> Result<Microseconds, EngineError> {
    let offset = segment_relative_time_at(segment, at)?;
    segment.source_timerange.start.get()
        .checked_add(offset.get())
        .map(Microseconds::new)
        .ok_or_else(|| EngineError::new(...))
}
```

**Integer interpolation pattern** (`crates/engine_core/src/frame_state.rs` lines 520-532):
```rust
fn eased_progress_per_mille(start: &Keyframe, at: Microseconds, end: Microseconds) -> Option<u32> {
    let span = end.get().checked_sub(start.at.get())?;
    let elapsed = at.get().checked_sub(start.at.get())?;
    let raw = (u128::from(elapsed) * 1_000_u128 / u128::from(span)).min(1_000) as u32;
    Some(match start.easing {
        KeyframeEasing::None => raw,
        KeyframeEasing::EaseIn => raw.saturating_mul(raw) / 1_000,
        KeyframeEasing::EaseOut => { /* integer math */ }
    })
}
```

**Planner instruction:** Put retime source-to-target mapping in `engine_core`, then feed render graph, preview, compiler, and audio diagnostics from that result. Do not implement retiming first as FFmpeg `setpts`, UI duration math, or adapter-only `durationMsWithSpeed`.

### Render Graph, Fingerprints, And Dirty Ranges

**Apply to:** `crates/render_graph/src/effects.rs`, `crates/render_graph/src/graph.rs`, `crates/render_graph/src/fingerprint.rs`, `crates/render_graph/src/incremental.rs`, render graph tests.

**Analog:** `crates/render_graph/src/graph.rs`, `crates/render_graph/src/fingerprint.rs`, `crates/render_graph/src/incremental.rs`.

**Support vocabulary pattern** (`crates/render_graph/src/graph.rs` lines 275-301):
```rust
pub struct RenderFilterIntent {
    pub node_id: RenderGraphNodeId,
    pub name: String,
    pub parameters: BTreeMap<String, String>,
    pub support: RenderIntentSupport,
    pub reason: String,
}

pub enum RenderIntentSupport {
    Supported,
    Degraded,
    Unsupported,
}
```

**Semantic fingerprint inclusion pattern** (`crates/render_graph/src/fingerprint.rs` lines 222-237):
```rust
fingerprint_parts(
    layer.node_id.clone(),
    &VideoLayerSemanticInput {
        stack_index: layer.stack_index,
        source_timerange: &layer.source_timerange,
        target_timerange: &layer.target_timerange,
        keyframes: &layer.keyframes,
        filters: &layer.filters,
        transition: layer.transition.as_ref(),
        visual: &layer.visual,
    },
    ...
)
```

**Stable node identity pattern** (`crates/render_graph/src/incremental.rs` lines 68-92 and 99-123):
```rust
pub fn segment_filter(..., filter_index: usize) -> Self {
    Self::new(RenderGraphNodeRole::SegmentFilter, draft_id)
        .with_track_id(track_id)
        .with_segment_id(segment_id)
        .with_material_id(material_id)
        .with_local_id(filter_index.to_string())
}

RenderGraphNodeRole::SegmentTransition => {
    format!("{}:transition", self.segment_role_prefix())
}
```

**Unsupported visual diagnostics pattern** (`crates/render_graph/src/graph.rs` lines 845-864):
```rust
if let SegmentBlendMode::Unsupported { name } = &visual.blend_mode {
    diagnostics.push(render_visual_diagnostic(..., "blendMode", RenderIntentSupport::Unsupported, ...));
}
if let SegmentMask::Unsupported { name } = &visual.mask {
    diagnostics.push(render_visual_diagnostic(..., "mask", RenderIntentSupport::Unsupported, ...));
}
```

**Planner instruction:** Every new effect, retime mode, transition window, mask, and blend field must be present in render graph intent, fingerprints, dirty facts, snapshot tests, and support diagnostics.

### Realtime Preview Runtime And GPU Compositor

**Apply to:** `crates/realtime_preview_runtime/src/effects.rs`, `capabilities.rs`, `parity.rs`, `gpu/compositor.rs`, `gpu/pipelines.rs`, capability tests.

**Analog:** `crates/realtime_preview_runtime/src/capabilities.rs`, `parity.rs`, `gpu/compositor.rs`, `gpu/pipelines.rs`.

**Current unsupported classifier to upgrade only with real support** (`crates/realtime_preview_runtime/src/capabilities.rs` lines 246-305):
```rust
if let SegmentMask::Unsupported { name } = &visual.mask {
    diagnostics.push(RealtimePreviewDiagnostic::new(..., RealtimePreviewSupport::Unsupported {
        reason: format!("segment mask {name} is unsupported in realtime preview"),
    }, ...));
}
for filter in &layer.filters {
    diagnostics.push(RealtimePreviewDiagnostic::new(..., RealtimePreviewDiagnosticDomain::Effect,
        RealtimePreviewSupport::Unsupported {
            reason: format!("filter {} is unsupported in realtime preview", filter.name),
        },
        ...
    ));
}
```

**Preview/export parity pattern** (`crates/realtime_preview_runtime/src/parity.rs` lines 20-34 and 108-127):
```rust
pub fn realtime_preview_parity_diagnostics(
    graph: &RenderGraph,
    report: &RealtimePreviewCapabilityReport,
) -> Vec<RealtimePreviewParityDiagnostic> {
    report.diagnostics.iter()
        .filter(|diagnostic| parity_domain(diagnostic.domain))
        .filter_map(|diagnostic| {
            let export_support = export_support_for(graph, diagnostic.domain, diagnostic.entity_id.as_deref())?;
            if equivalent_support(&diagnostic.support, export_support) { return None; }
            Some(...)
        })
}
```

**GPU compositor flow** (`crates/realtime_preview_runtime/src/gpu/compositor.rs` lines 73-114 and 646-690):
```rust
let capability = self.classifier.classify(graph);
diagnostics.extend(capability.diagnostics);
if let Some(texture) = target.texture() {
    let pipeline_resources = self.wgpu_pipeline_resources_for_graph(...)?;
    let (pixels, submitted_draws) = render_wgpu_graph(..., pipeline_resources)?;
    return Ok(RealtimePreviewCompositorOutput {
        render_backend: RealtimePreviewCompositorBackend::WgpuRenderPass,
        support,
        diagnostics,
        ...
    });
}

let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
    label: Some("realtime-preview-wgpu-graph-encoder"),
});
let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    label: Some("realtime-preview-wgpu-graph-render-pass"),
    ...
});
```

**Pipeline label provider pattern** (`crates/realtime_preview_runtime/src/gpu/pipelines.rs` lines 1-13):
```rust
pub struct RealtimePreviewPipelineSet {
    pub canvas_pipeline_label: &'static str,
    pub textured_quad_pipeline_label: &'static str,
}
```

**Planner instruction:** Add effect GPU passes under the compositor/pipeline path. Product evidence must come from `renderGraphGpuComposited`/native preview, not DOM/CSS overlays, screenshots, CPU probes, or artifact fallback.

### FFmpeg Compiler Ownership

**Apply to:** `crates/ffmpeg_compiler/src/effects.rs`, `crates/ffmpeg_compiler/src/filters.rs`, compiler tests.

**Analog:** `crates/ffmpeg_compiler/src/filters.rs`, `crates/ffmpeg_compiler/tests/capability_snapshots.rs`.

**Compiler-owned filter script pattern** (`crates/ffmpeg_compiler/src/filters.rs` lines 51-64 and 94-188):
```rust
pub fn generate_filter_script(
    plan: &RenderGraphPlan,
    context: &CompileContext,
    inputs: &[FfmpegInput],
    ass_sidecars: &[FfmpegSidecar],
    job_id: &str,
) -> Result<GeneratedFilterScript, FfmpegCompileError> {
    let path = context.artifact_path(&format!("{job_id}-filter.ffscript"));
    let input_indexes = input_index_by_material(inputs);
    ...
    lines.push(format!("[{current_video}]format=yuv420p[vout]"));
    Ok(GeneratedFilterScript { path, contents: lines.join(";\n"), has_audio_output, diagnostics })
}
```

**Visual transform filter pattern** (`crates/ffmpeg_compiler/src/filters.rs` lines 300-414):
```rust
fn compile_visual_layer(...) -> VisualLayerFilter {
    let active_start = target_delay_from_output(&layer.target_timerange, output_timerange(plan));
    let cropped_dimensions = cropped_dimensions(source_dimensions, &layer.visual.transform.crop);
    let (fit_filters, mut current_dimensions) = fit_mode_filters(...);
    if layer.visual.transform.opacity.value_millis < 1_000 {
        transform_filters.push(format!(
            "colorchannelmixer=aa={}",
            millis_decimal(layer.visual.transform.opacity.value_millis)
        ));
    }
    ...
}
```

**Timestamp filter pattern stays compiler-only** (`crates/ffmpeg_compiler/src/filters.rs` lines 425-455):
```rust
fn visual_setpts_filter(target_delay: Microseconds) -> String {
    if target_delay == Microseconds::ZERO {
        "setpts=PTS-STARTPTS".to_owned()
    } else {
        format!("setpts=PTS-STARTPTS+{}/TB", format_seconds(target_delay))
    }
}
```

**Capability error tests** (`crates/ffmpeg_compiler/tests/capability_snapshots.rs` lines 5-12 and 26-39):
```rust
let error = compile_ffmpeg_job(&common::export_plan(), &common::no_font_context())
    .expect_err("missing font should be classified");
assert_eq!(error.kind, FfmpegCompileErrorKind::MissingTextFont);

let error = compile_ffmpeg_job(...).expect_err("unsupported text resources should be classified before ASS output");
assert_eq!(error.kind, FfmpegCompileErrorKind::UnsupportedTextResource);
```

**Planner instruction:** FFmpeg `setpts`, `atempo`, `xfade`, `gblur`, blend, overlay, and color filters belong only here after render graph intent exists. Renderer/adapters must never construct FFmpeg strings.

### Runtime, Node Binding, And Desktop IPC

**Apply to:** `crates/editor_runtime/src/project_session_node.rs`, `project_session.rs`, `crates/bindings_node/src/project_session_service.rs`, `apps/desktop-electron/src/main/nativeBinding.ts`, preload.

**Analog:** Same files.

**Runtime JSON route pattern** (`crates/editor_runtime/src/project_session_node.rs` lines 1010-1068):
```rust
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
```

**Revision/kind/sequence gate pattern** (`crates/editor_runtime/src/project_session_node.rs` lines 2245-2305):
```rust
fn validate_interaction_revision(command: &str, expected_revision: u64, base_revision: u64) -> Option<Result<serde_json::Value>> { ... }
fn validate_interaction_kind(command: &str, expected: ProjectInteractionKind, received: ProjectInteractionKind) -> Option<Result<serde_json::Value>> { ... }
fn accept_interaction_sequence(command: &str, interaction: &mut DraftProjectInteractionSession, sequence: u64) -> Option<Result<serde_json::Value>> {
    match interaction.accept_sequence(sequence) {
        Ok(()) => None,
        Err(ProjectInteractionSequenceError::Zero) => Some(project_interaction_error(...)),
        Err(ProjectInteractionSequenceError::Stale { .. }) => Some(project_interaction_error(...)),
    }
}
```

**Provisional vs commit pattern** (`crates/editor_runtime/src/project_session_node.rs` lines 2321-2354 and 2357-2418):
```rust
fn provisional_interaction_payload(&self, payload: &ProjectInteractionPayload) -> Result<ProjectInteractionProvisionalResult, String> {
    let edit_payload = self.intent_payload(intent)?;
    let response = draft_commands::timeline::execute_timeline_edit(edit_payload)?;
    Ok(ProjectInteractionProvisionalResult {
        view_model: project_session_view_model(&response.draft, &response.command_state, &response.selection),
        delta: response.delta,
        draft: response.draft,
        selection: response.selection,
    })
}

fn commit_interaction_payload(&mut self, payload: ProjectInteractionPayload, interaction: &DraftProjectInteractionSession) -> Result<serde_json::Value> {
    let response = match draft_commands::timeline::execute_timeline_edit(edit_payload) { ... };
    ...
}
```

**Thin N-API adapter pattern** (`crates/bindings_node/src/project_session_service.rs` lines 7-40 and 96-100):
```rust
pub fn begin_project_interaction(request: serde_json::Value) -> Result<serde_json::Value> {
    runtime_value(editor_runtime::project_session_node::begin_project_interaction(request))
}

fn runtime_value(
    value: std::result::Result<serde_json::Value, editor_runtime::RuntimeError>,
) -> Result<serde_json::Value> {
    value.map_err(|error| napi::Error::from_reason(error.to_string()))
}
```

**Desktop binding types/functions** (`apps/desktop-electron/src/main/nativeBinding.ts` lines 391-426 and 1006-1044):
```typescript
export type ProjectInteractionPayload =
  | { kind: "selectedSegmentVisual"; patch: SegmentVisualPatch }
  | { kind: "playheadScrub"; playhead: Microseconds }
  | { kind: "timelineMoveTrim"; mode: "move" | "trimLeft" | "trimRight"; ... }
  | { kind: "keyframeEdit"; property: KeyframeProperty; at: Microseconds; ... };

export function updateProjectInteraction(
  request: UpdateProjectInteractionRequest
): CommandResultEnvelope<ProjectInteractionUpdateResponse> {
  const binding = loadNativeBinding();
  if (binding === null) { return bindingLoadError("updateProjectInteraction"); }
  return binding.updateProjectInteraction(request);
}
```

**Planner instruction:** Extend existing interaction payloads/kinds for effect strength, filter strength, mask handles, blend opacity, transition duration, and retime handles. Keep `bindings_node` thin and put semantics below `editor_runtime`.

### Desktop Renderer Surfaces

**Apply to:** `App.tsx`, `viewModel.ts`, `workspace/projectInteraction.ts`, `FeaturePanel.tsx`, `Inspector.tsx`, `PreviewMonitor.tsx`, `Timeline.tsx`, CSS, icons.

**Analog:** Same workspace files.

**App-level interaction controller** (`apps/desktop-electron/src/renderer/App.tsx` lines 876-1052):
```typescript
async function beginProjectInteractionSession(kind: ProjectInteractionKind) {
  const result = await window.videoEditorCore.beginProjectInteraction({
    sessionId: session.sessionId,
    expectedRevision: session.revision,
    kind
  });
  activeProjectInteractionRef.current = {
    interactionId: result.data.interactionId,
    kind: result.data.kind,
    baseRevision: result.data.baseRevision,
    generation: result.data.generation
  };
}

async function updateProjectInteractionSession(interactionId: string, sequence: number, payload: ProjectInteractionPayload) {
  const result = await window.videoEditorCore.updateProjectInteraction({ sessionId, expectedRevision, interactionId, sequence, payload });
  if (payload.kind === "playheadScrub") {
    void seekProjectInteractionPlayhead(result.data, payload.playhead);
  } else {
    void refreshRealtimePreviewForProjectInteraction(result.data);
  }
}
```

**Preview snapshot refresh for interactions** (`apps/desktop-electron/src/renderer/App.tsx` lines 2574-2587 and 2694-2731):
```typescript
async function refreshRealtimePreviewForProjectInteraction(update: ProjectInteractionUpdateResponse): Promise<void> {
  const previewTarget = previewRefreshTargetFromDelta({
    delta: update.provisionalDelta,
    viewModel: update.provisionalViewModel
  });
  const snapshotReady = await updateRealtimePreviewProjectSessionSnapshot({
    interactionId: update.interactionId
  });
  if (snapshotReady && previewTarget !== null) {
    await seekRealtimePreviewHost(previewTarget);
  }
}
```

**Shared interaction interface** (`apps/desktop-electron/src/renderer/workspace/projectInteraction.ts` lines 1-24):
```typescript
export type ProjectInteractionController = {
  begin: (kind: ProjectInteractionKind) => Promise<ProjectInteractionBeginResponse | null>;
  update: (interactionId: string, sequence: number, payload: ProjectInteractionPayload) => Promise<ProjectInteractionUpdateResponse | null>;
  commit: (interactionId: string) => Promise<ProjectInteractionCommitResponse | null>;
  cancel: (interactionId: string) => Promise<ProjectInteractionCancelResponse | null>;
};
```

**Resource panel unavailable gate to replace only when backed** (`apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx` lines 141-149 and 1118-1156):
```tsx
特效: {
  rail: ["热门", "画面", "氛围", "收藏"],
  cards: ["基础光效", "速度感", "复古颗粒", "镜头闪白"],
  unavailableReason: "特效需要稳定的预览和导出支持后开放"
},
转场: {
  rail: ["基础", "运镜", "遮罩", "收藏"],
  cards: ["叠化", "推拉", "闪白", "模糊转场"],
  unavailableReason: "转场需要时间线语义和导出一致后开放"
}

<div className="showcase-panel-layout" aria-disabled="true">
  <article className="showcase-card unavailable" aria-disabled="true">
    <span className="showcase-card-preview" aria-hidden="true" />
    <strong>{item}</strong>
    <em>暂不可用</em>
  </article>
</div>
```

**Inspector coalesced slider pattern** (`apps/desktop-electron/src/renderer/workspace/Inspector.tsx` lines 2286-2318 and 2322-2353):
```typescript
function queueInspectorVisualUpdate(state: VisualFormState, property?: KeyframeProperty): void {
  const payload = inspectorVisualInteractionPayload(selected, playheadAt, state, property);
  const interaction = beginInspectorVisualInteraction(property);
  interaction.pendingPayload = payload;
  if (interaction.rafId !== null) { return; }
  interaction.rafId = window.requestAnimationFrame(() => {
    interaction.rafId = null;
    flushInspectorVisualUpdate(interaction);
  });
}

async function finishInspectorVisualInteraction(action: "commit" | "cancel"): Promise<void> {
  await interaction.beginPromise;
  while (interaction.updateInFlight) { await new Promise((resolve) => window.setTimeout(resolve, 0)); }
  if (action === "commit") { await projectInteractions.commit(interaction.interactionId); }
  else { await projectInteractions.cancel(interaction.interactionId); }
}
```

**Preview native surface and ghost overlay pattern** (`apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` lines 401-420 and 940-1005):
```tsx
const showRealtimeSurface = !nativeSurfaceSuspended && nativeHostState.productReady && !nativeHostState.fallbackActive;
const selectionOverlay = buildSelectionOverlayModel(selectedSegment, nativeHostState.contentEvidence, previewDragPreview);
const textOverlayStyle = !showRealtimeSurface ? buildTextOverlayStyle(selectedSegment, previewDragPreview) : null;

<div
  className={`preview-selection-outline preview-selection-${selectionOverlay.source}`}
  data-interaction-source={previewInteractionEvidence === null ? undefined : "rust-provisional"}
  data-interaction-kind={previewInteractionEvidence?.kind}
  data-interaction-generation={previewInteractionEvidence?.generation}
  ...
/>
<div ref={nativeHostRef} className="preview-native-host" aria-label="实时预览画面">
```

**Timeline coalesced drag/keyframe pattern** (`apps/desktop-electron/src/renderer/workspace/Timeline.tsx` lines 1201-1236 and 1300-1378):
```typescript
function queueTimelineMoveTrimUpdate(interaction, payload): void {
  interaction.pendingPayload = payload;
  if (interaction.rafId !== null) { return; }
  interaction.rafId = window.requestAnimationFrame(() => {
    interaction.rafId = null;
    flushTimelineMoveTrimUpdate(interaction);
  });
}

interaction.beginPromise = projectInteractions.begin("keyframeEdit").then((begin) => {
  interaction.interactionId = begin.interactionId;
  flushKeyframeMarkerUpdate(interaction);
});
```

**Workspace category/view-model pattern** (`apps/desktop-electron/src/renderer/viewModel.ts` lines 47-79):
```typescript
export type WorkspaceCategory = "媒体" | "音频" | "文字" | "贴纸" | "特效" | "转场" | "字幕" | "滤镜" | "调节" | "模板" | "数字人";
export const WORKSPACE_CATEGORY_META: Record<WorkspaceCategory, WorkspaceCategoryMetadata> = {
  特效: { label: "特效" },
  转场: { label: "转场" },
  滤镜: { label: "滤镜" },
  调节: { label: "调节" },
  ...
};
```

**CSS layout patterns** (`apps/desktop-electron/src/renderer/styles.css` lines 854-978, `preview-inspector.css` lines 139-205, `timeline.css` lines 738-830):
```css
.showcase-panel-layout {
  display: grid;
  grid-template-columns: 116px minmax(0, 1fr);
  gap: 10px;
  overflow: hidden;
}

.preview-native-host {
  position: absolute;
  inset: 0;
  z-index: 4;
  pointer-events: none;
}

.segment-keyframe-marker {
  width: 16px;
  height: 16px;
  cursor: ew-resize;
  transform: translateX(-50%) rotate(45deg);
}
```

**Icon registration pattern** (`apps/desktop-electron/src/renderer/assets/icons/index.ts` lines 1-12 and 39-50; `manifest.json` lines 1-33):
```typescript
import categoryEffectIconUrl from "./category-effect.svg";
import categoryTransitionIconUrl from "./category-transition.svg";

export const appIconUrls = {
  categoryEffect: categoryEffectIconUrl,
  categoryTransition: categoryTransitionIconUrl,
};
```

**Planner instruction:** FeaturePanel, Inspector, Timeline, and PreviewMonitor may show compact controls only after Rust registry support and preview/export gates exist. Use Chinese operational labels from `19-UI-SPEC.md`, stable dimensions, app-local icons, and Rust interaction sessions. No renderer-owned time mapping, transition validation, FFmpeg strings, capability success, cache keys, or product fallback evidence.

### Adapter And Template Fixtures

**Apply to:** `crates/adapter_kaipai/src/mapper.rs`, `crates/testkit/tests/template_import_preview.rs`, `template_import_exports.rs`, new production effects fixtures, `apps/desktop-electron/tests/template-import.spec.ts`.

**Analog:** Same files.

**Adapter import/report pipeline** (`crates/adapter_kaipai/src/mapper.rs` lines 65-115):
```rust
bundle.validate()?;
let localized = localize_template_resources(ResourceLocalizationRequest { ... })?;
let context = MapperContext::new(bundle, &localized.manifest);
let mut state = MapperState::new(context);
state.report_items.extend(localized.diagnostics);
state.map_formula(formula, &canvas_config)?;
let plan = DraftImportPlan { schema_version: DraftImportPlanSchemaVersion::current(), ... };
validate_import_plan(&plan)?;
let report = AdaptationReport::new("kaipaiOfflineBundle", generated_at, state.report_items);
```

**External native effects stay report-only** (`crates/adapter_kaipai/src/mapper.rs` lines 698-728):
```rust
self.report_items.push(report_item(
    AdaptationStatus::NeedsNativeEffect,
    AdaptationSeverity::Warning,
    AdaptationCategory::NativeEffect,
    AdaptationTargetKind::Effect,
    effect_id,
    "Provider-native beauty effect requires a local implementation before it can be represented.",
    Some("Native effects are never classified as supported by fixture expectations."),
    ...
));
self.report_items.push(report_item(
    AdaptationStatus::Dropped,
    AdaptationCategory::Segment,
    AdaptationTargetKind::Filter,
    &format!("filter-{effect_id}"),
    "Native effect is omitted from the canonical draft filter stack.",
    ...
));
```

**Current transition mapping is simplistic and should be replaced by first-party semantics** (`crates/adapter_kaipai/src/mapper.rs` lines 985-998):
```rust
fn transition_from_formula(value: &Value) -> Result<Option<Transition>, AdapterKaipaiError> {
    let name = optional_string_field(transition, "name")
        .or_else(|| optional_string_field(transition, "type"))
        .unwrap_or("");
    if !matches!(name, "fade" | "dissolve") { return Ok(None); }
    Ok(Some(Transition {
        name: name.to_owned(),
        duration: ms_to_us(optional_u64_field(transition, "durationMs").unwrap_or(300)),
    }))
}
```

**Preview fixture no-fallback gate** (`crates/testkit/tests/template_import_preview.rs` lines 83-99 and 125-155):
```rust
let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput { draft: imported.draft, ... })?;
let report = RealtimePreviewCapabilityClassifier::supported_for_tests().classify(&prepared.graph);
assert_eq!(report.support, RealtimePreviewGraphSupport::Supported);
assert_no_realtime_fallback_evidence(case.family, &report);

let runtime_missing = RealtimePreviewCapabilityClassifier::supported_for_tests()
    .with_runtime_backend_available(false)
    .classify(&prepared.graph);
assert_eq!(runtime_missing.support, RealtimePreviewGraphSupport::Unsupported);
```

**Export canonical chain and project JSON guard** (`crates/testkit/tests/template_import_exports.rs` lines 281-315 and 716-740):
```rust
let normalized = normalize_draft(draft, &profile)?;
let range = resolve_render_range(&normalized, TargetTimerange::new(...))?;
let graph = build_render_graph(&normalized, &range)?;
let plan = RenderGraphPlan::new(graph, RenderOutputProfile::export_mp4(...))?;
compile_ffmpeg_job(&plan, &context)?;

for forbidden in ["templateId", "rawFormula", "renderUrl", "kaipai", "provider"] {
    assert!(!serialized.contains(forbidden), "project.json leaked provider/runtime evidence {forbidden}");
}
```

**Planner instruction:** Extend fixtures with supported first-party effects/retiming/transitions and explicit degraded/unsupported reports. Do not chase proprietary parity or place private IDs into canonical draft/render semantics.

### Guards, Package Scripts, And E2E

**Apply to:** `scripts/phase19-source-guards.sh`, `package.json`, `apps/desktop-electron/tests/production-effects.spec.ts`, existing interaction/template specs.

**Analog:** `scripts/phase17-1-source-guards.sh`, `scripts/phase18-source-guards.sh`, `scripts/no-product-fallback-guards.sh`, package scripts, Playwright specs.

**Guard helper pattern** (`scripts/phase17-1-source-guards.sh` lines 1-40):
```bash
#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase17.1 source guard violation: $1" >&2
  exit 1
}

require_fixed() {
  local file="$1"
  local text="$2"
  if ! rg -n --fixed-strings "$text" "$file" >/dev/null; then
    fail "missing required text '${text}' in ${file}"
  fi
}

matches_for_pattern() {
  local pattern="$1"
  shift
  rg -n --pcre2 "$pattern" "$@" 2>/dev/null | strip_comments
}
```

**Existing interaction guard requirements to extend** (`scripts/phase17-1-source-guards.sh` lines 185-231):
```bash
require_fixed "apps/desktop-electron/src/renderer/App.tsx" "ProjectInteractionController"
require_fixed "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx" "requestAnimationFrame"
require_fixed "apps/desktop-electron/src/renderer/workspace/Timeline.tsx" 'projectInteractions.begin("timelineMoveTrim")'
require_fixed "apps/desktop-electron/src/renderer/workspace/Inspector.tsx" "onPreviewChange"
require_fixed "apps/desktop-electron/tests/interaction-preview-inspector.spec.ts" "revisionUnchanged"
require_fixed "apps/desktop-electron/tests/template-import.spec.ts" "renderGraphGpuComposited"
require_fixed "apps/desktop-electron/tests/template-import.spec.ts" "fallbackActive"
```

**No-product-fallback guard pattern** (`scripts/no-product-fallback-guards.sh` lines 15-38 and 83-86):
```bash
fail_if_matches \
  "Electron realtime preview host must not request decoded/FFmpeg content evidence or expose mock/fallback playback displays" \
  'requestRealtimePreviewContentEvidence|mockFrameDisplay|requestFallbackFrame|ffmpegArtifactGenerated' \
  apps/desktop-electron/src/main/realtimePreviewHost.ts

if ! rg -q 'renderGraphGpuComposited' apps/desktop-electron/tests/product-user-journey.spec.ts apps/desktop-electron/tests/helpers/userJourney.ts; then
  echo "no-product-fallback violation: product playback must require renderGraphGpuComposited evidence" >&2
  exit 1
fi
```

**Package script pattern** (`package.json` lines 88-103):
```json
"test:phase17-1:guards": "bash scripts/phase17-1-source-guards.sh",
"test:phase17-1:desktop": "pnpm --filter @video-editor/desktop package:dir && pnpm --filter @video-editor/desktop exec playwright test tests/interaction-preview-inspector.spec.ts tests/interaction-timeline-keyframe.spec.ts tests/template-import.spec.ts tests/ui-regression.spec.ts --reporter=line --workers=1",
"test:phase18-source-guards": "bash scripts/phase18-source-guards.sh",
"test:phase18": "pnpm run test:phase18-rust && pnpm run test:phase18-source-guards && pnpm run test:phase18-abi && pnpm run test:phase18-server && pnpm run test:phase18-mobile-contracts && cargo check --workspace --locked && pnpm run test:no-product-fallback && pnpm run test:contracts"
```

**Playwright interaction assertions** (`apps/desktop-electron/tests/interaction-preview-inspector.spec.ts` lines 186-235; `interaction-timeline-keyframe.spec.ts` lines 135-172):
```typescript
await expect.poll(async () => interactionCommandsSince(app, beforeIndex), { timeout: 10_000 })
  .toEqual(expect.arrayContaining(["beginProjectInteraction", "updateProjectInteraction"]));
expect(commandCount(liveCalls, "commitProjectInteraction"), "slider drag must not commit before pointer-up").toBe(0);
expect(update.resultRevision).toBe(baseRevision);
expect(update.revisionUnchanged).toBe(true);
expectCoalescedInteractionTelemetry(liveCalls, "selectedSegmentVisual", "inspector visual slider");

expect(latestUpdate?.acceptedSequence, "scrub should accept multiple monotonic samples").toBeGreaterThan(1);
expect(latestUpdate?.coalescedThrough).toBe(latestUpdate?.acceptedSequence);
```

**Schema/export contract tests** (`crates/draft_model/tests/schema_exports.rs` lines 72-86 and 300-324):
```rust
const PUBLIC_TIMELINE_EDIT_PAYLOAD_CONTRACTS: &[&str] = &[
    "UpdateSegmentVisualCommandPayload",
    "SetSegmentKeyframeCommandPayload",
    "RemoveSegmentKeyframeCommandPayload",
];

export_decl::<SegmentBlendMode>(),
export_decl::<SegmentMask>(),
export_decl::<SegmentVisual>(),
export_decl::<Segment>(),
export_decl::<Draft>(),
```

**Planner instruction:** `phase19-source-guards.sh` should fail renderer-owned FFmpeg strings, renderer-owned time mapping, renderer transition overlap validation, cache/fingerprint decisions outside Rust, unsupported controls enabled as success, provider-native IDs in semantic code, fallback/DOM/CSS preview evidence, and mousemove save/revision/undo loops. Wire `test:phase19-rust`, `test:phase19-source-guards`, `test:phase19-desktop`, and `test:phase19` into `package.json`.

## Shared Patterns

### Rust-Owned Semantics
**Source:** `AGENTS.md`, `crates/draft_model/src/timeline.rs`, `crates/draft_commands/src/visual.rs`.  
**Apply to:** All semantic, command, render graph, preview, compiler, adapter, and UI files.  
UI emits commands/interactions. Rust owns draft state, time mapping, effect evaluation, transition validation, preview/export capability, dirty ranges, and committed revisions.

### Capability Reporting
**Source:** `crates/render_graph/src/graph.rs`, `crates/realtime_preview_runtime/src/capabilities.rs`, `crates/realtime_preview_runtime/src/parity.rs`.  
**Apply to:** Effects, filters, transitions, masks, blends, retime modes.  
Every capability must classify preview and export as supported, degraded, unsupported, or external/report-only. Unsupported cannot count as product success.

### High-Frequency Interactions
**Source:** `crates/draft_model/src/interaction.rs`, `crates/editor_runtime/src/project_session_node.rs`, `Inspector.tsx`, `Timeline.tsx`.  
**Apply to:** Effect/filter sliders, mask handles, blend opacity, transition duration handles, speed/retime handles, keyframe drags.  
Use begin -> coalesced update -> commit/cancel with monotonically increasing sequences. Do not save, increment revision, or push undo on every pointer move.

### Preview/Export Parity
**Source:** `crates/realtime_preview_runtime/src/parity.rs`, `crates/testkit/tests/template_import_preview.rs`, `crates/testkit/tests/template_import_exports.rs`.  
**Apply to:** Supported first-party effect slice and template-fidelity fixtures.  
Preview support and export support must be compared from graph/runtime facts. Product preview evidence comes from the native render-graph GPU path.

### Compiler Boundary
**Source:** `crates/ffmpeg_compiler/src/filters.rs`.  
**Apply to:** Export retiming, dissolve/crossfade, blur, blend, color adjustment, audio follow-speed.  
FFmpeg strings and filtergraph labels are generated only in compiler code from render graph plans.

### Adapter Boundary
**Source:** `crates/adapter_kaipai/src/mapper.rs`, `template_import_exports.rs`.  
**Apply to:** Kaipai/Jianying fixture mapping.  
Adapters translate supported external concepts into first-party semantics and report native/private concepts as needs-native/dropped/degraded. Canonical `project.json` must not leak provider IDs or runtime URLs.

### UI Design
**Source:** `19-UI-SPEC.md`, `FeaturePanel.tsx`, `Inspector.tsx`, `PreviewMonitor.tsx`, `Timeline.tsx`, CSS files.  
**Apply to:** Resource panel, inspector, preview, timeline, icons.  
Use compact Jianying-style controls, Chinese operational copy, stable dimensions, app-local SVG icons, cyan active states, and no explanatory marketing/debug copy in default mode.

## No Analog Found

No file lacked a close role/data-flow analog. New modules such as `crates/draft_model/src/effects.rs`, `crates/draft_commands/src/retiming.rs`, `crates/draft_commands/src/transition.rs`, `crates/render_graph/src/effects.rs`, `crates/realtime_preview_runtime/src/effects.rs`, `crates/ffmpeg_compiler/src/effects.rs`, `crates/testkit/tests/production_effects_*.rs`, and `scripts/phase19-source-guards.sh` do not have exact same-feature analogs, but each has a strong role-match listed above.

## Metadata

**Analog search scope:** `crates/draft_model`, `crates/draft_commands`, `crates/engine_core`, `crates/render_graph`, `crates/realtime_preview_runtime`, `crates/ffmpeg_compiler`, `crates/editor_runtime`, `crates/bindings_node`, `crates/adapter_kaipai`, `crates/testkit`, `apps/desktop-electron/src`, `apps/desktop-electron/tests`, `scripts`, `package.json`.  
**Files scanned:** 323 candidate Rust/TypeScript/TSX/CSS/shell/config files.  
**Pattern extraction date:** 2026-06-25.  
**Primary phase inputs:** `19-CONTEXT.md`, `19-RESEARCH.md`, `19-UI-SPEC.md`, `.planning/REQUIREMENTS.md`, `AGENTS.md`.  
**Project skill used:** `production-architecture-review`.

# Phase 15: audio-engine-and-dsp-timeline-pipeline - Pattern Map

**Mapped:** 2026-06-19
**Files analyzed:** 22 likely new/modified files
**Analogs found:** 22 / 22

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/audio_engine/Cargo.toml` | config | request-response | `crates/realtime_preview_runtime/Cargo.toml` | role-match |
| `crates/audio_engine/src/lib.rs` | service | request-response | `crates/realtime_preview_runtime/src/lib.rs` | role-match |
| `crates/audio_engine/src/dsp_timeline.rs` | model/service | transform | `crates/engine_core/src/frame_state.rs` | role+flow |
| `crates/audio_engine/src/mix_intent.rs` | model | transform | `crates/render_graph/src/graph.rs` | exact |
| `crates/audio_engine/src/session.rs` | service | request-response | `crates/realtime_preview_runtime/src/session.rs` | exact |
| `crates/audio_engine/src/output.rs` | service | streaming | `crates/realtime_preview_runtime/src/frame_provider.rs` / `crates/media_runtime_desktop/src/capabilities.rs` | role-match |
| `crates/audio_engine/src/telemetry.rs` | utility/model | event-driven | `crates/realtime_preview_runtime/src/telemetry.rs` | exact |
| `crates/audio_engine/tests/audio_session_generation.rs` | test | request-response | `crates/realtime_preview_runtime/tests/stale_frame_rejection.rs` | exact |
| `crates/audio_engine/tests/dsp_timeline.rs` | test | transform | `crates/engine_core/tests/frame_state_snapshots.rs` | role-match |
| `crates/audio_output_desktop/Cargo.toml` | config | request-response | `crates/media_runtime_desktop/Cargo.toml` | role-match |
| `crates/audio_output_desktop/src/lib.rs` | service | streaming | `crates/media_runtime_desktop/src/lib.rs` | role-match |
| `crates/audio_output_desktop/src/cpal_output.rs` | service | streaming | `crates/media_runtime_desktop/src/capabilities.rs` | role-match |
| `crates/audio_output_desktop/tests/audio_output_capabilities.rs` | test | request-response | `crates/media_runtime_desktop/tests/capabilities.rs` | role-match |
| `crates/draft_model/src/timeline.rs` | model | CRUD | `crates/draft_model/src/timeline.rs` existing audio/keyframe fields | exact |
| `crates/draft_commands/src/audio.rs` | service | CRUD | `crates/draft_commands/src/audio.rs` existing volume/mute commands | exact |
| `crates/draft_commands/src/delta.rs` | utility | event-driven | `crates/draft_commands/src/delta.rs` audio dirty domains | exact |
| `crates/render_graph/src/graph.rs` | model | transform | `crates/render_graph/src/graph.rs` `RenderAudioMix` | exact |
| `crates/ffmpeg_compiler/src/filters.rs` | service | transform | `crates/ffmpeg_compiler/src/filters.rs` audio filter generation | exact |
| `crates/bindings_node/src/audio_service.rs` | controller/service | request-response | `crates/bindings_node/src/realtime_preview_service.rs` | exact |
| `crates/draft_model/tests/schema_exports.rs` | test | transform | `crates/draft_model/tests/schema_exports.rs` Phase 14 contract test | exact |
| `apps/desktop-electron/src/renderer/workspace/*` | component | request-response | `FeaturePanel.tsx`, `PreviewMonitor.tsx`, `Timeline.tsx`, `Inspector.tsx` | exact |
| `scripts/phase15-source-guards.sh` / `package.json` | config/test | batch | `scripts/phase14-source-guards.sh`, `package.json` Phase 14 gates | exact |

## Pattern Assignments

### `crates/audio_engine/src/session.rs` (service, request-response)

**Analog:** `crates/realtime_preview_runtime/src/session.rs`

**Imports and ownership pattern** (lines 1-16):
```rust
use std::collections::{BTreeMap, BTreeSet};
use draft_model::{Draft, Microseconds, RationalFrameRate};
use crate::{PlaybackGeneration, PlaybackRate, PreviewCancellationToken, TimelineClock};
```

**Opaque runtime session map** (lines 51-79):
```rust
pub struct RealtimePreviewRuntime {
    next_session_id: u64,
    sessions: BTreeMap<PreviewSessionId, RealtimePreviewSession>,
}

pub fn create_session(&mut self, config: RealtimePreviewSessionConfig) -> Result<PreviewSessionId, RealtimePreviewError> {
    let session_id = PreviewSessionId::new(self.next_session_id);
    self.next_session_id = self.next_session_id.saturating_add(1);
    let clock = TimelineClock::new(Microseconds::ZERO, config.frame_rate.clone(), config.playback_rate);
    self.sessions.insert(session_id, RealtimePreviewSession::new(config, clock));
    Ok(session_id)
}
```

**Generation and cancellation pattern** (lines 181-207, 255-290):
```rust
pub fn next_cancellation_token(&mut self, session_id: PreviewSessionId) -> Result<PreviewCancellationToken, RealtimePreviewError> {
    let session = self.session_mut(session_id)?;
    let token = PreviewCancellationToken::new(session.next_cancellation_token);
    session.next_cancellation_token = session.next_cancellation_token.saturating_add(1);
    Ok(token)
}

let stale_rejected = request.playback_generation != self.clock.generation();
let canceled = request.cancellation_token.map(|token| self.canceled_tokens.contains(&token)).unwrap_or(false);
let presented = !stale_rejected && !canceled;
self.telemetry.record_request(&request, presented, stale_rejected, canceled);
```

**Error handling pattern** (lines 365-398):
```rust
pub enum RealtimePreviewError {
    UnknownSession { session_id: PreviewSessionId },
    Surface { session_id: PreviewSessionId, source: PreviewSurfaceError },
}

impl Error for RealtimePreviewError {}
```

Apply this directly to audio sessions: `AudioPreviewRuntime`, `AudioPreviewSessionId`, `AudioBufferRequest`, `AudioBufferResult`, and diagnostics must carry `PlaybackGeneration`, cancellation token, target timeline microseconds, and safe status.

---

### `crates/audio_engine/src/dsp_timeline.rs` (model/service, transform)

**Analogs:** `crates/draft_model/src/timeline.rs`, `crates/engine_core/src/frame_state.rs`, `crates/draft_commands/src/audio.rs`

**Typed integer semantic carriers** (`timeline.rs` lines 9-18, 117-145):
```rust
pub const MAX_SEGMENT_VOLUME_MILLIS: u32 = 4_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum TrackKind {
    Video,
    Audio,
    Text,
}

pub struct Keyframe {
    pub at: Microseconds,
    pub property: KeyframeProperty,
    pub value: KeyframeValue,
}
```

**Accepted draft state evaluation** (`frame_state.rs` lines 65-92, 400-407):
```rust
for track in &normalized.tracks {
    for segment in &track.segments {
        if !covers_timeline_position(segment, at) {
            continue;
        }
        let segment_time = segment_relative_time_at(segment, at)?;
        match track.kind {
            draft_model::TrackKind::Audio => audio_segments.push(FrameAudioSegment {
                volume_level_millis: resolve_segment_volume(segment, segment_time),
                ..audio_segment_fields
            }),
            _ => {}
        }
    }
}

fn resolve_segment_volume(segment: &NormalizedSegment, at: Microseconds) -> u32 {
    resolve_uint_keyframe(segment, KeyframeProperty::Volume, segment.volume_level_millis, at)
}
```

**Validation/command source pattern** (`audio.rs` lines 63-84, 133-145):
```rust
pub fn set_segment_volume(...) -> Result<TimelineCommandResponse, TimelineCommandError> {
    validate_volume(volume)?;
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;
    next_draft.tracks[track_index].segments[segment_index].volume = volume;
    validate_timeline_rules(&next_draft)?;
    let delta = audio_property_delta(...);
    Ok(response(..., delta))
}
```

Use this pattern for pan/fade/effect-slot carriers: commands mutate accepted draft semantics; `audio_engine` only consumes normalized accepted state and produces deterministic DSP plans.

---

### `crates/audio_engine/src/mix_intent.rs` and export integration (model, transform)

**Analog:** `crates/render_graph/src/graph.rs`

**Typed audio intent model** (lines 18-34, 131-144):
```rust
pub struct RenderGraph {
    pub video_layers: Vec<RenderVideoLayer>,
    pub audio_mixes: Vec<RenderAudioMix>,
    pub text_overlays: Vec<RenderTextOverlay>,
}

pub struct RenderAudioMix {
    pub node_id: RenderGraphNodeId,
    pub track_id: TrackId,
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
    pub keyframes: Vec<Keyframe>,
    pub volume_level_millis: u32,
    pub filters: Vec<RenderFilterIntent>,
}
```

**Compiler consumes intent, not renderer syntax** (`crates/ffmpeg_compiler/src/filters.rs` lines 139-177):
```rust
let has_audio_output = !matches!(plan.output_profile, RenderOutputProfile::PreviewFrame { .. })
    && !plan.graph.audio_mixes.is_empty();
if has_audio_output {
    for (audio_index, audio) in plan.graph.audio_mixes.iter().enumerate() {
        let input_index = input_indexes.get(&audio.material_id).ok_or_else(|| missing_input(&audio.material_id))?;
        let Some(clip) = clipped_source_timerange(&audio.source_timerange, &audio.target_timerange, output_timerange(plan)) else {
            continue;
        };
        lines.push(format!(
            "[{input_index}:a]atrim=start={start}:duration={duration},asetpts=PTS-STARTPTS,volume={volume}[{label}]",
            start = format_seconds(clip.start),
            duration = format_seconds(clip.duration),
            volume = volume_arg(audio.volume_level_millis)
        ));
    }
}
```

Phase 15 should add Rust-owned audio mix intent first, then make `render_graph`/`ffmpeg_compiler` consume that intent. Keep filter strings localized to `ffmpeg_compiler`.

---

### `crates/audio_engine/src/output.rs` and `crates/audio_output_desktop/src/cpal_output.rs` (service, streaming)

**Analog:** `crates/media_runtime_desktop/src/capabilities.rs`

**Capability report pattern** (lines 11-30):
```rust
pub fn probe_desktop_runtime_capabilities(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
) -> RuntimeCapabilities {
    let ffmpeg = probe_runtime_capabilities(executor, runtime);
    RuntimeCapabilities {
        media_io: media_io_capabilities(&ffmpeg),
        ffmpeg,
    }
}

fn media_io_capabilities(ffmpeg: &RuntimeCapabilityReport) -> RuntimeMediaIoCapabilities {
    RuntimeMediaIoCapabilities {
        windows: probe_windows_media_io_capabilities(),
        macos: probe_macos_media_io_capabilities(),
        fallback_ladder: fallback_ladder_capability(ffmpeg),
        ..capabilities
    }
}
```

**Status/diagnostic shape** (lines 33-63, 112-182):
```rust
CodecCapability {
    codec: "h264".to_owned(),
    status: RuntimeCapabilityStatus::Warning,
    fallback_reason: Some(MediaIoFallbackReason::HardwareDecodeUnavailable),
    diagnostic: Some("...platform decode proof is pending.".to_owned()),
}
```

Use the same trait + capability-report style for `AudioOutputDevice`, `AudioOutputStream`, mock output, and CPAL-backed CoreAudio/WASAPI. Keep native handles and stream configs inside Rust.

---

### `crates/audio_engine/src/telemetry.rs` (utility/model, event-driven)

**Analog:** `crates/realtime_preview_runtime/src/telemetry.rs`

**Bounded counter pattern** (lines 6-24, 45-90):
```rust
pub struct RealtimePreviewTelemetry {
    pub queue_latency_ms: u64,
    pub render_duration_ms: u64,
    pub presented_frame_count: u64,
    pub stale_rejected_count: u64,
    pub canceled_request_count: u64,
    pub target_time: Microseconds,
    pub generation: PlaybackGeneration,
}

if stale_rejected {
    self.stale_rejected_count = self.stale_rejected_count.saturating_add(1);
}
if canceled {
    self.canceled_request_count = self.canceled_request_count.saturating_add(1);
}
```

Audio telemetry should use saturating counters for presented buffers, stale/canceled buffers, underruns, output degraded/missing, and current generation. Do not expose raw ring buffer internals in production bindings.

---

### `crates/bindings_node/src/audio_service.rs` (controller/service, request-response)

**Analog:** `crates/bindings_node/src/realtime_preview_service.rs`

**Opaque binding session pattern** (lines 15-22, 33-71):
```rust
const SESSION_PREFIX: &str = "rtprev-session-";

pub struct RealtimePreviewBindingRegistry {
    runtime: RealtimePreviewRuntime,
    next_binding_id: u64,
    sessions: BTreeMap<String, PreviewSessionId>,
}

let binding_id = format!("{SESSION_PREFIX}{:016x}", self.next_binding_id);
self.next_binding_id = self.next_binding_id.saturating_add(1);
self.sessions.insert(binding_id.clone(), runtime_id);
Ok(RealtimePreviewSessionBindingResponse {
    session_id: binding_id,
    playback_generation: generation,
})
```

**Safe response mapping** (lines 155-176):
```rust
Ok(RealtimePreviewFrameBindingResponse {
    target_time_microseconds: result.target_time.get(),
    playback_generation: result.playback_generation.get(),
    presented: result.presented,
    stale_rejected: result.stale_rejected,
    canceled: result.canceled,
    diagnostics: result.diagnostics,
    telemetry: result.telemetry,
})
```

**Malformed opaque ID guard** (lines 504-518):
```rust
let suffix = session_id.strip_prefix(SESSION_PREFIX).ok_or_else(|| {
    RealtimePreviewBindingError::new(
        RealtimePreviewBindingErrorKind::MalformedSessionId,
        "realtime preview session IDs are opaque binding IDs",
    )
})?;
if suffix.len() != 16 || !suffix.chars().all(|char| char.is_ascii_hexdigit()) {
    return Err(...);
}
```

Copy this for audio session IDs, device IDs, and cancellation tokens exposed to TS. Bindings should return safe labels/status only, not native handles, backend names in production UI, buffer sizes, artifact roots, or FFmpeg syntax.

---

### Waveform/artifact UI contracts (controller/component, request-response)

**Analogs:** `crates/artifact_store/src/resource_index.rs`, `crates/artifact_store/src/generation.rs`

**Waveform is a resource kind, not draft truth** (`resource_index.rs` lines 14-27, 60-67):
```rust
pub enum ResourceKind {
    Material,
    Proxy,
    Thumbnail,
    Waveform,
    GraphSnapshot,
    PreviewArtifact,
}

pub struct ResourceRef {
    pub kind: ResourceKind,
    pub resource_id: ResourceId,
    pub stable_key: String,
    pub parent_material_id: Option<MaterialId>,
}
```

**Generation stays behind worker/store boundary** (`generation.rs` lines 78-124):
```rust
pub struct GenerationWorkerContext {
    bundle_path: PathBuf,
    pub job_id: String,
    pub chunk_index: u32,
    pub artifact_id: String,
    pub kind: ArtifactKind,
    pub cancel_token: CancelToken,
}

pub trait ArtifactGenerator {
    fn generate_waveform(
        &mut self,
        context: &GenerationWorkerContext,
        request: &WaveformGenerationRequest,
    ) -> Result<GeneratedArtifact, ArtifactStoreError>;
}
```

UI should consume generated command summaries/display-ready peak payloads only. Do not let renderer read SQLite, blob paths, fingerprints, dirty ranges, or artifact roots.

---

### Desktop renderer workspace files (component, request-response)

**Analogs:** `FeaturePanel.tsx`, `PreviewMonitor.tsx`, `Timeline.tsx`, `Inspector.tsx`, `viewModel.ts`

**Five-zone workspace and safe view model imports** (`PreviewMonitor.tsx` lines 1-29, 31-55):
```typescript
import {
  formatMicroseconds,
  summarizeRealtimePreviewDisplay,
  type PreviewDisplayState,
  type RealtimePreviewDisplayModel
} from "../viewModel";

type PreviewMonitorProps = {
  preview: PreviewDisplayState;
  resourcePreviewStatusLabel: string | null;
  playbackRunning: boolean;
  onTogglePlayback: () => void;
  onStopPlayback: () => void;
};
```

**Audio panel pattern to refine, not replace** (`FeaturePanel.tsx` lines 334-425):
```typescript
function AudioPanel({ workspace, onAddAudioSegment, onSetSelectedSegmentVolume, onSetSelectedTrackMute }: FeaturePanelProps) {
  const audioMaterials = workspace.materials.filter((material) => material.kind === "audio" && material.status === "available");
  const selectedSegment = getSelectedSegmentView(workspace.draft, workspace.selection);
  const selectedTrack = getSelectedTrackView(workspace.draft, workspace.selection);
  return (
    <div className="feature-panel-content">
      <div className="panel-header">
        <h2>音频</h2>
        <button className="primary-action">添加音频</button>
      </div>
      ...
    </div>
  );
}
```

**Timeline transport and fixed waveform placeholder** (`Timeline.tsx` lines 253-259, 422-457, 525-527):
```typescript
<TimelineIconButton
  label={isPlaybackRunning ? "暂停" : "播放"}
  symbol={isPlaybackRunning ? "⏸" : "▶"}
  disabled={(pending && !isPlaybackRunning) || !workspace.runtimeDiagnostics.canPreview}
/>

<TrackStateButton label={`${row.track.name} 静音状态：${row.muteLabel}`} symbol="静" active={row.track.muted} />

<span className="audio-waveform-placeholder" aria-label="音频波形占位">
  {AUDIO_WAVEFORM_PLACEHOLDER_PATTERN.map(...)}
</span>
```

**Workspace test harness** (`apps/desktop-electron/tests/workspace.spec.ts` lines 85-111, 153-166):
```typescript
const app = await electron.launch({
  args: [join(process.cwd(), "dist/main/index.cjs")],
  env: {
    VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
    VIDEO_EDITOR_TEST_WORKSPACE_FIXTURE: "demo",
    VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS: "1",
    VIDEO_EDITOR_TEST_MOCK_ARTIFACT_COMMANDS: "1",
  }
});

await expect
  .poll(async () => (await readExecuteCommandCalls(app)).some((call) => call.command === command))
  .toBe(true);
```

Phase 15 should add production labels from `15-UI-SPEC.md`, replace "毫音量" production copy with percent/pan/fade labels, and keep raw runtime terms behind developer diagnostics.

---

### `scripts/phase15-source-guards.sh` and `package.json` gates (config/test, batch)

**Analog:** `scripts/phase14-source-guards.sh`

**Guard helper pattern** (lines 29-64):
```bash
fail() {
  echo "phase14-source-guards: $1" >&2
  exit 1
}

matches_for_pattern() {
  local pattern="$1"
  shift
  rg -n --pcre2 "$pattern" "$@" 2>/dev/null | strip_comments
}

require_fixed() {
  local file="$1"
  local text="$2"
  if ! rg -n --fixed-strings "$text" "$file" >/dev/null; then
    fail "missing required text '${text}' in ${file}"
  fi
}
```

**Negative injected checks** (lines 66-83, 92-115):
```bash
assert_pattern_rejects() {
  local description="$1"
  local pattern="$2"
  local source="$3"
  ...
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase14Violation.ts"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase14Violation.ts" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
}
```

**Package gate style** (`package.json` lines 68-76):
```json
"test:phase14-rust": "cargo test -p artifact_store ... && cargo test -p bindings_node preview_commands -- --nocapture",
"test:phase14-source-guards": "bash scripts/phase14-source-guards.sh",
"test:phase14-workspace": "pnpm --filter @video-editor/desktop test:workspace -g \"资源任务|资源维护|素材资源状态|缓存空间|五大区域\"",
"test:phase14": "pnpm run test:phase14-rust && pnpm run test:phase14-source-guards && pnpm run test:phase14-workspace && pnpm run test:contracts"
```

Phase 15 guard patterns should reject renderer-owned `AudioGraph`, `DSP`, gain curves, pan laws, fade envelopes, sample/ring buffers, native handles, CPAL/CoreAudio/WASAPI terms in production UI, FFmpeg audio filters, SQLite/blob/artifact roots, fingerprints, dirty ranges, and raw session IDs.

---

### Contract schema export tests (test, transform)

**Analog:** `crates/draft_model/tests/schema_exports.rs`

**Contract presence and root pairing pattern** (lines 672-727):
```rust
let command_schema: serde_json::Value =
    serde_json::from_str(&command_schema_json()).expect("command schema should parse");
let command_envelope_ts = command_envelope_ts_contract();

for command_name in ["getArtifactStatus", "refreshArtifactStatus"] {
    assert!(
        command_schema.to_string().contains(command_name) && command_envelope_ts.contains(command_name),
        "generated command contracts should include artifact command {command_name}"
    );
}

assert_eq!(
    paired_command_names, command_name_enum,
    "every CommandName variant must appear exactly once in root command/payload pairing constraints"
);
```

**Forbidden transport fields** (lines 750-792):
```rust
for forbidden in [
    "artifactRoot",
    "blobPath",
    "cacheKey",
    "fingerprint",
    "dirtyRange",
    "sqlite",
] {
    assert!(
        !artifact_contract_text.contains(forbidden),
        "artifact transport contracts must not expose internal field {forbidden}"
    );
}
```

Add equivalent checks for audio commands/status contracts and forbid native handles, backend internals, raw session IDs where unsafe, raw buffer sizes, graph nodes, sample indices in production transport unless explicitly diagnostic.

## Shared Patterns

### Timeline Clock and Generation
**Source:** `crates/realtime_preview_runtime/src/clock.rs` lines 7-27, 96-207
**Apply to:** `audio_engine::session`, audio binding commands, audio tests
```rust
pub struct PlaybackGeneration(u64);

pub struct TimelineClock {
    position: Microseconds,
    frame_rate: RationalFrameRate,
    playback_rate: PlaybackRate,
    state: PlaybackState,
    generation: PlaybackGeneration,
}

pub fn seek(&mut self, target_time: Microseconds) {
    self.position = target_time;
    self.state = PlaybackState::Paused;
    self.advance_generation();
}
```

### Command-Owned Draft Mutation
**Source:** `crates/draft_commands/src/audio.rs` lines 63-130
**Apply to:** pan/fade/audio-effect command additions
```rust
let mut next_draft = draft.clone();
validate_track_unlocked(&next_draft.tracks[track_index])?;
next_draft.tracks[track_index].segments[segment_index].volume = volume;
validate_timeline_rules(&next_draft)?;
let delta = audio_property_delta(...);
Ok(response(next_draft, command_state, draft, selection, ..., delta))
```

### Dirty Domains for Audio
**Source:** `crates/draft_commands/src/delta.rs` lines 49-75, 116-134
**Apply to:** all accepted audio semantic edits
```rust
const AUDIO_PROPERTY_DOMAINS: &[DirtyDomain] = &[
    DirtyDomain::Audio,
    DirtyDomain::ExportPrep,
    DirtyDomain::Waveform,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];
```

### Safe Generated Contracts
**Source:** `crates/draft_model/tests/schema_exports.rs` lines 729-792
**Apply to:** audio command payloads, status summaries, device summaries
```rust
assert!(
    defs.contains_key(expected_contract)
        || command_envelope_ts.contains(&format!("export type {expected_contract}"))
        || command_result_ts.contains(&format!("export type {expected_contract}")),
    "artifact contracts should generate {expected_contract}"
);
```

### Renderer Boundary Guards
**Source:** `scripts/phase14-source-guards.sh` lines 85-187
**Apply to:** `scripts/phase15-source-guards.sh`
```bash
fail_matches \
  "renderer must not construct FFmpeg/ffprobe process commands or filter scripts" \
  "$RENDERER_FFMPEG_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'
```

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| none | n/a | n/a | Existing preview runtime, artifact store, command, export, binding, UI, guard, and contract patterns cover all likely Phase 15 files. |

## Metadata

**Analog search scope:** `crates/realtime_preview_runtime`, `crates/artifact_store`, `crates/media_runtime_desktop`, `crates/preview_service`, `crates/draft_commands`, `crates/engine_core`, `crates/render_graph`, `crates/ffmpeg_compiler`, `crates/bindings_node`, `apps/desktop-electron/src/renderer/workspace`, `apps/desktop-electron/tests`, `scripts`, `package.json`
**Files scanned:** 100+
**Pattern extraction date:** 2026-06-19
**Excluded:** `reference/` was not read.

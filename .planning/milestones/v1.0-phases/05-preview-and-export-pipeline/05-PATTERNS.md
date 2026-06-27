# Phase 05: Preview And Export Pipeline - Pattern Map

**Mapped:** 2026-06-17
**Files analyzed:** 27 likely new/modified files
**Analogs found:** 22 / 27

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/engine_core/src/lib.rs` | service | transform | `crates/draft_commands/src/lib.rs` | role-match |
| `crates/engine_core/src/normalize.rs` | service | transform | `crates/draft_commands/src/timeline.rs` | role-match |
| `crates/engine_core/src/frame_state.rs` | service | transform | `crates/draft_commands/src/timeline.rs` | role-match |
| `crates/engine_core/src/text_layout.rs` | utility | transform | `crates/draft_model/src/timeline.rs` | role-match |
| `crates/render_graph/src/lib.rs` | service | transform | `crates/engine_core/src/lib.rs` | partial |
| `crates/render_graph/src/graph.rs` | model | transform | `crates/draft_model/src/draft.rs` | role-match |
| `crates/render_graph/src/profile.rs` | model | transform | `crates/draft_model/src/material.rs` | role-match |
| `crates/ffmpeg_compiler/src/lib.rs` | service | transform | `crates/media_runtime/src/lib.rs` | role-match |
| `crates/ffmpeg_compiler/src/job.rs` | model | transform | `crates/media_runtime/src/probe.rs` | role-match |
| `crates/ffmpeg_compiler/src/filters.rs` | utility | transform | `crates/media_runtime/src/probe.rs` | partial |
| `crates/ffmpeg_compiler/src/ass.rs` | utility | file-I/O | `crates/testkit/src/lib.rs` | partial |
| `crates/media_runtime/src/job.rs` | service | streaming | `crates/media_runtime/src/process.rs` | role-match |
| `crates/media_runtime/src/validate.rs` | service | request-response | `crates/media_runtime/src/probe.rs` | exact |
| `crates/preview_service/src/lib.rs` | service | request-response | `crates/bindings_node/src/material_service.rs` | role-match |
| `crates/preview_service/src/cache.rs` | service | file-I/O | `crates/project_store/src/paths.rs` | role-match |
| `crates/preview_service/src/service.rs` | service | request-response | `crates/bindings_node/src/material_service.rs` | role-match |
| `crates/bindings_node/src/lib.rs` | route | request-response | `crates/bindings_node/src/lib.rs` | exact |
| `crates/bindings_node/src/preview_export_service.rs` | service | event-driven | `crates/bindings_node/src/material_service.rs` | role-match |
| `crates/draft_model/src/lib.rs` | model | request-response | `crates/draft_model/src/lib.rs` | exact |
| `crates/draft_model/tests/schema_exports.rs` | test | transform | `crates/draft_model/tests/schema_exports.rs` | exact |
| `apps/desktop-electron/src/generated/*.ts` | generated contract | request-response | `crates/draft_model/tests/schema_exports.rs` | exact |
| `apps/desktop-electron/src/renderer/commandHelpers.ts` | utility | request-response | `apps/desktop-electron/src/renderer/commandHelpers.ts` | exact |
| `apps/desktop-electron/src/renderer/App.tsx` | component | event-driven | `apps/desktop-electron/src/renderer/App.tsx` | exact |
| `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` | component | event-driven | `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` | exact |
| `crates/testkit/src/render_compare.rs` | utility | batch | `crates/testkit/src/lib.rs` | role-match |
| `scripts/phase5-source-guards.sh` | test | batch | `scripts/phase4-source-guards.sh` | exact |
| `package.json`, `justfile` | config | batch | `package.json`, `justfile` | exact |

## Pattern Assignments

### `crates/engine_core/src/lib.rs`, `normalize.rs`, `frame_state.rs` (service, transform)

**Analog:** `crates/draft_commands/src/lib.rs` and `crates/draft_commands/src/timeline.rs`

**Public crate boundary pattern** (`crates/draft_commands/src/lib.rs` lines 1-19):
```rust
//! Pure Rust command semantics for draft edits.
//!
//! This crate will own Jianying-style edit commands such as add, move, split,
//! trim, delete, undo/redo, snapping, and MainTrackMagnet behavior. It stays a
//! semantic layer: UI, filesystem, FFmpeg, preview, and platform execution
//! details belong outside this crate.

pub mod audio;
pub mod error;
pub mod history;
pub mod selection;
pub mod snapping;
pub mod text;
pub mod timeline;

pub use error::{TimelineCommandError, TimelineCommandErrorKind};
pub use selection::TimelineSelection;
```

**Core transform pattern** (`crates/draft_commands/src/timeline.rs` lines 48-55, 110-126):
```rust
pub fn validate_timeline_rules(draft: &Draft) -> Result<(), TimelineCommandError> {
    validate_timeranges(draft)?;
    validate_track_material_rules(draft)?;
    validate_segment_material_bounds(draft)?;
    validate_track_overlaps(draft)?;
    validate_draft(draft)?;
    Ok(())
}

pub fn visual_track_stack_order(draft: &Draft) -> Vec<TrackId> {
    draft
        .tracks
        .iter()
        .filter(|track| is_visual_track(track.kind))
        .map(|track| track.track_id.clone())
        .collect()
}
```

**Error handling pattern** (`crates/draft_commands/src/error.rs` lines 10-22, 73-79):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineCommandError {
    pub kind: TimelineCommandErrorKind,
}

impl TimelineCommandError {
    pub fn new(kind: TimelineCommandErrorKind) -> Self {
        Self { kind }
    }
}

impl From<DraftValidationError> for TimelineCommandError {
    fn from(error: DraftValidationError) -> Self {
        Self::new(TimelineCommandErrorKind::DraftValidationFailed {
            message: error.to_string(),
        })
    }
}
```

Apply this to `engine_core`: keep filesystem/Electron/FFmpeg out; expose `NormalizedDraft`, `FrameState`, and `resolve_*` APIs from `lib.rs`; return classified semantic errors instead of strings.

### `crates/render_graph/src/lib.rs`, `graph.rs`, `profile.rs` (service/model, transform)

**Analog:** `crates/draft_model/src/draft.rs` and current `crates/render_graph/src/lib.rs`

**Serde/ts-rs model pattern** (`crates/draft_model/src/draft.rs` lines 41-49):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub schema_version: DraftSchemaVersion,
    pub draft_id: DraftId,
    pub metadata: DraftMetadata,
    pub materials: Vec<Material>,
    pub tracks: Vec<Track>,
}
```

**Boundary pattern** (`crates/render_graph/src/lib.rs` lines 1-8):
```rust
//! Typed render intent graph.
//!
//! This crate will translate resolved draft frame state into a renderer-neutral
//! graph of materials, tracks, segments, filters, transitions, and text intents.
//! It does not execute FFmpeg jobs or decide editing behavior.

/// Boundary marker for render intent graph types.
pub const RENDER_GRAPH_BOUNDARY: &str = "semantic-render-intents";
```

Apply this to graph/profile types: use renderer-neutral Jianying terms (`material`, `track`, `segment`, `text`, `filter`, `transition`), stable ordering (`Vec` or `BTree*`), serde camelCase, no process execution, no FFmpeg arg strings.

### `crates/ffmpeg_compiler/src/lib.rs`, `job.rs`, `filters.rs`, `ass.rs` (service/model/utility, transform and file-I/O)

**Analog:** `crates/media_runtime/src/lib.rs`, `crates/media_runtime/src/probe.rs`, `crates/testkit/src/lib.rs`

**Runtime boundary separation** (`crates/media_runtime/src/lib.rs` lines 1-4, 27-43):
```rust
//! FFmpeg process runtime boundary.
//!
//! This crate owns the service boundary for FFmpeg and ffprobe execution. Pure
//! draft and timeline semantic crates must not depend on this trait.

pub trait FfmpegExecutor {
    fn executor_name(&self) -> &'static str;
    fn can_execute(&self, binary: &Path) -> bool;
    fn run_version_probe(&self, binary: &Path) -> std::io::Result<Output>;
    fn run(&self, binary: &Path, args: &[OsString]) -> std::io::Result<Output>;
}
```

**Structured job metadata pattern** (`crates/media_runtime/src/probe.rs` lines 38-50):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialProbeMetadata {
    pub status: MaterialProbeStatus,
    pub kind: MaterialProbeKind,
    pub duration_microseconds: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<RationalFrameRate>,
    pub has_video_stream: bool,
    pub has_audio_stream: bool,
    pub audio: Option<MaterialProbeAudio>,
}
```

**FFmpeg args as vectors, not shell strings** (`crates/media_runtime/src/probe.rs` lines 151-165):
```rust
let args = vec![
    OsString::from("-v"),
    OsString::from("error"),
    OsString::from("-print_format"),
    OsString::from("json"),
    OsString::from("-show_entries"),
    OsString::from(
        "stream=codec_type,codec_name,width,height,r_frame_rate,avg_frame_rate,duration,sample_rate,channels:format=duration",
    ),
    path.as_os_str().to_owned(),
];

let output = executor
    .run(&runtime.ffprobe.path, &args)
    .map_err(|error| process_error(error, path, runtime, executor))?;
```

Apply this to `FfmpegJob`: emit `Vec<OsString>` or structured args, filter script text/sidecars as derived artifacts, output validation expectations as data. Do not execute processes in `ffmpeg_compiler`.

### `crates/media_runtime/src/job.rs` and `validate.rs` (service, streaming/request-response)

**Analog:** `crates/media_runtime/src/process.rs` and `crates/media_runtime/src/probe.rs`

**Process launch/timeout pattern** (`crates/media_runtime/src/process.rs` lines 11-43):
```rust
pub fn run_process_with_timeout(
    binary: &Path,
    args: &[OsString],
    timeout: Duration,
) -> io::Result<Output> {
    let mut child = Command::new(binary)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let started = Instant::now();

    loop {
        if child.try_wait()?.is_some() {
            return child.wait_with_output();
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(io::ErrorKind::TimedOut, "..."));
        }
        thread::sleep(Duration::from_millis(10));
    }
}
```

**Classified runtime errors** (`crates/media_runtime/src/probe.rs` lines 52-76):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MaterialProbeErrorKind {
    MissingInput,
    RuntimeUnavailable,
    ProcessLaunchFailed,
    Timeout,
    ProbeFailed,
    MalformedJson,
    MissingStreams,
    InvalidDuration,
    InvalidFrameRate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialProbeError {
    pub kind: MaterialProbeErrorKind,
    pub path: PathBuf,
    pub ffprobe_path: PathBuf,
    pub executor: String,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
    pub message: String,
}
```

Apply this to export jobs: keep bounded logs, classify timeout/cancel/nonzero/malformed progress/validation failures, parse ffprobe JSON through the same normalized metadata style.

### `crates/preview_service/src/lib.rs`, `cache.rs`, `service.rs` (service, request-response/file-I/O)

**Analog:** `crates/bindings_node/src/material_service.rs`

**Service request/result pattern** (`crates/bindings_node/src/material_service.rs` lines 20-70):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportMaterialRequest {
    pub material_id: Option<MaterialId>,
    pub path: PathBuf,
    pub display_name: Option<String>,
    pub material_kind_hint: Option<MaterialKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedMaterialImport {
    pub material: Material,
    pub diagnostic: Option<MissingMaterialDiagnostic>,
    pub bundle_path: PathBuf,
    pub project_json_path: PathBuf,
}
```

**Recoverable diagnostic pattern** (`crates/bindings_node/src/material_service.rs` lines 176-217):
```rust
match probe_material_metadata(executor, runtime, &request.path) {
    Ok(metadata) => { /* update semantic state */ }
    Err(error) => {
        let status = if error.kind == MaterialProbeErrorKind::MissingInput {
            MaterialStatus::Missing
        } else {
            MaterialStatus::ProbeFailed
        };
        let diagnostic_kind = if status == MaterialStatus::Missing {
            MissingMaterialDiagnosticKind::MissingFile
        } else {
            MissingMaterialDiagnosticKind::ProbeFailed
        };
        /* return recoverable result with diagnostic */
    }
}
```

Apply this to preview generation: requests should contain draft/bundle/cache root/time range/profile; responses should return artifact metadata and diagnostics without mutating canonical draft. Cache invalidation should be conservative and range-based.

### `crates/bindings_node/src/lib.rs` and `preview_export_service.rs` (route/service, request-response/event-driven)

**Analog:** `crates/bindings_node/src/lib.rs`

**Command allowlist and dispatch pattern** (`crates/bindings_node/src/lib.rs` lines 42-87):
```rust
#[napi]
pub fn execute_command(command: serde_json::Value) -> Result<serde_json::Value> {
    let command_name = raw_command_name(&command);

    if let Some(name) = command_name.as_deref() {
        if !matches!(name, "ping" | "version" | "probeMediaRuntime" /* ... */) {
            return to_js_value(error_envelope(
                CommandErrorKind::UnsupportedCommand,
                format!("Unsupported command: {name}"),
                Some(name.to_string()),
            ));
        }
    }

    let envelope = match serde_json::from_value::<CommandEnvelope>(command) {
        Ok(envelope) => envelope,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid command envelope: {error}"),
                command_name,
            ));
        }
    };

    match envelope.command {
        CommandName::Ping => to_js_value(ping_envelope()),
        CommandName::Version => to_js_value(version_envelope()),
        /* command-specific routing */
    }
}
```

**Envelope helpers** (`crates/bindings_node/src/lib.rs` lines 135-159, 330-345):
```rust
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
) -> CommandResultEnvelope<serde_json::Value> { /* same envelope shape */ }

fn command_wire_name(command: &CommandName) -> Option<String> {
    serde_json::to_value(command)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
}
```

Apply this to preview/export commands: add command names to the Rust contract, allowlist them before deserialization, route to Rust-owned services, and return `CommandResultEnvelope` with events/progress metadata rather than renderer-built jobs.

### `crates/draft_model/src/lib.rs` and `tests/schema_exports.rs` (model/test, request-response/transform)

**Analog:** same files

**Command contract pattern** (`crates/draft_model/src/lib.rs` lines 37-46, 48-76):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandEnvelope {
    pub command: CommandName,
    pub payload: CommandPayload,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum CommandName {
    Ping,
    Version,
    ProbeMediaRuntime,
    ImportMaterial,
    /* ... */
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CommandPayload {
    Ping(PingCommandPayload),
    Version(VersionCommandPayload),
    /* ... */
}
```

**Generated contract update pattern** (`crates/draft_model/tests/schema_exports.rs` lines 36-105, 336-417):
```rust
#[test]
fn schema_exports_generated_contract_artifacts_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/command.schema.json");
    let draft_schema_path = root.join("schemas/draft.schema.json");
    let generated_dir = root.join("apps/desktop-electron/src/generated");

    let schema_json = command_schema_json();
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));
    /* build TS declarations with export_decl::<Type>() */
}

fn ts_config() -> Config {
    Config::new().with_large_int("number")
}

fn assert_or_update_contract_file(path: impl AsRef<Path>, expected: &str) {
    if env::var_os("VE_UPDATE_GENERATED_CONTRACTS").as_deref() == Some(std::ffi::OsStr::new("1")) {
        fs::create_dir_all(path.parent().expect("contract path should have parent")).expect("contract directory should be created");
        fs::write(path, expected).expect("contract artifact should be written");
        return;
    }
    let actual = fs::read_to_string(path).unwrap_or_else(|error| panic!("committed contract artifact should be readable: {error}"));
    assert_eq!(actual, expected, "generated contract artifact is stale");
}
```

Apply this to `PreviewFrameCommandPayload`, `PreviewSegmentCommandPayload`, `StartExportCommandPayload`, `CancelExportCommandPayload`, and response/event structs. Never hand-edit generated TS except by running the schema export update path.

### `apps/desktop-electron/src/renderer/commandHelpers.ts` (utility, request-response)

**Analog:** same file

**Command builder pattern** (`apps/desktop-electron/src/renderer/commandHelpers.ts` lines 48-60, 337-342):
```typescript
export function buildImportMaterialCommand(options: ImportMaterialOptions): CommandEnvelope {
  const payload = {
    kind: "importMaterial",
    draft: options.draft,
    bundlePath: options.bundlePath,
    materialPath: options.materialPath,
    materialId: options.materialId ?? null,
    displayName: options.displayName ?? null,
    materialKindHint: options.materialKindHint ?? null
  } satisfies ImportMaterialCommandPayload & { kind: "importMaterial" };

  return envelope("importMaterial", payload);
}

function envelope(command: CommandEnvelope["command"], payload: CommandEnvelope["payload"]): CommandEnvelope {
  return {
    command,
    payload,
    requestId: `${command}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`
  };
}
```

**Result/error pattern** (`apps/desktop-electron/src/renderer/commandHelpers.ts` lines 307-335):
```typescript
export function applyTimelineCommandResult(
  current: CommandContext,
  result: CommandResultEnvelope<TimelineCommandResponse>
): { state: CommandContext; errorMessage: string | null } {
  if (!result.ok || result.data === null) {
    return {
      state: current,
      errorMessage: commandErrorMessage(result)
    };
  }

  return {
    state: {
      draft: result.data.draft,
      commandState: result.data.commandState,
      selection: result.data.selection
    },
    errorMessage: null
  };
}
```

Apply this to preview/export helpers: build typed envelopes only; do not calculate render graphs, FFmpeg args, cache keys, waveform data, or derived scripts in TypeScript.

### `apps/desktop-electron/src/renderer/App.tsx` and `workspace/PreviewMonitor.tsx` (component, event-driven)

**Analog:** same files

**Renderer command execution pattern** (`apps/desktop-electron/src/renderer/App.tsx` lines 140-190):
```typescript
async function executeDraftCommand<T>(
  buildCommand: DraftCommandBuilder,
  pendingCommand: string,
  applyResult: DraftCommandResultApplier<T>
): Promise<void> {
  if (commandInFlightRef.current) {
    setWorkspace((current) => ({
      ...current,
      commandError: commandErrorMessage("上一个操作仍在执行，请等待剪辑核心返回")
    }));
    return;
  }

  commandInFlightRef.current = true;
  setWorkspace((current) => ({ ...current, pendingCommand, commandError: null }));

  try {
    const command = buildCommand(workspaceRef.current);
    const result = await window.videoEditorCore.executeCommand<T>(command);
    setWorkspace((current) => applyResult(current, result));
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    setWorkspace((current) => ({ ...current, pendingCommand: null, commandError: commandErrorMessage(message) }));
  } finally {
    commandInFlightRef.current = false;
  }
}
```

**Preview placeholder integration point** (`apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` lines 8-21):
```tsx
export function PreviewMonitor({ draftName, bindingStatus }: PreviewMonitorProps): React.ReactElement {
  return (
    <div className="preview-shell">
      <div className="preview-stage" aria-label="预览画面">
        <div className="preview-placeholder">
          <strong>预览将在下一阶段接入</strong>
          <span>{draftName}</span>
        </div>
      </div>
      <div className="preview-status" aria-live="polite">
        <span className={`status-dot ${bindingStatus.kind}`} />
        <span>{bindingStatus.label}</span>
      </div>
    </div>
  );
}
```

Apply this with Simplified Chinese labels and `aria-live` status. Renderer may store playhead/output path/progress UI state, but semantic preview/export work must remain behind `executeCommand`.

### `crates/testkit/src/render_compare.rs` and Phase 5 tests (utility/test, batch)

**Analog:** `crates/testkit/src/lib.rs`, `crates/testkit/tests/render_smoke.rs`, `crates/media_runtime/tests/material_probe.rs`

**Deterministic FFmpeg fixture pattern** (`crates/testkit/src/lib.rs` lines 213-245):
```rust
pub fn generate_tiny_lavfi_media() -> SmokeResult<TinyLavfiMedia> {
    let runtime = discover_runtime_config()?;
    let executor = DesktopFfmpegExecutor::default();
    let temp_dir = tempfile::Builder::new()
        .prefix("media-generated-")
        .tempdir()?;
    let media_dir = temp_dir.path().join("media-generated");
    std::fs::create_dir_all(&media_dir)?;
    let output_path = media_dir.join("tiny-render-smoke.mp4");

    run_ffmpeg_generate(&executor, &runtime, &output_path)?;

    if !output_path.is_file() {
        return Err(SmokeError::new(format!(
            "ffmpeg completed but did not create {}",
            output_path.display()
        )));
    }

    Ok(TinyLavfiMedia { _temp_dir: temp_dir, output_path })
}
```

**Runtime test pattern with fake executor** (`crates/media_runtime/tests/material_probe.rs` lines 95-123, 175-237):
```rust
#[test]
fn material_probe_bounds_process_output_and_classifies_timeout() {
    let temp_dir = tempfile::tempdir().expect("temp dir should create");
    let input = temp_dir.path().join("input.mp4");
    fs::write(&input, b"placeholder").expect("input fixture should write");
    let runtime = fake_runtime(temp_dir.path().join("ffprobe"));

    let timeout_executor = FakeExecutor::timeout();
    let timeout = probe_material_metadata(&timeout_executor, &runtime, &input)
        .expect_err("timeout should be classified");

    assert_eq!(timeout.kind, MaterialProbeErrorKind::Timeout);
}

impl FfmpegExecutor for FakeExecutor {
    fn executor_name(&self) -> &'static str { "fake-material-probe-executor" }
    fn can_execute(&self, _binary: &Path) -> bool { true }
    fn run_version_probe(&self, _binary: &Path) -> io::Result<Output> { self.run(_binary, &[]) }
    fn run(&self, _binary: &Path, _args: &[OsString]) -> io::Result<Output> { /* fake output */ }
}
```

Apply this to preview/export parity: generated temporary media, explicit expected metadata/tolerance helpers, fake executors for classification, and clear setup errors for missing FFmpeg/ffprobe.

### `scripts/phase5-source-guards.sh`, `package.json`, `justfile` (test/config, batch)

**Analog:** `scripts/phase4-source-guards.sh`, `package.json`, `justfile`

**Guard script pattern** (`scripts/phase4-source-guards.sh` lines 1-25, 59-68, 96-100):
```bash
#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase4-source-guards: rg is required" >&2
  exit 1
fi

fail_if_matches() {
  local description="$1"
  local pattern="$2"
  shift 2

  local output
  if output=$(rg -n --pcre2 "$pattern" "$@" 2>/dev/null); then
    echo "phase4-source-guards: ${description}" >&2
    echo "$output" >&2
    exit 1
  fi
}

fail_if_matches \
  "renderer must not construct FFmpeg, render graph, preview cache, or waveform behavior" \
  'ffmpeg|ffprobe|filter_complex|renderGraph|ffmpegScripts|previewCache|waveform' \
  "${renderer_files[@]}"

git diff --exit-code schemas "$GENERATED_DIR"
```

**Root gate pattern** (`package.json` lines 16-32, `justfile` lines 17-34):
```json
"test:runtime": "cargo test -p media_runtime discovery -- --nocapture",
"test:render-smoke": "cargo test -p testkit render_smoke -- --nocapture",
"test:phase4-source-guards": "bash scripts/phase4-source-guards.sh",
"test": "pnpm run test:rust && ... && pnpm run test:contracts"
```

```just
test:
  pnpm install --frozen-lockfile
  pnpm run test:rust
  pnpm run test:schema
  pnpm run test:render-smoke
  pnpm run test:phase4-source-guards
  pnpm run test:contracts
```

Apply this to add `test:phase5-render-core` and `test:phase5-source-guards`, then chain both into `pnpm run test` and `just test`.

## Shared Patterns

### Rust Semantic Purity
**Source:** `crates/draft_commands/src/lib.rs` lines 1-6; `crates/engine_core/src/lib.rs` lines 1-5
**Apply to:** `engine_core`, `render_graph`

Pure crates must not depend on filesystem, Electron, FFmpeg process execution, or platform runtime. Keep APIs as data transforms over `Draft`, normalized state, frame state, and render intents.

### Classified Errors
**Source:** `crates/media_runtime/src/probe.rs` lines 52-76; `crates/draft_commands/src/error.rs` lines 21-71
**Apply to:** all service/runtime modules

Use stable error-kind enums plus structured context fields. Do not return unclassified strings except as display messages.

### Command Envelope Boundary
**Source:** `crates/draft_model/src/lib.rs` lines 37-152; `crates/bindings_node/src/lib.rs` lines 42-122
**Apply to:** preview/export IPC commands, renderer helpers, generated TS contracts

Every renderer request should be a generated `CommandEnvelope`; binding dispatch validates command/payload pairing and maps service results into `CommandResultEnvelope`.

### Generated Contracts
**Source:** `crates/draft_model/tests/schema_exports.rs` lines 36-147 and 336-417
**Apply to:** `schemas/*.json`, `apps/desktop-electron/src/generated/*.ts`

Rust serde/ts-rs types are source of truth. Regenerate via `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture`; guard with `git diff --exit-code schemas apps/desktop-electron/src/generated`.

### Renderer Source Guards
**Source:** `scripts/phase4-source-guards.sh` lines 59-68
**Apply to:** `apps/desktop-electron/src/renderer/**`, desktop tests

Preserve and extend the guard that forbids renderer FFmpeg/render graph/cache/waveform construction. Add Phase 5 terms such as `FfmpegJob`, `filter_complex`, `ass`, `previewSegmentCache`, `renderIntent`, and direct `child_process` usage as needed.

### FFmpeg Runtime Boundary
**Source:** `crates/media_runtime/src/lib.rs` lines 27-43; `crates/media_runtime_desktop/src/lib.rs` lines 35-51
**Apply to:** export runtime, preview generation, validation

Process execution goes through `FfmpegExecutor` implementations and explicit argument vectors. Compiler crates produce plans; runtime crates execute and report progress/errors.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/render_graph/src/graph.rs` | model | transform | No implemented render graph model exists yet; copy serde model conventions, not behavior. |
| `crates/ffmpeg_compiler/src/filters.rs` | utility | transform | No existing FFmpeg filter-script generator exists; use vector/script boundary and snapshot tests from runtime/testkit. |
| `crates/ffmpeg_compiler/src/ass.rs` | utility | file-I/O | No subtitle/ASS sidecar generator exists; use deterministic file artifact and snapshot conventions. |
| `crates/preview_service/src/cache.rs` | service | file-I/O | No preview cache exists; use project path/file-boundary patterns and range overlap helpers from `draft_commands`. |
| `crates/media_runtime/src/job.rs` | service | streaming | Existing runtime is blocking; streaming progress/cancel needs a new API while preserving process/error patterns. |

## Metadata

**Analog search scope:** `crates/**`, `apps/desktop-electron/src/**`, `apps/desktop-electron/tests/**`, `scripts/**`, root `package.json`, root `justfile`; excluded `reference/**`.
**Files scanned:** 57 repo files from `rg --files`/`find`, plus required Phase 5 planning docs and `AGENTS.md`.
**Pattern extraction date:** 2026-06-17

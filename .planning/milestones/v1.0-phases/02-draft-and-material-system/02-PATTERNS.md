# Phase 02: Draft And Material System - Pattern Map

**Mapped:** 2026-06-17
**Files analyzed:** 18 file/module families
**Analogs found:** 18 / 18

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/draft_model/src/lib.rs` | model | request-response, transform | `crates/draft_model/src/lib.rs` | exact-extension |
| `crates/draft_model/src/draft.rs` | model | CRUD, transform | `crates/draft_model/src/lib.rs` | role-match |
| `crates/draft_model/src/material.rs` | model | CRUD, transform | `crates/draft_model/src/lib.rs` | role-match |
| `crates/draft_model/src/timeline.rs` | model | CRUD, transform | `crates/draft_model/src/lib.rs` | role-match |
| `crates/draft_model/src/ids.rs` | utility | transform | `crates/draft_model/src/lib.rs` | role-match |
| `crates/draft_model/src/validation.rs` | utility | transform | `crates/draft_model/src/lib.rs` | role-match |
| `crates/draft_model/tests/draft_schema.rs` | test | transform | `crates/draft_model/tests/contract.rs` | exact |
| `crates/draft_model/tests/schema_exports.rs` | test | file-I/O, transform | `crates/draft_model/tests/schema_exports.rs` | exact-extension |
| `schemas/draft.schema.json` | config | transform | `schemas/command.schema.json` via `schema_exports.rs` | generated |
| `apps/desktop-electron/src/generated/Draft.ts` | config | transform | `apps/desktop-electron/src/generated/CommandEnvelope.ts` via `schema_exports.rs` | generated |
| `crates/project_store/src/lib.rs` | service | file-I/O, CRUD | `crates/project_store/src/lib.rs` | exact-extension |
| `crates/project_store/src/bundle.rs` | service | file-I/O, CRUD | `crates/project_store/src/lib.rs` | role-match |
| `crates/project_store/src/paths.rs` | utility | file-I/O, transform | `crates/project_store/src/lib.rs` | role-match |
| `crates/project_store/src/error.rs` | utility | request-response | `crates/media_runtime/src/error.rs` | role-match |
| `crates/project_store/tests/project_bundle.rs` | test | file-I/O, CRUD | `crates/draft_model/tests/schema_exports.rs` | flow-match |
| `crates/media_runtime/src/probe.rs` | service | request-response, process-I/O | `crates/media_runtime/src/discovery.rs` and `crates/testkit/src/lib.rs` | exact |
| `crates/media_runtime/tests/material_probe.rs` | test | request-response, process-I/O | `crates/media_runtime/tests/discovery.rs` | exact |
| `crates/testkit/src/media.rs` / `crates/testkit/src/project.rs` | utility | file-I/O, process-I/O | `crates/testkit/src/lib.rs` | exact-extension |
| `fixtures/draft/positive/*`, `fixtures/draft/negative/*` | test fixture | file-I/O, transform | `fixtures/draft/*.json` plus `schema_exports.rs` fixture classifier | exact-extension |
| `crates/bindings_node/src/lib.rs` | controller | request-response | `crates/bindings_node/src/lib.rs` | exact-extension |
| `apps/desktop-electron/src/renderer/App.tsx` | component | request-response | `apps/desktop-electron/src/renderer/App.tsx` | role-match |

## Pattern Assignments

### `crates/draft_model/src/lib.rs` and schema modules (model, transform)

**Analog:** `crates/draft_model/src/lib.rs`

**Imports pattern** (lines 8-11):
```rust
use schemars::JsonSchema;
use serde::de;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
```

**Serde/schema/TS derive pattern** (lines 16-24):
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
```

**Tagged enum pattern** (lines 36-43):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CommandPayload {
    Ping(PingCommandPayload),
    Version(VersionCommandPayload),
    ProbeMediaRuntime(ProbeMediaRuntimeCommandPayload),
}
```

**Strict validation hook pattern** (lines 56-82):
```rust
impl<'de> Deserialize<'de> for CommandEnvelope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct CommandEnvelopeFields {
            command: CommandName,
            payload: CommandPayload,
            #[serde(default)]
            request_id: Option<String>,
        }

        let fields = CommandEnvelopeFields::deserialize(deserializer)?;
        if fields.payload.command_name() != fields.command {
            return Err(de::Error::custom(
                "command name does not match payload kind",
            ));
        }

        Ok(Self {
            command: fields.command,
            payload: fields.payload,
            request_id: fields.request_id,
        })
    }
}
```

**Apply to Phase 2:** Define `Draft`, `Material`, `Track`, `Segment`, `SourceTimerange`, `TargetTimerange`, `MainTrackMagnet`, `Keyframe`, `Filter`, and `Transition` as pure semantic types with `Serialize`, `Deserialize`, `JsonSchema`, and `TS`. Keep `#[serde(rename_all = "camelCase", deny_unknown_fields)]` on persisted structs. Use integer microseconds and rational frame-rate fields; do not use floating persisted seconds. Add schema-version validation/migration hooks here, not in `project_store`.

### `crates/draft_model/tests/draft_schema.rs` (test, transform)

**Analog:** `crates/draft_model/tests/contract.rs`

**Imports pattern** (lines 1-5):
```rust
use draft_model::{
    CommandEnvelope, CommandError, CommandErrorKind, CommandEvent, CommandName, CommandPayload,
    CommandResultEnvelope, PingResponse, VersionResponse,
};
use serde_json::json;
```

**Positive deserialization pattern** (lines 7-18):
```rust
#[test]
fn contract_deserializes_phase_one_command_envelopes() {
    let ping: CommandEnvelope = serde_json::from_value(json!({
        "command": "ping",
        "payload": { "kind": "ping" },
        "requestId": "req-ping-1"
    }))
    .expect("ping command envelope should deserialize");

    assert_eq!(ping.command, CommandName::Ping);
    assert!(matches!(ping.payload, CommandPayload::Ping(_)));
    assert_eq!(ping.request_id.as_deref(), Some("req-ping-1"));
}
```

**Negative strict-field pattern** (lines 96-105):
```rust
#[test]
fn contract_rejects_unknown_top_level_fields() {
    let result = serde_json::from_value::<CommandEnvelope>(json!({
        "command": "ping",
        "payload": { "kind": "ping" },
        "unexpected": true
    }));

    assert!(result.is_err(), "unknown envelope fields must fail");
}
```

**Apply to Phase 2:** Add draft schema tests for valid empty drafts, material records, tracks/segments/timeranges, unknown-field rejection, unknown future schema-version rejection, missing material status preservation, and terminology checks that reject `asset`/`clip` fixture drift.

### `crates/draft_model/tests/schema_exports.rs`, `schemas/draft.schema.json`, generated TS (test/config, file-I/O)

**Analog:** `crates/draft_model/tests/schema_exports.rs`

**Rust-owned artifact generation pattern** (lines 24-55):
```rust
#[test]
fn schema_exports_generated_contract_artifacts_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/command.schema.json");
    let generated_dir = root.join("apps/desktop-electron/src/generated");

    let schema_json = command_schema_json();
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));

    let command_envelope_ts = ts_contract(&[
        export_decl::<CommandName>(),
        export_decl::<PingCommandPayload>(),
        export_decl::<VersionCommandPayload>(),
        export_decl::<ProbeMediaRuntimeCommandPayload>(),
        export_decl::<CommandPayload>(),
        export_decl::<CommandEnvelope>(),
    ]);
    assert_or_update_contract_file(
        generated_dir.join("CommandEnvelope.ts"),
        &command_envelope_ts,
    );
}
```

**Update-or-compare drift gate** (lines 75-96):
```rust
fn assert_or_update_contract_file(path: impl AsRef<Path>, expected: &str) {
    let path = path.as_ref();

    if env::var_os("VE_UPDATE_GENERATED_CONTRACTS").as_deref() == Some(std::ffi::OsStr::new("1")) {
        fs::create_dir_all(path.parent().expect("contract path should have parent"))
            .expect("contract directory should be created");
        fs::write(path, expected).expect("contract artifact should be written");
        return;
    }

    let actual = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "committed contract artifact should be readable at {}: {error}",
            path.display()
        )
    });
    assert_eq!(
        actual,
        expected,
        "generated contract artifact is stale: {}. Run with VE_UPDATE_GENERATED_CONTRACTS=1 to refresh.",
        path.display()
    );
}
```

**Fixture classification pattern** (lines 99-161):
```rust
let fixture_dir = root.join("fixtures/draft");
let positive = BTreeSet::from(["minimal-command.json"]);
let negative = BTreeSet::from([
    "invalid-mismatched-command-payload.json",
    "invalid-unknown-field.json",
]);

let expected = positive.union(&negative).copied().collect::<BTreeSet<_>>();
assert_eq!(
    actual, expected,
    "every draft JSON fixture must be explicitly classified"
);
```

**Apply to Phase 2:** Extend the same test to write/compare `schemas/draft.schema.json` and generated `Draft.ts`/material contract declarations. Keep fixture sets explicit for `fixtures/draft/positive` and `fixtures/draft/negative` or use a similarly explicit classifier. Final verification should include `git diff --exit-code schemas apps/desktop-electron/src/generated`.

### `crates/project_store/src/lib.rs`, bundle/path modules (service, file-I/O)

**Analog:** `crates/project_store/src/lib.rs`

**Boundary documentation pattern** (lines 1-6):
```rust
//! `.veproj` project store service boundary.
//!
//! This crate owns filesystem abstraction for project bundle persistence. The
//! canonical project state will live in `.veproj/project.json`; previews,
//! waveforms, render graphs, FFmpeg scripts, and exports remain derived
//! artifacts outside the semantic draft model.
```

**Filesystem trait pattern** (lines 11-22):
```rust
pub trait PlatformFileSystem {
    /// Reads a UTF-8 project file from disk.
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// Writes a UTF-8 project file to disk, creating parent directories first
    /// when the platform supports it.
    fn write_string(&self, path: &Path, contents: &str) -> io::Result<()>;

    /// Returns whether a path exists.
    fn exists(&self, path: &Path) -> bool;
}
```

**Desktop implementation pattern** (lines 28-43):
```rust
impl PlatformFileSystem for StdPlatformFileSystem {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn write_string(&self, path: &Path, contents: &str) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, contents)
    }

    fn exists(&self, path: &Path) -> bool {
        PathBuf::from(path).exists()
    }
}
```

**Apply to Phase 2:** Implement `.veproj/project.json` create/open/save/autosave behind `PlatformFileSystem`. Path normalization and relative/external URI decisions belong here. Call `draft_model` validation/migration, but do not encode editing semantics in this crate. Preserve missing material entries and return diagnostics instead of deleting semantic records.

### `crates/project_store/src/error.rs` (utility, request-response)

**Analog:** `crates/media_runtime/src/error.rs`

**Structured error enum pattern** (lines 9-19):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Error)]
#[serde(rename_all = "camelCase")]
pub enum DiscoveryErrorKind {
    #[error("missing binary")]
    MissingBinary,
    #[error("version probe failed")]
    VersionProbeFailed,
    #[error("unsupported version")]
    UnsupportedVersion,
}
```

**UI-ready error payload pattern** (lines 21-31):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryError {
    pub kind: DiscoveryErrorKind,
    pub binary: BinaryKind,
    pub checked_paths: Vec<PathBuf>,
    pub remediation: String,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
}
```

**Constructor/remediation pattern** (lines 33-47):
```rust
impl DiscoveryError {
    pub(crate) fn missing_binary(binary: BinaryKind, checked_paths: Vec<PathBuf>) -> Self {
        let env_var = binary.env_var();
        let binary_name = binary.binary_name();
        Self {
            kind: DiscoveryErrorKind::MissingBinary,
            binary,
            checked_paths,
            remediation: format!(
                "Set {env_var} to a valid {binary_name} binary or install {binary_name} on PATH."
            ),
            stdout_summary: None,
            stderr_summary: None,
        }
    }
}
```

**Apply to Phase 2:** Use structured errors such as `InvalidProjectJson`, `UnsupportedSchemaVersion`, `ProjectIoFailed`, and recoverable warning/diagnostic types for missing materials. Include enough path/URI/remediation detail for later UI without reparsing JSON.

### `crates/project_store/tests/project_bundle.rs` (test, file-I/O)

**Analogs:** `crates/draft_model/tests/schema_exports.rs`, `crates/media_runtime/tests/discovery.rs`

**Root path pattern** from schema tests (lines 16-22):
```rust
fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("draft_model should live under crates/")
        .to_path_buf()
}
```

**Sandbox cleanup pattern** from discovery tests (lines 147-207):
```rust
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "video-editor-media-runtime-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
```

**Apply to Phase 2:** Use temp bundles for create/save/open/autosave. Assert semantic equality of loaded `Draft` values, not byte equality. Add tests for path resolution, inside-bundle relative paths, outside-bundle external/absolute URI preservation, unsupported schema versions, invalid `project.json`, and missing material diagnostics.

### `crates/media_runtime/src/probe.rs` (service, process-I/O)

**Analogs:** `crates/media_runtime/src/discovery.rs`, `crates/testkit/src/lib.rs`

**Discovery and argument-array process pattern** (discovery lines 63-68, 100-117):
```rust
pub fn discover_runtime_config() -> Result<RuntimeConfig, DiscoveryError> {
    let ffmpeg = resolve_binary(BinaryKind::Ffmpeg)?;
    let ffprobe = resolve_binary(BinaryKind::Ffprobe)?;

    Ok(RuntimeConfig { ffmpeg, ffprobe })
}

pub fn probe_binary_version_with_timeout(
    kind: BinaryKind,
    path: PathBuf,
    source: DiscoverySource,
    timeout: Duration,
) -> Result<DiscoveredBinary, DiscoveryError> {
    let args = vec![OsString::from("-version")];
    let output = run_process_with_timeout(&path, &args, timeout).map_err(|error| {
        DiscoveryError::version_probe_failed(
            kind,
            vec![path.clone()],
            None,
            Some(summarize_output(error.to_string().as_bytes())),
        )
    })?;
}
```

**ffprobe metadata command pattern** from `testkit` (lines 153-181):
```rust
let runtime = discover_runtime_config()?;
let executor = DesktopFfmpegExecutor::default();
let args = vec![
    OsString::from("-v"),
    OsString::from("error"),
    OsString::from("-output_format"),
    OsString::from("json"),
    OsString::from("-show_entries"),
    OsString::from("stream=codec_type,width,height,r_frame_rate,duration:format=duration"),
    path.as_os_str().to_owned(),
];
let output = executor
    .run(&runtime.ffprobe.path, &args)
    .map_err(|error| {
        SmokeError::new(format!(
            "failed to launch ffprobe at {}: {error}",
            runtime.ffprobe.path.display()
        ))
    })?;

if !output.status.success() {
    return Err(SmokeError::new(format!(
        "ffprobe metadata probe failed: stdout=`{}` stderr=`{}`",
        bounded_summary(&output.stdout),
        bounded_summary(&output.stderr)
    )));
}

parse_ffprobe_metadata(&output.stdout)
```

**JSON parse/normalization pattern** from `testkit` (lines 270-312):
```rust
fn parse_ffprobe_metadata(bytes: &[u8]) -> SmokeResult<SmokeMetadata> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
        SmokeError::new(format!("failed to parse ffprobe JSON metadata: {error}"))
    })?;
    let streams = value
        .get("streams")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| SmokeError::new("ffprobe JSON metadata did not include streams"))?;
    let video_stream = streams.iter().find(|stream| {
        stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("video")
    });
    let audio_stream = streams.iter().find(|stream| {
        stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("audio")
    });
    let video_stream =
        video_stream.ok_or_else(|| SmokeError::new("ffprobe did not report a video stream"))?;
}
```

**Apply to Phase 2:** Move normalized material probing into `media_runtime` rather than leaving it in `testkit`. Keep process execution through `FfmpegExecutor` and argument vectors. Return normalized metadata only: kind, duration microseconds, dimensions, rational fps, stream flags, sample rate/channel count, and bounded probe failure summaries. Do not persist raw ffprobe JSON in `project.json`.

### `crates/media_runtime/tests/material_probe.rs` (test, process-I/O)

**Analog:** `crates/media_runtime/tests/discovery.rs`

**Env isolation pattern** (lines 11-24):
```rust
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn discovery_runtime_config_prefers_explicit_env_paths_before_path() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("env-before-path");
    let env_ffmpeg = sandbox.bin("env", "ffmpeg", "ffmpeg version env-build\n", "", 0);
    let env_ffprobe = sandbox.bin("env", "ffprobe", "ffprobe version env-build\n", "", 0);

    let _env_ffmpeg = EnvVarGuard::set_path("VE_FFMPEG_PATH", &env_ffmpeg);
    let _env_ffprobe = EnvVarGuard::set_path("VE_FFPROBE_PATH", &env_ffprobe);
    let _path = EnvVarGuard::set_path("PATH", sandbox.dir("path"));
}
```

**Failure assertion pattern** (lines 91-117):
```rust
#[test]
fn discovery_bad_binary_error_uses_bounded_output_summary() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let sandbox = Sandbox::new("bad-binary");
    let long_stderr = "x".repeat(MAX_STDERR_SUMMARY_BYTES + 128);
    let bad_ffmpeg = sandbox.bin("env", "ffmpeg", "not really ffmpeg\n", &long_stderr, 23);
    let good_ffprobe = sandbox.bin("env", "ffprobe", "ffprobe version env-build\n", "", 0);

    let error = discover_runtime_config().expect_err("bad ffmpeg should fail version probe");

    assert_eq!(error.kind, DiscoveryErrorKind::VersionProbeFailed);
    assert_eq!(error.binary, BinaryKind::Ffmpeg);
    assert_eq!(error.checked_paths, vec![bad_ffmpeg]);
    assert_eq!(error.stdout_summary.as_deref(), Some("not really ffmpeg"));
    assert!(
        error.stderr_summary.as_ref().unwrap().len() <= MAX_STDERR_SUMMARY_BYTES,
        "stderr summary should be bounded"
    );
}
```

**Apply to Phase 2:** Test video/image/audio probes plus corrupt/missing/probe-failed inputs. Keep env-var mutation serialized. Use fake executors or generated testkit media as appropriate, but preserve bounded output summaries and classified error kinds.

### `crates/testkit/src/media.rs` and `crates/testkit/src/project.rs` (utility, file-I/O/process-I/O)

**Analog:** `crates/testkit/src/lib.rs`

**Generated media ownership pattern** (lines 63-75):
```rust
#[derive(Debug)]
pub struct TinyLavfiMedia {
    _temp_dir: tempfile::TempDir,
    output_path: PathBuf,
}

impl TinyLavfiMedia {
    /// Path to the generated MP4 output. The file is removed when this value is dropped.
    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
}
```

**Media generation pattern** (lines 108-131):
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

    Ok(TinyLavfiMedia {
        _temp_dir: temp_dir,
        output_path,
    })
}
```

**FFmpeg argument-vector pattern** (lines 222-267):
```rust
fn run_ffmpeg_generate(
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
) -> SmokeResult<()> {
    let args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-y"),
        OsString::from("-f"),
        OsString::from("lavfi"),
        OsString::from("-i"),
        OsString::from(format!(
            "testsrc2=size={TINY_WIDTH}x{TINY_HEIGHT}:rate={TINY_FPS}:duration={TINY_DURATION_SECONDS}"
        )),
        output_path.as_os_str().to_owned(),
    ];

    let output = executor.run(&runtime.ffmpeg.path, &args).map_err(|error| {
        SmokeError::new(format!(
            "failed to launch ffmpeg at {}: {error}",
            runtime.ffmpeg.path.display()
        ))
    })?;
}
```

**Apply to Phase 2:** Extend with deterministic image and audio-only helpers. Keep temporary directory lifetime owned by returned structs. Add project fixture helpers that create `.veproj/project.json` bundles for `project_store` tests without leaking filesystem behavior into `draft_model`.

### `crates/bindings_node/src/lib.rs` (controller, request-response)

**Analog:** `crates/bindings_node/src/lib.rs`

**NAPI command entrypoint pattern** (lines 26-59):
```rust
#[napi]
pub fn execute_command(command: serde_json::Value) -> Result<serde_json::Value> {
    let command_name = raw_command_name(&command);

    if let Some(name) = command_name.as_deref() {
        if name != "ping" && name != "version" && name != "probeMediaRuntime" {
            return to_js_value(error_envelope(
                CommandErrorKind::UnsupportedCommand,
                format!("Unsupported Phase 1 command: {name}"),
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
        CommandName::ProbeMediaRuntime => match discover_runtime_config() {
            Ok(config) => to_js_value(ok_envelope(config)),
            Err(error) => to_js_value(runtime_discovery_error_envelope(error)),
        },
    }
}
```

**Envelope helpers pattern** (lines 72-95):
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
```

**Apply to Phase 2:** Expose material/project commands only after Rust model/store/runtime behavior exists. Preserve typed command envelopes and structured error envelopes. Bindings should call Rust services; they should not construct FFmpeg commands or mutate project JSON directly.

### `apps/desktop-electron/src/renderer/App.tsx` (component, request-response)

**Analog:** `apps/desktop-electron/src/renderer/App.tsx`

**Generated contract consumption pattern** (lines 1-13):
```typescript
import { useEffect, useMemo, useState } from "react";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type { CommandResultEnvelope } from "../generated/CommandResultEnvelope";

type VideoEditorCoreApi = {
  ping: () => Promise<CommandResultEnvelope<PingResponse>>;
  version: () => Promise<CommandResultEnvelope<VersionResponse>>;
  executeCommand: (command: CommandEnvelope) => Promise<CommandResultEnvelope<unknown>>;
};
```

**Command-only smoke pattern** (lines 32-49):
```typescript
const smokeCommand = useMemo<CommandEnvelope>(
  () => ({
    command: "ping",
    payload: { kind: "ping" },
    requestId: "renderer-smoke-ping"
  }),
  []
);

const [ping, version, command] = await Promise.all([
  window.videoEditorCore.ping(),
  window.videoEditorCore.version(),
  window.videoEditorCore.executeCommand(smokeCommand)
]);
```

**Material-bin placeholder pattern** (lines 104-107):
```typescript
<section className="media-bin" aria-label="Material bin">
  <h2>Materials</h2>
  <div className="material-row">Draft media</div>
</section>
```

**Apply to Phase 2:** If Electron smoke is included, consume generated draft/material types and call `executeCommand`; keep it smoke-level. Do not add renderer-side FFmpeg calls, direct project JSON mutation, or rich Phase 4 material-bin behavior.

## Shared Patterns

### Pure Semantic Crate Boundary

**Source:** `docs/runtime-boundaries.md` lines 23-34
**Apply to:** `draft_model`, `draft_commands`, `engine_core`

```markdown
`draft_model`, `draft_commands`, and `engine_core` must remain pure semantic
crates. They may define draft/material/track/segment/time concepts and editing
semantics, but they must not depend on:

- `media_runtime::FfmpegExecutor`
- `project_store::PlatformFileSystem`
- `preview_service::PreviewRenderer`
- OS process execution details
- Electron, mobile, server, or filesystem runtime abstractions
```

### Runtime Trait Placement

**Source:** `docs/runtime-boundaries.md` lines 8-21
**Apply to:** `project_store`, `media_runtime`, `preview_service`

```markdown
Platform traits live at the consuming service boundary:

- `media_runtime::FfmpegExecutor` owns the FFmpeg and ffprobe process execution
  boundary.
- `project_store::PlatformFileSystem` owns filesystem access for `.veproj`
  project bundle persistence.
- `preview_service::PreviewRenderer` reserves the future preview rendering
  boundary for frames, segments, thumbnails, waveform cache, and invalidation.
```

### Bounded Process Execution

**Source:** `crates/media_runtime/src/process.rs` lines 11-43
**Apply to:** material probe and testkit media generation

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
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "{} timed out after {} ms",
                    binary.display(),
                    timeout.as_millis()
                ),
            ));
        }

        thread::sleep(Duration::from_millis(10));
    }
}
```

### Desktop Executor Injection

**Source:** `crates/media_runtime_desktop/src/lib.rs` lines 35-51
**Apply to:** desktop/testkit runtime calls, not pure model crates

```rust
impl FfmpegExecutor for DesktopFfmpegExecutor {
    fn executor_name(&self) -> &'static str {
        "desktop-ffmpeg-executor"
    }

    fn can_execute(&self, binary: &Path) -> bool {
        binary.is_file()
    }

    fn run_version_probe(&self, binary: &Path) -> std::io::Result<Output> {
        let args = vec![OsString::from("-version")];
        run_process_with_timeout(binary, &args, self.timeout)
    }

    fn run(&self, binary: &Path, args: &[OsString]) -> std::io::Result<Output> {
        run_process_with_timeout(binary, args, self.timeout)
    }
}
```

### Command Result Envelope

**Source:** `crates/draft_model/src/lib.rs` lines 100-135
**Apply to:** bindings and command/API results

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandResultEnvelope<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<CommandError>,
    pub events: Vec<CommandEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandError {
    pub kind: CommandErrorKind,
    pub message: String,
    pub command: Option<String>,
}
```

### Generated Fixture Gate

**Source:** `crates/draft_model/tests/schema_exports.rs` lines 127-160
**Apply to:** draft fixtures and generated schema/TS drift

```rust
let expected = positive.union(&negative).copied().collect::<BTreeSet<_>>();
let actual = fixture_names
    .iter()
    .map(String::as_str)
    .collect::<BTreeSet<_>>();
assert_eq!(
    actual, expected,
    "every draft JSON fixture must be explicitly classified"
);

for fixture_name in positive {
    let value = read_fixture(&fixture_dir, fixture_name);
    serde_json::from_value::<CommandEnvelope>(value.clone())
        .expect("positive fixture should deserialize through Rust model");
    schema
        .validate(&value)
        .expect("positive fixture should validate against JSON Schema");
}
```

## No Analog Found

No Phase 2 file family lacks an analog. Some exact implementation names are new (`draft.rs`, `material.rs`, `bundle.rs`, `probe.rs`), but their patterns are covered by existing Phase 1 model, store, runtime, binding, and testkit files.

## Metadata

**Analog search scope:** repo files excluding `reference/`; focused on `crates/draft_model`, `crates/project_store`, `crates/media_runtime`, `crates/media_runtime_desktop`, `crates/testkit`, `crates/bindings_node`, `apps/desktop-electron`, `fixtures/draft`, `schemas`, and `docs`.
**Files scanned:** 43 source/config/test files from `rg --files -g '!reference/**'`, plus required planning docs.
**Pattern extraction date:** 2026-06-17

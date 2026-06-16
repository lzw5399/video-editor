# Phase 1: Foundation And Golden Harness - Pattern Map

**Mapped:** 2026-06-17
**Files analyzed:** 42
**Analogs found:** 0 / 42

This is a brand-new project. No application source files exist yet, so there are no code analogs to copy from. Pattern assignments below use planning and research artifacts as reference-only sources. Do not copy code, XML, assets, presets, or implementation details from `reference/kdenlive` or `reference/mlt`; they are conceptual references only.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `Cargo.toml` | config | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `rust-toolchain.toml` | config | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `justfile` | config | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `package.json` | config | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `pnpm-workspace.yaml` | config | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `.nvmrc` | config | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `.gitignore` | config | file-I/O | `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md` | reference-only |
| `apps/desktop-electron/package.json` | config | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `apps/desktop-electron/tsconfig.json` | config | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `apps/desktop-electron/vite.config.ts` | config | request-response | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `apps/desktop-electron/playwright.config.ts` | config | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `apps/desktop-electron/src/main/index.ts` | controller | request-response | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `apps/desktop-electron/src/main/nativeBinding.ts` | service | request-response | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `apps/desktop-electron/src/preload/index.ts` | middleware | request-response | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `apps/desktop-electron/src/renderer/main.tsx` | component | event-driven | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `apps/desktop-electron/src/renderer/App.tsx` | component | request-response | `.planning/REQUIREMENTS.md` | reference-only |
| `apps/desktop-electron/src/generated/CommandEnvelope.ts` | model | transform | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `apps/desktop-electron/tests/electron-smoke.spec.ts` | test | request-response | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `crates/draft_model/Cargo.toml` | config | batch | `.planning/research/ARCHITECTURE.md` | reference-only |
| `crates/draft_model/src/lib.rs` | model | transform | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `crates/draft_model/tests/schema_exports.rs` | test | transform | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `crates/draft_commands/src/lib.rs` | service | CRUD | `.planning/research/ARCHITECTURE.md` | reference-only |
| `crates/engine_core/src/lib.rs` | service | transform | `.planning/research/ARCHITECTURE.md` | reference-only |
| `crates/render_graph/src/lib.rs` | model | transform | `.planning/research/ARCHITECTURE.md` | reference-only |
| `crates/ffmpeg_compiler/src/lib.rs` | service | transform | `.planning/research/ARCHITECTURE.md` | reference-only |
| `crates/media_runtime/src/lib.rs` | service | file-I/O | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `crates/media_runtime/src/discovery.rs` | service | file-I/O | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `crates/media_runtime/src/error.rs` | utility | transform | `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md` | reference-only |
| `crates/media_runtime/tests/discovery.rs` | test | file-I/O | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `crates/media_runtime_desktop/src/lib.rs` | service | file-I/O | `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md` | reference-only |
| `crates/preview_service/src/lib.rs` | service | streaming | `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md` | reference-only |
| `crates/project_store/src/lib.rs` | service | file-I/O | `.planning/research/ARCHITECTURE.md` | reference-only |
| `crates/bindings_node/Cargo.toml` | config | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `crates/bindings_node/build.rs` | config | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `crates/bindings_node/src/lib.rs` | controller | request-response | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `crates/bindings_node/tests/binding_smoke.rs` | test | request-response | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `crates/testkit/src/lib.rs` | utility | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `crates/testkit/tests/render_smoke.rs` | test | file-I/O | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `schemas/command.schema.json` | model | transform | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |
| `fixtures/draft/minimal-command.json` | test | transform | `.planning/REQUIREMENTS.md` | reference-only |
| `fixtures/media-generated/.gitkeep` | config | file-I/O | `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md` | reference-only |
| `goldens/README.md` | documentation | batch | `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md` | reference-only |
| `.github/workflows/ci.yml` | config | batch | `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` | reference-only |

## Pattern Assignments

### Workspace, Toolchain, and Command Files

**Applies to:** `Cargo.toml`, `rust-toolchain.toml`, `justfile`, `package.json`, `pnpm-workspace.yaml`, `.nvmrc`, `apps/desktop-electron/package.json`, root CI config

**Analog:** No source-code analog found. Use `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` and `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md`.

**Repository structure pattern** (`01-RESEARCH.md` lines 221-249):
```text
apps/
  desktop-electron/
    src/main/          # Electron main process, IPC handlers, native binding loader
    src/preload/       # contextBridge API; no raw ipcRenderer exposure
    src/renderer/      # minimal React/Vite shell and smoke UI
    src/generated/     # TypeScript types exported from Rust contracts
crates/
  draft_model/         # serde/schemars/ts-rs source-of-truth contract types
  draft_commands/      # compile-safe shell; semantic commands later
  engine_core/         # compile-safe shell; normalization later
  render_graph/        # compile-safe shell; render intents later
  ffmpeg_compiler/     # compile-safe shell; no UI dependency
  media_runtime/       # FfmpegExecutor trait, discovery, errors
  media_runtime_desktop/ # desktop process execution implementation
  preview_service/     # PreviewRenderer boundary stub/README
  project_store/       # PlatformFileSystem boundary shell
  bindings_node/       # napi-rs Electron-facing addon
  testkit/             # fixture generation, golden helpers, render smoke helpers
schemas/
fixtures/
  draft/
  media-generated/     # generated at test time; gitignored
goldens/
docs/
tools/
```

**Command gate pattern** (`01-RESEARCH.md` lines 404-422):
```make
dev:
  pnpm --filter @video-editor/desktop dev

build:
  pnpm install --frozen-lockfile
  cargo build --workspace --locked
  pnpm --filter @video-editor/bindings-node build
  pnpm --filter @video-editor/desktop build

test:
  pnpm install --frozen-lockfile
  cargo test --workspace --locked
  pnpm --filter @video-editor/bindings-node test
  pnpm --filter @video-editor/desktop test
  cargo test -p testkit render_smoke -- --nocapture
```

**Version/dependency pattern** (`01-RESEARCH.md` lines 107-132):
```text
Rust toolchain: 1.95.0 local; pin via rust-toolchain.toml
Cargo workspace: Cargo 1.95.0 local
just: 1.52.0 on crates.io; not installed locally
pnpm + Corepack: pnpm 10.32.1, Corepack 0.34.5 local
Electron: 42.4.1
React: 19.2.7
TypeScript: 6.0.3
Vite: 8.0.16
@napi-rs/cli: 3.7.2
@playwright/test: 1.61.0
```

**Validation pattern** (`01-RESEARCH.md` lines 474-491):
```text
Framework: Rust cargo test, Playwright Electron via @playwright/test 1.61.0, pnpm script tests.
Quick run command: just test
FOUND-01 -> just build
FOUND-02 -> pnpm --filter @video-editor/desktop test and cargo test -p bindings_node
FOUND-03 -> cargo test -p media_runtime discovery
FOUND-04 -> cargo test -p testkit render_smoke
TEST-01 -> cargo test -p draft_model schema
```

### Electron Main, Preload, Renderer, and Smoke Test

**Applies to:** `apps/desktop-electron/src/main/index.ts`, `apps/desktop-electron/src/main/nativeBinding.ts`, `apps/desktop-electron/src/preload/index.ts`, `apps/desktop-electron/src/renderer/*`, `apps/desktop-electron/tests/electron-smoke.spec.ts`

**Analog:** No source-code analog found. Use `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md`.

**IPC surface pattern** (`01-RESEARCH.md` lines 272-287):
```ts
contextBridge.exposeInMainWorld("videoEditorCore", {
  executeCommand: (command: CommandEnvelope) =>
    ipcRenderer.invoke("core:executeCommand", command),
  ping: () => ipcRenderer.invoke("core:ping"),
  version: () => ipcRenderer.invoke("core:version"),
});
```

**Main handler pattern** (`01-RESEARCH.md` lines 395-401):
```ts
ipcMain.handle("core:executeCommand", async (_event, command: CommandEnvelope) => {
  return nativeBinding.executeCommand(command);
});
```

**Boundary rule** (`AGENTS.md` lines 15-20):
```text
UI emits commands; Rust core owns project and timeline semantics. No UI code may directly construct FFmpeg commands.
Project language and code should follow Jianying concepts.
Render Graph isolates editing semantics from FFmpeg. FFmpeg Runtime executes jobs and reports progress/errors only.
Kdenlive and MLT are conceptual references only.
```

**Electron smoke requirement** (`01-RESEARCH.md` lines 349-353):
```text
In Phase 1, load the binding from Electron main process and add a Playwright Electron smoke that calls ping.
Warning sign: pnpm test passes for a Node script but Electron smoke fails with native module load errors.
```

### Rust Binding and Contract Model

**Applies to:** `crates/draft_model/src/lib.rs`, `crates/draft_model/tests/schema_exports.rs`, `crates/bindings_node/src/lib.rs`, `crates/bindings_node/build.rs`, `apps/desktop-electron/src/generated/CommandEnvelope.ts`, `schemas/command.schema.json`

**Analog:** No source-code analog found. Use `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md`.

**Envelope type pattern** (`01-RESEARCH.md` lines 252-269):
```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct CommandResultEnvelope<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<CommandError>,
    pub events: Vec<CommandEvent>,
}
```

**Schema/type export test pattern** (`01-RESEARCH.md` lines 381-391):
```rust
#[test]
fn export_command_contracts() {
    let schema = schemars::schema_for!(CommandEnvelope);
    let json = serde_json::to_string_pretty(&schema).unwrap();
    std::fs::write("../../schemas/command.schema.json", json).unwrap();

    CommandEnvelope::export_to("../../apps/desktop-electron/src/generated/CommandEnvelope.ts").unwrap();
}
```

**Contract decisions** (`01-CONTEXT.md` lines 22-26):
```text
Phase 1 binding scope is ping/version plus execute_command(command) -> ok/error/events.
Rust serde types are the source of truth for binding contracts.
Generate JSON Schema and TypeScript types from Rust-owned types.
All binding calls should return a standardized ok/error/events envelope.
```

**Drift prevention pattern** (`01-RESEARCH.md` lines 355-358 and 450-453):
```text
Generated schemas/*.json and src/generated/*.ts must not drift from Rust contract types.
Add a schema/type generation command and a git diff --exit-code or content comparison gate in just test.
Recommendation: commit generated schema/TS files and make just test fail if regeneration changes them.
```

### Rust Crate Shells and Layer Boundaries

**Applies to:** `crates/draft_commands/src/lib.rs`, `crates/engine_core/src/lib.rs`, `crates/render_graph/src/lib.rs`, `crates/ffmpeg_compiler/src/lib.rs`, `crates/project_store/src/lib.rs`, `crates/preview_service/src/lib.rs`

**Analog:** No source-code analog found. Use `.planning/research/ARCHITECTURE.md` and `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md`.

**Semantic spine pattern** (`.planning/research/ARCHITECTURE.md` lines 5-17):
```text
draft/project.json
  -> command
  -> normalized draft
  -> resolved frame state
  -> render graph
  -> FFmpeg job
  -> preview/export

Every layer should use Jianying-aligned concepts where possible.
```

**Layer responsibility pattern** (`.planning/research/ARCHITECTURE.md` lines 65-79):
```text
Electron shell owns windows, menu, file dialogs, permissions, packaging.
Renderer UI owns layout, drag gestures, selection, panels, timeline zoom.
Node binding owns stable IPC/API between UI and Rust.
draft_model owns draft/material/track/segment schema, time, migrations.
draft_commands owns add/move/split/trim/delete, undo/redo, snapping.
engine_core owns normalization, time mapping, track stacking, frame state.
render_graph owns typed render plan.
ffmpeg_compiler owns inputs, filter scripts, subtitles, encode args.
media_runtime owns ffprobe/ffmpeg execution, progress, cancel, errors.
preview_service owns preview frames/segments, thumbnails, waveform cache.
```

**Cross-platform boundary decisions** (`01-CONTEXT.md` lines 40-45):
```text
Pure semantic core crates must not depend on platform traits.
Platform differences are abstracted only at service boundaries.
Add FfmpegExecutor, PlatformFileSystem, and PreviewRenderer boundaries.
Put traits at consuming crate boundary: media_runtime, project_store, preview_service.
Do not create a generic all-purpose platform crate.
```

**Boundary pitfall** (`01-RESEARCH.md` lines 373-377):
```text
draft_model, draft_commands, or engine_core must not depend on filesystem/FFmpeg/platform traits.
Keep service traits in media_runtime, project_store, and preview_service.
Warning signs: a pure semantic crate imports std::process, which, Electron, filesystem trait objects, or FFmpeg names.
```

### FFmpeg Runtime and Desktop Execution

**Applies to:** `crates/media_runtime/src/lib.rs`, `crates/media_runtime/src/discovery.rs`, `crates/media_runtime/src/error.rs`, `crates/media_runtime/tests/discovery.rs`, `crates/media_runtime_desktop/src/lib.rs`

**Analog:** No source-code analog found. Use `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md` and `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md`.

**Discovery order pattern** (`01-RESEARCH.md` lines 289-305):
```rust
fn resolve_binary(env_name: &str, fallback_name: &str) -> Result<Utf8PathBuf, DiscoveryError> {
    if let Some(explicit) = std::env::var_os(env_name) {
        return validate_candidate(explicit, env_name);
    }
    which::which(fallback_name)
        .map_err(|_| DiscoveryError::missing_binary(fallback_name, vec![env_name, "PATH"]))
        .and_then(|path| validate_candidate(path, "PATH"))
}
```

**Error classification pattern** (`01-CONTEXT.md` lines 28-32):
```text
Support discovery through PATH and explicit environment variables: VE_FFMPEG_PATH and VE_FFPROBE_PATH.
Discovery must run version probes for both binaries.
Structured errors should include MissingBinary, VersionProbeFailed, or UnsupportedVersion, checked paths, remediation guidance, and bounded stderr summary.
Do not download, install, bundle, or redistribute FFmpeg in Phase 1.
```

**Runtime responsibility pattern** (`01-RESEARCH.md` lines 94-100):
```text
FFmpeg discovery: media_runtime owns binary lookup, version probing, and structured failures; UI only displays the error envelope.
Tiny render smoke: test harness drives FFmpeg/ffprobe through runtime abstractions and validates metadata.
Golden fixture validation: Rust schema/model tests own fixture discovery and validation.
```

**Security pattern** (`01-RESEARCH.md` lines 521-527):
```text
Explicit env var paths must be probed with -version.
Bound stderr summary in VersionProbeFailed and render smoke failures.
Use std::process::Command args, not shell-concatenated command strings.
UI never constructs FFmpeg args.
```

### Testkit, Fixtures, Goldens, and Render Smoke

**Applies to:** `crates/testkit/src/lib.rs`, `crates/testkit/tests/render_smoke.rs`, `fixtures/draft/minimal-command.json`, `fixtures/media-generated/.gitkeep`, `goldens/README.md`

**Analog:** No source-code analog found. Use `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md`, `.planning/REQUIREMENTS.md`, and `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md`.

**Golden fixture decision pattern** (`01-CONTEXT.md` lines 34-38):
```text
Create fixture/golden structure and include schema fixtures, tiny media generation, and a tiny render smoke test.
Tiny media fixtures are generated during tests with FFmpeg lavfi sources such as testsrc2 and sine.
Do not commit binary media files for this gate.
just test and CI fail when FFmpeg/ffprobe are missing.
Render smoke asserts output file existence and ffprobe metadata: approximate duration, fps, resolution, and video/audio stream presence.
Do not do pixel/hash comparison in Phase 1.
```

**Render smoke command shape** (`01-RESEARCH.md` lines 313-323):
```bash
ffmpeg -hide_banner -y \
  -f lavfi -i "testsrc2=size=160x90:rate=10:duration=1" \
  -f lavfi -i "sine=frequency=440:duration=1" \
  -c:v libx264 -pix_fmt yuv420p -c:a aac "$TMPDIR/tiny-smoke.mp4"

ffprobe -v error -output_format json \
  -show_entries stream=codec_type,width,height,r_frame_rate,duration:format=duration \
  "$TMPDIR/tiny-smoke.mp4"
```

**Render smoke pitfall** (`01-RESEARCH.md` lines 367-371):
```text
Pixel/hash assertions become flaky before render semantics exist.
Assert only file existence and ffprobe metadata in Phase 1.
Warning signs: test fixture includes committed binary media or image hash baselines.
```

**Requirement scope** (`.planning/REQUIREMENTS.md` lines 8-13 and 70-73):
```text
FOUND-01: Developer can build a Rust workspace and Electron desktop shell from a clean checkout.
FOUND-02: Electron can call the Rust core through a typed binding/API boundary.
FOUND-03: The app can discover configured FFmpeg and ffprobe binaries and report actionable errors when unavailable.
FOUND-04: The repository includes deterministic fixtures and golden test harnesses before feature work depends on media rendering.
TEST-01: Schema and model tests validate every golden draft fixture.
```

## Shared Patterns

### No Source-Code Analogs
**Source:** Repository scan excluding `reference/**`
**Apply to:** All Phase 1 implementation files
```text
Existing non-reference files are AGENTS.md, AI_Video_Editing_Single_Engine_Guideline.md, and .planning artifacts.
No apps/, crates/, schemas/, fixtures/, goldens/, tools/, or package/build source files exist yet.
Use planning/reference artifacts for intent only; establish the first code patterns in Phase 1.
```

### Terminology
**Source:** `AGENTS.md` lines 15-20 and `.planning/research/ARCHITECTURE.md` lines 17-18
**Apply to:** Rust types, IPC commands, schema, generated TypeScript, tests, docs
```text
Prefer draft/material/track/segment/keyframe/filter/transition-style terms.
Avoid alternate internal vocabulary such as Asset/Clip when Material/Segment are the intended concepts.
Use integer microseconds, frame indices, or rational frame rates for core time math.
```

### Contract and Validation
**Source:** `01-RESEARCH.md` lines 121-130 and 515-516
**Apply to:** `draft_model`, `bindings_node`, schema generation, fixtures, tests
```text
Use serde/serde_json on Rust-owned command/result/schema fixture types.
Use schemars to generate JSON Schema.
Use ts-rs to generate TypeScript bindings.
Use thiserror for structured Rust error enums.
Use jsonschema for runtime JSON Schema validation in tests.
Use serde deny_unknown_fields, JSON Schema validation, and typed command envelope.
```

### FFmpeg Process Safety
**Source:** `01-RESEARCH.md` lines 523-527
**Apply to:** `media_runtime`, `media_runtime_desktop`, `testkit`
```text
Expose only narrow contextBridge methods; never expose raw ipcRenderer.
Probe explicit FFmpeg/ffprobe env var paths with -version.
Bound stderr summaries.
Use std::process::Command args, not shell-concatenated command strings.
UI never constructs FFmpeg args.
```

### Out-of-Scope Guardrails
**Source:** `01-CONTEXT.md` lines 98-104
**Apply to:** All Phase 1 plans
```text
Do not bundle or redistribute FFmpeg.
Do not implement HardwareEncoder.
Do not implement mobile FFmpeg backends, static library loading, JNI loading, or mobile sandbox file implementations.
Do not implement real draft editing semantics, rich UI, export presets, or packaged runtime management.
```

## No Analog Found

All Phase 1 files have no close source-code analog in the codebase.

| File Group | Role | Data Flow | Reason |
|------------|------|-----------|--------|
| Rust workspace and crate shells | config/service/model | batch/transform/request-response/file-I/O | No `Cargo.toml`, `crates/`, or Rust source exists yet. |
| Electron desktop app | controller/component/middleware | request-response/event-driven | No `apps/desktop-electron`, Node package, Electron main/preload, or renderer source exists yet. |
| Node-API binding | controller/config | request-response/batch | No native binding crate or generated binding files exist yet. |
| FFmpeg runtime | service/utility/test | file-I/O | No runtime, process execution, discovery, or error modules exist yet. |
| Fixtures/goldens/testkit | utility/test/model | batch/file-I/O/transform | No `fixtures/`, `goldens/`, `schemas/`, or test harness source exists yet. |
| CI and command gates | config | batch | No `justfile`, GitHub Actions workflow, package scripts, or workspace manifests exist yet. |

## Metadata

**Analog search scope:** Repository root excluding `reference/**`, `node_modules/**`, and `target/**`; project-local `.codex/skills` and `.agents/skills` were checked and no project skills were found.
**Files scanned:** 14 non-reference project/planning files plus phase context/research.
**Pattern extraction date:** 2026-06-17
**Primary sources:** `AGENTS.md`, `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md`, `.planning/phases/01-foundation-and-golden-harness/01-RESEARCH.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/research/ARCHITECTURE.md`

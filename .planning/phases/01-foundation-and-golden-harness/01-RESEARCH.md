# Phase 1: Foundation And Golden Harness - Research

**Researched:** 2026-06-17
**Domain:** Rust/Electron monorepo foundation, Node-API binding, FFmpeg discovery, golden test harness
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
## Implementation Decisions

### Engineering Scaffold And Commands
- **D-01:** Use `just dev`, `just build`, and `just test` as the unified local and CI entrypoints. The `justfile` should wrap `pnpm`, `cargo`, Node-API binding build steps, schema/fixture checks, and render smoke gates.
- **D-02:** Use `pnpm workspace` for Node/Electron packages and pin the package manager with Corepack.
- **D-03:** Lay out the full target repository structure in Phase 1: `apps/desktop-electron`, planned Rust crates, `schemas/`, `fixtures/`, `goldens/`, `docs/`, and `tools/`. Unimplemented crates should be compile-safe shells with README/boundary notes rather than missing directories.
- **D-04:** Pin key toolchain versions in Phase 1, including `rust-toolchain.toml`, `packageManager`/Corepack, Node version guidance, and version policy for Electron, Playwright, and napi-rs or equivalent binding tooling.

### Electron Rust Binding Boundary
- **D-05:** Phase 1 binding scope is `ping`/`version` plus a typed command envelope such as `execute_command(command) -> ok/error/events`. Real editing commands are out of scope for this phase.
- **D-06:** Rust serde types are the source of truth for binding contracts. Generate JSON Schema and TypeScript types from Rust-owned types, and add contract tests to prevent IPC/schema drift.
- **D-07:** Code, API, schema, and tests use stable English Jianying concept names, while UI may display Chinese. Avoid parallel internal vocabulary such as `Asset`/`Clip` when `Material`/`Segment` are the intended concepts.
- **D-08:** All binding calls should return a standardized `ok/error/events` envelope from the start. This is the foundation for later command results, undo/redo events, timeline updates, preview progress, and export progress.

### FFmpeg And ffprobe Discovery
- **D-09:** Phase 1 supports FFmpeg/ffprobe discovery through `PATH` and explicit environment variables: `VE_FFMPEG_PATH` and `VE_FFPROBE_PATH`.
- **D-10:** Discovery must run version probes for both binaries and return structured failures when discovery or probing fails.
- **D-11:** Structured errors should include a stable error kind such as `MissingBinary`, `VersionProbeFailed`, or `UnsupportedVersion`, checked paths, remediation guidance, and a bounded stderr summary.
- **D-12:** Phase 1 does not download, install, bundle, or redistribute FFmpeg. It also does not handle FFmpeg license posture because no binary distribution happens in this phase. If a later packaged app distributes FFmpeg binaries, packaging/release work must revisit distribution and notices.

### Golden Fixtures And Test Gates
- **D-13:** Phase 1 creates the fixture/golden structure and includes schema fixtures, tiny media generation, and a tiny render smoke test. Full draft goldens wait until Phase 2 draft schema decisions.
- **D-14:** Tiny media fixtures are generated during tests with FFmpeg `lavfi` sources such as `testsrc2` and `sine`. Do not commit binary media files for this gate.
- **D-15:** `just test` and CI fail when FFmpeg/ffprobe are missing. Render smoke is a required Phase 1 gate, not a skipped optional test.
- **D-16:** Render smoke asserts output file existence and ffprobe metadata: approximate duration, fps, resolution, and video/audio stream presence. Do not do pixel/hash comparison in Phase 1.

### Cross-Platform Abstraction Boundaries
- **D-17:** Pure semantic core crates must not depend on platform traits. `draft_model`, `draft_commands`, and `engine_core` stay pure data/semantic layers.
- **D-18:** Platform differences are abstracted only at service boundaries: App Shell, `project_store`, `media_runtime`, `preview_service`, and the future preview renderer boundary.
- **D-19:** Phase 1 should add boundary interfaces for `FfmpegExecutor`, `PlatformFileSystem`, and `PreviewRenderer`. Only desktop FFmpeg/file-system behavior is implemented now; `PreviewRenderer` is a future boundary stub/README.
- **D-20:** Put cross-platform traits at the consuming crate boundary: `media_runtime::FfmpegExecutor`, `project_store::PlatformFileSystem`, and `preview_service::PreviewRenderer`. Do not create a generic all-purpose `platform` crate.
- **D-21:** `HardwareEncoder` is documented only in Phase 1, not implemented. It belongs with the real preview/export pipeline once export presets and encode paths exist.

### Agent Discretion
- The planner may choose exact dependency versions, file names, and smoke-test implementation details if they preserve the decisions above and keep `just test` as the required health gate.

### the agent's Discretion
- The planner may choose exact dependency versions, file names, and smoke-test implementation details if they preserve the decisions above and keep `just test` as the required health gate.

### Deferred Ideas (OUT OF SCOPE)
- App-bundled FFmpeg runtime management belongs to a later packaging/release phase.
- FFmpeg distribution and notices are deferred until the project actually distributes FFmpeg binaries.
- `HardwareEncoder` implementation, including NVENC/QSV/VideoToolbox/MediaCodec probing, is deferred until real preview/export work.
- Mobile iOS/Android FFmpeg backends, static library loading, JNI loading, and mobile sandbox file implementations are deferred beyond Phase 1.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FOUND-01 | Developer can build a Rust workspace and Electron desktop shell from a clean checkout. | Use a virtual Cargo workspace, pnpm workspace, Corepack-pinned package manager, `just build`, and CI parity gates. [VERIFIED: CONTEXT.md, REQUIREMENTS.md, Cargo docs, pnpm docs, just docs] |
| FOUND-02 | Electron can call the Rust core through a typed binding/API boundary. | Use napi-rs for a Node-API addon, Electron main/preload IPC, and Rust-owned serde/TS/schema contract generation. [VERIFIED: CONTEXT.md, NAPI-RS docs, Electron IPC docs, schemars docs, ts-rs docs] |
| FOUND-03 | The app can discover configured FFmpeg and ffprobe binaries and report actionable errors when unavailable. | Implement `media_runtime::FfmpegExecutor` discovery via `VE_FFMPEG_PATH`, `VE_FFPROBE_PATH`, then `PATH`, with version probes and structured errors. [VERIFIED: CONTEXT.md, local ffmpeg/ffprobe commands] |
| FOUND-04 | The repository includes deterministic fixtures and golden test harnesses before feature work depends on media rendering. | Generate tiny media from FFmpeg `lavfi` (`testsrc2`, `sine`), validate schema fixtures, and smoke-check output with ffprobe JSON. [VERIFIED: CONTEXT.md, FFmpeg filter docs, ffprobe docs] |
| TEST-01 | Schema and model tests validate every golden draft fixture. | In Phase 1 this means scaffold fixture discovery, schema generation, and one placeholder/minimal golden fixture contract; full draft goldens wait for Phase 2. [VERIFIED: CONTEXT.md, REQUIREMENTS.md] |
</phase_requirements>

## Project Constraints (from AGENTS.md)

- UI emits commands; Rust core owns project and timeline semantics; UI code must not directly construct FFmpeg commands. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is canonical; render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, and preview caches are derived artifacts. [VERIFIED: AGENTS.md]
- Use Jianying-aligned terminology across product language, desktop code, Rust types, IPC commands, docs, schema, and tests. [VERIFIED: AGENTS.md]
- Core time math must use integer microseconds, frame indices, or rational frame rates; avoid naked persisted floating-point time. [VERIFIED: AGENTS.md]
- Render Graph isolates editing semantics from FFmpeg; FFmpeg Runtime executes jobs and reports progress/errors only. [VERIFIED: AGENTS.md]
- Kdenlive and MLT are conceptual references only; do not copy GPL code, assets, XML definitions, presets, or UI implementation. [VERIFIED: AGENTS.md]
- Each roadmap phase must define executable gates before implementation is complete. [VERIFIED: AGENTS.md]
- FFmpeg distribution license posture is not handled in Phase 1 because no binaries are distributed, but later distribution must review LGPL/GPL/nonfree options and notices. [VERIFIED: AGENTS.md, CONTEXT.md]

## Summary

Phase 1 should establish a buildable monorepo, not product behavior. The standard foundation is a root `justfile`, a virtual Cargo workspace with compile-safe crate shells, a pnpm workspace for Electron/React/TypeScript packages, and a Node-API binding crate built with napi-rs. [VERIFIED: CONTEXT.md, Cargo docs, pnpm docs, NAPI-RS docs]

The first binding should be intentionally boring: `ping`, `version`, and `execute_command(CommandEnvelope) -> CommandResultEnvelope`, with Rust serde types as the source of truth and generated JSON Schema plus TypeScript bindings checked into predictable generated paths. [VERIFIED: CONTEXT.md, Electron IPC docs, schemars docs, ts-rs docs]

FFmpeg work in this phase is discovery and a tiny deterministic smoke gate only. The runtime should discover explicit env vars before `PATH`, probe versions, classify failures, generate tiny test media with FFmpeg lavfi sources, and validate the rendered output using ffprobe JSON metadata. [VERIFIED: CONTEXT.md, FFmpeg docs, ffprobe docs]

**Primary recommendation:** Plan Phase 1 around `just test` as the non-negotiable gate that runs `pnpm install --frozen-lockfile`, Rust checks/tests, napi binding build/test, schema/type generation drift checks, FFmpeg discovery tests, and one tiny render smoke. [VERIFIED: CONTEXT.md, just docs, pnpm docs]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Build orchestration | Tooling / CI | Rust + Node package managers | `just` owns developer-facing entrypoints while delegating to `cargo` and `pnpm`. [VERIFIED: CONTEXT.md, just docs] |
| Electron shell boot | Frontend Server / Desktop Shell | Browser / Renderer | Electron main process owns windows, preload, IPC registration, and binding loading; renderer stays UI-only. [CITED: https://www.electronjs.org/docs/latest/] |
| Rust binding contract | API / Backend | Frontend Server / Desktop Shell | Rust exposes typed functions through Node-API; Electron calls the binding through a narrow IPC/preload API. [CITED: https://napi.rs/docs/introduction/getting-started, https://www.electronjs.org/docs/latest/tutorial/ipc] |
| Draft command envelope | API / Backend | Browser / Client | Rust-owned serde types define command/result envelopes; browser receives typed results but does not mutate semantic state. [VERIFIED: CONTEXT.md, AGENTS.md] |
| FFmpeg discovery | API / Backend | OS / Runtime | `media_runtime` owns binary lookup, version probing, and structured failures; UI only displays the error envelope. [VERIFIED: CONTEXT.md] |
| Tiny render smoke | Tooling / CI | API / Backend | Test harness drives FFmpeg/ffprobe through runtime abstractions and validates metadata. [VERIFIED: CONTEXT.md, FFmpeg docs] |
| Golden fixture validation | Tooling / CI | API / Backend | Rust schema/model tests own fixture discovery and validation before feature phases depend on goldens. [VERIFIED: REQUIREMENTS.md] |

## Standard Stack

### Core
| Library / Tool | Version | Purpose | Why Standard |
|----------------|---------|---------|--------------|
| Rust toolchain | 1.95.0 local; pin via `rust-toolchain.toml` | Build all Rust crates | Local `rustc`/`cargo` are available and rustup supports checked-in toolchain files. [VERIFIED: local command, CITED: https://rust-lang.github.io/rustup/overrides.html] |
| Cargo workspace | Cargo 1.95.0 local | Rust monorepo and crate shells | Cargo workspaces share lockfile/output and support `members = ["crates/*"]`. [VERIFIED: local command, CITED: https://doc.rust-lang.org/cargo/reference/workspaces.html] |
| `just` | 1.52.0 on crates.io; not installed locally | Unified `dev`, `build`, `test` entrypoints | `just` stores project-specific recipes in `justfile`; planner must add a prerequisite/install step because `command -v just` failed locally. [VERIFIED: local command, crates.io, CITED: https://just.systems/man/en/] |
| pnpm + Corepack | pnpm 10.32.1, Corepack 0.34.5 local | Node workspace and pinned package manager | pnpm workspaces require `pnpm-workspace.yaml`; Corepack can pin `packageManager`. [VERIFIED: local command, CITED: https://pnpm.io/workspaces, https://pnpm.io/installation] |
| Electron | 42.4.1 | Desktop shell | Electron provides Chromium+Node desktop app runtime and official IPC/preload patterns. [VERIFIED: npm registry, CITED: https://www.electronjs.org/docs/latest/] |
| React | 19.2.7 | Renderer UI shell | Vite supports `react-ts` scaffolding; React keeps the desktop shell ready for Phase 4. [VERIFIED: npm registry, CITED: https://vite.dev/guide/] |
| TypeScript | 6.0.3 | Typed Electron/renderer code | Required for typed generated contract consumption. [VERIFIED: npm registry, CITED: https://www.typescriptlang.org/] |
| Vite | 8.0.16 | Renderer dev/build pipeline | Vite documents React TypeScript templates and production build scripts. [VERIFIED: npm registry, CITED: https://vite.dev/guide/] |
| `@napi-rs/cli` | 3.7.2 | Build Node-API native addon | NAPI-RS recommends starting from `@napi-rs/cli`; `napi build` emits `.node`, JS binding, and `.d.ts`. [VERIFIED: npm registry, CITED: https://napi.rs/docs/introduction/getting-started, https://napi.rs/docs/cli/build] |
| `napi`, `napi-derive`, `napi-build` | 3.9.2 / 3.5.6 / 2.3.2 | Rust Node-API addon implementation | NAPI-RS simple package uses `napi_derive::napi` and requires `build.rs`; crates are published from the NAPI-RS repo. [VERIFIED: crates.io, CITED: https://napi.rs/docs/introduction/simple-package, https://docs.rs/napi/latest/napi/, https://docs.rs/napi-derive/latest/napi_derive/, https://docs.rs/napi-build/latest/napi_build/] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde` / `serde_json` | 1.0.228 / 1.0.150 | Rust contract serialization and JSON IO | Use on Rust-owned command/result/schema fixture types. [VERIFIED: crates.io, CITED: https://docs.rs/serde/latest/serde/, https://docs.rs/serde_json/latest/serde_json/] |
| `schemars` | 1.2.1 | JSON Schema generation | Derive `JsonSchema` for Rust-owned contracts and generate `schemas/*.schema.json`. [VERIFIED: crates.io, CITED: https://docs.rs/schemars/latest/schemars/] |
| `ts-rs` | 12.0.1 | TypeScript binding generation | Derive `TS` and export generated types during tests or tooling. [VERIFIED: crates.io, CITED: https://docs.rs/ts-rs/latest/ts_rs/] |
| `thiserror` | 2.0.18 | Structured Rust error enums | Use for `DiscoveryErrorKind` and binding-safe error classification. [VERIFIED: crates.io, CITED: https://docs.rs/thiserror/latest/thiserror/] |
| `which` | 8.0.4 | Cross-platform PATH lookup | Use after `VE_FFMPEG_PATH` / `VE_FFPROBE_PATH` are absent. [VERIFIED: crates.io, CITED: https://docs.rs/which/latest/which/] |
| `camino` | 1.2.2 | UTF-8 project paths | Use for `.veproj`, fixture, and generated artifact paths where JSON/TypeScript interop expects UTF-8. [VERIFIED: crates.io, CITED: https://docs.rs/camino/latest/camino/] |
| `tempfile` | 3.27.0 | Isolated smoke-test temp dirs | Use for generated media/render outputs so binary artifacts are not committed. [VERIFIED: crates.io, CITED: https://docs.rs/tempfile/latest/tempfile/] |
| `assert_cmd` | 2.2.2 | CLI test assertions | Use for test harness tools such as `tools/render-smoke` or `cargo` subcommands. [VERIFIED: crates.io, CITED: https://docs.rs/assert_cmd/latest/assert_cmd/] |
| `insta` | 1.48.0 | Snapshot/golden assertions | Use for JSON schema and envelope snapshots once generated output stabilizes. [VERIFIED: crates.io, CITED: https://docs.rs/insta/latest/insta/] |
| `jsonschema` | 0.46.5 | Runtime JSON Schema validation in tests | Validate fixtures against generated schema and validate schema documents themselves. [VERIFIED: crates.io, CITED: https://docs.rs/jsonschema/latest/jsonschema/] |
| `@vitejs/plugin-react` | 6.0.2 | Vite React transform/HMR | Use in `apps/desktop-electron/vite.config.ts`. [VERIFIED: npm registry, CITED: https://github.com/vitejs/vite-plugin-react/tree/main/packages/plugin-react] |
| `@playwright/test` | 1.61.0 | Electron smoke/e2e harness | Use for the minimal Electron binding smoke; Playwright documents `_electron.launch`. [VERIFIED: npm registry, CITED: https://playwright.dev/docs/api/class-electron] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| napi-rs | Custom `node-gyp` addon | More manual ABI/build work; napi-rs has official CLI/type generation flow. [CITED: https://napi.rs/docs/introduction/getting-started] |
| Rust schema generation | Hand-written JSON Schema / TypeScript | Creates drift risk against Rust-owned serde contracts. [VERIFIED: CONTEXT.md, CITED: https://docs.rs/schemars/latest/schemars/, https://docs.rs/ts-rs/latest/ts_rs/] |
| Required render smoke | Optional/skipped FFmpeg test | Contradicts D-15; missing FFmpeg must fail Phase 1 gates. [VERIFIED: CONTEXT.md] |
| Pixel/hash render golden | Metadata-only smoke | Phase 1 explicitly avoids pixel/hash comparisons until later draft/render semantics exist. [VERIFIED: CONTEXT.md] |

**Installation:**
```bash
corepack enable pnpm
corepack use pnpm@10.32.1
pnpm install

pnpm add -D electron@42.4.1 typescript@6.0.3 vite@8.0.16 @vitejs/plugin-react@6.0.2 @playwright/test@1.61.0 @napi-rs/cli@3.7.2
pnpm add react@19.2.7 react-dom@19.2.7

cargo add --workspace serde@1.0.228 serde_json@1.0.150 schemars@1.2.1 ts-rs@12.0.1 thiserror@2.0.18 which@8.0.4 camino@1.2.2
cargo add --workspace --dev tempfile@3.27.0 assert_cmd@2.2.2 insta@1.48.0 jsonschema@0.46.5
```
[VERIFIED: npm registry, crates.io, slopcheck OK]

**Version verification:** Ran `npm view`, `cargo search`, and `cargo info` on 2026-06-17; versions above reflect registry results from this session. [VERIFIED: npm registry, crates.io]

## Package Legitimacy Audit

> `slopcheck scan --pkg npm|crates.io <package> --json` returned `OK` for every listed package. The installed `slopcheck 0.6.1` did not support `install --json`, so scan-mode JSON was used as the safe equivalent without mutating dependencies. [VERIFIED: local command]

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| electron | npm | created 2022-01-26 per npm metadata | 4,716,757/week | github.com/electron/electron | OK | Approved |
| @napi-rs/cli | npm | created 2022-01-26 per npm metadata | 1,149,779/week | github.com/napi-rs/napi-rs | OK | Approved |
| typescript | npm | created 2022-01-26 per npm metadata | 218,034,542/week | github.com/microsoft/TypeScript | OK | Approved |
| vite | npm | created 2022-01-28 per npm metadata | 139,974,183/week | github.com/vitejs/vite | OK | Approved |
| react | npm | created 2022-01-26 per npm metadata | 143,595,274/week | github.com/facebook/react | OK | Approved |
| react-dom | npm | created 2022-01-26 per npm metadata | 134,434,843/week | github.com/facebook/react | OK | Approved |
| @vitejs/plugin-react | npm | created 2022-01-27 per npm metadata | 64,038,290/week | github.com/vitejs/vite-plugin-react | OK | Approved |
| @playwright/test | npm | created 2022-01-27 per npm metadata | 41,476,311/week | github.com/microsoft/playwright | OK | Approved |
| just | crates.io | created 2016-10-23 | 300,269 recent | github.com/casey/just | OK | Approved as required CLI |
| serde | crates.io | created 2014-12-05 | 209,658,552 recent | github.com/serde-rs/serde | OK | Approved |
| serde_json | crates.io | created 2015-08-07 | 202,551,499 recent | github.com/serde-rs/json | OK | Approved |
| schemars | crates.io | created 2019-08-08 | 105,016,288 recent | github.com/GREsau/schemars | OK | Approved |
| ts-rs | crates.io | created 2020-12-15 | 3,678,144 recent | github.com/Aleph-Alpha/ts-rs | OK | Approved |
| napi / napi-derive / napi-build | crates.io | created 2017-11-30 / 2017-11-30 / 2020-03-16 | 10,832,248 / 10,702,895 / 9,302,906 recent | github.com/napi-rs/napi-rs | OK | Approved |
| thiserror / which / camino | crates.io | created 2019-10-09 / 2015-10-06 / 2021-02-23 | 267,058,134 / 65,579,104 / 46,489,889 recent | upstream repos present | OK | Approved |
| tempfile / assert_cmd / insta / jsonschema | crates.io | created 2015-04-14 / 2018-05-28 / 2019-01-13 / 2020-03-29 | 136,980,844 / 11,795,399 / 18,094,101 / 10,784,575 recent | upstream repos present | OK | Approved |

**Packages removed due to slopcheck [SLOP] verdict:** none. [VERIFIED: local slopcheck]
**Packages flagged as suspicious [SUS]:** none. [VERIFIED: local slopcheck]
**Node postinstall scripts:** `npm view <pkg> scripts.postinstall scripts.install scripts.preinstall` returned no values for the listed Node packages. [VERIFIED: npm registry]

## Architecture Patterns

### System Architecture Diagram

```text
Developer / CI
  |
  v
just dev | just build | just test
  |
  +--> pnpm workspace --------------------+
  |       |                               |
  |       v                               v
  |   Electron main/preload ----IPC----> renderer smoke UI
  |       |
  |       v
  |   Node-API addon (.node)
  |       |
  +------> Rust workspace
          |
          +--> draft_model contract types
          |       +--> schemas/*.schema.json
          |       +--> apps/desktop-electron/src/generated/*.ts
          |
          +--> bindings_node: ping/version/execute_command
          |
          +--> media_runtime: discover env/PATH -> probe ffmpeg/ffprobe
                  |
                  +--> success: binary metadata
                  +--> failure: MissingBinary | VersionProbeFailed | UnsupportedVersion
                  |
                  v
              render smoke: lavfi testsrc2 + sine -> mp4 -> ffprobe JSON metadata checks
```
[VERIFIED: CONTEXT.md, official docs cited in Sources]

### Recommended Project Structure

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
[VERIFIED: CONTEXT.md, .planning/research/ARCHITECTURE.md]

### Pattern 1: Rust-Owned Binding Envelope
**What:** Define `CommandEnvelope`, `CommandResultEnvelope`, `CommandError`, and `CommandEvent` in `draft_model`; export JSON Schema and TypeScript from the same Rust types. [VERIFIED: CONTEXT.md, schemars docs, ts-rs docs]

**When to use:** Every Electron-to-Rust call, including `ping`, `version`, and future edit commands. [VERIFIED: CONTEXT.md]

**Example:**
```rust
// Source: https://docs.rs/schemars/latest/schemars/, https://docs.rs/ts-rs/latest/ts_rs/
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
[VERIFIED: docs.rs]

### Pattern 2: Safe Electron IPC Surface
**What:** Main process registers `ipcMain.handle("core:executeCommand", ...)`; preload exposes only `executeCommand`, `ping`, and `version` via `contextBridge`. [CITED: https://www.electronjs.org/docs/latest/tutorial/ipc]

**When to use:** Renderer calls Rust only through this typed preload surface; do not expose raw `ipcRenderer`. [CITED: https://www.electronjs.org/docs/latest/tutorial/ipc, https://www.electronjs.org/docs/latest/tutorial/context-isolation]

**Example:**
```ts
// Source: https://www.electronjs.org/docs/latest/tutorial/ipc
contextBridge.exposeInMainWorld("videoEditorCore", {
  executeCommand: (command: CommandEnvelope) =>
    ipcRenderer.invoke("core:executeCommand", command),
  ping: () => ipcRenderer.invoke("core:ping"),
  version: () => ipcRenderer.invoke("core:version"),
});
```
[VERIFIED: Electron docs]

### Pattern 3: FFmpeg Discovery Order
**What:** Resolve explicit env var first, then `PATH`; run `-version`; bound stderr/stdout captured in errors. [VERIFIED: CONTEXT.md]

**When to use:** `media_runtime::discover_runtime_config()` and the render smoke setup. [VERIFIED: CONTEXT.md]

**Example:**
```rust
// Source: Phase context D-09..D-11 plus https://docs.rs/which/latest/which/
fn resolve_binary(env_name: &str, fallback_name: &str) -> Result<Utf8PathBuf, DiscoveryError> {
    if let Some(explicit) = std::env::var_os(env_name) {
        return validate_candidate(explicit, env_name);
    }
    which::which(fallback_name)
        .map_err(|_| DiscoveryError::missing_binary(fallback_name, vec![env_name, "PATH"]))
        .and_then(|path| validate_candidate(path, "PATH"))
}
```
[VERIFIED: CONTEXT.md, docs.rs]

### Pattern 4: Tiny Render Smoke
**What:** Generate all media during the test run with FFmpeg lavfi, write output to a temp dir, then verify duration/resolution/fps/stream presence with ffprobe JSON. [VERIFIED: CONTEXT.md, FFmpeg docs]

**When to use:** Required in `just test` and CI; no binary media should be committed. [VERIFIED: CONTEXT.md]

**Example command shape:**
```bash
ffmpeg -hide_banner -y \
  -f lavfi -i "testsrc2=size=160x90:rate=10:duration=1" \
  -f lavfi -i "sine=frequency=440:duration=1" \
  -c:v libx264 -pix_fmt yuv420p -c:a aac "$TMPDIR/tiny-smoke.mp4"

ffprobe -v error -output_format json \
  -show_entries stream=codec_type,width,height,r_frame_rate,duration:format=duration \
  "$TMPDIR/tiny-smoke.mp4"
```
[CITED: https://ffmpeg.org/ffmpeg-filters.html, https://ffmpeg.org/ffprobe.html]

### Anti-Patterns to Avoid
- **UI-owned FFmpeg strings:** Violates the architecture constraint that UI emits commands and Rust/runtime layers own media execution. [VERIFIED: AGENTS.md]
- **Raw `ipcRenderer` exposure:** Electron docs warn to limit renderer access instead of exposing full IPC APIs. [CITED: https://www.electronjs.org/docs/latest/tutorial/ipc]
- **Hand-written TypeScript contracts:** Contradicts Rust serde as source of truth and creates schema drift risk. [VERIFIED: CONTEXT.md]
- **Skipping render smoke when FFmpeg is missing:** Contradicts D-15; missing binaries are a failing Phase 1 condition. [VERIFIED: CONTEXT.md]
- **Generic platform crate:** Contradicts D-20; traits belong at consuming service boundaries. [VERIFIED: CONTEXT.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Node native binding build | Custom node-gyp/ABI loader | napi-rs CLI + `napi` crates | Official NAPI-RS flow builds native addons and generated JS/TS binding files. [CITED: https://napi.rs/docs/cli/build] |
| Schema generation | Manually maintained JSON Schema | `schemars` from Rust types | Schemars derives schema from Rust types and honors serde attributes. [CITED: https://docs.rs/schemars/latest/schemars/] |
| TS contract generation | Hand-written `.d.ts` for Rust envelopes | `ts-rs` | ts-rs derives TS declarations from Rust structs/enums and supports serde compatibility. [CITED: https://docs.rs/ts-rs/latest/ts_rs/] |
| PATH lookup | OS-specific shell scripts | `which` crate | `which` provides cross-platform executable lookup. [CITED: https://docs.rs/which/latest/which/] |
| Temp output lifecycle | Ad hoc `/tmp` path strings | `tempfile` | `tempfile` handles temporary file/dir cleanup and documents lifetime/security pitfalls. [CITED: https://docs.rs/tempfile/latest/tempfile/] |
| JSON Schema validation | Custom recursive validator | `jsonschema` crate | It validates instances and schema documents against supported drafts. [CITED: https://docs.rs/jsonschema/latest/jsonschema/] |
| Golden/snapshot mechanics | Custom diff format | `insta` | Use a maintained Rust snapshot test library for stable text/JSON snapshots. [VERIFIED: crates.io, CITED: https://docs.rs/insta/latest/insta/] |

**Key insight:** Phase 1 is infrastructure; custom infrastructure here becomes a permanent tax on every later phase. Use standard workspace, binding, schema, validation, and smoke-test tools so later work focuses on editor semantics. [VERIFIED: CONTEXT.md]

## Common Pitfalls

### Pitfall 1: Binding Works in Node but Not Electron
**What goes wrong:** The `.node` addon builds, but Electron cannot load it because paths, CJS/ESM mode, or native binary naming differ. [CITED: https://napi.rs/docs/introduction/getting-started]
**Why it happens:** NAPI-RS generated loaders are platform-aware and Electron packaging/loading paths differ from plain Node. [CITED: https://napi.rs/docs/introduction/getting-started]
**How to avoid:** In Phase 1, load the binding from Electron main process and add a Playwright Electron smoke that calls `ping`. [CITED: https://playwright.dev/docs/api/class-electron]
**Warning signs:** `pnpm test` passes for a Node script but Electron smoke fails with native module load errors. [ASSUMED]

### Pitfall 2: Schema and TS Types Drift
**What goes wrong:** Generated `schemas/*.json` and `src/generated/*.ts` differ from Rust contract types. [VERIFIED: CONTEXT.md]
**Why it happens:** Generated files are not checked in or drift checks are not part of `just test`. [VERIFIED: CONTEXT.md]
**How to avoid:** Add a schema/type generation command and a `git diff --exit-code` or content comparison gate in `just test`. [VERIFIED: CONTEXT.md]
**Warning signs:** UI TypeScript accepts fields that Rust rejects or schema fixtures pass in one layer only. [VERIFIED: CONTEXT.md]

### Pitfall 3: FFmpeg Discovery Error Is Not Actionable
**What goes wrong:** User sees `ENOENT` or raw stderr without remediation. [VERIFIED: CONTEXT.md]
**Why it happens:** Runtime executes `ffmpeg` directly without an explicit discovery/probe layer. [VERIFIED: CONTEXT.md]
**How to avoid:** Return `MissingBinary`, `VersionProbeFailed`, or `UnsupportedVersion` with checked paths and bounded stderr. [VERIFIED: CONTEXT.md]
**Warning signs:** Tests assert only failure, not error kind/remediation fields. [VERIFIED: CONTEXT.md]

### Pitfall 4: Render Smoke Is Too Ambitious
**What goes wrong:** Pixel/hash assertions become flaky before render semantics exist. [VERIFIED: CONTEXT.md]
**Why it happens:** Golden testing jumps ahead of Phase 2/5 draft/render decisions. [VERIFIED: CONTEXT.md]
**How to avoid:** Assert only file existence and ffprobe metadata in Phase 1. [VERIFIED: CONTEXT.md]
**Warning signs:** Test fixture includes committed binary media or image hash baselines. [VERIFIED: CONTEXT.md]

### Pitfall 5: Pure Crates Gain Platform Dependencies
**What goes wrong:** `draft_model`, `draft_commands`, or `engine_core` depend on filesystem/FFmpeg/platform traits. [VERIFIED: CONTEXT.md]
**Why it happens:** Cross-platform boundaries are added globally instead of at consuming service crates. [VERIFIED: CONTEXT.md]
**How to avoid:** Keep service traits in `media_runtime`, `project_store`, and `preview_service`. [VERIFIED: CONTEXT.md]
**Warning signs:** A pure semantic crate imports `std::process`, `which`, Electron, filesystem trait objects, or FFmpeg names. [VERIFIED: CONTEXT.md]

## Code Examples

### Schema and Type Export Test
```rust
// Source: https://docs.rs/schemars/latest/schemars/, https://docs.rs/ts-rs/latest/ts_rs/
#[test]
fn export_command_contracts() {
    let schema = schemars::schema_for!(CommandEnvelope);
    let json = serde_json::to_string_pretty(&schema).unwrap();
    std::fs::write("../../schemas/command.schema.json", json).unwrap();

    CommandEnvelope::export_to("../../apps/desktop-electron/src/generated/CommandEnvelope.ts").unwrap();
}
```
[VERIFIED: docs.rs]

### Electron Main Handler Shape
```ts
// Source: https://www.electronjs.org/docs/latest/tutorial/ipc
ipcMain.handle("core:executeCommand", async (_event, command: CommandEnvelope) => {
  return nativeBinding.executeCommand(command);
});
```
[VERIFIED: Electron docs]

### `justfile` Gate Shape
```make
# Source: https://just.systems/man/en/
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
[VERIFIED: CONTEXT.md, just docs]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Electron renderer directly requires Node/native modules | Context-isolated renderer with preload `contextBridge` and limited IPC surface | Electron security model is current in official docs | Prevents renderer from gaining broad Node/Electron access. [CITED: https://www.electronjs.org/docs/latest/tutorial/ipc, https://www.electronjs.org/docs/latest/tutorial/context-isolation] |
| Hand-maintained JS native addon build scripts | napi-rs CLI `napi build` and generated JS/TS binding files | NAPI-RS current docs | Reduces native binding build/platform boilerplate. [CITED: https://napi.rs/docs/cli/build] |
| Committed binary fixtures for smoke media | Generate tiny media at test time with lavfi | Locked Phase 1 decision | Keeps repo small and deterministic. [VERIFIED: CONTEXT.md, CITED: https://ffmpeg.org/ffmpeg-filters.html] |

**Deprecated/outdated:**
- Exposing raw `ipcRenderer` to the renderer is not acceptable for this project; expose only narrow preload methods. [CITED: https://www.electronjs.org/docs/latest/tutorial/ipc]
- Using Kdenlive/MLT code, XML, presets, or assets is not allowed; they are conceptual references only. [VERIFIED: AGENTS.md]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Electron native binding load failures may appear in Electron but not plain Node if loader paths or module formats differ. | Common Pitfalls | Planner should keep the Electron smoke test in Phase 1 instead of relying only on Node-level tests. |

## Open Questions

1. **Should CI target Node 24.12.0 exactly or an LTS range?**
   - What we know: Local Node is 24.12.0; Vite 8 requires Node 20.19+ or 22.12+. [VERIFIED: local command, CITED: https://vite.dev/guide/]
   - What's unclear: The project has not chosen a CI Node version policy beyond requiring guidance. [VERIFIED: CONTEXT.md]
   - Recommendation: Pin `.nvmrc`/CI to `24.12.0` for exact local parity in Phase 1, and document that Electron's bundled Node is separate from system Node. [VERIFIED: local command, CITED: https://www.electronjs.org/docs/latest/tutorial/tutorial-prerequisites]

2. **Should generated schema/TS files be committed?**
   - What we know: Phase 1 requires contract drift tests and Rust-owned generated contracts. [VERIFIED: CONTEXT.md]
   - What's unclear: The user has not explicitly chosen committed generated files vs generated-on-test only. [VERIFIED: CONTEXT.md]
   - Recommendation: Commit generated schema/TS files and make `just test` fail if regeneration changes them. [VERIFIED: CONTEXT.md]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Node.js | pnpm/Electron/Vite/napi-rs CLI | yes | v24.12.0 | Use pinned CI Node if local differs. [VERIFIED: local command] |
| npm | registry tooling/Corepack updates | yes | 11.6.2 | pnpm for project installs. [VERIFIED: local command] |
| pnpm | Node workspace | yes | 10.32.1 | Corepack can install pinned pnpm. [VERIFIED: local command, pnpm docs] |
| Corepack | package manager pinning | yes | 0.34.5 | Install/update Corepack with npm if absent. [VERIFIED: local command, pnpm docs] |
| Rust/Cargo | Rust workspace | yes | rustc 1.95.0 / cargo 1.95.0 | rustup toolchain file should install pinned toolchain. [VERIFIED: local command, rustup docs] |
| just | unified commands | no | crates.io latest 1.52.0 | Install via Homebrew or Cargo before running Phase 1 gates; CI must install it before `just build/test`. [VERIFIED: local command, crates.io, CONTEXT.md] |
| FFmpeg | discovery and render smoke | yes | 8.1 | No fallback; missing binary fails `just test`. [VERIFIED: local command, CONTEXT.md] |
| ffprobe | metadata validation | yes | 8.1 | No fallback; missing binary fails `just test`. [VERIFIED: local command, CONTEXT.md] |
| slopcheck | package legitimacy audit | yes | 0.6.1 | Scan-mode JSON used because `install --json` is unsupported. [VERIFIED: local command] |

**Missing dependencies with no fallback:**
- `just` is required by locked decision D-01 and is not installed locally; planner must include a setup step such as `brew install just` or `cargo install just --locked`. [VERIFIED: local command, CONTEXT.md, crates.io]

**Missing dependencies with fallback:** Corepack can provision the pinned pnpm if a fresh environment lacks pnpm. [VERIFIED: pnpm docs]

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test`, Playwright Electron via `@playwright/test` 1.61.0, pnpm script tests. [VERIFIED: cargo docs, Playwright docs, npm registry] |
| Config file | None exists yet; Wave 0 must create `Cargo.toml`, `pnpm-workspace.yaml`, package scripts, and optional `playwright.config.ts`. [VERIFIED: repository scan] |
| Quick run command | `just test` [VERIFIED: CONTEXT.md] |
| Full suite command | `just test` in Phase 1; later phases may split quick/full once suites grow. [VERIFIED: CONTEXT.md] |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| FOUND-01 | Clean checkout builds Rust workspace and Electron shell | integration | `just build` | no, Wave 0 |
| FOUND-02 | Electron calls Rust binding and receives typed response | Electron smoke + Rust unit | `pnpm --filter @video-editor/desktop test` and `cargo test -p bindings_node` | no, Wave 0 |
| FOUND-03 | FFmpeg/ffprobe discovery succeeds or returns structured actionable error | Rust unit/integration | `cargo test -p media_runtime discovery` | no, Wave 0 |
| FOUND-04 | Fixture/golden structure and tiny render smoke exist | Rust integration | `cargo test -p testkit render_smoke` | no, Wave 0 |
| TEST-01 | Schema/model tests validate every golden draft fixture | Rust unit/integration | `cargo test -p draft_model schema` | no, Wave 0 |

### Sampling Rate
- **Per task commit:** `just test` because Phase 1 is small and all gates are foundation-critical. [VERIFIED: CONTEXT.md]
- **Per wave merge:** `just test` plus `git diff --exit-code schemas apps/desktop-electron/src/generated` after generation. [VERIFIED: CONTEXT.md]
- **Phase gate:** `just build` and `just test` green before `$gsd-verify-work`. [VERIFIED: GSD config, CONTEXT.md]

### Wave 0 Gaps
- [ ] `Cargo.toml` workspace and compile-safe crate shells for all planned crates. [VERIFIED: repository scan, CONTEXT.md]
- [ ] `package.json`, `pnpm-workspace.yaml`, `apps/desktop-electron/package.json`, and Corepack `packageManager`. [VERIFIED: repository scan, CONTEXT.md]
- [ ] `justfile` with `dev`, `build`, `test`. [VERIFIED: repository scan, CONTEXT.md]
- [ ] `crates/draft_model` contract types and schema/type generation test. [VERIFIED: repository scan, CONTEXT.md]
- [ ] `crates/media_runtime` discovery tests for env var, PATH, missing binary, bad binary. [VERIFIED: repository scan, CONTEXT.md]
- [ ] `crates/testkit` render smoke using lavfi + ffprobe metadata. [VERIFIED: repository scan, CONTEXT.md]
- [ ] Minimal Playwright Electron binding smoke. [VERIFIED: repository scan, Playwright docs]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | No auth in Phase 1. [VERIFIED: REQUIREMENTS.md] |
| V3 Session Management | no | No sessions in Phase 1. [VERIFIED: REQUIREMENTS.md] |
| V4 Access Control | limited | Renderer cannot bypass preload/main IPC boundary; Rust validates command envelopes. [VERIFIED: Electron IPC docs, CONTEXT.md] |
| V5 Input Validation | yes | serde + `deny_unknown_fields`, JSON Schema validation, typed command envelope. [VERIFIED: schemars docs, serde docs, CONTEXT.md] |
| V6 Cryptography | no | No cryptographic feature in Phase 1. [VERIFIED: REQUIREMENTS.md] |

### Known Threat Patterns for Rust/Electron/FFmpeg Foundation

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Renderer invokes arbitrary IPC/native methods | Elevation of privilege | Expose only narrow `contextBridge` methods; never expose raw `ipcRenderer`. [CITED: https://www.electronjs.org/docs/latest/tutorial/ipc] |
| Command envelope accepts unknown fields | Tampering | Use serde `deny_unknown_fields`, schema validation, and negative fixtures. [VERIFIED: serde docs, schemars docs] |
| User-controlled FFmpeg path executes unintended binary | Spoofing / Tampering | Explicit env var paths must be probed with `-version`, errors include checked path, and later UI should display the resolved path. [VERIFIED: CONTEXT.md] |
| Unbounded process stderr floods logs/UI | Denial of service | Bound stderr summary in `VersionProbeFailed` and render smoke failures. [VERIFIED: CONTEXT.md] |
| Shell injection through FFmpeg command strings | Tampering / Elevation of privilege | Use `std::process::Command` args, not shell-concatenated command strings; UI never constructs FFmpeg args. [VERIFIED: AGENTS.md] |

## Sources

### Primary (HIGH confidence)
- `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md` - locked implementation decisions and deferrals.
- `.planning/REQUIREMENTS.md` - Phase 1 requirement IDs and descriptions.
- `AGENTS.md` - project architecture, terminology, testing, and licensing constraints.
- https://www.electronjs.org/docs/latest/ - Electron app model and quick-start example.
- https://www.electronjs.org/docs/latest/tutorial/ipc - IPC, preload, `contextBridge`, `ipcMain.handle`, and raw IPC exposure warning.
- https://www.electronjs.org/docs/latest/tutorial/context-isolation - context isolation reference.
- https://pnpm.io/workspaces - pnpm workspace requirements.
- https://pnpm.io/installation - Corepack and `packageManager` pinning.
- https://doc.rust-lang.org/cargo/reference/workspaces.html - Cargo workspace structure.
- https://rust-lang.github.io/rustup/overrides.html - `rust-toolchain.toml` behavior.
- https://just.systems/man/en/ - just recipes and command-runner behavior.
- https://napi.rs/docs/introduction/getting-started and https://napi.rs/docs/cli/build - NAPI-RS CLI setup/build behavior.
- https://docs.rs/schemars/latest/schemars/ - Rust JSON Schema generation.
- https://docs.rs/ts-rs/latest/ts_rs/ - Rust to TypeScript generation.
- https://ffmpeg.org/ffmpeg-filters.html - `testsrc2` and `sine` lavfi sources.
- https://ffmpeg.org/ffprobe.html - ffprobe JSON output and `show_entries`.

### Secondary (MEDIUM confidence)
- npm registry via `npm view` - versions, publish metadata, repository URLs, and postinstall script checks.
- crates.io via `cargo search`, `cargo info`, and crates.io API - crate versions, metadata, repositories, and download counts.
- slopcheck 0.6.1 scan output - package legitimacy status.

### Tertiary (LOW confidence)
- None beyond the single assumption logged in the Assumptions Log.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - Locked by context and verified against official docs, npm registry, crates.io, and slopcheck.
- Architecture: HIGH - Directly constrained by AGENTS.md, CONTEXT.md, and existing project research.
- Pitfalls: MEDIUM - Most are verified from official docs/context; one Electron-vs-Node native loading risk is marked assumed.
- Package legitimacy: HIGH - slopcheck returned OK, package names were confirmed from official docs/docs.rs where recommended, and registries were checked in the correct ecosystems.
- Environment: HIGH - Local tools were probed directly.

**Research date:** 2026-06-17
**Valid until:** 2026-07-17 for architecture decisions; re-check package versions before install because Electron, Vite, Playwright, and napi-rs move quickly.

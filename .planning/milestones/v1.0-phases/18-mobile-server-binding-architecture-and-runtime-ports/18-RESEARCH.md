# Phase 18: mobile-server-binding-architecture-and-runtime-ports - Research

**Researched:** 2026-06-25
**Domain:** Rust portable runtime, Electron Node-API adapter, C ABI, opaque handle lifecycle, server export runtime
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Binding Ownership

- **D-01:** Rust core must expose a shared production API below `bindings_node`; Node-API, C ABI, future JNI/Swift, and server entrypoints are adapters over that shared Rust API, not independent semantic implementations.
- **D-02:** `bindings_node` should become a thin desktop adapter for JSON/Node-API transport, Electron IPC, and desktop resource wiring. It must not own editing semantics, project lifecycle semantics, preview scheduling policy, render/export behavior, or resource lifetime policy.
- **D-03:** If an existing boundary in `bindings_node` is structurally wrong for portable ownership, replace it destructively instead of preserving compatibility. The project is not launched; old partial paths should be removed or guarded, not wrapped as a fallback success path.

#### Runtime And Resource Handles

- **D-04:** Runtime sessions, project sessions, media handles, frame handles, texture handles, and artifact handles must be represented as opaque Rust-owned IDs with owner session, generation, reference count or lease count, explicit release, cascading close, and debug leak diagnostics.
- **D-05:** Stale generation, wrong owner, wrong device, expired lease, double release, and unknown handle cases must fail closed with typed diagnostics. They must not silently fall back to byte copies, mock resources, artifact previews, or Electron-owned state.
- **D-06:** Handle lifetime authority belongs in Rust. JavaScript, C callers, future JNI, and future Swift callers may hold opaque tokens only; they cannot fabricate handles, mutate handle metadata, or infer lifetime from object garbage collection.

#### Low-Copy Media And Preview Boundary

- **D-07:** Large media frames and preview outputs should cross language/runtime boundaries through handles whenever a handle path is available. Raw byte transfer is acceptable only for explicit diagnostic/test surfaces or unsupported paths that are reported as failures/degradations, not as product success.
- **D-08:** Native texture/device identity must remain explicit: backend, adapter/device ID, owner session, generation, dimensions, pixel format, and color metadata must be verified before import or presentation.
- **D-09:** Preview and render evidence must continue to follow the no-product-fallback rule. CPU readback, artifact output, DOM evidence, mock surfaces, or debug probes do not prove a production preview/render path.

#### C ABI And Mobile Contracts

- **D-10:** Add a contract-first portable C ABI surface for runtime/session/handle operations with stable error codes, explicit ownership, and smoke tests. The C ABI should call the shared Rust API and should not duplicate Node-API request handling logic.
- **D-11:** JNI and Swift/ObjC are represented in Phase 18 as contract documents, headers/type maps, and smoke-level handle/session tests. Full Android/iOS app shells, UI, packaging, permissions UX, and store deployment are out of scope.
- **D-12:** Mobile lifecycle contracts must cover app background/foreground, sandboxed media permissions, file handles, texture/device handles, cancellation, and session close semantics so future mobile apps do not reinterpret desktop behavior.

#### Server Runtime

- **D-13:** Server runtime must be a first-party Rust runtime surface that opens `.veproj/project.json` through `project_store`, resolves bundle-relative materials, compiles render/export jobs through the shared render/export path, and reports progress/cancellation/errors without Electron.
- **D-14:** Server runtime should not depend on Electron, BrowserWindow, preload IPC, DOM state, desktop UI view models, or desktop-only native surface handles.
- **D-15:** Server runtime verification must use real `.veproj` fixtures and export/progress evidence. A CLI that only prints parsed project metadata is insufficient for Phase 18 success.

#### Verification And Gates

- **D-16:** Phase 18 plans must include source guards that fail if semantics are duplicated across Node/C/server adapters, if renderer/main constructs render/export behavior, or if product success can be satisfied by fallback/mock/artifact evidence.
- **D-17:** Phase 18 verification should include Rust unit/integration tests, ABI/header/schema drift checks, C ABI smoke tests, server export smoke tests, Node-API desktop smoke tests, and existing contract gates.
- **D-18:** UI design work is not a primary Phase 18 deliverable. If any desktop UI changes are required to expose runtime diagnostics, an independent UI/design review subagent should audit them for product quality and can make production-grade layout/style corrections.

### the agent's Discretion

- Planner/researcher may choose the exact crate names and file splits, but should prefer names aligned with the established stack: `bindings_node`, later `bindings_c`, portable runtime/session/handle APIs, `project_store`, `media_runtime`, `realtime_preview_runtime`, `task_runtime`, `render_graph`, `ffmpeg_compiler`, and `media_runtime_desktop`.
- Planner may split the phase into more plans if the shared API, C ABI, server runtime, handle registry, platform contracts, and aggregate gates cannot be implemented safely in a smaller wave count.

### Deferred Ideas (OUT OF SCOPE)

- Full iOS and Android applications, mobile UI, platform permission UX, app packaging, and store readiness are deferred beyond Phase 18.
- Phase 19 owns production effects, retiming, transition semantics, masks, filters, and complex template fidelity after portable runtime/binding foundations are in place.
- Cloud multi-tenant rendering, job queue service deployment, authentication, billing, and remote storage synchronization are not Phase 18 scope unless represented only as local server-runtime extension points.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PLAT-01 | Rust core exposes a C/FFI boundary for mobile/server shells. [VERIFIED: .planning/REQUIREMENTS.md] | Add `bindings_c` as a contract-first `cdylib`/`staticlib` adapter over shared Rust runtime APIs, with `#[repr(C)]` structs and generated headers. [CITED: https://doc.rust-lang.org/reference/linkage.html] |
| PLAT-02 | Server renderer can render a `.veproj` without Electron. [VERIFIED: .planning/REQUIREMENTS.md] | Move project open/material resolution/export scheduling below `bindings_node`, then add a server crate/bin that calls the same runtime export service. [VERIFIED: crates/bindings_node/src/preview_export_service.rs] |
| PLAT-03 | iOS and Android extension points are represented by ABI/JNI/Swift contract documents and smoke-level handle/session tests, while full mobile apps remain deferred. [VERIFIED: .planning/REQUIREMENTS.md] | Document JNI thread/lifecycle contracts and Swift/ObjC C-import ownership over the C ABI without building app shells. [CITED: https://developer.android.com/training/articles/perf-jni] [CITED: https://swift.org/blog/improving-usability-of-c-libraries-in-swift/] |
| BIND-01 | Binding architecture separates desktop Node-API, portable C ABI, future Android JNI, future iOS Swift/ObjC, and server entrypoints without duplicating draft semantics. [VERIFIED: .planning/REQUIREMENTS.md] | Use one shared Rust runtime crate under Node/C/server adapters; add source guards that forbid semantic implementation in adapters. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md] |
| BIND-02 | Runtime sessions, project sessions, media handles, frame handles, texture handles, and artifact handles use opaque IDs with owner session, generation, reference count, explicit release, cascading session-close release, and debug leak diagnostics. [VERIFIED: .planning/REQUIREMENTS.md] | Generalize the existing `FramePool` and `NativeTextureLeaseRegistry` owner/generation/release/leak pattern into a shared handle registry. [VERIFIED: crates/media_runtime/src/frame.rs] [VERIFIED: crates/media_runtime/src/texture.rs] |
| BIND-03 | Large media frames and preview outputs cross language boundaries through handle-based or low-copy paths whenever supported, with GPU texture/frame handles bound to their device/context lifetime. [VERIFIED: .planning/REQUIREMENTS.md] | Preserve texture/device identity checks and reject CPU/artifact paths as product compositor input. [VERIFIED: crates/realtime_preview_runtime/src/media_io_adapter.rs] |
| BIND-04 | Server runtime can open `.veproj`, resolve materials, run render/export jobs, and report progress without Electron. [VERIFIED: .planning/REQUIREMENTS.md] | Reuse `project_store`, `render_graph`, `ffmpeg_compiler`, `media_runtime`, `media_runtime_desktop`, and `task_runtime` through shared services rather than Electron IPC. [VERIFIED: docs/runtime-boundaries.md] |
| BIND-05 | ABI, serialization, and binding smoke tests protect contract drift across desktop, mobile prototypes, and server rendering. [VERIFIED: .planning/REQUIREMENTS.md] | Add C header drift, C smoke, Node smoke, server export smoke, contract diff, and source guard gates to the aggregate phase command. [VERIFIED: package.json] |
</phase_requirements>

## Project Constraints (from AGENTS.md)

- UI emits commands; Rust core owns project and timeline semantics. [VERIFIED: AGENTS.md]
- UI code must not construct FFmpeg commands. [VERIFIED: AGENTS.md]
- Known-wrong preview, edit, render, session, media, or native-surface ownership boundaries must be replaced with the long-term production architecture rather than patched. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is the canonical source of truth; render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, and preview caches are derived artifacts. [VERIFIED: AGENTS.md]
- Product language and domain types should use Jianying-aligned terminology such as draft/material/track/segment/keyframe/filter/transition. [VERIFIED: AGENTS.md]
- Core time math must use integer microseconds, frame indices, or rational frame rates rather than naked floating-point persisted semantics. [VERIFIED: AGENTS.md]
- Render Graph isolates editing semantics from FFmpeg, and FFmpeg Runtime executes jobs and reports progress/errors without deciding editing behavior. [VERIFIED: AGENTS.md]
- Kdenlive and MLT are conceptual references only; do not copy GPL code, assets, XML definitions, presets, or UI implementation. [VERIFIED: AGENTS.md]
- External drafts go through adapters and compatibility reports; proprietary IDs are external references, not internal render semantics. [VERIFIED: AGENTS.md]
- Each roadmap phase must define executable gates before implementation is complete. [VERIFIED: AGENTS.md]
- FFmpeg distribution requires LGPL/GPL/nonfree build option, notice, and commercial-obligation review before shipping. [VERIFIED: AGENTS.md]

## Summary

Phase 18 should be planned as a destructive ownership split, not as additive FFI wrappers around `bindings_node`. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md] The current code keeps the Node-API surface explicit, but `bindings_node` still owns the project session registry, material/project IO schedulers, project-session snapshots, and export registry. [VERIFIED: crates/bindings_node/src/lib.rs] [VERIFIED: crates/bindings_node/src/project_session_service.rs] [VERIFIED: crates/bindings_node/src/preview_export_service.rs]

The strongest reusable implementation pattern is already in `media_runtime`: `FramePool` models owner session, lease IDs, lease limits, explicit release, cascading close, and leak diagnostics, while `NativeTextureLeaseRegistry` validates owner session, generation, backend, device identity, dimensions, and pixel format. [VERIFIED: crates/media_runtime/src/frame.rs] [VERIFIED: crates/media_runtime/src/texture.rs] Phase 18 should generalize this into a shared Rust handle/session registry used by Node-API, C ABI, server runtime, and future JNI/Swift contracts. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md]

The server path must be a real export runtime over `.veproj/project.json`, bundle-relative material resolution, Render Graph, FFmpeg compilation, scheduled FFmpeg execution, progress, cancellation, and validation. [VERIFIED: .planning/ROADMAP.md] A CLI that only parses project metadata is explicitly insufficient. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md]

**Primary recommendation:** Add a shared `editor_runtime` crate below adapters, move project session/export/handle authority into it, keep `bindings_node` as a thin JSON/N-API adapter, add `bindings_c` with generated `cbindgen` headers, and add `server_runtime` as a first-party Rust library+bin over the same services. [ASSUMED]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Runtime session and handle authority | Rust Runtime/Core | Adapter tiers | Rust must own opaque IDs, owner session, generation, ref/lease counts, release, cascading close, and leak diagnostics. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md] |
| Desktop Node-API binding | Electron Main / Native Adapter | Rust Runtime/Core | `bindings_node` should parse/serialize JSON and expose N-API methods while delegating semantics to the shared runtime. [VERIFIED: crates/bindings_node/src/lib.rs] |
| Portable C ABI | C ABI Adapter | Rust Runtime/Core | C exports should expose stable error codes and opaque tokens over the shared runtime without duplicating request handling. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md] |
| Future JNI contract | Mobile Adapter Contract | Rust Runtime/Core | Android JNI has process/thread/lifecycle constraints, but full Android shells are deferred. [CITED: https://developer.android.com/training/articles/perf-jni] [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md] |
| Future Swift/ObjC contract | Mobile Adapter Contract | Rust Runtime/Core | Swift imports C APIs and needs explicit pointer/ownership contracts; full iOS shells are deferred. [CITED: https://swift.org/blog/improving-usability-of-c-libraries-in-swift/] [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md] |
| Server `.veproj` export | Server Runtime | Rust Runtime/Core | Server must open `.veproj`, resolve materials, run export jobs, and report progress without Electron. [VERIFIED: .planning/ROADMAP.md] |
| Project persistence | Database / Storage | Rust Runtime/Core | `project_store` owns `.veproj/project.json` open/save validation and material URI checks. [VERIFIED: crates/project_store/src/bundle.rs] |
| Render/export semantics | Rust Runtime/Core | Media Runtime | `engine_core`, `render_graph`, and `ffmpeg_compiler` produce render intent and FFmpeg jobs; runtime executes jobs. [VERIFIED: crates/bindings_node/src/preview_export_service.rs] |
| Electron renderer UI | Browser / Client | Electron Main | Renderer must remain command/UI-only and must not construct FFmpeg/render behavior. [VERIFIED: AGENTS.md] |

## Current Chain

- `bindings_node/src/lib.rs` exports explicit N-API functions such as `openProjectSession`, `executeProjectIntent`, realtime preview controls, and export controls. [VERIFIED: crates/bindings_node/src/lib.rs]
- `bindings_node/src/project_session_service.rs` owns a `OnceLock<Mutex<ProjectSessionRegistry>>`, stores project drafts/revisions/selections/playheads/active interactions, uses `StdPlatformFileSystem`, and runs project IO/material probe scheduling. [VERIFIED: crates/bindings_node/src/project_session_service.rs]
- `bindings_node/src/preview_export_service.rs` owns `SchedulerExportService`, a global export registry, export worker threads, validation jobs, `prepare_export_job`, Render Graph build, FFmpeg compile, and Desktop FFmpeg capability probing. [VERIFIED: crates/bindings_node/src/preview_export_service.rs]
- `apps/desktop-electron/src/main/nativeBinding.ts` loads the N-API addon, verifies the expected function list, and exposes typed wrapper functions. [VERIFIED: apps/desktop-electron/src/main/nativeBinding.ts]
- `apps/desktop-electron/src/main/index.ts` wires Electron IPC to native binding functions and still has test-only mock response paths for runtime capabilities/export/audio/artifact commands. [VERIFIED: apps/desktop-electron/src/main/index.ts]
- `apps/desktop-electron/src/preload/index.ts` exposes sandboxed `videoEditorCore`, `videoEditorPlatform`, and `videoEditorRealtimePreviewHost` methods to the renderer. [VERIFIED: apps/desktop-electron/src/preload/index.ts]
- `media_runtime::FramePool` and `media_runtime::NativeTextureLeaseRegistry` already implement the owner/generation/release validation model Phase 18 needs to generalize. [VERIFIED: crates/media_runtime/src/frame.rs] [VERIFIED: crates/media_runtime/src/texture.rs]

## Standard Stack

### Core

| Library / Crate | Version | Purpose | Why Standard |
|-----------------|---------|---------|--------------|
| Rust workspace | rustc 1.95.0, edition 2024 | Runtime/core implementation and FFI exports | Workspace already pins Rust 1.95.0 and edition 2024. [VERIFIED: Cargo.toml] [VERIFIED: rustc --version] |
| `editor_runtime` | new crate | Shared project/session/export/handle API below adapters | Required by locked decision that adapters must not duplicate semantics. [ASSUMED] |
| `bindings_node` + `napi` + `napi-derive` | crate 0.1.0; `napi` 3.9.2; `napi-derive` 3.5.6 | Desktop Node-API adapter | Existing N-API surface uses `#[napi]` and `serde_json::Value`; package legitimacy for `napi` and `napi-derive` is OK. [VERIFIED: crates/bindings_node/Cargo.toml] [VERIFIED: crates.io] [CITED: https://napi.rs/docs/introduction/getting-started] |
| `bindings_c` | new crate | Portable C ABI for sessions, handles, errors, and smoke tests | Rust supports `cdylib`/`staticlib` crate types and C-compatible exported functions. [ASSUMED] [CITED: https://doc.rust-lang.org/reference/linkage.html] |
| `cbindgen` | 0.29.4, published 2017-04-12 | Generate C headers from Rust ABI declarations | Package legitimacy returned OK, source repo is Mozilla, and registry reports 842,504 weekly downloads. [VERIFIED: crates.io] [CITED: https://github.com/mozilla/cbindgen] |
| `server_runtime` | new crate/bin | Open `.veproj`, resolve materials, run export, report progress without Electron | Cargo packages can contain library and binary targets, so the binary should call shared library services. [ASSUMED] [CITED: https://doc.rust-lang.org/cargo/reference/cargo-targets.html] |

### Supporting

| Library / Crate | Version | Purpose | When to Use |
|-----------------|---------|---------|-------------|
| `project_store` | workspace crate | `.veproj/project.json` open/save/autosave and material URI validation | Use for desktop and server project lifecycle. [VERIFIED: crates/project_store/src/bundle.rs] |
| `task_runtime` | workspace crate | Priority queues, cancellation, freshness, telemetry, resource budgets | Use for export/server jobs and shared runtime schedulers. [VERIFIED: crates/task_runtime/src/scheduler.rs] |
| `media_runtime` | workspace crate | FFmpeg job execution contracts, frame handles, texture handles, runtime validation | Use for frame/texture/resource handle models and export job progress/errors. [VERIFIED: crates/media_runtime/src/frame.rs] |
| `media_runtime_desktop` | workspace crate | Desktop/server FFmpeg executor over bundled binaries | Use as the first server runtime executor until a dedicated non-desktop executor is split. [VERIFIED: docs/runtime-boundaries.md] |
| `render_graph` | workspace crate | Render intent graph | Use before FFmpeg compilation for export. [VERIFIED: crates/bindings_node/src/preview_export_service.rs] |
| `ffmpeg_compiler` | workspace crate | Render graph to FFmpeg job/sidecars/validation expectations | Use from shared export service, not from Electron renderer/main. [VERIFIED: crates/bindings_node/src/preview_export_service.rs] |
| `testkit` | workspace crate | Render/export fixture helpers | Use for server export smoke and cross-binding parity fixtures. [VERIFIED: package.json] |
| `@napi-rs/cli` | 3.7.2, published 2026-06-14 | Builds the existing desktop native addon | Existing dependency is flagged SUS by package legitimacy because the version is very new; planner must add a checkpoint before upgrading or reinstalling. [WARNING: flagged as suspicious — verify before using.] [VERIFIED: npm registry] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Shared `editor_runtime` crate | Keep semantics in `bindings_node` and wrap it from C/server | Violates locked decisions and would duplicate or depend on desktop Node-API for portable/server surfaces. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md] |
| `cbindgen` generated header | Hand-written `.h` file | Hand-written headers drift from Rust ABI and weaken BIND-05 drift protection. [CITED: https://github.com/mozilla/cbindgen] |
| Explicit release functions | JS/Java/Swift garbage collection finalizers | Locked decisions require Rust-owned lifetime authority and explicit release/cascading close. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md] |
| Server runtime over shared export service | Server CLI that parses project metadata only | Context says metadata-only CLI is insufficient for Phase 18 success. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md] |

**Installation:**

```bash
# Do not require a global cbindgen install.
# scripts/phase18-abi-drift.sh bootstraps and reuses a repo-local 0.29.4 binary.
bash scripts/phase18-abi-drift.sh --self-test
```

**Version verification:**

```bash
cargo search cbindgen --limit 3
cargo search napi --limit 5
cargo search napi-derive --limit 5
npm view @napi-rs/cli version time.modified time.created repository.url scripts.postinstall
```

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `cbindgen` | crates.io | published 2017-04-12 | 842,504/week | github.com/mozilla/cbindgen | OK | Approved for header generation. [VERIFIED: crates.io] |
| `napi` | crates.io | published 2017-11-30 | 889,093/week | github.com/napi-rs/napi-rs | OK | Existing dependency approved. [VERIFIED: crates.io] |
| `napi-derive` | crates.io | published 2017-11-30 | 877,784/week | github.com/napi-rs/napi-rs | OK | Existing dependency approved. [VERIFIED: crates.io] |
| `@napi-rs/cli` | npm | created 2020-11-09; version 3.7.2 published 2026-06-14 | 1,115,825/week | github.com/napi-rs/napi-rs | SUS | Existing dependency; checkpoint before upgrade/reinstall because seam flagged `too-new`. [VERIFIED: npm registry] |

**Packages removed due to [SLOP] verdict:** none. [VERIFIED: package-legitimacy check]
**Packages flagged as suspicious [SUS]:** `@napi-rs/cli` if the plan installs or upgrades it. [VERIFIED: package-legitimacy check]

*Packages discovered via WebSearch or training data that have not been verified against an authoritative source are tagged `[ASSUMED]` and the planner must gate each install behind a `checkpoint:human-verify` task.* [VERIFIED: package_legitimacy_protocol]

## Architecture Patterns

### System Architecture Diagram

```text
Electron renderer
  -> preload typed APIs
  -> Electron main IPC validation
  -> bindings_node JSON/N-API adapter
  -> editor_runtime shared Rust API
       -> RuntimeSessionRegistry
       -> ProjectSessionService -> project_store -> .veproj/project.json
       -> HandleRegistry -> media_runtime frame/texture/artifact handles
       -> ExportService -> engine_core -> render_graph -> ffmpeg_compiler
       -> TaskScheduler -> media_runtime / media_runtime_desktop FFmpeg execution
       -> progress/errors/diagnostics

C caller / future JNI / future Swift
  -> bindings_c extern "C" ABI + generated header
  -> same editor_runtime shared Rust API
  -> same handles, sessions, export service, diagnostics

Server CLI / server library
  -> server_runtime entrypoint
  -> same editor_runtime shared Rust API
  -> open .veproj, resolve materials, export, progress, cancel, validate
```

The diagram places semantic ownership in `editor_runtime`, not in `bindings_node`, `bindings_c`, Electron main, or server CLI. [ASSUMED]

### Recommended Project Structure

```text
crates/
├── editor_runtime/        # shared project/session/export/handle services below adapters
├── bindings_node/         # thin N-API/serde_json desktop adapter
├── bindings_c/            # C ABI cdylib/staticlib, cbindgen config, C smoke fixtures
├── server_runtime/        # Rust library + bin for Electron-free .veproj export
├── media_runtime/         # existing media/frame/texture/runtime contracts
├── project_store/         # existing .veproj source of truth
├── task_runtime/          # existing scheduler/cancellation/telemetry
└── testkit/               # existing fixtures and render smoke helpers

docs/
├── mobile-runtime-contracts.md       # JNI/Swift lifecycle, file, texture, permission contracts
└── runtime-boundaries.md             # update/supersede Phase 18 boundary ownership

scripts/
└── phase18-source-guards.sh          # semantic duplication, fallback success, ABI drift guards
```

This file split is a prescriptive recommendation under the phase's crate-name discretion. [ASSUMED]

### Pattern 1: Shared Runtime API Under Adapters

**What:** Put `RuntimeSessionRegistry`, `ProjectSessionService`, `ExportService`, and `HandleRegistry` in `editor_runtime`; adapters only convert transport types to shared request/response structs. [ASSUMED]

**When to use:** Use for every Node/C/server operation that touches project lifecycle, media handles, frame/texture handles, export jobs, or cancellation. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md]

**Example:**

```rust
// Source: repo pattern from bindings_node project_session_service + Rust ABI docs.
pub struct EditorRuntime {
    sessions: RuntimeSessionRegistry,
    projects: ProjectSessionService,
    exports: ExportService,
    handles: HandleRegistry,
}

impl EditorRuntime {
    pub fn open_project_session(
        &mut self,
        request: OpenProjectSession,
    ) -> Result<ProjectSessionOpened, RuntimeError> {
        self.projects.open(request, &mut self.handles, &mut self.exports)
    }
}
```

### Pattern 2: Opaque Numeric Handle Tokens

**What:** Public adapters should pass tokens such as `{ kind, id, ownerSession, generation }` or C `ve_handle_t` values, while Rust stores the resource metadata and actual resources. [ASSUMED]

**When to use:** Use for runtime sessions, project sessions, media, frames, textures, artifacts, export jobs, and leases. [VERIFIED: .planning/REQUIREMENTS.md]

**Example:**

```rust
// Source: Rust Reference repr(C) layout + existing FramePool/TextureHandle contracts.
#[repr(C)]
pub struct ve_handle_t {
    pub kind: u32,
    pub id: u64,
    pub owner_session: u64,
    pub generation: u64,
}

pub fn resolve_texture(handle: ve_handle_t, expected_device: RuntimeDeviceId) -> Result<TextureLease, RuntimeError> {
    HANDLE_REGISTRY.resolve_texture(handle, expected_device)
}
```

### Pattern 3: C ABI Error Buffer With Explicit Release

**What:** C functions should return stable integer status codes and write structured JSON/error text into caller-provided buffers or Rust-allocated strings that have matching release functions. [CITED: https://doc.rust-lang.org/nomicon/ffi.html]

**When to use:** Use for C smoke tests and mobile contract prototypes where C callers cannot consume Rust `Result` or `serde_json::Value`. [CITED: https://doc.rust-lang.org/nomicon/ffi.html]

**Example:**

```rust
// Source: Rust 2024 unsafe attributes + Rust FFI docs.
#[repr(C)]
pub struct ve_status_t {
    pub code: i32,
    pub required_len: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn ve_runtime_open_project(
    runtime: ve_handle_t,
    path: *const std::ffi::c_char,
    out_json: *mut std::ffi::c_char,
    out_len: usize,
) -> ve_status_t {
    bindings_c::open_project(runtime, path, out_json, out_len)
}
```

### Pattern 4: Server Runtime Uses Same Export Service

**What:** Server entrypoints should call `editor_runtime::ExportService::start_project_session_export` and poll/subscribe progress from the same job registry used by Node/C. [ASSUMED]

**When to use:** Use for `.veproj` export CLI and integration tests. [VERIFIED: .planning/ROADMAP.md]

**Example:**

```rust
// Source: Cargo library+binary target pattern and current export service flow.
fn main() -> anyhow::Result<()> {
    let args = ServerExportArgs::parse();
    let mut runtime = EditorRuntime::server_default(args.ffmpeg_runtime_dir)?;
    let session = runtime.open_project_session(OpenProjectSession::from_path(args.project)?)?;
    let job = runtime.start_export(session.handle, args.output, args.preset)?;
    runtime.wait_for_export(job.handle)?;
    Ok(())
}
```

### Anti-Patterns to Avoid

- **Adapter-owned session registry:** Duplicates semantics across Node/C/server and violates D-01/D-02. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md]
- **C ABI directly calling `bindings_node` helpers:** Couples portable runtime to Node-API JSON/error envelopes. [VERIFIED: crates/bindings_node/src/lib.rs]
- **Server CLI reimplementing export:** Risks divergent `.veproj` resolution, render graph, FFmpeg compile, progress, cancellation, and validation behavior. [VERIFIED: crates/bindings_node/src/preview_export_service.rs]
- **Raw frame bytes as product path:** Violates low-copy and no-fallback decisions when a handle path exists. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md]
- **GC-driven handle lifetime:** Violates explicit release and Rust-owned lifetime authority. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md]
- **Desktop-only texture assumptions in mobile contracts:** Texture backend/device identity must remain explicit. [VERIFIED: crates/media_runtime/src/texture.rs]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| C header drift | Manual `.h` files | `cbindgen` 0.29.4 | Generated headers protect ABI drift and package legitimacy is OK. [VERIFIED: crates.io] |
| Adapter-specific project sessions | Separate Node/C/server registries | Shared `editor_runtime::ProjectSessionService` | Locked decisions require one Rust semantic owner. [VERIFIED: 18-CONTEXT.md] |
| Frame/texture lifecycle | JS/C-side maps or GC finalizers | Rust `HandleRegistry` based on `FramePool`/`NativeTextureLeaseRegistry` | Existing code already validates owner, generation, lease release, and leaks. [VERIFIED: crates/media_runtime/src/frame.rs] |
| Server export orchestration | CLI-specific render/export pipeline | Shared `ExportService` over `project_store`, `render_graph`, `ffmpeg_compiler`, `task_runtime`, `media_runtime` | Current export semantics are already multi-stage and should not be duplicated. [VERIFIED: crates/bindings_node/src/preview_export_service.rs] |
| Mobile lifecycle semantics | Platform-specific reinterpretation later | `docs/mobile-runtime-contracts.md` plus ABI smoke tests now | Phase requires lifecycle, permissions, file, texture, cancellation, and close contracts before full apps. [VERIFIED: .planning/ROADMAP.md] |
| Fallback product evidence | CPU/artifact/mock proof paths | Fail-closed diagnostics and no-product-fallback guards | Product success cannot be fallback/mock/artifact/CPU evidence. [VERIFIED: docs/no-product-fallback-policy.md] |

**Key insight:** Phase 18 is about moving authority below adapters; adding more bindings before that split creates multiple semantic owners and makes BIND-01/BIND-05 harder to verify. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md]

## Runtime State Inventory

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | Test-result `.veproj/project.json` files exist under `test-results/phase15-3`, but no runtime session/handle state is stored in canonical `.veproj/project.json`. [VERIFIED: find project.json] [VERIFIED: crates/project_store/src/bundle.rs] | No data migration unless implementation changes draft schema; Phase 18 should keep handles derived/runtime-only. [VERIFIED: AGENTS.md] |
| Live service config | None found; no external live service stores binding/session state for this phase. [VERIFIED: rg service/config scope] | None. [VERIFIED: rg service/config scope] |
| OS-registered state | None found; native addon is loaded from app/native/resources paths, not OS registration. [VERIFIED: apps/desktop-electron/src/main/nativeBinding.ts] | None; regenerate app-native artifacts after N-API split. [VERIFIED: find native artifacts] |
| Secrets/env vars | `VE_NATIVE_BINDING_PATH`, `VE_TEXT_FONT_PATH`, and many `VIDEO_EDITOR_TEST_*` env vars exist; no secret files were found. [VERIFIED: rg env vars] [VERIFIED: find .env/secret] | Do not add production dependence on test env vars; add guards that server/C paths do not use test mocks or product fallback envs. [VERIFIED: scripts/no-product-fallback-guards.sh] |
| Build artifacts | `apps/desktop-electron/native/index.darwin-arm64.node`, `index.cjs`, `index.d.ts`, and `target/debug/libbindings_node.*` exist. [VERIFIED: find native/build artifacts] | Regenerate N-API addon and generated `.d.ts`; add generated C header drift gate for `bindings_c`. [VERIFIED: apps/desktop-electron/package.json] |

**Nothing found in category:** Live service config and OS-registered state have no runtime systems to migrate beyond regenerated local build artifacts. [VERIFIED: local inventory commands]

## Common Pitfalls

### Pitfall 1: Wrapping `bindings_node` Instead Of Splitting It

**What goes wrong:** C/server adapters end up calling Node-oriented serde envelopes or duplicating project/export behavior. [VERIFIED: crates/bindings_node/src/lib.rs]
**Why it happens:** The current registry and export service live in `bindings_node`. [VERIFIED: crates/bindings_node/src/project_session_service.rs] [VERIFIED: crates/bindings_node/src/preview_export_service.rs]
**How to avoid:** Move shared services first, then make Node/C/server adapters call them. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md]
**Warning signs:** `bindings_c` depends on `bindings_node`, server tests import `bindings_node`, or C ABI request types mirror Node `serde_json::Value` envelopes. [ASSUMED]

### Pitfall 2: Handles Without Owner/Generation Checks

**What goes wrong:** Stale, wrong-owner, wrong-device, or double-release handles can access the wrong resource or mask use-after-free. [VERIFIED: crates/media_runtime/src/texture.rs]
**Why it happens:** Callers may treat opaque IDs as simple integers instead of validated leases. [ASSUMED]
**How to avoid:** Make every resolve/release path validate kind, owner session, generation, device, ref/lease count, and terminal state. [VERIFIED: .planning/REQUIREMENTS.md]
**Warning signs:** `HashMap<u64, Resource>` without generation, owner, or release diagnostics. [ASSUMED]

### Pitfall 3: Header Drift

**What goes wrong:** The C header no longer matches exported Rust functions or struct layout. [CITED: https://github.com/mozilla/cbindgen]
**Why it happens:** Manual headers or generated headers are not checked in drift gates. [ASSUMED]
**How to avoid:** Generate the header with pinned `cbindgen` and fail the phase gate on dirty header diffs. [VERIFIED: package-legitimacy check]
**Warning signs:** C smoke tests include local declarations instead of the generated project header. [ASSUMED]

### Pitfall 4: Server Export Without Product Evidence

**What goes wrong:** Server path passes by parsing `.veproj` but never proves FFmpeg export/progress/validation. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md]
**Why it happens:** Metadata parsing is easier than sharing export runtime ownership. [ASSUMED]
**How to avoid:** Use real `.veproj` fixtures and assert output file, duration/fps/dimensions/audio, progress, cancellation, and no Electron dependency. [VERIFIED: docs/product-e2e-acceptance-policy.md]
**Warning signs:** Server tests only assert project title/material count. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md]

### Pitfall 5: Mobile Contracts That Ignore Platform Lifecycles

**What goes wrong:** Future JNI/Swift bindings reinterpret session close, backgrounding, file permission revocation, and texture ownership differently. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md]
**Why it happens:** Desktop paths have process-stable files and resources, while mobile apps have sandbox permissions and lifecycle interruptions. [CITED: https://developer.android.com/training/articles/perf-jni]
**How to avoid:** Document background/foreground, permission invalidation, file-handle lifetime, texture/device lifetime, cancellation, and cascading close now. [VERIFIED: .planning/ROADMAP.md]
**Warning signs:** Mobile contract docs only list function names and omit lifecycle/error behavior. [ASSUMED]

## Code Examples

Verified patterns from official sources and the existing codebase:

### Rust 2024 C Export Shape

```rust
// Source: https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-attributes.html
// Source: https://doc.rust-lang.org/nomicon/ffi.html
#[repr(C)]
pub struct ve_runtime_config_t {
    pub flags: u32,
}

#[unsafe(no_mangle)]
pub extern "C" fn ve_runtime_create(config: ve_runtime_config_t, out: *mut ve_handle_t) -> ve_status_t {
    bindings_c::runtime_create(config, out)
}
```

### Handle Resolve Must Validate Owner And Generation

```rust
// Source: crates/media_runtime/src/texture.rs
fn validate_handle(expected: &TextureHandle, registered: &TextureHandle) -> Result<(), RuntimeError> {
    ensure!(expected.owner_session == registered.owner_session, RuntimeErrorKind::OwnerSessionMismatch);
    ensure!(expected.generation == registered.generation, RuntimeErrorKind::StaleGeneration);
    ensure!(expected.device_id == registered.device_id, RuntimeErrorKind::DeviceMismatch);
    Ok(())
}
```

### Server Export Should Call Shared Service

```rust
// Source: crates/bindings_node/src/preview_export_service.rs
let session = runtime.open_project_session(OpenProjectSession { bundle_path })?;
let job = runtime.start_project_session_export(StartProjectSessionExport {
    session: session.handle,
    output_path,
    preset,
})?;
runtime.wait_for_export(job.handle)?;
```

### Header Generation Gate

```bash
# Source: https://github.com/mozilla/cbindgen
cbindgen --config crates/bindings_c/cbindgen.toml --crate bindings_c --output crates/bindings_c/include/video_editor_runtime.h
git diff --exit-code crates/bindings_c/include/video_editor_runtime.h
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Generic public `executeCommand` from desktop shell | Explicit native APIs for project sessions, audio, artifact, export, and realtime preview | Prior phases through 17.1 | Phase 18 should preserve explicit APIs and avoid generic adapter envelopes. [VERIFIED: .planning/STATE.md] |
| Renderer-held canonical draft and renderer-derived preview/export payloads | Rust project sessions emit view models and snapshots by session ID/revision | Prior phases through 17.1 | Shared runtime must preserve Rust-owned draft/session state. [VERIFIED: .planning/STATE.md] |
| Binding-owned scheduler/policy for preview/export/audio/artifacts | `task_runtime` provides scheduler/cancellation/freshness/telemetry contracts | Phase 16 | Server runtime should reuse scheduler contracts. [VERIFIED: package.json] |
| CPU/artifact/mock preview evidence | Native renderGraph GPU compositor evidence for product preview | Phases 15.2/17.1 | Phase 18 low-copy handles must not regress to artifact/CPU success. [VERIFIED: docs/no-product-fallback-policy.md] |
| Desktop-only runtime path | Planned portable Node/C/server/mobile contract split | Phase 18 | Shared runtime layer is now required before Phase 19 effects expansion. [VERIFIED: .planning/ROADMAP.md] |

**Deprecated/outdated:**

- Public generic desktop command envelopes for product operations are outdated; explicit APIs and source guards replaced them. [VERIFIED: .planning/STATE.md]
- Product success through fallback/mock/artifact/CPU paths is forbidden. [VERIFIED: docs/no-product-fallback-policy.md]
- Keeping legacy partial boundaries by wrapping them is forbidden for this greenfield product unless explicitly requested. [VERIFIED: docs/refactor-and-legacy-cleanup-policy.md]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | New shared crate should be named `editor_runtime`. | Summary / Project Structure | Low; planner can rename while preserving boundary. |
| A2 | New server crate/bin should be named `server_runtime`. | Standard Stack / Project Structure | Low; planner can choose another aligned name. |
| A3 | Handle public token shape can use numeric `kind/id/owner_session/generation` fields. | Architecture Patterns / Code Examples | Medium; implementation may prefer string IDs or packed values, but validation requirements remain. |
| A4 | Server runtime uses `media_runtime_desktop` as the first FFmpeg executor adapter in Phase 18. | Resolved Decisions | Low; split only if implementation proves desktop naming leaks product semantics. |
| A5 | C ABI exposes stable typed status/error/handle structs plus bounded JSON/string buffers for complex diagnostics and project/export payloads. | Resolved Decisions | Low; raw `serde_json` ownership cannot cross the ABI. |

## Open Questions (RESOLVED)

1. **Server FFmpeg executor naming:** RESOLVED — Server runtime uses `media_runtime_desktop` as the first FFmpeg executor adapter in Phase 18. Do not create a separate `media_runtime_server` unless implementation proves desktop naming leaks product semantics into the server API or dependency graph. [VERIFIED: docs/runtime-boundaries.md]

2. **C ABI response shape:** RESOLVED — The C ABI uses stable typed status/error/handle structs plus bounded JSON/string buffers for complex diagnostics and project/export payloads. No raw `serde_json::Value`, Rust allocation ownership, or serde-owned response object crosses the ABI boundary. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md]

3. **`@napi-rs/cli` changes:** RESOLVED — Do not upgrade or reinstall `@napi-rs/cli` in Phase 18 unless a blocking checkpoint verifies package metadata. Current Phase 18 plans avoid changing it. [VERIFIED: npm registry] [VERIFIED: package-legitimacy check]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust toolchain | Workspace, C ABI, server runtime | ✓ | rustc 1.95.0 / cargo 1.95.0 | None needed. [VERIFIED: rustc --version] |
| Node.js | Desktop build/test | ✓ | v24.15.0 | Existing warning only; package expects 24.12.0. [VERIFIED: node --version] [VERIFIED: package.json] |
| pnpm | Desktop build/test | ✓ | 10.32.1 | None needed. [VERIFIED: pnpm --version] |
| npm | Registry/package verification | ✓ | 11.12.1 | None needed. [VERIFIED: npm --version] |
| `cbindgen` CLI | C header generation | Project-local bootstrap planned | 0.29.4 | `scripts/phase18-abi-drift.sh` installs or reuses a deterministic repo-local pinned binary and fails if the resolved version is not exactly 0.29.4; no global manual setup required. [VERIFIED: command -v cbindgen] |
| Bundled FFmpeg/ffprobe | Desktop/server export smoke | ✓ | `apps/desktop-electron/runtime/ffmpeg/darwin-arm64` exists | Do not use PATH as product fallback. [VERIFIED: find runtime/ffmpeg] [VERIFIED: docs/runtime-boundaries.md] |
| PATH FFmpeg/ffprobe | Diagnostics only | ✓ | 8.1.2 | Product/server gates should use configured bundled runtime, not PATH fallback. [VERIFIED: ffmpeg -version] [VERIFIED: docs/runtime-boundaries.md] |

**Missing dependencies with no fallback:**

- None. `cbindgen` is absent globally, but Phase 18 plans require `scripts/phase18-abi-drift.sh` to bootstrap a project-local pinned 0.29.4 binary before header generation gates run. [VERIFIED: command -v cbindgen]

**Missing dependencies with fallback:**

- None for required Phase 18 gates. [VERIFIED: environment audit]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` 1.95.0, Playwright 1.61.0, shell source guards. [VERIFIED: cargo --version] [VERIFIED: apps/desktop-electron/package.json] |
| Config file | `package.json` scripts plus existing crate integration tests; no Phase 18 scripts exist yet. [VERIFIED: package.json] |
| Quick run command | `cargo test -p media_runtime frame_pool -- --nocapture && cargo test -p task_runtime scheduler_contracts -- --nocapture` [VERIFIED: package.json] |
| Full suite command | `pnpm run test:phase18` after Wave 0 creates it. [ASSUMED] |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| PLAT-01 | C ABI exposes runtime/session/handle open/release/error contracts. [VERIFIED: .planning/REQUIREMENTS.md] | ABI smoke | `cargo test -p bindings_c --test abi_smoke -- --nocapture` | ❌ Wave 0 |
| PLAT-02 | Server renders `.veproj` without Electron. [VERIFIED: .planning/REQUIREMENTS.md] | integration/export smoke | `cargo test -p server_runtime --test server_export_smoke -- --nocapture` | ❌ Wave 0 |
| PLAT-03 | JNI/Swift contracts cover lifecycle/file/texture/cancel/close and smoke-level handles. [VERIFIED: .planning/REQUIREMENTS.md] | docs + contract smoke | `cargo test -p bindings_c --test mobile_contract_handles -- --nocapture && bash scripts/phase18-mobile-contract-guards.sh` | ❌ Wave 0 |
| BIND-01 | Node/C/server adapters do not duplicate draft/project/export semantics. [VERIFIED: .planning/REQUIREMENTS.md] | source guard | `bash scripts/phase18-source-guards.sh` | ❌ Wave 0 |
| BIND-02 | Handles validate owner, generation, ref/lease count, release, cascading close, leaks. [VERIFIED: .planning/REQUIREMENTS.md] | unit | `cargo test -p editor_runtime --test handle_registry -- --nocapture` | ❌ Wave 0 |
| BIND-03 | Large frames/preview outputs use handles and reject unnecessary byte/artifact product paths. [VERIFIED: .planning/REQUIREMENTS.md] | unit + guard | `cargo test -p editor_runtime --test handle_registry -- --nocapture && pnpm run test:no-product-fallback` | ❌ Wave 0 / ✅ existing guard |
| BIND-04 | Server open/resolve/export/progress/cancel works without Electron. [VERIFIED: .planning/REQUIREMENTS.md] | integration | `cargo test -p server_runtime --test server_export_smoke -- --nocapture` | ❌ Wave 0 |
| BIND-05 | ABI/header/schema/binding drift is caught. [VERIFIED: .planning/REQUIREMENTS.md] | drift + smoke | `pnpm run test:contracts && bash scripts/phase18-abi-drift.sh && cargo test -p bindings_node --test binding_smoke -- --nocapture` | ❌ Wave 0 / ✅ partial existing |

### Sampling Rate

- **Per task commit:** `cargo check --workspace --locked` plus the affected crate test. [VERIFIED: package.json]
- **Per wave merge:** `pnpm run test:phase18` after Wave 0 creates it. [ASSUMED]
- **Phase gate:** `pnpm run test:phase18 && pnpm run test:no-product-fallback && pnpm run test:contracts`. [ASSUMED] [VERIFIED: package.json]

### Wave 0 Gaps

- [ ] `crates/editor_runtime/` — shared runtime/session/export/handle service crate. [ASSUMED]
- [ ] `crates/bindings_c/` — C ABI crate with `cdylib`/`staticlib`, generated header, and C smoke tests. [ASSUMED]
- [ ] `crates/server_runtime/` — Electron-free server export runtime and bin target. [ASSUMED]
- [ ] `docs/mobile-runtime-contracts.md` — JNI/Swift lifecycle/file/texture/cancellation contract. [ASSUMED]
- [ ] `scripts/phase18-source-guards.sh` — semantic duplication/fallback/adapter ownership guard. [ASSUMED]
- [ ] `scripts/phase18-abi-drift.sh` — cbindgen header regeneration and diff guard. [ASSUMED]
- [ ] `package.json` scripts `test:phase18-rust`, `test:phase18-source-guards`, `test:phase18-abi`, `test:phase18-server`, and `test:phase18`. [ASSUMED]
- [ ] `scripts/phase18-abi-drift.sh` bootstraps a deterministic project-local `cbindgen` 0.29.4 binary and fails if the resolved version differs. [VERIFIED: command -v cbindgen]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | No user authentication is in Phase 18 scope. [VERIFIED: .planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md] |
| V3 Session Management | yes | Runtime/project sessions must be opaque, owner-bound, generation-checked, explicitly closed, and cascade-release resources. [VERIFIED: .planning/REQUIREMENTS.md] |
| V4 Access Control | yes | Handle resolution must reject wrong owner, wrong device, stale generation, unknown handle, and expired lease. [VERIFIED: .planning/REQUIREMENTS.md] |
| V5 Input Validation | yes | C ABI buffers, paths, JSON payloads, `.veproj` material URIs, output paths, and IPC sender URLs require validation. [VERIFIED: crates/project_store/src/bundle.rs] [VERIFIED: apps/desktop-electron/src/main/index.ts] |
| V6 Cryptography | no | No new cryptographic primitive is required; do not hand-roll crypto if future signing/checksums are added. [ASSUMED] |

### Known Threat Patterns for Rust/Electron/FFI Runtime

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Stale or fabricated handle token | Elevation of Privilege / Tampering | Rust-owned handle registry with owner session, generation, kind, ref/lease count, and typed diagnostics. [VERIFIED: crates/media_runtime/src/texture.rs] |
| Use-after-release or double release across C/JNI/Swift | Tampering / Denial of Service | Explicit release returns typed error on unknown/double release and session close emits leak diagnostics. [VERIFIED: crates/media_runtime/src/frame.rs] |
| Path traversal or machine-local material dependency in server export | Tampering / Information Disclosure | Use `project_store` material URI classification and bundle-relative resolution. [VERIFIED: crates/project_store/src/bundle.rs] |
| Electron IPC from untrusted renderer | Spoofing | Keep `assertAllowedIpcSender` on desktop IPC. [VERIFIED: apps/desktop-electron/src/main/index.ts] |
| Product fallback/mock success through test env vars | Tampering / Repudiation | Run no-product-fallback and Phase 18 source guards. [VERIFIED: scripts/no-product-fallback-guards.sh] |
| C ABI buffer overflow or unterminated strings | Tampering / Denial of Service | Use bounded buffers, required-length returns, UTF-8 validation, and explicit string free APIs. [CITED: https://doc.rust-lang.org/nomicon/ffi.html] |
| JNI `JNIEnv` used on wrong thread | Denial of Service | Document future JNI attach/thread rules and keep Phase 18 JNI as contract only. [CITED: https://developer.android.com/training/articles/perf-jni] |

## Sources

### Primary (HIGH confidence)

- `AGENTS.md` — project architecture, no fallback, destructive refactor, Rust-owned semantics, `.veproj` source of truth. [VERIFIED: AGENTS.md]
- `.planning/phases/18-mobile-server-binding-architecture-and-runtime-ports/18-CONTEXT.md` — locked Phase 18 decisions, discretion, deferred scope. [VERIFIED: 18-CONTEXT.md]
- `.planning/REQUIREMENTS.md` and `.planning/ROADMAP.md` — PLAT/BIND requirement text and success criteria. [VERIFIED: .planning/REQUIREMENTS.md] [VERIFIED: .planning/ROADMAP.md]
- `crates/bindings_node/src/lib.rs` — current Node-API entry surface. [VERIFIED: codebase grep]
- `crates/bindings_node/src/project_session_service.rs` — current project-session registry/lifecycle/scheduler ownership. [VERIFIED: codebase grep]
- `crates/bindings_node/src/preview_export_service.rs` — current export registry/scheduler/render graph/FFmpeg compile ownership. [VERIFIED: codebase grep]
- `crates/media_runtime/src/frame.rs` and `crates/media_runtime/src/texture.rs` — reusable handle/lease validation and leak diagnostics. [VERIFIED: codebase grep]
- `crates/realtime_preview_runtime/src/media_io_adapter.rs` — low-copy/fallback rejection behavior. [VERIFIED: codebase grep]
- `crates/project_store/src/bundle.rs` and `crates/task_runtime/src/scheduler.rs` — `.veproj` and scheduler foundations. [VERIFIED: codebase grep]

### Secondary (MEDIUM confidence)

- Rust Reference linkage and layout docs — `cdylib`, `staticlib`, `repr(C)` C layout. [CITED: https://doc.rust-lang.org/reference/linkage.html] [CITED: https://doc.rust-lang.org/reference/type-layout.html]
- Rust Nomicon FFI — extern C and ownership examples. [CITED: https://doc.rust-lang.org/nomicon/ffi.html]
- Rust 2024 unsafe attributes guide — `no_mangle`/`export_name` as unsafe attributes. [CITED: https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-attributes.html]
- Cargo targets docs — packages can have library and binary targets. [CITED: https://doc.rust-lang.org/cargo/reference/cargo-targets.html]
- cbindgen official repo/docs — header generation from Rust crates. [CITED: https://github.com/mozilla/cbindgen]
- NAPI-RS docs — `#[napi]` and CLI build workflow. [CITED: https://napi.rs/docs/introduction/getting-started] [CITED: https://napi.rs/docs/cli/build]
- Android JNI tips — `JavaVM`/`JNIEnv`, native threads, and lifecycle constraints. [CITED: https://developer.android.com/training/articles/perf-jni]
- Swift C library usability/interoperability guidance. [CITED: https://swift.org/blog/improving-usability-of-c-libraries-in-swift/]

### Tertiary (LOW confidence)

- Proposed crate names and exact C response shape are recommendations under agent discretion. [ASSUMED]

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH for existing repo crates and Rust/NAPI versions; MEDIUM for `cbindgen` integration; LOW for proposed crate names. [VERIFIED: Cargo.toml] [VERIFIED: crates.io] [ASSUMED]
- Architecture: HIGH for ownership gaps and required target boundary because they are locked by context and verified in code. [VERIFIED: 18-CONTEXT.md] [VERIFIED: codebase grep]
- Pitfalls: HIGH for adapter/session/export ownership pitfalls; MEDIUM for C/JNI/Swift FFI details from official docs. [VERIFIED: codebase grep] [CITED: https://doc.rust-lang.org/nomicon/ffi.html]

**Research date:** 2026-06-25
**Valid until:** 2026-07-25 for architecture; re-check package versions and `@napi-rs/cli` legitimacy before installation or upgrade. [VERIFIED: package-legitimacy check]

# Phase 18: Mobile/Server Binding Architecture And Runtime Ports - Context

**Gathered:** 2026-06-25T07:16:00+08:00
**Status:** Ready for planning
**Mode:** Auto discussion from user-approved GSD continuation

<domain>
## Phase Boundary

Phase 18 turns the desktop-first Rust core into a portable runtime surface. The phase must separate desktop Node-API, a portable C ABI, future JNI/Swift contracts, and server entrypoints without duplicating draft, timeline, preview, render, media, export, scheduler, or project-store semantics.

This phase is architecture and runtime plumbing, not mobile product UI. Full iOS/Android applications remain deferred. Server runtime is in scope as a real first-party runtime path that can open `.veproj`, resolve materials, run render/export jobs, and report progress without Electron.

</domain>

<decisions>
## Implementation Decisions

### Binding Ownership

- **D-01:** Rust core must expose a shared production API below `bindings_node`; Node-API, C ABI, future JNI/Swift, and server entrypoints are adapters over that shared Rust API, not independent semantic implementations.
- **D-02:** `bindings_node` should become a thin desktop adapter for JSON/Node-API transport, Electron IPC, and desktop resource wiring. It must not own editing semantics, project lifecycle semantics, preview scheduling policy, render/export behavior, or resource lifetime policy.
- **D-03:** If an existing boundary in `bindings_node` is structurally wrong for portable ownership, replace it destructively instead of preserving compatibility. The project is not launched; old partial paths should be removed or guarded, not wrapped as a fallback success path.

### Runtime And Resource Handles

- **D-04:** Runtime sessions, project sessions, media handles, frame handles, texture handles, and artifact handles must be represented as opaque Rust-owned IDs with owner session, generation, reference count or lease count, explicit release, cascading close, and debug leak diagnostics.
- **D-05:** Stale generation, wrong owner, wrong device, expired lease, double release, and unknown handle cases must fail closed with typed diagnostics. They must not silently fall back to byte copies, mock resources, artifact previews, or Electron-owned state.
- **D-06:** Handle lifetime authority belongs in Rust. JavaScript, C callers, future JNI, and future Swift callers may hold opaque tokens only; they cannot fabricate handles, mutate handle metadata, or infer lifetime from object garbage collection.

### Low-Copy Media And Preview Boundary

- **D-07:** Large media frames and preview outputs should cross language/runtime boundaries through handles whenever a handle path is available. Raw byte transfer is acceptable only for explicit diagnostic/test surfaces or unsupported paths that are reported as failures/degradations, not as product success.
- **D-08:** Native texture/device identity must remain explicit: backend, adapter/device ID, owner session, generation, dimensions, pixel format, and color metadata must be verified before import or presentation.
- **D-09:** Preview and render evidence must continue to follow the no-product-fallback rule. CPU readback, artifact output, DOM evidence, mock surfaces, or debug probes do not prove a production preview/render path.

### C ABI And Mobile Contracts

- **D-10:** Add a contract-first portable C ABI surface for runtime/session/handle operations with stable error codes, explicit ownership, and smoke tests. The C ABI should call the shared Rust API and should not duplicate Node-API request handling logic.
- **D-11:** JNI and Swift/ObjC are represented in Phase 18 as contract documents, headers/type maps, and smoke-level handle/session tests. Full Android/iOS app shells, UI, packaging, permissions UX, and store deployment are out of scope.
- **D-12:** Mobile lifecycle contracts must cover app background/foreground, sandboxed media permissions, file handles, texture/device handles, cancellation, and session close semantics so future mobile apps do not reinterpret desktop behavior.

### Server Runtime

- **D-13:** Server runtime must be a first-party Rust runtime surface that opens `.veproj/project.json` through `project_store`, resolves bundle-relative materials, compiles render/export jobs through the shared render/export path, and reports progress/cancellation/errors without Electron.
- **D-14:** Server runtime should not depend on Electron, BrowserWindow, preload IPC, DOM state, desktop UI view models, or desktop-only native surface handles.
- **D-15:** Server runtime verification must use real `.veproj` fixtures and export/progress evidence. A CLI that only prints parsed project metadata is insufficient for Phase 18 success.

### Verification And Gates

- **D-16:** Phase 18 plans must include source guards that fail if semantics are duplicated across Node/C/server adapters, if renderer/main constructs render/export behavior, or if product success can be satisfied by fallback/mock/artifact evidence.
- **D-17:** Phase 18 verification should include Rust unit/integration tests, ABI/header/schema drift checks, C ABI smoke tests, server export smoke tests, Node-API desktop smoke tests, and existing contract gates.
- **D-18:** UI design work is not a primary Phase 18 deliverable. If any desktop UI changes are required to expose runtime diagnostics, an independent UI/design review subagent should audit them for product quality and can make production-grade layout/style corrections.

### the agent's Discretion

- Planner/researcher may choose the exact crate names and file splits, but should prefer names aligned with the established stack: `bindings_node`, later `bindings_c`, portable runtime/session/handle APIs, `project_store`, `media_runtime`, `realtime_preview_runtime`, `task_runtime`, `render_graph`, `ffmpeg_compiler`, and `media_runtime_desktop`.
- Planner may split the phase into more plans if the shared API, C ABI, server runtime, handle registry, platform contracts, and aggregate gates cannot be implemented safely in a smaller wave count.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Direction And Constraints

- `AGENTS.md` — Project architecture, production refactor policy, no fallback-as-success, Rust-owned semantics, and planned binding crates.
- `.planning/PROJECT.md` — Core value, no product fallback, no legacy compatibility by default, and production E2E acceptance decisions.
- `.planning/ROADMAP.md` §Phase 18 — Phase goal, requirements, and success criteria.
- `.planning/REQUIREMENTS.md` §BIND-01..BIND-05 and §PLAT-01..PLAT-03 — Binding and platform requirements.
- `docs/refactor-and-legacy-cleanup-policy.md` — Required posture for destructive refactors and legacy path removal.
- `docs/no-product-fallback-policy.md` — Product success cannot be proven by fallback/mock/artifact/CPU/debug paths.
- `docs/product-e2e-acceptance-policy.md` — Product-facing features require real workflow/product evidence.
- `docs/runtime-boundaries.md` — Existing runtime responsibility boundaries; Phase 18 should update or supersede any boundary that conflicts with portable runtime ownership.

### Prior Phase Decisions To Preserve

- `.planning/phases/17.1-interaction-session-and-template-import-main-chain-hardening/17.1-02-SUMMARY.md` — Rust-owned interaction session lifecycle and explicit IPC path.
- `.planning/phases/17.1-interaction-session-and-template-import-main-chain-hardening/17.1-06-SUMMARY.md` — Aggregate high-frequency interaction/source guard pattern.
- `.planning/phases/17.1-interaction-session-and-template-import-main-chain-hardening/17.1-07-SUMMARY.md` — Native compositor evidence and absolute provisional update decisions.

### Current Implementation Anchors

- `crates/bindings_node/src/lib.rs` — Current Node-API entry surface and legacy command restrictions.
- `crates/bindings_node/src/project_session_service.rs` — Current desktop project session registry, session lifecycle, template import, interactions, and export entrypoints.
- `apps/desktop-electron/src/main/nativeBinding.ts` — Desktop TypeScript binding adapter and generated/handwritten request types.
- `apps/desktop-electron/src/preload/index.ts` — Electron preload IPC exposure.
- `crates/media_runtime/src/frame.rs` — Existing `FramePool`, frame lease, owner session, generation, release, and leak diagnostics.
- `crates/media_runtime/src/texture.rs` — Existing `TextureHandle` and native texture lease validation.
- `crates/realtime_preview_runtime/src/media_io_adapter.rs` — Existing preview media IO handoff and pending frame release behavior.
- `crates/project_store/src/bundle.rs` — `.veproj` open/save bundle source of truth.
- `crates/task_runtime/src/scheduler.rs` — Existing scheduler/admission/cancellation behavior to reuse for server jobs.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- `media_runtime::FramePool` already models owner session, generation, lease release, outstanding lease limit, and close-time leak diagnostics. Phase 18 should generalize this pattern rather than invent JS-owned lifecycle tracking.
- `media_runtime::TextureHandle` and `NativeTextureLeaseRegistry` already validate owner session, generation, backend, device identity, dimensions, and pixel format. This is the right basis for portable frame/texture handle contracts.
- `ProjectInteractionSession` from Phase 17.1 proves the repo pattern for Rust-owned session state with expected revision, generation, monotonic sequence, cancel, and one-shot commit.
- `task_runtime` already provides scheduler, freshness, cancellation, telemetry, and resource classes. Server export/runtime entrypoints should reuse it instead of building ad hoc job threads.
- `project_store` and `.veproj/project.json` remain the canonical project source. Server runtime must use the same store path as desktop open/save/export.

### Established Patterns

- Explicit native APIs replaced generic `executeCommand` for product operations. Phase 18 should keep explicit transport methods for Node/C/server instead of expanding generic envelopes.
- Source guards are used as architecture gates. Phase 18 should add guards for duplicated semantics, renderer/main render behavior, fallback success, and adapter-owned lifetime policy.
- Product evidence requires real compositor/export/runtime facts, not DOM or artifact evidence.

### Integration Points

- `bindings_node` currently exposes many N-API functions directly and owns an in-process `ProjectSessionRegistry` guarded by `Mutex<...>`. The planner should decide whether to move registry/lifecycle ownership into a shared portable runtime crate before adding C/server surfaces.
- `apps/desktop-electron/src/main/nativeBinding.ts` and preload IPC should continue as desktop transport adapters after the Rust API is separated.
- Server runtime should connect through `project_store`, `render_graph`, `ffmpeg_compiler`, `media_runtime`, `media_runtime_desktop`, and `task_runtime`, without Electron.
- C ABI should bind to the same shared Rust runtime/session API used by Node and server, with ABI tests preventing drift.

</code_context>

<specifics>
## Specific Ideas

- The user explicitly allows destructive refactors because the project is not launched.
- The user wants the best production architecture, not compatibility preservation.
- Normal errors should be typed failures with diagnostics, not fallback-as-success or hidden containment.
- Testing should be broad and realistic; acceptance should cover multiple scenarios, not only narrow unit cases.
- If UI/style changes occur, assign an independent UI/design review subagent and allow it to improve layout/style beyond the narrow code change when product quality is weak.

</specifics>

<deferred>
## Deferred Ideas

- Full iOS and Android applications, mobile UI, platform permission UX, app packaging, and store readiness are deferred beyond Phase 18.
- Phase 19 owns production effects, retiming, transition semantics, masks, filters, and complex template fidelity after portable runtime/binding foundations are in place.
- Cloud multi-tenant rendering, job queue service deployment, authentication, billing, and remote storage synchronization are not Phase 18 scope unless represented only as local server-runtime extension points.

</deferred>

---

*Phase: 18-Mobile/Server Binding Architecture And Runtime Ports*
*Context gathered: 2026-06-25T07:16:00+08:00*

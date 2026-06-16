# Phase 1: Foundation And Golden Harness - Context

**Gathered:** 2026-06-17
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase creates the buildable foundation for a new Jianying-style desktop video editor: Rust/Electron workspace scaffolding, a typed Electron-to-Rust binding path, FFmpeg/ffprobe discovery, cross-platform service-boundary abstractions, deterministic fixture structure, and a required tiny render smoke gate. It does not implement real draft editing semantics, rich UI, packaged FFmpeg distribution, mobile backends, or export presets.

</domain>

<decisions>
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Direction
- `.planning/PROJECT.md` - Product identity, out-of-scope boundaries, terminology constraints, and architecture constraints.
- `.planning/REQUIREMENTS.md` - Phase 1 requirements `FOUND-01` through `FOUND-04` and `TEST-01`.
- `.planning/ROADMAP.md` - Phase 1 goal, success criteria, and planned work slices.

### Research
- `.planning/research/SUMMARY.md` - Current stack, architecture, terminology, testing, and MVP summary.
- `.planning/research/STACK.md` - Rust/Electron stack recommendation, planned crate layout, and version policy notes.
- `.planning/research/ARCHITECTURE.md` - Semantic spine, repository layout, layer responsibilities, Kdenlive/MLT/Jianying lessons.
- `.planning/research/PITFALLS.md` - Duplicate state, UI-owned semantics, terminology drift, time bugs, preview/export drift, FFmpeg leakage, and licensing risk notes.

### Source Guideline
- `AI_Video_Editing_Single_Engine_Guideline.md` - Layered single-engine guideline. Apply the user correction that this project is a general video editor, not an oral-video/AI talking-head product.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- No application source code exists yet. The reusable assets are planning artifacts and local references under `reference/`, especially Kdenlive, MLT, and pyJianYingDraft.

### Established Patterns
- This is a new project. The first implementation must establish monorepo, Rust workspace, binding, schema generation, fixture, and test patterns rather than adapt existing code.
- GSD workflow is active. Planning and implementation should update `.planning/` artifacts and commit scoped changes.

### Integration Points
- New code connects through the planned repository roots: `apps/desktop-electron`, `crates/*`, `schemas/`, `fixtures/`, `goldens/`, `tools/`, and `docs/`.
- The first Rust/Electron integration point is the Node-API binding crate exposing `ping`, `version`, and a typed command envelope.

</code_context>

<specifics>
## Specific Ideas

- The project should follow a cross-platform pattern of interface/trait abstraction plus platform-specific backends, but only at service boundaries. Do not inject a giant `PlatformSystem` into pure semantic crates.
- The intended future platform split is desktop Electron first, with iOS/Android/server later. Phase 1 should make future backend replacement visible without implementing mobile backends.
- User explicitly selected not to handle FFmpeg license posture in Phase 1 because no FFmpeg binary is bundled, downloaded, or redistributed in this phase.

</specifics>

<deferred>
## Deferred Ideas

- App-bundled FFmpeg runtime management belongs to a later packaging/release phase.
- FFmpeg distribution and notices are deferred until the project actually distributes FFmpeg binaries.
- `HardwareEncoder` implementation, including NVENC/QSV/VideoToolbox/MediaCodec probing, is deferred until real preview/export work.
- Mobile iOS/Android FFmpeg backends, static library loading, JNI loading, and mobile sandbox file implementations are deferred beyond Phase 1.

</deferred>

---

*Phase: 1-Foundation And Golden Harness*
*Context gathered: 2026-06-17*

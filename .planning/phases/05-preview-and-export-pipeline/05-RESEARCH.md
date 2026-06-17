# Phase 05: Preview And Export Pipeline - Research

**Researched:** 2026-06-17 [VERIFIED: local date/environment]
**Domain:** Rust editing semantics, render graph compilation, FFmpeg preview/export runtime, Electron command integration, golden validation [VERIFIED: .planning/ROADMAP.md; VERIFIED: .planning/phases/05-preview-and-export-pipeline/05-CONTEXT.md]
**Confidence:** HIGH for codebase boundaries and local FFmpeg viability; MEDIUM for cross-host text pixel determinism because fonts and FFmpeg builds vary by machine. [VERIFIED: codebase grep; VERIFIED: local ffmpeg probes; CITED: https://ffmpeg.org/ffmpeg-filters.html]

<user_constraints>
## User Constraints (from CONTEXT.md)

The following constraints are copied verbatim from `.planning/phases/05-preview-and-export-pipeline/05-CONTEXT.md`. [VERIFIED: .planning/phases/05-preview-and-export-pipeline/05-CONTEXT.md]

### Locked Decisions
## Implementation Decisions

### Shared Semantic Path
- Normalize draft semantics in `engine_core` before either preview or export reads the timeline. Track stacking, target/source timerange mapping, muted tracks, material status, segment volume, and text segment semantics must resolve there.
- Frame state evaluation should be deterministic at integer microsecond timeline times and rational frame rates. Persisted semantics must not introduce naked floating-point seconds.
- Preview and export must consume the same normalized draft and resolved frame state path. If preview needs a lightweight representation, it should be a derived view of the same frame state rather than a separate renderer-only model.
- Keep `draft_model` and `draft_commands` pure. Phase 5 code may depend on them from `engine_core`, but filesystem, Electron, FFmpeg process execution, and platform concerns stay outside pure semantic crates.

### Render Graph And FFmpeg Compiler
- `render_graph` should own typed renderer-neutral intents for materials, visual layers, audio mixes, text overlays, filters, transitions, output profile, and time ranges. It must not execute FFmpeg or decide edit behavior.
- `ffmpeg_compiler` should compile render graph intents into a structured `FfmpegJob`: inputs, generated filter script text, generated subtitle/text artifacts if needed, output path, encode settings, and validation metadata.
- UI and Electron code must never construct FFmpeg commands, filter graphs, render graphs, preview cache keys, waveform behavior, or derived scripts directly.
- FFmpeg compiler snapshot tests should use stable ordering and fixture paths so diffs are reviewable. Generated scripts are derived artifacts and must not become `.veproj/project.json` state.

### Preview Service And Cache
- `preview_service` owns preview frame and preview segment requests, cache keys, cache directory layout, and invalidation logic. It should sit above `engine_core`, `render_graph`, `ffmpeg_compiler`, and `media_runtime`, not inside UI.
- Preview frame requests should return deterministic frame artifacts or metadata for a timeline time. MVP can use FFmpeg-generated still frames or small segment renders rather than GPU real-time rendering.
- Preview segment cache invalidation should be range-based and conservative: timeline/text/audio edits invalidate overlapping target ranges and relevant derived caches, while unrelated ranges remain valid.
- Cache outputs, thumbnails, waveforms, render graphs, FFmpeg scripts, and exported videos remain derived artifacts outside the canonical draft.

### Export Runtime And Job Behavior
- Export should expose a Rust-owned command/API path that accepts draft, output path, and preset-like settings, then returns/export streams job state, progress, logs, cancel results, and classified errors.
- Desktop MVP may run FFmpeg through the existing `media_runtime` / `media_runtime_desktop` binary boundary. Mobile/server runtime abstractions remain future backends and must not leak into pure semantic crates.
- Progress can be deterministic and testable for MVP by parsing FFmpeg progress output or by reporting staged progress around compile/run/probe validation, as long as the API shape supports real streaming later.
- Export validation must use ffprobe metadata for file existence, approximate duration, fps, resolution, audio stream presence, and non-empty output.

### Text Layout Determinism
- Phase 5 must pin deterministic MVP text rendering settings. Text/subtitle segments use the existing Jianying-aligned `TextSegment` semantics and should produce stable preview/export output.
- Prefer a local, documented default font and explicit layout assumptions. If a target machine lacks the exact font, the pipeline should report a classified limitation or fall back deterministically rather than silently changing layout.
- MVP text rendering may use generated ASS/subtitle artifacts or FFmpeg drawtext if that keeps preview/export parity testable. The chosen path must be covered by snapshot/golden tests.
- Advanced text bubbles, text effects, proprietary presets, animated keyframes, and effect libraries stay deferred unless represented as no-op/degraded intents.

### Desktop UI Integration
- Desktop visible UI remains Simplified Chinese. Preview/export controls, status, progress, cancel, logs, and errors should use Chinese labels and Jianying-style editing terms.
- The center preview monitor should move from placeholder to a command-driven preview state: seek/scrub requests a deterministic preview frame, short playback uses cached preview segment output, and failure states are visible without corrupting draft state.
- The export surface should be MVP-level and practical: choose output path/preset, start export, show progress/log summary, cancel, and report validation result.
- Renderer code can hold UI state such as playhead and selected export destination, but all semantic preview/export compilation and FFmpeg execution must route through Rust-owned commands or binding APIs.

### Testing And Gates
- Add focused Rust tests for normalization, frame-state snapshots, track stacking, source/target time mapping, text layout determinism, render graph snapshots, FFmpeg job/script snapshots, preview cache invalidation, export validation, and classified runtime errors.
- Add preview/export parity tests for golden drafts. Frame parity may use documented tolerance rather than byte-perfect video comparison.
- Extend public test commands with Phase 5-specific gates and include them in the root `pnpm run test` / `just test` path before Phase 5 is considered complete.
- Keep source guards current: no renderer FFmpeg/render graph/cache construction, no float persisted semantics, no internal `Asset`/`Clip` vocabulary regression, and no derived artifacts in draft fixtures/schema.

### the agent's Discretion
- The planner may choose exact module boundaries, API names, cache key format, and whether preview frame artifacts are PNG/JPEG/metadata-first, as long as the shared semantic path and gates above hold.
- The planner may choose whether the first export API is synchronous in tests and adapted to async job state for Electron, or async from the start, as long as progress/cancel/error contracts are represented and testable.
- The planner may keep desktop UI integration thin if full rich playback is too large, but must still prove seek preview, short cached preview segment, and MP4 export through executable gates.

### Deferred Ideas (OUT OF SCOPE)
- Packaged app offline launch, FFmpeg distribution manifests, third-party notices, and release hardening belong to Phase 6.
- Jianying/CapCut/Kaipai draft import/export adapters remain post-MVP.
- GPU real-time preview, mobile/server FFmpeg runtimes, nested sequences, advanced masks, proprietary effects, and full effect preset parity remain post-MVP.
</user_constraints>

## Project Constraints (from AGENTS.md)

- UI emits commands; Rust core owns project and timeline semantics; UI code must not directly construct FFmpeg commands. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is the canonical semantic source of truth; render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, and preview caches are derived artifacts. [VERIFIED: AGENTS.md]
- Product language, desktop code, Rust domain types, IPC commands, docs, schema, and tests should follow Jianying concepts; prefer draft/material/track/segment/keyframe/filter/transition-style terms. [VERIFIED: AGENTS.md]
- Core time math must use integer microseconds, frame indices, or rational frame rates; avoid naked floating-point time in persisted semantics. [VERIFIED: AGENTS.md]
- Render Graph isolates editing semantics from FFmpeg; FFmpeg Runtime executes jobs and reports progress/errors without deciding editing behavior. [VERIFIED: AGENTS.md]
- Kdenlive and MLT are conceptual references only; do not copy GPL code, assets, XML definitions, presets, or UI implementation. [VERIFIED: AGENTS.md]
- External drafts go through adapters and produce compatibility reports; proprietary IDs are external references, not internal render semantics. [VERIFIED: AGENTS.md]
- Each roadmap phase must define executable gates before implementation is considered complete. [VERIFIED: AGENTS.md]
- FFmpeg distribution must be reviewed for LGPL/GPL/nonfree build options, notices, and commercial product obligations before release work. [VERIFIED: AGENTS.md]
- Direct repo edits should start through GSD workflow entry points unless explicitly bypassed; this research file is requested as the GSD phase research artifact. [VERIFIED: AGENTS.md; VERIFIED: user prompt]

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TEXT-03 | Text layout uses pinned fonts and deterministic settings for preview/export parity. [VERIFIED: .planning/REQUIREMENTS.md] | Generate deterministic ASS text artifacts from `TextSegment`, pin `TextLayoutProfile`, require runtime font capability checks, and compare preview/export through the same FFmpeg graph. [VERIFIED: crates/draft_model/src/timeline.rs; CITED: https://ffmpeg.org/ffmpeg-filters.html] |
| PREV-01 | User can preview the current draft in the center player. [VERIFIED: .planning/REQUIREMENTS.md] | Add preview commands through `bindings_node` to `preview_service`; renderer receives artifact paths/status only. [VERIFIED: apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx; VERIFIED: crates/preview_service/src/lib.rs] |
| PREV-02 | User can seek/scrub the playhead and request a deterministic preview frame. [VERIFIED: .planning/REQUIREMENTS.md] | Use `engine_core::resolve_frame_state` at integer microsecond time and compile the same render graph range to a single image output. [VERIFIED: crates/engine_core/src/lib.rs; CITED: https://ffmpeg.org/ffmpeg-formats.html] |
| PREV-03 | User can play a short preview segment using a cache generated from the same render path as export. [VERIFIED: .planning/REQUIREMENTS.md] | `preview_service` should compile range render graphs with preview output profiles and store derived MP4 segments under a cache root. [VERIFIED: crates/preview_service/src/lib.rs; VERIFIED: .planning/phases/05-preview-and-export-pipeline/05-CONTEXT.md] |
| PREV-04 | Preview cache invalidates only affected ranges after timeline or text edits. [VERIFIED: .planning/REQUIREMENTS.md] | Store cache entries with target ranges and semantic fingerprints; invalidate overlapping ranges conservatively after accepted Rust command responses. [VERIFIED: .planning/phases/05-preview-and-export-pipeline/05-CONTEXT.md] |
| EXP-01 | User can export the draft to H.264 MP4 with a small preset set. [VERIFIED: .planning/REQUIREMENTS.md] | `ffmpeg_compiler` should emit `libx264`/AAC MP4 jobs for local and CI-capable FFmpeg runtimes. [VERIFIED: local `ffmpeg -encoders`; CITED: https://ffmpeg.org/ffmpeg.html] |
| EXP-02 | Export uses the same normalized draft, resolved frame state, render graph, and FFmpeg compilation path as preview. [VERIFIED: .planning/REQUIREMENTS.md] | Enforce the pipeline `Draft -> NormalizedDraft -> FrameState/RenderRange -> RenderGraph -> FfmpegJob -> media_runtime` for both preview and export. [VERIFIED: .planning/phases/05-preview-and-export-pipeline/05-CONTEXT.md] |
| EXP-03 | Export reports progress, supports cancel, captures logs, and classifies common FFmpeg errors. [VERIFIED: .planning/REQUIREMENTS.md] | Extend `media_runtime` beyond blocking `Output` capture to a streaming job API that parses `-progress pipe:1`, stores bounded stderr logs, and kills the child process on cancellation. [VERIFIED: crates/media_runtime/src/process.rs; CITED: https://ffmpeg.org/ffmpeg.html] |
| EXP-04 | Export output is validated for duration, fps, resolution, audio stream, and file existence. [VERIFIED: .planning/REQUIREMENTS.md] | Reuse the existing ffprobe JSON normalization pattern and add output validation metadata checks. [VERIFIED: crates/media_runtime/src/probe.rs; CITED: https://ffmpeg.org/ffprobe.html] |
| TEST-03 | Engine tests cover normalization, time mapping, track stacking, text layout, and frame-state snapshots. [VERIFIED: .planning/REQUIREMENTS.md] | Add focused `engine_core` unit tests and snapshot JSON fixtures before preview/export services depend on frame state. [VERIFIED: crates/engine_core/src/lib.rs; VERIFIED: crates/draft_commands/src/timeline.rs] |
| TEST-04 | Render graph and FFmpeg compiler outputs have snapshot tests. [VERIFIED: .planning/REQUIREMENTS.md] | Add stable pretty JSON/script snapshot tests with deterministic ordering and fixture paths. [VERIFIED: crates/render_graph/src/lib.rs; crates/ffmpeg_compiler/src/lib.rs] |
| TEST-05 | Preview frame and exported frame match within documented tolerance for golden drafts. [VERIFIED: .planning/REQUIREMENTS.md] | Use the same compiled graph to render one preview frame and one export sample frame, then compare dimensions, timestamp metadata, and pixel tolerance in `testkit`. [VERIFIED: crates/testkit/src/lib.rs; VERIFIED: local FFmpeg feature probe] |
</phase_requirements>

## Summary

Phase 5 should fill the existing Rust crate shells rather than create a parallel renderer-owned pipeline. `engine_core`, `render_graph`, `ffmpeg_compiler`, and `preview_service` currently expose only boundary markers or empty traits, while `media_runtime` already owns FFmpeg/ffprobe discovery, blocking process execution, and normalized material probing. [VERIFIED: crates/engine_core/src/lib.rs; VERIFIED: crates/render_graph/src/lib.rs; VERIFIED: crates/ffmpeg_compiler/src/lib.rs; VERIFIED: crates/preview_service/src/lib.rs; VERIFIED: crates/media_runtime/src/lib.rs] The planner should sequence work in the roadmap order: semantic normalization first, render graph/FFmpeg job compilation second, preview cache third, export job runtime fourth. [VERIFIED: .planning/ROADMAP.md]

The core technical decision is to make preview and export share the same compile path. A preview still frame is just a render graph for a one-frame or tiny time range with an image output profile; a cached preview segment is the same graph family with a short MP4 output profile; final export is the same graph family with a full-range MP4 output profile. [VERIFIED: .planning/phases/05-preview-and-export-pipeline/05-CONTEXT.md; CITED: https://ffmpeg.org/ffmpeg.html] This avoids a common parity failure where preview has a separate renderer model that drifts from export. [ASSUMED]

Local runtime capability is sufficient for MVP tests: FFmpeg/ffprobe 8.1 are installed, `libx264`, AAC, MP4 muxing, `image2`, `overlay`, `trim`, `setpts`, `amix`, `drawtext`, `ass`, and `subtitles` are available in the local build. [VERIFIED: local `ffmpeg -version`; VERIFIED: local `ffmpeg -filters`; VERIFIED: local `ffmpeg -encoders`; VERIFIED: local `ffmpeg -muxers`] CI installs `ffmpeg` through apt and already runs `just test`, so Phase 5 must add capability probes rather than assume every FFmpeg has `libass` or `libx264`. [VERIFIED: .github/workflows/ci.yml; CITED: https://ffmpeg.org/ffmpeg-filters.html]

**Primary recommendation:** Implement a Rust-owned pipeline with the dependency direction `draft_model -> engine_core -> render_graph -> ffmpeg_compiler -> preview_service/export service -> media_runtime_desktop`, expose only command/status/artifact contracts through `bindings_node`, and add Phase 5 gates to both `pnpm run test` and `just test`. [VERIFIED: Cargo.toml; VERIFIED: package.json; VERIFIED: .planning/ROADMAP.md]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Draft/timeline normalization | API / Rust semantic core | Database / `.veproj` input | `engine_core` should read `Draft` and produce derived normalized state; `.veproj/project.json` remains canonical semantic input. [VERIFIED: AGENTS.md; VERIFIED: crates/engine_core/src/lib.rs] |
| Frame-state evaluation | API / Rust semantic core | Render graph | Frame state is editing semantics at integer microsecond times; render graph consumes it but should not decide edit behavior. [VERIFIED: .planning/phases/05-preview-and-export-pipeline/05-CONTEXT.md] |
| Render graph construction | API / Rust render planning | FFmpeg compiler | `render_graph` owns renderer-neutral intents and must not execute FFmpeg. [VERIFIED: crates/render_graph/src/lib.rs; VERIFIED: AGENTS.md] |
| FFmpeg job/script compilation | API / Rust compiler | Media runtime | `ffmpeg_compiler` owns inputs, filter script text, sidecar text artifacts, encode settings, and validation metadata; `media_runtime` executes. [VERIFIED: crates/ffmpeg_compiler/src/lib.rs; VERIFIED: crates/media_runtime/src/lib.rs] |
| FFmpeg process execution | API / Runtime boundary | Desktop backend | `media_runtime` defines process behavior and `media_runtime_desktop` implements the desktop executor. [VERIFIED: crates/media_runtime/src/lib.rs; crates/media_runtime_desktop/src/lib.rs] |
| Preview frame/segment cache | API / Preview service | Filesystem cache | `preview_service` owns cache keys, layout, invalidation, and artifact metadata; UI receives derived artifacts only. [VERIFIED: crates/preview_service/src/lib.rs; VERIFIED: 05-CONTEXT.md] |
| Export job state/progress/cancel | API / Rust binding service | Media runtime | Export commands should start/status/cancel Rust-owned jobs; Electron displays Chinese status and logs. [VERIFIED: crates/bindings_node/src/lib.rs; CITED: https://ffmpeg.org/ffmpeg.html] |
| Desktop preview/export controls | Browser / Renderer | Rust binding commands | Renderer can hold playhead/output-path UI state, but must not construct render graphs, FFmpeg commands, cache keys, or scripts. [VERIFIED: scripts/phase4-source-guards.sh; VERIFIED: 05-CONTEXT.md] |
| Output metadata validation | API / Runtime/probe service | Testkit | Existing ffprobe JSON normalization pattern belongs in `media_runtime`; `testkit` can assert deterministic metadata for gates. [VERIFIED: crates/media_runtime/src/probe.rs; crates/testkit/src/lib.rs] |

## Standard Stack

### Core

| Library / Crate | Version | Purpose | Why Standard |
|-----------------|---------|---------|--------------|
| `draft_model` | local `0.1.0` [VERIFIED: Cargo metadata] | Canonical draft/material/track/segment/time/text/audio schema. [VERIFIED: crates/draft_model/src/lib.rs] | Existing Rust-generated schema and TypeScript contracts already make it the semantic source of truth. [VERIFIED: crates/draft_model/tests/schema_exports.rs] |
| `draft_commands` | local `0.1.0` [VERIFIED: Cargo metadata] | Accepted timeline/text/audio edits and Rust-owned command semantics. [VERIFIED: crates/draft_commands/src/lib.rs] | Phase 5 must derive preview invalidation from accepted command outputs, not duplicate edit semantics. [VERIFIED: crates/draft_commands/src/timeline.rs] |
| `engine_core` | local shell `0.1.0` [VERIFIED: Cargo metadata] | Normalize drafts and evaluate frame state for preview/export. [VERIFIED: crates/engine_core/src/lib.rs] | Locked decision makes it the first shared semantic layer before render graph. [VERIFIED: 05-CONTEXT.md] |
| `render_graph` | local shell `0.1.0` [VERIFIED: Cargo metadata] | Typed renderer-neutral intents for visual/audio/text output. [VERIFIED: crates/render_graph/src/lib.rs] | Keeps editing semantics separated from FFmpeg command syntax. [VERIFIED: AGENTS.md] |
| `ffmpeg_compiler` | local shell `0.1.0` [VERIFIED: Cargo metadata] | Compile render graph to structured FFmpeg jobs, scripts, sidecars, and validation expectations. [VERIFIED: crates/ffmpeg_compiler/src/lib.rs] | Prevents UI and runtime from hand-building FFmpeg arguments. [VERIFIED: 05-CONTEXT.md] |
| `media_runtime` | local `0.1.0` [VERIFIED: Cargo metadata] | FFmpeg/ffprobe discovery, runtime errors, process execution, material probing. [VERIFIED: crates/media_runtime/src/lib.rs] | Existing boundary already prevents pure crates from depending on process execution. [VERIFIED: crates/media_runtime/src/lib.rs] |
| `media_runtime_desktop` | local `0.1.0` [VERIFIED: Cargo metadata] | Desktop `FfmpegExecutor` implementation. [VERIFIED: crates/media_runtime_desktop/src/lib.rs] | Desktop MVP runtime should stay behind this backend and not leak into semantic crates. [VERIFIED: AGENTS.md] |
| `preview_service` | local shell `0.1.0` [VERIFIED: Cargo metadata] | Preview frame/segment orchestration, cache layout, invalidation. [VERIFIED: crates/preview_service/src/lib.rs] | Locked decision names it as owner of preview artifacts and cache behavior. [VERIFIED: 05-CONTEXT.md] |
| `bindings_node` | local `0.1.0` [VERIFIED: Cargo metadata] | Electron-facing command router and Rust service composition. [VERIFIED: crates/bindings_node/src/lib.rs] | Existing renderer calls `window.videoEditorCore.executeCommand`, so Phase 5 should add commands/contracts there. [VERIFIED: apps/desktop-electron/src/preload/index.ts; apps/desktop-electron/src/renderer/commandHelpers.ts] |
| `testkit` | local `0.1.0` [VERIFIED: Cargo metadata] | Generated media fixtures, render smoke, golden helpers. [VERIFIED: crates/testkit/src/lib.rs] | Existing tests already use FFmpeg-generated fixtures and ffprobe metadata. [VERIFIED: crates/testkit/tests/render_smoke.rs] |

### Supporting

| Library / Tool | Version | Purpose | When to Use |
|----------------|---------|---------|-------------|
| FFmpeg / ffprobe | Local `8.1`; CI installs apt `ffmpeg`. [VERIFIED: local `ffmpeg -version`; VERIFIED: .github/workflows/ci.yml] | Still-frame extraction, preview MP4 segments, final MP4 export, metadata validation. [CITED: https://ffmpeg.org/ffmpeg.html; CITED: https://ffmpeg.org/ffprobe.html] | Use only through `media_runtime` / `media_runtime_desktop`; never from renderer or pure semantic crates. [VERIFIED: AGENTS.md; VERIFIED: scripts/phase4-source-guards.sh] |
| `serde` / `serde_json` | `serde 1.0.228`, `serde_json 1.0.150` [VERIFIED: Cargo manifests; VERIFIED: cargo search] | Stable snapshot serialization for normalized draft, frame state, render graph, FFmpeg job metadata. [VERIFIED: crates/draft_model/Cargo.toml; crates/media_runtime/Cargo.toml] | Use pretty JSON snapshots with sorted/BTree collections for reviewable diffs. [VERIFIED: crates/draft_model/tests/schema_exports.rs] |
| `thiserror` | `2.0.18` [VERIFIED: Cargo manifests; VERIFIED: cargo search] | Structured runtime/service errors. [VERIFIED: crates/media_runtime/Cargo.toml; crates/project_store/Cargo.toml] | Use for new classified preview/export/runtime errors if those errors do not need ts-rs exports. [VERIFIED: crates/media_runtime/src/error.rs] |
| `tempfile` | `3.27.0` [VERIFIED: Cargo manifests; VERIFIED: cargo search] | Isolated render/cache/export test directories. [VERIFIED: crates/testkit/Cargo.toml] | Use in Rust tests and `testkit` helpers; do not commit binary artifacts. [VERIFIED: crates/testkit/src/lib.rs] |
| Playwright Electron | `@playwright/test 1.61.0` [VERIFIED: apps/desktop-electron/package.json; VERIFIED: npm registry] | Desktop preview/export UI smoke and layout checks. [VERIFIED: apps/desktop-electron/tests/workspace.spec.ts] | Use for command visibility, Chinese labels, preview monitor state, export status/cancel/log surface. [VERIFIED: 05-CONTEXT.md] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| FFmpeg-generated preview frames/segments | GPU real-time preview renderer | GPU preview is deferred and would create a second render path, increasing preview/export drift risk. [VERIFIED: 05-CONTEXT.md; ASSUMED] |
| Generated ASS subtitles | FFmpeg `drawtext` per text segment | `drawtext` works locally, but direct text strings require careful escaping; FFmpeg docs recommend file-based text for avoiding escape complexity. [VERIFIED: local `ffmpeg -filters`; CITED: https://ffmpeg.org/ffmpeg-filters.html] |
| JSON/script fixture snapshots | Adding a snapshot crate such as `insta` | The repo already uses generated files and git diff as golden drift checks; adding a new crate is unnecessary for Phase 5. [VERIFIED: crates/draft_model/tests/schema_exports.rs; VERIFIED: package.json] |
| Blocking `FfmpegExecutor::run` only | Streaming child process API | Blocking execution cannot report progress or support responsive cancellation; Phase 5 requires progress/cancel. [VERIFIED: crates/media_runtime/src/process.rs; VERIFIED: .planning/REQUIREMENTS.md] |

**Installation:** No new npm or Rust crates are recommended for Phase 5. [VERIFIED: Cargo metadata; VERIFIED: apps/desktop-electron/package.json]

```bash
# Use existing dependencies.
pnpm install --frozen-lockfile
```

**Version verification:** Existing npm package versions were checked with `npm view <package> version time.modified homepage repository.url` on 2026-06-17; existing Rust crate versions were checked with `cargo metadata --locked` and `cargo search <crate> --limit 1`. [VERIFIED: npm registry; VERIFIED: cargo metadata; VERIFIED: crates.io index via cargo search]

## Package Legitimacy Audit

> Phase 5 should not add external packages. This audit records existing direct npm packages used by the desktop/test stack and the external package gate result. [VERIFIED: apps/desktop-electron/package.json; VERIFIED: Cargo metadata]

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `electron` | npm | Existing direct dependency, current npm version `42.4.1`, modified 2026-06-16. [VERIFIED: npm registry] | Not re-queried for Phase 5 because no install is recommended. [VERIFIED: apps/desktop-electron/package.json] | `github.com/electron/electron` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `react` | npm | Existing direct dependency, current npm version `19.2.7`, modified 2026-06-16. [VERIFIED: npm registry] | Not re-queried for Phase 5 because no install is recommended. [VERIFIED: apps/desktop-electron/package.json] | `github.com/facebook/react` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `react-dom` | npm | Existing direct dependency, current npm version `19.2.7`, modified 2026-06-16. [VERIFIED: npm registry] | Not re-queried for Phase 5 because no install is recommended. [VERIFIED: apps/desktop-electron/package.json] | `github.com/facebook/react` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `@playwright/test` | npm | Existing dev dependency, current npm version `1.61.0`, modified 2026-06-17. [VERIFIED: npm registry] | Not re-queried for Phase 5 because no install is recommended. [VERIFIED: apps/desktop-electron/package.json] | `github.com/microsoft/playwright` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `@napi-rs/cli` | npm | Existing dev dependency, current npm version `3.7.2`, modified 2026-06-14. [VERIFIED: npm registry] | Not re-queried for Phase 5 because no install is recommended. [VERIFIED: apps/desktop-electron/package.json] | `github.com/napi-rs/napi-rs` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `vite` | npm | Existing dev dependency, current npm version `8.0.16`, modified 2026-06-15. [VERIFIED: npm registry] | Not re-queried for Phase 5 because no install is recommended. [VERIFIED: apps/desktop-electron/package.json] | `github.com/vitejs/vite` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `typescript` | npm | Existing dev dependency, current npm version `6.0.3`, modified 2026-06-17. [VERIFIED: npm registry] | Not re-queried for Phase 5 because no install is recommended. [VERIFIED: apps/desktop-electron/package.json] | `github.com/microsoft/TypeScript` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |

**Packages removed due to slopcheck [SLOP] verdict:** none. [VERIFIED: slopcheck output]
**Packages flagged as suspicious [SUS]:** none. [VERIFIED: slopcheck output]

Notes: Installed `slopcheck 0.6.1` does not support the protocol's `--json` flag, so text output was used. `slopcheck install` invoked `npm install` as a side effect; the resulting `package.json` dependency churn and `package-lock.json` were removed, and existing unrelated untracked `reference/` content was not touched. [VERIFIED: terminal output; VERIFIED: git status]

## Architecture Patterns

### System Architecture Diagram

```text
Accepted Draft + bundle path + output/profile request
  -> engine_core::normalize_draft
      -> validates material status, muted tracks, source/target timing, stacking, text profile
  -> engine_core::resolve_frame_state / resolve_render_range
      -> deterministic visual layers + audio segments + text overlays at integer times
  -> render_graph::build_render_graph
      -> typed renderer-neutral graph, no FFmpeg syntax, no filesystem execution
  -> ffmpeg_compiler::compile_ffmpeg_job
      -> FfmpegJob { inputs, filter script, ASS sidecars, args, output profile, validation expectations }
  -> media_runtime[_desktop]
      -> executes ffmpeg, parses progress/logs, supports cancel, returns classified errors
  -> ffprobe validation
      -> file existence, duration, fps, resolution, audio stream, non-empty output
  -> bindings_node command envelope
      -> renderer receives artifact/status/log metadata in Chinese UI

Preview frame:
  same path, one-frame/image output profile

Cached preview segment:
  same path, short MP4 output profile, stored under preview_service cache

Final export:
  same path, full range MP4 output profile, stored at user-selected output path
```

This architecture is derived from the Phase 5 locked shared semantic path and the existing crate boundary comments. [VERIFIED: 05-CONTEXT.md; VERIFIED: crates/*/src/lib.rs]

### Recommended Project Structure

```text
crates/
├── engine_core/src/
│   ├── lib.rs              # public normalize/evaluate API [RECOMMENDED]
│   ├── normalize.rs        # NormalizedDraft, NormalizedTrack, NormalizedSegment [RECOMMENDED]
│   ├── frame_state.rs      # FrameState and RenderRange evaluation [RECOMMENDED]
│   └── text_layout.rs      # deterministic TextLayoutProfile and ASS-safe text model [RECOMMENDED]
├── render_graph/src/
│   ├── lib.rs              # graph types + builder export [RECOMMENDED]
│   ├── graph.rs            # RenderGraph, VideoLayer, AudioMix, TextOverlay [RECOMMENDED]
│   └── profile.rs          # PreviewFrame, PreviewSegment, ExportMp4 profiles [RECOMMENDED]
├── ffmpeg_compiler/src/
│   ├── lib.rs              # compile_ffmpeg_job API [RECOMMENDED]
│   ├── job.rs              # FfmpegJob, FfmpegInput, FfmpegSidecar, EncodeSettings [RECOMMENDED]
│   ├── filters.rs          # deterministic filter script generation [RECOMMENDED]
│   └── ass.rs              # ASS sidecar generation from text intents [RECOMMENDED]
├── media_runtime/src/
│   ├── job.rs              # streaming job state/progress/cancel/error contracts [RECOMMENDED]
│   └── validate.rs         # ffprobe output validation [RECOMMENDED]
├── preview_service/src/
│   ├── lib.rs              # request APIs [RECOMMENDED]
│   ├── cache.rs            # range cache key/layout/invalidation [RECOMMENDED]
│   └── service.rs          # orchestration over engine/render/compiler/runtime [RECOMMENDED]
├── bindings_node/src/
│   ├── lib.rs              # command routing [VERIFIED: current file]
│   └── preview_export_service.rs # job registry + command adapters [RECOMMENDED]
└── testkit/src/
    ├── lib.rs              # current helpers [VERIFIED: current file]
    └── render_compare.rs   # metadata + pixel tolerance helpers [RECOMMENDED]
```

Use module splits only if files remain readable; the hard constraint is dependency direction, not exact filenames. [VERIFIED: 05-CONTEXT.md]

### Dependency Direction

| Crate | May Depend On | Must Not Depend On |
|-------|---------------|--------------------|
| `draft_model` | existing serde/schema deps only. [VERIFIED: crates/draft_model/Cargo.toml] | `draft_commands`, `engine_core`, `render_graph`, `ffmpeg_compiler`, `media_runtime`, `project_store`, `bindings_node`. [VERIFIED: AGENTS.md] |
| `draft_commands` | `draft_model`. [VERIFIED: crates/draft_commands/Cargo.toml] | FFmpeg, filesystem, Electron, preview/export crates. [VERIFIED: package.json source guards; VERIFIED: AGENTS.md] |
| `engine_core` | `draft_model`; optionally `serde` for snapshots if exported/tested. [RECOMMENDED; VERIFIED: current shell has no deps] | `media_runtime`, `media_runtime_desktop`, `project_store`, `bindings_node`, `std::process`, filesystem execution. [VERIFIED: 05-CONTEXT.md] |
| `render_graph` | `draft_model`, `engine_core`; `serde` for snapshots. [RECOMMENDED] | `media_runtime`, `media_runtime_desktop`, `bindings_node`, process execution. [VERIFIED: crates/render_graph/src/lib.rs] |
| `ffmpeg_compiler` | `render_graph`; `serde`; std paths for planned sidecars. [RECOMMENDED] | `media_runtime_desktop`, UI/Electron, edit semantics. [VERIFIED: crates/ffmpeg_compiler/src/lib.rs] |
| `media_runtime` | existing runtime deps plus `ffmpeg_compiler` only if job structs live there; otherwise accept a local runtime job type converted from compiler output. [RECOMMENDED] | `draft_model`, `engine_core`, `render_graph`, `bindings_node`. [VERIFIED: crates/media_runtime/src/lib.rs] |
| `preview_service` | `draft_model`, `engine_core`, `render_graph`, `ffmpeg_compiler`, `media_runtime`, `project_store` path helpers. [RECOMMENDED; VERIFIED: project_store path helpers exist] | `media_runtime_desktop`, Electron/Node. [VERIFIED: AGENTS.md] |
| `bindings_node` | service crates plus `media_runtime_desktop` and `project_store`. [VERIFIED: current bindings pattern] | Renderer-only logic, direct FFmpeg string assembly outside Rust service calls. [VERIFIED: scripts/phase4-source-guards.sh] |

### Pattern 1: Normalized Draft and Frame State

**What:** Convert `Draft` into immutable, sorted, render-ready semantic structures before preview/export. [VERIFIED: 05-CONTEXT.md]

**When to use:** Use for every preview frame, preview segment, export, and golden test. [VERIFIED: 05-CONTEXT.md]

**Example:**

```rust
// Source: recommended API derived from crates/engine_core/src/lib.rs and Phase 5 context.
pub struct EngineProfile {
    pub output_width: u32,
    pub output_height: u32,
    pub frame_rate: RationalFrameRate,
    pub text_layout: TextLayoutProfile,
}

pub fn normalize_draft(draft: &Draft, profile: &EngineProfile) -> Result<NormalizedDraft, EngineError>;

pub fn resolve_frame_state(
    normalized: &NormalizedDraft,
    at: Microseconds,
) -> Result<FrameState, EngineError>;
```

The normalized model should include source time mapping as `source_at = source.start + (at - target.start)` using checked integer microsecond arithmetic. [VERIFIED: crates/draft_model/src/time.rs; VERIFIED: crates/draft_commands/src/timeline.rs]

### Pattern 2: Renderer-Neutral Render Graph

**What:** Build a typed graph with video layers, audio mixes, text overlays, and output profiles, independent of FFmpeg syntax. [VERIFIED: crates/render_graph/src/lib.rs; VERIFIED: 05-CONTEXT.md]

**When to use:** Use after `engine_core` has resolved semantic timing and before compiling any preview/export job. [VERIFIED: 05-CONTEXT.md]

**Example:**

```rust
// Source: recommended API derived from render_graph crate boundary.
pub enum RenderOutputProfile {
    PreviewFrame { at: Microseconds, format: StillFormat },
    PreviewSegment { range: TargetTimerange, width: u32, height: u32 },
    ExportMp4 { range: TargetTimerange, preset: ExportPreset },
}

pub fn build_render_graph(
    normalized: &NormalizedDraft,
    range: TargetTimerange,
    profile: RenderOutputProfile,
) -> Result<RenderGraph, RenderGraphError>;
```

Use `BTreeMap`/sorted vectors in graph serialization so snapshot diffs are stable. [VERIFIED: crates/draft_model/src/timeline.rs uses BTreeMap for filter parameters]

### Pattern 3: Structured FFmpeg Job, Not Shell Strings

**What:** Compile graph output into an argument vector, filter script text, sidecar artifacts, and validation expectations. [VERIFIED: 05-CONTEXT.md; CITED: https://ffmpeg.org/ffmpeg.html]

**When to use:** Use for preview stills, preview segments, and final MP4 export. [VERIFIED: 05-CONTEXT.md]

**Example:**

```rust
// Source: recommended API derived from ffmpeg_compiler crate boundary.
pub struct FfmpegJob {
    pub inputs: Vec<FfmpegInput>,
    pub filter_script: String,
    pub sidecars: Vec<FfmpegSidecar>,
    pub args: Vec<OsString>,
    pub output_path: PathBuf,
    pub validation: OutputValidationExpectation,
}

pub fn compile_ffmpeg_job(
    graph: &RenderGraph,
    context: &CompileContext,
) -> Result<FfmpegJob, FfmpegCompileError>;
```

FFmpeg docs support complex filtergraphs with `-filter_complex` and explicit `-map` for selecting filtergraph outputs. [CITED: https://ffmpeg.org/ffmpeg.html]

### Pattern 4: Streaming Runtime Job

**What:** Extend `media_runtime` from blocking `run() -> Output` to a streaming job API with progress events and cancellation. [VERIFIED: crates/media_runtime/src/process.rs]

**When to use:** Required for EXP-03 and useful for preview segment generation status. [VERIFIED: .planning/REQUIREMENTS.md]

**Example:**

```rust
// Source: recommended API derived from existing FfmpegExecutor and FFmpeg -progress docs.
pub trait FfmpegJobExecutor {
    fn run_job(
        &self,
        job: &FfmpegJob,
        sink: &mut dyn FfmpegEventSink,
        cancel: &CancelToken,
    ) -> Result<FfmpegJobResult, FfmpegRuntimeError>;
}
```

FFmpeg can emit program-friendly progress to a URL such as `pipe:1`; parse key-value updates and compute percent from expected output duration. [CITED: https://ffmpeg.org/ffmpeg.html]

### Anti-Patterns to Avoid

- **Renderer-assembled FFmpeg commands:** This violates AGENTS and existing Phase 4 source guards; add Phase 5 guards for `filter_complex`, `renderGraph`, `ffmpegScripts`, `previewCache`, `drawtext`, and direct cache key terms in renderer. [VERIFIED: AGENTS.md; VERIFIED: scripts/phase4-source-guards.sh]
- **Separate preview renderer model:** This risks preview/export drift; preview should be a different output profile over the same normalized graph. [VERIFIED: 05-CONTEXT.md]
- **Persisting generated artifacts in `.veproj/project.json`:** Existing schema tests reject derived artifact fields, and Phase 5 must keep render graphs/scripts/cache/exports derived. [VERIFIED: crates/draft_model/tests/draft_schema.rs; VERIFIED: AGENTS.md]
- **Using floats for semantic time:** Existing model uses `Microseconds` and rational frame rates; source guards already forbid float time patterns in core/generated contracts. [VERIFIED: crates/draft_model/src/time.rs; VERIFIED: package.json]
- **Byte-perfect cross-machine text/video assertions:** Font rasterization and FFmpeg builds vary; use same-runtime preview/export parity with documented tolerance plus deterministic script/ASS snapshots. [VERIFIED: local FFmpeg feature probe; ASSUMED]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Video decoding, encoding, muxing, frame extraction | Custom decoder/encoder or canvas renderer | FFmpeg through `media_runtime` | FFmpeg already supports filtergraphs, MP4 muxing, image2 stills, progress output, and local `libx264`/AAC. [VERIFIED: local FFmpeg probes; CITED: https://ffmpeg.org/ffmpeg.html] |
| Material/output metadata parsing | Ad hoc text parsing of ffprobe console output | ffprobe JSON with normalized Rust structs | Existing probe code already uses `-print_format json` and structured stream/format normalization. [VERIFIED: crates/media_runtime/src/probe.rs; CITED: https://ffmpeg.org/ffprobe.html] |
| Text rendering engine | Custom text rasterizer in Rust/Canvas | Generated ASS subtitles or documented FFmpeg text filters | FFmpeg filters support `ass`/`subtitles` with libass locally; generated sidecars are snapshot-testable. [VERIFIED: local `ffmpeg -filters`; CITED: https://ffmpeg.org/ffmpeg-filters.html] |
| Timeline edit diffing in UI for cache invalidation | Renderer-side semantic diff engine | Accepted Rust command responses plus `preview_service` range invalidation | Renderer must not own timeline semantics; preview service owns cache invalidation. [VERIFIED: 05-CONTEXT.md; VERIFIED: scripts/phase4-source-guards.sh] |
| Process cancellation and timeout behavior in Electron renderer | Browser timers or raw IPC process handles | Rust runtime job API over child process lifecycle | Process execution belongs to `media_runtime`; existing runtime already kills hung process after timeout. [VERIFIED: crates/media_runtime/src/process.rs; crates/media_runtime_desktop/src/lib.rs] |
| Snapshot framework dependency | New external snapshot crate | Pretty JSON/script files plus `git diff --exit-code` style gates | The repo already enforces generated contract drift through files and git diff. [VERIFIED: crates/draft_model/tests/schema_exports.rs; VERIFIED: package.json] |

**Key insight:** The hard part is not invoking FFmpeg; it is preserving one semantic path from draft edits to preview and export while making generated artifacts reviewable, invalidatable, cancellable, and testable. [VERIFIED: 05-CONTEXT.md]

## Common Pitfalls

### Pitfall 1: Preview/Export Parity Drift
**What goes wrong:** Preview uses a different model or filter path than export, so frames differ after text/audio/track edits. [VERIFIED: 05-CONTEXT.md]  
**Why it happens:** Preview is treated as a UI feature rather than an output profile over the render graph. [ASSUMED]  
**How to avoid:** Require preview frame, preview segment, and export to start from `NormalizedDraft` and compile through `RenderGraph -> FfmpegJob`. [VERIFIED: 05-CONTEXT.md]  
**Warning signs:** Renderer code mentions FFmpeg/render graph/cache keys; `preview_service` accepts UI-derived layer lists; tests compare only UI state without generated job snapshots. [VERIFIED: scripts/phase4-source-guards.sh]

### Pitfall 2: Text Is Visually Non-Deterministic
**What goes wrong:** The same `TextSegment` produces different preview/export text placement or glyph metrics across machines. [ASSUMED]  
**Why it happens:** Font fallback, fontconfig, libass availability, and drawtext escaping differ by runtime. [VERIFIED: local FFmpeg config; CITED: https://ffmpeg.org/ffmpeg-filters.html]  
**How to avoid:** Add `TextLayoutProfile { font_family, font_path, font_size, alignment, safe_area, line_wrap_policy }`, resolve it once in `engine_core`, generate ASS sidecars deterministically, and classify missing font/libass as a runtime limitation. [RECOMMENDED; VERIFIED: 05-CONTEXT.md]  
**Warning signs:** Direct `drawtext=text=...` strings in generated scripts, no font path in snapshots, CI and local goldens differ. [CITED: https://ffmpeg.org/ffmpeg-filters.html]

### Pitfall 3: Blocking Runtime Cannot Report Progress or Cancel
**What goes wrong:** Export freezes until FFmpeg exits, cancel cannot interrupt, and logs arrive only after failure. [VERIFIED: crates/media_runtime/src/process.rs]  
**Why it happens:** Existing `FfmpegExecutor::run` returns `std::process::Output`, which is available only after process completion. [VERIFIED: crates/media_runtime/src/lib.rs; crates/media_runtime/src/process.rs]  
**How to avoid:** Add streaming process execution with stdout progress parsing, stderr log buffering, and cancellation state. [CITED: https://ffmpeg.org/ffmpeg.html]  
**Warning signs:** Export command returns final output only, no job id/status command, no cancel test with a long-running FFmpeg fixture. [VERIFIED: .planning/REQUIREMENTS.md]

### Pitfall 4: FFmpeg Capability Assumptions
**What goes wrong:** Local tests pass but CI or another desktop machine lacks `libx264`, `subtitles`, `ass`, or expected fonts. [VERIFIED: local FFmpeg config; VERIFIED: .github/workflows/ci.yml]  
**Why it happens:** FFmpeg builds are configurable; filters/encoders depend on build options. [CITED: https://ffmpeg.org/ffmpeg-filters.html]  
**How to avoid:** Add a `probe_render_runtime_capabilities` command/test that checks required encoders, muxers, filters, and font paths before render tests. [RECOMMENDED]  
**Warning signs:** Tests call render without checking capabilities; errors are generic `ProcessLaunchFailed` rather than `MissingEncoder` or `MissingFilter`. [VERIFIED: crates/media_runtime/src/probe.rs]

### Pitfall 5: Cache Invalidation Too Broad or Too Narrow
**What goes wrong:** Every edit clears all preview cache, or overlapping edited ranges serve stale preview segments. [VERIFIED: 05-CONTEXT.md]  
**Why it happens:** Cache entries are not indexed by target time range and semantic fingerprint. [ASSUMED]  
**How to avoid:** Store `{target_start, target_end, profile, normalized_fingerprint, material_dependencies}` per cache entry and invalidate overlapping ranges conservatively after accepted command responses. [RECOMMENDED; VERIFIED: 05-CONTEXT.md]  
**Warning signs:** Cache key is just draft id, or invalidation runs in renderer. [VERIFIED: scripts/phase4-source-guards.sh]

## Code Examples

Verified and recommended patterns from the codebase and official docs:

### FFmpeg Progress Argument Pattern

```rust
// Source: FFmpeg official docs for -progress plus existing OsString args pattern.
let args = vec![
    OsString::from("-hide_banner"),
    OsString::from("-nostats"),
    OsString::from("-progress"),
    OsString::from("pipe:1"),
    OsString::from("-i"),
    input_path.as_os_str().to_owned(),
    OsString::from("-filter_complex_script"),
    filter_script_path.as_os_str().to_owned(),
    OsString::from("-map"),
    OsString::from("[vout]"),
    OsString::from("-map"),
    OsString::from("[aout]"),
    OsString::from("-c:v"),
    OsString::from("libx264"),
    OsString::from("-c:a"),
    OsString::from("aac"),
    output_path.as_os_str().to_owned(),
];
```

FFmpeg `-progress pipe:1` writes program-friendly progress and `-map` can select outputs from complex filtergraphs. [CITED: https://ffmpeg.org/ffmpeg.html] Use vectors of `OsString`, not shell-concatenated strings, matching the existing runtime process style. [VERIFIED: crates/media_runtime/src/probe.rs]

### Preview Cache Entry Shape

```rust
// Source: recommended structure derived from Phase 5 cache constraints.
pub struct PreviewCacheEntry {
    pub cache_key: PreviewCacheKey,
    pub target_start: Microseconds,
    pub target_end: Microseconds,
    pub artifact_path: PathBuf,
    pub semantic_fingerprint: String,
    pub material_ids: Vec<MaterialId>,
}

pub fn invalidates(entry: &PreviewCacheEntry, changed: &TargetTimerange) -> bool {
    changed.start.get() < entry.target_end.get()
        && entry.target_start.get() < changed.start.get().saturating_add(changed.duration.get())
}
```

The overlap predicate follows the existing `target_ranges_overlap` semantics used by timeline validation. [VERIFIED: crates/draft_commands/src/timeline.rs]

### Output Validation Shape

```rust
// Source: recommended API derived from existing media_runtime probe metadata normalization.
pub struct OutputValidationExpectation {
    pub duration: TargetTimerange,
    pub frame_rate: RationalFrameRate,
    pub width: u32,
    pub height: u32,
    pub expect_audio_stream: bool,
}

pub fn validate_rendered_output(
    runtime: &RuntimeConfig,
    executor: &impl FfmpegExecutor,
    output_path: &Path,
    expected: &OutputValidationExpectation,
) -> Result<OutputValidationReport, OutputValidationError>;
```

Existing material probing already parses ffprobe JSON stream and format metadata into typed Rust values. [VERIFIED: crates/media_runtime/src/probe.rs; CITED: https://ffmpeg.org/ffprobe.html]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| UI directly building preview/export commands | Rust command/binding path with generated contracts | Locked before Phase 5 in AGENTS and Phase 5 context. [VERIFIED: AGENTS.md; VERIFIED: 05-CONTEXT.md] | Planner must add Rust commands and source guards, not renderer shortcuts. |
| Blocking FFmpeg process wrapper only | Streaming progress/cancel/log runtime API for export | Required in Phase 5 by EXP-03. [VERIFIED: .planning/REQUIREMENTS.md] | `media_runtime` needs a new execution path while keeping the old blocking helper for probes/smokes. |
| Preview placeholder shell | Command-driven preview frame and cached segment | Phase 4 deferred real preview to Phase 5. [VERIFIED: apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx; VERIFIED: 04-CONTEXT.md] | Desktop plan must replace placeholder state with preview artifact/status UI. |
| Material probe metadata only | Render output validation via ffprobe | Required in Phase 5 by EXP-04. [VERIFIED: .planning/REQUIREMENTS.md] | Extend probe/validation helpers for rendered outputs, not just imported materials. |

**Deprecated/outdated:**
- GPU real-time preview for MVP: explicitly deferred; FFmpeg-generated frames/segments are the Phase 5 MVP path. [VERIFIED: 05-CONTEXT.md]
- Separate preview render semantics: conflicts with EXP-02 and the shared semantic path decision. [VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: 05-CONTEXT.md]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Separate preview renderer models commonly cause preview/export drift. | Summary, Common Pitfalls | Planner might overweight a separate preview implementation and miss parity risk. |
| A2 | Font rasterization and FFmpeg builds can prevent byte-perfect cross-machine text/video assertions. | Anti-Patterns, Common Pitfalls | Planner might create brittle CI goldens that fail on another OS or FFmpeg package. |
| A3 | Cache invalidation errors commonly come from missing range/fingerprint indexes. | Common Pitfalls | Planner might under-specify cache metadata and create stale preview bugs. |

## Open Questions (RESOLVED)

1. **Which test font is the Phase 5 deterministic default?**
   - Resolution: Phase 5 uses an explicit `TextLayoutProfile` instead of ambient font discovery. The profile default is `PingFang SC` for the desktop product language, with deterministic fallback candidates checked in order: `VE_TEXT_FONT_PATH`, `/System/Library/Fonts/PingFang.ttc`, `/System/Library/Fonts/Supplemental/Arial Unicode.ttf`, `/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc`, then `/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf`. [RESOLVED]
   - Behavior: if none of the configured candidates can be resolved by the runtime/compiler capability probe, preview/export returns a classified text font limitation instead of silently selecting a different font. Tests may set `VE_TEXT_FONT_PATH` to pin a host-specific font path without changing `.veproj/project.json`. [RESOLVED]

2. **Where should preview/export job state live?**
   - Resolution: preview cache/request state lives in `preview_service`; streaming export progress, cancel tokens, bounded logs, runtime error classification, and output validation primitives live in `media_runtime`; `bindings_node::preview_export_service` owns only the Electron-facing in-process job registry that maps command IDs to Rust job IDs/status. The renderer stores only display state such as playhead, selected output path, job id, and last status. [RESOLVED]

3. **How strict should preview/export parity be?**
   - Resolution: Phase 5 parity is not byte-perfect. The documented MVP tolerance is exact width/height and expected frame index, timestamp within one output frame duration, mean absolute RGB delta <= 8.0, and 99th percentile RGB delta <= 24 when comparing preview frame output to the matching exported frame under the same local FFmpeg/runtime. Missing filters, encoders, or fonts are classified setup failures, not silent skips. [RESOLVED]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust toolchain | Workspace build/tests | yes [VERIFIED: local command] | `rustc 1.95.0`, `cargo 1.95.0` [VERIFIED: local command] | CI installs Rust 1.95.0. [VERIFIED: .github/workflows/ci.yml] |
| Node.js | Electron build/tests | yes [VERIFIED: local command] | `v24.12.0` [VERIFIED: local command] | CI setup-node installs 24.12.0. [VERIFIED: .github/workflows/ci.yml] |
| pnpm | Package install/scripts | yes [VERIFIED: local command] | `10.32.1` [VERIFIED: local command] | CI activates pnpm 10.32.1 through Corepack. [VERIFIED: .github/workflows/ci.yml] |
| just | Public gates | yes [VERIFIED: local command] | `11.6.2` [VERIFIED: local command] | CI installs `just` if missing. [VERIFIED: .github/workflows/ci.yml] |
| FFmpeg | Preview/export render execution | yes [VERIFIED: local command] | `8.1` [VERIFIED: local command] | Runtime discovery errors already exist; CI installs apt `ffmpeg`. [VERIFIED: crates/media_runtime/src/discovery.rs; VERIFIED: .github/workflows/ci.yml] |
| ffprobe | Output metadata validation | yes [VERIFIED: local command] | `8.1` [VERIFIED: local command] | Runtime discovery errors already exist; CI installs apt `ffmpeg`. [VERIFIED: crates/media_runtime/src/discovery.rs; VERIFIED: .github/workflows/ci.yml] |
| `libx264` encoder | H.264 MP4 export | yes locally [VERIFIED: local `ffmpeg -encoders`] | FFmpeg 8.1 build | Add capability check; fail with classified `MissingEncoder` if absent. [RECOMMENDED] |
| `aac` encoder | MP4 audio export | yes locally [VERIFIED: local `ffmpeg -encoders`] | FFmpeg 8.1 build | Export silent audio or video-only when graph has no audio; classify missing encoder when audio is required. [RECOMMENDED] |
| `ass`/`subtitles` filters | Text rendering | yes locally [VERIFIED: local `ffmpeg -filters`] | FFmpeg 8.1 build with libass | Fall back to classified text-render limitation or drawtext-only degraded mode. [RECOMMENDED] |
| Deterministic text font | TEXT-03 tests | partial [VERIFIED: local font search] | `Arial Unicode.ttf` local; CI unknown | Use `VE_TEXT_FONT_PATH` and Wave 0 capability probe. [RECOMMENDED] |

**Missing dependencies with no fallback:** none detected locally for Phase 5 MVP rendering. [VERIFIED: local probes]

**Missing dependencies with fallback:** deterministic CI font path is not verified yet; add Wave 0 probe/fallback. [VERIFIED: local font search; VERIFIED: .github/workflows/ci.yml]

## Validation Architecture

Nyquist validation is enabled because `.planning/config.json` has `workflow.nyquist_validation: true`. [VERIFIED: .planning/config.json]

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` for core/compiler/runtime/testkit; Playwright Test `1.61.0` for Electron workspace; bash source guards. [VERIFIED: Cargo.toml; VERIFIED: apps/desktop-electron/package.json; VERIFIED: scripts/phase4-source-guards.sh] |
| Config file | `Cargo.toml`, `package.json`, `Justfile`, `apps/desktop-electron/playwright.config.ts`. [VERIFIED: codebase files] |
| Quick run command | `pnpm run test:phase5-render-core` after Wave 0 adds it. [RECOMMENDED] |
| Full suite command | `just test` and `pnpm run test`. [VERIFIED: Justfile; VERIFIED: package.json] |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| TEXT-03 | Pinned text layout profile and deterministic ASS/script output | unit/snapshot | `cargo test -p engine_core text_layout -- --nocapture && cargo test -p ffmpeg_compiler ass -- --nocapture` | no, Wave 0 [VERIFIED: crate shells] |
| PREV-01 | Center preview requests a Rust-generated artifact | Electron E2E + binding test | `cargo test -p bindings_node preview_commands -- --nocapture && pnpm --filter @video-editor/desktop test:workspace -g "预览"` | no, Wave 0 [VERIFIED: current preview placeholder] |
| PREV-02 | Seek/scrub deterministic preview frame at microsecond time | unit/integration | `cargo test -p preview_service preview_frame -- --nocapture` | no, Wave 0 |
| PREV-03 | Short preview segment cache from same render path | integration | `cargo test -p preview_service preview_segment_cache -- --nocapture` | no, Wave 0 |
| PREV-04 | Range invalidation keeps unrelated cache entries | unit | `cargo test -p preview_service invalidation -- --nocapture` | no, Wave 0 |
| EXP-01 | H.264 MP4 export preset creates MP4 output | integration/render | `cargo test -p testkit phase5_export_smoke -- --nocapture` | no, Wave 0 |
| EXP-02 | Preview/export compile through same normalized graph | snapshot/integration | `cargo test -p testkit preview_export_parity -- --nocapture` | no, Wave 0 |
| EXP-03 | Progress, cancel, bounded logs, classified errors | runtime integration | `cargo test -p media_runtime export_job_runtime -- --nocapture` | no, Wave 0 |
| EXP-04 | ffprobe validates output duration/fps/resolution/audio/file | integration | `cargo test -p media_runtime output_validation -- --nocapture` | no, Wave 0 |
| TEST-03 | Engine snapshots cover normalization, mapping, stacking, text | unit/snapshot | `cargo test -p engine_core -- --nocapture` | no, Wave 0 |
| TEST-04 | Render graph and FFmpeg job/script snapshots | unit/snapshot | `cargo test -p render_graph -- --nocapture && cargo test -p ffmpeg_compiler -- --nocapture` | no, Wave 0 |
| TEST-05 | Preview frame and exported frame match tolerance | render golden | `cargo test -p testkit preview_export_parity -- --nocapture` | no, Wave 0 |

### Sampling Rate

- **Per task commit:** Run the narrowest crate test plus `pnpm run test:phase5-source-guards` after it exists. [RECOMMENDED]
- **Per wave merge:** Run `pnpm run test:phase5-render-core` and any affected desktop Playwright focused test. [RECOMMENDED]
- **Phase gate:** Run `just build`, `just test`, `pnpm run test:phase5-render-core`, `pnpm run test:phase5-source-guards`, and `git diff --exit-code schemas apps/desktop-electron/src/generated`. [VERIFIED: Justfile; VERIFIED: package.json]

### Wave 0 Gaps

- [ ] Add `crates/engine_core/tests/normalization.rs` for TEST-03. [RECOMMENDED]
- [ ] Add `crates/engine_core/tests/frame_state_snapshots.rs` for TEST-03. [RECOMMENDED]
- [ ] Add `crates/render_graph/tests/render_graph_snapshots.rs` for TEST-04. [RECOMMENDED]
- [ ] Add `crates/ffmpeg_compiler/tests/ffmpeg_job_snapshots.rs` and `ass_snapshots.rs` for TEST-04/TEXT-03. [RECOMMENDED]
- [ ] Add `crates/preview_service/tests/cache_invalidation.rs` and `preview_generation.rs` for PREV-02/PREV-03/PREV-04. [RECOMMENDED]
- [ ] Add `crates/media_runtime/tests/export_job.rs` and `output_validation.rs` for EXP-03/EXP-04. [RECOMMENDED]
- [ ] Add `crates/testkit/tests/preview_export_parity.rs` for TEST-05. [RECOMMENDED]
- [ ] Add `scripts/phase5-source-guards.sh` and root scripts `test:phase5-render-core`, `test:phase5-source-guards`. [RECOMMENDED]
- [ ] Extend `apps/desktop-electron/tests/workspace.spec.ts` or add `preview-export.spec.ts` for Chinese preview/export UI behavior. [RECOMMENDED]

## Security Domain

Security enforcement is enabled because `.planning/config.json` has `workflow.security_enforcement: true`. [VERIFIED: .planning/config.json]

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | Local desktop MVP has no user authentication surface in Phase 5. [VERIFIED: .planning/REQUIREMENTS.md] |
| V3 Session Management | no | No web session/token storage is introduced. [VERIFIED: .planning/REQUIREMENTS.md] |
| V4 Access Control | yes | Restrict renderer to preload command bridge; keep filesystem/process operations in Rust/Electron privileged tiers. [VERIFIED: apps/desktop-electron/src/preload/index.ts; VERIFIED: scripts/phase4-source-guards.sh] |
| V5 Input Validation | yes | Validate draft schema, material URI resolution, output paths, timeranges, render profiles, and FFmpeg job capability preconditions. [VERIFIED: crates/draft_model/src/validation.rs; VERIFIED: crates/project_store/src/paths.rs] |
| V6 Cryptography | no | Phase 5 cache fingerprints are non-security identifiers; do not use them for trust decisions. [RECOMMENDED] |
| V8 Data Protection | yes | Prevent derived artifacts, logs, cache files, and exports from becoming canonical `.veproj/project.json` state. [VERIFIED: AGENTS.md; VERIFIED: crates/draft_model/tests/draft_schema.rs] |
| V12 File and Resources | yes | Output paths, sidecar paths, cache paths, and material paths must be resolved/canonicalized at Rust service boundaries; renderer must not pass arbitrary shell fragments. [VERIFIED: crates/project_store/src/paths.rs; VERIFIED: crates/media_runtime/src/probe.rs] |

### Known Threat Patterns for Phase 5 Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Shell injection through FFmpeg arguments | Tampering / Elevation | Use `Command::new(binary).args(args)` with `OsString` vectors; never shell-concatenate renderer/user input. [VERIFIED: crates/media_runtime/src/process.rs; CITED: https://nodejs.org/api/child_process.html] |
| Path traversal for preview cache/output sidecars | Tampering / Information Disclosure | Resolve cache roots under project/cache temp dirs and reject output sidecar paths outside expected roots. [VERIFIED: crates/project_store/src/paths.rs; RECOMMENDED] |
| Unbounded FFmpeg logs filling memory/UI | Denial of Service | Store bounded stdout/stderr summaries like existing discovery/probe code. [VERIFIED: crates/media_runtime/src/discovery.rs; crates/media_runtime/src/probe.rs] |
| Stale preview cache after edit | Tampering | Invalidate by target range and semantic fingerprint in `preview_service`; never from renderer. [VERIFIED: 05-CONTEXT.md] |
| Malicious media path causing process failure | Denial of Service | Classify runtime/probe/render errors and keep draft state unchanged. [VERIFIED: crates/media_runtime/src/probe.rs; VERIFIED: crates/bindings_node/src/lib.rs] |

## Sources

### Primary (HIGH confidence)
- `.planning/phases/05-preview-and-export-pipeline/05-CONTEXT.md` - locked Phase 5 decisions, boundaries, deferred scope. [VERIFIED: file read]
- `.planning/REQUIREMENTS.md` - Phase 5 requirement IDs and success criteria. [VERIFIED: file read]
- `.planning/ROADMAP.md` - planned Phase 5 wave split and dependencies. [VERIFIED: file read]
- `AGENTS.md` - project constraints for architecture, time model, rendering, references, testing, licensing. [VERIFIED: file read]
- `Cargo.toml`, crate `Cargo.toml` files, and `cargo metadata --locked` - workspace crate topology and versions. [VERIFIED: local command]
- `crates/*/src/lib.rs`, `crates/media_runtime/src/process.rs`, `crates/media_runtime/src/probe.rs`, `crates/bindings_node/src/lib.rs`, `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx`, `scripts/phase4-source-guards.sh` - existing implementation boundaries. [VERIFIED: codebase grep/read]
- Local FFmpeg/ffprobe probes - runtime version, filters, encoders, muxers, fonts. [VERIFIED: local commands]
- FFmpeg official docs - `ffmpeg`, filters, formats, ffprobe behavior. [CITED: https://ffmpeg.org/ffmpeg.html; CITED: https://ffmpeg.org/ffmpeg-filters.html; CITED: https://ffmpeg.org/ffmpeg-formats.html; CITED: https://ffmpeg.org/ffprobe.html]

### Secondary (MEDIUM confidence)
- npm registry via `npm view` - current Electron/React/Playwright/Vite/TypeScript package versions and repositories. [VERIFIED: npm registry]
- crates.io index via `cargo search` - existing Rust dependency versions. [VERIFIED: cargo search]
- Node.js child process official docs - process spawn/cancel/security pattern for comparison; Rust implementation should stay in `media_runtime`. [CITED: https://nodejs.org/api/child_process.html]

### Tertiary (LOW confidence)
- General claims about preview/export drift, cache invalidation failure modes, and cross-machine pixel brittleness are marked `[ASSUMED]` where used. [ASSUMED]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - local crate/package versions and FFmpeg capabilities were verified. [VERIFIED: cargo metadata; VERIFIED: npm registry; VERIFIED: local FFmpeg probes]
- Architecture: HIGH - phase context and crate boundary comments are explicit; exact API names remain planner discretion. [VERIFIED: 05-CONTEXT.md; VERIFIED: crate shells]
- Pitfalls: MEDIUM - major risks are grounded in current code and official FFmpeg docs, with ecosystem behavior assumptions logged separately. [VERIFIED: codebase; CITED: ffmpeg docs; ASSUMED]

**Research date:** 2026-06-17 [VERIFIED: local date/environment]
**Valid until:** 2026-07-17 for codebase-local recommendations; re-check FFmpeg/npm package versions and CI runtime capabilities before implementation if delayed beyond 30 days. [ASSUMED]

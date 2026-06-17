# Phase 5: Preview And Export Pipeline - Context

**Gathered:** 2026-06-17
**Status:** Ready for planning
**Mode:** Autonomous smart discuss, using prior user direction to continue without pausing for every default

<domain>
## Phase Boundary

Phase 5 connects the Rust-owned draft and timeline semantics to the first real preview/export pipeline. It must deliver deterministic normalized draft state, resolved frame state, a typed render graph, FFmpeg compiler outputs, preview frame/segment generation with cache invalidation, and H.264 MP4 export with progress, cancellation, logs, classified errors, and executable golden gates.

This phase does not introduce mobile/server runtimes, Jianying/CapCut draft adapters, GPU real-time rendering, advanced proprietary effects, packaged release checks, or FFmpeg binary redistribution. Phase 6 owns packaging and release hardening.

</domain>

<decisions>
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

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/draft_model` already owns Jianying-aligned `Draft`, `Material`, `Track`, `Segment`, `SourceTimerange`, `TargetTimerange`, `TextSegment`, `SegmentVolume`, `Filter`, and `Transition` contracts.
- `crates/draft_commands` already owns timeline edits, undo/redo, snapping, main-track magnet behavior, text edits, and audio volume/mute commands.
- `crates/media_runtime` and `crates/media_runtime_desktop` already provide FFmpeg/ffprobe binary discovery, process execution, material probing, timeout behavior, and a desktop `FfmpegExecutor`.
- `crates/testkit` already generates tiny media fixtures and has render smoke helpers for FFmpeg-backed tests.
- `apps/desktop-electron` already has a Chinese Jianying-style workspace, command-only helpers, a preview monitor placeholder, and Playwright workspace tests.

### Established Patterns
- Rust serde/ts-rs types are the source of truth for command/schema/TypeScript generated contracts.
- Root `just test` and `pnpm run test` are the public gates; phase-specific scripts are added to package scripts and chained into the root gate.
- Electron renderer calls `window.videoEditorCore.executeCommand` and receives standardized `ok/error/events` envelopes.
- Derived media artifacts are excluded from `.veproj/project.json`; project persistence stays in `project_store`, while runtime/probing stays in media service boundaries.

### Integration Points
- `engine_core/src/lib.rs`, `render_graph/src/lib.rs`, `ffmpeg_compiler/src/lib.rs`, and `preview_service/src/lib.rs` are current Phase 5 implementation shells.
- `bindings_node/src/lib.rs` is the Electron-facing command router that will need preview/export command variants or equivalent Rust-owned APIs.
- `draft_model/src/lib.rs` and `draft_model/tests/schema_exports.rs` are the generated contract/schema update points.
- `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx`, `App.tsx`, `commandHelpers.ts`, `viewModel.ts`, and workspace tests are the desktop integration points.
- `scripts/phase4-source-guards.sh` and package scripts show the existing source guard style to extend for Phase 5.

</code_context>

<specifics>
## Specific Ideas

- User wants a general Jianying-like editor, not oral-video tooling.
- User wants internal and external terminology to stay Jianying-aligned instead of inventing separate internal names.
- User wants the project to stay layered for future desktop/mobile/server expansion, but Phase 5 should implement the desktop MVP path first.
- Kdenlive and MLT are conceptual references for monitor/jobs/render separation and media-engine abstractions; do not copy their GPL code, assets, XML definitions, presets, or UI implementation.
- pyJianYingDraft is a vocabulary reference only; `.veproj/project.json` remains the canonical draft format.
- Desktop visible language is Simplified Chinese.
- Each step must be testable.

</specifics>

<deferred>
## Deferred Ideas

- Packaged app offline launch, FFmpeg distribution manifests, third-party notices, and release hardening belong to Phase 6.
- Jianying/CapCut/Kaipai draft import/export adapters remain post-MVP.
- GPU real-time preview, mobile/server FFmpeg runtimes, nested sequences, advanced masks, proprietary effects, and full effect preset parity remain post-MVP.

</deferred>

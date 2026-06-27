# Phase 13: Incremental Render Graph, Dirty Ranges, And Cache Coherence - Context

**Gathered:** 2026-06-18
**Status:** Ready for research and design
**Source:** User request for Phase 13 context/research/design artifacts

<domain>
## Phase Boundary

Phase 13 makes accepted draft edits produce enough semantic change metadata for incremental render graph updates and deterministic cache invalidation. The phase owns `CommandDelta`, stable render graph node identity, node/content/runtime fingerprints, dirty range propagation, and undo/redo cache coherence strategy.

The phase does not implement Phase 14's SQLite artifact store, blob layout, resumable artifact generation, storage quotas, or garbage collection. It does not implement Phase 16's priority scheduler. It should stage contracts so those phases can consume the same dirty ranges, node IDs, fingerprints, and invalidation reasons without redesign.

</domain>

<decisions>
## Locked Decisions

### Scope

- Write scope for this research/design task is limited to:
  - `.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-CONTEXT.md`
  - `.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-RESEARCH.md`
  - `.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-DESIGN.md`
- Do not modify product source files while producing these artifacts.
- Do not modify Phase 10.1 files.
- Do not commit.

### Requirements

- Phase 13 must satisfy INCR-01 through INCR-05:
  - Stable render graph node identities tied to semantic draft entities, with separate fingerprints for content, inputs, and runtime capabilities.
  - Accepted commands emit `CommandDelta` data with changed entity IDs, changed domains, and changed integer-microsecond ranges.
  - Dirty range propagation spans preview, export preparation, audio, thumbnails, waveforms, proxies, graph snapshots, and preview cache using integer/rational time.
  - Undo/redo restores semantic state and either restores matching graph/cache snapshots or invalidates affected ranges deterministically.
  - Large-timeline tests verify graph diff cost, dirty range accuracy, and preview/export consistency after localized edits.

### Architecture

- UI emits commands; Rust core owns project and timeline semantics.
- UI code must not directly construct FFmpeg commands, render graphs, graph diffs, dirty ranges, cache keys, or cache invalidation decisions.
- `.veproj/project.json` remains the only canonical semantic source of truth.
- Render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, preview caches, graph snapshots, and derived artifact metadata remain rebuildable derived artifacts.
- `draft_model`, `draft_commands`, and `engine_core` stay pure semantic crates. They must not depend on preview runtime, artifact store, filesystem, FFmpeg execution, Electron, or platform APIs.
- Render Graph isolates editing semantics from FFmpeg. FFmpeg Runtime executes jobs and reports progress/errors; it does not decide editing behavior.

### Terminology And Time

- Use Jianying-aligned concepts: `draft`, `material`, `track`, `segment`, `keyframe`, `filter`, `transition`, `render graph`, `dirty range`, `preview`, `export`, `proxy`, `thumbnail`, `waveform`.
- Avoid parallel terms such as `asset`, `clip`, or UI-only graph/cache vocabulary when existing project terminology is clear.
- Persisted and contract time values must use integer microseconds, frame indices, or rational frame rates.
- Do not introduce naked `f32`/`f64` time into persisted draft semantics, command deltas, render graph identity, dirty ranges, or cache invalidation contracts.

### Node Identity And Fingerprints

- Render graph node identity must be stable and derived from semantic draft entities and graph role, not from content hashes alone.
- Fingerprints describe the current content, inputs, output profile, schema/generator version, and runtime capability state of a node or artifact.
- Node identity answers "what semantic thing is this"; fingerprints answer "is this exact output still valid."

### Dirty Propagation

- Dirty propagation must be domain-aware. Timing, visual, text, audio, material, effect/filter, transition, canvas/profile, runtime capability, and derived artifact changes should not collapse into an untyped "everything changed" event except for explicit full-invalidate fallbacks.
- Dirty ranges should be merged deterministically and carried as `TargetTimerange` values, using integer microseconds.
- Propagation must cover preview, export preparation, audio, thumbnails, waveforms, proxies, render graph snapshots, and preview cache.

### Undo/Redo

- Undo/redo must restore semantic state first.
- Graph/cache coherence may use matching session snapshots where cheap and available, but must always have a deterministic invalidation fallback.
- Cache correctness is more important than preserving stale artifacts across undo/redo.

### Phase Staging

- Phase 13 should define in-memory and command/binding-safe contracts.
- Phase 14 will persist artifact metadata in a project-local SQLite index and blob directories.
- Phase 16 will schedule preview, decode, artifact, proxy, waveform, export, and IO jobs using dirty work units and playback generation.

</decisions>

<canonical_refs>
## Canonical References

Downstream agents MUST read these before planning or implementation:

- `AGENTS.md`
- `.planning/PROJECT.md`
- `.planning/ROADMAP.md` Phase 13 section
- `.planning/REQUIREMENTS.md` INCR requirements
- `.planning/notes/production-editor-architecture-decisions.md`
- `.planning/research/questions.md`
- `docs/runtime-boundaries.md`
- `crates/draft_model/src/lib.rs`
- `crates/draft_model/src/time.rs`
- `crates/draft_model/src/timeline.rs`
- `crates/draft_commands/src/history.rs`
- `crates/draft_commands/src/timeline.rs`
- `crates/draft_commands/src/audio.rs`
- `crates/draft_commands/src/text.rs`
- `crates/draft_commands/src/visual.rs`
- `crates/draft_commands/src/keyframe.rs`
- `crates/draft_commands/src/canvas.rs`
- `crates/engine_core/src/normalize.rs`
- `crates/engine_core/src/frame_state.rs`
- `crates/render_graph/src/graph.rs`
- `crates/render_graph/src/profile.rs`
- `crates/preview_service/src/cache.rs`
- `crates/preview_service/src/service.rs`
- `crates/bindings_node/src/preview_export_service.rs`
- `crates/preview_service/tests/cache_invalidation.rs`
- `package.json`
- `justfile`

</canonical_refs>

<specifics>
## Design Questions To Resolve

- What exact `CommandDelta` shape should commands return, and should selection-only commands emit an empty/no-op delta?
- Which entity identifiers are needed for draft, material, track, segment, graph node, cache/artifact, and runtime capability changes?
- Which dirty domains are required now, and which domains should be reserved for Phase 14, Phase 15, Phase 16, and Phase 18?
- How should old and new ranges be represented for move, trim, split, delete, keyframe, track mute, canvas/profile, material relink, and runtime capability changes?
- What stable render graph node ID scheme covers material, canvas, video segment, audio segment, text overlay, transition, filter/effect, mix, output, and sampled frame roles?
- How should graph diff metadata distinguish unchanged node identity with changed fingerprint from added/removed node identity?
- Which cache invalidation APIs should remain in `preview_service`, and which should become graph/artifact-store-facing contracts later?
- What should undo/redo store in session history: draft snapshots only, graph snapshots, fingerprint maps, or invalidation replay data?

</specifics>

<deferred>
## Deferred Ideas

- Phase 14: project-local SQLite artifact index, blob storage, artifact GC, storage quotas, resumable generation, and on-disk graph snapshot persistence.
- Phase 15: independent low-latency audio engine and DSP graph implementation.
- Phase 16: priority-aware task runtime, cancellation, starvation control, queue telemetry, and background artifact scheduling.
- Phase 17: portable handle registry and C ABI/mobile/server binding contracts.
- Phase 18: production effects recovery, retiming, transitions, masks, blends, and template fidelity gates.
- Direct GPL/Kdenlive/MLT runtime integration and proprietary Jianying/CapCut effect parity.

</deferred>

---

*Phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence*
*Context gathered: 2026-06-18*

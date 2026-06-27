# Phase 14: asset-resource-manager-and-derived-artifact-store - Context

**Gathered:** 2026-06-19 (assumptions mode)
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 14 adds a production resource and derived-artifact layer for the editor. It owns the project-local `.veproj/derived` storage boundary, a SQLite artifact index at `.veproj/derived/artifact-store.sqlite`, stable resource/dependency identity, chunked/resumable/cancellable derived generation contracts, and DB-driven cache GC/quota/sync manifest definitions for local artifacts. It must not change `.veproj/project.json` into an artifact database, and it must not move render, cache, fingerprint, or invalidation semantics into the Electron renderer.

</domain>

<decisions>
## Implementation Decisions

### Storage Boundary
- **D-01:** Add a Rust-owned derived artifact store under `.veproj/derived/`, with `.veproj/derived/artifact-store.sqlite` as the validity index and project-relative blob paths for generated outputs. `.veproj/project.json` remains canonical semantic state only.
- **D-02:** The artifact store must be rebuildable. Artifact rows, thumbnails, waveforms, proxies, graph snapshots, preview artifacts, FFmpeg sidecars, and generation/job rows must not be serialized into draft/project semantic JSON.

### Resource Identity And Dependencies
- **D-03:** Artifact dependencies must be keyed by stable semantic/resource identities: `MaterialId`, project-relative material references, render graph stable node keys, graph/source/runtime/output fingerprints, dirty domains, and integer dirty ranges.
- **D-04:** Fonts and supported effect resources enter the artifact index as resource/dependency rows derived from existing text/filter/transition references. Do not introduce new canonical `Material` variants for cache-only font/effect facts unless a later semantic phase requires it.
- **D-05:** Replacement, relink, rename, and delete handling must invalidate or regenerate artifacts by dependency rows and dirty facts from Rust-owned command/render semantics. Broad full-draft invalidation is allowed only for explicitly recorded overflow or unknown-dependency cases, not as the normal path.

### Generation Lifecycle, GC, And Sync Manifests
- **D-06:** Proxy, thumbnail, waveform, graph snapshot, and preview artifact generation must persist generation status and chunk/job rows in the artifact store so work can be resumed or cancelled after interruption.
- **D-07:** Phase 14 may define scheduler-compatible generation contracts and use existing cancellation primitives, but full priority scheduling/backpressure remains Phase 16 scope.
- **D-08:** GC, quota, and sync manifest behavior must be DB-driven by artifact rows, dependency rows, byte accounting, tombstones, relative blob paths, and content/source fingerprints. Remote-provider protocols and cloud rendering transport are out of scope; Phase 14 defines the local manifest contract future remote phases consume.

### Binding And UI Boundary
- **D-09:** Bindings and desktop UI may expose artifact status, generation progress, cancellation, quota/GC maintenance, and displayable project-relative artifact refs. TypeScript must not compute artifact roots, cache keys, fingerprints, invalidation scopes, or SQLite behavior.
- **D-10:** The current renderer-provided `/tmp/video-editor-preview-cache` path is transitional. Planning should move preview/artifact path resolution into Rust/project-store owned APIs and preserve renderer only as command transport and presentation.

### the agent's Discretion
The planner may choose concrete crate boundaries, table names, and command names as long as the plan preserves the decisions above and the existing Rust ownership model. The local sync manifest format can be minimal, but it must be deterministic, project-relative, fingerprinted, and sufficient for later server/mobile artifact reconciliation.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Contracts
- `.planning/PROJECT.md` — Project architecture, canonical project format, Rust ownership constraints, and production editor goal.
- `.planning/REQUIREMENTS.md` — ASSET-01 through ASSET-05 acceptance requirements.
- `.planning/ROADMAP.md` — Phase 14 goal, dependency on Phase 13, UI hint, and later Phase 15/16 consumers.
- `.planning/STATE.md` — Prior decisions, including renderer/source-guard boundaries and Phase 13 completion state.

### Phase 13 Dependency
- `.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-CONTEXT.md` — Locked decisions for dirty ranges, graph node IDs, and renderer ownership boundaries.
- `.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-DESIGN.md` — Incremental graph and cache coherence design that Phase 14 persists.
- `.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-VERIFICATION.md` — Verified Phase 13 behavior and must-haves.
- `.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-SECURITY.md` — Dirty-range overflow/security follow-up that Phase 14 must preserve.

### Phase 14 Research
- `.planning/phases/14-asset-resource-manager-and-derived-artifact-store/14-RESEARCH.md` — Technical research for SQLite artifact index, derived blob store, resource manager, invalidation, generation lifecycle, and verification strategy.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/project_store/src/lib.rs` and `crates/project_store/src/paths.rs` already own project-bundle path semantics, project-relative references, external source classification, and traversal rejection.
- `crates/draft_model/src/validation.rs` rejects derived artifact leakage into draft/project JSON.
- `crates/draft_commands/src/delta.rs` already produces material dependency deltas, dirty ranges, dirty domains, and invalidation scopes.
- `crates/render_graph/src/incremental.rs` and `crates/render_graph/src/fingerprint.rs` provide stable graph node keys and graph/source/runtime/output fingerprint concepts.
- `crates/preview_service/src/cache.rs` and generated preview contracts expose the current transitional preview cache/invalidation surface that Phase 14 should replace or route through Rust-owned artifact paths.
- `crates/media_runtime/src/job.rs` has cancellation primitives that can be reused for artifact generation job cancellation semantics.

### Established Patterns
- Rust crates own semantic decisions and expose typed contracts to `bindings_node`; generated TypeScript is a transport/presentation contract, not an authority for cache or render semantics.
- Source guards already reject renderer-owned dirty range, graph diff, graph node ID, cache key, invalidation, and FFmpeg command decisions.
- Project bundle tests guard against derived fields inside `project.json`, so Phase 14 should add derived storage alongside the project bundle rather than expanding semantic schema for caches.

### Integration Points
- `project_store` is the likely home for project-local path resolution and `.veproj/derived` location helpers.
- A dedicated Rust artifact/resource layer can sit beside `preview_service`, `render_graph`, and `media_runtime`, consuming their facts rather than replacing their semantics.
- `bindings_node` should expose artifact/resource commands only after Rust APIs decide paths, fingerprints, invalidation, generation status, and GC outcomes.
- Desktop UI should eventually display resource/artifact status as production UI state, but debug paths, cache roots, and SQLite internals should stay hidden by default.

</code_context>

<specifics>
## Specific Ideas

- Use a project-local SQLite index named `.veproj/derived/artifact-store.sqlite`.
- Store blob paths project-relative to `.veproj/derived`, not absolute desktop paths.
- Persist dependency rows for materials, graph nodes, dirty domains/ranges, runtime capability fingerprint, output profile fingerprint, source material fingerprint, graph fingerprint, generation parameters, schema version, and generator version.
- Define a local sync manifest as a deterministic derived artifact manifest over relative paths, fingerprints, byte sizes, dependency rows, generation status, and tombstones. Remote protocol details stay deferred.

</specifics>

<deferred>
## Deferred Ideas

- Full scheduler priority/backpressure and interactive job orchestration belong to Phase 16.
- Remote-provider/cloud-rendering transport and server sync protocol implementation belong to later mobile/server/cloud phases.
- Large Jianying-style UI cleanup remains a product/UI alignment workstream; Phase 14 should only expose resource/artifact status needed for production semantics and should avoid debug-heavy UI.

</deferred>

---

*Phase: 14-asset-resource-manager-and-derived-artifact-store*
*Context gathered: 2026-06-19*

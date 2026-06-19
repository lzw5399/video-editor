# Phase 14: asset-resource-manager-and-derived-artifact-store - Discussion Log (Assumptions Mode)

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions captured in CONTEXT.md — this log preserves the analysis.

**Date:** 2026-06-19
**Phase:** 14-asset-resource-manager-and-derived-artifact-store
**Mode:** assumptions --auto
**Areas analyzed:** Storage Boundary, Resource Identity And Dependencies, Generation Lifecycle And GC, Binding And UI Boundary

## Assumptions Presented

### Storage Boundary

| Assumption | Confidence | Evidence |
|------------|------------|----------|
| Phase 14 should add a Rust-owned derived artifact store under `.veproj/derived/`, with `.veproj/derived/artifact-store.sqlite` as the validity index and project-relative blob paths for generated outputs; `project.json` remains semantic only. | Confident | `.planning/ROADMAP.md`, `.planning/PROJECT.md`, `crates/project_store/src/lib.rs`, `crates/draft_model/src/validation.rs`, `crates/project_store/tests/project_bundle.rs` |

### Resource Identity And Dependencies

| Assumption | Confidence | Evidence |
|------------|------------|----------|
| Artifact dependencies should be keyed by stable semantic/resource identities: `MaterialId`, validated project-relative material references, render graph stable node keys, graph/source/runtime/output fingerprints, dirty domains, and integer dirty ranges. Fonts and supported effects should enter the artifact index as resource/dependency rows derived from existing text/filter/transition refs, not as new canonical `Material` variants by default. | Likely | `crates/draft_model/src/material.rs`, `crates/project_store/src/paths.rs`, `crates/draft_model/src/delta.rs`, `crates/draft_commands/src/delta.rs`, `crates/render_graph/src/incremental.rs`, `crates/render_graph/src/fingerprint.rs` |

### Generation Lifecycle And GC

| Assumption | Confidence | Evidence |
|------------|------------|----------|
| Proxy, thumbnail, waveform, graph snapshot, and preview artifact generation should persist job/chunk/status rows in the artifact store and use existing runtime cancellation primitives, while full priority scheduling/backpressure remains Phase 16 scope. GC/quota/sync-manifest work should be DB-driven by artifact rows, dependency rows, byte accounting, tombstones, and relative blob fingerprints. | Likely | `.planning/REQUIREMENTS.md`, `crates/media_runtime/src/job.rs`, `.planning/phases/14-asset-resource-manager-and-derived-artifact-store/14-RESEARCH.md`, `.planning/notes/production-editor-architecture-decisions.md` |

### Binding And UI Boundary

| Assumption | Confidence | Evidence |
|------------|------------|----------|
| Bindings and desktop UI should expose artifact status, progress, cancellation, quota/GC maintenance, and displayable project-relative artifact refs only; TypeScript should not compute artifact roots, cache keys, fingerprints, invalidation scope, or SQLite behavior. The current renderer-supplied `/tmp` preview `cacheRoot` should be treated as transitional. | Confident | `apps/desktop-electron/src/renderer/App.tsx`, `apps/desktop-electron/src/generated/CommandEnvelope.ts`, `crates/bindings_node/src/lib.rs`, `scripts/phase13-source-guards.sh`, `.planning/phases/14-asset-resource-manager-and-derived-artifact-store/14-RESEARCH.md` |

## Corrections Made

No corrections — auto mode accepted Confident/Likely assumptions.

## Auto-Resolved

- Optional cloud/server synchronization manifest shape beyond local relative paths and fingerprints: resolved as a local deterministic manifest contract for Phase 14. Remote-provider protocols and cloud rendering transport stay deferred.

## External Research

No new external research was performed during assumptions mode. Phase 14 already has `14-RESEARCH.md`, and the only remaining research topic is remote sync protocol detail outside this phase's implementation boundary.

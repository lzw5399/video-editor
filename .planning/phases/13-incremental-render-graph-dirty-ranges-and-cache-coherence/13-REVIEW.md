---
phase: 13-incremental-render-graph-dirty-ranges-and-cache-coherence
reviewed: "2026-06-19T02:08:39Z"
depth: standard
files_reviewed: 41
files_reviewed_list:
  - apps/desktop-electron/src/generated/CommandEnvelope.ts
  - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
  - apps/desktop-electron/src/renderer/commandHelpers.ts
  - crates/bindings_node/src/lib.rs
  - crates/bindings_node/src/preview_export_service.rs
  - crates/bindings_node/tests/export_commands.rs
  - crates/bindings_node/tests/preview_commands.rs
  - crates/draft_commands/src/audio.rs
  - crates/draft_commands/src/canvas.rs
  - crates/draft_commands/src/delta.rs
  - crates/draft_commands/src/history.rs
  - crates/draft_commands/src/keyframe.rs
  - crates/draft_commands/src/lib.rs
  - crates/draft_commands/src/text.rs
  - crates/draft_commands/src/timeline.rs
  - crates/draft_commands/src/visual.rs
  - crates/draft_commands/tests/command_delta.rs
  - crates/draft_model/src/delta.rs
  - crates/draft_model/src/lib.rs
  - crates/draft_model/src/timeline.rs
  - crates/draft_model/tests/contract.rs
  - crates/draft_model/tests/schema_exports.rs
  - crates/preview_service/src/cache.rs
  - crates/preview_service/src/lib.rs
  - crates/preview_service/src/service.rs
  - crates/preview_service/tests/cache_invalidation.rs
  - crates/preview_service/tests/dirty_propagation.rs
  - crates/render_graph/Cargo.toml
  - crates/render_graph/src/fingerprint.rs
  - crates/render_graph/src/graph.rs
  - crates/render_graph/src/incremental.rs
  - crates/render_graph/src/lib.rs
  - crates/render_graph/tests/canvas_background.rs
  - crates/render_graph/tests/node_identity.rs
  - crates/render_graph/tests/render_graph_snapshots.rs
  - crates/testkit/src/large_timeline.rs
  - crates/testkit/src/lib.rs
  - crates/testkit/tests/large_timeline_incremental.rs
  - crates/testkit/tests/preview_export_parity.rs
  - schemas/command.schema.json
  - scripts/phase13-source-guards.sh
findings:
  critical: 2
  warning: 1
  info: 0
  total: 3
status: issues_found
---

# Phase 13: Code Review Report

**Reviewed:** 2026-06-19T02:08:39Z
**Depth:** standard
**Files Reviewed:** 41
**Status:** issues_found

## Summary

Reviewed the Phase 13 command delta, render graph fingerprint, preview cache invalidation, desktop command helpers, binding, tests, generated contracts, and source guard surfaces. The implementation has correctness defects in the export cancellation state machine and in the desktop helpers that are supposed to carry the new dirty/cache coherence facts. There is also an incomplete graph-node fingerprint surface for filter/transition node IDs.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: BLOCKER - Cancelled export jobs can be overwritten as running/completed

**File:** `crates/bindings_node/src/preview_export_service.rs:567`

**Issue:** `cancel()` marks an active job as `Cancelled` at lines 567-584, but the background export thread still applies runtime events and final states without checking the current terminal phase. `apply_runtime_event()` rewrites progress events back to `Running` at lines 681-691, `mark_validating()` rewrites the job to `Validating` at lines 703-711, and the completed validation path rewrites it to `Completed` at lines 602-618. If cancellation races with FFmpeg exit or validation, status polling can report a user-cancelled job as running or completed, which is incorrect user-visible behavior.

**Fix:**

```rust
fn update_status_if_not_terminal(
    &self,
    job_id: &str,
    update: impl FnOnce(&mut ExportJobStatusResponse),
) -> Result<(), ExportCommandError> {
    self.update_status(job_id, |status| {
        if matches!(
            status.phase,
            ExportJobPhase::Completed
                | ExportJobPhase::Failed
                | ExportJobPhase::ValidationFailed
                | ExportJobPhase::Cancelled
        ) {
            return;
        }
        update(status);
    })
}
```

Use this guarded update for runtime progress, validating, validation success/failure, and runtime failure paths, or explicitly check `cancel_token.is_cancelled()` before moving from FFmpeg completion into validation/completed.

### CR-02: BLOCKER - Desktop command builders drop Phase 13 dirty/cache facts

**File:** `apps/desktop-electron/src/renderer/commandHelpers.ts:487`

**Issue:** The generated command contracts expose `changedGraphNodeIds`, `changedDomains`, runtime/output fingerprints, `fullDraft`, schema version, generator version, and export `dirtyFacts`, but the hand-written helpers do not allow callers to pass them. `buildInvalidatePreviewCacheCommand()` only sends `entries`, `changedRanges`, `changedMaterialIds`, and `reason` at lines 487-501, so UI invalidations cannot target graph-node or runtime/output-profile changes. `buildStartExportCommand()` only sends `draft`, `outputPath`, and `preset` at lines 506-518, so export jobs started through the helper lose `ExportPrepDirtyFacts`. This breaks the Phase 13 product path even though Rust-side contracts support the data.

**Fix:**

```ts
type InvalidatePreviewCacheOptions = {
  entries: PreviewCacheEntryRef[];
  changedRanges: DirtyRange[];
  changedMaterialIds: MaterialId[];
  changedGraphNodeIds?: string[];
  changedDomains?: InvalidatePreviewCacheCommandPayload["changedDomains"];
  runtimeCapabilityFingerprint?: string | null;
  outputProfileFingerprint?: string | null;
  fullDraft?: boolean;
  reason: string;
  artifactSchemaVersion?: number;
  generatorVersion?: string;
};

type StartExportOptions = {
  draft: Draft;
  outputPath: string;
  preset: ExportPreset;
  dirtyFacts?: StartExportCommandPayload["dirtyFacts"];
};
```

Forward every optional field into the payload so the desktop UI can use the same v2 dirty facts that the Rust binding and schema already define.

## Warnings

### WR-01: WARNING - Filter and transition graph node IDs are exposed but never fingerprinted

**File:** `crates/render_graph/src/fingerprint.rs:80`

**Issue:** `RenderGraphNodeId` defines stable IDs for `SegmentFilter` and `SegmentTransition`, and `RenderFilterIntent` / `RenderTransitionIntent` carry those IDs in `crates/render_graph/src/graph.rs:172` and `crates/render_graph/src/graph.rs:182`. However `node_fingerprints()` only emits fingerprints for canvas, materials, video layers, audio mixes, text overlays, and sampled frames at lines 80-126. Preview cache keys are derived from snapshot node fingerprints in `crates/preview_service/src/service.rs:388`, so cache entries never contain filter or transition node keys. Any invalidation that targets those stable node IDs cannot match existing entries.

**Fix:** Add node fingerprints for filter and transition intents, or stop exposing those IDs as independently targetable graph nodes until they participate in snapshots. For example, extend `node_fingerprints()` with per-layer filter and transition fingerprints that use the existing `RenderFilterIntent.node_id` and `RenderTransitionIntent.node_id`.

---

_Reviewed: 2026-06-19T02:08:39Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_

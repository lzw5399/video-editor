---
phase: 14-asset-resource-manager-and-derived-artifact-store
reviewed: 2026-06-19T06:47:49Z
depth: standard
files_reviewed: 54
files_reviewed_list:
  - .github/workflows/ci.yml
  - Cargo.toml
  - package.json
  - schemas/command.schema.json
  - scripts/phase13-source-guards.sh
  - scripts/phase14-source-guards.sh
  - apps/desktop-electron/src/generated/CommandEnvelope.ts
  - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
  - apps/desktop-electron/src/main/index.ts
  - apps/desktop-electron/src/renderer/App.tsx
  - apps/desktop-electron/src/renderer/commandHelpers.ts
  - apps/desktop-electron/src/renderer/styles.css
  - apps/desktop-electron/src/renderer/viewModel.ts
  - apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx
  - apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx
  - apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx
  - apps/desktop-electron/tests/workspace.spec.ts
  - crates/artifact_store/Cargo.toml
  - crates/artifact_store/src/blob_store.rs
  - crates/artifact_store/src/dependencies.rs
  - crates/artifact_store/src/error.rs
  - crates/artifact_store/src/fingerprint.rs
  - crates/artifact_store/src/gc.rs
  - crates/artifact_store/src/generation.rs
  - crates/artifact_store/src/invalidation.rs
  - crates/artifact_store/src/jobs.rs
  - crates/artifact_store/src/lib.rs
  - crates/artifact_store/src/manifest.rs
  - crates/artifact_store/src/paths.rs
  - crates/artifact_store/src/quota.rs
  - crates/artifact_store/src/resource_index.rs
  - crates/artifact_store/src/schema.rs
  - crates/artifact_store/tests/artifact_generation.rs
  - crates/artifact_store/tests/artifact_jobs.rs
  - crates/artifact_store/tests/blob_store.rs
  - crates/artifact_store/tests/gc_quota_manifest.rs
  - crates/artifact_store/tests/invalidation.rs
  - crates/artifact_store/tests/resource_index.rs
  - crates/artifact_store/tests/sqlite_schema.rs
  - crates/bindings_node/Cargo.toml
  - crates/bindings_node/src/artifact_store_service.rs
  - crates/bindings_node/src/lib.rs
  - crates/bindings_node/src/material_service.rs
  - crates/bindings_node/src/preview_export_service.rs
  - crates/bindings_node/src/realtime_preview_service.rs
  - crates/bindings_node/tests/artifact_store_commands.rs
  - crates/bindings_node/tests/preview_commands.rs
  - crates/draft_model/src/lib.rs
  - crates/draft_model/src/material.rs
  - crates/draft_model/src/validation.rs
  - crates/draft_model/tests/schema_exports.rs
  - crates/preview_service/Cargo.toml
  - crates/preview_service/src/service.rs
  - crates/preview_service/tests/preview_generation.rs
findings:
  critical: 2
  warning: 2
  info: 0
  total: 4
status: issues_found
---

# Phase 14: Code Review Report

**Reviewed:** 2026-06-19T06:47:49Z
**Depth:** standard
**Files Reviewed:** 54
**Status:** issues_found

## Summary

Reviewed the Phase 14 artifact store, bindings, generated command contracts, renderer command plumbing, source guards, and tests. The implementation has serious durable-state bugs in the artifact generation path: failed and cancelled generation can leave jobs permanently active, and generated material artifacts are not linked to the material/resource dependencies that the invalidation layer uses. GC and schema-contract coverage also have production risks.

## Critical Issues

### CR-01: BLOCKER - Failed or cancelled generation leaves jobs stuck active

**File:** `crates/artifact_store/src/generation.rs:335`
**Issue:** `run_generation` marks a chunk `running` at line 335, then returns directly on cancellation, generator errors, empty output, or MIME mismatch at lines 336-362. None of those paths call `fail_generation_chunk` or `acknowledge_generation_cancelled`, even though the job APIs support those terminal states. Because `next_pending_chunk` only resumes `waiting`, `failed`, or `cancelled` chunks, these errors leave chunks stuck as `running` and jobs stuck as `running` or `cancelRequested`; retry/resume/status then misreport the task as active forever.
**Fix:** Persist a terminal job/chunk state before returning errors after `start_generation_chunk`. On generator/validation errors call `fail_generation_chunk`, and on either cancellation branch call `acknowledge_generation_cancelled`. Add tests that assert durable status after generator failure, empty bytes, MIME mismatch, and mid-chunk cancellation.

```rust
start_generation_chunk(&mut store, &request.job_id, chunk.chunk_index)?;
if generation_cancel_requested(&store, &request.job_id)? {
    acknowledge_generation_cancelled(&mut store, &request.job_id)?;
    return generation_cancelled(&request.job_id);
}

let generated = match generate(generator, &context) {
    Ok(generated) => generated,
    Err(error) => {
        fail_generation_chunk(&mut store, &request.job_id, chunk.chunk_index, &error.to_string())?;
        return Err(error);
    }
};
```

### CR-02: BLOCKER - Generated artifacts are not invalidated by material/resource changes

**File:** `crates/artifact_store/src/generation.rs:151`
**Issue:** `ProxyGenerationRequest`, `ThumbnailGenerationRequest`, and `WaveformGenerationRequest` all carry `resource_id`, `material_id`, and `source_ref` (lines 127-132, 172-177, and 216-221), but `into_generation_request` drops those fields and only persists generic fingerprints and chunks (lines 151-166, 195-210, 241-256). Later, `upsert_generation_dependencies` records generation parameters plus generic `source`, `runtime`, `output`, and `graph` fingerprint rows (lines 419-467), but no `ArtifactDependency::material(...)` or `ArtifactDependency::resource(...)`. The invalidation layer finds stale artifacts from changed material/resource IDs, so proxies, thumbnails, and waveforms produced by this path can remain `ready` after source relinks, deletes, or material edits.
**Fix:** Carry material/resource identity into the dependency upsert. Either extend `ArtifactGenerationRequest` with `material_id`, `resource_id`, and `source_ref`, or keep typed request-specific dependency insertion before completion. Persist at least material and resource dependencies, and use a source fingerprint key that can be correlated to that material/resource.

```rust
dependencies.push(DependencyUpsert::new(ArtifactDependency::material(material_id.as_str())));
dependencies.push(DependencyUpsert::new(ArtifactDependency::resource(resource_id.as_str())));
dependencies.push(DependencyUpsert::new(
    ArtifactDependency::source_fingerprint(DependencyFingerprint::new(
        format!("source:{}", material_id.as_str()),
        source_fingerprint,
    )),
));
```

Add regression tests that generate each artifact type, call `mark_dirty_from_command_delta` with the material ID and `mark_dirty_for_source_change` for relink/delete, then assert the generated artifact becomes dirty or tombstoned.

## Warnings

### WR-01: WARNING - GC excludes stale generated artifacts that still have dependency rows

**File:** `crates/artifact_store/src/gc.rs:77`
**Issue:** GC candidates require `status IN ('dirty','failed','tombstoned')` and `artifact_id NOT IN (SELECT artifact_id FROM artifact_dependency)` (lines 77-83). `live_artifact_ids` also treats every artifact with any dependency row as live (lines 271-285). Real generated artifacts always receive dependency rows from `upsert_generation_dependencies`, so dirty/failed/tombstoned artifacts created by normal generation are never reclaimable unless another path deletes their dependencies first. The existing GC tests only cover dependency-free stale artifacts, so quota cleanup can look successful in tests while leaking derived blobs in production.
**Fix:** Do not use the mere existence of dependency metadata as a liveness root. Base liveness on ready/current artifacts, active jobs, manifest roots, and explicit tombstones; allow dirty/failed/tombstoned artifacts with dependency rows to become candidates once they are not otherwise live. Add a test where a dirty generated artifact still has dependency rows and is selected by `plan_garbage_collection`.

### WR-02: WARNING - JSON schema command/payload pairing omits Phase 14 artifact commands

**File:** `crates/draft_model/tests/schema_exports.rs:2979`
**Issue:** The generated TypeScript contract includes Phase 14 artifact commands, and runtime deserialization rejects mismatches, but the JSON schema pairing constraints jump from `invalidatePreviewCache` directly to `startExport` (lines 2979-2999). The generated schema has artifact commands in the enum and payload definitions, yet the root `oneOf` also jumps from `invalidatePreviewCache` to `startExport` (schemas/command.schema.json:26033-26058). External clients validating against `schemas/command.schema.json` can therefore miss command/payload mismatches for `getArtifactStatus`, `refreshArtifactStatus`, `retryArtifactGeneration`, `resumeArtifactGeneration`, `cancelArtifactGeneration`, `getArtifactQuotaStatus`, and `runArtifactGarbageCollection`.
**Fix:** Add command/payload pairing entries for every Phase 14 artifact command in `command_payload_pairing_constraints()`, regenerate `schemas/command.schema.json`, and add a schema-export test that every `CommandName` variant appears in the root pairing constraints exactly once.

---

_Reviewed: 2026-06-19T06:47:49Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_

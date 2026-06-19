---
phase: 14-asset-resource-manager-and-derived-artifact-store
reviewed: 2026-06-19T07:01:06Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - crates/artifact_store/src/gc.rs
  - crates/artifact_store/src/generation.rs
  - crates/artifact_store/src/jobs.rs
  - crates/artifact_store/tests/artifact_generation.rs
  - crates/artifact_store/tests/artifact_jobs.rs
  - crates/artifact_store/tests/gc_quota_manifest.rs
  - crates/draft_model/tests/schema_exports.rs
  - schemas/command.schema.json
findings:
  critical: 1
  warning: 0
  info: 0
  total: 1
status: issues_found
---

# Phase 14: Code Review Report

**Reviewed:** 2026-06-19T07:01:06Z
**Depth:** standard
**Files Reviewed:** 8
**Status:** issues_found

## Summary

Re-reviewed the scoped Phase 14 fixes after the review-fix commits. The previous findings for generation failure/cancel terminal persistence, generated artifact dependency rows, GC liveness with dependency rows, and Phase 14 command/payload schema pairing are addressed in the current code. One blocking lifecycle regression remains: failed and cancelled jobs are advertised as retryable/resumable, but the chunk transition guard prevents those jobs from actually being restarted.

Verification run:

```bash
cargo test -p artifact_store --test artifact_generation --test artifact_jobs --test gc_quota_manifest
cargo test -p draft_model --test schema_exports
```

Both commands passed.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: BLOCKER - Retry/resume cannot restart terminal failed or cancelled jobs

**File:** `crates/artifact_store/src/jobs.rs:388`
**Issue:** `next_pending_chunk` deliberately returns `failed` and `cancelled` chunks as pending work, and `resume_generation_job` / `job_status_summary` advertise failed or cancelled jobs as resumable/retryable. But the first real restart path calls `start_generation_chunk`, which flows into `transition_chunk`; `transition_chunk` unconditionally calls `ensure_job_not_terminal`, and `ensure_job_not_terminal` rejects `failed` and `cancelled` job statuses. The net behavior is internally contradictory: a failed generation is now correctly persisted as terminal, but any later retry/resume attempt will fail with "terminal job state cannot be overwritten" before the chunk can move back to `running`.

**Fix:** Add an explicit restart path for resumable terminal jobs instead of reusing the normal active-job transition guard. For example, make retry/resume first move a failed/cancelled job back to `resumable` or `waiting`, clear `cancel_requested`, then allow only failed/cancelled/waiting chunks selected by the resume plan to transition to `running`.

```rust
pub fn restart_generation_job(
    store: &mut ArtifactStore,
    job_id: &str,
) -> Result<ArtifactGenerationJob, ArtifactStoreError> {
    // Only failed/cancelled jobs with incomplete chunks should be restartable.
    let Some(plan) = resume_generation_job(store, job_id)? else {
        return invalid_job(job_id, "job is not resumable");
    };
    if plan.pending_chunks.is_empty() {
        return invalid_job(job_id, "job has no pending chunks");
    }

    store.connection().execute(
        "UPDATE generation_job
         SET status = 'waiting', cancel_requested = 0, updated_at_unix_ms = ?2
         WHERE job_id = ?1 AND status IN ('failed', 'cancelled', 'resumable')",
        params![job_id, now_unix_ms()],
    ).map_err(|source| sqlite_error(store, source))?;

    get_generation_job(store, job_id)?
        .ok_or_else(|| invalid_job_err(job_id, "job disappeared after restart"))
}
```

Add regression coverage that creates a failed job and an acknowledged cancelled job, asserts `resume_generation_job` returns pending chunks, then verifies the actual restart path can call `start_generation_chunk` and complete the pending chunk successfully. Also assert that completed jobs still reject late mutation.

---

_Reviewed: 2026-06-19T07:01:06Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_

---
phase: 14-asset-resource-manager-and-derived-artifact-store
reviewed: 2026-06-19T07:12:09Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - crates/artifact_store/src/gc.rs
  - crates/artifact_store/src/generation.rs
  - crates/artifact_store/src/jobs.rs
  - crates/artifact_store/tests/artifact_generation.rs
  - crates/artifact_store/tests/artifact_jobs.rs
  - crates/artifact_store/tests/gc_quota_manifest.rs
  - crates/draft_model/tests/schema_exports.rs
  - schemas/command.schema.json
  - crates/bindings_node/src/artifact_store_service.rs
findings:
  critical: 1
  warning: 0
  info: 0
  total: 1
status: issues_found
---

# Phase 14: Code Review Report

**Reviewed:** 2026-06-19T07:12:09Z
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

Re-reviewed the current Phase 14 fixes with emphasis on the prior findings and the `restart_generation_job` lifecycle. The scoped artifact store fixes now address the previous direct failure: `restart_generation_job` reopens failed/cancelled jobs to `resumable`, and `artifact_jobs_restart_terminal_failed_and_cancelled_jobs_before_resume_execution` proves the reopened chunks can start and complete.

One blocker remains at the command boundary. The fixed restart primitive is not invoked by the user-facing retry/resume command handlers, so retry/resume still return summaries without changing job state.

Verification run: not run during this final review.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: BLOCKER - Retry/resume commands never invoke the fixed restart lifecycle

**File:** `crates/bindings_node/src/artifact_store_service.rs:90`

**Issue:** `resume_generation` only checks that `resume_generation_job` returns a plan and then returns `job_status_summary`; `retry_generation` only checks `summary.can_retry` and returns the same summary. Neither method calls `restart_generation_job`, and both take `&self`, so they cannot mutate the store even though the actual restart primitive requires `&mut ArtifactStore`. The command handler routes `retryArtifactGeneration` and `resumeArtifactGeneration` into these no-op methods, which means the UI can receive a successful action response while the failed/cancelled job remains terminal and no chunk is reopened for execution.

**Fix:**

```rust
use artifact_store::jobs::{
    cancel_generation_job, job_status_summary, list_active_generation_jobs, restart_generation_job,
    ArtifactGenerationJob, GenerationJobStatus, GenerationStatusSummary,
};

pub fn resume_generation(
    &mut self,
    job_id: &str,
) -> Result<ArtifactGenerationTaskSummary, ArtifactBindingError> {
    let job = restart_generation_job(&mut self.store, job_id)
        .map_err(ArtifactBindingError::Store)?;
    Ok(task_summary_from_job(&job))
}

pub fn retry_generation(
    &mut self,
    job_id: &str,
) -> Result<ArtifactGenerationTaskSummary, ArtifactBindingError> {
    let summary = job_status_summary(&self.store, job_id)
        .map_err(ArtifactBindingError::Store)?
        .ok_or(ArtifactBindingError::UnknownJob)?;
    if !summary.can_retry {
        return Err(ArtifactBindingError::ActionUnavailable);
    }
    let job = restart_generation_job(&mut self.store, job_id)
        .map_err(ArtifactBindingError::Store)?;
    Ok(task_summary_from_job(&job))
}
```

Also make the command handler bind these services as mutable before calling retry/resume, and add a binding-level regression test that creates a failed job and an acknowledged cancelled job, invokes `retryArtifactGeneration` / `resumeArtifactGeneration` through `execute_command`, and asserts the persisted job status becomes `resumable` with `cancel_requested = 0`.

---

_Reviewed: 2026-06-19T07:12:09Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_

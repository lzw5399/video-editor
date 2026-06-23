---
phase: "16-task-scheduler-job-isolation-and-performance-telemetry"
plan: "01B"
type: execute
wave: 2
depends_on:
  - "16-01"
files_modified:
  - "crates/task_runtime/src/lib.rs"
  - "crates/task_runtime/src/job.rs"
  - "crates/task_runtime/src/cancellation.rs"
  - "crates/task_runtime/src/config.rs"
  - "crates/task_runtime/src/scheduler.rs"
  - "crates/task_runtime/src/telemetry.rs"
  - "crates/task_runtime/src/testing.rs"
  - "crates/task_runtime/tests/scheduler_contracts.rs"
  - "crates/task_runtime/tests/scheduler_telemetry.rs"
autonomous: true
requirements:
  - SCHED-01
  - SCHED-03
  - SCHED-04
user_setup: []
must_haves:
  truths:
    - "SCHED-01/D-05/D-07/D-08/D-09: Jobs are typed by domain, priority, resource class, freshness, cancellation, and bounded queue policy."
    - "SCHED-03/D-10/D-18: Resource budgets serialize through portable typed Rust config with no Electron paths or desktop-only assumptions."
    - "SCHED-04/D-13: Telemetry snapshots include queue latency, wait/run time, job duration, cancellation, stale rejection, fallback/unavailable classification, cache, first-frame, dropped/repeated frames, queue depth, and saturation."
  artifacts:
    - path: "crates/task_runtime/src/job.rs"
      provides: "Job identity, domain, priority, resource, freshness, and result contracts"
      contains: ["JobDomain", "JobPriority", "ResourceClass", "JobEnvelope", "JobResult"]
    - path: "crates/task_runtime/src/scheduler.rs"
      provides: "Scheduler admission, bounded queue, coalescing/rejection, cancellation, and completion gates"
      contains: ["JobScheduler", "SchedulerRejected", "submit", "cancel"]
    - path: "crates/task_runtime/src/telemetry.rs"
      provides: "Scheduler-wide telemetry aggregation"
      contains: ["SchedulerTelemetrySnapshot", "SchedulerTelemetrySummary"]
  key_links:
    - from: "crates/task_runtime/src/scheduler.rs"
      to: "crates/task_runtime/src/config.rs"
      via: "scheduler admission enforces typed resource budgets and queue policies"
      pattern: "TaskRuntimeConfig|ResourceBudget|QueuePolicy"
    - from: "crates/task_runtime/src/scheduler.rs"
      to: "crates/task_runtime/src/telemetry.rs"
      via: "admission, cancellation, stale rejection, and completion update scheduler telemetry"
      pattern: "SchedulerTelemetrySnapshot"
---

<objective>
Implement the `task_runtime` scheduler core after the workspace/freshness
migration. This implements D-05, D-07, D-08, D-09, D-10, D-13, D-15, and D-18
for the Rust-owned scheduler contracts, resource budgets, cancellation,
backpressure, telemetry, and deterministic tests.

Purpose: give downstream preview, audio, export, artifact, probe, IO, and
analysis integrations a concrete scheduler policy to submit work into.
Output: task job/config/cancellation/scheduler/telemetry modules plus contract
and telemetry tests.
</objective>

<execution_context>
@/Users/zhiwen/.codex/gsd-core/workflows/execute-plan.md
@/Users/zhiwen/.codex/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md
@.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-RESEARCH.md
@.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-PATTERNS.md
@.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-VALIDATION.md
@.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-01-SUMMARY.md
@crates/realtime_preview_runtime/src/request.rs
@crates/realtime_preview_runtime/src/scheduler.rs
@crates/realtime_preview_runtime/src/telemetry.rs
@crates/audio_engine/src/session.rs
@crates/audio_engine/src/telemetry.rs
@crates/media_runtime/src/job.rs
</context>

<must_haves>
  <truths>
    - D-05: Required domains exist: interactive preview/scrub/seek, decode, audio, artifact generation, export, media probe, filesystem IO, and analysis.
    - D-07: stale-sensitive job envelopes carry target timeline microseconds and `PlaybackGeneration`.
    - D-08/D-09: cancellation and bounded admission release accounting, reject/coalesce work, and emit telemetry.
    - D-10/D-18: config is portable and ready for Phase 17 mapping without desktop-only path fields.
    - D-13: telemetry records the full scheduler budget surface required by SCHED-04.
  </truths>
  <prohibitions>
    - Do not add a new external scheduler, async, or telemetry package.
    - Do not encode Electron, FFmpeg binary paths, renderer frame tokens, retry/fallback policy, or desktop worker names into `task_runtime` config.
    - Do not depend on wall-clock sleeps for deterministic scheduler correctness tests.
  </prohibitions>
</must_haves>

<tasks>

<task type="auto" tdd="true">
  <name>Task 16-01B-01: Implement typed job, cancellation, and config contracts</name>
  <files>crates/task_runtime/src/lib.rs, crates/task_runtime/src/job.rs, crates/task_runtime/src/cancellation.rs, crates/task_runtime/src/config.rs</files>
  <read_first>
    - `crates/task_runtime/src/lib.rs`
    - `crates/task_runtime/src/freshness.rs`
    - `crates/realtime_preview_runtime/src/request.rs`
    - `crates/realtime_preview_runtime/src/scheduler.rs`
    - `crates/audio_engine/src/session.rs`
    - `crates/media_runtime/src/job.rs`
  </read_first>
  <action>Create the scheduler contract types per D-05, D-07, D-08, D-09, D-10, and D-18: `JobId`, `JobDomain`, `JobPriority`, `ResourceClass`, `JobFreshness`, `JobEnvelope`, `JobResult`, `SchedulerRejected`, `TaskCancellationToken`, `TaskRuntimeConfig`, `ResourceBudget`, and `QueuePolicy`. Use serde camelCase and deny unknown fields on serialized contracts. Config must define portable domain/resource capacities and queue-depth policy without Electron paths, FFmpeg paths, worker labels, renderer-controlled priority, freshness, retry, or fallback policy fields.</action>
  <verify>
    <automated>cargo test -p task_runtime scheduler_contracts -- --nocapture</automated>
    <automated>cargo test -p task_runtime config -- --nocapture</automated>
  </verify>
  <acceptance_criteria>
    - `JobDomain` includes interactive preview/scrub/seek, decode, audio, artifact generation, export, media probe, filesystem IO, and analysis.
    - Portable config serializes without Electron paths, FFmpeg paths, retry/fallback policy, or platform-specific worker names.
    - Cancellation tokens are cloneable, observable, and ready to gate queued, running, and completion paths.
    - Job envelopes carry target timeline microseconds and `PlaybackGeneration` for stale-sensitive work.
  </acceptance_criteria>
  <done>The scheduler has typed job, cancellation, freshness, and portable config contracts before queue execution is added.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 16-01B-02: Implement bounded scheduler admission and telemetry</name>
  <files>crates/task_runtime/src/lib.rs, crates/task_runtime/src/scheduler.rs, crates/task_runtime/src/telemetry.rs, crates/task_runtime/src/testing.rs</files>
  <read_first>
    - `crates/task_runtime/src/job.rs`
    - `crates/task_runtime/src/cancellation.rs`
    - `crates/task_runtime/src/config.rs`
    - `crates/realtime_preview_runtime/src/scheduler.rs`
    - `crates/realtime_preview_runtime/src/telemetry.rs`
    - `crates/audio_engine/src/session.rs`
    - `crates/audio_engine/src/telemetry.rs`
  </read_first>
  <action>Implement `JobScheduler`, scheduler admission state, deterministic testing hooks, and `SchedulerTelemetrySnapshot`. Enforce bounded admission, priority ordering, obsolete preview coalescing, cancellation of queued/running jobs, completion freshness rejection, resource accounting, queue depth, resource saturation, and telemetry summaries with deterministic fake executor hooks. This implements D-08, D-09, D-13, and D-15 for the scheduler core.</action>
  <verify>
    <automated>cargo test -p task_runtime scheduler_contracts -- --nocapture</automated>
    <automated>cargo test -p task_runtime scheduler_telemetry -- --nocapture</automated>
  </verify>
  <acceptance_criteria>
    - Interactive and realtime priorities cannot be admitted behind saturated background/export capacity when their own budgets have capacity.
    - Full queues coalesce obsolete preview work or return classified rejection instead of growing unbounded.
    - Cancellation decrements queued/in-flight accounting once and increments cancellation telemetry.
    - Telemetry records queue latency, wait/run duration, job duration, cancellation, stale rejection, fallback/unavailable classification, cache, first-frame, dropped/repeated frames, queue depth, and saturation.
  </acceptance_criteria>
  <done>The scheduler core can admit, reject, cancel, complete, and report jobs deterministically under explicit budgets.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 16-01B-03: Add deterministic contract and telemetry tests</name>
  <files>crates/task_runtime/tests/scheduler_contracts.rs, crates/task_runtime/tests/scheduler_telemetry.rs, crates/task_runtime/src/testing.rs</files>
  <read_first>
    - `crates/task_runtime/src/scheduler.rs`
    - `crates/task_runtime/src/telemetry.rs`
    - `crates/task_runtime/src/testing.rs`
    - `crates/realtime_preview_runtime/src/scheduler.rs`
    - `crates/realtime_preview_runtime/src/telemetry.rs`
    - `crates/audio_engine/src/session.rs`
    - `.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-VALIDATION.md`
  </read_first>
  <action>Add tests that fail the pre-Phase-16 shape: priority lanes under saturation, bounded queue coalescing, stale generation rejection before completion commit, cancellation release accounting, resource config serialization, queue latency percentiles, job duration/wait/run time summaries, fallback/unavailable classification, cache hit counters, first-frame time, dropped/repeated frame counters, queue depth, and resource saturation. Use fake clocks/executors from `task_runtime::testing`; assertions must inspect scheduler state and telemetry rather than only successful job completion. This implements D-13 and D-15 at the foundation layer.</action>
  <verify>
    <automated>cargo test -p task_runtime -- --nocapture</automated>
    <automated>cargo check --workspace --locked</automated>
  </verify>
  <acceptance_criteria>
    - `scheduler_contracts.rs` asserts priority, resource, freshness, cancellation, backpressure, and portable config behavior.
    - `scheduler_telemetry.rs` asserts p50/p95/max queue and duration summaries plus cancel/stale/reject/cache/fallback/depth/saturation counters.
    - Tests do not pass by only advancing a playhead or returning an artifact result.
    - Tests cover stale and cancelled completions before any visible-state mutation callback is invoked.
  </acceptance_criteria>
  <done>Foundation tests make the Rust scheduler contract observable before integration work begins.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Domain crate -> `task_runtime` | Preview/audio/export/artifact/probe adapters submit typed work into shared scheduler policy. |
| Completion handler -> visible state | Scheduler completion may mutate preview/audio/artifact-visible status only after freshness and cancellation checks. |
| Config source -> Rust scheduler | Config reaches scheduler through typed Rust config, not raw renderer policy. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-16-01B-A | Denial of Service | `JobScheduler` admission | mitigate | Enforce bounded queues, resource budgets, rejection/coalescing, and deterministic saturation tests. |
| T-16-01B-B | Tampering | completion freshness | mitigate | Require `JobFreshness` with target timeline microseconds, `PlaybackGeneration`, expected revision where applicable, and stale rejection telemetry before commit. |
| T-16-01B-C | Spoofing | fallback/unavailable telemetry | mitigate | Record fallback/unavailable as diagnostic classification only; product success gates remain in later plans and no scheduler test treats fallback as success. |
| T-16-01B-D | Information Disclosure | telemetry/config serialization | mitigate | Expose typed budget and aggregate telemetry fields only; exclude filesystem paths, raw command args, and renderer-controlled policy. |
| T-16-01B-SC | Tampering | npm/pip/cargo installs | accept | No new external package installs are planned; use existing Rust/std dependencies and existing workspace crates. |
</threat_model>

<verification>
<automated>cargo test -p task_runtime -- --nocapture</automated>
<automated>cargo check --workspace --locked</automated>
<automated>git diff --check -- . ':!reference'</automated>
</verification>

<success_criteria>
`task_runtime` owns typed scheduler contracts and proves bounded
freshness/cancellation/config/telemetry behavior with deterministic Rust tests.
</success_criteria>

## Artifacts this phase produces

- `task_runtime::JobScheduler`
- `task_runtime::JobDomain`, `JobPriority`, `ResourceClass`, `JobFreshness`, `JobEnvelope`, `JobResult`, `SchedulerRejected`
- `task_runtime::TaskCancellationToken`
- `task_runtime::TaskRuntimeConfig`, `ResourceBudget`, `QueuePolicy`
- `task_runtime::SchedulerTelemetrySnapshot`, `SchedulerTelemetrySummary`
- `task_runtime::testing` fake clock/executor utilities
- `crates/task_runtime/tests/scheduler_contracts.rs`
- `crates/task_runtime/tests/scheduler_telemetry.rs`

<output>
Create `.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-01B-SUMMARY.md` when done.
</output>

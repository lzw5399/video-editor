# Phase 16: Task Scheduler, Job Isolation, And Performance Telemetry - Context

**Gathered:** 2026-06-23
**Status:** Ready for planning
**Source:** $gsd-autonomous --from 16 --to 18, smart discuss equivalent

<domain>
## Phase Boundary

Phase 16 adds a production Rust-owned task runtime and job scheduler for preview,
decode, artifact generation, export, media probing, filesystem IO, and analysis
work. The phase must isolate time-sensitive preview/audio work from heavy export
and derived artifact work, align stale-sensitive jobs to `TimelineClock` and
`PlaybackGeneration`, expose bounded cancellation/backpressure/resource policies,
and publish performance telemetry strong enough to prove SCHED-01 through
SCHED-04 under real desktop workflows.

</domain>

<decisions>
## Implementation Decisions

### Scheduler Ownership
- **D-01:** The scheduler is a Rust-owned production runtime boundary, not an Electron main-process queue, renderer debounce layer, or binding-owned compatibility shim.
- **D-02:** UI code may request work and display productized status only; it must not decide queue priority, retry/fallback behavior, timeline freshness, resource budgets, or FFmpeg/export execution policy.
- **D-03:** A new `task_runtime` or equivalently named crate should own scheduler contracts, queue policy, job identity, cancellation, backpressure, telemetry aggregation, and test fixtures. Existing preview/export/artifact/audio crates should integrate through typed interfaces rather than each owning separate ad hoc scheduling policy.
- **D-04:** Destructive replacement is preferred over compatibility layering. Any legacy synchronous frame pump, poll loop, generic command queue, fallback-success path, or unbounded spawn behavior that conflicts with the scheduler boundary must be removed or gated from product paths.

### Job Model And Isolation
- **D-05:** Jobs must be typed by domain, priority, freshness, and resource class. Required domains are interactive preview/scrub/seek, decode, audio, artifact generation, export, media probe, filesystem IO, and analysis.
- **D-06:** Interactive preview, playhead scrubbing, inspector recompute, realtime audio, and first-frame requests are latency-sensitive and must not share an unconstrained worker pool with export, proxy, waveform, thumbnail, cache rebuild, or bulk probe work.
- **D-07:** Every stale-sensitive job carries target timeline microseconds and `PlaybackGeneration`; stale completion must be rejected before mutating preview/audio/artifact-visible state.
- **D-08:** Cancellation is first-class. Cancelled jobs must release queued work, decrement in-flight accounting, emit telemetry, and avoid presenting or committing obsolete results.

### Backpressure And Resource Limits
- **D-09:** Queues are bounded by explicit policy. When full, the scheduler may coalesce/drop obsolete preview work or reject low-priority work with a classified error; it must not silently stretch playback cadence or accumulate unbounded memory.
- **D-10:** Resource budgets are explicit and configurable for desktop development through typed Rust config surfaced by a narrow native binding. Budgets must be shaped so Phase 17 can map the same contracts onto mobile/server runtimes.
- **D-11:** Heavy export and artifact jobs may reserve CPU/IO resources, but they cannot starve supported preview frame delivery, playhead scrubbing, inspector edits, or audio output.
- **D-12:** Native/GPU/resource lifetimes remain explicit: bounded in-flight queues, completion-driven release where applicable, deterministic cancellation, and observable backpressure.

### Telemetry And Product Evidence
- **D-13:** Scheduler telemetry must include queue latency, job duration, wait time, run time, cancellation count, stale rejection count, fallback/unavailable classification, cache hit rate, first-frame time, dropped/repeated frame budgets, queue depth, and resource saturation.
- **D-14:** Product UI should not expose raw scheduler internals by default. Runtime/backend/cache/graph diagnostics remain developer-diagnostics-only unless surfaced as concise product exception copy.
- **D-15:** Tests must fail the known bad state: export/artifact load must not be able to block real preview cadence, and green tests must prove visible preview motion plus scheduler telemetry, not merely playhead advancement or artifact generation.
- **D-16:** Fallback is diagnostic evidence only. A fallback, CPU probe, artifact, mock, or DOM token may explain unavailability but may not satisfy product scheduler/preview success.

### Phase Execution Scope
- **D-17:** Phase 16 should deliver the scheduler foundation and integrate at least the preview, artifact generation, export, media probe, and audio-preview boundaries far enough that cross-domain starvation and cancellation can be tested.
- **D-18:** Full mobile/server binding implementation is deferred to Phase 17, but Phase 16 scheduler APIs must avoid desktop-only assumptions that would block Phase 17.
- **D-19:** Production effects, retiming, transitions, filters, and masks remain Phase 18 work. Phase 16 must expose scheduling hooks that those capabilities can use later without adding a second scheduler.

### the agent's Discretion
- The exact crate/module names may vary if the existing workspace strongly favors another name, but the ownership boundary and typed scheduler contracts are locked.
- The initial queue algorithms, worker counts, and telemetry histogram implementations are at the agent's discretion as long as they are deterministic, bounded, testable, and configurable.

</decisions>

<specifics>
## Specific Ideas

- Continue the current production architecture direction: Rust owns scheduler,
  sessions, preview/audio/export semantics, and telemetry; Electron remains a
  shell for controls and productized display.
- The user explicitly allows destructive refactors and does not want compatibility
  preservation, fallback ladders, or temporary containment when the architecture is
  wrong.
- Verification should behave like a real tester: stress concurrent export,
  artifact generation, preview playback, scrubbing, cancellation, save/reopen, and
  product UI evidence.

</specifics>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase Requirements
- `.planning/ROADMAP.md` Phase 16 section - Phase goal, dependencies, SCHED requirements, and success criteria.
- `.planning/REQUIREMENTS.md` `SCHED-01` through `SCHED-04` - Scheduler and performance requirements.

### Architecture Boundaries
- `.planning/PROJECT.md` Constraints - Rust-owned project/timeline/rendering semantics, no product fallback, no legacy compatibility by default, product E2E acceptance.
- `.planning/notes/production-editor-architecture-decisions.md` section 7 - Unified task runtime and scheduler direction.
- `docs/runtime-boundaries.md` Phase 11/16 ownership notes - Realtime preview boundary and explicit Phase 16 ownership of scheduling/fairness/background jobs/cancellation.
- `docs/no-product-fallback-policy.md` - Product success must not be satisfied by fallback, mock, artifact, CPU, or DOM evidence.
- `docs/refactor-and-legacy-cleanup-policy.md` - Replace obsolete product paths rather than preserving partial compatibility.
- `docs/product-e2e-acceptance-policy.md` - Visible editor features require normal Playwright/Electron product workflow proof.

### Existing Scheduler-Adjacent Code
- `crates/realtime_preview_runtime/src/scheduler.rs` - Existing realtime playback scheduler and backpressure policy.
- `crates/realtime_preview_runtime/src/telemetry.rs` - Existing preview telemetry shape.
- `crates/audio_engine/src/session.rs` and `crates/audio_engine/src/telemetry.rs` - Audio session/generation telemetry patterns.
- `crates/artifact_store/src/jobs.rs` - Current artifact generation job model to migrate behind scheduler policy.
- `crates/media_runtime/src/job.rs` - Existing export/runtime job contracts.
- `crates/bindings_node/src/realtime_preview_service.rs` - Desktop binding integration points that must stay thin.
- `apps/desktop-electron/src/main/realtimePreviewHost.ts` - Main-process preview host boundary that must remain control/telemetry oriented.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `TimelineClock` and `PlaybackGeneration` already exist in `realtime_preview_runtime` and should be reused rather than duplicated.
- Realtime preview telemetry already reports first-frame, seek, queue/render latency, frame pacing, stale rejection, cancellation, fallback, cache hit, target time, and generation fields.
- Audio sessions already reject stale/cancelled buffers with generation-aware telemetry.
- Artifact store already has generation job concepts, invalidation, quotas, and tests that can be moved behind a shared scheduler.

### Established Patterns
- Runtime crates expose typed Rust contracts and bindings_node maps them through narrow JSON/Node-API responses.
- Product Electron APIs are explicit, not generic command envelopes; renderer state consumes Rust-owned view models and telemetry.
- Tests use Rust crate tests for domain contracts plus Playwright/Electron product E2E for user-visible evidence.

### Integration Points
- `crates/realtime_preview_runtime` for interactive preview scheduler integration and frame pacing telemetry.
- `crates/audio_engine` for audio buffer job freshness and output starvation tests.
- `crates/artifact_store` for proxy/thumbnail/waveform/cache generation job migration.
- `crates/media_runtime` and `crates/media_runtime_desktop` for export/probe job integration.
- `crates/bindings_node` and `apps/desktop-electron/src/main` for narrow scheduler capability/telemetry/status APIs.
- `apps/desktop-electron/tests` for product workflow starvation, cancellation, and product UI diagnostic-boundary evidence.

</code_context>

<deferred>
## Deferred Ideas

- Full C ABI/JNI/Swift/server runtime ports are Phase 17.
- Retiming, effects, filters, masks, and transitions are Phase 18.
- Advanced cluster/distributed scheduler execution is out of scope for Phase 16 unless it is needed to keep the local API portable.

</deferred>

---

*Phase: 16-task-scheduler-job-isolation-and-performance-telemetry*
*Context gathered: 2026-06-23*

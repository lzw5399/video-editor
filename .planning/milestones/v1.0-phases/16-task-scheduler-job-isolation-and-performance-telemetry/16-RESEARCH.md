# Phase 16: Task Scheduler, Job Isolation, And Performance Telemetry - Research

**Researched:** 2026-06-23  
**Domain:** Rust-owned editor task runtime, job isolation, cancellation, and performance telemetry  
**Confidence:** HIGH for codebase findings and locked decisions; MEDIUM for initial telemetry budgets where Phase 16 thresholds are new.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Scheduler Ownership
- **D-01:** The scheduler is a Rust-owned production runtime boundary, not an Electron main-process queue, renderer debounce layer, or binding-owned compatibility shim.
- **D-02:** UI code may request work and display productized status only; it must not decide queue priority, retry/fallback behavior, timeline freshness, resource budgets, or FFmpeg/export execution policy.
- **D-03:** A new `task_runtime` or equivalently named crate should own scheduler contracts, queue policy, job identity, cancellation, backpressure, telemetry aggregation, and test fixtures. Existing preview/export/artifact/audio crates should integrate through typed interfaces rather than each owning separate ad hoc scheduling policy.
- **D-04:** Destructive replacement is preferred over compatibility layering. Any legacy synchronous frame pump, poll loop, generic command queue, fallback-success path, or unbounded spawn behavior that conflicts with the scheduler boundary must be removed or gated from product paths.

#### Job Model And Isolation
- **D-05:** Jobs must be typed by domain, priority, freshness, and resource class. Required domains are interactive preview/scrub/seek, decode, audio, artifact generation, export, media probe, filesystem IO, and analysis.
- **D-06:** Interactive preview, playhead scrubbing, inspector recompute, realtime audio, and first-frame requests are latency-sensitive and must not share an unconstrained worker pool with export, proxy, waveform, thumbnail, cache rebuild, or bulk probe work.
- **D-07:** Every stale-sensitive job carries target timeline microseconds and `PlaybackGeneration`; stale completion must be rejected before mutating preview/audio/artifact-visible state.
- **D-08:** Cancellation is first-class. Cancelled jobs must release queued work, decrement in-flight accounting, emit telemetry, and avoid presenting or committing obsolete results.

#### Backpressure And Resource Limits
- **D-09:** Queues are bounded by explicit policy. When full, the scheduler may coalesce/drop obsolete preview work or reject low-priority work with a classified error; it must not silently stretch playback cadence or accumulate unbounded memory.
- **D-10:** Resource budgets are explicit and configurable for desktop development through typed Rust config surfaced by a narrow native binding. Budgets must be shaped so Phase 17 can map the same contracts onto mobile/server runtimes.
- **D-11:** Heavy export and artifact jobs may reserve CPU/IO resources, but they cannot starve supported preview frame delivery, playhead scrubbing, inspector edits, or audio output.
- **D-12:** Native/GPU/resource lifetimes remain explicit: bounded in-flight queues, completion-driven release where applicable, deterministic cancellation, and observable backpressure.

#### Telemetry And Product Evidence
- **D-13:** Scheduler telemetry must include queue latency, job duration, wait time, run time, cancellation count, stale rejection count, fallback/unavailable classification, cache hit rate, first-frame time, dropped/repeated frame budgets, queue depth, and resource saturation.
- **D-14:** Product UI should not expose raw scheduler internals by default. Runtime/backend/cache/graph diagnostics remain developer-diagnostics-only unless surfaced as concise product exception copy.
- **D-15:** Tests must fail the known bad state: export/artifact load must not be able to block real preview cadence, and green tests must prove visible preview motion plus scheduler telemetry, not merely playhead advancement or artifact generation.
- **D-16:** Fallback is diagnostic evidence only. A fallback, CPU probe, artifact, mock, or DOM token may explain unavailability but may not satisfy product scheduler/preview success.

#### Phase Execution Scope
- **D-17:** Phase 16 should deliver the scheduler foundation and integrate at least the preview, artifact generation, export, media probe, and audio-preview boundaries far enough that cross-domain starvation and cancellation can be tested.
- **D-18:** Full mobile/server binding implementation is deferred to Phase 17, but Phase 16 scheduler APIs must avoid desktop-only assumptions that would block Phase 17.
- **D-19:** Production effects, retiming, transitions, filters, and masks remain Phase 18 work. Phase 16 must expose scheduling hooks that those capabilities can use later without adding a second scheduler.

### the agent's Discretion
- The exact crate/module names may vary if the existing workspace strongly favors another name, but the ownership boundary and typed scheduler contracts are locked.
- The initial queue algorithms, worker counts, and telemetry histogram implementations are at the agent's discretion as long as they are deterministic, bounded, testable, and configurable.

### Deferred Ideas (OUT OF SCOPE)
- Full C ABI/JNI/Swift/server runtime ports are Phase 17.
- Retiming, effects, filters, masks, and transitions are Phase 18.
- Advanced cluster/distributed scheduler execution is out of scope for Phase 16 unless it is needed to keep the local API portable.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SCHED-01 | Preview, decode, artifact generation, export, media probing, and filesystem IO run through priority-aware queues with cancellation, backpressure, target timeline microseconds, and `PlaybackGeneration`. | Build a Rust `task_runtime` crate that owns typed job metadata, priority lanes, resource classes, generation freshness, cancellation, and bounded queue policy. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md] |
| SCHED-02 | Export and heavy artifact jobs cannot block playhead scrubbing, inspector edits, or preview frame delivery on supported hardware. | Replace binding-local export/artifact/audio/preview worker paths with shared scheduler lanes and add starvation tests that run export/artifact/probe load while preview/audio continue meeting cadence gates. [VERIFIED: codebase grep] |
| SCHED-03 | Thread-pool and resource limits are explicit, configurable for desktop development, and ready to map onto mobile/server runtimes. | Use typed scheduler config in Rust and expose only narrow binding APIs for capability/status/telemetry; do not encode Electron, FFmpeg paths, or desktop thread assumptions into core scheduler types. [CITED: docs/runtime-boundaries.md] |
| SCHED-04 | Performance telemetry records queue latency, job duration, cancellation, fallback, cache hit rate, first-frame time, and dropped-frame budgets. | Extend existing preview/audio telemetry into scheduler-wide snapshots and add per-domain queue/run/cancel/stale/cache/resource saturation counters. [VERIFIED: codebase grep] |
</phase_requirements>

## Project Constraints (from AGENTS.md)

- UI emits commands and Rust owns project/timeline semantics; UI code must not construct FFmpeg commands. [CITED: AGENTS.md]
- Structurally wrong preview, edit, render, session, media, or native-surface boundaries must be replaced with the production architecture, not patched with offsets, resync tricks, fallback ladders, or temporary compatibility layers. [CITED: AGENTS.md]
- `.veproj/project.json` is canonical; render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, and preview caches are derived artifacts. [CITED: AGENTS.md]
- Product language, schema, docs, IPC, and tests should use Jianying concepts such as draft, material, track, segment, keyframe, filter, and transition. [CITED: AGENTS.md]
- Core time math must use integer microseconds, frame indices, or rational frame rates; persisted semantics must not use naked floating-point time. [CITED: AGENTS.md]
- Render Graph isolates editing semantics from FFmpeg; FFmpeg Runtime executes jobs and reports progress/errors, but does not decide editing behavior. [CITED: AGENTS.md]
- FFmpeg distribution remains subject to LGPL/GPL/nonfree, notices, and commercial-product legal review before external redistribution. [CITED: AGENTS.md]
- Planning research must respect no-product-fallback, no-legacy-compatibility-by-default, and product E2E acceptance policies. [CITED: docs/no-product-fallback-policy.md] [CITED: docs/refactor-and-legacy-cleanup-policy.md] [CITED: docs/product-e2e-acceptance-policy.md]

## Research Summary

Phase 16 should add a new Rust workspace member named `task_runtime` and make it the production owner of job scheduling, queue policy, cancellation, freshness, resource budgets, and scheduler telemetry. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md] The target is not a generic Electron queue and not another binding-level worker registry; it is a Rust runtime boundary sitting between `bindings_node` command APIs and domain executors for preview, audio, artifact generation, export, media probe, filesystem IO, and later analysis/effects work. [CITED: .planning/notes/production-editor-architecture-decisions.md]

The most important current codebase finding is that heavy and time-sensitive work is already split across separate local schedulers: preview has binding-owned worker maps and thread loops, export has a binding-owned thread registry, audio has a binding-owned refill thread and synchronous FFmpeg decode windows, artifact refresh performs thumbnail generation inline, and material import probes with ffprobe inline. [VERIFIED: codebase grep] Those paths already have useful typed concepts such as `TimelineClock`, `PlaybackGeneration`, cancellation tokens, artifact job status, FFmpeg progress, and preview frame pacing telemetry, but they do not share a queue/fairness/resource policy. [VERIFIED: codebase grep]

**Primary recommendation:** introduce `crates/task_runtime` first, keep it dependency-light, integrate preview/audio/export/artifact/probe through typed adapters, and gate the phase with scheduler stress tests that fail if export/artifact/probe work delays visible preview cadence, audio refill, or stale-generation rejection. [ASSUMED]

## Current Codebase Findings

### Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Scheduler policy, job identity, priorities, cancellation, backpressure, resource budgets | Rust `task_runtime` | Domain crates | Phase 16 locks this as a Rust-owned production runtime boundary, not Electron or binding policy. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md] |
| Timeline freshness and stale-result rejection | Rust runtime/session layer | Preview/audio/artifact adapters | `TimelineClock` and `PlaybackGeneration` already exist and increment on seek, play, pause, stop, accepted edit, draft reload, material relink, surface detach, and runtime reset. [VERIFIED: crates/realtime_preview_runtime/src/clock.rs:96] |
| Preview cadence and GPU presentation | `realtime_preview_runtime` under scheduler control | `bindings_node` thin bridge | The production preview path must remain Rust-owned and prove `renderGraphGpuComposited` visible evidence; Electron is control/telemetry only. [CITED: docs/runtime-boundaries.md] |
| Audio preview refill and sync | `audio_engine` / `audio_output_desktop` under scheduler control | `bindings_node` thin bridge | Audio already shares `TimelineClock`/`PlaybackGeneration`, but current native output/refill is binding-owned. [VERIFIED: crates/audio_engine/src/session.rs:5] [VERIFIED: crates/bindings_node/src/audio_service.rs:58] |
| Export execution and progress | `media_runtime` job executor under scheduler control | `bindings_node` status API | Current export registry spawns a thread from bindings; scheduler must own export admission and resources. [VERIFIED: crates/bindings_node/src/preview_export_service.rs:216] |
| Artifact generation and cache maintenance | `artifact_store` under scheduler control | Project/session adapter | Artifact store has persisted generation jobs, chunks, cancel/resume, quota, and invalidation, but refresh currently runs generation inline. [VERIFIED: crates/artifact_store/src/jobs.rs:167] [VERIFIED: crates/bindings_node/src/artifact_store_service.rs:445] |
| Media probing and filesystem IO | `media_runtime` / `project_store` under scheduler control | Project/session adapter | Material import currently discovers runtime and probes in the project session path; Phase 16 must route probe/IO through bounded queues. [VERIFIED: crates/bindings_node/src/project_session_service.rs:1704] |
| Product status display | Electron renderer/main | Rust telemetry source | UI may display productized status, but raw scheduler internals remain developer diagnostics by locked decision. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md] |

### Reusable Assets

- `TimelineClock` and `PlaybackGeneration` are defined in `realtime_preview_runtime::clock`; generation is a `u64` wrapper and advances on seek, scrub, play, pause, stop, accepted edit, draft reload, material relink, surface detach, and reset. [VERIFIED: crates/realtime_preview_runtime/src/clock.rs:7] [VERIFIED: crates/realtime_preview_runtime/src/clock.rs:141]
- Realtime preview request payloads already carry `target_time`, `playback_generation`, optional audio sync, optional cancellation token, mode, queue latency, render duration, fallback reason, cache-hit, repeated-frame, and dropped-frame fields. [VERIFIED: crates/realtime_preview_runtime/src/request.rs:32]
- Realtime preview telemetry already records first-frame latency, seek latency, queue latency, render duration, presented/dropped/repeated frames, stale rejection, cancellation, fallback, cache hit, target time, generation, and frame pacing. [VERIFIED: crates/realtime_preview_runtime/src/telemetry.rs:10]
- Audio preview runtime already validates stale generation, cancellation, and session bounds before marking an audio buffer presented. [VERIFIED: crates/audio_engine/src/session.rs:254]
- Artifact store already has durable generation job/chunk status, cancel, resume, and active-job queries in SQLite-backed state. [VERIFIED: crates/artifact_store/src/jobs.rs:50] [VERIFIED: crates/artifact_store/src/jobs.rs:460]
- `media_runtime::run_export_job` already reports FFmpeg start/progress/completed events, supports cancel tokens, bounds timeout, and classifies runtime errors. [VERIFIED: crates/media_runtime/src/job.rs:211]

### Legacy Or Ad Hoc Paths To Replace Or Gate

| Current Path | Finding | Phase 16 Action |
|--------------|---------|-----------------|
| `crates/bindings_node/src/preview_export_service.rs` | `ExportJobRegistry` owns an `Arc<Mutex<BTreeMap<...>>>`, creates a `CancelToken`, inserts status, and starts export with `thread::spawn`. [VERIFIED: crates/bindings_node/src/preview_export_service.rs:210] | Move export admission, cancellation, status updates, and validation follow-up behind `task_runtime` export jobs; `bindings_node` should submit/query/cancel through thin APIs. |
| `crates/bindings_node/src/realtime_preview_service.rs` | Binding registry owns `still_frame_workers` and `playback_workers`, starts threads for still-frame and playback loops, and sleeps on an idle poll interval. [VERIFIED: crates/bindings_node/src/realtime_preview_service.rs:202] [VERIFIED: crates/bindings_node/src/realtime_preview_service.rs:769] [VERIFIED: crates/bindings_node/src/realtime_preview_service.rs:885] | Keep preview cadence/compositor contracts in Rust, but move worker lifecycle/admission into `task_runtime` interactive lanes. |
| `crates/bindings_node/src/audio_service.rs` | Native audio output owns an `audio-preview-refill` thread; refill loops sleep/poll and render audio chunks outside shared scheduler policy. [VERIFIED: crates/bindings_node/src/audio_service.rs:58] [VERIFIED: crates/bindings_node/src/audio_service.rs:994] | Scheduler must own realtime audio/refill work as latency-sensitive jobs with underrun and queue-depth telemetry. |
| `crates/bindings_node/src/audio_service.rs` | Audio decode windows call `discover_runtime_config`, `DesktopFfmpegExecutor::with_timeout`, and `executor.run` directly. [VERIFIED: crates/bindings_node/src/audio_service.rs:1154] | Route decode jobs through scheduler resource class `Decode`/`Audio` instead of direct FFmpeg execution from binding service code. |
| `crates/bindings_node/src/artifact_store_service.rs` | `refresh_material_thumbnails` discovers runtime, creates a desktop executor, loops materials, and calls `generate_thumbnail_artifact` inline. [VERIFIED: crates/bindings_node/src/artifact_store_service.rs:445] | Refresh should enqueue artifact jobs and return status; scheduler controls concurrency, stale commit, and cancellation. |
| `crates/bindings_node/src/artifact_store_service.rs` | `DesktopThumbnailGenerator` calls FFmpeg through `executor.run` during artifact generation. [VERIFIED: crates/bindings_node/src/artifact_store_service.rs:610] | Keep FFmpeg command construction in generator/runtime, but execute under scheduler resource budgets and cancellation. |
| `crates/bindings_node/src/project_session_service.rs` and `material_service.rs` | Material import performs runtime discovery and ffprobe metadata probing inline before mutating the draft. [VERIFIED: crates/bindings_node/src/project_session_service.rs:1711] [VERIFIED: crates/bindings_node/src/material_service.rs:177] | Route media probe and project-bundle IO through scheduler; commit material state only if request/session revision is still current. |
| `apps/desktop-electron/src/main/index.ts` | Test-only native command mock helpers can return export/audio/artifact/runtime responses in main process. [VERIFIED: apps/desktop-electron/src/main/index.ts:249] [VERIFIED: apps/desktop-electron/src/main/index.ts:919] | Keep as explicit test harness only if normal product paths and product success tests cannot use them; source guards should block scheduler success from mock responses. |
| `request_preview_frame_with_executor` / preview artifact APIs | Preview artifact generation remains callable through service APIs. [VERIFIED: crates/bindings_node/src/preview_export_service.rs:133] | Preserve only as artifact/diagnostic generation; product playback success must remain `renderGraphGpuComposited`. [CITED: docs/no-product-fallback-policy.md] |

### Existing Validation Surface

- `package.json` has no `test:phase16` script yet. [VERIFIED: package.json:11]
- Existing product preview cadence tests already require renderGraph GPU product path, visible preview pixel motion, 3s target time advance of at least 2,900,000 microseconds, 90 accounted frames for 30fps, no artifact fallback frame loop, frame interval p95 <= 50ms, max <= 75ms, and scheduler lateness p95 <= 12ms. [VERIFIED: apps/desktop-electron/tests/product-preview-cadence.spec.ts:60] [VERIFIED: apps/desktop-electron/tests/product-preview-cadence.spec.ts:147]
- `scripts/no-product-fallback-guards.sh` blocks product playback success through decoded/FFmpeg content evidence, mock/fallback display, product backend values other than `renderGraphGpu`/`none`, and missing `renderGraphGpuComposited` evidence. [VERIFIED: scripts/no-product-fallback-guards.sh:15]
- `scripts/phase15-3-source-guards.sh` already blocks Electron realtime preview host ownership of playback cadence, fake compositor evidence, telemetry interval fanout, and renderer telemetry polling. [VERIFIED: scripts/phase15-3-source-guards.sh:108]
- `scripts/phase14-source-guards.sh` intentionally blocked Phase 16 scheduler policy leakage before this phase, showing the planned boundary now needs a deliberate source-guard update. [VERIFIED: scripts/phase14-source-guards.sh:85]

## Production Target Architecture

### Target Chain

```text
Electron UI controls/status
  -> Electron main/preload IPC routing
  -> bindings_node thin JSON/Node-API methods
  -> task_runtime::JobScheduler
       - typed JobId / JobDomain / JobPriority / ResourceClass
       - target_timeline_time_us + PlaybackGeneration freshness
       - bounded priority queues, coalescing, rejection, cancellation
       - telemetry aggregation and scheduler snapshots
  -> domain adapters
       - preview/audio interactive jobs
       - decode/media probe jobs
       - artifact generation/cache/GC jobs
       - export/validation jobs
       - filesystem IO jobs
  -> existing runtime executors
       - realtime_preview_runtime compositor/presenter
       - audio_engine/audio_output_desktop
       - media_runtime/media_runtime_desktop
       - artifact_store/project_store
  -> completion gate
       - cancel check
       - generation/session/revision freshness check
       - derived-artifact state commit or explicit stale rejection
  -> Rust telemetry snapshot
  -> productized UI status or developer diagnostics
```

This chain preserves the documented boundary where Electron controls UI and safe IPC routing, `bindings_node` maps route/types, Rust runtime crates own sessions and telemetry, and semantic crates stay pure. [CITED: docs/runtime-boundaries.md]

### Standard Stack

| Area | Recommendation | Source |
|------|----------------|--------|
| Scheduler crate | Add `crates/task_runtime` to the Rust workspace; no new external package is required for the first implementation. [ASSUMED] | Existing workspace is Rust 2024 and has explicit members in `Cargo.toml`. [VERIFIED: Cargo.toml:1] |
| Concurrency primitives | Use standard Rust `std::thread`, `Arc`, `Mutex`, `Condvar`, atomics, `BinaryHeap`/`VecDeque`, and `mpsc`/custom wakeups as needed. [ASSUMED] | Existing preview/export/audio/process code already uses `std::thread`, `Arc`, `Mutex`, atomics, and channels. [VERIFIED: crates/bindings_node/src/realtime_preview_service.rs:7] [VERIFIED: crates/media_runtime/src/job.rs:6] |
| Serialization | Use existing `serde`/`serde_json` for typed config and telemetry crossing bindings. [VERIFIED: codebase grep] | The workspace crates already use `serde`/`serde_json`; `draft_model` pins `serde = 1.0.228` and `serde_json = 1.0.150`. [VERIFIED: crates/draft_model/Cargo.toml] |
| Error typing | Use local Rust enums and existing `thiserror` where already present; do not add a generic error stack for scheduler core. [ASSUMED] | `media_runtime`, `artifact_store`, and `project_store` already depend on `thiserror` in their own crates. [VERIFIED: codebase grep] |
| Binding exposure | Expose narrow Node-API methods for scheduler config/status/telemetry; do not expose raw queue internals or priority mutation to UI. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md] | `bindings_node` already uses explicit N-API methods rather than a generic scheduler bridge. [VERIFIED: crates/bindings_node/src/lib.rs:200] |
| Product tests | Extend Playwright/Electron product E2E and Rust crate tests; add `test:phase16`. [ASSUMED] | Current phase scripts follow Rust/domain + source guards + product E2E pattern. [VERIFIED: package.json:69] |

**Package Legitimacy Audit:** no new external packages are recommended for Phase 16 research, so no package-legitimacy gate is required. [VERIFIED: codebase grep]

### Job Model

Use a `JobEnvelope` with these required fields: `job_id`, `domain`, `priority`, `resource_class`, `freshness`, `cancellation_token`, `submitted_at`, `deadline_or_budget`, `queue_policy`, and `telemetry_labels`. [ASSUMED] Freshness should include `target_timeline_time_us: Option<u64>`, `playback_generation: Option<PlaybackGeneration>`, `project_session_id`, and `expected_revision` where state mutation depends on current draft/session state. [VERIFIED: crates/realtime_preview_runtime/src/request.rs:32] [VERIFIED: crates/bindings_node/src/project_session_service.rs:637]

Recommended domains: `InteractivePreview`, `ScrubSeek`, `Decode`, `Audio`, `Artifact`, `Export`, `MediaProbe`, `FilesystemIo`, and `Analysis`. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md]

Recommended resource classes: `GpuPresent`, `GpuDecode`, `CpuDecode`, `AudioRealtime`, `FfmpegProcess`, `DiskIo`, `SqliteWrite`, `BackgroundCpu`, and `ValidationProbe`. [ASSUMED]

Recommended priorities: `Realtime`, `Interactive`, `UserVisible`, `Background`, and `Maintenance`. [ASSUMED] `Realtime` and `Interactive` should preempt/coalesce obsolete work before any background/export admission consumes all worker capacity. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md]

### Queue Policy

Interactive preview/scrub/first-frame jobs should use bounded queues with coalescing by `(session_id, target_timeline_time_us, playback_generation, request_kind)` and should drop obsolete queued work when a newer generation arrives. [ASSUMED] Export/artifact/media-probe jobs should use bounded background queues that reject or delay work with a classified `SchedulerRejected` status instead of unbounded spawning. [ASSUMED] Cancellation must mark queued work, running work, and completion handlers, and it must decrement in-flight accounting exactly once. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md]

### Telemetry Budget Recommendations

| Metric | Initial Phase 16 Budget | Basis |
|--------|--------------------------|-------|
| Preview 30fps 3s playback | Account for >= 90 frames via presented+dropped policy; if dropped is 0, present >= 90 frames. | Existing product cadence gate. [VERIFIED: apps/desktop-electron/tests/product-preview-cadence.spec.ts:154] |
| Visible preview path | Backend `renderGraphGpu`, evidence `renderGraphGpuComposited`, no fallback active, visible center hash changes. | Existing product cadence and journey gates. [VERIFIED: apps/desktop-electron/tests/product-preview-cadence.spec.ts:102] [VERIFIED: apps/desktop-electron/tests/helpers/userJourney.ts:347] |
| Frame pacing | Interval p50 25-42ms, p95 <= 50ms, max <= 75ms, scheduler lateness p95 <= 12ms. | Existing cadence gate. [VERIFIED: apps/desktop-electron/tests/product-preview-cadence.spec.ts:191] |
| Presentation snapshot query | p50 <= 16ms, p95 <= 50ms; snapshot reads not a cadence driver. | Existing cadence gate. [VERIFIED: apps/desktop-electron/tests/product-preview-cadence.spec.ts:163] |
| Interactive queue latency under export/artifact/probe load | p95 <= 16ms, max <= 50ms for preview/scrub/first-frame admission. | Recommended to align with frame-budget sensitivity; needs Phase 16 confirmation. [ASSUMED] |
| Audio refill latency under export/artifact/probe load | Refill queue latency p95 <= 50ms and no underruns during 3s product playback. | Recommended because existing audio refill low-water is 1,500,000us and poll interval is 100ms, but no scheduler budget exists yet. [VERIFIED: crates/bindings_node/src/audio_service.rs:38] [ASSUMED] |
| Background queue rejection | Low-priority work returns classified rejection/coalescing status before unbounded memory growth. | Locked queue/backpressure decision. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md] |
| Stale completion | Stale preview/audio/artifact-visible completions mutate no state and increment stale rejection telemetry. | Locked stale-generation decision and existing preview/audio behavior. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md] [VERIFIED: crates/realtime_preview_runtime/src/session.rs:319] |

## Implementation Guidance

### Phase-Sized Slices

1. **Scheduler foundation:** add `crates/task_runtime`, workspace membership, typed job/domain/resource/freshness/cancel/config/telemetry models, deterministic fake-clock test harness, and `test:phase16-rust` skeleton. [ASSUMED]
2. **Preview and audio integration:** move binding-owned still/playback worker admission and audio refill admission into `task_runtime` realtime/interactive lanes while keeping compositor/audio output semantics in existing domain crates. [VERIFIED: crates/bindings_node/src/realtime_preview_service.rs:769] [VERIFIED: crates/bindings_node/src/audio_service.rs:994]
3. **Export and validation integration:** replace binding-owned `ExportJobRegistry` spawning with scheduler-managed export jobs; `media_runtime::run_export_job` remains the FFmpeg process executor but no longer owns cross-domain admission or fairness. [VERIFIED: crates/bindings_node/src/preview_export_service.rs:226] [VERIFIED: crates/media_runtime/src/job.rs:211]
4. **Artifact/probe/IO integration:** route thumbnail/waveform/proxy generation, artifact GC/refresh, media ffprobe, and project bundle IO through scheduler resource classes; material/artifact state commits must re-check session revision, generation, and cancellation. [VERIFIED: crates/bindings_node/src/artifact_store_service.rs:445] [VERIFIED: crates/bindings_node/src/material_service.rs:177]
5. **Telemetry binding and developer diagnostics:** expose `getTaskRuntimeStatus`, `getTaskRuntimeTelemetry`, and typed config methods through `bindings_node`; Electron may show product-safe summaries but raw queue/resource details remain developer diagnostics. [ASSUMED] [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md]
6. **Guards and aggregate gates:** add Phase 16 source guards rejecting direct binding-level scheduler bypasses, update no-product-fallback checks, and add product E2E stress tests for export/artifact/probe pressure during preview/audio playback. [ASSUMED]

### File And Module Touch Map

| Area | Expected Touch | Reason |
|------|----------------|--------|
| `Cargo.toml` | Add `crates/task_runtime` workspace member. | Workspace membership is explicit today. [VERIFIED: Cargo.toml:1] |
| `crates/task_runtime/` | New crate with scheduler contracts, bounded queues, resource budgets, cancellation, telemetry, fake executors, and tests. | Locked new boundary. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md] |
| `crates/realtime_preview_runtime` | Either move shared `TimelineClock`/`PlaybackGeneration` to `task_runtime` or re-export/adapt them without duplicating types; integrate preview job freshness and queue telemetry. | Clock/generation currently live in preview runtime. [VERIFIED: crates/realtime_preview_runtime/src/clock.rs:7] |
| `crates/audio_engine` | Integrate audio buffer/refill freshness, underrun telemetry, and scheduler job metadata without moving audio semantics into bindings. | Audio runtime already has generation-aware status. [VERIFIED: crates/audio_engine/src/session.rs:93] |
| `crates/artifact_store` | Add scheduler-facing artifact job adapters and stale/cancel commit guards around generated artifact writes. | Artifact generation writes blobs and job chunks after generation. [VERIFIED: crates/artifact_store/src/generation.rs:312] |
| `crates/media_runtime` | Add scheduler-friendly job interfaces if needed; keep process execution and FFmpeg progress parsing here. | Runtime already owns process execution, progress, timeout, and cancellation. [VERIFIED: crates/media_runtime/src/job.rs:211] |
| `crates/media_runtime_desktop` | Keep desktop executor as backend; do not let it own global fairness. | Desktop executor is the desktop FFmpeg shell. [VERIFIED: crates/media_runtime_desktop/src/lib.rs:33] |
| `crates/bindings_node` | Replace direct worker registries and direct heavy work with scheduler submit/status/cancel/telemetry calls. | Current binding services own export/preview/audio/artifact/probe heavy paths. [VERIFIED: codebase grep] |
| `apps/desktop-electron/src/main` | Add narrow scheduler telemetry/status IPC; ensure test mocks cannot satisfy product scheduler success. | Main process currently routes explicit APIs and test mocks. [VERIFIED: apps/desktop-electron/src/main/index.ts:249] |
| `apps/desktop-electron/tests` | Add product stress E2E and assertions that visible preview/audio remain live under export/artifact/probe pressure. | Product E2E is required for visible behavior. [CITED: docs/product-e2e-acceptance-policy.md] |
| `scripts/phase16-source-guards.sh` | New guard blocking direct `thread::spawn`, direct `DesktopFfmpegExecutor::run`, direct artifact generation, and queue policy in Electron/product paths except allowlisted scheduler/runtime internals. | Existing phase guards enforce architecture boundaries. [VERIFIED: scripts/phase15-3-source-guards.sh:108] |
| `package.json` | Add `test:phase16-rust`, `test:phase16-source-guards`, `test:phase16-desktop`, and aggregate `test:phase16`. | No Phase 16 gate exists today. [VERIFIED: package.json:11] |

### Don't Hand-Roll

| Problem | Do Not Build | Use Instead | Why |
|---------|--------------|-------------|-----|
| FFmpeg process execution and progress parsing | New per-domain process runners | `media_runtime::run_export_job`, `run_process_with_timeout`, and `FfmpegExecutor` adapters | Existing runtime already handles argument arrays, timeouts, cancellation, summaries, and progress parsing. [VERIFIED: crates/media_runtime/src/job.rs:211] |
| Timeline freshness IDs | Parallel generation counters in each subsystem | Existing `PlaybackGeneration`/`TimelineClock`, moved or adapted through `task_runtime` | Existing preview/audio semantics already depend on this type. [VERIFIED: crates/realtime_preview_runtime/src/clock.rs:7] |
| Artifact persistence | New artifact DB or JSON job state | Existing `artifact_store` generation jobs/chunks, invalidation, quota, and blob store | `.veproj/project.json` stays canonical and artifacts are derived; artifact store already models generation. [CITED: AGENTS.md] [VERIFIED: crates/artifact_store/src/jobs.rs:167] |
| Product playback proof | DOM tokens, preview artifacts, CPU fingerprints, mock frames | Visible `renderGraphGpuComposited` realtime preview evidence and pixel motion | No-product-fallback policy forbids fallback as product success. [CITED: docs/no-product-fallback-policy.md] |
| Scheduler policy in Electron | Main-process queues, renderer debounces, user-visible backend selectors | Rust `task_runtime` contracts and narrow telemetry/status IPC | Locked ownership excludes Electron queue policy. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md] |
| Unbounded background spawning | One `thread::spawn` per export/preview/audio/artifact request | Bounded scheduler lanes with explicit resource budgets and rejection/coalescing | Current ad hoc spawns are the starvation risk Phase 16 must eliminate. [VERIFIED: codebase grep] |

### Source Guard Recommendations

- Add a Phase 16 guard that fails direct `thread::spawn` in `crates/bindings_node/src/preview_export_service.rs`, `realtime_preview_service.rs`, and `audio_service.rs` except where explicitly marked as scheduler-owned transition code during a single implementation slice. [ASSUMED]
- Add a guard that fails `DesktopFfmpegExecutor::default`, `DesktopFfmpegExecutor::with_timeout`, and `executor.run` in `bindings_node` services outside scheduler adapters. [ASSUMED]
- Add a guard that fails `generate_thumbnail_artifact`, `generate_proxy_artifact`, `generate_waveform_artifact`, and `probe_material_metadata` direct calls from binding/project-session paths unless routed through scheduler adapter functions. [ASSUMED]
- Extend no-product-fallback guard so scheduler success cannot be satisfied by test mock export/runtime/audio/artifact responses. [ASSUMED]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Rust framework | Cargo tests with Rust 1.95.0 workspace. [VERIFIED: Cargo.toml:21] |
| Desktop E2E framework | Playwright 1.61.0 through Electron desktop package scripts. [VERIFIED: apps/desktop-electron/package.json:29] |
| Existing fallback guard | `pnpm run test:no-product-fallback`. [VERIFIED: package.json:77] |
| Existing product cadence gate | `pnpm --filter @video-editor/desktop exec playwright test tests/product-preview-cadence.spec.ts --reporter=line`. [VERIFIED: scripts/phase15-3-desktop-gate.sh:27] |
| Recommended quick run | `cargo test -p task_runtime -- --nocapture && cargo test -p bindings_node scheduler -- --nocapture`. [ASSUMED] |
| Recommended full phase gate | `pnpm run test:phase16`. [ASSUMED] |

### Phase Requirements To Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| SCHED-01 | All required domains submit through scheduler with priority, cancellation, backpressure, target time, and generation fields. | Rust unit/contract + source guard | `cargo test -p task_runtime scheduler_contracts -- --nocapture`; `bash scripts/phase16-source-guards.sh` | ❌ Wave 0 |
| SCHED-02 | Export/artifact/probe pressure cannot block preview cadence, scrubbing, inspector edits, or audio refill. | Rust simulation + Playwright/Electron E2E | `cargo test -p task_runtime starvation -- --nocapture`; `pnpm --filter @video-editor/desktop exec playwright test tests/product-scheduler-stress.spec.ts --reporter=line` | ❌ Wave 0 |
| SCHED-03 | Resource budgets are explicit, configurable, and portable. | Rust unit + binding contract | `cargo test -p task_runtime config -- --nocapture && cargo test -p bindings_node scheduler_config -- --nocapture` | ❌ Wave 0 |
| SCHED-04 | Telemetry records queue latency, duration, cancellation, stale rejection, fallback/unavailable, cache hit, first-frame, dropped/repeated frames, queue depth, and saturation. | Rust unit + E2E telemetry assertion | `cargo test -p task_runtime telemetry -- --nocapture && pnpm --filter @video-editor/desktop exec playwright test tests/product-scheduler-stress.spec.ts --reporter=line` | ❌ Wave 0 |

### Required Rust Tests

- `task_runtime::tests::priority_scheduler_runs_interactive_before_background_when_resources_are_saturated`: submit long export/artifact jobs, then preview/scrub jobs; assert interactive queue latency budget and resource accounting. [ASSUMED]
- `task_runtime::tests::bounded_queue_coalesces_obsolete_preview_work`: fill preview queue with old generation requests, enqueue newer generation, assert old jobs are dropped/coalesced and stale telemetry increments. [ASSUMED]
- `task_runtime::tests::cancelled_job_releases_inflight_and_does_not_commit`: cancel queued and running jobs; assert in-flight count, queue depth, and completion handler behavior. [ASSUMED]
- `task_runtime::tests::stale_generation_completion_is_rejected_before_state_mutation`: simulate a generation change while artifact/preview/audio work runs; assert no stale completion mutates visible state. [ASSUMED]
- `task_runtime::tests::resource_budget_config_serializes_for_bindings_without_desktop_paths`: assert portable config JSON has domains/resource classes/counts but no Electron or platform-specific paths. [ASSUMED]
- `bindings_node::tests::scheduler_status_is_thin_and_product_safe`: assert binding returns status/telemetry snapshots without exposing raw queue internals to normal product UI. [ASSUMED]
- `bindings_node::tests::export_uses_scheduler_registry_not_binding_thread_registry`: assert starting export returns scheduler job ID/status and can be cancelled through scheduler. [ASSUMED]
- `bindings_node::tests::artifact_refresh_enqueues_jobs_without_inline_generation`: assert refresh status returns queued/running task summaries and does not call generator synchronously in the binding thread. [ASSUMED]

### Required Product E2E Tests

- Add `apps/desktop-electron/tests/product-scheduler-stress.spec.ts` with a workflow that creates/opens a project, imports moving video and audio fixtures, starts product preview playback, starts export from the top-right modal, triggers artifact refresh/thumbnail work, optionally imports/probes another media fixture, scrubs the timeline, and asserts visible preview motion plus scheduler telemetry remain within budget. [ASSUMED] Product E2E must start from the UI and verify visible preview/export evidence rather than implementation-adjacent signals. [CITED: docs/product-e2e-acceptance-policy.md]
- The E2E must fail if product playback uses `requestProjectSessionPreviewFrame`, artifact fallback, mock/backend selector paths, CPU probe evidence, DOM-only motion, or test mock export responses. [CITED: docs/no-product-fallback-policy.md] [VERIFIED: scripts/no-product-fallback-guards.sh:15]
- The E2E should assert `renderGraphGpuComposited`, visible center hash change, no fallback active, preview target-time advancement, frame pacing p95 <= 50ms, schedule lateness p95 <= 12ms, and new scheduler telemetry fields for queue latency/depth/resource saturation during export/artifact load. [VERIFIED: apps/desktop-electron/tests/product-preview-cadence.spec.ts:147] [ASSUMED]

### Wave 0 Gaps

- [ ] `crates/task_runtime/Cargo.toml` and `crates/task_runtime/src/lib.rs` with scheduler contracts and fake-clock tests. [ASSUMED]
- [ ] `scripts/phase16-source-guards.sh` for scheduler ownership and no direct heavy-work bypass. [ASSUMED]
- [ ] `apps/desktop-electron/tests/product-scheduler-stress.spec.ts` for export/artifact/probe pressure during preview/audio. [ASSUMED]
- [ ] `package.json` scripts: `test:phase16-rust`, `test:phase16-source-guards`, `test:phase16-desktop`, and `test:phase16`. [ASSUMED]
- [ ] Binding contract tests for scheduler status/config/telemetry. [ASSUMED]

### Runtime State Inventory

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | Artifact generation job/chunk state persists in artifact-store SQLite tables through `artifact_store::jobs`; job statuses include waiting/running/completed/failed/cancelRequested/cancelled/resumable. [VERIFIED: crates/artifact_store/src/jobs.rs:50] | Preserve or migrate persisted artifact job semantics behind scheduler adapters; ensure stale/cancelled jobs do not commit ready artifact rows. |
| Live service config | No external live service config was found in the Phase 16 scope; scheduler config should be typed Rust config surfaced by narrow binding, not UI-local config. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md] | Add default scheduler config in Rust and optional desktop-dev override through binding only. |
| OS-registered state | No OS-level registrations were found for scheduler state; current runtime state is in-process worker threads, native preview surfaces, audio output streams, and FFmpeg child processes. [VERIFIED: codebase grep] | Cancellation/drop must stop in-process workers and child processes deterministically before old paths are removed. |
| Secrets/env vars | No scheduler secrets were identified; test-only env flags exist for runtime/export/audio/preview behavior and must remain test harnesses, not product success paths. [VERIFIED: apps/desktop-electron/src/main/index.ts:919] | Add source guards so test env mocks cannot satisfy product scheduler acceptance. |
| Build artifacts | Native Node-API binding is built via `napi build`; desktop package script provisions bundled FFmpeg runtime before building. [VERIFIED: apps/desktop-electron/package.json:13] | After adding `task_runtime`, run native build/tests so generated binding artifacts reflect new APIs; do not rely on system FFmpeg for product runtime. |

### Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust/Cargo | Rust crates and scheduler tests | yes | `rustc 1.95.0`, `cargo 1.95.0` [VERIFIED: local command] | None needed |
| Node.js | Electron/Vite/Playwright tooling | yes | `v24.15.0` [VERIFIED: local command] | Project engine expects 24.12.0+, so current version is suitable. [VERIFIED: package.json:7] |
| pnpm | Desktop scripts | yes | `10.32.1` [VERIFIED: local command] | None needed |
| FFmpeg/ffprobe system binaries | Local smoke convenience | yes | `8.1.2` [VERIFIED: local command] | Product uses bundled runtime provisioned by desktop scripts, not PATH. [CITED: docs/runtime-boundaries.md] |
| GSD `gsd-tools.cjs` | Research-plan/init/cache/commit seams | broken | Fails with `Cannot find module '../../../package.json'` [VERIFIED: local command] | Manual codebase research and direct file write; do not block Phase 16 planning on this shim failure. |

### Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | Desktop local editor phase has no auth surface. [VERIFIED: codebase grep] |
| V3 Session Management | partial | Treat project/preview/audio/export scheduler sessions as local opaque IDs with generation and cancellation; do not expose raw internals to renderer. [ASSUMED] |
| V4 Access Control | partial | IPC sender assertions and project-session ownership checks must remain before scheduler actions. [VERIFIED: apps/desktop-electron/src/main/index.ts:249] [VERIFIED: crates/bindings_node/src/project_session_service.rs:686] |
| V5 Input Validation | yes | Use typed serde payloads, bounded queue config validation, checked paths, expected revision, and integer microseconds. [VERIFIED: crates/bindings_node/src/lib.rs:200] |
| V6 Cryptography | no | Scheduler phase does not introduce cryptography; do not hand-roll crypto. [ASSUMED] |

Known threat patterns:

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Renderer/main process bypasses scheduler and starts direct FFmpeg/artifact work | Tampering / Denial of Service | Source guards plus narrow `bindings_node` scheduler APIs. [ASSUMED] |
| Unbounded jobs exhaust CPU/IO/memory and starve preview/audio | Denial of Service | Bounded queues, resource budgets, explicit rejection/coalescing, and starvation tests. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md] |
| Stale generation/export/artifact result mutates current draft-visible state | Tampering | Completion freshness gate with `PlaybackGeneration`/expected revision and stale rejection telemetry. [VERIFIED: crates/realtime_preview_runtime/src/session.rs:319] |
| Shell/argument injection through media/export paths | Tampering | Keep argument-array process launches in `media_runtime`/desktop executor; do not construct shell command strings in UI. [VERIFIED: crates/media_runtime/src/job.rs:227] |
| Product fallback accepted as success | Spoofing | `test:no-product-fallback` plus E2E assertions for `renderGraphGpuComposited` evidence and visible pixel motion. [VERIFIED: scripts/no-product-fallback-guards.sh:15] |

## Risks And Open Questions

### Architecture Risks

- Moving `PlaybackGeneration` from `realtime_preview_runtime` into `task_runtime` may cause broad type churn; re-exporting or adapting may be safer for Phase 16 if the type remains single-source and not duplicated. [ASSUMED]
- A scheduler that only wraps current `thread::spawn` sites without removing binding-owned worker policy would violate D-01/D-04 and would not prove SCHED-02. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md]
- If artifact refresh remains synchronous while export is scheduler-managed, Phase 16 can still starve preview/audio through thumbnail generation and SQLite writes. [VERIFIED: crates/bindings_node/src/artifact_store_service.rs:445]
- If media import/probe remains synchronous, importing large or slow media can still block project-session command processing and obscure SCHED-01. [VERIFIED: crates/bindings_node/src/project_session_service.rs:1704]
- If telemetry is only exposed in Rust tests and not available through desktop product E2E, tests may miss real Electron/N-API contention. [ASSUMED]
- Current `media_runtime::run_process_with_timeout` and `run_export_job` use helper threads for stdout/stderr/progress; scheduler should treat those as resource cost of FFmpeg jobs rather than remove them blindly. [VERIFIED: crates/media_runtime/src/process.rs:56] [VERIFIED: crates/media_runtime/src/job.rs:244]

### Open Questions

1. **Should `TimelineClock`/`PlaybackGeneration` move to `task_runtime` or stay in `realtime_preview_runtime` with re-exports?**  
   What we know: the types already support preview and audio, and Phase 16 needs them for cross-domain freshness. [VERIFIED: crates/realtime_preview_runtime/src/clock.rs:7]  
   Recommendation: keep one canonical type; move only if the edit is contained and tests prove no duplicate generation type appears. [ASSUMED]

2. **Should material import return immediately with pending probe status or block on a scheduler `submit_and_wait` probe?**  
   What we know: current import returns after ffprobe and draft mutation, while SCHED-01 requires media probing through queues. [VERIFIED: crates/bindings_node/src/material_service.rs:177]  
   Recommendation: use scheduler admission for probe either way; choose immediate pending status only if the UI already supports material probe-pending state without breaking product flow. [ASSUMED]

3. **How many desktop worker lanes should default config allocate?**  
   What we know: Phase 16 requires explicit configurable limits, but hardware support targets are not specified in the context. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md]  
   Recommendation: start conservative with distinct realtime/interactive, FFmpeg process, disk IO, and background CPU capacities, then tune from telemetry. [ASSUMED]

4. **Should export validation run in the export resource class or a separate validation/probe class?**  
   What we know: export thread currently runs FFmpeg export, then ffprobe validation before marking completed. [VERIFIED: crates/bindings_node/src/preview_export_service.rs:321]  
   Recommendation: represent validation as a child job or phase under scheduler telemetry so export completion cannot hide validation starvation. [ASSUMED]

### Assumptions Log

| # | Claim | Risk if Wrong |
|---|-------|---------------|
| A1 | Phase 16 can avoid adding external scheduler/async packages and use standard Rust concurrency for first implementation. | If workload complexity exceeds this, planner may need a package audit and a broader async/runtime migration. |
| A2 | Initial interactive queue latency budget p95 <= 16ms / max <= 50ms is feasible on supported desktop hardware. | If too strict, E2E may be flaky; if too loose, scheduler may still hide starvation. |
| A3 | Audio refill p95 <= 50ms and no underruns during 3s stress playback is a reasonable first budget. | If audio device behavior is highly variable, tests may need deterministic fake audio output plus smaller product smoke assertions. |
| A4 | Product scheduler stress can be covered by a new Playwright spec using existing fixture media and test controls. | If current UI lacks hooks to trigger enough artifact/probe pressure, planner must add test-only stress commands guarded from product success. |
| A5 | Moving or adapting `PlaybackGeneration` can be done without blocking Phase 16. | Duplicate generation types would weaken stale-result guarantees. |

## Sources

### Primary

- `AGENTS.md` - project architecture, no fallback, no legacy compatibility, time model, render graph, testing, and licensing constraints. [CITED: AGENTS.md]
- `.planning/PROJECT.md` - active product scope and constraints. [CITED: .planning/PROJECT.md]
- `.planning/ROADMAP.md` Phase 16 - goal, requirements, and success criteria. [CITED: .planning/ROADMAP.md]
- `.planning/REQUIREMENTS.md` SCHED-01 through SCHED-04. [CITED: .planning/REQUIREMENTS.md]
- `.planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md` - locked scheduler decisions and scope. [CITED: .planning/phases/16-task-scheduler-job-isolation-and-performance-telemetry/16-CONTEXT.md]
- `.planning/notes/production-editor-architecture-decisions.md` section 7 - unified task runtime. [CITED: .planning/notes/production-editor-architecture-decisions.md]
- `docs/runtime-boundaries.md` - runtime ownership and Phase 16 scheduling boundary. [CITED: docs/runtime-boundaries.md]
- `docs/no-product-fallback-policy.md` - fallback cannot satisfy product success. [CITED: docs/no-product-fallback-policy.md]
- `docs/refactor-and-legacy-cleanup-policy.md` - replace obsolete product paths. [CITED: docs/refactor-and-legacy-cleanup-policy.md]
- `docs/product-e2e-acceptance-policy.md` - user-visible completion requires normal product E2E evidence. [CITED: docs/product-e2e-acceptance-policy.md]

### Codebase

- `crates/realtime_preview_runtime/src/clock.rs`, `request.rs`, `scheduler.rs`, `session.rs`, `telemetry.rs` - preview clock/generation, request, cadence, stale rejection, and telemetry. [VERIFIED: codebase grep]
- `crates/audio_engine/src/session.rs`, `telemetry.rs` and `crates/bindings_node/src/audio_service.rs` - audio generation/cancellation/refill/decode behavior. [VERIFIED: codebase grep]
- `crates/artifact_store/src/jobs.rs`, `generation.rs` and `crates/bindings_node/src/artifact_store_service.rs` - artifact job state, cancel/resume, and inline thumbnail refresh. [VERIFIED: codebase grep]
- `crates/media_runtime/src/job.rs`, `process.rs`, `probe.rs` and `crates/media_runtime_desktop/src/lib.rs` - FFmpeg/ffprobe process execution, timeout, cancellation, and desktop executor. [VERIFIED: codebase grep]
- `crates/bindings_node/src/preview_export_service.rs`, `realtime_preview_service.rs`, `project_session_service.rs`, `material_service.rs`, `lib.rs` - current integration and legacy paths. [VERIFIED: codebase grep]
- `apps/desktop-electron/src/main/index.ts`, `realtimePreviewHost.ts`, `apps/desktop-electron/tests/product-preview-cadence.spec.ts`, `tests/helpers/userJourney.ts` - desktop routing, product preview host, and existing cadence/product evidence gates. [VERIFIED: codebase grep]
- `package.json`, `apps/desktop-electron/package.json`, `scripts/no-product-fallback-guards.sh`, `scripts/phase15-3-source-guards.sh`, `scripts/phase14-source-guards.sh` - current scripts and source guards. [VERIFIED: codebase grep]

## Metadata

**Confidence breakdown:**
- Current codebase findings: HIGH - verified with direct file reads and code grep.
- Production target architecture: HIGH for ownership and boundary; MEDIUM for exact queue algorithms and telemetry budgets.
- Validation architecture: HIGH for existing gates; MEDIUM for proposed new Phase 16 tests and thresholds.

**Research date:** 2026-06-23  
**Valid until:** 2026-07-23, or earlier if Phase 16 implementation changes scheduler ownership boundaries.

**Tooling note:** `gsd-tools.cjs query init.phase-op 16` and `gsd_run query init.phase-op 16` both failed locally with `Cannot find module '../../../package.json'`, so research-plan caching and GSD commit automation were not available in this run. [VERIFIED: local command]

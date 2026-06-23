# Phase 16: Task Scheduler, Job Isolation, And Performance Telemetry - Pattern Map

**Mapped:** 2026-06-23
**Files analyzed:** 31 new/modified/gated files
**Analogs found:** 28 / 31

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `Cargo.toml` | config | transform | `Cargo.toml` workspace members lines 1-18 | exact |
| `crates/task_runtime/Cargo.toml` | config | transform | `crates/media_runtime/Cargo.toml` lines 1-20 | exact |
| `crates/task_runtime/src/lib.rs` | provider | event-driven | `crates/realtime_preview_runtime/src/lib.rs` lines 1-67; `crates/media_runtime/src/lib.rs` lines 1-80 | exact |
| `crates/task_runtime/src/job.rs` | model | event-driven | `crates/realtime_preview_runtime/src/request.rs` lines 9-49; `crates/media_runtime/src/job.rs` lines 16-120 | exact |
| `crates/task_runtime/src/freshness.rs` | model | event-driven | `crates/realtime_preview_runtime/src/clock.rs` lines 7-24 and 96-144 | exact |
| `crates/task_runtime/src/cancellation.rs` | utility | event-driven | `crates/media_runtime/src/job.rs` lines 180-195; `crates/audio_engine/src/session.rs` lines 171-189 | exact |
| `crates/task_runtime/src/config.rs` | config | event-driven | `crates/audio_engine/src/session.rs` lines 39-49; `crates/realtime_preview_runtime/src/scheduler.rs` lines 19-23 | role-match |
| `crates/task_runtime/src/scheduler.rs` | service | event-driven | `crates/realtime_preview_runtime/src/scheduler.rs` lines 400-515 | role-match |
| `crates/task_runtime/src/telemetry.rs` | utility | event-driven | `crates/realtime_preview_runtime/src/telemetry.rs` lines 10-29 and 121-195; `crates/audio_engine/src/telemetry.rs` lines 7-18 | exact |
| `crates/task_runtime/src/testing.rs` | testkit | event-driven | `crates/realtime_preview_runtime/src/scheduler.rs` lines 900-928; `crates/bindings_node/tests/export_commands.rs` lines 1-58 | role-match |
| `crates/task_runtime/tests/scheduler_contracts.rs` | test | event-driven | `crates/realtime_preview_runtime/src/scheduler.rs` lines 900-928; `crates/audio_engine/src/session.rs` lines 254-300 | role-match |
| `crates/task_runtime/tests/starvation.rs` | test | event-driven | `apps/desktop-electron/tests/product-preview-cadence.spec.ts` lines 147-203 | partial |
| `crates/task_runtime/tests/scheduler_telemetry.rs` | test | event-driven | `crates/realtime_preview_runtime/src/telemetry.rs` lines 206-278 | exact |
| `crates/bindings_node/tests/scheduler_runtime.rs` | test | request-response | `crates/bindings_node/tests/export_commands.rs` lines 171-212; `crates/bindings_node/tests/audio_service.rs` lines 50-113 | exact |
| `scripts/phase16-source-guards.sh` | utility | transform | `scripts/no-product-fallback-guards.sh` lines 1-25; `scripts/phase15-3-source-guards.sh` lines 108-153 | exact |
| `apps/desktop-electron/tests/product-scheduler-stress.spec.ts` | test | request-response | `apps/desktop-electron/tests/product-preview-cadence.spec.ts` lines 90-120, 147-203, 400-420 | exact |
| `package.json` | config | batch | `package.json` phase scripts lines 69-83 | exact |
| `crates/realtime_preview_runtime` | service | event-driven | `crates/realtime_preview_runtime/src/scheduler.rs` lines 464-512 | exact |
| `crates/audio_engine` | service | event-driven | `crates/audio_engine/src/session.rs` lines 254-300 | exact |
| `crates/artifact_store` | service/model | CRUD + file-I/O | `crates/artifact_store/src/jobs.rs` lines 167-240 and 460-637 | exact |
| `crates/media_runtime` | service | streaming | `crates/media_runtime/src/job.rs` lines 211-359; `crates/media_runtime/src/probe.rs` lines 151-190 | exact |
| `crates/bindings_node/src/lib.rs` | route | request-response | `crates/bindings_node/src/lib.rs` lines 200-243 and 245-290 | exact |
| `crates/bindings_node/src/realtime_preview_service.rs` | service | event-driven | same file lines 195-203, 769-865, 888-1048 | legacy-to-replace |
| `crates/bindings_node/src/preview_export_service.rs` | service | streaming | same file lines 217-330 and 412-490 | legacy-to-replace |
| `crates/bindings_node/src/audio_service.rs` | service | event-driven + streaming | same file lines 994-1061 and 1154-1210 | legacy-to-replace |
| `crates/bindings_node/src/artifact_store_service.rs` | service | CRUD + file-I/O | same file lines 445-520 and 611-690 | legacy-to-replace |
| `crates/bindings_node/src/material_service.rs` | service | request-response | same file lines 177-190 | legacy-to-replace |
| `crates/bindings_node/src/project_session_service.rs` | service | request-response | same file lines 637-660 and 1704-1795 | legacy-to-replace |
| `apps/desktop-electron/src/main/index.ts` | route | request-response | same file lines 647-651 and 919-1012 | role-match |
| `apps/desktop-electron/src/main/realtimePreviewHost.ts` | provider | event-driven | same file lines 480-540 and 1038-1054 | role-match |
| `apps/desktop-electron/src/preload/index.ts` | route | request-response | no direct scheduler analog read | no-analog |

## Existing Analogs To Reuse

| Pattern | Source | Concrete Excerpt | Apply To |
|---|---|---|---|
| Workspace crate registration | `Cargo.toml:1-18` | Workspace members are explicit strings and shared package metadata is at `Cargo.toml:21-24`. | Add `crates/task_runtime`; do not use planned-members. |
| Crate manifest | `crates/media_runtime/Cargo.toml:1-20` | Package uses `edition.workspace`, `rust-version.workspace`, `license.workspace`, `publish = false`, `[lib] path = "src/lib.rs"`, and local path deps. | `crates/task_runtime/Cargo.toml`. |
| Public crate boundary | `crates/realtime_preview_runtime/src/lib.rs:1-67` | Module list followed by explicit `pub use` groups for runtime contracts. | `task_runtime/src/lib.rs`; keep exports narrow and typed. |
| Runtime boundary doc comment | `crates/media_runtime/src/lib.rs:1-4` | Declares ownership boundary and dependency direction in crate docs. | `task_runtime/src/lib.rs`; state that task runtime owns scheduling policy and semantic crates stay pure. |
| Typed serde payloads | `crates/realtime_preview_runtime/src/request.rs:32-49` | `RealtimePreviewFrameRequest` uses `#[serde(rename_all = "camelCase", deny_unknown_fields)]`, `Microseconds`, `PlaybackGeneration`, optional cancellation, queue latency, duration, fallback/cache/drop flags. | `task_runtime/src/job.rs` job envelope and scheduler telemetry labels. |
| Shared generation freshness | `crates/realtime_preview_runtime/src/clock.rs:7-24,96-144` | `PlaybackGeneration` is a transparent `u64` wrapper; `TimelineClock` owns `position`, frame rate, playback rate, state, generation; `seek` advances generation. | `task_runtime/src/freshness.rs`; keep one canonical generation type, via move or re-export. |
| Bounded queue policy | `crates/realtime_preview_runtime/src/scheduler.rs:400-415` | `RealtimePlaybackPresentationQueuePolicy` exposes `max_in_flight_presentations`, `backpressure_timeout`, and `has_capacity`. | `task_runtime/src/config.rs` and `scheduler.rs` resource budgets/backpressure. |
| Runtime-owned presentation scheduler | `crates/realtime_preview_runtime/src/scheduler.rs:424-512` | Scheduler owns config/draft snapshot/evidence; `present_tick` prepares graph, calls presenter trait, fails if no frame was presented, and records `RenderGraphGpuComposited` evidence. | Keep preview compositor in preview runtime; schedule admission in `task_runtime`. |
| Error style | `crates/realtime_preview_runtime/src/scheduler.rs:644-660` | Local enum, `Display`, and `Error` impl instead of generic stringly errors. | `task_runtime` scheduler and rejection errors. |
| Telemetry snapshot | `crates/realtime_preview_runtime/src/telemetry.rs:10-29` | First-frame/seek latency, queue latency, render duration, frame counts, stale/cancel/fallback/cache counts, target time, generation, frame pacing. | `task_runtime/src/telemetry.rs` scheduler-wide snapshot. |
| Percentile sample buffer | `crates/realtime_preview_runtime/src/telemetry.rs:121-195` | Bounded `VecDeque`, p50/p95/max summaries, schedule lateness, render duration, dropped frames. | Queue latency/run duration/resource saturation histograms. |
| Audio stale/cancel/bounded handling | `crates/audio_engine/src/session.rs:254-300` | Request computes `stale_rejected`, `canceled`, `bounded_rejected`, then records telemetry and returns classified result. | `task_runtime` completion gate and scheduler tests. |
| Audio telemetry counters | `crates/audio_engine/src/telemetry.rs:7-18` | Presented/stale/canceled/underrun/degraded/bounded counters with target time and generation. | Scheduler audio lane telemetry. |
| Durable artifact job model | `crates/artifact_store/src/jobs.rs:167-240` | Generation request/job/chunk/summary structs use typed artifact kind, status, progress, chunk target microseconds, and JSON generation parameters. | Artifact scheduler adapter; do not create a second artifact job DB. |
| Artifact cancel/resume/status | `crates/artifact_store/src/jobs.rs:460-637` | Cancel requests set `cancelRequested`; acknowledgement marks chunks/job cancelled; active-job listing filters terminal states. | Scheduler artifact cancellation and UI task summaries. |
| Artifact write guard | `crates/artifact_store/src/generation.rs:312-422` | Generation checks cancel before start, after chunk start, after generator error, before blob write, then completes chunk. | Add scheduler freshness/cancel gate before artifact-visible state commits. |
| FFmpeg export executor | `crates/media_runtime/src/job.rs:211-359` | `run_export_job` launches FFmpeg with args array, emits started/progress/completed, supports cancel kill, timeout, and classified errors. | `task_runtime` export adapter; do not rebuild FFmpeg process management. |
| FFmpeg progress parsing | `crates/media_runtime/src/job.rs:361-455` | `-progress pipe:1` handling and microsecond progress parsing. | Export telemetry mapping. |
| Process timeout helper | `crates/media_runtime/src/process.rs:11-53` | External process wait loop has explicit timeout and kills child. | Probe/validation jobs under scheduler resource budgets. |
| Material probe executor | `crates/media_runtime/src/probe.rs:151-190` | ffprobe uses argument array, `executor.run`, classified probe errors, JSON normalization. | Media-probe scheduler adapter. |
| N-API typed JSON parsing | `crates/bindings_node/src/lib.rs:200-243` | Export APIs parse specific request structs and return envelope errors for invalid payloads. | `getTaskRuntimeStatus`, `getTaskRuntimeTelemetry`, scheduler config APIs. |
| Explicit audio N-API route names | `crates/bindings_node/src/lib.rs:245-290` | One exported method per command, no generic queue mutation API. | Scheduler binding surface should be status/config/telemetry/submit through typed domain methods only. |
| Product-safe preview status | `apps/desktop-electron/src/main/realtimePreviewHost.ts:1038-1054` | Product-ready only when backend is `renderGraphGpu` and evidence is `renderGraphGpuComposited`; otherwise product backend is `none`. | Scheduler UI status must stay productized; raw queues are diagnostics only. |
| IPC sender guard | `apps/desktop-electron/src/main/index.ts:647-651` | Rejects IPC from untrusted renderer frame URL. | Any new scheduler IPC routes. |
| Product E2E evidence | `apps/desktop-electron/tests/product-preview-cadence.spec.ts:147-203` | Requires no artifact fallback, renderGraphGpu path, 3s target advance, frame accounting, lightweight snapshots, pacing p95, visible pixel change. | `product-scheduler-stress.spec.ts`. |
| Product readiness polling | `apps/desktop-electron/tests/product-preview-cadence.spec.ts:400-420` | Waits for productReady, no fallback, renderGraphGpuComposited evidence, presented count and target advancing. | Stress E2E helper. |
| Source guard style | `scripts/no-product-fallback-guards.sh:1-25` | `fail_if_matches` uses `rg -n`, emits policy violation, exits nonzero. | `scripts/phase16-source-guards.sh`. |
| Existing boundary guard style | `scripts/phase15-3-source-guards.sh:108-153` | Blocks Electron cadence/fake evidence/polling and requires product journey evidence strings. | Phase 16 scheduler ownership guards. |

## Pattern Assignments

### `crates/task_runtime/*` (runtime service/model/config/telemetry)

| New File | Copy Pattern From | Required Shape |
|---|---|---|
| `Cargo.toml` | `crates/media_runtime/Cargo.toml:1-20`; root `Cargo.toml:1-18` | Add a dependency-light crate with workspace edition/rust/license; likely deps: `draft_model`, `serde`, `serde_json`, optional `thiserror` only if local error enums need it. |
| `src/lib.rs` | `crates/realtime_preview_runtime/src/lib.rs:1-67`; `crates/media_runtime/src/lib.rs:1-4` | Crate docs define Rust-owned scheduler policy boundary; modules and `pub use` are explicit. |
| `src/job.rs` | `crates/realtime_preview_runtime/src/request.rs:32-49`; `crates/media_runtime/src/job.rs:16-120` | `JobId`, `JobDomain`, `JobPriority`, `ResourceClass`, `JobFreshness`, `JobEnvelope`, `JobResult`; serde camelCase and deny unknown fields. |
| `src/freshness.rs` | `crates/realtime_preview_runtime/src/clock.rs:7-24,96-144`; `crates/bindings_node/src/project_session_service.rs:644-660` | Reuse or move `PlaybackGeneration`; freshness includes target microseconds, project session ID, expected revision, and stale rejection classification. |
| `src/cancellation.rs` | `crates/media_runtime/src/job.rs:180-195`; `crates/audio_engine/src/session.rs:171-189` | Cloneable cancel token; cancel must affect queued/running/completion gates and telemetry exactly once. |
| `src/config.rs` | `crates/realtime_preview_runtime/src/scheduler.rs:400-415`; `crates/audio_engine/src/session.rs:39-49` | Explicit resource capacities, queue depths, timeout/backpressure, domain defaults; portable, no Electron/FFmpeg path assumptions. |
| `src/scheduler.rs` | `crates/realtime_preview_runtime/src/scheduler.rs:424-512`; `crates/audio_engine/src/session.rs:254-300` | Owns admission, capacity, coalescing/rejection, execution state, and completion gates; domain executors remain adapters. |
| `src/telemetry.rs` | `crates/realtime_preview_runtime/src/telemetry.rs:10-29,121-195`; `crates/audio_engine/src/telemetry.rs:7-18` | Queue latency, job duration, wait/run time, cancel/stale/reject/fallback/cache counts, first-frame, dropped/repeated frames, queue depth, resource saturation. |
| `src/testing.rs` | `crates/realtime_preview_runtime/src/scheduler.rs:900-928`; `crates/bindings_node/tests/export_commands.rs:1-58` | Fake clock/executor harness, deterministic queues, saturation scenarios, explicit assertions. |

Core code excerpts to copy structurally:

```rust
// crates/realtime_preview_runtime/src/request.rs:32-49
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RealtimePreviewFrameRequest {
    pub target_time: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub cancellation_token: Option<PreviewCancellationToken>,
    pub mode: PreviewRequestMode,
    pub queue_latency_ms: u64,
    pub render_duration_ms: u64,
    pub cache_hit: bool,
    pub repeated_frame: bool,
    pub dropped_frame: bool,
}
```

```rust
// crates/realtime_preview_runtime/src/scheduler.rs:400-415
pub struct RealtimePlaybackPresentationQueuePolicy {
    pub max_in_flight_presentations: usize,
    pub backpressure_timeout: Duration,
}

impl RealtimePlaybackPresentationQueuePolicy {
    pub const fn has_capacity(self, in_flight_count: usize) -> bool {
        in_flight_count < self.max_in_flight_presentations
    }
}
```

```rust
// crates/audio_engine/src/session.rs:254-261
let stale_rejected = request.playback_generation != self.clock.generation();
let canceled = request
    .cancellation_token
    .map(|token| self.canceled_tokens.contains(&token))
    .unwrap_or(false);
let bounded_rejected = self.bound_violations(&request).is_some();
let presented = !stale_rejected && !canceled && !bounded_rejected;
```

### `crates/bindings_node` scheduler integration (route/service, request-response)

| Target File | Copy Pattern From | Required Shape |
|---|---|---|
| `src/lib.rs` | `crates/bindings_node/src/lib.rs:200-243,245-290` | Add narrow `#[napi(js_name = "...")]` methods for scheduler status/telemetry/config. Parse typed JSON and return existing envelope errors. |
| `tests/scheduler_runtime.rs` | `crates/bindings_node/tests/export_commands.rs:171-212`; `crates/bindings_node/tests/audio_service.rs:50-113` | Cover submit/status/cancel telemetry, stale generation, wrong project/session identity, and product-safe JSON. |
| `src/realtime_preview_service.rs` | replace lines 195-203, 769-865, 888-1048 | Binding can keep thin session handles and presenter wiring; scheduler owns worker lifecycle/admission. |
| `src/preview_export_service.rs` | replace lines 217-330; preserve status/event mapping lines 412-490 | Start export through task runtime; keep `media_runtime::run_export_job` as executor. |
| `src/audio_service.rs` | replace lines 994-1061 and 1154-1210 | Audio refill/decode jobs enter realtime/decode lanes; binding does not own refill thread or direct FFmpeg execution. |
| `src/artifact_store_service.rs` | replace lines 445-520 and 611-690 | Refresh enqueues artifact jobs and returns task status; generator runs under scheduler resource budget. |
| `src/material_service.rs` / `project_session_service.rs` | replace `material_service.rs:177-190` and `project_session_service.rs:1704-1795`; keep `project_session_service.rs:644-660` | Probe through scheduler and commit only if expected revision/session is still current. |

Binding route excerpt to copy:

```rust
// crates/bindings_node/src/lib.rs:200-212
#[napi(js_name = "startProjectSessionExport")]
pub fn start_project_session_export(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<StartProjectSessionExportRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid startProjectSessionExport payload: {error}"),
                Some("startProjectSessionExport".to_string()),
            ));
        }
    };
    start_project_session_export_command(request)
}
```

Revision freshness excerpt to copy:

```rust
// crates/bindings_node/src/project_session_service.rs:644-660
pub(crate) fn project_session_snapshot(
    session_id: &str,
    expected_revision: u64,
) -> std::result::Result<ProjectSessionPreviewSnapshot, String> {
    let registry = global_project_session_registry();
    let registry = registry.lock().map_err(|_| "project session registry lock poisoned".to_string())?;
    let session = registry.sessions.get(session_id).ok_or_else(|| format!("Project session not found: {session_id}"))?;
    if expected_revision != session.revision {
        return Err(format!("Stale project session revision: expected {}, current {}", expected_revision, session.revision));
    }
```

### Tests and guards

| New File | Copy Pattern From | Required Shape |
|---|---|---|
| `crates/task_runtime/tests/scheduler_contracts.rs` | `crates/realtime_preview_runtime/src/scheduler.rs:900-928`; `crates/audio_engine/src/session.rs:254-300` | Assert priority/resource/freshness/cancel/rejection contracts using fake executor. |
| `crates/task_runtime/tests/starvation.rs` | `apps/desktop-electron/tests/product-preview-cadence.spec.ts:147-203` | Simulate export/artifact/probe saturation, then assert interactive queue latency and no preview/audio starvation. |
| `crates/task_runtime/tests/scheduler_telemetry.rs` | `crates/realtime_preview_runtime/src/telemetry.rs:206-278` | Assert p50/p95/max summaries, queue depth, resource saturation, stale/cancel/cache counters. |
| `crates/bindings_node/tests/scheduler_runtime.rs` | `crates/bindings_node/tests/project_session.rs:1489-1533,2287-2319,2373-2397,2436-2517` | Assert stale revision rejection, no renderer draft payload, explicit project session identity, and thin binding status. |
| `apps/desktop-electron/tests/product-scheduler-stress.spec.ts` | `apps/desktop-electron/tests/product-preview-cadence.spec.ts:90-120,147-203,400-420` | Normal user workflow: import moving AV fixture, start playback, start export/artifact/probe load, scrub, assert visible preview motion and scheduler telemetry budgets. |
| `scripts/phase16-source-guards.sh` | `scripts/no-product-fallback-guards.sh:1-25`; `scripts/phase15-3-source-guards.sh:108-153`; `scripts/phase14-source-guards.sh:85-110,176-183` | Use `rg` fail patterns plus required evidence strings. Allowlist scheduler/runtime internals only. |
| `package.json` | `package.json:69-83` | Add `test:phase16-rust`, `test:phase16-source-guards`, `test:phase16-desktop`, `test:phase16`; keep `test:no-product-fallback`. |

Test/guard excerpts:

```typescript
// apps/desktop-electron/tests/product-preview-cadence.spec.ts:412-420
if (
  lastState?.ok === true &&
  lastState.productReady &&
  !lastState.fallbackActive &&
  lastState.backend === "renderGraphGpu" &&
  lastState.contentEvidence?.source === "renderGraphGpuComposited" &&
  presented > baselinePresented &&
  target > baselineTarget
) {
  return lastState;
}
```

```bash
# scripts/no-product-fallback-guards.sh:4-12
fail_if_matches() {
  local label="$1"
  local pattern="$2"
  shift 2

  if rg -n "$pattern" "$@"; then
    echo "no-product-fallback violation: ${label}" >&2
    exit 1
  fi
}
```

## Legacy Binding-Owned Paths To Replace Or Gate

| Current File | Existing Path | Evidence | Phase 16 Action | Guard Suggestion |
|---|---|---|---|---|
| `crates/bindings_node/src/realtime_preview_service.rs` | Binding registry owns `still_frame_workers` and `playback_workers`. | lines 195-203 | Move worker lifecycle/admission to `task_runtime` interactive lanes; binding keeps session/presenter/status facade. | Fail `still_frame_workers|playback_workers|rt-preview-still|rt-preview-playback` outside scheduler adapter allowlist. |
| `crates/bindings_node/src/realtime_preview_service.rs` | `thread::Builder::spawn` starts still/playback loops. | lines 769-865 | Replace with scheduler submissions and cancel handles. | Fail `thread::Builder::new\(\).*rt-preview` in binding service. |
| `crates/bindings_node/src/realtime_preview_service.rs` | Playback loop sleeps on idle poll and drives frames from binding worker. | lines 888-1048 | Scheduler should own cadence/backpressure; binding receives telemetry/events. | Fail `thread::sleep\(REALTIME_PLAYBACK_IDLE_POLL_INTERVAL\)` in binding path. |
| `crates/bindings_node/src/preview_export_service.rs` | `ExportJobRegistry` owns status map, cancel token, and `thread::spawn`. | lines 217-330 | Submit export job to scheduler; keep export status mapping and `media_runtime::run_export_job` executor. | Fail `ExportJobRegistry|thread::spawn\(move \|\|.*run_export_thread` unless replaced by scheduler registry. |
| `crates/bindings_node/src/audio_service.rs` | Native audio refill thread and poll loop. | lines 994-1061 | Refill work becomes realtime audio scheduler jobs with underrun/queue telemetry. | Fail `audio-preview-refill|AUDIO_PREVIEW_REFILL_POLL_INTERVAL|run_audio_refill_loop` outside scheduler adapter. |
| `crates/bindings_node/src/audio_service.rs` | Binding discovers FFmpeg runtime and runs decode directly. | lines 1154-1210 | Decode windows run through `Decode`/`Audio` resource classes. | Fail `DesktopFfmpegExecutor::with_timeout|executor.run\(&runtime.ffmpeg.path` in binding audio service. |
| `crates/bindings_node/src/artifact_store_service.rs` | Thumbnail refresh discovers runtime, loops materials, and calls generation inline. | lines 445-520 | Enqueue artifact jobs and return status; scheduler controls concurrency/cancel/stale commit. | Fail direct `generate_thumbnail_artifact` from binding refresh path. |
| `crates/bindings_node/src/artifact_store_service.rs` | Desktop thumbnail generator runs FFmpeg under binding-owned refresh. | lines 611-690 | Keep generator logic but execute through artifact scheduler adapter. | Fail `executor.run(&self.runtime.ffmpeg.path` outside scheduler adapter. |
| `crates/bindings_node/src/material_service.rs` | Import calls `probe_material_metadata` before draft mutation. | lines 177-190 | Media probe enters scheduler; commit only if request/session revision remains current. | Fail direct `probe_material_metadata` from binding/project-session paths. |
| `crates/bindings_node/src/project_session_service.rs` | Import discovers runtime and creates `DesktopFfmpegExecutor` inline. | lines 1704-1723 | Route probe/IO through scheduler or scheduler-controlled wait. | Fail `discover_runtime_config|DesktopFfmpegExecutor::default` in import path except scheduler adapter. |
| `apps/desktop-electron/src/main/index.ts` | Test runtime capability mock can return ready FFmpeg/ffprobe. | lines 919-1012 | Keep only as test harness; product scheduler success/E2E cannot accept mock runtime as evidence. | Extend no-product-fallback and Phase 16 guard for scheduler success from `VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES`. |
| `apps/desktop-electron/src/main/realtimePreviewHost.ts` | Product readiness already gates fallback; mock native surface remains test-only. | lines 1038-1054 and 1198-1212 | Keep product-safe status; do not expose raw queue internals or mock backend as success. | Require product E2E evidence `renderGraphGpuComposited`; fail mock/backend success strings. |

## Suggested New Files

| File | Role | Data Flow | Primary Analog | Notes |
|---|---|---|---|---|
| `crates/task_runtime/Cargo.toml` | config | transform | `crates/media_runtime/Cargo.toml:1-20` | Add workspace member in root `Cargo.toml`. |
| `crates/task_runtime/src/lib.rs` | provider | event-driven | `crates/realtime_preview_runtime/src/lib.rs:1-67` | Explicit modules and re-exports. |
| `crates/task_runtime/src/job.rs` | model | event-driven | `crates/realtime_preview_runtime/src/request.rs:32-49` | Job envelope: domain, priority, resource, freshness, cancel, submitted_at, budgets, queue policy labels. |
| `crates/task_runtime/src/freshness.rs` | model | event-driven | `crates/realtime_preview_runtime/src/clock.rs:7-24,96-144` | One canonical `PlaybackGeneration`; no duplicate counters. |
| `crates/task_runtime/src/cancellation.rs` | utility | event-driven | `crates/media_runtime/src/job.rs:180-195` | Cloneable cancellation with telemetry hooks. |
| `crates/task_runtime/src/config.rs` | config | event-driven | `crates/realtime_preview_runtime/src/scheduler.rs:400-415` | Per-domain queue depth and resource capacities. |
| `crates/task_runtime/src/scheduler.rs` | service | event-driven | `crates/realtime_preview_runtime/src/scheduler.rs:424-512` | Admission/coalescing/rejection/completion gates. |
| `crates/task_runtime/src/telemetry.rs` | utility | event-driven | `crates/realtime_preview_runtime/src/telemetry.rs:10-29,121-195` | Scheduler-wide snapshots and histograms. |
| `crates/task_runtime/src/testing.rs` | testkit | event-driven | `crates/realtime_preview_runtime/src/scheduler.rs:900-928` | Fake clock/executor and saturation harness. |
| `crates/task_runtime/tests/scheduler_contracts.rs` | test | event-driven | `crates/audio_engine/src/session.rs:254-300` | Domains/priorities/resources/freshness/config. |
| `crates/task_runtime/tests/starvation.rs` | test | event-driven | `apps/desktop-electron/tests/product-preview-cadence.spec.ts:147-203` | Interactive preview/audio under export/artifact/probe pressure. |
| `crates/task_runtime/tests/scheduler_telemetry.rs` | test | event-driven | `crates/realtime_preview_runtime/src/telemetry.rs:206-278` | Queue latency, durations, cancel/stale, depth, saturation. |
| `crates/bindings_node/tests/scheduler_runtime.rs` | test | request-response | `crates/bindings_node/tests/project_session.rs:1489-1533` | Thin binding APIs, stale identity, no raw internals. |
| `scripts/phase16-source-guards.sh` | utility | transform | `scripts/no-product-fallback-guards.sh:1-25` | Block direct bypasses and mock/fallback success. |
| `apps/desktop-electron/tests/product-scheduler-stress.spec.ts` | test | request-response | `apps/desktop-electron/tests/product-preview-cadence.spec.ts:90-120,147-203,400-420` | Product stress proof for SCHED-02/SCHED-04. |

## Shared Patterns

### Authentication / IPC Sender Guard

**Source:** `apps/desktop-electron/src/main/index.ts:647-651`  
**Apply to:** New scheduler IPC routes in main/preload.

```typescript
function assertAllowedIpcSender(event: IpcMainInvokeEvent): void {
  const senderUrl = event.senderFrame.url;
  if (!isAllowedRendererUrl(senderUrl)) {
    throw new Error(`Rejected IPC from untrusted renderer: ${senderUrl}`);
  }
}
```

### Error Handling

**Source:** `crates/realtime_preview_runtime/src/scheduler.rs:644-660`  
**Apply to:** `task_runtime` scheduler/rejection errors.

Use a local enum with `Display` and `Error`; classify missing prerequisite, stale, canceled, rejected/backpressure, executor failure, and resource saturation. Do not return generic string errors from scheduler core.

### Validation

**Source:** `crates/audio_engine/src/session.rs:476-482`, `crates/artifact_store/src/jobs.rs:668-674`, `crates/bindings_node/src/lib.rs:200-243`  
**Apply to:** Config, job envelopes, binding requests.

Use explicit validation before accepting config/jobs. Binding routes parse typed JSON and return the existing `CommandErrorKind::InvalidPayload` envelope.

### Completion Freshness

**Source:** `crates/bindings_node/src/project_session_service.rs:644-660`; `crates/audio_engine/src/session.rs:254-300`  
**Apply to:** Preview/audio/artifact/probe/export validation completion handlers.

Every stale-sensitive completion checks `PlaybackGeneration` and/or expected project revision before mutating visible state, then increments stale rejection telemetry.

### Product Evidence

**Source:** `apps/desktop-electron/src/main/realtimePreviewHost.ts:1038-1054`; `apps/desktop-electron/tests/product-preview-cadence.spec.ts:147-203`  
**Apply to:** Product scheduler stress E2E.

Product success requires render graph GPU composited evidence, visible motion, telemetry, and no fallback/mock/backend token success.

## Risky Overlap / Conflict Notes For Parallel Executors

| Area | Conflict Risk | Coordination Rule |
|---|---|---|
| `PlaybackGeneration` ownership | Moving it from `realtime_preview_runtime` while preview/audio work is in progress can create duplicate generation types. | Choose one owner first; re-export/adapt during integration, then guard against duplicate `PlaybackGeneration` definitions. |
| `crates/bindings_node/src/realtime_preview_service.rs` | Preview scheduler work, native surface work, and telemetry work all touch worker maps and playback loop. | Land scheduler admission boundary before telemetry shape changes; avoid parallel edits to worker map removal. |
| `crates/bindings_node/src/audio_service.rs` | Audio refill thread, FFmpeg decode, waveform display, and device status share the same file. | Split work by line ownership: session/control APIs vs refill/decode adapter vs tests. |
| `crates/bindings_node/src/preview_export_service.rs` | Export status mapping and thread registry replacement overlap. | Preserve status/event response shape while replacing admission/cancel/status storage. |
| `crates/bindings_node/src/artifact_store_service.rs` | Artifact UI status and generation execution currently live together. | First introduce scheduler adapter returning existing `ArtifactGenerationTaskSummary`; then remove inline generator calls. |
| `project_session_service.rs` material import | Async probe semantics may alter existing import return shape. | Decide whether probe is queued-and-return or scheduler submit-and-wait before implementation starts; either way, admission must be scheduler-owned. |
| Source guards | Guards may fail during intermediate migration while old and new paths coexist. | Add allowlist comments only for scheduler adapters and remove them by phase closeout. |
| `media_runtime` helper threads | `run_export_job` and `run_process_with_timeout` legitimately spawn helper reader threads. | Guards must target binding-owned worker policy, not executor-internal stdout/stderr readers. |
| Product mocks | Test runtime mocks and mock surfaces exist for harnesses. | E2E success and Phase 16 gates must reject mock/fallback evidence as product success. |
| `package.json` scripts | Multiple phase scripts are dense and easy to conflict. | Append Phase 16 scripts near Phase 15.3 scripts; keep existing scripts intact. |

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `crates/task_runtime/src/scheduler.rs` full multi-domain scheduler | service | event-driven | Existing preview scheduler is single-domain; no shared multi-domain resource scheduler exists yet. |
| `apps/desktop-electron/tests/product-scheduler-stress.spec.ts` full workflow | test | request-response | Product cadence tests exist, but no export/artifact/probe concurrent stress workflow exists. |
| `apps/desktop-electron/src/preload/index.ts` scheduler IPC additions | route | request-response | Preload was not read for scheduler-specific analog; use existing explicit API exposure style during planning. |

## Metadata

**Analog search scope:** `crates/realtime_preview_runtime`, `crates/audio_engine`, `crates/artifact_store`, `crates/media_runtime`, `crates/bindings_node`, `apps/desktop-electron/src/main`, `apps/desktop-electron/tests`, `scripts`, root package/workspace config.  
**Files scanned:** 31 focused files plus phase artifacts.  
**Pattern extraction date:** 2026-06-23.

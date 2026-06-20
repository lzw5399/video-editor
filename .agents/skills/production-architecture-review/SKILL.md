---
name: production-architecture-review
description: Production-grade architecture design and implementation review standards for the Video Editor repo. Use automatically for architecture design, technical design, implementation planning, plan review, code review, realtime preview, rendering, media pipeline, Electron/Rust boundaries, subordinate feedback, or any request asking whether a technical direction is correct, production-ready, or should be reworked.
---

# Production Architecture Review

## Review Posture

Treat Video Editor as a pure self-owned, production video editing application, not a prototype, demo, wrapper, or fallback-driven product. The first product is Electron desktop, but draft semantics, preview behavior, render graph, media IO, and export must be correct enough to support future mobile and server surfaces.

Start every review from repo facts: inspect the current code path, tests, telemetry, and relevant architecture docs before judging the proposal. Do not accept broad claims without locating the concrete implementation points.

Default to this standard: every media/edit/render/preview chain must be the most correct production-grade chain the project can own. If the current chain is structurally wrong, say that directly and recommend replacing it. Do not advise incremental fixes on top of an invalid foundation unless they are explicitly temporary containment with a removal path.

For architecture design or implementation planning, use the same bar before proposing a plan: define the production target chain first, then design toward it. Do not design around a known-wrong legacy boundary just because it is already present.

## Required Review Output

For architecture or implementation reviews, answer in this order:

1. **Decision**: `confirmed`, `partially correct`, or `wrong direction`.
2. **Current chain**: what the code actually does today, with file/line references when possible.
3. **Production target**: what the chain should be for a self-owned editor.
4. **Gap**: the precise technical mismatch and why repeated patching will or will not work.
5. **Required action**: narrow fix, staged refactor, or destructive redesign.
6. **Verification gates**: tests, telemetry thresholds, and product evidence that would fail the known bad state.

Keep findings concrete. Prefer "this synchronous N-API call decodes and presents on the Electron main cadence" over "preview is slow".

## Non-Negotiable Architecture Checks

- UI emits commands; Rust owns draft, timeline, playback, and render semantics. UI code must not construct FFmpeg commands or decide editing semantics.
- `.veproj/project.json` is canonical. Render graphs, FFmpeg scripts, thumbnails, waveforms, proxies, and caches are derived artifacts.
- Persisted time and timeline math use integer microseconds, frame indices, or rational frame rates. Flag naked floating-point time in semantics.
- Render Graph separates editing semantics from FFmpeg. FFmpeg Runtime executes compiled jobs and reports progress; it must not decide edit behavior.
- Product preview must be a real compositor path. Do not count artifact fallback, mock surfaces, CPU readback diagnostics, or screenshot hash changes as proof of production realtime preview.
- Native/GPU resource lifetimes must be explicit: bounded in-flight queues, fence/completion-driven release, deterministic cancellation, and backpressure. Never rely on guessed delays.

## Realtime Preview Standard

For preview reviews, compare the implementation against this target:

- Playback cadence is driven by a Rust-owned scheduler/service, not by Electron repeatedly calling a blocking "present next frame" API.
- Electron IPC for playback is control/telemetry oriented. Main-process calls must not synchronously perform decode, graph build, texture import, GPU encode, surface present, and wait in one frame-budget-sensitive path.
- GPU submit/present returns without per-frame `Wait`; native texture leases are released only after GPU submission completion.
- Decode and texture interop are pipelined with bounded queues and cancellation. Reuse expensive platform resources such as texture caches where safe.
- Backpressure is bounded and visible in telemetry. A full queue may drop or skip obsolete frames, but must not silently stretch playback cadence.
- Tests must fail the known bad state. For 30fps 3s playback, accepting about 20-25 frames or p50 presentation calls around 100ms is not a production gate.

## Destructive Refactor Rule

When the chain is wrong at its boundary or ownership level, explicitly recommend a destructive refactor. Use phrases like:

- "Do not keep patching this path; replace the frame pump boundary."
- "This is a containment fix only; it should be deleted after the new scheduler lands."
- "Passing tests are not meaningful because the threshold accepts the known bad behavior."

Only recommend local patches when the architecture is sound and the bug is local.

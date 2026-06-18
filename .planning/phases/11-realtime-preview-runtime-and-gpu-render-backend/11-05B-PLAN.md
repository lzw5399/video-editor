---
phase: "11-realtime-preview-runtime-and-gpu-render-backend"
plan: "05B"
type: execute
wave: 6
depends_on:
  - "11-05"
  - "11-04B"
files_modified:
  - "apps/desktop-electron/src/renderer/viewModel.ts"
  - "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"
  - "apps/desktop-electron/tests/workspace.spec.ts"
autonomous: true
requirements:
  - RTPREV-03
  - RTPREV-05
user_setup: []
must_haves:
  truths:
    - "Desktop UI displays realtime backend, latency, frame pacing, stale rejection, cache, and fallback diagnostics returned by Rust/main."
    - "Supported preview responses do not show FFmpeg as the active realtime backend."
    - "Fallback artifact display appears only when Rust reports a fallback decision."
    - "Renderer display code formats telemetry only and does not own fallback ladder, cache key, FFmpeg, render graph, or support classification logic."
  artifacts:
    - path: "apps/desktop-electron/src/renderer/viewModel.ts"
      provides: "UI display model for realtime telemetry and fallback diagnostics"
    - path: "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"
      provides: "Chinese telemetry/fallback status rendering"
    - path: "apps/desktop-electron/tests/workspace.spec.ts"
      provides: "Playwright telemetry and fallback display coverage"
  key_links:
    - from: "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"
      to: "apps/desktop-electron/src/renderer/viewModel.ts"
      via: "binding/main telemetry mapped into display labels"
      pattern: "realtimePreviewTelemetry"
    - from: "apps/desktop-electron/tests/workspace.spec.ts"
      to: "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"
      via: "mocked Rust/main responses drive UI display"
      pattern: "fallback"
---

<objective>
Display realtime preview telemetry and fallback diagnostics in the desktop preview UI without moving fallback decisions into renderer code.

Purpose: make RTPREV-05 visible to users by formatting Rust-owned backend/fallback state in the workspace, while preserving the renderer UI-only boundary.
Output: renderer display model fields, PreviewMonitor telemetry/fallback UI, and Playwright coverage for supported and fallback responses.
</objective>

<execution_context>
@/Users/zhiwen/.codex/get-shit-done/workflows/execute-plan.md
@/Users/zhiwen/.codex/get-shit-done/templates/summary.md
</execution_context>

<context>
@AGENTS.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/STATE.md
@.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-CONTEXT.md
@.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-RESEARCH.md
@.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-DESIGN.md
@.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-04B-SUMMARY.md
@.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-05-SUMMARY.md
@apps/desktop-electron/src/renderer/viewModel.ts
@apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx
@apps/desktop-electron/tests/workspace.spec.ts
</context>

## Artifacts this plan produces

- desktop realtime telemetry display model
- Chinese backend/fallback labels
- first-frame latency display
- seek latency display
- frame pacing/drop/repeat counters
- stale rejection count display
- fallback reason/count display
- fallback artifact display state
- Playwright telemetry/fallback UI tests

<tasks>

<task type="auto" tdd="true">
  <name>Task 11-05B-01: Add display model fields for realtime telemetry and fallback diagnostics</name>
  <files>apps/desktop-electron/src/renderer/viewModel.ts</files>
  <read_first>
    - `apps/desktop-electron/src/renderer/viewModel.ts`
    - `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-05-SUMMARY.md`
    - `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-DESIGN.md`
  </read_first>
  <action>Extend renderer display models with fields for realtime backend used, first-frame latency, seek latency, frame pacing/drop/repeat counts, stale rejection count, fallback reason, fallback count, cache hit count, and fallback artifact display state. The model may format and label values; it must not decide the fallback ladder, build cache keys, construct FFmpeg commands, construct render graphs, or infer support from draft contents.</action>
  <acceptance_criteria>
    Display model types can represent supported realtime, offscreen/mock realtime, cache-hit fallback, FFmpeg artifact fallback, and unsupported/degraded diagnostics from Rust responses without adding renderer-owned support classification.
  </acceptance_criteria>
  <verify>
    <automated>pnpm --filter @video-editor/desktop build</automated>
  </verify>
  <done>Task complete when renderer display types can carry all backend/fallback telemetry emitted by 11-05.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 11-05B-02: Render telemetry/fallback display and Playwright coverage</name>
  <files>apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx, apps/desktop-electron/tests/workspace.spec.ts</files>
  <read_first>
    - `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx`
    - `apps/desktop-electron/tests/workspace.spec.ts`
    - `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-04B-SUMMARY.md`
    - `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-05-SUMMARY.md`
  </read_first>
  <action>Render Chinese labels for realtime backend, first-frame latency, seek latency, frame pacing/drop/repeat counts, stale rejection count, fallback reason, fallback count, cache hits, and fallback artifact state. Use mocked main/binding responses in Playwright to verify supported preview responses do not show FFmpeg as active backend, fallback artifact display appears only when Rust reports fallback, and telemetry values remain display-only data.</action>
  <acceptance_criteria>
    Playwright tests verify telemetry and fallback diagnostics are displayed from mocked binding/main responses, supported preview responses do not show FFmpeg as active backend, fallback artifact display appears only when Rust reports fallback, and renderer source contains no semantic fallback decision logic.
  </acceptance_criteria>
  <verify>
    <automated>pnpm --filter @video-editor/desktop test:workspace -g "实时预览|fallback|telemetry"</automated>
    <automated>pnpm --filter @video-editor/desktop build</automated>
  </verify>
  <done>Task complete when desktop UI shows realtime telemetry/fallback state while remaining a display-only consumer.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| binding/main -> renderer display | Telemetry and fallback diagnostics cross into UI display models. |
| renderer display -> user | User sees backend/fallback status that must not misrepresent supported realtime behavior. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-11-19 | Repudiation | telemetry display | mitigate | Render backend, fallback reason, target state, and counters from Rust/main response fields. |
| T-11-20 | Tampering | renderer fallback UI | mitigate | Tests and later guards reject renderer-owned fallback ladder or support inference logic. |
| T-11-21 | Information disclosure | diagnostics display | accept | Diagnostics are local preview/runtime status with no secrets or native handles displayed. |
| T-11-SC | Tampering | package installs | mitigate | This plan adds no external package installs. |
</threat_model>

<verification>
<automated>pnpm --filter @video-editor/desktop test:workspace -g "实时预览|fallback|telemetry"</automated>
<automated>pnpm --filter @video-editor/desktop build</automated>
</verification>

<source_audit>
GOAL | Phase 11 | realtime preview telemetry and fallback state are visible in desktop UI | 11-05B | COVERED
REQ | RTPREV-03 | UI distinguishes supported realtime preview from FFmpeg fallback artifacts | 11-05B | COVERED
REQ | RTPREV-05 | latency, pacing, stale, fallback, and cache telemetry are displayed | 11-05B | COVERED
CONTEXT | CTX-RendererUIOnly | renderer formats telemetry only and does not own fallback/cache/render semantics | 11-05B | COVERED
RESEARCH | Fallback Ladder | fallback decisions are Rust-owned and displayed as diagnostics | 11-05B | COVERED
</source_audit>

<success_criteria>
The desktop preview UI displays backend, telemetry, and fallback diagnostics from Rust/main responses, and tests prove the renderer remains a formatting/display layer rather than a fallback decision owner.
</success_criteria>

<output>
Create `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-05B-SUMMARY.md` when done.
</output>

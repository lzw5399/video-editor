---
phase: "11-realtime-preview-runtime-and-gpu-render-backend"
plan: "04B"
type: execute
wave: 5
depends_on:
  - "11-04"
files_modified:
  - "apps/desktop-electron/src/main/nativeBinding.ts"
  - "apps/desktop-electron/src/main/realtimePreviewHost.ts"
  - "apps/desktop-electron/src/main/index.ts"
  - "apps/desktop-electron/src/preload/index.ts"
  - "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"
  - "apps/desktop-electron/src/renderer/workspace/preview-inspector.css"
  - "apps/desktop-electron/tests/workspace.spec.ts"
autonomous: true
requirements:
  - RTPREV-02
  - RTPREV-03
  - RTPREV-05
user_setup: []
must_haves:
  truths:
    - "Electron main acquires native window handles and coordinates preview surface session lifecycle through the Node-API binding from 11-04."
    - "Preload exposes a narrow preview host bridge for rectangle updates and telemetry requests only."
    - "Renderer reserves and measures a stable native preview host rectangle without constructing render graphs, FFmpeg commands, GPU command lists, cache keys, or fallback decisions."
    - "Playwright proves the host rectangle is nonzero, positioned correctly, and reports fallback/display state from mocked Rust responses."
  artifacts:
    - path: "apps/desktop-electron/src/main/realtimePreviewHost.ts"
      provides: "main-process native handle and bounds coordinator"
    - path: "apps/desktop-electron/src/preload/index.ts"
      provides: "narrow realtime preview host bridge"
    - path: "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"
      provides: "UI-only native preview host rectangle"
    - path: "apps/desktop-electron/tests/workspace.spec.ts"
      provides: "Playwright native preview rect smoke coverage"
  key_links:
    - from: "apps/desktop-electron/src/main/index.ts"
      to: "apps/desktop-electron/src/main/realtimePreviewHost.ts"
      via: "window lifecycle creates, updates, and closes runtime surface host"
      pattern: "createRealtimePreviewHost"
    - from: "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"
      to: "apps/desktop-electron/src/preload/index.ts"
      via: "rect/scale updates only"
      pattern: "preview-native-host"
---

<objective>
Bridge the native preview surface into the Electron shell while keeping the renderer UI-only.

Purpose: make the Rust-owned preview surface visible in the desktop workspace by adding main-process handle coordination, a narrow preload API, a stable renderer host rectangle, and Playwright smoke coverage.
Output: Electron realtime preview host service, preload bridge, renderer host rectangle, CSS, and workspace tests for layout and mocked fallback display.
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
@.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-04-SUMMARY.md
@apps/desktop-electron/src/main/index.ts
@apps/desktop-electron/src/main/nativeBinding.ts
@apps/desktop-electron/src/preload/index.ts
@apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx
@apps/desktop-electron/tests/workspace.spec.ts
</context>

## Artifacts this plan produces

- Electron `realtimePreviewHost.ts`
- main-process native window handle coordination
- preload realtime preview host bridge
- `.preview-native-host` UI-only rectangle
- preview host bounds observer
- mocked attach-failure fallback display
- Playwright nonzero rect smoke tests

<tasks>

<task type="auto" tdd="true">
  <name>Task 11-04B-01: Add Electron main/preload native preview host bridge</name>
  <files>apps/desktop-electron/src/main/nativeBinding.ts, apps/desktop-electron/src/main/realtimePreviewHost.ts, apps/desktop-electron/src/main/index.ts, apps/desktop-electron/src/preload/index.ts</files>
  <read_first>
    - `apps/desktop-electron/src/main/index.ts`
    - `apps/desktop-electron/src/main/nativeBinding.ts`
    - `apps/desktop-electron/src/preload/index.ts`
    - `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-04-SUMMARY.md`
  </read_first>
  <action>Add a main-process `realtimePreviewHost.ts` that creates a runtime session after `BrowserWindow` readiness, obtains `BrowserWindow.getNativeWindowHandle()` only in main, attaches/detaches the Rust surface, forwards renderer rect/scale updates to bounds APIs, and closes the session before window close. Update `nativeBinding.ts` to expose the 11-04 binding functions and update preload with a narrow platform bridge for rect updates and telemetry requests. Main and preload may coordinate handles, bounds, and telemetry; they must not build render graphs, construct FFmpeg commands, decide fallback routing, own cache keys, or evaluate timeline semantics.</action>
  <acceptance_criteria>
    Main process owns native handle acquisition; preload exposes rect/telemetry APIs only; session close runs on window close; mocked attach failures return fallback diagnostics instead of renderer-owned composition decisions.
  </acceptance_criteria>
  <verify>
    <automated>pnpm --filter @video-editor/desktop build</automated>
  </verify>
  <done>Task complete when Electron main/preload routes native preview hosting through the binding from 11-04 without leaking semantic preview ownership to TypeScript.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 11-04B-02: Reserve renderer host rectangle and prove layout smoke coverage</name>
  <files>apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx, apps/desktop-electron/src/renderer/workspace/preview-inspector.css, apps/desktop-electron/tests/workspace.spec.ts</files>
  <read_first>
    - `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx`
    - `apps/desktop-electron/src/renderer/workspace/preview-inspector.css`
    - `apps/desktop-electron/tests/workspace.spec.ts`
    - `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-DESIGN.md`
  </read_first>
  <action>Update renderer `PreviewMonitor` to reserve a stable `.preview-native-host` rectangle, observe its bounds and scale, and send integer coordinates plus scale millis through preload. Display only the host status and Chinese fallback/telemetry labels supplied by main. Renderer code may measure DOM geometry and render labels only; it must not import `wgpu`, WebGPU, render graph builders, FFmpeg selection, cache keys, native handles, or fallback ladder logic.</action>
  <acceptance_criteria>
    Playwright tests prove the preview host rect is nonzero at 1280x800 and 1120x720, does not overlap timeline/inspector, rect updates reach the mocked main host as integer coordinates/scale millis, telemetry text is visible after a mocked first frame, and fallback display appears when native surface attach fails.
  </acceptance_criteria>
  <verify>
    <automated>pnpm --filter @video-editor/desktop test:workspace -g "实时预览|native preview|五大区域"</automated>
    <automated>pnpm --filter @video-editor/desktop build</automated>
  </verify>
  <done>Task complete when the desktop workspace reserves and verifies a native preview host rectangle while renderer code remains UI-only.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| renderer -> preload/main | DOM rect and user actions cross from sandboxed renderer into trusted main process. |
| main -> native binding | Main passes native window handles and surface bounds to Rust. |
| binding telemetry -> renderer | Runtime status and fallback display values cross into UI labels. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-11-13 | Elevation of privilege | renderer preview bridge | mitigate | Preload exposes rect/telemetry APIs only; main acquires native handles and rejects untrusted sender URLs. |
| T-11-14 | Denial of service | resize/update bridge | mitigate | Coalesce rect updates where existing workspace patterns support it and validate nonzero bounds before calling native bindings. |
| T-11-15 | Information disclosure | native handles | mitigate | Renderer never receives `HWND`, `NSView`, GPU device, surface, or command encoder handles. |
| T-11-SC | Tampering | package installs | mitigate | This plan adds no external package installs. |
</threat_model>

<verification>
<automated>pnpm --filter @video-editor/desktop test:workspace -g "实时预览|native preview|五大区域"</automated>
<automated>pnpm --filter @video-editor/desktop build</automated>
</verification>

<source_audit>
GOAL | Phase 11 | native realtime preview surface visible in the Electron desktop shell | 11-04B | COVERED
REQ | RTPREV-02 | Windows/macOS native surface host is exercised through desktop shell layout | 11-04B | COVERED
REQ | RTPREV-03 | renderer sends rect updates only and does not invoke FFmpeg per frame | 11-04B | COVERED
REQ | RTPREV-05 | telemetry/fallback status crosses from Rust/main into UI display | 11-04B | COVERED
CONTEXT | CTX-DesktopTarget | Windows and macOS desktop first | 11-04B | COVERED
CONTEXT | CTX-RendererBoundary | renderer UI only, no GPU/FFmpeg/render graph/cache semantics | 11-04B | COVERED
RESEARCH | Electron Embedding Recommendation | native child window/view path integrated through main process | 11-04B | COVERED
</source_audit>

<success_criteria>
Electron main/preload bridges the Rust native preview surface into the workspace, and Playwright proves a stable UI-only host rectangle plus mocked telemetry/fallback display without renderer-owned preview semantics.
</success_criteria>

<output>
Create `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-04B-SUMMARY.md` when done.
</output>

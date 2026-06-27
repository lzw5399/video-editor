---
phase: 11-realtime-preview-runtime-and-gpu-render-backend
plan: 04B
subsystem: desktop-native-preview
tags: [electron, preload, native-preview, playwright, realtime-preview]

requires:
  - phase: 11-realtime-preview-runtime-and-gpu-render-backend
    provides: Native surface contracts and thin Node-API realtime preview bindings from Plan 11-04
provides:
  - Electron main-process realtime preview host service that owns native window handle acquisition
  - Narrow preload bridge for preview host rectangle updates and telemetry requests
  - Renderer-reserved `.preview-native-host` rectangle with Chinese status, telemetry, and fallback labels
  - Playwright smoke coverage for nonzero host geometry, integer bounds delivery, and mocked fallback display
affects: [phase-11, phase-12-media-io, phase-17-bindings, desktop-preview, realtime-preview-runtime]

tech-stack:
  added: []
  patterns:
    - Electron main acquires native parent handles and coordinates Rust preview surface session lifecycle
    - Preload exposes only `updateHostRect` and `getTelemetry` for realtime preview hosting
    - Renderer measures DOM geometry and displays main-provided labels without owning render or fallback semantics

key-files:
  created:
    - apps/desktop-electron/src/main/realtimePreviewHost.ts
  modified:
    - apps/desktop-electron/src/main/nativeBinding.ts
    - apps/desktop-electron/src/main/index.ts
    - apps/desktop-electron/src/preload/index.ts
    - apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx
    - apps/desktop-electron/src/renderer/workspace/preview-inspector.css
    - apps/desktop-electron/tests/workspace.spec.ts
    - crates/bindings_node/src/realtime_preview_service.rs

key-decisions:
  - "Realtime preview host IPC is window-scoped in Electron main; renderer sends only integer rect and scale millis."
  - "Renderer displays realtime preview status/telemetry/fallback labels returned by main and never receives native handles, GPU objects, surface internals, command encoders, or cache keys."
  - "The Node-API JSON handle payload accepts integral JS number values so Electron native handles can cross from main to Rust validation."

patterns-established:
  - "Use `registerRealtimePreviewHost(window, assertAllowedIpcSender)` during BrowserWindow creation."
  - "Use `.preview-native-host` as the measured UI-only reservation for the Rust-owned native preview surface."
  - "Use Playwright main-process test globals only under `VIDEO_EDITOR_TEST_RECORD_COMMANDS=1` to prove host lifecycle without exposing production debug UI."

requirements-completed: [RTPREV-02, RTPREV-03, RTPREV-05]

duration: 9min
completed: 2026-06-18
---

# Phase 11 Plan 04B: Electron Native Preview Host Summary

**Electron main/preload bridge and renderer-reserved native preview host rectangle for Rust-owned realtime preview surfaces**

## Performance

- **Duration:** 9 min
- **Started:** 2026-06-18T17:03:44Z
- **Completed:** 2026-06-18T17:12:55Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- Added an Electron main-process realtime preview host service that creates/closes binding sessions, acquires `BrowserWindow.getNativeWindowHandle()` only in main, attaches/detaches surfaces, updates bounds, and queries telemetry.
- Added a narrow preload API exposing only `updateHostRect` and `getTelemetry`.
- Added a renderer `.preview-native-host` reservation that measures DOM geometry, sends rounded viewport rects plus scale millis, and displays only Chinese status/telemetry/fallback labels supplied by main.
- Added Playwright coverage for the native preview host bridge, required 1280x800 and 1120x720 geometry, integer bounds delivery, mocked first-frame telemetry, mocked attach-failure fallback display, and the existing five-region layout gate.

## Task Commits

1. **Task 11-04B-01 RED:** `c4c59f2` test: add failing native preview host bridge test.
2. **Task 11-04B-01 GREEN:** `6df7bf4` feat: add native preview host bridge.
3. **Task 11-04B-02 RED:** `c2f4516` test: add failing native preview rectangle smoke tests.
4. **Task 11-04B-02 GREEN:** `1524a53` feat: reserve native preview host rectangle.

**Plan metadata:** pending final docs commit.

## Files Created/Modified

- `apps/desktop-electron/src/main/realtimePreviewHost.ts` - Window-scoped realtime preview host lifecycle, native parent handle acquisition, bounds validation, telemetry query, close cleanup, and test-only lifecycle recording.
- `apps/desktop-electron/src/main/nativeBinding.ts` - TypeScript wrappers for the Plan 11-04 realtime preview binding entrypoints.
- `apps/desktop-electron/src/main/index.ts` - BrowserWindow registration for the realtime preview host service.
- `apps/desktop-electron/src/preload/index.ts` - Narrow realtime preview host bridge for rect update and telemetry request only.
- `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` - UI-only native preview host reservation, geometry observer, integer rect forwarding, and Chinese telemetry/fallback display.
- `apps/desktop-electron/src/renderer/workspace/preview-inspector.css` - Stable host rectangle and compact status/fallback overlay styling.
- `apps/desktop-electron/tests/workspace.spec.ts` - Playwright RED/GREEN smoke coverage for host bridge, host geometry, telemetry, fallback, and five-region layout.
- `crates/bindings_node/src/realtime_preview_service.rs` - Auto-fix for integral JS-number parent-handle deserialization at the Node-API JSON boundary.

## Decisions Made

- Main owns all native handle acquisition and native surface binding calls; renderer never receives raw parent handles or child handles.
- Preload stays narrow: no render graph, FFmpeg, fallback routing, cache key, GPU object, or timeline semantics API was added.
- Renderer only measures the reserved host element and displays labels returned from main; it does not decide fallback routing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Accepted large integral JS native handle numbers in the Rust binding**
- **Found during:** Task 11-04B-01 (main/preload native preview host bridge)
- **Issue:** Electron native handles passed through the JSON-shaped NAPI payload could arrive in Rust as floating-point JSON numbers when larger than V8 small integers, causing `parentHandle` deserialization to reject an otherwise integral native handle.
- **Fix:** Added a custom deserializer for `RealtimePreviewSurfaceBindingDescriptor.parent_handle` that accepts unsigned integers, integral JS number values, and numeric strings while still rejecting non-integer values.
- **Files modified:** `crates/bindings_node/src/realtime_preview_service.rs`
- **Verification:** `cargo test -p bindings_node surface_parent_handle_accepts_integral_js_number_values -- --nocapture`; focused Playwright bridge test passed.
- **Committed in:** `6df7bf4`

---

**Total deviations:** 1 auto-fixed (Rule 1).
**Impact on plan:** The fix was required for the planned Electron main -> native binding bridge to accept real native parent handles. No scope expansion or package changes.

## Known Stubs

None. Existing unrelated placeholder/deferred UI copy in older workspace tests and components was not introduced by this plan.

## Threat Flags

None - the new renderer -> preload/main and main -> native binding trust-boundary surfaces were covered by the plan threat model.

## Issues Encountered

- The first Task 11-04B-01 green run exposed the native handle JSON deserialization bug documented above.
- The initial fallback smoke assertion expected a successful bounds-update call even when attach was intentionally mocked to fail; the test was corrected so fallback requires DOM geometry and the main-provided diagnostic, while the success path still verifies integer bounds delivery.

## Verification

- `pnpm --filter @video-editor/desktop test:workspace -g "native preview host bridge"` - passed.
- `cargo test -p bindings_node surface_parent_handle_accepts_integral_js_number_values -- --nocapture` - passed.
- `pnpm --filter @video-editor/desktop test:workspace -g "实时预览 native preview"` - passed.
- `pnpm --filter @video-editor/desktop test:workspace -g "实时预览|native preview|五大区域"` - passed; 4 Playwright tests ran.
- `pnpm --filter @video-editor/desktop build` - passed.

## Boundary Notes

- Renderer does not import WebGPU/wgpu, build render graphs, construct FFmpeg commands, own cache keys, receive native handles, receive GPU objects, or decide fallback routing.
- Preload exposes only realtime preview host rectangle update and telemetry request methods.
- Main rejects untrusted IPC senders through the existing sender URL gate before servicing realtime preview host IPC.
- `reference/` remained untracked and untouched.

## User Setup Required

None.

## Next Phase Readiness

The desktop shell now has a tested UI reservation and IPC path for a Rust-owned native realtime preview surface. Later Phase 11/12 work can replace the mock/test host behavior with platform child HWND/NSView creation and media texture/frame interop without moving preview semantics into the renderer.

## Self-Check: PASSED

- Verified created file exists: `apps/desktop-electron/src/main/realtimePreviewHost.ts`.
- Verified task commits exist: `c4c59f2`, `6df7bf4`, `c2f4516`, `1524a53`.
- Verified required plan commands passed: workspace grep and desktop build.
- Verified `reference/` remains untracked and unstaged.

---
*Phase: 11-realtime-preview-runtime-and-gpu-render-backend*
*Completed: 2026-06-18*

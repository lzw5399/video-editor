---
phase: 11-realtime-preview-runtime-and-gpu-render-backend
plan: 04
subsystem: realtime-preview-runtime
tags: [rust, node-api, realtime-preview, native-surface, tdd]

requires:
  - phase: 11-realtime-preview-runtime-and-gpu-render-backend
    provides: Runtime session/clock contracts, graph preparation, frame providers, and offscreen GPU compositor from Plans 11-01 through 11-03B
provides:
  - Rust-owned native preview surface bounds, parent-handle descriptors, lifecycle validation, and typed diagnostics
  - Platform-gated Windows HWND and macOS NSView raw-window-handle adapters
  - Runtime session surface attach/update/detach hooks that preserve playback generation
  - Thin Node-API realtime preview session, surface, seek, frame, draft snapshot, and telemetry bindings
affects: [phase-11, phase-12-media-io, phase-17-bindings, realtime-preview-runtime, desktop-native-preview]

tech-stack:
  added: []
  patterns:
    - Native preview handles are accepted only as binding input and remain opaque outside Rust/native code
    - Node-API allocates opaque session IDs and delegates validation/generation/telemetry to realtime_preview_runtime
    - Surface lifecycle errors are typed in Rust and translated through the binding boundary without exposing child handles

key-files:
  created:
    - crates/realtime_preview_runtime/src/platform/mod.rs
    - crates/realtime_preview_runtime/src/platform/windows.rs
    - crates/realtime_preview_runtime/src/platform/macos.rs
    - crates/bindings_node/src/realtime_preview_service.rs
  modified:
    - Cargo.lock
    - crates/realtime_preview_runtime/src/gpu/mod.rs
    - crates/realtime_preview_runtime/src/gpu/surface.rs
    - crates/realtime_preview_runtime/src/lib.rs
    - crates/realtime_preview_runtime/src/session.rs
    - crates/bindings_node/Cargo.toml
    - crates/bindings_node/src/lib.rs

key-decisions:
  - "Preview surface contracts live in realtime_preview_runtime; bindings only allocate opaque session IDs and translate JSON."
  - "TypeScript receives generation, frame, and telemetry data, but no HWND, NSView, native child handle, GPU device, command encoder, surface internals, or cache keys."
  - "Surface attach/update/detach advances playback generation through the runtime session to keep stale preview work rejected."

patterns-established:
  - "Use PreviewSurfaceHost for Rust-owned attach/detach lifecycle validation."
  - "Use RealtimePreviewBindingRegistry for Node-API opaque session ID mapping."
  - "Use direct NAPI JSON entrypoints for preview sessions while keeping semantic work in realtime_preview_runtime."

requirements-completed: [RTPREV-01, RTPREV-02, RTPREV-03, RTPREV-05]

duration: 8min
completed: 2026-06-18
---

# Phase 11 Plan 04: Native Surface And Binding Summary

**Rust-owned native preview surface contracts with thin Node-API realtime preview session bindings**

## Performance

- **Duration:** 8 min
- **Started:** 2026-06-18T16:51:39Z
- **Completed:** 2026-06-18T16:59:45Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- Added `PreviewSurfaceBounds`, `PreviewSurfaceDescriptor`, `NativeParentWindowHandle`, `PreviewSurfaceHost`, and typed lifecycle diagnostics for invalid bounds, invalid scale, missing handles, double attach/detach, unavailable surfaces, and lost surfaces.
- Added platform-gated Windows HWND and macOS NSView adapters using `raw-window-handle` vocabulary.
- Added runtime session methods for surface attach, bounds update, detach, and telemetry query with playback generation preservation.
- Added `realtime_preview_service` in `bindings_node` with opaque binding session IDs and direct NAPI entrypoints for create/close session, attach/update/detach surface, update draft snapshot, seek, request frame, and telemetry.
- Added co-located TDD tests for native surface contracts and binding boundary behavior.

## Task Commits

1. **Task 11-04-01 RED:** `584f38c` test: add failing native surface contract tests.
2. **Task 11-04-01 GREEN:** `0170d8d` feat: implement native preview surface contracts.
3. **Task 11-04-02 RED:** `32d4f3f` test: add failing realtime preview binding tests.
4. **Task 11-04-02 GREEN:** `679755f` feat: expose realtime preview session bindings.

**Plan metadata:** pending final docs commit.

## Files Created/Modified

- `crates/realtime_preview_runtime/src/gpu/surface.rs` - Native surface bounds, descriptors, lifecycle host, typed diagnostics, and co-located `native_surface_contracts` tests.
- `crates/realtime_preview_runtime/src/platform/mod.rs` - Platform-gated native surface module boundary.
- `crates/realtime_preview_runtime/src/platform/windows.rs` - Windows HWND parent-handle adapter to `raw-window-handle` Win32 types.
- `crates/realtime_preview_runtime/src/platform/macos.rs` - macOS NSView parent-handle adapter to `raw-window-handle` AppKit types.
- `crates/realtime_preview_runtime/src/session.rs` - Surface attach/update/detach and telemetry session methods with generation advancement.
- `crates/realtime_preview_runtime/src/gpu/mod.rs` and `src/lib.rs` - Public module exposure for surface/platform contracts.
- `crates/bindings_node/src/realtime_preview_service.rs` - Thin registry, opaque session IDs, JSON-facing request/response structs, and `realtime_preview_bindings` tests.
- `crates/bindings_node/src/lib.rs` - Direct NAPI realtime preview entrypoints.
- `crates/bindings_node/Cargo.toml` and `Cargo.lock` - Workspace dependency on `realtime_preview_runtime`.

## Decisions Made

- Binding session IDs use `rtprev-session-<hex>` opaque strings and reject malformed IDs before runtime lookup.
- Native handles are one-way binding inputs; responses include only opaque session IDs, integer generations/times, frame status, backend enum, and telemetry counters.
- Binding tests target the Rust service directly, so they verify boundary behavior without requiring a JavaScript runtime or exposing binding internals.

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

None.

## Threat Flags

None - new native surface and binding trust-boundary surfaces were covered by the plan threat model.

## Issues Encountered

- `cargo fmt --all` reformatted two unrelated runtime test files; those self-authored formatting-only changes were reverted before commits.
- The first compile pass caught an import path mismatch for surface types; fixed by keeping runtime session imports pointed at `crate::gpu::surface`.
- The first binding compile pass caught a missing runtime-error conversion; fixed by explicitly mapping runtime errors into binding error kinds.
- GSD state helper commands partially updated `STATE.md` but removed current-position frontmatter fields; the close-out metadata was repaired narrowly before the final docs commit.

## Verification

- `cargo test -p realtime_preview_runtime native_surface_contracts -- --nocapture` - passed; 4 native surface contract tests ran.
- `cargo check -p realtime_preview_runtime --locked` - passed.
- `cargo test -p bindings_node realtime_preview_bindings -- --nocapture` - passed; 4 binding tests ran.
- `pnpm --filter @video-editor/desktop build:native` - passed; napi release build completed.

## User Setup Required

None.

## Next Phase Readiness

Plan 11-04B can build the Electron main/preload/renderer bridge against opaque session/surface APIs. Rust now owns native surface validation, lifecycle, generation updates, and telemetry; TypeScript can call the binding without receiving GPU or native child internals.

## Self-Check: PASSED

- Verified created files exist: `crates/realtime_preview_runtime/src/platform/mod.rs`, `windows.rs`, `macos.rs`, `crates/bindings_node/src/realtime_preview_service.rs`, and this summary.
- Verified task commits exist: `584f38c`, `0170d8d`, `32d4f3f`, `679755f`.
- Verified required verification commands passed.
- Verified `reference/` remains untracked and unstaged.

---
*Phase: 11-realtime-preview-runtime-and-gpu-render-backend*
*Completed: 2026-06-18*

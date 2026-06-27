---
phase: 01-foundation-and-golden-harness
plan: 03
subsystem: rust-runtime-boundaries
tags: [rust, cargo, ffmpeg, filesystem, preview, testkit]

requires:
  - phase: 01-01
    provides: Root Rust workspace metadata, locked Cargo resolution, and command entrypoints
provides:
  - Compile-safe service-boundary crate shells for runtime, desktop runtime, project store, preview service, and testkit
  - Trait ownership boundaries for FFmpeg execution, project filesystem access, and future preview rendering
  - Runtime guardrail documentation for pure crate isolation, FFmpeg distribution scope, and deferred hardware encoder work
affects: [phase-1-foundation, media-runtime, project-store, preview-service, testkit, packaging]

tech-stack:
  added: []
  patterns:
    - Service traits live in their consuming crates instead of a generic platform crate
    - Desktop backends are injected at app shell or service boundaries
    - FFmpeg distribution and HardwareEncoder implementation remain deferred

key-files:
  created:
    - crates/media_runtime/Cargo.toml
    - crates/media_runtime/src/lib.rs
    - crates/media_runtime_desktop/Cargo.toml
    - crates/media_runtime_desktop/src/lib.rs
    - crates/project_store/Cargo.toml
    - crates/project_store/src/lib.rs
    - crates/preview_service/Cargo.toml
    - crates/preview_service/src/lib.rs
    - crates/testkit/Cargo.toml
    - crates/testkit/src/lib.rs
    - docs/runtime-boundaries.md
  modified:
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "Placed runtime/platform traits at consuming service boundaries: media_runtime, project_store, and preview_service."
  - "Kept draft_model, draft_commands, and engine_core isolated from runtime/platform traits."
  - "Documented FFmpeg as local env/PATH discovery only for Phase 1, with no download, bundling, redistribution, or license review."
  - "Deferred HardwareEncoder to later preview/export pipeline work and did not create a Rust type for it."

patterns-established:
  - "Service-boundary crates compile independently and can be injected by desktop, future mobile, or future server shells."
  - "Phase 1 boundary shells document future responsibilities without implementing discovery, preview generation, or render smoke behavior early."

requirements-completed: [FOUND-01, FOUND-03, FOUND-04]

duration: 3 min
completed: 2026-06-17
---

# Phase 1 Plan 03: Service Boundary Crates And Runtime Guardrails Summary

**Compile-safe runtime, filesystem, preview, and testkit service boundaries with documented FFmpeg distribution and hardware-encoder deferrals.**

## Performance

- **Duration:** 3 min
- **Started:** 2026-06-16T21:34:52Z
- **Completed:** 2026-06-16T21:37:50Z
- **Tasks:** 2
- **Files modified:** 13

## Accomplishments

- Added `media_runtime`, `media_runtime_desktop`, `project_store`, `preview_service`, and `testkit` as real Cargo workspace members.
- Defined `media_runtime::FfmpegExecutor`, `media_runtime_desktop::DesktopFfmpegExecutor`, `project_store::PlatformFileSystem`, `project_store::StdPlatformFileSystem`, and `preview_service::PreviewRenderer`.
- Documented service-boundary ownership, pure semantic crate isolation, FFmpeg Phase 1 distribution limits, and the deferred `HardwareEncoder` scope.

## Task Commits

1. **Task 01-W1-03: Add service-boundary trait shells** - `1aa8efb` (chore)
2. **Task 01-W1-04: Document runtime and platform guardrails** - `d4f28e1` (docs)

## Files Created/Modified

- `Cargo.toml` - Added service-boundary crates to workspace membership and left only future `bindings_node` in planned members.
- `Cargo.lock` - Refreshed workspace lock metadata for the new local service crates.
- `crates/media_runtime/Cargo.toml` - Added compile-safe media runtime crate manifest.
- `crates/media_runtime/src/lib.rs` - Defines the `FfmpegExecutor` service-boundary trait.
- `crates/media_runtime_desktop/Cargo.toml` - Added desktop runtime crate manifest with a local `media_runtime` dependency.
- `crates/media_runtime_desktop/src/lib.rs` - Defines `DesktopFfmpegExecutor` as the desktop shell implementation.
- `crates/project_store/Cargo.toml` - Added project store crate manifest.
- `crates/project_store/src/lib.rs` - Defines `PlatformFileSystem` and `StdPlatformFileSystem`.
- `crates/preview_service/Cargo.toml` - Added preview service crate manifest.
- `crates/preview_service/src/lib.rs` - Defines boundary-only `PreviewRenderer`.
- `crates/testkit/Cargo.toml` - Added testkit crate manifest.
- `crates/testkit/src/lib.rs` - Adds a compile-safe shell marker for future fixture/golden/render smoke helpers.
- `docs/runtime-boundaries.md` - Documents runtime trait placement, pure crate isolation, FFmpeg distribution scope, desktop/project/preview runtime boundaries, and deferred `HardwareEncoder`.

## Verification

- `cargo fmt --all --check` - PASS
- `cargo check -p media_runtime -p media_runtime_desktop -p project_store -p preview_service -p testkit --locked` - PASS
- `test ! -d crates/platform` - PASS
- `grep -n "does not download\\|does not.*bundle\\|media_runtime::FfmpegExecutor\\|project_store::PlatformFileSystem\\|preview_service::PreviewRenderer\\|HardwareEncoder" docs/runtime-boundaries.md` - PASS
- `grep -R "HardwareEncoder" crates && exit 1 || true` - PASS
- `grep -R "FfmpegExecutor\\|PlatformFileSystem\\|PreviewRenderer" crates/draft_model crates/draft_commands crates/engine_core && exit 1 || true` - PASS

## Decisions Made

- Put service traits at the consuming service crates instead of creating a broad `platform` crate.
- Kept desktop-specific runtime behavior in `media_runtime_desktop` and documented Electron/iOS/Android/server backends as injected at app shell or service boundaries.
- Treated `PreviewRenderer` and `testkit` as boundary shells only; preview generation and render smoke helpers remain for later Phase 1 plans.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated root workspace membership and lockfile**
- **Found during:** Task 01-W1-03 (Add service-boundary trait shells)
- **Issue:** The task file list did not include root `Cargo.toml` or `Cargo.lock`, but the planned locked `cargo check -p media_runtime -p media_runtime_desktop -p project_store -p preview_service -p testkit --locked` cannot run until the new crates are workspace members and the lockfile reflects them.
- **Fix:** Added the service-boundary crates to root workspace membership, removed them from `workspace.metadata.video-editor.planned-members`, regenerated local lock metadata, and reran the locked check.
- **Files modified:** `Cargo.toml`, `Cargo.lock`
- **Verification:** `cargo check -p media_runtime -p media_runtime_desktop -p project_store -p preview_service -p testkit --locked`
- **Committed in:** `1aa8efb`

---

**Total deviations:** 1 auto-fixed (Rule 3: 1)
**Impact on plan:** Required for the planned Cargo verification gate. No runtime discovery, FFmpeg execution, preview generation, mobile backend, server backend, or hardware encoder implementation was added.

## Issues Encountered

- The first locked service-crate check failed because `Cargo.lock` did not yet include the new local workspace packages. Regenerating local lock metadata resolved it without adding external dependencies.

## Known Stubs

None. The boundary-only crates are intentional Phase 1 shells and do not block this plan's goal.

## Threat Flags

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for Plan 01-04 to add the Node-API binding crate against the Rust-owned command contract and service-boundary layout.

## Self-Check: PASSED

- Key files exist on disk.
- Task commits `1aa8efb` and `d4f28e1` exist in git history.
- Plan verification commands passed.
- Stub scan over touched files found no TODO/FIXME/placeholder or hardcoded empty UI data patterns.

---
*Phase: 01-foundation-and-golden-harness*
*Completed: 2026-06-17*

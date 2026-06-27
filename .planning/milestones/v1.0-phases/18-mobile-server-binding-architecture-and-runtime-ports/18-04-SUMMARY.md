---
phase: 18-mobile-server-binding-architecture-and-runtime-ports
plan: "04"
subsystem: c-abi-bindings
tags: [rust, c-abi, cbindgen, mobile-contracts, handles, ffi]
requires:
  - phase: "18-01"
    provides: "editor_runtime shared runtime/session/export/handle authority"
  - phase: "18-02"
    provides: "Phase 18 source guards, ABI drift script, and mobile smoke guard"
  - phase: "18-03"
    provides: "bindings_node delegated project-session/export semantics to editor_runtime"
provides:
  - "Portable bindings_c C ABI over editor_runtime runtime/session/handle operations"
  - "Stable repr(C) runtime, handle, status, texture descriptor, and buffer contracts"
  - "Pinned cbindgen 0.29.4 generated video_editor_runtime.h header"
  - "ABI and mobile handle smoke tests for owner/generation/device/release/cascade diagnostics"
affects: [phase-18, bindings-c, mobile-contracts, server-runtime, bindings-node]
tech-stack:
  added:
    - "bindings_c dependency on media_runtime for C texture metadata conversion"
    - "bindings_c serde/serde_json diagnostics for bounded C buffers"
    - "Project-local cbindgen 0.29.4 generated header"
  patterns:
    - "C ABI functions validate raw transport inputs and delegate semantic checks to editor_runtime"
    - "C callers receive stable repr(C) handles/statuses plus bounded JSON diagnostics"
    - "Header drift is protected by scripts/phase18-abi-drift.sh"
key-files:
  created:
    - crates/bindings_c/cbindgen.toml
    - crates/bindings_c/include/video_editor_runtime.h
    - crates/bindings_c/tests/abi_smoke.rs
    - crates/bindings_c/tests/mobile_contract_handles.rs
  modified:
    - Cargo.lock
    - crates/bindings_c/Cargo.toml
    - crates/bindings_c/src/lib.rs
    - crates/editor_runtime/src/handles.rs
key-decisions:
  - "bindings_c exposes explicit C ABI functions rather than a generic JSON command envelope."
  - "Handle owner/generation/device/release checks stay in editor_runtime::HandleRegistry; the C adapter only reconstructs opaque tokens for validation."
  - "Generated C headers are checked in and regenerated only through the pinned scripts/phase18-abi-drift.sh cbindgen 0.29.4 flow."
  - "editor_runtime gained a shared HandleRegistry::retain operation so C retain/release behavior is not implemented as adapter-local reference policy."
patterns-established:
  - "Task-level ABI smoke tests read the generated header and fail when exported symbols drift."
  - "Mobile smoke tests use the C ABI exactly as future JNI/Swift wrappers will: opaque handles, explicit release, and close-time leak diagnostics."
  - "C diagnostic buffers are caller-owned and bounded; insufficient capacity returns VE_STATUS_BUFFER_TOO_SMALL."
requirements-completed: [PLAT-01, PLAT-03, BIND-02, BIND-03, BIND-05]
duration: 19 min
completed: 2026-06-25
status: complete
---

# Phase 18 Plan 04: Portable C ABI Runtime Binding Summary

**Portable C ABI over editor_runtime with generated cbindgen header, stable status/handle contracts, and mobile-held handle lifecycle smoke tests.**

## Performance

- **Duration:** 19 min
- **Started:** 2026-06-25T01:46:47Z
- **Completed:** 2026-06-25T02:05:55Z
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments

- Added `bindings_c` runtime/session/handle exports over `editor_runtime`, including runtime create/close, project open, handle acquire/retain/release, texture resolve, bounded diagnostics, and buffer free APIs.
- Generated and committed `crates/bindings_c/include/video_editor_runtime.h` through the project-local pinned `cbindgen 0.29.4` drift script.
- Added ABI smoke tests for invalid pointers, invalid UTF-8, undersized buffers, fabricated handles, wrong owner, stale generation, double release, and no Node adapter dependency.
- Added mobile-oriented smoke tests for explicit release, retained handles, media/frame/texture/artifact tokens, texture device/metadata validation, and cascade-close leak diagnostics.

## Task Commits

Each task was committed atomically with RED and GREEN TDD gates:

1. **Task 1 RED: C ABI smoke tests** - `0464c67` (test)
2. **Task 1 GREEN: Portable C ABI runtime handles** - `3c815a2` (feat)
3. **Task 2 RED: Generated header ABI check** - `41ad1aa` (test)
4. **Task 2 GREEN: Pinned generated C header** - `ff3e970` (feat)
5. **Task 3 RED: Mobile handle contract smoke** - `ef49499` (test)
6. **Task 3 GREEN: Mobile cascade diagnostics** - `3df0981` (feat)

## Files Created/Modified

- `crates/bindings_c/src/lib.rs` - Implements the C ABI adapter with `#[repr(C)]` statuses, handles, texture descriptors, bounded diagnostics, panic boundaries, and explicit release functions over `editor_runtime`.
- `crates/bindings_c/Cargo.toml` - Adds `media_runtime`, `serde`, and `serde_json` dependencies plus test-only fixtures dependencies.
- `crates/bindings_c/cbindgen.toml` - Defines the generated C header surface for the ABI exports.
- `crates/bindings_c/include/video_editor_runtime.h` - Generated C header for mobile/server shells.
- `crates/bindings_c/tests/abi_smoke.rs` - Covers ABI runtime/project/handle behavior, invalid input errors, generated header symbol coverage, and no desktop adapter dependency.
- `crates/bindings_c/tests/mobile_contract_handles.rs` - Covers mobile-held handle release, wrong owner, stale generation, texture device/metadata mismatch, and cascade close diagnostics.
- `crates/editor_runtime/src/handles.rs` - Adds shared `HandleRegistry::retain` so retain/release semantics are not adapter-owned.
- `Cargo.lock` - Locks the new `bindings_c` dependency graph entries.

## Decisions Made

- Kept `bindings_c` independent from `bindings_node`; source guards and tests avoid even literal desktop-adapter references in the C crate.
- Used stable typed C structs/enums for status, runtime, handle, texture metadata, and buffers, while JSON remains bounded diagnostic text only.
- Treated the `cbindgen` install as script-owned tool bootstrap under `target/phase18-tools`, not a global tool or Node/NAPI package change.
- Added retain in `editor_runtime` instead of tracking C-side retain counts, preserving Rust-owned lifetime authority.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added shared handle retain support**
- **Found during:** Task 1 (C ABI retain/release implementation)
- **Issue:** The plan required `ve_handle_retain`, but `editor_runtime::HandleRegistry` only supported acquire/release/resolve. Implementing retain in `bindings_c` would have created adapter-owned lifetime policy.
- **Fix:** Added `HandleRegistry::retain` and changed release to decrement retained references before final explicit release.
- **Files modified:** `crates/editor_runtime/src/handles.rs`, `crates/bindings_c/src/lib.rs`, `crates/bindings_c/tests/abi_smoke.rs`, `crates/bindings_c/tests/mobile_contract_handles.rs`
- **Verification:** `cargo test -p bindings_c --test abi_smoke -- --nocapture`; `cargo test -p bindings_c --test mobile_contract_handles -- --nocapture`; `bash scripts/phase18-source-guards.sh --plan 04`
- **Committed in:** `3c815a2` and `3df0981`

---

**Total deviations:** 1 auto-fixed (1 Rule 2 missing critical functionality)
**Impact on plan:** The fix kept the ABI aligned with the planned ownership boundary. No adapter-local handle metadata or Node dependency was introduced.

## Issues Encountered

- Task 1's staged `bash scripts/phase18-source-guards.sh --plan 04` attempt failed before Task 2 because the guard requires `crates/bindings_c/cbindgen.toml` and `crates/bindings_c/include/video_editor_runtime.h`. The guard was rerun after Task 2 and during final verification, and passed both times.
- The first Task 3 RED run exposed a test helper shadowing bug before reaching the intended cascade diagnostic assertion. The test was corrected before committing the RED gate.
- `scripts/phase18-abi-drift.sh` installed audited `cbindgen 0.29.4` into `target/phase18-tools/cbindgen-0.29.4/install`; no `@napi-rs/cli` package was upgraded, reinstalled, or edited.
- Verification emitted the pre-existing `media_runtime_desktop` macOS `AVAsset::tracksWithMediaType` deprecation warning and existing unused helper warnings in `bindings_node`; these are out of scope for this C ABI plan.
- `requirements.mark-complete PLAT-01 PLAT-03 BIND-02 BIND-03 BIND-05` returned `not_found` because the current requirements file stores those IDs as narrative requirement rows rather than SDK-markable checklist entries. No manual requirements rewrite was made outside the SDK.

## Verification

- `cargo test -p bindings_c --test abi_smoke -- --nocapture` - passed, 4 tests.
- `cargo test -p bindings_c --test mobile_contract_handles -- --nocapture` - passed, 2 tests.
- `bash scripts/phase18-abi-drift.sh` - passed using project-local `cbindgen 0.29.4`.
- `bash scripts/phase18-source-guards.sh --plan 04` - passed.
- `bash scripts/phase18-mobile-contract-guards.sh --smoke-only` - passed.
- `cargo check --workspace --locked` - passed with the pre-existing warnings noted above.

## TDD Gate Compliance

- RED gate present for Task 1: `0464c67`.
- GREEN gate present for Task 1: `3c815a2`.
- RED gate present for Task 2: `41ad1aa`.
- GREEN gate present for Task 2: `ff3e970`.
- RED gate present for Task 3: `ef49499`.
- GREEN gate present for Task 3: `3df0981`.

## Known Stubs

None. Stub scan found no TODO/FIXME/placeholder or UI-flowing hardcoded empty data in the created/modified files.

## Threat Mitigations

- **T-18-13 / C ABI buffers:** Raw output buffers are checked for null pointers and capacity before writing bounded diagnostic JSON.
- **T-18-14 / Handle elevation:** C handles are reconstructed into `editor_runtime::HandleToken` values and validated through `HandleRegistry` owner/generation/device checks.
- **T-18-15 / FFI panic boundary:** Exported C functions use a panic boundary and return `VE_STATUS_PANIC` instead of unwinding across FFI.
- **T-18-16 / Header drift:** `scripts/phase18-abi-drift.sh` regenerates and diff-checks `video_editor_runtime.h`.
- **T-18-SC / Toolchain tampering:** Header generation uses exactly project-local `cbindgen 0.29.4` and rejects version mismatches.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 18-05 can build server runtime entrypoints over the same `editor_runtime` contracts. Plan 18-06 can add mobile contract docs and run the full Phase 18 source/mobile/aggregate gates using the C ABI header and smoke tests from this plan.

## Self-Check: PASSED

- Found key created/modified files on disk: `crates/bindings_c/cbindgen.toml`, `crates/bindings_c/include/video_editor_runtime.h`, `crates/bindings_c/tests/abi_smoke.rs`, `crates/bindings_c/tests/mobile_contract_handles.rs`, `crates/bindings_c/src/lib.rs`, and `crates/editor_runtime/src/handles.rs`.
- Confirmed all task commits exist in git history via commit-object checks: `0464c67`, `3c815a2`, `41ad1aa`, `ff3e970`, `ef49499`, and `3df0981`.

---
*Phase: 18-mobile-server-binding-architecture-and-runtime-ports*
*Completed: 2026-06-25*

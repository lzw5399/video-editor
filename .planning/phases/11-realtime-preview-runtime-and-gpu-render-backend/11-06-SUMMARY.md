---
phase: 11-realtime-preview-runtime-and-gpu-render-backend
plan: 06
subsystem: realtime-preview-runtime
tags: [rust, realtime-preview, text, parity, fallback, tdd]

requires:
  - phase: 11-realtime-preview-runtime-and-gpu-render-backend
    provides: Runtime capability classifier, GPU compositor subset, fallback ladder, and preview/export parity diagnostics from Plans 11-01 through 11-05B
provides:
  - Explicit GPU text preview outcome boundary
  - TextParityUnsupported fallback diagnostics when repository font parity is not proven
  - Golden realtime/export parity coverage for supported and divergent graphs
  - Conservative preview service default that routes text through fallback diagnostics
affects: [phase-11, phase-12-media-io, phase-18-effects-retiming, realtime-preview-runtime, preview-service]

tech-stack:
  added: []
  patterns:
    - Text GPU preview is opt-in only after repository font parity proof
    - Text fallback classification is owned by realtime_preview_runtime and preview_service, not the renderer
    - Realtime/export parity diagnostics are golden-tested as serialized contracts

key-files:
  created:
    - crates/realtime_preview_runtime/src/gpu/text.rs
    - crates/realtime_preview_runtime/tests/text_parity.rs
    - crates/testkit/tests/realtime_preview_parity.rs
  modified:
    - Cargo.lock
    - crates/testkit/Cargo.toml
    - crates/realtime_preview_runtime/src/capabilities.rs
    - crates/realtime_preview_runtime/src/gpu/mod.rs
    - crates/realtime_preview_runtime/tests/capability_matrix.rs
    - crates/realtime_preview_runtime/tests/parity_diagnostics.rs
    - crates/preview_service/src/realtime_backend.rs

key-decisions:
  - "GPU text parity is not claimed in Phase 11 because no repository font parity proof is wired; text preview is classified as unsupported with TextParityUnsupported fallback diagnostics."
  - "Realtime/export parity diagnostics include supported no-divergence and divergent text/effect golden cases."
  - "Preview service defaults to text fallback unless GPU text parity is explicitly enabled by a proven implementation."

patterns-established:
  - "Use gpu::text::classify_text_preview_outcome for the Rust-owned text preview boundary."
  - "Use TextParityUnsupported for unproven GPU text parity instead of degraded or silent support."

requirements-completed: [RTPREV-02, RTPREV-03, RTPREV-04]

duration: 5min
completed: 2026-06-18
---

# Phase 11 Plan 06: Text Preview Parity Boundary Summary

**Text preview now fails closed through TextParityUnsupported fallback diagnostics, with golden realtime/export parity tests preventing silent text drift**

## Performance

- **Duration:** 5 min
- **Started:** 2026-06-18T17:46:35Z
- **Completed:** 2026-06-18T17:51:55Z
- **Tasks:** 1
- **Files modified:** 10

## Accomplishments

- Added `gpu::text` as the runtime-owned text preview outcome boundary.
- Classified text as unsupported with `TextParityUnsupported` unless GPU text parity is explicitly proven with repository fonts.
- Changed default realtime preview and preview service text behavior to fail closed through fallback diagnostics.
- Added TDD RED/GREEN coverage for text parity and testkit realtime/export parity goldens.

## Task Commits

1. **Task 11-06-01 RED:** `e9d63c3` test: add failing text parity diagnostics tests.
2. **Task 11-06-01 GREEN:** `a6ab074` feat: enforce text parity fallback diagnostics.

**Plan metadata:** pending final docs commit.

## Files Created/Modified

- `crates/realtime_preview_runtime/src/gpu/text.rs` - Text preview outcome boundary and `TextParityUnsupported` diagnostic construction.
- `crates/realtime_preview_runtime/src/gpu/mod.rs` - Exports the text preview boundary.
- `crates/realtime_preview_runtime/src/capabilities.rs` - Defaults GPU text parity to false and delegates text diagnostics to the runtime text boundary.
- `crates/preview_service/src/realtime_backend.rs` - Defaults preview service text parity to false so text routes through the fallback ladder.
- `crates/realtime_preview_runtime/tests/text_parity.rs` - Text parity tests proving unsupported fallback diagnostics instead of silent support.
- `crates/testkit/tests/realtime_preview_parity.rs` - Golden tests for supported no-divergence and divergent text/effect realtime/export parity.
- `crates/realtime_preview_runtime/tests/capability_matrix.rs` - Updated text capability expectation to unsupported fallback.
- `crates/realtime_preview_runtime/tests/parity_diagnostics.rs` - Updated serialized parity snapshot for unsupported text fallback.
- `crates/testkit/Cargo.toml` and `Cargo.lock` - Added local test dependency on `realtime_preview_runtime` for testkit parity goldens.

## Decisions Made

GPU text parity is explicitly unsupported in this plan. The repo has approved `glyphon` in research, but this implementation does not prove text parity with repository fonts, so it does not enable GPU text rendering or approximate export output.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Defaulted preview service text parity to fallback**
- **Found during:** Task 11-06-01 GREEN
- **Issue:** The plan required unproven text parity to route through the 11-05 fallback diagnostics path, but `RealtimePreviewServiceConfig::new` defaulted `gpu_text_parity` to true.
- **Fix:** Changed the preview service default to false so text graphs receive `TextParityUnsupported` unless an explicit parity-proven path enables GPU text.
- **Files modified:** `crates/preview_service/src/realtime_backend.rs`
- **Verification:** `cargo test -p preview_service fallback_ladder -- --nocapture` passed.
- **Committed in:** `a6ab074`

---

**Total deviations:** 1 auto-fixed Rule 2 issue.
**Impact on plan:** The deviation was necessary to satisfy the critical no-silent-text-approximation constraint and did not expand renderer ownership.

## Known Stubs

None.

## Threat Flags

None - the text/render graph to realtime text path and fallback ladder boundary were covered by the plan threat model.

## Issues Encountered

- RED testkit draft setup initially used a non-existent `Draft.id` field; corrected the test to use the existing draft constructor state before committing the RED gate.
- `cargo fmt --all` touched unrelated files; those specific formatting-only paths were reverted before committing.

## Verification

- `cargo test -p realtime_preview_runtime text_parity -- --nocapture` - passed; text fallback and existing filtered text capability tests ran.
- `cargo test -p testkit realtime_preview_parity -- --nocapture` - passed; 2 realtime/export parity golden tests ran.
- `cargo test -p realtime_preview_runtime parity_diagnostics -- --nocapture` - passed; 3 existing parity diagnostic tests ran.
- `cargo test -p preview_service fallback_ladder -- --nocapture` - passed; 2 fallback ladder tests ran.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 11-07 can rely on a conservative text preview contract: supported realtime graphs stay clean, divergent text/effect graphs emit golden-tested diagnostics, and text will not be GPU-rendered until parity is proven by repository fonts and tests.

## Self-Check: PASSED

- Verified created files exist: `gpu/text.rs`, `text_parity.rs`, `realtime_preview_parity.rs`, and this summary.
- Verified task commits exist: `e9d63c3`, `a6ab074`.
- Verified required verification commands passed.
- Verified `reference/` remains untracked and unstaged.

---
*Phase: 11-realtime-preview-runtime-and-gpu-render-backend*
*Completed: 2026-06-18*

---
phase: 19-production-effects-retiming-and-transition-semantics
plan: "01"
subsystem: testing
tags: [rust, playwright, source-guards, retiming, transitions, production-effects]

requires:
  - phase: 18-mobile-server-binding-architecture-and-runtime-ports
    provides: shared editor_runtime, desktop adapter, server/runtime boundary, and no-fallback posture
provides:
  - Phase 19 staged source guard entrypoint and root package scripts
  - RED Rust gates for typed production effects, retiming, transitions, audio DSP retiming, render graph intents, realtime preview, FFmpeg compiler, and testkit parity
  - RED desktop Playwright gate for visible Phase 19 controls and no-fallback product evidence
  - Wave 0 validation command inventory
affects:
  - Phase 19 implementation plans 02-15
  - draft_model
  - draft_commands
  - engine_core
  - audio_engine
  - render_graph
  - realtime_preview_runtime
  - ffmpeg_compiler
  - testkit
  - desktop-electron

tech-stack:
  added: []
  patterns:
    - staged bash source guards with Wave 0 and capability-specific modes
    - compileable RED contract tests that fail via assertions against current string-only semantics
    - desktop RED gate that blocks visible controls until generated Rust contracts and no-fallback evidence exist

key-files:
  created:
    - scripts/phase19-source-guards.sh
    - crates/draft_model/tests/production_effects_contracts.rs
    - crates/draft_commands/tests/retiming_commands.rs
    - crates/draft_commands/tests/transition_commands.rs
    - crates/engine_core/tests/retiming.rs
    - crates/render_graph/tests/production_effects.rs
    - crates/realtime_preview_runtime/tests/production_effects.rs
    - crates/ffmpeg_compiler/tests/production_effects.rs
    - crates/testkit/tests/production_effects_preview.rs
    - crates/testkit/tests/production_effects_exports.rs
    - apps/desktop-electron/tests/production-effects.spec.ts
  modified:
    - package.json
    - crates/audio_engine/tests/dsp_timeline.rs
    - .planning/phases/19-production-effects-retiming-and-transition-semantics/19-VALIDATION.md

key-decisions:
  - "Wave 0 is intentionally RED-only; GREEN implementation belongs to later Phase 19 plans."
  - "PRODFX requirements are not marked complete by 19-01 because this plan establishes gates rather than delivering production semantics."
  - "The Phase 19 guard permits renderer consumption of Rust CommandDelta changedRanges while rejecting renderer-owned dirty/cache/fingerprint construction."

patterns-established:
  - "Staged Phase guard modes: --wave0, --retiming, --retiming-audio, --transition, --effects, --mask-blend, and --ui."
  - "RED tests include both phase19_ names and plan-specified Cargo filter tokens so verification commands cannot pass with zero tests."
  - "Desktop production-effects gate checks generated Rust contracts before allowing visible effect/speed/transition workflows."

requirements-completed: []

duration: 12 min
completed: 2026-06-25
status: complete
---

# Phase 19 Plan 01: Wave 0 Guards And RED Gates Summary

**Phase 19 now has executable source guards and RED gates that fail renderer-owned semantics, string-only effects/transitions, fallback success, and missing retiming/audio/render/export contracts before implementation begins.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-25T06:51:44Z
- **Completed:** 2026-06-25T07:03:59Z
- **Tasks:** 3
- **Files modified:** 14

## Accomplishments

- Added `scripts/phase19-source-guards.sh` with staged modes for Wave 0 and future Phase 19 slices, plus root package scripts `test:phase19-rust`, `test:phase19-source-guards`, `test:phase19-desktop`, and `test:phase19`.
- Added RED Rust semantic tests for typed capability contracts, retiming commands, transition commands, engine time mapping, audio DSP retime mapping, render graph intent/fingerprints, realtime preview support, FFmpeg compiler output, and testkit preview/export parity.
- Added the RED desktop `production-effects.spec.ts` gate and updated `19-VALIDATION.md` with Wave 0 artifacts and exact RED commands while keeping `nyquist_compliant` and `wave_0_complete` false for final closeout.

## Task Commits

1. **Task 1: Add Phase 19 source guards and package scripts** - `bbfc2a1` (feat)
2. **Task 2: Create RED Rust semantic tests** - `6a4cd093` (test)
3. **Task 3: Create RED preview export template and desktop tests** - `515909d` (test)

**Plan metadata:** pending closeout commit

## Files Created/Modified

- `scripts/phase19-source-guards.sh` - Staged architecture/source guard for Phase 19 ownership boundaries and future artifact checks.
- `package.json` - Phase 19 root test script entrypoints.
- `crates/draft_model/tests/production_effects_contracts.rs` - RED typed capability/filter/transition/retime contract checks.
- `crates/draft_commands/tests/retiming_commands.rs` - RED Rust-owned retiming command and source mapping checks.
- `crates/draft_commands/tests/transition_commands.rs` - RED transition adjacency/undo command checks.
- `crates/engine_core/tests/retiming.rs` - RED engine-owned segment time map and frame-state facts checks.
- `crates/audio_engine/tests/dsp_timeline.rs` - RED audio DSP retime/follow-speed diagnostics check.
- `crates/render_graph/tests/production_effects.rs` - RED graph intent/fingerprint/dirty facts checks.
- `crates/realtime_preview_runtime/tests/production_effects.rs` - RED registry-backed GPU preview support checks.
- `crates/ffmpeg_compiler/tests/production_effects.rs` - RED compiler-owned filtergraph and unsupported export classification checks.
- `crates/testkit/tests/production_effects_preview.rs` - RED template preview parity/performance fixture checks.
- `crates/testkit/tests/production_effects_exports.rs` - RED template export parity/fallback report checks.
- `apps/desktop-electron/tests/production-effects.spec.ts` - RED desktop generated-contract and no-fallback product evidence gate.
- `.planning/phases/19-production-effects-retiming-and-transition-semantics/19-VALIDATION.md` - Wave 0 artifact and RED command inventory.

## Decisions Made

- Wave 0 remains RED-only by design. Later Phase 19 plans must turn these gates green; 19-01 does not implement production effects, retiming, transitions, masks, blends, or UI enablement.
- PRODFX-01 through PRODFX-05 are not marked complete. This plan establishes the executable gates for those requirements but does not satisfy the production behavior.
- The source guard distinguishes renderer consumption of Rust-provided `CommandDelta.changedRanges` from forbidden renderer construction of dirty ranges, cache keys, or semantic fingerprints.

## Verification

- `pnpm run test:phase19-source-guards -- --wave0` - PASS.
- `pnpm run test:phase19-source-guards -- --retiming-audio` - PASS.
- `pnpm run test:phase19-source-guards -- --transition` - PASS.
- `pnpm run test:phase19-source-guards -- --effects` - PASS.
- `pnpm run test:phase19-source-guards -- --mask-blend` - PASS.
- `pnpm run test:phase19-source-guards -- --ui` - PASS.
- `cargo fmt --all` - PASS.
- RED Rust wrapper from Task 2 - PASS because each planned Cargo command failed for the intended missing implementation.
- RED preview/export/desktop wrapper from Task 3 - PASS because runtime, compiler, testkit, and Playwright gates failed for the intended missing implementation.

Note: pnpm printed the existing engine warning that local Node is `v24.15.0` while `package.json` asks for `24.12.0`; the guard command still passed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Narrowed the dirty/cache guard to avoid blocking Rust delta consumption**
- **Found during:** Task 1
- **Issue:** The initial `--wave0` source guard rejected existing renderer/main reads of Rust-provided `CommandDelta.changedRanges`, which are allowed routing data rather than renderer-owned dirty range semantics.
- **Fix:** Replaced broad `changedRanges` matching with construction-oriented dirty/cache/fingerprint patterns such as `computeDirtyRange`, `previewCacheKey`, and `semanticFingerprint`.
- **Files modified:** `scripts/phase19-source-guards.sh`
- **Verification:** `pnpm run test:phase19-source-guards -- --wave0`
- **Committed in:** `bbfc2a1`

**2. [Rule 1 - Bug] Renamed RED tests so plan-specified Cargo filters execute them**
- **Found during:** Task 2
- **Issue:** `cargo test -p draft_model production_effects_contracts -- --nocapture` initially matched zero tests because the function names lacked the Cargo filter token.
- **Fix:** Renamed RED tests to include both `phase19_` and the plan's filter tokens, preventing false green zero-test runs.
- **Files modified:** `crates/draft_model/tests/production_effects_contracts.rs`, `crates/draft_commands/tests/retiming_commands.rs`, `crates/draft_commands/tests/transition_commands.rs`, `crates/engine_core/tests/retiming.rs`, `crates/render_graph/tests/production_effects.rs`
- **Verification:** Task 2 RED wrapper executed all intended tests and passed only because they failed for missing semantics.
- **Committed in:** `6a4cd093`

---

**Total deviations:** 2 auto-fixed (2 bugs)
**Impact on plan:** Both fixes strengthened the gates and prevented false failures or false greens. No implementation scope was added.

## Issues Encountered

- RED tests intentionally fail until later Phase 19 implementation plans add typed contracts, registry support, retime mapping, transition commands, compiler output, and product E2E parity.
- No authentication gates occurred.

## Known Stubs

None. The only "placeholder" scan hit is assertion text in `apps/desktop-electron/tests/production-effects.spec.ts` describing static placeholder strings; no product stub or empty data source was introduced.

## Threat Flags

None. The plan added tests, package scripts, a source guard, and validation notes only; it introduced no new network endpoints, auth paths, file access surface beyond test source reads, or schema trust-boundary changes.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 19 Plan 02 can now define the typed capability registry and effect/transition/retime contracts against executable RED gates. Full Phase 19 validation remains pending; `19-VALIDATION.md` intentionally keeps `nyquist_compliant: false` and `wave_0_complete: false` until final aggregate closeout and independent UI audit requirements are satisfied.

## Self-Check: PASSED

- Created/modified files listed above exist on disk.
- Task commits found: `bbfc2a1`, `6a4cd093`, `515909d`.
- Final guard check `pnpm run test:phase19-source-guards -- --wave0` passed.
- Unrelated `.planning/research/.cache/` remains untracked and unstaged.

---
*Phase: 19-production-effects-retiming-and-transition-semantics*
*Completed: 2026-06-25*

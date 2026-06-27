---
phase: 18-mobile-server-binding-architecture-and-runtime-ports
plan: "02"
subsystem: validation-guards
tags: [rust, shell, source-guards, cbindgen, mobile-contracts, package-scripts]
requires:
  - phase: "18-01"
    provides: "editor_runtime authority crate plus bindings_c and server_runtime shells"
provides:
  - "Phase 18 architecture source guard with negative self-tests and owner-aware staged modes"
  - "Pinned cbindgen 0.29.4 ABI header drift gate with project-local bootstrap"
  - "Mobile runtime contract guard for JNI/Swift lifecycle and handle ownership expectations"
  - "Root package scripts for Phase 18 per-wave and aggregate verification"
affects: [phase-18, bindings-node, bindings-c, server-runtime, mobile-contracts, validation]
tech-stack:
  added:
    - "Project-local cbindgen 0.29.4 bootstrap path under target/phase18-tools"
  patterns:
    - "Phase guards accept pnpm's literal -- separator before forwarded script arguments"
    - "Full/default Phase 18 source guard is reserved for Plan 06/aggregate; staged modes cover Plan 03, Plan 04, Plan 05, and mobile contracts"
key-files:
  created:
    - scripts/phase18-source-guards.sh
    - scripts/phase18-abi-drift.sh
    - scripts/phase18-mobile-contract-guards.sh
  modified:
    - package.json
key-decisions:
  - "Source guard full/default mode requires every Phase 18 artifact and should run only in Plan 06 or aggregate verification."
  - "Staged source guard modes check only their owning completed plan/dependency artifacts: --plan 03, --plan 04, --plan 05, and --mobile-contracts."
  - "ABI drift uses a script-owned project-local cbindgen 0.29.4 binary and rejects any resolved version other than exactly cbindgen 0.29.4."
  - "No @napi-rs/cli package upgrade, reinstall, or lockfile edit was made."
patterns-established:
  - "Guard scripts include self-tests that inject forbidden code and prove comment-only text is ignored."
  - "Future aggregate scripts expose stable test:phase18-* names while Wave 1 verification uses self-test arguments only."
requirements-completed: [PLAT-01, PLAT-02, PLAT-03, BIND-01, BIND-02, BIND-03, BIND-04, BIND-05]
duration: 13 min
completed: 2026-06-25
status: complete
---

# Phase 18 Plan 02: Guard And Validation Scaffold Summary

**Executable Phase 18 guard scaffold for adapter ownership, pinned C ABI header drift, mobile lifecycle contracts, and aggregate test script wiring.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-06-25T00:47:07Z
- **Completed:** 2026-06-25T01:00:34Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments

- Added `scripts/phase18-source-guards.sh` with comment-filtered pattern matching, negative self-tests, full/default aggregate mode, and staged `--plan 03`, `--plan 04`, `--plan 05`, and `--mobile-contracts` modes.
- Added `scripts/phase18-abi-drift.sh` with project-local `cbindgen 0.29.4` bootstrap, exact version assertion, header regeneration, and git diff drift checking.
- Added `scripts/phase18-mobile-contract-guards.sh` for Android JNI lifecycle, Swift/ObjC C import ownership, sandboxed file handles, texture/device identity, cancellation, explicit release, session close, and mobile handle smoke-test expectations.
- Wired root `package.json` scripts: `test:phase18-rust`, `test:phase18-source-guards`, `test:phase18-abi`, `test:phase18-server`, `test:phase18-mobile-contracts`, and `test:phase18`.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Add failing source guard self-test** - `dbfcd8c` (test)
2. **Task 1 GREEN: Implement Phase 18 source guards** - `63e6069` (feat)
3. **Task 2 RED: Add failing ABI/mobile guard self-tests** - `4f7a378` (test)
4. **Task 2 GREEN: Implement ABI and mobile contract guards** - `7dbd5da` (feat)
5. **Task 3: Wire Phase 18 package scripts** - `33c2891` (chore)

## Files Created/Modified

- `scripts/phase18-source-guards.sh` - Architecture guard for adapter semantic duplication, C-to-Node dependency violations, Electron render/export ownership, fallback success evidence, and adapter-owned lifetime policy.
- `scripts/phase18-abi-drift.sh` - Pinned `cbindgen 0.29.4` bootstrap/version/drift gate for `crates/bindings_c/include/video_editor_runtime.h`.
- `scripts/phase18-mobile-contract-guards.sh` - Mobile lifecycle and handle ownership contract guard with full and smoke-only modes.
- `package.json` - Adds Phase 18 per-wave and aggregate script names.

## Decisions Made

- Full/default source guard mode intentionally requires future Plan 03-06 artifacts and was not run during Plan 18-02.
- Staged source guard modes are owner-aware so Wave 2 plans do not fail because sibling/later artifacts are absent.
- ABI self-test uses fake `cbindgen` binaries and a temp git repo, so it proves version/drift behavior without installing tooling during this plan.
- Package scripts expose the future aggregate gate now, but this plan verifies only the self-test forms because C ABI, server, and mobile contract artifacts are later-plan outputs.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed mobile guard self-test path override handling**
- **Found during:** Task 2 (ABI and mobile contract guard scripts)
- **Issue:** The mobile guard cached default doc/test paths before self-test overrides, then expected-failure branches exited the parent shell.
- **Fix:** Resolved doc/test paths at validation time and ran expected-failure checks in subshells.
- **Files modified:** `scripts/phase18-mobile-contract-guards.sh`
- **Verification:** `bash scripts/phase18-mobile-contract-guards.sh --self-test` passed.
- **Committed in:** `7dbd5da`

**2. [Rule 1 - Bug] Accepted pnpm's forwarded argument separator**
- **Found during:** Task 3 (package script verification)
- **Issue:** `pnpm run test:phase18-*- -- --self-test` invokes scripts with a literal leading `--`, so guards treated the self-test flag as unknown or ran full/default mode.
- **Fix:** Normalized a leading `--` before mode dispatch in all three Phase 18 guard scripts.
- **Files modified:** `scripts/phase18-source-guards.sh`, `scripts/phase18-abi-drift.sh`, `scripts/phase18-mobile-contract-guards.sh`
- **Verification:** All three `pnpm run test:phase18-* -- --self-test` commands passed.
- **Committed in:** `33c2891`

---

**Total deviations:** 2 auto-fixed (2 Rule 1 bugs)
**Impact on plan:** Both fixes were necessary for the planned self-test gates to run through the package scripts. No scope expansion or package changes were introduced.

## Issues Encountered

- `pnpm` emitted the existing engine warning: package expects Node `24.12.0`, current runtime is `24.15.0`. This did not block verification.
- Full `pnpm run test:phase18` was not run by design; `18-VALIDATION.md` states the full aggregate is valid only after Plans 03-06 create the required Node/C/server/mobile artifacts.
- `requirements.mark-complete PLAT-01 PLAT-02 PLAT-03 BIND-01 BIND-02 BIND-03 BIND-04 BIND-05` returned `not_found` because the current requirements file stores those IDs as narrative requirement rows rather than SDK-markable checklist entries.

## Verification

- `bash scripts/phase18-source-guards.sh --self-test` - passed.
- `bash scripts/phase18-abi-drift.sh --self-test` - passed.
- `bash scripts/phase18-mobile-contract-guards.sh --self-test` - passed.
- `pnpm run test:phase18-source-guards -- --self-test` - passed with the pre-existing Node engine warning.
- `pnpm run test:phase18-abi -- --self-test` - passed with the pre-existing Node engine warning.
- `pnpm run test:phase18-mobile-contracts -- --self-test` - passed with the pre-existing Node engine warning.
- `node -e 'JSON.parse(...)'` for `package.json` - passed.
- Package script inspection confirmed `test:phase18` includes `pnpm run test:no-product-fallback` and `pnpm run test:contracts`, and no Phase 18 script contains watch flags.

## TDD Gate Compliance

- RED gate present for Task 1: `dbfcd8c`.
- GREEN gate present for Task 1: `63e6069`.
- RED gate present for Task 2: `4f7a378`.
- GREEN gate present for Task 2: `7dbd5da`.
- Task 3 was not a TDD task.

## Known Stubs

None. Stub scan found no TODO/FIXME/placeholder or UI-flowing empty-data patterns in the created/modified files.

## Threat Mitigations

- **T-18-05 / Source guard tampering:** Negative self-tests prove forbidden adapter/server/Electron/fallback/lifetime patterns are detected and comment-only text is ignored.
- **T-18-06 / Verification repudiation:** Root scripts now name the Phase 18 Rust, source guard, ABI, server, mobile, no-fallback, and contract gates.
- **T-18-07 / C ABI header drift:** ABI guard regenerates the C header through exactly `cbindgen 0.29.4` and fails dirty header diffs.
- **T-18-SC / Package tool risk:** The script bootstraps only `cbindgen 0.29.4` project-locally when full ABI drift runs and does not modify `@napi-rs/cli`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 18-03 can thin `bindings_node` and run `bash scripts/phase18-source-guards.sh --plan 03`. Plans 18-04 and 18-05 can add C ABI/server artifacts and use `--plan 04`, ABI drift, `--plan 05`, and server smoke gates. Plan 18-06 can add mobile contract docs and run full/default source guards plus the aggregate `pnpm run test:phase18`.

## Self-Check: PASSED

- Found created files on disk: `scripts/phase18-source-guards.sh`, `scripts/phase18-abi-drift.sh`, and `scripts/phase18-mobile-contract-guards.sh`.
- Confirmed `package.json` contains all Phase 18 scripts named by the plan.
- Confirmed task commits exist in git history: `dbfcd8c`, `63e6069`, `4f7a378`, `7dbd5da`, and `33c2891`.

---
*Phase: 18-mobile-server-binding-architecture-and-runtime-ports*
*Completed: 2026-06-25*

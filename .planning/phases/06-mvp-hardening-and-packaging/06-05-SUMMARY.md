---
phase: 06-mvp-hardening-and-packaging
plan: 05
subsystem: release
tags: [docs, release, ffmpeg, guards, packaging]
requires:
  - phase: 06-mvp-hardening-and-packaging
    provides: Packaged app and no-mock workflow gates from Plans 06-01 through 06-04
provides:
  - External FFmpeg MVP release manifest
  - Third-party notice posture
  - MVP known limits and post-MVP backlog
  - Phase 06 release guard script
  - Root pnpm and just Phase 06 gate wiring
affects: [docs, release-gates, packaging, phase-06]
tech-stack:
  added: []
  patterns:
    - External FFmpeg posture is guarded by exact release-doc strings.
    - Ordinary root tests run Phase 06 runtime/release checks while packaged verification stays explicit.
key-files:
  created:
    - docs/release-ffmpeg-manifest.md
    - docs/third-party-notices.md
    - docs/mvp-known-limits.md
    - scripts/phase6-release-guards.sh
  modified:
    - package.json
    - justfile
    - apps/desktop-electron/package.json
    - scripts/phase4-source-guards.sh
key-decisions:
  - "Phase 6 ships external/user-provided FFmpeg only; no FFmpeg binary is bundled by Phase 6."
  - "Root test includes `test:phase6`, but slower packaged verification remains explicit through `test:phase6-packaging` and `just test-phase6-packaging`."
  - "Desktop package default `test` excludes packaged specs so root tests do not rely on prebuilt package artifacts."
patterns-established:
  - "Release guard checks exact external-runtime strings, app/root script presence, package config, Phase 5 source guards, and generated contract drift."
  - "Phase 4 source guard allows `commandHelpers.ts` as the display/envelope boundary for Rust-owned runtime diagnostics while keeping component-level renderer runtime ownership blocked."
requirements-completed: [TEST-06, TEST-07]
duration: 35 min
completed: 2026-06-18
---

# Phase 06 Plan 05: Release Readiness Summary

**Phase 06 now has release posture docs, guarded public Phase 06 gates, and explicit packaged verification commands.**

## Performance

- **Duration:** 35 min
- **Completed:** 2026-06-18
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- Added `docs/release-ffmpeg-manifest.md` with exact guardable strings:
  `FFmpeg is external/user-provided for the MVP`, `VE_FFMPEG_PATH`, `VE_FFPROBE_PATH`, `No FFmpeg binary is bundled by Phase 6`, and `Homebrew --enable-gpl is development/test only`.
- Added `docs/third-party-notices.md` distinguishing the project MIT license and dependency posture from external user-provided FFmpeg.
- Added `docs/mvp-known-limits.md` covering external FFmpeg, signing/notarization, Phases 7-13 semantics, Jianying/CapCut/Kaipai compatibility backlog, mobile, and server scope.
- Added executable `scripts/phase6-release-guards.sh` for release-doc, script-surface, package-config, Phase 5 source-guard, and generated-contract checks.
- Added root scripts: `test:phase6-packaging`, `test:phase6-runtime`, `test:phase6-release-gates`, and `test:phase6`.
- Added `just test-phase6-packaging` and wired `just test` to run `pnpm run test:phase6`.
- Updated desktop default `test` to exclude packaged specs so packaging remains an explicit slower gate.

## Task Commits

1. **Task 1: Write release docs for external FFmpeg posture, notices, known limits, and backlog** - `ef1cd8b` (docs)
2. **Task 2: Add Phase 06 release guards and public root gates** - `0ebebaa` (test)

## Files Created/Modified

- `docs/release-ffmpeg-manifest.md` - external FFmpeg/runtime posture and future bundled FFmpeg checklist.
- `docs/third-party-notices.md` - MIT/dependency notices plus external FFmpeg note.
- `docs/mvp-known-limits.md` - MVP limits and post-MVP backlog.
- `scripts/phase6-release-guards.sh` - release guard.
- `package.json` - Phase 06 root scripts and root test wiring.
- `justfile` - `test` Phase 06 wiring and explicit packaged command.
- `apps/desktop-electron/package.json` - excludes packaged specs from default desktop test.
- `scripts/phase4-source-guards.sh` - allows command helper display/envelope boundary for runtime diagnostics.

## Decisions Made

- Bundled FFmpeg remains deferred; any later bundle must ship manifest, notices, legal review, source-offer review, and packaged resource tests together.
- Root `pnpm run test` and `just test` include non-packaging Phase 06 checks only.
- Packaged smoke/workflow gates remain explicit through `pnpm run test:phase6-packaging` and `just test-phase6-packaging`.

## Deviations from Plan

- Added `apps/desktop-electron/package.json` and `scripts/phase4-source-guards.sh` to the touched files because full root gate verification exposed that default desktop tests were implicitly discovering packaged specs, and the older Phase 4 source guard needed the same `commandHelpers.ts` display/envelope boundary already used by Phase 5 guards.

## Issues Encountered

- Initial `pnpm run test` failed at `test:phase4-source-guards` after Phase 6 diagnostics introduced read-only FFmpeg/ffprobe labels in `commandHelpers.ts`. The guard was updated to keep blocking renderer-owned runtime/render behavior outside the command helper boundary.

## Verification

- `test -f docs/release-ffmpeg-manifest.md && test -f docs/third-party-notices.md && test -f docs/mvp-known-limits.md`
- `rg -n "FFmpeg is external/user-provided for the MVP|VE_FFMPEG_PATH|VE_FFPROBE_PATH|No FFmpeg binary is bundled by Phase 6|Homebrew --enable-gpl is development/test only" docs/release-ffmpeg-manifest.md`
- `rg -n "signing|notarization|Phases 7-13|Jianying|CapCut|Kaipai|mobile|server" docs/mvp-known-limits.md`
- `pnpm run test:phase6-release-gates`
- `pnpm run test:phase6-runtime`
- `pnpm run test:phase6-packaging`
- `pnpm run test`
- `/Users/zhiwen/.cargo/bin/just test`

## User Setup Required

- No new service setup.
- No-mock runtime and packaged gates require local `ffmpeg` and `ffprobe` through `PATH` or `VE_FFMPEG_PATH` / `VE_FFPROBE_PATH`.
- Public distribution still needs signing/notarization and any future bundled FFmpeg legal artifacts before shipping outside local MVP use.

## Next Phase Readiness

Ready to close Phase 06 and move into Phase 07: the MVP is now packaged, runtime-diagnosed, no-mock workflow-tested, and release posture/documentation is guarded.

## Self-Check: PASSED

All Plan 05 tasks are implemented, committed, and covered by the required release, runtime, packaged, root pnpm, and just gates.

---
*Phase: 06-mvp-hardening-and-packaging*
*Completed: 2026-06-18*

---
phase: 01-foundation-and-golden-harness
plan: 08
subsystem: desktop-shell
tags: [electron, react, vite, playwright, napi-rs, ipc]

requires:
  - phase: 01-04
    provides: Node-API binding crate exposing ping, version, and execute_command
  - phase: 01-06
    provides: Rust-generated TypeScript command/result contracts
provides:
  - Minimal Electron desktop package with pinned approved dependencies
  - Context-isolated preload bridge exposing only videoEditorCore ping, version, and executeCommand
  - Centralized native binding loader with bounded standardized envelope errors
  - Playwright Electron smoke proving renderer-to-Rust binding calls
affects: [phase-1-foundation, electron-shell, bindings-node, command-contracts, desktop-ui]

tech-stack:
  added: [electron-42.4.1, react-19.2.7, react-dom-19.2.7, vite-8.0.16, '@vitejs/plugin-react-6.0.2', '@playwright/test-1.61.0', '@napi-rs/cli-3.7.2']
  patterns:
    - Electron main owns native binding loading and registers only core:ping, core:version, and core:executeCommand
    - Preload exposes a fixed videoEditorCore object and never exposes raw ipcRenderer or arbitrary channels
    - Renderer imports generated Rust-owned TypeScript contracts and stays UI-only

key-files:
  created:
    - apps/desktop-electron/package.json
    - apps/desktop-electron/tsconfig.json
    - apps/desktop-electron/vite.config.ts
    - apps/desktop-electron/playwright.config.ts
    - apps/desktop-electron/index.html
    - apps/desktop-electron/src/main/nativeBinding.ts
    - apps/desktop-electron/src/renderer/App.tsx
    - apps/desktop-electron/tests/electron-smoke.spec.ts
  modified:
    - .gitignore
    - pnpm-lock.yaml
    - apps/desktop-electron/src/main/index.ts
    - apps/desktop-electron/src/preload/index.ts
    - apps/desktop-electron/src/renderer/main.tsx
    - apps/desktop-electron/src/renderer/styles.css

key-decisions:
  - "Kept the Electron privileged boundary in main/preload: renderer code calls only window.videoEditorCore and never imports Electron or Node APIs."
  - "Built the native addon through approved @napi-rs/cli during desktop build/test instead of committing native artifacts."
  - "Used the Rust-generated CommandEnvelope and CommandResultEnvelope TypeScript contracts at the Electron IPC/test boundary."

patterns-established:
  - "Desktop build emits main, preload, and renderer bundles through Vite modes, with generated native outputs ignored."
  - "Native binding load failures return bounded ok/error/events envelopes rather than throwing raw module-load errors to the renderer."
  - "Playwright Electron tests launch dist/main/index.cjs and assert the exact preload API shape."

requirements-completed: [FOUND-02, FOUND-01]

duration: 11 min
completed: 2026-06-17
---

# Phase 1 Plan 08: Minimal Electron Shell And Binding Smoke Summary

**Electron desktop smoke shell with a context-isolated preload bridge calling the Rust Node-API binding through generated command contracts.**

## Performance

- **Duration:** 11 min
- **Started:** 2026-06-16T22:24:16Z
- **Completed:** 2026-06-16T22:36:07Z
- **Tasks:** 2
- **Files modified:** 14

## Accomplishments

- Added the `@video-editor/desktop` Electron workspace with pinned approved Electron, React, Vite, TypeScript, Playwright, and napi-rs CLI versions.
- Implemented a narrow Electron main/preload boundary with only `core:ping`, `core:version`, and `core:executeCommand`.
- Added a native binding loader that centralizes napi-rs addon loading and maps load failures into bounded command envelopes.
- Built a sparse editor workbench first screen with material bin, preview monitor, inspector, and timeline regions.
- Added a Playwright Electron smoke test proving renderer calls reach the Rust binding through `window.videoEditorCore`.

## Task Commits

1. **Task 01-W4-03: Configure desktop Electron package** - `2f36e74` (chore)
2. **Task 01-W4-04 RED: Add failing Electron IPC smoke** - `dabd9cd` (test)
3. **Task 01-W4-04 GREEN: Implement Electron binding bridge** - `1de78b2` (feat)

_Note: Task 01-W4-04 followed RED/GREEN TDD. No separate refactor commit was needed._

## Files Created/Modified

- `apps/desktop-electron/package.json` - Defines desktop package scripts and approved dependencies, including napi-rs native build wiring.
- `apps/desktop-electron/tsconfig.json` - TypeScript configuration for Electron, renderer, config, and tests.
- `apps/desktop-electron/vite.config.ts` - Vite build modes for main, preload, and renderer bundles.
- `apps/desktop-electron/playwright.config.ts` - Playwright Electron smoke configuration.
- `apps/desktop-electron/index.html` - Renderer HTML entry.
- `apps/desktop-electron/src/main/index.ts` - Electron main process window setup and the three planned IPC handlers.
- `apps/desktop-electron/src/main/nativeBinding.ts` - Centralized native addon loader with bounded envelope errors.
- `apps/desktop-electron/src/preload/index.ts` - Context-isolated `window.videoEditorCore` API.
- `apps/desktop-electron/src/renderer/App.tsx` - Sparse Jianying-style workbench smoke UI using generated command contracts.
- `apps/desktop-electron/src/renderer/main.tsx` - React renderer entrypoint.
- `apps/desktop-electron/src/renderer/styles.css` - Desktop workbench layout and restrained editor styling.
- `apps/desktop-electron/tests/electron-smoke.spec.ts` - Playwright Electron bridge smoke using generated TypeScript contracts.
- `.gitignore` - Ignores generated native, Playwright, and build outputs.
- `pnpm-lock.yaml` - Locks approved desktop dependency graph.

## Verification

- `pnpm --filter @video-editor/desktop build` - PASS.
- `pnpm --filter @video-editor/desktop test` - PASS, 1 Playwright Electron smoke passed.
- `grep -R "ipcRenderer" apps/desktop-electron/src/renderer apps/desktop-electron/src/preload | grep -v "invoke" | grep -v "preload/index.ts" && exit 1 || true` - PASS.
- `grep -R "ffmpeg\\|ffprobe" apps/desktop-electron/src && exit 1 || true` - PASS.

## TDD Gate Compliance

- **RED:** `dabd9cd` added the Playwright Electron smoke and native build test infrastructure. After Electron binary download completed, the RED run failed because `window.videoEditorCore` was not exposed.
- **GREEN:** `1de78b2` implemented native loading, main IPC handlers, preload exposure, and renderer contract calls. The Electron smoke and grep gates passed.
- **REFACTOR:** No separate refactor commit was needed.

## Decisions Made

- Kept generated native artifacts out of git and rebuilt them through `pnpm run build:native`.
- Emitted the napi-rs JS loader as `index.cjs` so it remains loadable from a `type: module` package.
- Left the BrowserWindow preload sandbox enabled; privileged native loading stays in the Electron main process.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added minimal entrypoints during package configuration**
- **Found during:** Task 01-W4-03 (Configure desktop Electron package)
- **Issue:** The package build script could not pass with only config files because Vite needed main, preload, and renderer entries.
- **Fix:** Added minimal placeholder entrypoints and a sparse workbench shell in the Task 1 commit; Task 2 replaced the placeholders with the required IPC and binding behavior.
- **Files modified:** `apps/desktop-electron/src/main/index.ts`, `apps/desktop-electron/src/preload/index.ts`, `apps/desktop-electron/src/renderer/main.tsx`, `apps/desktop-electron/src/renderer/styles.css`
- **Verification:** `pnpm --filter @video-editor/desktop build`
- **Committed in:** `2f36e74`

**2. [Rule 3 - Blocking] Added approved native build tooling to desktop tests**
- **Found during:** Task 01-W4-04 (Add narrow preload IPC and Electron binding smoke)
- **Issue:** Electron could not prove a real Rust binding call without building a `.node` addon artifact for the app process.
- **Fix:** Added approved `@napi-rs/cli@3.7.2`, wired `build:native`, and ignored generated native output.
- **Files modified:** `apps/desktop-electron/package.json`, `pnpm-lock.yaml`, `.gitignore`
- **Verification:** `pnpm --filter @video-editor/desktop test`
- **Committed in:** `dabd9cd`

**3. [Rule 1 - Bug] Fixed CJS native loader path for Electron main**
- **Found during:** Task 01-W4-04 GREEN verification
- **Issue:** Vite's CJS output rewrote `createRequire(import.meta.url)` into an unusable expression, causing the Electron app to hang before the smoke assertion. The generated napi loader also needed a CJS extension under the app's `type: module` package.
- **Fix:** Switched to `createRequire(__filename)` and emitted the generated napi loader as `native/index.cjs`.
- **Files modified:** `apps/desktop-electron/src/main/nativeBinding.ts`, `apps/desktop-electron/package.json`
- **Verification:** `pnpm --filter @video-editor/desktop test`
- **Committed in:** `1de78b2`

### Tooling Lookup Deviations

**1. Context7 CLI unavailable**
- **Found during:** Electron and Playwright documentation lookup
- **Issue:** Required Context7 CLI fallback reported `ctx7 not found`.
- **Fix:** Used the already-approved versions and IPC patterns from `01-RESEARCH.md`, then verified behavior with the executable Electron smoke.
- **Files modified:** None
- **Verification:** `pnpm --filter @video-editor/desktop test`

---

**Total deviations:** 3 auto-fixed (Rule 3: 2, Rule 1: 1), 1 tooling lookup fallback.
**Impact on plan:** All fixes were required to make the planned Electron binding smoke executable and did not expand Phase 1 beyond ping/version/executeCommand.

## Issues Encountered

- The first RED run timed out while Playwright downloaded the Electron binary. Re-running after the binary was present produced the intended RED failure: `window.videoEditorCore` was missing.
- The generated native and Playwright output folders are intentionally ignored and regenerated by build/test commands.

## Known Stubs

None blocking. The renderer is intentionally a sparse smoke workbench; real material data, timeline interaction, and inspector state are deferred to later UI phases.

## Threat Flags

None. The new renderer-to-preload, preload-to-main, and main-to-native surfaces are covered by the plan threat model and mitigated by the fixed preload API, exact IPC channel list, generated TypeScript contracts, sandboxed preload, and bounded native load errors.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for Plan 01-09 to wire the final `just` build/test gates and CI around the Rust, generated-contract, render-smoke, and Electron smoke checks.

## Self-Check: PASSED

- Key files exist on disk: `apps/desktop-electron/package.json`, `apps/desktop-electron/src/main/index.ts`, `apps/desktop-electron/src/main/nativeBinding.ts`, `apps/desktop-electron/src/preload/index.ts`, `apps/desktop-electron/src/renderer/App.tsx`, and `apps/desktop-electron/tests/electron-smoke.spec.ts`.
- Task commits `2f36e74`, `dabd9cd`, and `1de78b2` exist in git history.
- Plan verification commands passed.
- Stub scan found no TODO/FIXME/placeholder markers or blocking UI stubs.

---
*Phase: 01-foundation-and-golden-harness*
*Completed: 2026-06-17*

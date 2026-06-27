---
phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
plan: "09"
subsystem: desktop-ui
tags: [electron, react, rust, template-import, kaipai, e2e]

requires:
  - phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
    provides: Backend template import/report/resource/session/preview/export gates through plan 17-08
provides:
  - Desktop UI entry for offline Kaipai formula bundle import
  - Sandboxed Electron main/preload/native bridge for importKaipaiFormulaBundle
  - Product-safe Chinese adaptation report panel
  - Product E2E covering report, timeline, preview, export, persistence, and no-fallback assertions
affects: [desktop-electron, bindings_node, adapter_kaipai, product-e2e, phase17]

tech-stack:
  added: []
  patterns:
    - Renderer sends narrow session/revision/path import requests through preload/main to Rust
    - Product report copy maps Rust AdaptationReport statuses to bounded Chinese UI text
    - Project-session export resolves bundle-relative material paths before FFmpeg compilation

key-files:
  created:
    - apps/desktop-electron/tests/template-import.spec.ts
  modified:
    - apps/desktop-electron/src/main/nativeBinding.ts
    - apps/desktop-electron/src/main/index.ts
    - apps/desktop-electron/src/preload/index.ts
    - apps/desktop-electron/src/renderer/App.tsx
    - apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx
    - apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx
    - apps/desktop-electron/src/renderer/styles.css
    - apps/desktop-electron/tests/helpers/foregroundProductApp.ts
    - crates/bindings_node/src/lib.rs
    - crates/adapter_kaipai/src/mapper.rs
    - crates/adapter_kaipai/tests/mapper.rs
    - package.json

key-decisions:
  - "Desktop template import uses an explicit native bridge and IPC channel instead of a generic command envelope."
  - "The renderer renders only bounded product copy from Rust AdaptationReport data; raw provider JSON, provenance, paths, and diagnostics stay out of the UI."
  - "The Phase 17 desktop gate refreshes the packaged app before macOS foreground product E2E, because that helper launches the packaged bundle."
  - "Project-session export resolves bundle-relative material URIs against the canonical .veproj path before FFmpeg compilation without rewriting project.json."
  - "Kaipai video imports no longer imply audio streams unless the resource is explicitly audio."

patterns-established:
  - "Template import UI state updates from ProjectSessionTemplateImportResponse revision/viewModel/materials only."
  - "Template report rows use status/category labels and fixed product copy, not backend message/details/provenance strings."
  - "Product E2E fixture preparation writes temp bundle JSON with seeded resource checksums so Rust default checksum verification remains enabled."

requirements-completed: [COMP-01, COMP-02, TEST-E2E-01, NO-FALLBACK-01]

duration: 30min
completed: 2026-06-24
status: complete
---

# Phase 17 Plan 09: Desktop Template Import UI Summary

**Offline template import now has an Electron UI entry, product-safe adaptation report, Rust-owned import bridge, and product E2E proof through preview, export, persistence, and no-fallback gates.**

## Performance

- **Duration:** 30 min
- **Started:** 2026-06-24T10:43:57Z
- **Completed:** 2026-06-24T11:13:18Z
- **Tasks:** 3
- **Files modified:** 13

## Accomplishments

- Added `window.videoEditorCore.importKaipaiFormulaBundle`, `core:importKaipaiFormulaBundle`, and `platform:openTemplateBundle` with explicit types and native binding validation.
- Added the desktop "智能包装" template import entry and report panel with counts/items for supported, approximated, dropped, missing resource, and native-effect-dependent statuses.
- Added product E2E coverage that imports three offline fixture bundles through the normal UI, verifies safe Chinese report copy, asserts clean persisted `project.json`, checks real GPU-composited preview evidence, exports output, and rejects fallback evidence.
- Wired `test:phase17-desktop` into the aggregate Phase 17 gate and made it refresh the packaged macOS app before foreground E2E.
- Fixed Rust-side blockers found by the product E2E: bundle-relative material URI resolution for project-session export and unsafe audio metadata for imported video-only template resources.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add product E2E for offline template import UI** - `8407605` (test)
2. **Task 2: Wire Electron main/preload native import bridge** - `2fc4cda` (feat)
3. **Task 3: Add template import entry and report panel** - `87fd8f0` (feat)

## Files Created/Modified

- `apps/desktop-electron/tests/template-import.spec.ts` - Product E2E for offline template import, report copy, preview/export evidence, and clean persistence.
- `apps/desktop-electron/src/main/nativeBinding.ts` - Typed native wrapper and load validation for `importKaipaiFormulaBundle`.
- `apps/desktop-electron/src/main/index.ts` - Main-process IPC bridge, platform template picker, and test observations.
- `apps/desktop-electron/src/preload/index.ts` - Sandboxed renderer API for the new import command and template picker.
- `apps/desktop-electron/src/renderer/App.tsx` - Template import command handler, report state, session revision update, and preview/export invalidation.
- `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx` - Prop threading for template report and import callback.
- `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx` - Template panel and product-safe report rendering.
- `apps/desktop-electron/src/renderer/styles.css` - Dense desktop panel/report styling.
- `apps/desktop-electron/tests/helpers/foregroundProductApp.ts` - Packaged macOS test launcher forwards template bundle selections.
- `crates/bindings_node/src/lib.rs` - Project-session export resolves bundle-relative material paths before FFmpeg compilation.
- `crates/adapter_kaipai/src/mapper.rs` - Imported video materials no longer claim audio streams without evidence.
- `crates/adapter_kaipai/tests/mapper.rs` - Updated mapper expectation for video-only import metadata.
- `package.json` - Added `test:phase17-desktop` and included it in `test:phase17`.

## Decisions Made

- Kept UI semantics narrow: renderer selects an offline bundle/resource root and passes `sessionId`, `expectedRevision`, `bundlePath`, `resourceRoot`, and `importId` to Rust.
- Did not render backend `message`, `details`, `provenance`, raw formula IDs, local paths, or remote URLs in the report panel.
- Made the desktop Phase 17 gate package first because macOS product E2E intentionally launches the packaged foreground app for realistic preview evidence.
- Resolved bundle-relative media paths only in the transient export draft, preserving `.veproj/project.json` as canonical relative project data.
- Treated video import audio metadata conservatively until the adapter or media probe has explicit audio-stream evidence.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Rewrote E2E fixture bundles with real seeded checksums**
- **Found during:** Task 3
- **Issue:** The product E2E seeds local resources while Rust defaults `verifyResourceSha256` to true; using fixture placeholder checksums would reject otherwise valid UI imports.
- **Fix:** Test fixture preparation now creates temp bundle JSON and rewrites only existing checksum fields to the seeded resource hashes.
- **Files modified:** `apps/desktop-electron/tests/template-import.spec.ts`
- **Verification:** `pnpm --filter @video-editor/desktop exec playwright test tests/template-import.spec.ts --reporter=line`
- **Committed in:** `87fd8f0`

**2. [Rule 3 - Blocking] Forwarded template picker data through macOS foreground E2E launcher**
- **Found during:** Task 3
- **Issue:** The packaged foreground launcher did not pass `VIDEO_EDITOR_TEST_OPEN_TEMPLATE_BUNDLE` to the app, so the UI click opened the real picker instead of deterministic fixture selections.
- **Fix:** Added `--video-editor-test-open-template-bundle` and `--video-editor-test-template-resource-root` forwarding, and made `test:phase17-desktop` package before running the E2E.
- **Files modified:** `apps/desktop-electron/tests/helpers/foregroundProductApp.ts`, `package.json`
- **Verification:** `pnpm run test:phase17`
- **Committed in:** `87fd8f0`

**3. [Rule 1 - Bug] Resolved bundle-relative imported material paths before project-session export**
- **Found during:** Task 3
- **Issue:** Imported template materials are persisted as `.veproj`-relative URIs, but export handed those relative paths to FFmpeg from the app process cwd.
- **Fix:** `startProjectSessionExport` now resolves material URIs against the session bundle path in Rust before export compilation, without mutating persisted project JSON.
- **Files modified:** `crates/bindings_node/src/lib.rs`
- **Verification:** `pnpm --filter @video-editor/desktop exec playwright test tests/template-import.spec.ts --reporter=line`, `cargo check --workspace --locked`
- **Committed in:** `87fd8f0`

**4. [Rule 1 - Bug] Stopped marking imported video-only template resources as audio-capable**
- **Found during:** Task 3
- **Issue:** The adapter inferred `hasAudio=true` for every video resource, causing FFmpeg export to reference missing `[0:a]` streams for video-only fixtures.
- **Fix:** Kaipai mapper now sets audio metadata only for explicit audio resources; mapper tests were updated accordingly.
- **Files modified:** `crates/adapter_kaipai/src/mapper.rs`, `crates/adapter_kaipai/tests/mapper.rs`
- **Verification:** `cargo test -p adapter_kaipai offline_mapper_maps_main_video_to_provider_neutral_import_plan -- --nocapture`, `pnpm run test:phase17`
- **Committed in:** `87fd8f0`

---

**Total deviations:** 4 auto-fixed (2 blocking, 2 bugs)
**Impact on plan:** All fixes were required to make the planned product UI import/export path work end to end. No UI-owned draft/render semantics or fallback path was added.

## Verification

All required commands passed:

- `pnpm --filter @video-editor/desktop exec playwright test tests/template-import.spec.ts --reporter=line`
- `pnpm run test:phase17`
- `pnpm run test:no-product-fallback`
- `cargo check --workspace --locked`
- `pnpm run test:contracts`

Additional focused check passed:

- `cargo test -p adapter_kaipai offline_mapper_maps_main_video_to_provider_neutral_import_plan -- --nocapture`

Notes:

- `pnpm` reported the existing engine warning because current Node is `v24.15.0` while `package.json` requests `24.12.0`.
- Rust builds reported the existing `objc2_av_foundation::AVAsset::tracksWithMediaType` deprecation warning.

## Known Stubs

None blocking. The template report's pre-import empty state is intentional product copy, and existing search/input placeholders in the feature panel were not introduced by this plan.

## Issues Encountered

- The macOS product E2E uses the packaged app through CDP, so a plain renderer build left the test on stale app assets until the package was refreshed.
- Product export surfaced two Rust correctness issues in imported template materials: bundle-relative path resolution and unsupported inferred audio metadata.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Desktop users can import offline template bundles through the normal product UI and see bounded adaptation report copy.
- Product E2E now proves report visibility, clean persistence, real preview evidence, export output, and no fallback success.
- Remaining future work should build on Rust-owned adapter/render semantics; no renderer fallback or draft-construction path was introduced.

## Self-Check: PASSED

- Key created file exists: `apps/desktop-electron/tests/template-import.spec.ts`.
- Key modified files exist across Electron main/preload/renderer, Rust adapter/binding, and root scripts.
- Task commits found: `8407605`, `2fc4cda`, `87fd8f0`.
- Plan verification commands passed.

---
*Phase: 17-template-import-core-and-kaipai-offline-adapter-foundation*
*Completed: 2026-06-24*

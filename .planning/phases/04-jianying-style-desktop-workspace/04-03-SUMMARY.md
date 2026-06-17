---
phase: 04-jianying-style-desktop-workspace
plan: 03
subsystem: desktop-ui
tags: [electron, react, typescript, timeline, command-contracts, jianying-workspace]

requires:
  - phase: 04-jianying-style-desktop-workspace
    provides: Chinese workspace shell, material/text/audio panels, inspector, command helpers, and accepted Rust response state
  - phase: 03-timeline-command-core
    provides: generated add/select/move/split/trim/delete/undo/redo timeline command contracts and TimelineCommandResponse
provides:
  - Fixed-row Chinese timeline visualization derived from accepted draft state
  - Timeline command helper builders for add/select/move/split/trim/delete/undo/redo
  - Deterministic MVP timeline controls wired through window.videoEditorCore.executeCommand
  - Stable timeline ruler, transport, track header, segment block, selected state, and playhead styling
affects: [phase-04-desktop-workspace, desktop-renderer, phase-04-playwright-gates]

tech-stack:
  added: []
  patterns:
    - Renderer timeline math derives display rows from generated Draft arrays without mutating semantic draft state
    - Timeline controls build generated command envelopes and replace accepted state only through TimelineCommandResponse
    - Phase 4 playhead state is renderer-only display state and not preview/render truth

key-files:
  created:
    - apps/desktop-electron/src/renderer/workspace/Timeline.tsx
  modified:
    - apps/desktop-electron/src/renderer/App.tsx
    - apps/desktop-electron/src/renderer/commandHelpers.ts
    - apps/desktop-electron/src/renderer/viewModel.ts
    - apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx
    - apps/desktop-electron/src/renderer/styles.css

key-decisions:
  - "Kept timeline display derivation in viewModel.ts as read-only calculations over accepted Draft.tracks and Track.segments."
  - "Implemented deterministic numeric/button timeline edits before pointer-drag behavior, matching the Phase 4 MVP boundary."
  - "Kept the playhead as renderer-only display state; preview frame truth remains deferred to Phase 5."
  - "Centralized add/select/move/split/trim/delete/undo/redo envelope construction in commandHelpers.ts."

patterns-established:
  - "Timeline.tsx owns fixed-row visualization and compact command controls while App.tsx owns executeCommand calls."
  - "Rejected timeline command results preserve prior draft, commandState, and selection through applyTimelineCommandResult."
  - "Hover and selected segment states use background, border, and inset outline changes that do not resize rows or blocks."

requirements-completed: [UI-04, UI-05, TEST-06]

duration: 10min
completed: 2026-06-17
---

# Phase 04 Plan 03: Timeline Interaction Surface Summary

**Fixed Jianying-style timeline rows with deterministic command-only edit controls backed by generated Rust timeline envelopes.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-06-17T10:41:58Z
- **Completed:** 2026-06-17T10:52:12Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added `Timeline.tsx` with a fixed transport strip, `28px` ruler, `160px` track headers, fixed video/text/filter rows, fixed audio rows, playhead display, and truncating Chinese segment labels.
- Added view-model timeline derivation helpers for rows, segment positions, ruler ticks, and `00:00:00.000` microsecond display formatting.
- Added generated command builders for `addSegment`, `selectTimelineSegments`, `moveSegment`, `splitSegment`, `trimSegment`, `deleteSegment`, `undoTimelineEdit`, and `redoTimelineEdit`.
- Wired deterministic controls for material-to-track add, segment select, numeric move, split, trim left/right, delete confirmation, undo, and redo through `window.videoEditorCore.executeCommand`.
- Preserved accepted state replacement through `TimelineCommandResponse` and kept rejected commands in Chinese error state without local retry or draft mutation.

## Task Commits

Each task was committed atomically:

1. **Task 1: Render fixed-row timeline from accepted draft state** - `120bb61` (feat)
2. **Task 2: Wire MVP timeline controls through generated commands** - `bc7ceb9` (feat)

## Files Created/Modified

- `apps/desktop-electron/src/renderer/workspace/Timeline.tsx` - Fixed-row timeline visualization, transport strip, playhead display, and deterministic timeline controls.
- `apps/desktop-electron/src/renderer/commandHelpers.ts` - Generated timeline command envelope builders and existing accepted response applier.
- `apps/desktop-electron/src/renderer/App.tsx` - Command-only timeline handlers and renderer-only playhead state.
- `apps/desktop-electron/src/renderer/viewModel.ts` - Timeline row, segment, ruler, and block-style display derivations.
- `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx` - Wires the dedicated `Timeline` component into the `时间线` region.
- `apps/desktop-electron/src/renderer/styles.css` - Stable timeline transport, ruler, track row, segment block, selected state, and playhead styling.

## Decisions Made

- Deterministic button/input controls are the Phase 4 editing surface; full pointer drag behavior remains a later enhancement over the same command path.
- The material selector in the timeline transport filters to available video, image, and audio materials so generic `addSegment` routes to compatible MVP tracks.
- Trim controls send explicit target timeranges to Rust and leave snapping, invalid edit rejection, and history updates to the command core.

## Deviations from Plan

None - plan executed exactly as written.

**Total deviations:** 0 auto-fixed.
**Impact on plan:** No scope expansion.

## Issues Encountered

- Existing generated `CommandEnvelope.ts` imports `TrimSegmentDirection` from `Draft.ts`, while the current generated `Draft.ts` does not visibly export that type. The planned Vite build still passed, and generated files were not edited.
- `KAIPAI_FORMULA_ADAPTER_RESEARCH.md` and `reference/` remained untracked external files and were left unstaged.

## Known Stubs

None blocking. The existing `preview-placeholder` CSS/copy remains the intentional Phase 4 monitor shell from Plan 04-01; preview rendering is still Phase 5 scope.

## Authentication Gates

None.

## Threat Flags

None. The plan introduced no renderer network endpoints, filesystem access, Electron/Node imports, FFmpeg construction, render graph generation, waveform path, preview cache path, auth path, or schema trust boundary outside the planned timeline command surface.

## Verification

- `pnpm --filter @video-editor/desktop build:electron` - PASS after Task 1.
- `pnpm --filter @video-editor/desktop build:electron` - PASS after Task 2.
- `pnpm --filter @video-editor/desktop build:electron` - PASS plan-level verification.
- Source check for required command helper names - PASS.
- Renderer source check for direct draft/timerange/main-track mutation patterns - PASS.
- Renderer source check for Electron/Node/FFmpeg/render graph/preview cache/waveform leakage - PASS.
- Generated contract drift check for `schemas` and `apps/desktop-electron/src/generated` - PASS.

## Self-Check: PASSED

- Created file exists: `apps/desktop-electron/src/renderer/workspace/Timeline.tsx`.
- Modified files exist: `App.tsx`, `commandHelpers.ts`, `viewModel.ts`, `WorkspaceShell.tsx`, and `styles.css`.
- Task commits `120bb61` and `bc7ceb9` exist in git history.
- Plan verification command passed.
- Stub scan found only intentional null checks and the existing Phase 4 preview placeholder.
- `reference/` and `KAIPAI_FORMULA_ADAPTER_RESEARCH.md` remain untracked and unstaged.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for Plan 04-04 to add Electron/Playwright proof for command-only timeline edits, Phase 4 editor flow, layout stability, and final source guards.

---
*Phase: 04-jianying-style-desktop-workspace*
*Completed: 2026-06-17*

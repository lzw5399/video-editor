# Jianying Pro Alignment Regression

This is the regression strategy for keeping the desktop UI aligned with the Jianying-style reference while still reducing feature scope.

## Visual Gates

- Capture our editor at `1280x800` and `1120x720` after major UI changes.
- Verify the five regions are visible, non-overlapping, and clipped only at intentional scroll containers.
- Verify production screenshots do not include developer diagnostics, FFmpeg/ffprobe labels, preview artifact paths, cache paths, or manual test path fields.
- Keep screenshot checks on the production-default UI. Tests that need artifact paths must opt into developer diagnostics explicitly.

Existing gates to extend:

- `apps/desktop-electron/tests/workspace.spec.ts`
- `apps/desktop-electron/tests/electron-smoke.spec.ts`
- `apps/desktop-electron/tests/runtime-diagnostics.spec.ts`

## Interaction Gates

- Top feature tabs switch panels without changing accepted draft state.
- Material import uses the system dialog by default; manual path import is developer diagnostics only.
- Preview controls route through generated command envelopes and update accepted preview status without exposing derived artifact paths in production.
- Export controls remain command-driven and should eventually move toward a modal/dropdown pattern matching the reference capture.
- Timeline buttons, ruler seeking, playhead drag, track mute, segment select/add/split/trim/delete, undo/redo, and zoom must remain stable across desktop viewport sizes.

## Source Guards

Add or extend guards so production UI fails tests when it exposes:

- `FFmpeg`, `ffprobe`, runtime probe details, raw diagnostics, artifact paths, cache paths, or filesystem debug paths in default renderer copy;
- renderer-owned FFmpeg/render graph/export script construction;
- direct renderer mutation of draft tracks, segments, timeranges, keyframes, visual/text/audio semantics, undo/redo, or snapping state.

Allowed exceptions:

- `commandHelpers.ts` may format runtime diagnostics for developer diagnostics and tests.
- runtime diagnostics tests may opt into `VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS=1`.
- production command errors may show user-safe failure copy, but not internal paths or raw probe output.

## Next UI Alignment Pass

1. Recapture the missing Jianying states with a manifest before using the screenshots as hard references.
2. Replace remaining demo-like editor controls with production controls that match the reference hierarchy.
3. Introduce modal/dropdown tests for export and draft parameter editing.
4. Add icon parity work: map existing compact controls to known symbols first, then add generated SVG only where no suitable symbol exists.
5. Run full workspace tests, source guards, and visual screenshot inspection before accepting the UI pass.


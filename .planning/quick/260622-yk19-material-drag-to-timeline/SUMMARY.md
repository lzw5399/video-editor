# Material Drag To Timeline Summary

## Result

Implemented material drag from the product media panel to the timeline track area. The renderer only transfers the imported `materialId` and the drop target calls the existing `addTimelineSegmentIntent` path, so Rust session/core still owns segment ID allocation, target track selection, timeranges, snapping, overlap handling, selection, and undo/redo.

The production architecture review subagent confirmed this is the correct boundary. If future drag placement needs drop-location behavior, it should be added as a new Rust-owned high-level intent rather than renderer-supplied track or timerange fields.

## Changed

- Added a shared renderer drag data type for material drags.
- Made available material rows draggable while keeping the existing add button as a secondary accessible command.
- Made the timeline track list a material drop target with product-state feedback.
- Added Playwright helper coverage for real drag-to-timeline.
- Switched the primary product playback gate setup to drag material into the timeline before asserting project-session intent ownership.

## Verification

- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback rejects missing render-graph GPU compositor evidence" --workers=1`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`

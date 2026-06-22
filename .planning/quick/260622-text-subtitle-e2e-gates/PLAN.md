---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Text And Subtitle E2E Gates

## Goal

Add broader product end-to-end validation for real native render-graph text and subtitle editing: multiple font families/styles, simultaneous subtitles, sequential subtitle cues, preview-side visual edits, and text content edits.

## Scope

- Extend native preview text overlay evidence with style and visual fields needed to prove edits reached the render graph.
- Add product E2E coverage using real packaged/native preview flow, real fixture media, and native host PNG pixel checks.
- Cover multiple active overlays at the same timeline time and different subtitle cues at different timeline times.
- Cover subtitle movement, rotation, font/style changes, and content editing via the inspector path.

## Verification

- Rust scheduler/bindings tests for evidence mapping.
- Packaged Electron product E2E text/subtitle gate.
- Existing combo preview, P0 portrait, and resize grow/shrink gates if touched.

# Combo Preview Evidence Summary

## Result

Strengthened the product combo playback gate so native realtime preview evidence reports the active render-graph text overlays. The combo UAT now proves playback includes video, external audio, product text, and two SRT cues on the native render-graph surface, with the active subtitle changing from the first cue to the second during playback.

## Changed

- Added `activeTextOverlays` to Rust realtime scheduler evidence and native preview presentation evidence.
- Threaded the evidence through the Node binding and Electron realtime preview host types.
- Added Rust coverage that scheduler presentation evidence includes active subtitle text from the render graph.
- Hardened the product combo Playwright gate to assert:
  - native audio preview command uses project session identity and no draft payload,
  - active render-graph text evidence includes product text and the current SRT cue,
  - SRT cue evidence changes from first cue to second cue during playback,
  - preview remains render-graph GPU composited with aligned native surface,
  - no DOM text overlay or artifact preview frame loop is accepted.

## Evidence

- Generated playback screenshots:
  - `test-results/phase15-3/combo-preview-first-subtitle.png`
  - `test-results/phase15-3/combo-preview-second-subtitle.png`

## Verification

- `cargo test -p bindings_node scheduler_playback_presents_render_graph_gpu_evidence_after_draft_and_surface_ready -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback UAT composites video external audio text and two-cue SRT on the native surface" --workers=1`
- `cargo fmt --all --check`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`

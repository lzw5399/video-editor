---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Combo Preview Evidence Gate

## Goal

Strengthen the product playback UAT for video + external audio + text + two-cue SRT so it proves native render-graph preview evidence includes the active text/subtitle overlays and subtitle content changes over playback time.

## Scope

- Add non-UI realtime preview presentation evidence for active text/subtitle overlays from the Rust render graph.
- Thread that evidence through bindings and Electron host state.
- Harden the combo product E2E to assert native audio, no fallback, no DOM text overlay, surface placement, and subtitle cue change through presentation evidence.

## Verification

- Passed: `cargo test -p bindings_node scheduler_playback_presents_render_graph_gpu_evidence_after_draft_and_surface_ready -- --nocapture`
- Passed: `corepack pnpm --dir apps/desktop-electron run build:electron`
- Passed: `corepack pnpm --dir apps/desktop-electron run package:dir`
- Passed: `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product playback UAT composites video external audio text and two-cue SRT on the native surface" --workers=1`
- Passed: `cargo fmt --all --check`
- Passed: `corepack pnpm -w run test:phase3-source-guards`
- Passed: `git diff --check`

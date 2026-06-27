---
phase: 20-long-timeline-product-uat-and-guard-baseline
reviewed: 2026-06-28T05:20:00+08:00
depth: standard
files_reviewed: 14
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
resolved_findings: 5
status: clean
---

# Phase 20: Code Review Closeout

**Status:** clean after remediation

## Resolved Findings

| ID | Prior Severity | Resolution |
|----|----------------|------------|
| CR-01 | Critical | Required-token guards now strip comment-only matches and include self-tests proving comment-only required markers fail. |
| CR-02 | Critical | Pressure UAT now starts a real export job, samples active job state during playback/scrub/edit/commit/cancel, waits for completion, and validates exported media. |
| WR-01 | Warning | Phase 20 TypeScript draft-construction guard now catches `Array.from`, loop/push, and helper-builder long draft construction patterns with self-tests. |
| WR-02 | Warning | Rust canonical project check now recursively rejects derived/runtime/export/cache fields anywhere in `project.json`. |
| IN-01 | Info | Export file evidence now records file size, distinguishing missing output from zero-byte/truncated output. |

## Additional Closeout Fix

The strengthened pressure UAT exposed a real FFmpeg export failure for the 180-segment product fixture. The compiler now keeps the render graph in Rust and optimizes only the safe sequential full-canvas subset:

- video layers: contiguous single-stack full-canvas segments compile as `concat -> single fit/scale -> subtitles`, avoiding 180 parallel `scale` filters while preserving 1920x1080 output;
- text overlays: same-font-directory overlays compile into one ASS sidecar with per-overlay styles/dialogues;
- audio mixes: contiguous unity-retime audio segments concat per track before a small cross-track `amix`;
- fallback path: complex transforms, effects, masks, blends, transitions, keyframes, non-contiguous ranges, and automation stay on the existing explicit filter path.

## Verification

Passed:

- `cargo fmt --check`
- `cargo test -p ffmpeg_compiler --tests -- --nocapture`
- `cargo test -p testkit --test long_timeline_product_fixture -- --nocapture`
- `pnpm --filter @video-editor/desktop build`
- `pnpm --filter @video-editor/desktop package:dir && pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts -g "pressure" --reporter=line --workers=1`
- `pnpm run test:phase20`

Notes:

- `pnpm run test:phase20` passed all three packaged Phase 20 UATs, `cargo check --workspace --locked`, and generated-contract consistency.
- Warnings observed are pre-existing: Node engine warning (`24.12.0` wanted, `24.15.0` used), macOS AVFoundation deprecation warning, and existing unused Rust helper warnings in `bindings_node`.

# Preview Telemetry Subscription Boundary Summary

Status: complete

## Changes

- Removed the renderer-visible realtime preview `getTelemetry` polling API from preload.
- Added a main-process telemetry subscription/fanout channel for preview host state snapshots.
- Replaced `PreviewMonitor` renderer polling with `subscribeTelemetry`.
- Updated product workflow helpers and cadence tests to observe host state through subscriptions.
- Added source guards preventing renderer telemetry polling and preload `getTelemetry` re-exposure.
- Renamed display-only duration input state in `FeaturePanel` so phase11 guards continue to reserve bare seconds names for forbidden request/schema contracts.

## Verification

- `cargo test -p media_runtime discovery -- --nocapture`
- `cargo fmt --all --check`
- `corepack pnpm run test:phase6-release-gates`
- `corepack pnpm --dir apps/desktop-electron run provision:ffmpeg-runtime`
- `corepack pnpm run test:phase11-source-guards`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase15-3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `git diff --check`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --grep "native preview host bridge|fallback source guard" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --reporter=line`

## Evidence

`product-preview-cadence.spec.ts` passed both single-video and video+external-audio+text+two-cue-SRT scenarios with 90 accounted frames over the 3s window, no dropped frames, no artifact fallback, and real `renderGraphGpuComposited` evidence.

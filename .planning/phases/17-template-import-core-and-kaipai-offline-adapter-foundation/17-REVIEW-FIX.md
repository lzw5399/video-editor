---
phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
review: 17-REVIEW.md
fixed: 2026-06-24T11:53:22Z
status: fixed
---

# Phase 17 Code Review Fix Report

## Fixed Issues

- CR-01: Reworked the Kaipai mapper so supported formula sections are mapped cumulatively instead of through a single fixture-family branch. Mixed templates now preserve main video, PIP, text sticker, BGM, and native-effect diagnostics in one import plan.
- CR-02: Tightened the FFmpeg compiler full-canvas identity fast path so rotated full-canvas layers still compile through the generic rotation filter path.
- WR-01: Added scoped cleanup for files copied during a failed Kaipai import persistence transaction, limited to available localized refs under `resources/template-import/...`.

## Regression Coverage

- Added a mixed Kaipai mapper regression test for video + PIP + text + BGM + native-effect evidence.
- Added a full-canvas rotation compiler regression test that requires `rotate=` with expanded rotated bounds.
- Added a project-session import failure test that forces resource-index persistence failure and verifies copied resources are removed.
- Updated report snapshots and product E2E expectations so supported base video/canvas evidence is reported alongside native-effect degradation.

## Verification

- `cargo test -p adapter_kaipai offline_mapper -- --nocapture`
- `cargo test -p ffmpeg_compiler transform -- --nocapture`
- `cargo test -p bindings_node project_session_import_kaipai -- --nocapture`
- `pnpm run test:phase17-source-guards`
- `pnpm run test:phase17-desktop`
- `pnpm run test:phase17`

Known non-blocking warnings observed during verification:

- Node engine warning: project expects `24.12.0`, current runtime is `v24.15.0`.
- Existing macOS AVFoundation deprecation warning in `crates/media_runtime_desktop/src/platform/macos.rs`.

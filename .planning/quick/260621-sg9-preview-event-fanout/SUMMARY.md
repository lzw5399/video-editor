---
status: complete
completed_at: "2026-06-21T16:15:49Z"
---

# Preview Event Fanout Summary

Removed the Electron main-process realtime preview telemetry interval. Rust `bindings_node` now exposes a Node-API realtime preview event subscription, and Electron main fans out telemetry from Rust native playback/control events instead of polling on a timer.

## Changes

- Added `subscribeRealtimePreviewEvents` / `unsubscribeRealtimePreviewEvents` to the native binding.
- Emitted Rust preview events for session lifecycle, control changes, presented frames, playback end, and playback errors.
- Replaced `RealtimePreviewHost` telemetry `setInterval` fanout with native-event-triggered fanout.
- Unrefed the N-API threadsafe callback so event subscription does not keep Electron alive during teardown.
- Throttled presentation evidence snapshot refresh during frame events while still pushing per-frame telemetry and target-time updates.
- Added source guards preventing the main preview host from reintroducing telemetry interval fanout.
- Updated product cadence tests to require native preview event evidence in addition to the 3s / 90-frame gate.

## Verification

- `cargo fmt --all --check`
- `cargo check -p bindings_node`
- `cargo test -p bindings_node realtime_preview -- --nocapture`
- `corepack pnpm run test:phase11-source-guards`
- `corepack pnpm run test:phase15-3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/electron-smoke.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "native preview host bridge keeps handles" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/packaged-smoke.spec.ts --reporter=line`

Cadence evidence: single-video and video + external-audio + text + two-cue-SRT product preview both presented 90/90 frames with 0 drops, `targetDeltaMicroseconds=2966637`, native `framePresented=90`, and Electron presentation snapshot reads reduced to 13 over the 3s window.

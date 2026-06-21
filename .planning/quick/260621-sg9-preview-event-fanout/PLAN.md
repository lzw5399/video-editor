# Preview Event Fanout

## Goal

Remove the Electron main-process realtime preview telemetry interval. Rust preview playback already owns the media clock and presentation worker; Electron should receive playback/presentation notifications from Rust and fan out subscribed telemetry state, not poll `getRealtimePreviewPresentationState` on a timer.

## Scope

- Add a Node-API subscription endpoint for realtime preview events using a thread-safe callback.
- Emit Rust binding events when playback presents a frame, hits an unsupported presentation error, reaches sequence end, or control state changes.
- Replace `RealtimePreviewHost` `setInterval` fanout with event-triggered refresh/fanout.
- Keep explicit command responses for attach, snapshot, seek, play, pause, stop.
- Add tests/guards that fail if the main preview host reintroduces interval-based telemetry fanout.

## Verification

- `cargo fmt --all --check`
- `cargo test -p bindings_node realtime_preview -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --reporter=line`
- Source guard rejecting realtime preview host `setInterval` fanout.

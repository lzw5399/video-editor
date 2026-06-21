# Preview Telemetry Subscription Boundary

## Goal

Remove renderer-owned realtime preview telemetry polling so Electron UI no longer looks like a playback cadence participant. Realtime preview playback remains Rust-owned; Electron should issue control commands and subscribe to host telemetry snapshots.

## Scope

- Replace `PreviewMonitor`'s renderer `setInterval(...getTelemetry...)` loop with a subscription-style bridge contract.
- Move telemetry refresh cadence into Electron main/host as status fanout only, not frame pump or presentation work.
- Remove renderer-visible `getTelemetry`; tests observe telemetry through the same subscription bridge while main keeps internal snapshot refresh.
- Add a source guard that rejects renderer realtime-preview polling intervals.
- Update focused tests/types for the new subscribe/unsubscribe bridge shape.

## Verification

- `cargo fmt --all --check`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase11-source-guards`
- `corepack pnpm run test:phase15-3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts --grep \"native preview host bridge|fallback source guard\" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --reporter=line`

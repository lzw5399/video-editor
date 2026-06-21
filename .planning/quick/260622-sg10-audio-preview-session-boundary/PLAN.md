# Audio Preview Session Boundary

## Goal

Move product audio preview off renderer-owned `Draft` payloads. Audio preview commands should identify the Rust project session and expected revision, then Rust should read the canonical session draft internally before creating playback sessions, seeking, waveform reads, or status refreshes.

## Scope

- Extend the audio preview binding request contract to accept `projectSessionId` and `expectedRevision`.
- Route audio preview draft access through `project_session_service::project_session_snapshot`.
- Update Electron renderer/main/native binding types so product audio preview sends session identity instead of `draft`.
- Keep playback controls as control commands, not timeline semantic commands.
- Add source guards rejecting renderer-owned audio preview `draft` payloads in product code.
- Preserve existing native audio playback and waveform gates.

## Verification

- `cargo fmt --all --check`
- `cargo test -p bindings_node audio_service -- --nocapture`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:native`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm run test:phase11-source-guards`
- `corepack pnpm run test:phase15-3-source-guards`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`

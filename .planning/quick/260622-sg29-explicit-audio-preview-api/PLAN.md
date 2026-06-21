# Quick Task: 260622-sg29 Explicit Audio Preview API

## Objective

Remove the renderer's generic `CommandEnvelope` construction from product audio preview, device, and waveform controls. Audio preview is part of the playback main path, so Electron should call explicit native APIs and Rust should keep project-session snapshot/revision validation inside the audio service.

## Production Boundary

- Renderer must not construct audio preview command envelopes.
- Electron preload/main should expose explicit audio preview APIs.
- Rust binding should expose explicit audio preview entry points that call `audio_service` directly, preserving project-session identity, revision checks, playback generation, stale rejection, device selection, and waveform display contracts.
- Generic `executeCommand` may remain temporarily for non-product compatibility/artifact containment, but product audio preview must leave that path.

## Work Items

1. Add explicit Rust Node-API functions for create/play/pause/stop/seek/cancel/status/device/waveform audio preview actions.
2. Add typed nativeBinding/preload/main wrappers and test observation support for explicit audio calls.
3. Update renderer audio preview handlers to call explicit APIs instead of `executeCommand`.
4. Remove audio preview command builders/imports from renderer command helpers.
5. Add source guards/tests preventing renderer audio preview from returning to `build*AudioPreview*Command`, `build*Waveform*Command`, or generic audio `executeCommand`.

## Verification

- `cargo test -p bindings_node audio_service`
- `cargo test -p bindings_node project_session audio_preview_commands_use_project_session_snapshot_without_renderer_draft`
- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm run test:phase15-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- Focused Playwright audio preview/product playback tests if build-level changes require UI proof.
- `cargo fmt --all --check`
- `git diff --check`

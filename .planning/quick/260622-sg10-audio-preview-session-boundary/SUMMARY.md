# Audio Preview Session Boundary Summary

## Outcome

Completed. Product audio preview commands no longer carry renderer-owned `Draft` payloads. Renderer/main now send `projectSessionId` and `expectedRevision`; Rust validates project-session ownership and reads the canonical draft snapshot from `project_session_service` before create/play/waveform/device operations.

## Changes

- Removed `draft` from `AudioPreviewCommandPayload` and regenerated command contracts.
- Routed audio preview create/play/waveform/device handling through Rust project session snapshots.
- Added Rust-owned demo fixture session creation for startup/demo flows.
- Updated Electron renderer, preload, main test observation, and command helpers to pass session identity instead of draft payloads.
- Added source guards rejecting audio preview draft payloads and requiring session identity fields.
- Fixed realtime preview failure-state fanout so missing-compositor rejection updates subscribed product telemetry state.
- Tightened product journey/audio assertions while removing a duplicate visible-pixel sampling race.

## Verification

- `VE_UPDATE_GENERATED_CONTRACTS=1 cargo test -p draft_model schema_exports -- --nocapture`
- `cargo fmt --all --check`
- `cargo test -p bindings_node audio_service -- --nocapture`
- `cargo test -p bindings_node --test project_session audio_preview_commands_use_project_session_snapshot_without_renderer_draft -- --nocapture`
- `cargo test -p media_runtime discovery -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/workspace.spec.ts -g "音频预览|波形" --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --reporter=line`
- `corepack pnpm --filter @video-editor/desktop test:packaged-smoke`
- `corepack pnpm run test:phase6-release-gates`
- `corepack pnpm run test:phase11-source-guards`
- `corepack pnpm run test:phase15-source-guards`
- `corepack pnpm run test:phase15-3-source-guards`

## Notes

- Product preview cadence gate reported 90 presented frames over the 3 second windows for both single-video and video+external-audio+text+SRT scenarios, with 0 dropped frames.
- Packaged FFmpeg/ffprobe resolution remains bundled-only; packaged smoke and phase6 release guards passed with external runtime overrides ignored.
- Existing warning remains: `AVAsset::tracksWithMediaType` is deprecated in `media_runtime_desktop`.

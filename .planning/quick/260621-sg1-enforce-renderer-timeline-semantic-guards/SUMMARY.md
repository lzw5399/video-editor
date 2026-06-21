---
status: complete
completed: 2026-06-21
---

# Summary

Moved product track mutation commands from renderer-owned `trackId` payloads to Rust session selected-track intents. The UI now selects a track as interaction state, then asks Rust to rename, lock, hide, or mute the selected track.

Strengthened `scripts/phase3-source-guards.sh` with construction-site scoped guards for low-level timeline edit command/payload construction, multiline semantic-field detection, renderer submodule command-dispatch bypass checks, and negative self-tests for bad add/trim/track mutation payloads.

Added test observation for `timelineSemanticKeys` and product journey assertions that Rust session edit intents do not carry renderer-owned segment/track/timerange semantic keys.

Verification:

- `corepack pnpm run test:phase3-source-guards`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "product user editing matrix uses real commands" --reporter=line`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `cargo test -p bindings_node --test binding_smoke execute_command_rejects_public_timeline_edit_commands -- --nocapture`

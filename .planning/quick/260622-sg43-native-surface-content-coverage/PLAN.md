---
status: complete
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Native Surface Content Coverage

## Goal

Make the realtime preview placement proof production-grade by verifying rendered WGPU content coverage during playback, not just native child-view frame alignment.

## Scope

- Audit and fix macOS WGPU surface sizing if the `CAMetalLayer` drawable size and WGPU surface configuration use different pixel spaces.
- Expose runtime placement geometry needed to diagnose native surface placement/content issues.
- Add a product playback gate that captures playing-state evidence and fails if rendered video is only present in a lower-left subregion while telemetry claims alignment.

## Verification

- `cargo fmt --all --check`
- `cargo test -p realtime_preview_runtime -- --nocapture`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "native surface aligned|composites video external audio text" --workers=1`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --workers=1`
- `git diff --check`

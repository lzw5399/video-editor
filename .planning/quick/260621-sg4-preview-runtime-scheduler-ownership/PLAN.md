# Preview Runtime Scheduler Ownership

## Goal

Move realtime preview playback timing ownership out of `bindings_node` and into `realtime_preview_runtime`, so the binding layer remains a native presentation adapter rather than the owner of frame cadence, drop/skip policy, or fixed 30fps timing.

## Scope

- Add runtime-owned playback cadence primitives derived from rational frame rate and playback rate.
- Replace binding-owned `PLAYBACK_FRAME_DURATION_US`, due-tick structs, anchor math, and frame advancement with runtime scheduler types.
- Keep native GPU/AppKit presenter resources in `bindings_node` for this slice; do not move platform handles across crate boundaries yet.
- Add tests for 24fps, 29.97fps, 30fps, non-1x playback rate, stale generation, and late-frame drop accounting.
- Add a source guard that prevents reintroducing binding-owned playback timing policy.

## Verification

- `cargo fmt --all --check`
- `cargo test -p realtime_preview_runtime scheduler -- --nocapture`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `cargo test -p bindings_node realtime_preview_bindings -- --nocapture`
- `corepack pnpm run test:phase3-source-guards`

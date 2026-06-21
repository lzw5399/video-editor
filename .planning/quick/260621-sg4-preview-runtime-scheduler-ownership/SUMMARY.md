---
status: complete
---

# Preview Runtime Scheduler Ownership Summary

## Result

Moved realtime preview playback timing policy out of `bindings_node` and into `realtime_preview_runtime`. The binding layer still owns native GPU/AppKit/resource adapter execution for this slice, but it no longer defines fixed frame cadence, late-frame selection, drop accounting, or in-flight presentation policy.

## Changes

- Added runtime scheduler cadence primitives derived from `RationalFrameRate` and `PlaybackRate`.
- Added runtime-owned playback timeline state for prewarm, due-frame selection, stale-generation rejection, late-frame skip/drop accounting, and post-present advancement.
- Added runtime-owned presentation queue policy for max in-flight surface presentations and fence backpressure timeout.
- Replaced binding-local playback anchor/due tick/frame structs and fixed `33_333us` frame duration with runtime scheduler contracts.
- Added a Phase 3 source guard that fails if binding-owned playback cadence/drop/backpressure policy is reintroduced.
- Moved late-frame skip coverage from binding internals to runtime scheduler tests and added rational frame-rate, non-1x playback-rate, generation, and queue-policy coverage.

## Verification

- `cargo fmt --all --check`
- `cargo test -p realtime_preview_runtime scheduler -- --nocapture`
- `cargo test -p bindings_node realtime_preview_bindings -- --nocapture`
- `cargo test -p bindings_node --test project_session -- --nocapture`
- `corepack pnpm run test:phase3-source-guards`

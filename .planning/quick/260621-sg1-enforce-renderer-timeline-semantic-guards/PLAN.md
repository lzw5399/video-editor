---
status: complete
created: 2026-06-21
skill: gsd-quick
---

# Enforce Renderer Timeline Semantic Source Guards

## Goal

Make the Phase 3 source guard fail when renderer/main/preload product code adds timeline semantic construction to the UI boundary, specifically low-level segment/track IDs and source/target timerange payload construction for add/move/trim-style editing commands.

## Production Constraint

Rust session/core owns timeline edit semantics. UI may display draft/view-model data and forward user intent, but it must not construct authoritative segment IDs, track IDs, source timeranges, target timeranges, or legacy timeline command payloads in product paths.

## Verification

- `corepack pnpm run test:phase3-source-guards`
- Targeted negative fixture or script self-test proving a newly added renderer product semantic payload would fail.

---
status: completed
created: 2026-06-22
skill: gsd-quick
review_skill: production-architecture-review
---

# Resize Grow/Shrink Preview Gate

## Goal

Make resize regression coverage explicit at both product UAT and Rust preview-service levels: playback resize must be tested while growing the window and then shrinking it.

## Scope

- Keep the existing product Playwright gate that resizes 1120x720 -> 1500x900 -> 1120x720 during playback.
- Extend the Rust scheduler resize unit test so the service contract also covers both grow and shrink transitions.
- Preserve the production contract: resize updates surface bounds only; it must not restart playback, advance generation, resync the draft, or stop frame presentation.

## Verification

- Focused Rust test for scheduler resize during playback.
- Focused Playwright product resize test if native app launch is available.
- `git diff --check`.

---
status: complete
created: 2026-06-23
completed: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# Titlebar Jianying Chrome

## Goal

Move the product titlebar closer to the Jianying Pro reference while preserving the current product architecture boundaries.

## Production Decision

Confirmed narrow UI fix: the titlebar is product chrome owned by the Electron renderer. Updating visible save/status chrome and layout does not affect Rust-owned draft/timeline/render semantics as long as it remains display-only and does not introduce renderer-owned project state.

## Scope

- Compare current `workspace-1280x800.png` against `02-workspace-media-window.png`.
- Replace the single cyan-dot `本地草稿` titlebar status with Jianying-like macOS window dots plus local autosave status copy.
- Keep the real draft name centered and the export action top-right.
- Keep production mode free of diagnostic/runtime/cache/fallback copy.
- Add UI regression assertions so the titlebar chrome remains visible and unclipped at 1280x800 and 1120x720.

## Verification

- `build:electron`
- UI reference regression and refreshed static/narrow screenshots.
- Packaged playing native-surface regression to refresh playback screenshot evidence.
- `test:phase3-source-guards`
- `git diff --check`

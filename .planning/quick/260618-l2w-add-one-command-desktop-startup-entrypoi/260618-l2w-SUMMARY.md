---
status: complete
completed: 2026-06-18
---

# Quick Task 260618-l2w Summary

Added `pnpm start` as the primary one-command desktop startup entrypoint and `just start` as a matching `just desktop` alias.

Updated English and Chinese README quick-start sections so the first run path opens the Electron desktop editor instead of only starting a development server.

Verification:

- `pnpm run` lists `start` and `desktop`.
- `pnpm --filter @video-editor/desktop build` passed.
- `just --list` could not be run because `just` is not installed in the current shell.

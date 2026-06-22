---
status: in_progress
created: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# UI Reference Material Thumbnails

## Goal

Make product UI reference screenshots show real material thumbnails in the media bin instead of black placeholder cards, matching the Jianying reference material library more closely without faking product preview/render success.

## Scope

- Keep the renderer boundary unchanged: material cards may only show images from project-relative `thumbnailRef` display refs.
- Bind artifact status requests to the Rust project session, so the artifact service reads the canonical draft material list rather than a standalone resource-panel session.
- Add a production artifact status read model that returns per-material thumbnail statuses and `displayRef` values from `.veproj/derived` ready artifacts.
- Make refresh generate missing visual thumbnails through the bundled FFmpeg runtime and persist them through the existing artifact store blob/generation path.
- Remove the UI-reference artifact command mock from this path and add assertions that material cards render real `<img>` thumbnails rather than fallback text.

## Verification

- UI reference regression Playwright test.
- Screenshot inspection for `material-library-1280x800.png`, `workspace-1280x800.png`, `workspace-playing-1280x800.png`, and `workspace-1120x720.png`.
- `build:electron`.
- `git diff --check`.

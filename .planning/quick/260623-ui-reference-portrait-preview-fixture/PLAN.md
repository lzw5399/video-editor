---
status: complete
created: 2026-06-23
skill: gsd-quick
review_skill: production-architecture-review
---

# UI Reference Portrait Preview Fixture

## Goal

Make UI reference workspace screenshots closer to the Jianying Pro reference by using a healthy portrait video fixture for the preview canvas, while keeping external audio and image materials available in the material bin.

## Scope

- Switch the UI reference video fixture from the 16:9 moving test source to the committed portrait test source.
- Update selectors and expectations that reference the video filename.
- Refresh 1280x800 and 1120x720 screenshots through the native first-frame screen capture path.

## Verification

- UI reference regression Playwright test.
- Inspect refreshed workspace/preview screenshots.
- `build:electron`.
- `git diff --check`.

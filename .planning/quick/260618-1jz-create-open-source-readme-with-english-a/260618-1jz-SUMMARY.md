---
status: complete
task_id: 260618-1jz
slug: create-open-source-readme-with-english-a
completed: 2026-06-18
---

# Summary

Created the open-source README pair for Video Editor:

- `README.md` is the primary English landing page.
- `README.zh-CN.md` is the Chinese version.
- Both files include a language switch, layered architecture explanation, current implementation status, repository layout, quick start commands, project format boundaries, development constraints, compatibility adapter direction, and license notes.

The README now references `docs/assets/architecture.png` as the stable architecture diagram path. The image file itself was intentionally left for later replacement by the user.

## Verification

- Checked root `package.json`, `Cargo.toml`, `justfile`, `docs/runtime-boundaries.md`, and `.planning/STATE.md` before writing.
- Kept the adapter description separate from the main editing path: external Jianying/CapCut drafts map into `.veproj/project.json` plus a compatibility report, then enter the normal core flow.
- Did not modify or revert unrelated dirty source files.

---
quick_id: 260618-jwf
status: planned
created: 2026-06-18
---

# Quick Task 260618-jwf: 新增一个一键启动 Electron 桌面端编辑器的命令

## Scope

Provide a single root command that installs locked dependencies if needed, builds the desktop Electron app, and launches the built desktop editor.

## Tasks

1. Add a root `pnpm run desktop` script.
2. Add a matching `just desktop` recipe for users who have `just` installed.
3. Verify the desktop build and bounded Electron launch.


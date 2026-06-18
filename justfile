# Prerequisite: install `just` before using these root entrypoints.
# If missing locally, run `cargo install just --locked`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
  @printf 'Available recipes:\n  start\n  desktop\n  dev\n  build\n  test\n'

start: desktop

dev:
  pnpm run dev

desktop:
  pnpm install --frozen-lockfile
  pnpm --filter @video-editor/desktop build
  pnpm --filter @video-editor/desktop exec electron dist/main/index.cjs

build:
  pnpm install --frozen-lockfile
  pnpm run build:rust
  pnpm --filter @video-editor/desktop build

test:
  pnpm install --frozen-lockfile
  pnpm run test:rust
  pnpm run test:schema
  pnpm run test:draft-fixtures
  pnpm run test:project-store
  pnpm run test:runtime
  pnpm run test:material-probe
  pnpm run test:material-service
  pnpm run test:bindings
  pnpm run test:desktop
  pnpm run test:render-smoke
  pnpm run test:phase2-source-guards
  pnpm run test:phase3-commands
  pnpm run test:phase3-source-guards
  pnpm run test:phase4-source-guards
  pnpm run test:phase4-workspace
  pnpm run test:phase5-render-core
  pnpm run test:phase5-source-guards
  pnpm run test:phase5-workspace
  pnpm run test:phase6
  pnpm run test:phase7
  pnpm run test:phase8
  pnpm run test:phase9
  pnpm run test:phase10
  pnpm run test:contracts

test-phase6-packaging:
  pnpm install --frozen-lockfile
  pnpm run test:phase6-packaging

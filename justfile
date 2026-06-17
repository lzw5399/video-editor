# Prerequisite: install `just` before using these root entrypoints.
# If missing locally, run `cargo install just --locked`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
  @printf 'Available recipes:\n  dev\n  build\n  test\n'

dev:
  pnpm run dev

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
  pnpm run test:contracts

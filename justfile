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
  cargo test -p draft_model schema -- --nocapture
  cargo test -p media_runtime discovery -- --nocapture
  cargo test -p bindings_node -- --nocapture
  pnpm --filter @video-editor/desktop test
  cargo test -p testkit render_smoke -- --nocapture
  git diff --exit-code schemas apps/desktop-electron/src/generated

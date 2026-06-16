# Prerequisite: install `just` before using these root entrypoints.
# If missing locally, run `cargo install just --locked`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

default:
  @printf 'Available recipes:\n  dev\n  build\n  test\n'

dev:
  pnpm run dev

build:
  pnpm install --frozen-lockfile
  pnpm run build

test:
  pnpm install --frozen-lockfile
  pnpm run test

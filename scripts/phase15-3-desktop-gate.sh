#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DESKTOP_DIR="${ROOT_DIR}/apps/desktop-electron"
COREPACK_SHIMS="$(mktemp -d)"

cleanup() {
  rm -rf "${COREPACK_SHIMS}"
}
trap cleanup EXIT

corepack enable --install-directory "${COREPACK_SHIMS}" >/dev/null
export PATH="${COREPACK_SHIMS}:${PATH}"

run_desktop() {
  corepack pnpm --dir "${DESKTOP_DIR}" "$@"
}

run_desktop run clean:build
run_desktop run build:native
run_desktop exec vite build --mode main
run_desktop exec vite build --mode preload
run_desktop exec vite build
run_desktop exec electron-builder --dir --config electron-builder.yml --publish=never
run_desktop exec playwright test \
  tests/project-entry.spec.ts \
  tests/export-modal.spec.ts \
  tests/inspector-modal.spec.ts \
  tests/ui-reference-regression.spec.ts \
  tests/workspace.spec.ts \
  tests/product-user-journey.spec.ts \
  tests/real-workflow.spec.ts \
  --reporter=line

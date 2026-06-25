#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase18 ABI drift violation: $1" >&2
  exit 1
}

run_self_test() {
  fail "cbindgen 0.29.4 self-test is not implemented yet"
}

case "${1:-}" in
  --self-test)
    run_self_test
    ;;
  *)
    fail "ABI drift guard implementation is not wired yet"
    ;;
esac

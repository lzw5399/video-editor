#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase18 mobile runtime contract violation: $1" >&2
  exit 1
}

run_self_test() {
  fail "mobile runtime contract self-test is not implemented yet"
}

case "${1:-}" in
  --self-test)
    run_self_test
    ;;
  *)
    fail "mobile runtime contract guard implementation is not wired yet"
    ;;
esac

#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase18 source guard violation: $1" >&2
  exit 1
}

strip_comments() {
  cat
}

matches_for_pattern() {
  local _pattern="$1"
  shift
  return 0
}

assert_pattern_rejects() {
  local description="$1"
  local pattern="$2"
  local source="$3"
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' RETURN
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase18Violation.txt"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase18Violation.txt" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "// $source" >"$tmp_dir/CommentOnly.txt"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.txt" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

BINDINGS_C_NODE_DEP_PATTERN='bindings_node'
SERVER_ELECTRON_DEP_PATTERN='BrowserWindow|preload|apps/desktop-electron'
ELECTRON_RENDER_EXPORT_PATTERN='buildRenderGraph|compileFfmpeg|startExportJob'
FALLBACK_SUCCESS_PATTERN='mockPreviewSuccess|artifactPreviewSuccess|cpuReadbackPreviewSuccess|domOverlayPreviewSuccess'
ADAPTER_LIFETIME_POLICY_PATTERN='HashMap<.*HandleToken|next_generation|release_handle'

run_self_test() {
  assert_pattern_rejects \
    "bindings_c dependency on bindings_node" \
    "$BINDINGS_C_NODE_DEP_PATTERN" \
    'use bindings_node::open_project_session;'
  assert_pattern_rejects \
    "server runtime dependency on Electron desktop source" \
    "$SERVER_ELECTRON_DEP_PATTERN" \
    'import { BrowserWindow } from "electron";'
  assert_pattern_rejects \
    "Electron-owned render/export behavior" \
    "$ELECTRON_RENDER_EXPORT_PATTERN" \
    'const graph = buildRenderGraph(draft); compileFfmpeg(graph); startExportJob(graph);'
  assert_pattern_rejects \
    "fallback/mock/artifact success evidence" \
    "$FALLBACK_SUCCESS_PATTERN" \
    'const artifactPreviewSuccess = true;'
  assert_pattern_rejects \
    "adapter-owned lifetime policy" \
    "$ADAPTER_LIFETIME_POLICY_PATTERN" \
    'const handles: HashMap<HandleToken, Resource> = new Map(); let next_generation = 1; release_handle(token);'
}

case "${1:-}" in
  --self-test)
    run_self_test
    ;;
  *)
    fail "phase18 source guard implementation is not wired yet"
    ;;
esac

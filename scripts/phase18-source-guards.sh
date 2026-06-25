#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase18 source guard violation: $1" >&2
  exit 1
}

strip_comments() {
  rg -v '^[^:]+:[0-9]+:[[:space:]]*(//|/\*|\*|#)' \
    | rg -v '^[0-9]+:[[:space:]]*(//|/\*|\*|#)' \
    | rg -v '^\s*(//|/\*|\*|#)' \
    || true
}

matches_for_pattern() {
  local pattern="$1"
  shift
  rg -n --pcre2 "$pattern" "$@" 2>/dev/null | strip_comments
}

require_file() {
  local file="$1"
  [ -f "$file" ] || fail "missing required Phase 18 artifact ${file}"
}

require_fixed() {
  local file="$1"
  local text="$2"
  if ! rg -n --fixed-strings "$text" "$file" >/dev/null; then
    fail "missing required text '${text}' in ${file}"
  fi
}

fail_matches() {
  local message="$1"
  local pattern="$2"
  shift 2
  local matches
  matches="$(matches_for_pattern "$pattern" "$@" || true)"
  if [ -n "$matches" ]; then
    printf '%s\n' "$matches" >&2
    fail "$message"
  fi
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

BINDINGS_C_NODE_DEP_PATTERN='(?:bindings_node::|extern\s+crate\s+bindings_node|\bbindings_node\s*=|\bbindings_node\b[[:space:]]*\{|\bbindings_node\b[[:space:]]*\()'
SERVER_ELECTRON_DEP_PATTERN='\b(?:electron|BrowserWindow|ipcMain|ipcRenderer|WebContents|preload|document\.|window\.|HTMLElement|apps/desktop-electron|@video-editor/desktop|nativeBinding)\b'
ELECTRON_RENDER_EXPORT_PATTERN='\b(?:buildRenderGraph|compileRenderGraph|compileFfmpeg|compileFfmpegJob|renderGraphToFfmpeg|filter_complex|filterComplex|ffmpegArgs|ffmpegCommand|spawn\s*\(\s*["'\'']ffmpeg|new\s+RenderGraph|new\s+Ffmpeg)\b'
FALLBACK_SUCCESS_PATTERN='\b(?:(?:mock|artifact|cpuReadback|cpuProbe|decodedCpu|domOverlay|debug|legacy)(?:Preview|Render|Export|Playback)?Success|(?:fallback|mock|artifact|cpu|dom|debug|legacy)[A-Za-z]*(?:Satisfied|Accepted|EvidenceOk)|success[A-Za-z]*(?:Mock|Artifact|Cpu|Dom|Fallback|Legacy))\b'
ADAPTER_LIFETIME_POLICY_PATTERN='\b(?:HashMap<[^;\n]*(?:HandleToken|RuntimeSessionId|ProjectSessionHandle|FrameHandle|TextureHandle)|BTreeMap<[^;\n]*(?:HandleToken|RuntimeSessionId|ProjectSessionHandle|FrameHandle|TextureHandle)|next_(?:handle|generation|session|lease)_id|release_handle\s*\(|retain_count|ref_count|Arc<Mutex<HashMap<[^;\n]*(?:HandleToken|ProjectSessionHandle))\b'
ADAPTER_SEMANTIC_DUPLICATION_PATTERN='\b(?:struct\s+ProjectSessionRegistry|struct\s+ProjectSession\s*\{|struct\s+ActiveProjectInteraction|ProjectSessionRegistry\s*\{|SchedulerExportService|ExportJobRegistry|prepare_export_job\s*\(|run_export_thread|DesktopFfmpegExecutor::with_timeout|draft_commands::timeline::execute_timeline_edit|project_store::(?:create_project_bundle|open_project_bundle|save_project_bundle))\b'

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
  assert_pattern_rejects \
    "adapter-owned duplicated project/export semantics" \
    "$ADAPTER_SEMANTIC_DUPLICATION_PATTERN" \
    'struct ProjectSessionRegistry { sessions: HashMap<String, ProjectSession> }'
  echo "phase18 source guard self-test passed"
}

require_plan01_runtime_files() {
  for file in \
    "crates/editor_runtime/Cargo.toml" \
    "crates/editor_runtime/src/lib.rs" \
    "crates/editor_runtime/src/session.rs" \
    "crates/editor_runtime/src/project_session.rs" \
    "crates/editor_runtime/src/export.rs" \
    "crates/editor_runtime/src/handles.rs" \
    "crates/editor_runtime/tests/project_session_runtime.rs" \
    "crates/editor_runtime/tests/handle_registry.rs"; do
    require_file "$file"
  done
}

require_plan03_node_files() {
  require_plan01_runtime_files
  for file in \
    "crates/bindings_node/Cargo.toml" \
    "crates/bindings_node/src/lib.rs" \
    "crates/bindings_node/src/project_session_service.rs" \
    "crates/bindings_node/src/preview_export_service.rs" \
    "apps/desktop-electron/src/main/index.ts" \
    "apps/desktop-electron/src/main/nativeBinding.ts" \
    "apps/desktop-electron/src/preload/index.ts" \
    "apps/desktop-electron/src/renderer/App.tsx" \
    "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"; do
    require_file "$file"
  done
}

require_plan04_c_files() {
  require_plan01_runtime_files
  for file in \
    "crates/bindings_c/Cargo.toml" \
    "crates/bindings_c/src/lib.rs" \
    "crates/bindings_c/cbindgen.toml" \
    "crates/bindings_c/include/video_editor_runtime.h" \
    "crates/bindings_c/tests/abi_smoke.rs" \
    "scripts/phase18-abi-drift.sh"; do
    require_file "$file"
  done
}

require_plan05_server_files() {
  require_plan01_runtime_files
  for file in \
    "crates/server_runtime/Cargo.toml" \
    "crates/server_runtime/src/lib.rs" \
    "crates/server_runtime/src/main.rs" \
    "crates/server_runtime/tests/server_export_smoke.rs"; do
    require_file "$file"
  done
}

require_mobile_contract_files() {
  for file in \
    "docs/mobile-runtime-contracts.md" \
    "scripts/phase18-mobile-contract-guards.sh" \
    "crates/bindings_c/tests/mobile_contract_handles.rs"; do
    require_file "$file"
  done
}

scan_bindings_node_adapter() {
  fail_matches \
    "bindings_node must stay a thin Node-API adapter over editor_runtime; project/session/export semantics belong in editor_runtime" \
    "$ADAPTER_SEMANTIC_DUPLICATION_PATTERN" \
    "crates/bindings_node/src"
  fail_matches \
    "bindings_node must not own portable handle lifetime policy; handles are Rust-owned by editor_runtime" \
    "$ADAPTER_LIFETIME_POLICY_PATTERN" \
    "crates/bindings_node/src"
}

scan_electron_render_export_boundary() {
  fail_matches \
    "Electron renderer/main/preload must not construct render graphs, FFmpeg jobs, or export scripts; UI emits commands and Rust owns render/export semantics" \
    "$ELECTRON_RENDER_EXPORT_PATTERN" \
    "apps/desktop-electron/src/main" \
    "apps/desktop-electron/src/preload" \
    "apps/desktop-electron/src/renderer"
  fail_matches \
    "product success must not be satisfied by mock, artifact, CPU readback, DOM, debug, fallback, or legacy evidence" \
    "$FALLBACK_SUCCESS_PATTERN" \
    "apps/desktop-electron/src/main" \
    "apps/desktop-electron/src/preload" \
    "apps/desktop-electron/src/renderer" \
    "apps/desktop-electron/tests" \
    --glob '!**/node_modules/**'
}

scan_bindings_c_adapter() {
  fail_matches \
    "bindings_c must call editor_runtime directly and must never depend on the desktop Node-API adapter" \
    "$BINDINGS_C_NODE_DEP_PATTERN" \
    "crates/bindings_c"
  require_fixed "crates/bindings_c/Cargo.toml" "editor_runtime"
  fail_matches \
    "bindings_c must not duplicate project/session/export semantics; C ABI functions delegate to editor_runtime" \
    "$ADAPTER_SEMANTIC_DUPLICATION_PATTERN" \
    "crates/bindings_c"
  fail_matches \
    "bindings_c must not own handle lifetime policy; C callers hold opaque Rust-owned tokens only" \
    "$ADAPTER_LIFETIME_POLICY_PATTERN" \
    "crates/bindings_c"
}

scan_server_runtime_adapter() {
  fail_matches \
    "server_runtime must be Electron-free and must not import BrowserWindow, preload, DOM, nativeBinding, or desktop app source" \
    "$SERVER_ELECTRON_DEP_PATTERN" \
    "crates/server_runtime"
  require_fixed "crates/server_runtime/Cargo.toml" "editor_runtime"
  fail_matches \
    "server_runtime must not duplicate project/session/export registries; server entrypoints delegate to editor_runtime shared services" \
    "$ADAPTER_SEMANTIC_DUPLICATION_PATTERN" \
    "crates/server_runtime"
  fail_matches \
    "server_runtime must not own portable handle lifetime policy; handles are owned by editor_runtime" \
    "$ADAPTER_LIFETIME_POLICY_PATTERN" \
    "crates/server_runtime"
}

scan_mobile_contract_boundary() {
  require_fixed "scripts/phase18-mobile-contract-guards.sh" "mobile runtime contract"
  fail_matches \
    "mobile contracts must not route future JNI/Swift ownership through the desktop Node-API adapter" \
    "$BINDINGS_C_NODE_DEP_PATTERN" \
    "docs/mobile-runtime-contracts.md" \
    "crates/bindings_c/tests/mobile_contract_handles.rs"
}

run_plan03() {
  require_plan03_node_files
  require_fixed "crates/bindings_node/Cargo.toml" "editor_runtime"
  scan_bindings_node_adapter
  scan_electron_render_export_boundary
  echo "phase18 source guards passed for plan 03"
}

run_plan04() {
  require_plan04_c_files
  scan_bindings_c_adapter
  echo "phase18 source guards passed for plan 04"
}

run_plan05() {
  require_plan05_server_files
  scan_server_runtime_adapter
  echo "phase18 source guards passed for plan 05"
}

run_mobile_contracts() {
  require_mobile_contract_files
  scan_mobile_contract_boundary
  echo "phase18 source guards passed for mobile contracts"
}

run_full() {
  require_file "scripts/phase18-source-guards.sh"
  require_file "scripts/phase18-abi-drift.sh"
  require_file "scripts/phase18-mobile-contract-guards.sh"
  require_file "package.json"
  require_fixed "package.json" "\"test:phase18-rust\""
  require_fixed "package.json" "\"test:phase18-source-guards\""
  require_fixed "package.json" "\"test:phase18-abi\""
  require_fixed "package.json" "\"test:phase18-server\""
  require_fixed "package.json" "\"test:phase18-mobile-contracts\""
  require_fixed "package.json" "\"test:phase18\""
  require_plan03_node_files
  require_plan04_c_files
  require_plan05_server_files
  require_mobile_contract_files
  scan_bindings_node_adapter
  scan_electron_render_export_boundary
  scan_bindings_c_adapter
  scan_server_runtime_adapter
  scan_mobile_contract_boundary
  echo "phase18 source guards passed"
}

usage() {
  cat <<'USAGE'
Usage: bash scripts/phase18-source-guards.sh [--self-test|--plan 03|--plan 04|--plan 05|--mobile-contracts]

Modes:
  --self-test          Run negative guard injections only.
  --plan 03           Check Node/Electron adapter ownership after Plan 03 artifacts exist.
  --plan 04           Check C ABI adapter ownership after Plan 04 artifacts exist.
  --plan 05           Check server runtime ownership after Plan 05 artifacts exist.
  --mobile-contracts  Check mobile contract ownership after Plan 06 mobile artifacts exist.
  default/full        Require every Phase 18 artifact and run all scans. Reserved for Plan 06/aggregate.
USAGE
}

if [ "${1:-}" = "--" ]; then
  shift
fi

case "${1:-}" in
  --self-test)
    run_self_test
    ;;
  --plan)
    case "${2:-}" in
      03) run_plan03 ;;
      04) run_plan04 ;;
      05) run_plan05 ;;
      *) fail "unknown staged plan '${2:-}'; expected 03, 04, or 05" ;;
    esac
    ;;
  --plan=03)
    run_plan03
    ;;
  --plan=04)
    run_plan04
    ;;
  --plan=05)
    run_plan05
    ;;
  --mobile-contracts)
    run_mobile_contracts
    ;;
  -h|--help)
    usage
    ;;
  "")
    run_full
    ;;
  *)
    fail "unknown argument '$1'"
    ;;
esac

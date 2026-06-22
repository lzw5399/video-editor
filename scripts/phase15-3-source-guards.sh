#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase15.3 source guard violation: $1" >&2
  exit 1
}

require_file() {
  local file="$1"
  [ -f "$file" ] || fail "missing required file ${file}"
}

require_fixed() {
  local file="$1"
  local text="$2"
  if ! rg -n --fixed-strings "$text" "$file" >/dev/null; then
    fail "missing required text '${text}' in ${file}"
  fi
}

fail_if_matches() {
  local label="$1"
  local pattern="$2"
  shift 2

  if rg -n --pcre2 "$pattern" "$@"; then
    fail "$label"
  fi
}

require_file "apps/desktop-electron/src/renderer/assets/icons/manifest.json"
require_file "docs/ui-reference/jianying-pro/manifest.json"
require_file "apps/desktop-electron/tests/ui-reference-regression.spec.ts"
require_file "apps/desktop-electron/tests/project-entry.spec.ts"
require_file "apps/desktop-electron/tests/export-modal.spec.ts"
require_file "apps/desktop-electron/tests/product-user-journey.spec.ts"
require_file "apps/desktop-electron/tests/helpers/userJourney.ts"

for icon in play pause undo redo delete split zoom-in zoom-out; do
  require_file "apps/desktop-electron/src/renderer/assets/icons/${icon}.svg"
done

require_fixed "apps/desktop-electron/src/renderer/assets/icons/index.ts" "appIconUrls"
require_fixed "apps/desktop-electron/src/renderer/assets/icons/manifest.json" "icons/MaterialReplacement/play.svg"
require_fixed "apps/desktop-electron/src/renderer/assets/icons/manifest.json" "icons/ToolBar/cutting.svg"
require_fixed "docs/ui-reference/jianying-pro/manifest.json" "\"pixelGolden\": false"
require_fixed "docs/ui-reference/jianying-pro/manifest.json" "\"provisional\": true"
require_fixed "apps/desktop-electron/tests/ui-reference-regression.spec.ts" "FORBIDDEN_DEFAULT_COPY"
require_fixed "apps/desktop-electron/tests/ui-reference-regression.spec.ts" "project-entry-1280x800.png"
require_fixed "apps/desktop-electron/tests/ui-reference-regression.spec.ts" "workspace-1280x800.png"
require_fixed "apps/desktop-electron/tests/ui-reference-regression.spec.ts" "workspace-1120x720.png"
require_fixed "apps/desktop-electron/tests/ui-reference-regression.spec.ts" "export-advanced-dropdown-1280x800.png"
require_fixed "apps/desktop-electron/tests/ui-reference-regression.spec.ts" "expectNoOverlap"
require_fixed "apps/desktop-electron/tests/ui-reference-regression.spec.ts" "collectProductSurfaceCopy"
require_fixed "apps/desktop-electron/tests/ui-reference-regression.spec.ts" "项目入口"
require_fixed "apps/desktop-electron/tests/ui-reference-regression.spec.ts" "产品操作"
require_fixed "apps/desktop-electron/tests/ui-reference-regression.spec.ts" "音频采样率选项"

fail_if_matches \
  "runtime renderer code must import/reference only copied app-local icons, not the root icons tree" \
  '(\.\./){2,}icons/|/Users/zhiwen/code/video-editor/icons|source:\s*["'\'']icons/' \
  apps/desktop-electron/src/renderer \
  --glob '!assets/icons/manifest.json' \
  --glob '!assets/icons/*.svg'

fail_if_matches \
  "PreviewMonitor must not expose a permanent production export panel" \
  '导出面板|export-panel' \
  apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx

fail_if_matches \
  "default workspace shell/timeline must not expose backend, mock, request-preview, artifact, or cache copy" \
  'FFmpeg|ffprobe|Mock|requestProjectSessionPreviewFrame|生成预览片段|资源维护|运行环境诊断|草稿包路径' \
  apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx \
  apps/desktop-electron/src/renderer/workspace/Timeline.tsx

fail_if_matches \
  "automatic runtime capability probing must not publish engineering diagnostics through product commandError" \
  'commandError:[[:space:]]*runtimeDiagnostics\.statusLabel|commandError:[[:space:]]*runtimeDiagnostics\.statusDetail' \
  apps/desktop-electron/src/renderer/App.tsx

fail_if_matches \
  "product UI must use user-facing preview/export labels instead of host/log diagnostics copy" \
  'aria-label="实时预览宿主"|实时预览宿主不可用|导出日志' \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx \
  apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx

fail_if_matches \
  "product export modal must not directly surface raw export diagnostics" \
  'const exportMessage = exportState\.error \?\? exportState\.diagnosticLabel \?\? exportState\.logSummary' \
  apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx

fail_if_matches \
  "export diagnostic copy must be product-safe, not render-graph or runtime wording" \
  '渲染图失败|运行时不可用|运行时失败' \
  apps/desktop-electron/src/renderer/viewModel.ts

require_fixed "apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx" "productExportStatusMessage(exportState)"
require_fixed "apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx" "isProductSafeStatusCopy(exportState.logSummary)"

fail_if_matches \
  "product UI must not expose backend/mock selector controls" \
  'backend.*(select|button|combobox)|mock.*(select|button|combobox)|(select|button|combobox).*backend|(select|button|combobox).*mock' \
  apps/desktop-electron/src/renderer/workspace

fail_if_matches \
  "Electron realtime preview host must not own playback cadence or fake compositor evidence" \
  'playbackTimer|playbackTickInFlight|presentPlaybackTimerTick|pauseAtSequenceEnd|presentPlaybackTick|schedulerCompositedEvidence' \
  apps/desktop-electron/src/main/realtimePreviewHost.ts

fail_if_matches \
  "Electron realtime preview host must subscribe to native playback events instead of interval fanout" \
  'setInterval\(|TELEMETRY_FANOUT_INTERVAL|startTelemetryFanout|telemetryFanoutTimer' \
  apps/desktop-electron/src/main/realtimePreviewHost.ts

require_fixed "apps/desktop-electron/src/main/realtimePreviewHost.ts" "subscribeRealtimePreviewEvents"
require_fixed "apps/desktop-electron/src/main/realtimePreviewHost.ts" "nativePreviewEventBridgeInstalled"

fail_if_matches \
  "PreviewMonitor must subscribe to realtime telemetry instead of polling getTelemetry on a renderer interval" \
  'playbackRunning\s*\?\s*33|setInterval\(|bridge\.getTelemetry\(' \
  apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx

fail_if_matches \
  "preload realtime preview host must not expose renderer getTelemetry polling APIs" \
  'getTelemetry|realtimePreviewHost:getTelemetry' \
  apps/desktop-electron/src/preload/index.ts

fail_if_matches \
  "renderer/main/preload/native binding must sync realtime preview from project session snapshots, not renderer-owned Draft payloads" \
  'updateDraftSnapshot|updateRealtimePreviewDraftSnapshot|RealtimePreviewDraftSnapshotRequest|realtimePreviewHost:updateDraftSnapshot|bridge\.updateDraftSnapshot' \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx \
  apps/desktop-electron/src/preload/index.ts \
  apps/desktop-electron/src/main/realtimePreviewHost.ts \
  apps/desktop-electron/src/main/nativeBinding.ts \
  crates/bindings_node/src/lib.rs

fail_if_matches \
  "renderer must not start export through renderer-owned draft payloads; use startProjectSessionExport" \
  'buildStartExportCommand|command:[[:space:]]*"startExport"|kind:[[:space:]]*"startExport"' \
  apps/desktop-electron/src/renderer apps/desktop-electron/src/preload

require_fixed "apps/desktop-electron/tests/helpers/userJourney.ts" "waitForVisiblePreviewCenterChange"
require_fixed "apps/desktop-electron/tests/helpers/userJourney.ts" "renderGraphGpuComposited"
require_fixed "apps/desktop-electron/tests/helpers/userJourney.ts" "requestProjectSessionPreviewFrameCount"
require_fixed "apps/desktop-electron/tests/helpers/userJourney.ts" "getByRole(\"main\", { name: \"项目入口\" })"
require_fixed "apps/desktop-electron/tests/helpers/userJourney.ts" "getByRole(\"button\", { name: \"导入素材\" })"
require_fixed "apps/desktop-electron/tests/helpers/userJourney.ts" "importMaterialsThroughProductPicker"
require_fixed "apps/desktop-electron/tests/product-user-journey.spec.ts" "waitForProductPlaybackSuccess"
require_fixed "apps/desktop-electron/tests/product-user-journey.spec.ts" "renderGraphGpuComposited"
require_fixed "apps/desktop-electron/tests/product-user-journey.spec.ts" "requestProjectSessionPreviewFrameCount"

require_fixed "package.json" "\"test:phase15-3-source-guards\""
require_fixed "package.json" "bash scripts/phase15-3-source-guards.sh"

echo "phase15.3 source guards passed"

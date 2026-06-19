#!/usr/bin/env bash
set -euo pipefail

fail_if_matches() {
  local label="$1"
  local pattern="$2"
  shift 2

  if rg -n "$pattern" "$@"; then
    echo "phase15.1 source guard violation: ${label}" >&2
    exit 1
  fi
}

fail_if_matches \
  "product playback must not be driven by main-process preview-frame timers" \
  'requestPlaybackFrame|playbackTick|playbackTimer|tickPlaybackFrame|presentRealtimePreviewFrame|setInterval\(' \
  apps/desktop-electron/src/main/realtimePreviewHost.ts

fail_if_matches \
  "renderer must not own render graph, FFmpeg, GPU command, or preview cache construction" \
  'buildRenderGraph|new RenderGraph|GPUCommand|GPUDevice|navigator\.gpu|cacheKey\s*=|previewCache\s*=|child_process|execFile|spawn\(' \
  apps/desktop-electron/src/renderer

fail_if_matches \
  "product user journey must require composited preview evidence only" \
  'source:\s*"decoded"|source:\s*"mock"|source:\s*"offscreen"' \
  apps/desktop-electron/tests/product-user-journey.spec.ts apps/desktop-electron/tests/helpers/userJourney.ts

fail_if_matches \
  "timeline track operations must stay command-owned instead of renderer draft mutation" \
  '\.tracks\s*=|tracks\.(push|splice|sort)\(|\.segments\s*=|segments\.(push|splice|sort)\(' \
  apps/desktop-electron/src/renderer

echo "phase15.1 source guards passed"

#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase11-source-guards: rg is required" >&2
  exit 1
fi

RENDERER_DIR="apps/desktop-electron/src/renderer"
RENDERER_WORKSPACE_DIR="apps/desktop-electron/src/renderer/workspace"
PACKAGE_JSON="package.json"
CONTRACT_FILES=(
  "schemas/draft.schema.json"
  "schemas/command.schema.json"
  "apps/desktop-electron/src/generated/Draft.ts"
  "apps/desktop-electron/src/generated/CommandEnvelope.ts"
)
RUNTIME_BOUNDARY_DOC="docs/runtime-boundaries.md"

fail() {
  echo "phase11-source-guards: $1" >&2
  exit 1
}

strip_comments() {
  rg -v ':[[:space:]]*(//|/\*|\*|#)' \
    | rg -v '^\s*(//|/\*|\*|#)' \
    || true
}

matches_for_pattern() {
  local pattern="$1"
  shift
  rg -n --pcre2 "$pattern" "$@" 2>/dev/null | strip_comments
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

require_fixed() {
  local file="$1"
  local text="$2"
  if ! rg -n --fixed-strings "$text" "$file" >/dev/null; then
    fail "missing required text '${text}' in ${file}"
  fi
}

assert_pattern_rejects() {
  local description="$1"
  local pattern="$2"
  local source="$3"
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' RETURN
  printf '%s\n' "$source" >"$tmp_dir/InjectedRendererOwnership.tsx"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedRendererOwnership.tsx" || true)" ]; then
    fail "negative check did not catch injected renderer-owned ${description}"
  fi
  printf '%s\n' "// $source" >"$tmp_dir/CommentOnly.tsx"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.tsx" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

GPU_OWNERSHIP_PATTERN='\b(?:GPUDevice|GPUCanvasContext|GPUCommandEncoder|GPURenderPassEncoder|navigator\.gpu|wgpu)\b|createCommandEncoder|beginRenderPass|\bdraw\s*\('
RENDER_FFMPEG_PATTERN='\b(?:build_render_graph|RenderGraph|compile_ffmpeg_job|FfmpegExecutor|FfmpegJob|filter_complex|filterComplex|ffmpegArgs|ffmpegScripts|exportScript|AssSidecar|generateAss|assContents|child_process|execFile|exec\s*\(|spawn\s*\()\b'
CACHE_DIRTY_PATTERN='\b(?:previewCacheKey|semanticFingerprint|materialDependencies|dirtyRanges?|changedRanges|changedMaterialIds|dirtyRangePropagation|invalidateDirtyRange)\b'
FALLBACK_SEMANTICS_PATTERN='\b(?:fallbackLadder|chooseFallback|selectFallback|routeFallback|classifyFallback)\b|fallbackReason\s*(?<![=!<>])=(?!=)'
TIMELINE_MUTATION_PATTERN='(?:draft|current|nextDraft|workspace\.draft)\.tracks\s*=|\.tracks\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(|(?:track|candidate|selectedTrack)\.segments\s*=|\.segments\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(|(?:draft|current|nextDraft|workspace\.draft)(?:\.tracks|\["tracks"\])\s*\[[^]]+\]\s*=|(?:track|candidate|selectedTrack)(?:\.segments|\["segments"\])\s*\[[^]]+\]\s*=|\.(?:sourceTimerange|targetTimerange)\s*=|(?:sourceTimerange|targetTimerange)\.(?:start|duration)\s*=|(?:segment|selectedSegment|currentSegment|candidate)\.(?:keyframes|text|visual|volume)\s*(?<![=!<>])=(?!=)|\.(?:keyframes|segments|tracks)\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(|\.text\.(?:content|source|style|textBox|layoutRegion|wrapping|bubble|effect)\s*(?<![=!<>])=(?!=)|\.visual\.(?:transform|fitMode|backgroundFilling|blendMode|mask|visible)\s*(?<![=!<>])=(?!=)|\.volume\.levelMillis\s*(?<![=!<>])=(?!=)|\.(?:undoStack|redoStack)\s*(?<![=!<>])=(?!=)|\.(?:undoStack|redoStack)\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\('
KEYFRAME_EVALUATION_PATTERN='\b(?:evaluateKeyframes?|resolveKeyframes?|sampleAnimation|sampleAnimated|interpolateKeyframes?|interpolateAnimation|evaluateEasing|applyEasing|frameTimeAnimation)\b|(?:Math\.(?:sin|cos|pow|sqrt)|progressPerMille).*(?:keyframe|easing|animation)|(?:keyframe|easing|animation).*(?:Math\.(?:sin|cos|pow|sqrt)|progressPerMille)'
FLOAT_TIMELINE_PATTERN='\b(?:targetTimeSeconds|target_time_seconds|timelineSeconds|timeline_seconds|durationSeconds|duration_seconds|sourceTimeSeconds|source_time_seconds|targetTimerangeSeconds|target_timerange_seconds|sourceTimerangeSeconds|source_timerange_seconds|seconds\s*:\s*f32|seconds\s*:\s*f64)\b'

assert_pattern_rejects "GPU/WebGPU command ownership" "$GPU_OWNERSHIP_PATTERN" "const device: GPUDevice = await navigator.gpu.requestAdapter();"
assert_pattern_rejects "render graph and FFmpeg ownership" "$RENDER_FFMPEG_PATTERN" "const graph: RenderGraph = build_render_graph(draft);"
assert_pattern_rejects "cache key and dirty range ownership" "$CACHE_DIRTY_PATTERN" "const previewCacheKey = semanticFingerprint + changedRanges.length;"
assert_pattern_rejects "fallback semantics" "$FALLBACK_SEMANTICS_PATTERN" "const fallbackLadder = chooseFallback(material);"
assert_pattern_rejects "timeline mutation" "$TIMELINE_MUTATION_PATTERN" "draft.tracks.push(track);"
assert_pattern_rejects "keyframe evaluation" "$KEYFRAME_EVALUATION_PATTERN" "const value = interpolateKeyframes(segment.keyframes, targetTime);"
assert_pattern_rejects "floating-point persisted timeline request fields" "$FLOAT_TIMELINE_PATTERN" "type Request = { targetTimeSeconds: number };"

for script in \
  "test:phase11-rust" \
  "test:phase11-source-guards" \
  "test:phase11-workspace" \
  "test:phase11"; do
  require_fixed "$PACKAGE_JSON" "\"${script}\""
done
require_fixed "$PACKAGE_JSON" "bash scripts/phase11-source-guards.sh"
require_fixed "$PACKAGE_JSON" "实时预览|fallback|telemetry|五大区域"
require_fixed "$PACKAGE_JSON" "test:contracts"

require_fixed "$RUNTIME_BOUNDARY_DOC" "Runtime Boundaries"

fail_matches \
  "renderer must not own WebGPU, wgpu, GPU devices, surfaces, command encoders, render passes, or draw command lists" \
  "$GPU_OWNERSHIP_PATTERN" \
  "$RENDERER_DIR"

fail_matches \
  "renderer must not construct render graphs, FFmpeg jobs/scripts, ASS sidecars, process execution, or export runtime commands" \
  "$RENDER_FFMPEG_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "renderer must not construct preview cache keys, semantic fingerprints, material dependency sets, or dirty range propagation" \
  "$CACHE_DIRTY_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "renderer must not choose realtime preview fallback semantics or assign fallback reasons" \
  "$FALLBACK_SEMANTICS_PATTERN" \
  "$RENDERER_WORKSPACE_DIR"

fail_matches \
  "renderer realtime preview monitor must subscribe to host telemetry instead of polling telemetry snapshots" \
  'setInterval\(|bridge\.getTelemetry\(' \
  apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx

fail_matches \
  "preload realtime preview host must not expose a renderer getTelemetry polling API" \
  'getTelemetry|realtimePreviewHost:getTelemetry' \
  apps/desktop-electron/src/preload/index.ts

fail_matches \
  "renderer must not directly mutate draft tracks, track segments, timeranges, keyframes, visual/text/audio semantics, or undo/redo stacks" \
  "$TIMELINE_MUTATION_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "renderer must not evaluate keyframes, interpolate animation, sample frame-time animation, or implement easing math" \
  "$KEYFRAME_EVALUATION_PATTERN" \
  "$RENDERER_DIR"

fail_matches \
  "persisted timeline request contracts must not use floating-point seconds fields" \
  "$FLOAT_TIMELINE_PATTERN" \
  crates/draft_model/src \
  crates/draft_commands/src \
  crates/bindings_node/src \
  schemas \
  apps/desktop-electron/src/generated \
  apps/desktop-electron/src/renderer

git diff --exit-code "${CONTRACT_FILES[@]}" >/dev/null

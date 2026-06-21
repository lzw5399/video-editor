#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase8-source-guards: rg is required" >&2
  exit 1
fi

GENERATED_FILES=(
  "schemas/draft.schema.json"
  "schemas/command.schema.json"
  "apps/desktop-electron/src/generated/Draft.ts"
  "apps/desktop-electron/src/generated/CommandEnvelope.ts"
)
VISUAL_CONTRACT_FILES=(
  "crates/draft_model/src/timeline.rs"
  "crates/draft_model/src/validation.rs"
  "crates/draft_model/src/lib.rs"
  "crates/draft_commands/src/visual.rs"
  "crates/draft_commands/src/timeline.rs"
  "crates/engine_core/src/frame_state.rs"
  "crates/engine_core/src/normalize.rs"
  "crates/render_graph/src/graph.rs"
  "crates/ffmpeg_compiler/src/filters.rs"
  "schemas/draft.schema.json"
  "schemas/command.schema.json"
  "apps/desktop-electron/src/generated/Draft.ts"
  "apps/desktop-electron/src/generated/CommandEnvelope.ts"
)
UI_FILES=(
  "apps/desktop-electron/src/renderer/workspace/Inspector.tsx"
  "apps/desktop-electron/src/renderer/workspace/preview-inspector.css"
  "apps/desktop-electron/tests/workspace.spec.ts"
)
RENDERER_DIR="apps/desktop-electron/src/renderer"
PACKAGE_FILES=("package.json" "justfile")

fail() {
  echo "phase8-source-guards: $1" >&2
  exit 1
}

fail_matches() {
  local message="$1"
  local pattern="$2"
  shift 2
  local matches
  matches="$(
    rg -n --pcre2 "$pattern" "$@" 2>/dev/null \
      | rg -v ':[[:space:]]*(//|/\*|\*|#)' \
      | rg -v 'renderGraphFailed' \
      | rg -v 'renderGraphGpu|renderGraphGpuComposited' \
      || true
  )"
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

for symbol in \
  "SegmentVisual" \
  "SegmentTransform" \
  "SegmentFitMode" \
  "SegmentBackgroundFilling" \
  "SegmentBlendMode" \
  "SegmentMask"; do
  found=false
  for file in "${GENERATED_FILES[@]}"; do
    if rg -n --fixed-strings "$symbol" "$file" >/dev/null; then
      found=true
      break
    fi
  done
  [ "$found" = "true" ] || fail "generated contracts must contain ${symbol}"
done

fail_matches \
  "generated public command contracts must not expose structural visual edit payloads" \
  'UpdateSegmentVisualCommandPayload|"\s*updateSegmentVisual\s*"' \
  schemas/command.schema.json apps/desktop-electron/src/generated/CommandEnvelope.ts

for text in \
  "画面基础表单" \
  "显示画面" \
  "位置" \
  "缩放" \
  "旋转" \
  "不透明度" \
  "适应方式" \
  "裁剪" \
  "背景填充" \
  "混合模式" \
  "蒙版" \
  "应用画面"; do
  found=false
  for file in "${UI_FILES[@]}"; do
    if rg -n --fixed-strings "$text" "$file" >/dev/null; then
      found=true
      break
    fi
  done
  [ "$found" = "true" ] || fail "missing required Chinese visual UI copy: ${text}"
done

require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "画面变换 command-only transform"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "expectNoLeftSecondaryMenu"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "五大区域"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "updateSegmentVisual"

fail_matches \
  "Phase 08 semantic surfaces must keep Jianying terms and avoid Asset/Clip/LayerItem vocabulary" \
  '\b(Asset|Clip|LayerItem|SceneSize)\b' \
  "${VISUAL_CONTRACT_FILES[@]}" "${UI_FILES[@]}"

fail_matches \
  "renderer must not mutate draft tracks, track segments, timeranges, or main-track magnet semantics directly" \
  '(?:draft|current|nextDraft|workspace\.draft)\.tracks\s*=|\.tracks\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(|(?:track|candidate)\.segments\s*=|\.segments\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(|(?:draft|current|nextDraft|workspace\.draft)(?:\.tracks|\["tracks"\])\s*\[[^]]+\]\s*=|(?:track|candidate)(?:\.segments|\["segments"\])\s*\[[^]]+\]\s*=|\.(?:sourceTimerange|targetTimerange|mainTrackMagnet)\s*=|(?:sourceTimerange|targetTimerange|mainTrackMagnet)\.(?:start|duration|enabled)\s*=' \
  "$RENDERER_DIR"

fail_matches \
  "renderer must not mutate segment visual, transform, crop, fit, background, blend, mask, undo, or redo semantics directly" \
  '(?:segment|selectedSegment|currentSegment|candidate)\.visual\s*(?<![=!<>])=(?!=)|\.visual\.(?:visible|transform|fitMode|backgroundFilling|blendMode|mask)\s*(?<![=!<>])=(?!=)|\.transform\.(?:position|scale|rotation|opacity|crop|anchor)\s*(?<![=!<>])=(?!=)|\.position\.(?:x|y)\s*(?<![=!<>])=(?!=)|\.scale\.(?:xMillis|yMillis)\s*(?<![=!<>])=(?!=)|\.rotation\.degrees\s*(?<![=!<>])=(?!=)|\.opacity\.valueMillis\s*(?<![=!<>])=(?!=)|\.crop\.(?:leftMillis|rightMillis|topMillis|bottomMillis)\s*(?<![=!<>])=(?!=)|\.anchor\.(?:xMillis|yMillis)\s*(?<![=!<>])=(?!=)|\.fitMode\s*(?<![=!<>])=(?!=)|\.backgroundFilling\s*(?<![=!<>])=(?!=)|\.blendMode\s*(?<![=!<>])=(?!=)|\.mask\s*(?<![=!<>])=(?!=)|\.(?:undoStack|redoStack)\s*(?<![=!<>])=(?!=)|\.(?:undoStack|redoStack)\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(' \
  "$RENDERER_DIR"

fail_matches \
  "renderer must not own FFmpeg/render graph/export validation/preview cache semantics for transforms" \
  'filter_complex|filterComplex|FfmpegJob|renderGraph|RenderGraph|ffmpegArgs|ffmpegScripts|exportScript|OutputValidation|validationExpectation|export_dimensions|outputWidth|outputHeight|previewCacheKey|semanticFingerprint|materialDependencies|changedRanges|changedMaterialIds|child_process|spawn\(|execFile|exec\(|process\.' \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "Phase 08 must not add icon package dependencies" \
  'lucide-react|react-icons|@fortawesome|@heroicons' \
  "${PACKAGE_FILES[@]}"

git diff --exit-code schemas apps/desktop-electron/src/generated >/dev/null

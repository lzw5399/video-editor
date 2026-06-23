#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase7-source-guards: rg is required" >&2
  exit 1
fi

GENERATED_FILES=(
  "schemas/draft.schema.json"
  "schemas/command.schema.json"
  "apps/desktop-electron/src/generated/Draft.ts"
  "apps/desktop-electron/src/generated/CommandEnvelope.ts"
)
CANVAS_CONTRACT_FILES=(
  "crates/draft_model/src/canvas.rs"
  "crates/draft_model/src/draft.rs"
  "crates/draft_model/src/lib.rs"
  "crates/draft_model/src/validation.rs"
  "crates/draft_commands/src/canvas.rs"
  "schemas/draft.schema.json"
  "schemas/command.schema.json"
  "apps/desktop-electron/src/generated/Draft.ts"
  "apps/desktop-electron/src/generated/CommandEnvelope.ts"
)
UI_FILES=(
  "apps/desktop-electron/src/renderer/workspace/Inspector.tsx"
  "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"
  "apps/desktop-electron/tests/workspace.spec.ts"
)
RENDERER_DIR="apps/desktop-electron/src/renderer"
PACKAGE_FILES=("package.json" "apps/desktop-electron/package.json")
COORDINATE_DOC="docs/canvas-coordinate-system.md"

fail() {
  echo "phase7-source-guards: $1" >&2
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

for symbol in "canvasConfig" "DraftCanvasConfig" "CanvasAspectRatio" "CanvasBackground"; do
  found=false
  for file in "${GENERATED_FILES[@]}"; do
    if rg -n --fixed-strings "$symbol" "$file" >/dev/null; then
      found=true
      break
    fi
  done
  [ "$found" = "true" ] || fail "generated contracts must contain ${symbol}"
done

require_fixed "apps/desktop-electron/src/main/nativeBinding.ts" 'kind: "updateDraftCanvasConfig"'
require_fixed "crates/bindings_node/src/project_session_service.rs" "UpdateDraftCanvasConfig"
require_fixed "apps/desktop-electron/src/renderer/App.tsx" "updateDraftCanvasConfig"

require_fixed "$COORDINATE_DOC" "Origin: origin at canvas center."
require_fixed "$COORDINATE_DOC" "+X right"
require_fixed "$COORDINATE_DOC" "+Y up"
require_fixed "$COORDINATE_DOC" "half canvas width"
require_fixed "$COORDINATE_DOC" "half canvas height"
require_fixed "$COORDINATE_DOC" "坐标以画布中心为原点，X 向右，Y 向上"

fail_matches \
  "Phase 07 semantic surfaces must keep Jianying terms and avoid Asset/Clip/Stage/SceneSize" \
  '\b(Asset|Clip|Stage|SceneSize)\b' \
  "${CANVAS_CONTRACT_FILES[@]}" "${UI_FILES[@]}"

fail_matches \
  "renderer must not directly mutate draft canvas semantics, undo/redo stacks, or canvas fields" \
  '(?:draft|workspace\.draft|current\.draft|nextDraft)\.canvasConfig\s*=|\.canvasConfig\.(?:aspectRatio|width|height|frameRate|background)\s*=|\.(?:undoStack|redoStack)\s*=|\.(?:undoStack|redoStack)\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(' \
  "$RENDERER_DIR"

fail_matches \
  "renderer must not own FFmpeg/render graph/export validation/preview cache/output dimension semantics" \
  'filter_complex|filterComplex|FfmpegJob|\bRenderGraph\b|\brenderGraph(?!Gpu|GpuComposited)\b|ffmpegArgs|ffmpegScripts|exportScript|OutputValidation|validationExpectation|export_dimensions|outputWidth|outputHeight|previewCacheKey|semanticFingerprint|materialDependencies|changedRanges|changedMaterialIds|child_process|spawn\(|execFile|exec\(|process\.' \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "production preview/export services must not keep hard-coded MVP canvas profiles" \
  'EngineProfile::mvp_default\(' \
  "crates/preview_service/src" "crates/bindings_node/src"

fail_matches \
  "production export service must not use preset-owned export dimensions" \
  'export_dimensions\(' \
  "crates/bindings_node/src/preview_export_service.rs"

for text in "草稿参数" "画布比例" "画布尺寸" "帧率" "画布背景" "黑色" "纯色" "模糊填充" "图片背景" "未接入"; do
  found=false
  for file in "${UI_FILES[@]}"; do
    if rg -n --fixed-strings "$text" "$file" >/dev/null; then
      found=true
      break
    fi
  done
  [ "$found" = "true" ] || fail "missing required Chinese canvas UI copy: ${text}"
done

fail_matches \
  "draft canvas settings must be realtime and must not expose an apply button" \
  '应用草稿参数' \
  "apps/desktop-electron/src/renderer" \
  "apps/desktop-electron/src/main"

require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "expectNoLeftSecondaryMenu"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "canvas-1280x800.png"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "canvas-1120x720.png"

fail_matches \
  "Phase 07 must not add icon package dependencies" \
  'lucide-react|react-icons|@fortawesome|@heroicons' \
  "${PACKAGE_FILES[@]}"

git diff --exit-code schemas apps/desktop-electron/src/generated >/dev/null

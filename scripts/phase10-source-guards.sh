#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase10-source-guards: rg is required" >&2
  exit 1
fi

GENERATED_FILES=(
  "schemas/draft.schema.json"
  "schemas/command.schema.json"
  "apps/desktop-electron/src/generated/Draft.ts"
  "apps/desktop-electron/src/generated/CommandEnvelope.ts"
)
KEYFRAME_CONTRACT_FILES=(
  "crates/draft_model/src/timeline.rs"
  "crates/draft_model/src/validation.rs"
  "crates/draft_commands/src/keyframe.rs"
  "crates/bindings_node/src/command.rs"
  "crates/engine_core/src/frame_state.rs"
  "crates/render_graph/src/graph.rs"
  "schemas/draft.schema.json"
  "schemas/command.schema.json"
  "apps/desktop-electron/src/generated/Draft.ts"
  "apps/desktop-electron/src/generated/CommandEnvelope.ts"
)
UI_FILES=(
  "apps/desktop-electron/src/renderer/workspace/Inspector.tsx"
  "apps/desktop-electron/src/renderer/workspace/Timeline.tsx"
  "apps/desktop-electron/src/renderer/workspace/preview-inspector.css"
  "apps/desktop-electron/src/renderer/workspace/timeline.css"
  "apps/desktop-electron/tests/workspace.spec.ts"
)
RENDERER_DIR="apps/desktop-electron/src/renderer"

fail() {
  echo "phase10-source-guards: $1" >&2
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
      | rg -v 'formatKeyframeInterpolation|formatKeyframeEasing|KEYFRAME_INTERPOLATIONS|KEYFRAME_EASINGS' \
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

for symbol in \
  "KeyframeProperty" \
  "KeyframeValue" \
  "KeyframeInterpolation" \
  "KeyframeEasing" \
  "SetSegmentKeyframeCommandPayload" \
  "RemoveSegmentKeyframeCommandPayload" \
  "setSegmentKeyframe" \
  "removeSegmentKeyframe"; do
  found=false
  for file in "${GENERATED_FILES[@]}"; do
    if rg -n --fixed-strings "$symbol" "$file" >/dev/null; then
      found=true
      break
    fi
  done
  [ "$found" = "true" ] || fail "generated contracts must contain ${symbol}"
done

for text in \
  "关键帧" \
  "动画" \
  "添加位置 X关键帧" \
  "删除位置 X关键帧" \
  "关键帧标记" \
  "关键帧命令处理中" \
  "特效动画暂未接入" \
  "还没有关键帧" \
  "缓入缓出"; do
  found=false
  for file in "${UI_FILES[@]}"; do
    if rg -n --fixed-strings "$text" "$file" >/dev/null; then
      found=true
      break
    fi
  done
  [ "$found" = "true" ] || fail "missing required Chinese keyframe/animation UI copy: ${text}"
done

require_fixed "apps/desktop-electron/src/renderer/commandHelpers.ts" "buildSetSegmentKeyframeCommand"
require_fixed "apps/desktop-electron/src/renderer/commandHelpers.ts" "buildRemoveSegmentKeyframeCommand"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "command-only keyframe"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "setSegmentKeyframe"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "removeSegmentKeyframe"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "keyframeAt"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "五大区域"

fail_matches \
  "Phase 10 keyframe semantic surfaces must keep Jianying terms and avoid Asset/Clip/LayerItem vocabulary" \
  '\b(Asset|Clip|LayerItem|AnimationClip|KeyframeTrack)\b' \
  "${KEYFRAME_CONTRACT_FILES[@]}" "${UI_FILES[@]}"

fail_matches \
  "renderer must not directly mutate draft tracks, track segments, segment keyframes, timeranges, or undo/redo stacks" \
  '(?:draft|current|nextDraft|workspace\.draft)\.tracks\s*=|\.tracks\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(|(?:track|candidate)\.segments\s*=|\.segments\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(|(?:segment|selectedSegment|currentSegment|candidate)\.keyframes\s*(?<![=!<>])=(?!=)|\.keyframes\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(|\.(?:sourceTimerange|targetTimerange)\s*=|(?:sourceTimerange|targetTimerange)\.(?:start|duration)\s*=|\.(?:undoStack|redoStack)\s*(?<![=!<>])=(?!=)|\.(?:undoStack|redoStack)\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(' \
  "$RENDERER_DIR"

fail_matches \
  "renderer must not own visual/text/audio persisted semantics while adding keyframes" \
  '(?:segment|selectedSegment|currentSegment|candidate)\.visual\s*(?<![=!<>])=(?!=)|(?:segment|selectedSegment|currentSegment|candidate)\.text\s*(?<![=!<>])=(?!=)|(?:segment|selectedSegment|currentSegment|candidate)\.volume\s*(?<![=!<>])=(?!=)|\.visual\.(?:transform|fitMode|backgroundFilling|blendMode|mask)\s*(?<![=!<>])=(?!=)|\.text\.(?:content|source|style|textBox|layoutRegion|wrapping|bubble|effect)\s*(?<![=!<>])=(?!=)|\.volume\.levelMillis\s*(?<![=!<>])=(?!=)' \
  "$RENDERER_DIR"

fail_matches \
  "renderer must not evaluate keyframes, interpolate animation, sample frame-time animation, or implement easing math" \
  '\b(?:evaluateKeyframes?|resolveKeyframes?|sampleAnimation|sampleAnimated|interpolateKeyframes?|interpolateAnimation|evaluateEasing|applyEasing|frameTimeAnimation)\b|(?:Math\.(?:sin|cos|pow|sqrt)|progressPerMille).*(?:keyframe|easing|animation)|(?:keyframe|easing|animation).*(?:Math\.(?:sin|cos|pow|sqrt)|progressPerMille)' \
  "$RENDERER_DIR"

fail_matches \
  "renderer must not own FFmpeg, ASS sidecar, render graph, export validation, process, or preview/export cache semantics" \
  'filter_complex|filterComplex|FfmpegJob|renderGraph|RenderGraph|ffmpegArgs|ffmpegScripts|exportScript|AssSidecar|generateAss|assContents|OutputValidation|validationExpectation|previewCacheKey|semanticFingerprint|materialDependencies|changedRanges|changedMaterialIds|child_process|spawn\(|execFile|exec\(|process\.' \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

git diff --exit-code schemas apps/desktop-electron/src/generated >/dev/null

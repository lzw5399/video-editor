#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase9-source-guards: rg is required" >&2
  exit 1
fi

GENERATED_FILES=(
  "schemas/draft.schema.json"
  "schemas/command.schema.json"
  "apps/desktop-electron/src/generated/Draft.ts"
  "apps/desktop-electron/src/generated/CommandEnvelope.ts"
)
TEXT_CONTRACT_FILES=(
  "crates/draft_model/src/timeline.rs"
  "crates/draft_model/src/validation.rs"
  "crates/draft_model/src/lib.rs"
  "crates/draft_commands/src/text.rs"
  "crates/draft_commands/src/timeline.rs"
  "crates/engine_core/src/text_layout.rs"
  "crates/render_graph/src/graph.rs"
  "crates/ffmpeg_compiler/src/ass.rs"
  "crates/ffmpeg_compiler/src/job.rs"
  "schemas/draft.schema.json"
  "schemas/command.schema.json"
  "apps/desktop-electron/src/generated/Draft.ts"
  "apps/desktop-electron/src/generated/CommandEnvelope.ts"
)
UI_FILES=(
  "apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx"
  "apps/desktop-electron/src/renderer/workspace/Inspector.tsx"
  "apps/desktop-electron/src/renderer/workspace/preview-inspector.css"
  "apps/desktop-electron/tests/workspace.spec.ts"
)
RENDERER_DIR="apps/desktop-electron/src/renderer"
PACKAGE_FILES=("package.json" "apps/desktop-electron/package.json" "justfile")

fail() {
  echo "phase9-source-guards: $1" >&2
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

for symbol in \
  "TextSegmentSource" \
  "TextFont" \
  "TextBox" \
  "TextLayoutRegion" \
  "TextWrapping" \
  "TextBubbleRef" \
  "TextEffectRef" \
  "ImportSubtitleSrtCommandPayload" \
  "importSubtitleSrt"; do
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
  "文字" \
  "字幕 / 导入字幕" \
  "自动生成字幕片段" \
  "文本框" \
  "行高" \
  "字间距" \
  "安全区域" \
  "花字" \
  "气泡" \
  "暂未接入" \
  "应用文字"; do
  found=false
  for file in "${UI_FILES[@]}"; do
    if rg -n --fixed-strings "$text" "$file" >/dev/null; then
      found=true
      break
    fi
  done
  [ "$found" = "true" ] || fail "missing required Chinese text/subtitle UI copy: ${text}"
done

fail_matches \
  "desktop UI must not leak implementation-facing Rust/SRT parser copy" \
  'Rust 解析 SRT' \
  "${UI_FILES[@]}"

require_fixed "crates/draft_commands/src/text.rs" "importSubtitleSrt"
require_fixed "crates/draft_commands/src/text.rs" "parse_srt"
require_fixed "crates/ffmpeg_compiler/src/job.rs" "UnsupportedTextResource"
require_fixed "crates/ffmpeg_compiler/src/ass.rs" "UnsupportedTextResource"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "command-only text edit"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "SRT import command path sends raw SRT"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "expectNoLeftSecondaryMenu"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "五大区域"

fail_matches \
  "Phase 09 text/subtitle semantic surfaces must keep Jianying terms and avoid caption/asset/clip/layer-item vocabulary" \
  '\b(Caption|CaptionCue|CaptionAsset|SubtitleAsset|Asset|Clip|LayerItem)\b' \
  "${TEXT_CONTRACT_FILES[@]}" "${UI_FILES[@]}"

fail_matches \
  "renderer must not directly mutate draft tracks, track segments, timeranges, text semantics, or undo/redo stacks" \
  '(?:draft|current|nextDraft|workspace\.draft)\.tracks\s*=|\.tracks\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(|(?:track|candidate)\.segments\s*=|\.segments\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(|(?:draft|current|nextDraft|workspace\.draft)(?:\.tracks|\["tracks"\])\s*\[[^]]+\]\s*=|(?:track|candidate)(?:\.segments|\["segments"\])\s*\[[^]]+\]\s*=|\.(?:sourceTimerange|targetTimerange)\s*=|(?:sourceTimerange|targetTimerange)\.(?:start|duration)\s*=|(?:segment|selectedSegment|currentSegment|candidate)\.text\s*(?<![=!<>])=(?!=)|\.text\.(?:content|source|style|textBox|layoutRegion|wrapping|bubble|effect)\s*(?<![=!<>])=(?!=)|\.style\.(?:font|fontSize|color|alignment|lineHeightMillis|letterSpacingMillis|stroke|shadow|background)\s*(?<![=!<>])=(?!=)|\.font\.(?:family|fontRef)\s*(?<![=!<>])=(?!=)|\.textBox\.(?:widthMillis|heightMillis)\s*(?<![=!<>])=(?!=)|\.layoutRegion\.(?:xMillis|yMillis|widthMillis|heightMillis)\s*(?<![=!<>])=(?!=)|\.wrapping\s*(?<![=!<>])=(?!=)|\.(?:undoStack|redoStack)\s*(?<![=!<>])=(?!=)|\.(?:undoStack|redoStack)\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(' \
  "$RENDERER_DIR"

fail_matches \
  "renderer must not parse SRT cues or own subtitle timing semantics" \
  'srtContent\s*\.\s*(?:split|match|matchAll|replace|replaceAll)\s*\(|new\s+RegExp\s*\([^)]*(?:-->|SRT|subtitle)|(?:cue|subtitleCue|srtCue)s?\s*=|parseSrt|parseSubtitle|toSubtitleSegments|HH:MM:SS' \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "renderer must not own FFmpeg, ASS sidecar, render graph, export validation, process, or preview/export cache semantics" \
  'filter_complex|filterComplex|FfmpegJob|renderGraph|RenderGraph|ffmpegArgs|ffmpegScripts|exportScript|AssSidecar|ASS sidecar|generateAss|assContents|OutputValidation|validationExpectation|previewCacheKey|semanticFingerprint|materialDependencies|changedRanges|changedMaterialIds|child_process|spawn\(|execFile|exec\(|process\.' \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "Phase 09 must not add icon package dependencies" \
  'lucide-react|react-icons|@fortawesome|@heroicons' \
  "${PACKAGE_FILES[@]}"

git diff --exit-code schemas apps/desktop-electron/src/generated >/dev/null

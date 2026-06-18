#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase10-1-source-guards: rg is required" >&2
  exit 1
fi

RENDERER_DIR="apps/desktop-electron/src/renderer"
RENDERER_WORKSPACE_FILES=(
  "apps/desktop-electron/src/renderer/App.tsx"
  "apps/desktop-electron/src/renderer/commandHelpers.ts"
  "apps/desktop-electron/src/renderer/viewModel.ts"
  "apps/desktop-electron/src/renderer/workspace"
)
CANONICAL_SCHEMA_FILES=(
  "schemas/draft.schema.json"
  "apps/desktop-electron/src/generated/Draft.ts"
  "fixtures/draft/positive"
)
UI_AND_TEST_FILES=(
  "apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx"
  "apps/desktop-electron/src/renderer/workspace/Inspector.tsx"
  "apps/desktop-electron/tests/workspace.spec.ts"
)
PACKAGE_FILES=("package.json" "apps/desktop-electron/package.json" "justfile")

fail() {
  echo "phase10-1-source-guards: $1" >&2
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

for text in \
  "导入字幕" \
  "SRT 内容" \
  "SRT 字幕" \
  "应用文字" \
  "应用画面" \
  "字幕 SRT import command path sends raw SRT" \
  "editTextSegment" \
  "updateSegmentVisual"; do
  found=false
  for file in "${UI_AND_TEST_FILES[@]}"; do
    if rg -n --fixed-strings "$text" "$file" >/dev/null; then
      found=true
      break
    fi
  done
  [ "$found" = "true" ] || fail "missing required SRT import/edit UI or test coverage text: ${text}"
done

require_fixed "apps/desktop-electron/src/renderer/commandHelpers.ts" "buildImportSubtitleSrtCommand"
require_fixed "apps/desktop-electron/src/renderer/commandHelpers.ts" "buildEditTextSegmentCommand"
require_fixed "apps/desktop-electron/src/renderer/commandHelpers.ts" "buildUpdateSegmentVisualCommand"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "srtContent"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "textSource"

fail_matches \
  "renderer must not import Node, Electron, process, or platform modules" \
  'from\s+["'\''](?:electron|node:[^"'\'']+|fs|path|child_process|os|crypto|stream|buffer)["'\'']|require\s*\(\s*["'\''](?:electron|node:[^"'\'']+|fs|path|child_process|os|crypto|stream|buffer)["'\'']\s*\)|\bprocess\.' \
  "$RENDERER_DIR"

fail_matches \
  "renderer must not directly mutate draft tracks, track segments, timeranges, text, visual, audio, or undo/redo state" \
  '(?:draft|current|nextDraft|workspace\.draft)\.tracks\s*=|\.tracks\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(|(?:track|candidate|selectedTrack)\.segments\s*=|\.segments\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(|(?:draft|current|nextDraft|workspace\.draft)(?:\.tracks|\["tracks"\])\s*\[[^]]+\]\s*=|(?:track|candidate|selectedTrack)(?:\.segments|\["segments"\])\s*\[[^]]+\]\s*=|\.(?:sourceTimerange|targetTimerange)\s*=|(?:sourceTimerange|targetTimerange)\.(?:start|duration)\s*=|(?:segment|selectedSegment|currentSegment|candidate)\.(?:text|visual|volume)\s*(?<![=!<>])=(?!=)|\.text\.(?:content|source|style|textBox|layoutRegion|wrapping|bubble|effect)\s*(?<![=!<>])=(?!=)|\.style\.(?:font|fontSize|color|alignment|lineHeightMillis|letterSpacingMillis|stroke|shadow|background)\s*(?<![=!<>])=(?!=)|\.font\.(?:family|fontRef)\s*(?<![=!<>])=(?!=)|\.textBox\.(?:widthMillis|heightMillis)\s*(?<![=!<>])=(?!=)|\.layoutRegion\.(?:xMillis|yMillis|widthMillis|heightMillis)\s*(?<![=!<>])=(?!=)|\.visual\.(?:transform|fitMode|backgroundFilling|blendMode|mask|visible)\s*(?<![=!<>])=(?!=)|\.volume\.levelMillis\s*(?<![=!<>])=(?!=)|\.(?:undoStack|redoStack)\s*(?<![=!<>])=(?!=)|\.(?:undoStack|redoStack)\.(?:push|pop|shift|unshift|splice|sort|reverse)\s*\(' \
  "$RENDERER_DIR"

fail_matches \
  "renderer must not construct FFmpeg, render graph, export scripts, process execution, or preview/export cache semantics" \
  'filter_complex|filterComplex|FfmpegJob|renderGraph|RenderGraph|ffmpegArgs|ffmpegScripts|exportScript|AssSidecar|ASS sidecar|generateAss|assContents|OutputValidation|validationExpectation|previewCacheKey|previewCachePath|semanticFingerprint|materialDependencies|changedRanges|changedMaterialIds|child_process|spawn\(|execFile|exec\(' \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "renderer must send raw SRT content and must not parse cue timing or create subtitle cue segments" \
  'srtContent\s*\.\s*(?:split|match|matchAll|replace|replaceAll)\s*\(|new\s+RegExp\s*\([^)]*(?:-->|SRT|subtitle)|(?:cue|subtitleCue|srtCue)s?\s*=|parseSrt|parseSubtitle|toSubtitleSegments|HH:MM:SS|-->\s*.*(?:split|match|RegExp)' \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "canonical draft schema must not persist derived waveform, preview, render graph, export, proxy, or FFmpeg artifacts" \
  '\b(thumbnails|thumbnailPath|waveforms|waveformPath|waveformCache|previewCaches|previewCache|previewCachePath|previewFrame|previewFrames|renderGraph|renderGraphs|ffmpegScripts|filterScripts|rawProbeJson|exports|exportJobs|proxyFiles|proxyPath|derivedArtifacts)\b' \
  "${CANONICAL_SCHEMA_FILES[@]}"

fail_matches \
  "Phase 10.1 must not add icon package dependencies" \
  'lucide-react|react-icons|@fortawesome|@heroicons' \
  "${PACKAGE_FILES[@]}"

git diff --exit-code schemas apps/desktop-electron/src/generated >/dev/null

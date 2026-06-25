#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase19 source guard violation: $1" >&2
  exit 1
}

require_file() {
  local file="$1"
  [ -f "$file" ] || fail "missing required Phase 19 artifact ${file}"
}

require_fixed() {
  local file="$1"
  local text="$2"
  if ! rg -n --fixed-strings "$text" "$file" >/dev/null; then
    fail "missing required text '${text}' in ${file}"
  fi
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
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase19Violation.tsx"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase19Violation.tsx" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "// $source" >"$tmp_dir/CommentOnly.tsx"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.tsx" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

ELECTRON_FFMPEG_OWNERSHIP_PATTERN='\b(?:buildRenderGraph|compileRenderGraph|compileFfmpeg|compileFfmpegJob|renderGraphToFfmpeg|filter_complex|filterComplex|ffmpegArgs|ffmpegCommand|ffmpegFilter|FfmpegJob|FfmpegExecutor|spawn\s*\(\s*["'\'']ffmpeg|execFile\s*\(\s*["'\'']ffmpeg|new\s+RenderGraph|new\s+Ffmpeg)\b'
RENDERER_RETIME_MATH_PATTERN='\b(?:sourceToTarget|targetToSource|sourceTime(?:At|For)|targetTime(?:At|For)|retime(?:Source|Target|Mapping)|speedCurve|speedHandle|durationMsWithSpeed|playbackRateForSegment|segmentSpeedAt)\b'
TRANSITION_VALIDATION_PATTERN='\b(?:validateTransitionOverlap|transitionOverlap|adjacentSegmentTransition|applyTransitionWindow|canApplyTransition|transitionAdjacency|transitionWindowForSegment)\b'
EFFECT_EVALUATION_PATTERN='\b(?:evaluate(?:Effect|Filter|Mask|Blend)|apply(?:Effect|Filter|Mask|Blend)To(?:Frame|Pixel|Preview|Canvas)|cssFilterForEffect|filterParametersToCss|mixBlendModeForSegment|maskPathForSegment)\b'
CACHE_FINGERPRINT_PATTERN='\b(?:previewCacheKey|semanticFingerprint|graphFingerprint|cacheFingerprint|effectFingerprint|retimeFingerprint|transitionFingerprint|computeDirtyRanges?|buildDirtyRanges?|makeDirtyRanges?|deriveDirtyRanges?)\b'
PROVIDER_NATIVE_SEMANTIC_PATTERN='\b(?:providerNativeEffect|nativeEffectId|nativeFilterId|nativeTransitionId|kaipaiEffectId|kaipaiFilterId|kaipaiTransitionId|jianyingEffectId|jianyingFilterId|jianyingTransitionId|capcutEffectId|capcutFilterId|capcutTransitionId|externalEffectId|externalFilterId|externalTransitionId)\b'
FALLBACK_SUCCESS_PATTERN='\b(?:(?:fallback|mock|artifact|cpuReadback|cpuProbe|decodedCpu|domOverlay|domOnly|debug|legacy)(?:Preview|Render|Export|Playback|Effect|Transition)?Success|(?:fallback|mock|artifact|cpu|dom|debug|legacy)[A-Za-z]*(?:Satisfied|Accepted|EvidenceOk)|success[A-Za-z]*(?:Fallback|Mock|Artifact|Cpu|Dom|Legacy))\b'
POINTER_SAVE_LOOP_PATTERN='\b(?:pointermove|pointerMove|mousemove|mouseMove|onPointerMove|onMouseMove|dragMove|sliderMove|scrubMove|handle[A-Za-z]*(?:Drag|Slider|Scrub))[A-Za-z0-9_.,:;() =>{}[\]"'\'']{0,240}\b(?:saveProjectBundle|pushUndo|revision\s*(?:\+\+|=)|incrementRevision|executeProjectIntent|executeProjectTimelineIntent)\b'
PERSISTED_RETIME_FLOAT_PATTERN='\b(?:speedSeconds|durationSeconds|targetTimeSeconds|sourceTimeSeconds|retimeSeconds|speedFloat|speedF32|speedF64|durationF32|durationF64)\b'

ELECTRON_BOUNDARY_DIRS=(
  "apps/desktop-electron/src/main"
  "apps/desktop-electron/src/preload"
  "apps/desktop-electron/src/renderer"
)

PRODUCT_TEST_DIRS=(
  "apps/desktop-electron/tests"
)

CANONICAL_SEMANTIC_DIRS=(
  "crates/draft_model/src"
  "crates/draft_commands/src"
  "crates/engine_core/src"
  "crates/audio_engine/src"
  "crates/render_graph/src"
  "crates/realtime_preview_runtime/src"
  "crates/ffmpeg_compiler/src"
  "crates/editor_runtime/src"
  "apps/desktop-electron/src"
)

run_self_test() {
  assert_pattern_rejects \
    "renderer-owned FFmpeg construction" \
    "$ELECTRON_FFMPEG_OWNERSHIP_PATTERN" \
    'const ffmpegArgs = ["-filter_complex", graphScript]; compileFfmpegJob(ffmpegArgs);'
  assert_pattern_rejects \
    "renderer-owned retiming source mapping" \
    "$RENDERER_RETIME_MATH_PATTERN" \
    'const source = sourceTimeAt(targetTime, durationMsWithSpeed(segment));'
  assert_pattern_rejects \
    "renderer-owned transition validation" \
    "$TRANSITION_VALIDATION_PATTERN" \
    'if (validateTransitionOverlap(left, right)) applyTransitionWindow(segment);'
  assert_pattern_rejects \
    "renderer-owned effect evaluation" \
    "$EFFECT_EVALUATION_PATTERN" \
    'const style = cssFilterForEffect(filterParametersToCss(effect));'
  assert_pattern_rejects \
    "renderer-owned cache fingerprint semantics" \
    "$CACHE_FINGERPRINT_PATTERN" \
    'const key = semanticFingerprint(effectFingerprint(filter));'
  assert_pattern_rejects \
    "provider-native IDs in canonical semantics" \
    "$PROVIDER_NATIVE_SEMANTIC_PATTERN" \
    'const nativeEffectId = clip.kaipaiEffectId;'
  assert_pattern_rejects \
    "fallback evidence as product success" \
    "$FALLBACK_SUCCESS_PATTERN" \
    'const artifactPreviewSuccess = true;'
  assert_pattern_rejects \
    "pointer sample save or undo loop" \
    "$POINTER_SAVE_LOOP_PATTERN" \
    'function onPointerMove() { executeProjectIntent(payload); saveProjectBundle(); }'
  echo "phase19 source guard self-test passed"
}

require_wave0_files() {
  require_file "scripts/phase19-source-guards.sh"
  require_file "package.json"
  require_fixed "package.json" "\"test:phase19-rust\""
  require_fixed "package.json" "\"test:phase19-source-guards\""
  require_fixed "package.json" "\"test:phase19-desktop\""
  require_fixed "package.json" "\"test:phase19\""
  require_fixed "package.json" "bash scripts/phase19-source-guards.sh"
}

scan_electron_semantic_boundary() {
  fail_matches \
    "Electron renderer/main/preload must not construct FFmpeg filter/export jobs for Phase 19 semantics" \
    "$ELECTRON_FFMPEG_OWNERSHIP_PATTERN" \
    "${ELECTRON_BOUNDARY_DIRS[@]}"
  fail_matches \
    "Electron renderer/main/preload must not own source-to-target retiming math" \
    "$RENDERER_RETIME_MATH_PATTERN" \
    "${ELECTRON_BOUNDARY_DIRS[@]}"
  fail_matches \
    "Electron renderer/main/preload must not own transition overlap or adjacency validation" \
    "$TRANSITION_VALIDATION_PATTERN" \
    "${ELECTRON_BOUNDARY_DIRS[@]}"
  fail_matches \
    "Electron renderer/main/preload must not evaluate effect, filter, mask, or blend rendering semantics" \
    "$EFFECT_EVALUATION_PATTERN" \
    "${ELECTRON_BOUNDARY_DIRS[@]}"
  fail_matches \
    "Electron renderer/main/preload must not own dirty-range, cache-key, or fingerprint semantics" \
    "$CACHE_FINGERPRINT_PATTERN" \
    "${ELECTRON_BOUNDARY_DIRS[@]}"
}

scan_no_fallback_success() {
  fail_matches \
    "product code/tests must not count DOM, artifact, CPU, mock, debug, fallback, or legacy output as Phase 19 success" \
    "$FALLBACK_SUCCESS_PATTERN" \
    "${ELECTRON_BOUNDARY_DIRS[@]}" \
    "${PRODUCT_TEST_DIRS[@]}"
  bash scripts/no-product-fallback-guards.sh >/dev/null
}

scan_provider_native_semantics() {
  fail_matches \
    "provider-native effect/filter/transition IDs must not become internal Phase 19 semantics" \
    "$PROVIDER_NATIVE_SEMANTIC_PATTERN" \
    "${CANONICAL_SEMANTIC_DIRS[@]}"
}

scan_pointer_save_loops() {
  fail_matches \
    "high-frequency pointer/drag/slider/scrub samples must not directly save, increment revision, or push undo entries" \
    "$POINTER_SAVE_LOOP_PATTERN" \
    "${ELECTRON_BOUNDARY_DIRS[@]}"
}

require_retiming_files() {
  require_file "crates/draft_commands/tests/retiming_commands.rs"
  require_file "crates/engine_core/tests/retiming.rs"
  require_file "crates/draft_commands/src/retiming.rs"
  require_file "crates/engine_core/src/time_mapping.rs"
  require_file "schemas/draft.schema.json"
  require_file "schemas/command.schema.json"
  require_file "apps/desktop-electron/src/generated/Draft.ts"
  require_file "apps/desktop-electron/src/generated/CommandResultEnvelope.ts"
  require_fixed "crates/draft_commands/tests/retiming_commands.rs" "phase19_"
  require_fixed "crates/engine_core/tests/retiming.rs" "phase19_"
  require_fixed "crates/draft_commands/src/retiming.rs" "SetSegmentRetime"
  require_fixed "crates/draft_commands/src/retiming.rs" "ClearSegmentRetime"
  require_fixed "crates/draft_commands/src/timeline.rs" "TimelineEditPayload::SetSegmentRetime"
  require_fixed "crates/draft_commands/src/timeline.rs" "TimelineEditPayload::ClearSegmentRetime"
  require_fixed "crates/engine_core/src/time_mapping.rs" "SegmentTimeMap"
  require_fixed "crates/engine_core/src/time_mapping.rs" "source_position_for_retime"
  require_fixed "crates/engine_core/src/time_mapping.rs" "retimed_source_range"
  require_fixed "crates/engine_core/src/time_mapping.rs" "audio_retime_diagnostic"
  require_fixed "schemas/draft.schema.json" "SegmentRetiming"
  require_fixed "schemas/draft.schema.json" "SpeedRatio"
  require_fixed "apps/desktop-electron/src/generated/Draft.ts" "export type SegmentRetiming"
  require_fixed "apps/desktop-electron/src/generated/Draft.ts" "export type SpeedRatio"
  require_fixed "apps/desktop-electron/src/generated/CommandResultEnvelope.ts" "setSegmentRetime"
  require_fixed "apps/desktop-electron/src/generated/CommandResultEnvelope.ts" "clearSegmentRetime"
  fail_matches \
    "generated retime contracts must not persist naked floating retime/time fields" \
    "$PERSISTED_RETIME_FLOAT_PATTERN" \
    "schemas/draft.schema.json" \
    "schemas/command.schema.json" \
    "apps/desktop-electron/src/generated/Draft.ts" \
    "apps/desktop-electron/src/generated/CommandEnvelope.ts" \
    "apps/desktop-electron/src/generated/CommandResultEnvelope.ts"
}

require_retiming_audio_files() {
  require_file "crates/audio_engine/tests/dsp_timeline.rs"
  require_fixed "crates/audio_engine/tests/dsp_timeline.rs" "phase19_"
}

require_transition_files() {
  require_file "crates/draft_commands/tests/transition_commands.rs"
  require_fixed "crates/draft_commands/tests/transition_commands.rs" "phase19_"
}

require_effect_files() {
  require_file "crates/draft_model/tests/production_effects_contracts.rs"
  require_file "crates/render_graph/tests/production_effects.rs"
  require_file "crates/realtime_preview_runtime/tests/production_effects.rs"
  require_file "crates/ffmpeg_compiler/tests/production_effects.rs"
  require_fixed "crates/draft_model/tests/production_effects_contracts.rs" "phase19_"
  require_fixed "crates/render_graph/tests/production_effects.rs" "phase19_"
  require_fixed "crates/realtime_preview_runtime/tests/production_effects.rs" "phase19_"
  require_fixed "crates/ffmpeg_compiler/tests/production_effects.rs" "phase19_"
}

require_mask_blend_files() {
  require_effect_files
  require_fixed "crates/realtime_preview_runtime/tests/production_effects.rs" "mask"
  require_fixed "crates/realtime_preview_runtime/tests/production_effects.rs" "blend"
}

require_ui_files() {
  require_file "apps/desktop-electron/tests/production-effects.spec.ts"
  require_fixed "apps/desktop-electron/tests/production-effects.spec.ts" "production-effects"
}

run_wave0() {
  run_self_test
  require_wave0_files
  scan_electron_semantic_boundary
  scan_no_fallback_success
  scan_provider_native_semantics
  scan_pointer_save_loops
  echo "phase19 source guards passed for wave0"
}

run_retiming() {
  run_wave0
  require_retiming_files
  echo "phase19 source guards passed for retiming"
}

run_retiming_audio() {
  run_retiming
  require_retiming_audio_files
  echo "phase19 source guards passed for retiming-audio"
}

run_transition() {
  run_wave0
  require_transition_files
  echo "phase19 source guards passed for transition"
}

run_effects() {
  run_wave0
  require_effect_files
  echo "phase19 source guards passed for effects"
}

run_mask_blend() {
  run_effects
  require_mask_blend_files
  echo "phase19 source guards passed for mask-blend"
}

run_ui() {
  run_wave0
  require_ui_files
  echo "phase19 source guards passed for ui"
}

run_full() {
  require_wave0_files
  require_retiming_files
  require_retiming_audio_files
  require_transition_files
  require_effect_files
  require_mask_blend_files
  require_ui_files
  scan_electron_semantic_boundary
  scan_no_fallback_success
  scan_provider_native_semantics
  scan_pointer_save_loops
  echo "phase19 source guards passed"
}

usage() {
  cat <<'USAGE'
Usage: bash scripts/phase19-source-guards.sh [--wave0|--retiming|--retiming-audio|--transition|--effects|--mask-blend|--ui|--self-test]

Modes:
  --self-test        Run negative guard injections only.
  --wave0           Check Wave 0 guard/script artifacts plus current architecture boundaries.
  --retiming        Add retiming semantic RED/implementation artifact checks.
  --retiming-audio  Add audio retiming artifact checks.
  --transition      Add transition artifact checks.
  --effects         Add effect/capability/preview/export artifact checks.
  --mask-blend      Add mask/blend artifact checks.
  --ui              Add desktop product E2E artifact checks.
  default/full      Require every Phase 19 artifact and run all scans.
USAGE
}

if [ "${1:-}" = "--" ]; then
  shift
fi

case "${1:-}" in
  --self-test)
    run_self_test
    ;;
  --wave0)
    run_wave0
    ;;
  --retiming)
    run_retiming
    ;;
  --retiming-audio)
    run_retiming_audio
    ;;
  --transition)
    run_transition
    ;;
  --effects)
    run_effects
    ;;
  --mask-blend)
    run_mask_blend
    ;;
  --ui)
    run_ui
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

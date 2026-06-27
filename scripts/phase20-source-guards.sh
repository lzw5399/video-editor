#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase20 source guard violation: $1" >&2
  exit 1
}

require_file() {
  local file="$1"
  [ -f "$file" ] || fail "missing required Phase 20 artifact ${file}"
}

require_fixed() {
  local file="$1"
  local text="$2"
  if [ -z "$(matches_for_fixed_text "$file" "$text")" ]; then
    fail "missing required text '${text}' in ${file}"
  fi
}

strip_comments() {
  rg -v '^[^:]+:[0-9]+:[[:space:]]*(//|/\*|\*)' \
    | rg -v '^[0-9]+:[[:space:]]*(//|/\*|\*)' \
    | rg -v '^\s*(//|/\*|\*)' \
    || true
}

matches_for_pattern() {
  local pattern="$1"
  shift
  rg -n --pcre2 "$pattern" "$@" 2>/dev/null | strip_comments
}

matches_for_fixed_text() {
  local file="$1"
  local text="$2"
  rg -n --fixed-strings "$text" "$file" 2>/dev/null | strip_comments
}

matches_for_multiline_pattern() {
  local pattern="$1"
  shift
  rg -n -U --pcre2 "$pattern" "$@" 2>/dev/null | strip_comments
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

fail_multiline_matches() {
  local message="$1"
  local pattern="$2"
  shift 2
  local matches
  matches="$(matches_for_multiline_pattern "$pattern" "$@" || true)"
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
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase20Violation.ts"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase20Violation.ts" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "$source" | sed 's|^|// |' >"$tmp_dir/CommentOnly.ts"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.ts" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

assert_multiline_pattern_rejects() {
  local description="$1"
  local pattern="$2"
  local source="$3"
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' RETURN
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase20Violation.ts"
  if [ -z "$(matches_for_multiline_pattern "$pattern" "$tmp_dir/InjectedPhase20Violation.ts" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "$source" | sed 's|^|// |' >"$tmp_dir/CommentOnly.ts"
  if [ -n "$(matches_for_multiline_pattern "$pattern" "$tmp_dir/CommentOnly.ts" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

assert_required_fixed_rejects_comment_only() {
  local description="$1"
  local required="$2"
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' RETURN
  printf '// %s\n' "$required" >"$tmp_dir/CommentOnly.ts"
  if [ -n "$(matches_for_fixed_text "$tmp_dir/CommentOnly.ts" "$required")" ]; then
    fail "required-text check matched comment-only ${description}"
  fi
  printf 'const activePhase20Token = "%s";\n' "$required" >"$tmp_dir/Active.ts"
  if [ -z "$(matches_for_fixed_text "$tmp_dir/Active.ts" "$required")" ]; then
    fail "required-text check did not catch active ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

PHASE20_TS_DRAFT_CONSTRUCTION_PATTERN='(?s)(?:\b(?:Array\.from|new\s+Array)\s*\([^)]*(?:180|540|PRODUCT_SEGMENTS_PER_TRACK|PRODUCT_TOTAL_SEGMENTS)[^)]*\).{0,1200}\b(?:segmentId|sourceTimerange|targetTimerange|materials\s*:|tracks\s*:|segments\s*:)|\bfor\s*\([^)]*(?:180|540|PRODUCT_SEGMENTS_PER_TRACK|PRODUCT_TOTAL_SEGMENTS)[^)]*\)\s*\{.{0,1200}\b(?:segments|tracks|materials)\.push\s*\(\s*\{.{0,1200}\b(?:segmentId|sourceTimerange|targetTimerange|materials\s*:|tracks\s*:|segments\s*:)|\bfunction\s+\w*(?:Segment|Track|Material|Draft)\w*\s*\([^)]*\)\s*(?::\s*(?:Segment|Track|Material|Draft)[A-Za-z0-9_<>,\s\[\]]*)?\{.{0,1200}\b(?:segments|tracks|materials)\.push\s*\(\s*\{.{0,1200}\b(?:segmentId|sourceTimerange|targetTimerange|materials\s*:|tracks\s*:|segments\s*:))'
PHASE20_RENDERER_FFMPEG_PATTERN='\b(?:buildRenderGraph|compileRenderGraph|compileFfmpeg|compileFfmpegJob|renderGraphToFfmpeg|filter_complex|filterComplex|ffmpegArgs|ffmpegCommand|ffmpegFilter|FfmpegJob|FfmpegExecutor|spawn\s*\(\s*["'\'']ffmpeg|execFile\s*\(\s*["'\'']ffmpeg|new\s+RenderGraph|new\s+Ffmpeg)\b'
PHASE20_FALLBACK_SUCCESS_PATTERN='\b(?:(?:fallback|mock|artifact|cpuReadback|cpuProbe|decodedCpu|domOverlay|domOnly|nativeVideo|firstFrame|fileExistsOnly|sourceOnly)[A-Za-z]*(?:Success|Succeeded|Satisfied|Accepted|EvidenceOk)|success[A-Za-z]*(?:Fallback|Mock|Artifact|Cpu|Dom|NativeVideo|FirstFrame|FileExists|SourceOnly))\b'
PHASE20_DEV_ONLY_UAT_PATTERN='\b(?:launchDevApp|startDevApp|devOnlyLongTimelineUat|devOnlyProductCloseout|test:real-workflow|pnpm\s+(?:--filter\s+@video-editor/desktop\s+)?(?:dev|exec\s+electron))\b'
PHASE20_ENABLED_TEST_MOCK_PATTERN='VIDEO_EDITOR_TEST_MOCK_(?:PREVIEW|EXPORT|ARTIFACT|AUDIO|RUNTIME_CAPABILITIES)[A-Za-z_]*:\s*["'\'']1["'\'']'
PHASE20_SOURCE_ONLY_EXPORT_PATTERN='\b(?:sourceOnlyExportEvidenceOk|sourceOnlyExportSuccess|fileExistsOnlyExportSuccess|exportFileExistsSuccess|metadataOnlyExportSuccess|firstFrameOnlyExportSuccess)\b'

PHASE20_RUST_FILES=(
  "crates/testkit/src/large_timeline.rs"
  "crates/testkit/src/bin/phase20_long_fixture.rs"
  "crates/testkit/tests/long_timeline_product_fixture.rs"
  "crates/testkit/tests/large_timeline_incremental.rs"
)

PHASE20_PLAYWRIGHT_FILES=(
  "apps/desktop-electron/tests/helpers/longTimelineFixture.ts"
  "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts"
  "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts"
)

PHASE20_PRODUCT_SOURCE_FILES=(
  "apps/desktop-electron/src/main"
  "apps/desktop-electron/src/preload"
  "apps/desktop-electron/src/renderer"
  "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts"
  "apps/desktop-electron/tests/helpers/longTimelineFixture.ts"
)

run_self_test() {
  assert_required_fixed_rejects_comment_only \
    "Phase 20 required production preview token" \
    "renderGraphGpuComposited"
  assert_multiline_pattern_rejects \
    "TypeScript-owned 540-segment draft construction" \
    "$PHASE20_TS_DRAFT_CONSTRUCTION_PATTERN" \
    'const tracks = Array.from({ length: 180 }, (_, index) => ({
  segmentId: `video-${index}`,
  sourceTimerange: { start: 0, duration: 1000000 },
  targetTimerange: { start: index * 1000000, duration: 1000000 }
}));'
  assert_multiline_pattern_rejects \
    "TypeScript-owned loop/push long draft construction" \
    "$PHASE20_TS_DRAFT_CONSTRUCTION_PATTERN" \
    'const segments = [];
for (let index = 0; index < 540; index += 1) {
  segments.push({
    segmentId: `video-${index}`,
    sourceTimerange: { start: 0, duration: 1000000 },
    targetTimerange: { start: index * 1000000, duration: 1000000 }
  });
}'
  assert_multiline_pattern_rejects \
    "TypeScript-owned helper long draft construction" \
    "$PHASE20_TS_DRAFT_CONSTRUCTION_PATTERN" \
    'function buildLongSegments(): Segment[] {
  const segments = [];
  for (let index = 0; index < PRODUCT_TOTAL_SEGMENTS; index += 1) {
    segments.push({
      segmentId: `segment-${index}`,
      targetTimerange: { start: index * 1000000, duration: 1000000 }
    });
  }
  return segments;
}'
  assert_pattern_rejects \
    "renderer-owned FFmpeg/render graph semantics" \
    "$PHASE20_RENDERER_FFMPEG_PATTERN" \
    'const ffmpegArgs = ["-filter_complex", renderGraphScript]; compileFfmpegJob(ffmpegArgs);'
  assert_pattern_rejects \
    "fallback evidence as product success" \
    "$PHASE20_FALLBACK_SUCCESS_PATTERN" \
    'const artifactPreviewSuccess = true;'
  assert_pattern_rejects \
    "source-only export success" \
    "$PHASE20_SOURCE_ONLY_EXPORT_PATTERN" \
    'const fileExistsOnlyExportSuccess = existsSync(outputPath);'
  assert_pattern_rejects \
    "dev-only product closeout" \
    "$PHASE20_DEV_ONLY_UAT_PATTERN" \
    'const app = await launchDevApp({ phase20: true });'
  assert_pattern_rejects \
    "enabled product mock switch" \
    "$PHASE20_ENABLED_TEST_MOCK_PATTERN" \
    'VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS: "1"'
  echo "phase20 source guard self-test passed"
}

require_phase20_rust_artifacts() {
  for file in "${PHASE20_RUST_FILES[@]}"; do
    require_file "$file"
  done

  require_fixed "crates/testkit/src/large_timeline.rs" "PHASE20_PRODUCT_SEGMENTS_PER_TRACK"
  require_fixed "crates/testkit/src/large_timeline.rs" "PHASE20_BLOCKING_SEGMENTS_PER_TRACK"
  require_fixed "crates/testkit/src/large_timeline.rs" "PHASE20_DIAGNOSTIC_SEGMENTS_PER_TRACK"
  require_fixed "crates/testkit/src/large_timeline.rs" "PHASE20_SEGMENT_DURATION_US"
  require_fixed "crates/testkit/src/large_timeline.rs" "Phase20ProductMediaUris"
  require_fixed "crates/testkit/src/large_timeline.rs" "phase20_product_timeline_config"
  require_fixed "crates/testkit/src/large_timeline.rs" "phase20_blocking_timeline_config"
  require_fixed "crates/testkit/src/large_timeline.rs" "phase20_diagnostic_timeline_config"
  require_fixed "crates/testkit/src/large_timeline.rs" "build_phase20_product_timeline"
  require_fixed "crates/testkit/src/bin/phase20_long_fixture.rs" "save_project_bundle"
  require_fixed "crates/testkit/src/bin/phase20_long_fixture.rs" "open_project_bundle"
  require_fixed "crates/testkit/src/bin/phase20_long_fixture.rs" "segmentsPerTrack"
  require_fixed "crates/testkit/tests/long_timeline_product_fixture.rs" "phase20_materializer_writes_reopenable_canonical_bundle"
  require_fixed "crates/testkit/tests/long_timeline_product_fixture.rs" "phase20_materializer_project_json_excludes_derived_artifacts"
  require_fixed "crates/testkit/tests/long_timeline_product_fixture.rs" "PHASE20_PRODUCT_SEGMENTS_PER_TRACK"
  require_fixed "crates/testkit/tests/large_timeline_incremental.rs" "phase20_blocking_1000_segments_per_track_keeps_localized_diff_bounded"
  require_fixed "crates/testkit/tests/large_timeline_incremental.rs" "phase20_diagnostic_3000_segments_per_track_reports_structural_stats"
  require_fixed "crates/testkit/tests/large_timeline_incremental.rs" "#[ignore"
}

require_phase20_playwright_artifacts() {
  for file in "${PHASE20_PLAYWRIGHT_FILES[@]}"; do
    require_file "$file"
  done

  require_fixed "apps/desktop-electron/tests/helpers/longTimelineFixture.ts" "generatePhase20LongTimelineFixture"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineFixture.ts" "phase20_long_fixture"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineFixture.ts" "PRODUCT_SEGMENTS_PER_TRACK"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineFixture.ts" "PRODUCT_TOTAL_SEGMENTS"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineFixture.ts" "materializerSummary"

  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "expectCanonicalDraftStable"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "expectNoDerivedArtifactPollution"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "expectPhase20PreviewProductionEvidence"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "expectPhase20ExportMedia"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "requestProjectSessionPreviewFrameCount"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "fallbackActive"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "renderGraphGpuComposited"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "frameDisplay"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "probeMediaRuntime"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "bundled"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "readFfprobeJson"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "sampleExportFrames"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "sampledFramesJsonPath"
  require_fixed "apps/desktop-electron/tests/helpers/longTimelineEvidence.ts" "minDistinctSampleHashes"

  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "launchPackagedApp"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "generatePhase20LongTimelineFixture"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "renderGraphGpuComposited"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "expectPhase20ExportMedia"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "expectCanonicalDraftStable"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "reopenCycles: 2"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "exportValidations: 2"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "firstExport"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "secondExport"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "startPhase20ExportPressureThroughProductUi"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "commitProjectInteraction"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "cancelProjectInteraction"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "queueLatencyUs"
  require_fixed "apps/desktop-electron/tests/product-long-timeline-uat.spec.ts" "fallbackCount"
}

scan_phase20_source_boundaries() {
  fail_multiline_matches \
    "Phase 20 TypeScript must orchestrate the Rust materializer instead of constructing the 540-segment draft" \
    "$PHASE20_TS_DRAFT_CONSTRUCTION_PATTERN" \
    "${PHASE20_PLAYWRIGHT_FILES[@]}"
  fail_matches \
    "Electron and Phase 20 UAT code must not own FFmpeg/render graph construction semantics" \
    "$PHASE20_RENDERER_FFMPEG_PATTERN" \
    "${PHASE20_PRODUCT_SOURCE_FILES[@]}"
  fail_matches \
    "Phase 20 product success must not be satisfied by fallback/mock/artifact/CPU/DOM/native-video/first-frame/file-exists/source-only evidence" \
    "$PHASE20_FALLBACK_SUCCESS_PATTERN" \
    "${PHASE20_PLAYWRIGHT_FILES[@]}" \
    "apps/desktop-electron/src/main" \
    "apps/desktop-electron/src/renderer"
  fail_matches \
    "Phase 20 export success must not be source-only or file-exists-only" \
    "$PHASE20_SOURCE_ONLY_EXPORT_PATTERN" \
    "${PHASE20_PLAYWRIGHT_FILES[@]}"
  fail_matches \
    "Phase 20 packaged product UAT must not close on dev-only Electron" \
    "$PHASE20_DEV_ONLY_UAT_PATTERN" \
    "${PHASE20_PLAYWRIGHT_FILES[@]}"
  fail_matches \
    "Phase 20 packaged product UAT must not enable mock preview/export/artifact/audio/runtime success switches" \
    "$PHASE20_ENABLED_TEST_MOCK_PATTERN" \
    "${PHASE20_PLAYWRIGHT_FILES[@]}"
}

run_full() {
  run_self_test
  require_phase20_rust_artifacts
  require_phase20_playwright_artifacts
  scan_phase20_source_boundaries
  echo "phase20 source guards passed"
}

usage() {
  cat <<'USAGE'
Usage: bash scripts/phase20-source-guards.sh [--self-test]

Modes:
  --self-test  Run injected negative guard checks only.
  default      Require Phase 20 Rust/product artifacts and scan source boundaries.
USAGE
}

if [ "${1:-}" = "--" ]; then
  shift
fi

case "${1:-}" in
  --self-test)
    run_self_test
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

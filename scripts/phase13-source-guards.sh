#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase13-source-guards: rg is required" >&2
  exit 1
fi

RENDERER_DIR="apps/desktop-electron/src/renderer"
RENDERER_WORKSPACE_DIR="apps/desktop-electron/src/renderer/workspace"
PACKAGE_JSON="package.json"
PHASE13_TEST_TARGETS=(
  "crates/draft_commands/tests/command_delta.rs"
  "crates/render_graph/tests/node_identity.rs"
  "crates/preview_service/tests/dirty_propagation.rs"
  "crates/testkit/tests/large_timeline_incremental.rs"
  "crates/testkit/tests/preview_export_parity.rs"
)
SEMANTIC_CONTRACT_SURFACES=(
  "crates/draft_model/src"
  "crates/draft_commands/src"
  "crates/render_graph/src"
  "crates/preview_service/src"
  "schemas"
  "apps/desktop-electron/src/generated"
)
CANONICAL_DRAFT_SURFACES=(
  "schemas/draft.schema.json"
  "fixtures/draft/positive"
  "apps/desktop-electron/src/generated/Draft.ts"
)

fail() {
  echo "phase13-source-guards: $1" >&2
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
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase13Violation.ts"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase13Violation.ts" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "// $source" >"$tmp_dir/CommentOnly.ts"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.ts" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

RENDERER_GRAPH_DIRTY_CACHE_PATTERN='\b(?:RenderGraphNodeId|renderGraphNodeId|RenderGraphDiff|renderGraphDiff|graphDiff|dirtyRanges?|DirtyRange|dirtyRangePropagation|changedGraphNodeIds|previewCacheKey|cacheKey|cacheFingerprint|semanticFingerprint|nodeFingerprint|invalidationDecision|invalidateDirtyRange|artifactSchemaVersion|generatorVersion)\b'
RENDERER_FFMPEG_PATTERN='\b(?:FfmpegJob|FfmpegExecutor|ffmpegArgs|ffprobeArgs|filter_complex|filterComplex|ffmpegScripts|exportScript|AssSidecar|child_process|execFile|exec\s*\(|spawn\s*\()\b'
FLOAT_TIME_PATTERN='\b(?:targetTimeSeconds|target_time_seconds|timelineSeconds|timeline_seconds|durationSeconds|duration_seconds|sourceTimeSeconds|source_time_seconds|targetTimerangeSeconds|target_timerange_seconds|sourceTimerangeSeconds|source_timerange_seconds|seconds\s*:\s*f32|seconds\s*:\s*f64)\b'
DERIVED_ARTIFACT_PATTERN='\b(?:previewCaches?|previewArtifacts?|renderGraph|graphSnapshots?|ffmpegScripts?|proxyFiles?|thumbnailPath|waveformPath|artifactStore|artifact-store|artifact_store|derivedArtifacts?)\b'
PHASE14_OR_16_SCOPE_PATTERN='\b(?:artifact-store\.sqlite|artifactStoreSqlite|rusqlite|sqlx|CREATE TABLE|JobScheduler|priorityQueue|starvation|backpressure)\b'

assert_pattern_rejects \
  "renderer-owned graph/dirty/cache decisions" \
  "$RENDERER_GRAPH_DIRTY_CACHE_PATTERN" \
  "const previewCacheKey = buildKey(RenderGraphNodeId.from(segment), dirtyRanges);"
assert_pattern_rejects \
  "renderer-owned FFmpeg command construction" \
  "$RENDERER_FFMPEG_PATTERN" \
  "const ffmpegArgs = ['-filter_complex', graphScript];"
assert_pattern_rejects \
  "floating-point timeline contract time" \
  "$FLOAT_TIME_PATTERN" \
  "type BadRequest = { targetTimeSeconds: number };"
assert_pattern_rejects \
  "derived artifact leakage into canonical draft" \
  "$DERIVED_ARTIFACT_PATTERN" \
  "const projectJson = { previewCaches: [], renderGraph: {} };"

for script in \
  "test:phase13-rust" \
  "test:phase13-source-guards" \
  "test:phase13"; do
  require_fixed "$PACKAGE_JSON" "\"${script}\""
done
require_fixed "$PACKAGE_JSON" "bash scripts/phase13-source-guards.sh"
require_fixed "$PACKAGE_JSON" "cargo test -p draft_commands --test command_delta -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p render_graph --test node_identity -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p preview_service --test dirty_propagation -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p testkit large_timeline -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p testkit large_timeline_incremental -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p testkit preview_export_parity -- --nocapture"
require_fixed "$PACKAGE_JSON" "pnpm run test:contracts"

require_fixed "apps/desktop-electron/src/generated/CommandEnvelope.ts" "export type DirtyRange ="
require_fixed "apps/desktop-electron/src/generated/CommandEnvelope.ts" "export type ExportPrepDirtyFacts ="
require_fixed "apps/desktop-electron/src/generated/CommandResultEnvelope.ts" "export type CommandDelta ="
require_fixed "apps/desktop-electron/src/generated/CommandResultEnvelope.ts" "export type TimelineCommandResponse ="
require_fixed "apps/desktop-electron/src/generated/CommandResultEnvelope.ts" "export type ExportPrepDirtyFacts ="
require_fixed "schemas/command.schema.json" "\"DirtyRange\""
require_fixed "schemas/command.schema.json" "\"ExportPrepDirtyFacts\""

fail_matches \
  "generic preview cache invalidation command must not be public; Rust session/export services own preview cache invalidation" \
  "invalidatePreviewCache|InvalidatePreviewCacheCommandPayload|PreviewCacheEntryRef" \
  "apps/desktop-electron/src/generated/CommandEnvelope.ts" \
  "schemas/command.schema.json"

for target in "${PHASE13_TEST_TARGETS[@]}"; do
  [ -f "$target" ] || fail "missing Phase 13 test target ${target}"
done

fail_matches \
  "renderer must not compute dirty ranges, graph diffs, graph node IDs, cache keys, fingerprints, or invalidation decisions" \
  "$RENDERER_GRAPH_DIRTY_CACHE_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts' \
  --glob '!viewModel.ts'

fail_matches \
  "renderer must not construct FFmpeg/ffprobe process commands or filter scripts" \
  "$RENDERER_FFMPEG_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "persisted and binding-facing timeline time must avoid naked floating-point seconds fields" \
  "$FLOAT_TIME_PATTERN" \
  "${SEMANTIC_CONTRACT_SURFACES[@]}" \
  "$RENDERER_WORKSPACE_DIR"

fail_matches \
  "canonical draft schema, positive fixtures, and generated draft contracts must not contain derived artifact metadata" \
  "$DERIVED_ARTIFACT_PATTERN" \
  "${CANONICAL_DRAFT_SURFACES[@]}"

fail_matches \
  "Phase 13 must not add Phase 14 SQLite artifact store or Phase 16 scheduler behavior" \
  "$PHASE14_OR_16_SCOPE_PATTERN" \
  crates \
  apps/desktop-electron/src \
  scripts \
  --glob '!crates/artifact_store/**' \
  --glob '!crates/bindings_node/tests/artifact_store_commands.rs' \
  --glob '!scripts/phase*-source-guards.sh'

git diff --exit-code schemas apps/desktop-electron/src/generated >/dev/null

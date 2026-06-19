#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase14-source-guards: rg is required" >&2
  exit 1
fi

RENDERER_DIR="apps/desktop-electron/src/renderer"
PACKAGE_JSON="package.json"
CANONICAL_DRAFT_SURFACES=(
  "schemas/draft.schema.json"
  "fixtures/draft/positive"
  "apps/desktop-electron/src/generated/Draft.ts"
)

fail() {
  echo "phase14-source-guards: $1" >&2
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
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase14Violation.ts"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase14Violation.ts" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "// $source" >"$tmp_dir/CommentOnly.ts"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.ts" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

RENDERER_ARTIFACT_ROOT_PATTERN='\b(?:artifactRoot|artifactStoreRoot|artifactBlobRoot|derivedRoot|derivedArtifactRoot|blobRoot|artifactStorePath|artifactStoreDbPath|artifact-store\.sqlite|\.veproj/derived|rusqlite|sqlite3?|SQLite|CREATE TABLE|INSERT INTO artifact|UPDATE artifact)\b'
RENDERER_ARTIFACT_INTERNAL_PATTERN='\b(?:artifactStore|artifact_store|ArtifactStore|BlobStore|BlobWriteIntent|BlobRecord|derivedArtifacts?|artifactTombstone|syncManifestEntry|quotaState|generationChunk|generationJob)\b'
RENDERER_CACHE_DECISION_PATTERN='\b(?:cacheKey|previewCacheKey|artifactKey|cacheFingerprint|semanticFingerprint|sourceFingerprint|graphFingerprint|runtimeCapabilityFingerprint|outputProfileFingerprint|blobFingerprint|fingerprintBytes|fingerprintFile|RenderGraphNodeId|renderGraphNodeId|graphNodeKeys?|dirtyRanges?|DirtyRange|dirtyDomains?|invalidationDecision|invalidateDirtyRange)\b'
RENDERER_FFMPEG_PATTERN='\b(?:FfmpegJob|FfmpegExecutor|ffmpegArgs|ffprobeArgs|filter_complex|filterComplex|ffmpegScripts|exportScript|AssSidecar|child_process|execFile|exec\s*\(|spawn\s*\()\b'
CANONICAL_DERIVED_PATTERN='\b(?:artifactStore|artifact-store|artifact_store|derivedArtifacts?|previewCaches?|graphSnapshots?|waveformPath|thumbnailPath|proxyFiles?|ffmpegScripts?)\b'

assert_pattern_rejects \
  "renderer artifact-root computation" \
  "$RENDERER_ARTIFACT_ROOT_PATTERN" \
  "const artifactRoot = path.join(projectPath, '.veproj/derived');"
assert_pattern_rejects \
  "renderer SQLite/internal artifact store leakage" \
  "$RENDERER_ARTIFACT_INTERNAL_PATTERN" \
  "const store = new ArtifactStore('artifact-store.sqlite');"
assert_pattern_rejects \
  "renderer-owned cache key, fingerprint, graph, dirty, or invalidation decisions" \
  "$RENDERER_CACHE_DECISION_PATTERN" \
  "const cacheKey = sourceFingerprint + renderGraphNodeId + dirtyRange.start;"
assert_pattern_rejects \
  "renderer-owned FFmpeg command construction" \
  "$RENDERER_FFMPEG_PATTERN" \
  "const ffmpegArgs = ['-filter_complex', graphScript];"
assert_pattern_rejects \
  "canonical draft derived artifact leakage" \
  "$CANONICAL_DERIVED_PATTERN" \
  "const projectJson = { derivedArtifacts: [], previewCaches: [] };"

require_fixed "$PACKAGE_JSON" "\"test:phase14-rust\""
require_fixed "$PACKAGE_JSON" "cargo test -p artifact_store sqlite_schema -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p artifact_store blob_store -- --nocapture"
require_fixed "$PACKAGE_JSON" "\"test:phase14-source-guards\""
require_fixed "$PACKAGE_JSON" "bash scripts/phase14-source-guards.sh"
require_fixed "$PACKAGE_JSON" "\"test:phase14\""
require_fixed "$PACKAGE_JSON" "pnpm run test:phase14-rust && pnpm run test:phase14-source-guards"

fail_matches \
  "renderer must not compute artifact roots, blob paths, SQLite paths, or SQL for derived artifacts" \
  "$RENDERER_ARTIFACT_ROOT_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "renderer must not own artifact store/blob/generation/quota/sync internals" \
  "$RENDERER_ARTIFACT_INTERNAL_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts' \
  --glob '!viewModel.ts'

fail_matches \
  "renderer must not compute cache keys, fingerprints, graph node IDs, dirty ranges, or invalidation decisions" \
  "$RENDERER_CACHE_DECISION_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts' \
  --glob '!viewModel.ts'

fail_matches \
  "renderer must not construct FFmpeg/ffprobe process commands or filter scripts" \
  "$RENDERER_FFMPEG_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "canonical draft schema, positive fixtures, and generated draft contracts must not contain derived artifact metadata" \
  "$CANONICAL_DERIVED_PATTERN" \
  "${CANONICAL_DRAFT_SURFACES[@]}"

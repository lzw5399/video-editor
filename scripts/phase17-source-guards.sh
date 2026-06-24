#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase17 source guard violation: $1" >&2
  exit 1
}

require_file() {
  local file="$1"
  [ -f "$file" ] || fail "missing required file ${file}"
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
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase17Violation.ts"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase17Violation.ts" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "// $source" >"$tmp_dir/CommentOnly.ts"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.ts" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

CORE_BOUNDARY_PATHS=(
  "crates/draft_model/src"
  "crates/draft_commands/src"
  "crates/engine_core/src"
  "crates/render_graph/src"
  "crates/ffmpeg_compiler/src"
  "crates/preview_service/src"
  "crates/realtime_preview_runtime/src"
  "crates/project_store/src"
  "crates/bindings_node/src"
  "apps/desktop-electron/src"
)

CANONICAL_CONTRACT_PATHS=(
  "crates/draft_import/src"
  "crates/draft_model/src"
  "crates/project_store/src"
  "schemas/adaptation-report.schema.json"
  "apps/desktop-electron/src/generated/TemplateImport.ts"
)

PROVIDER_LEAKAGE_PATTERN='\b(?:Kaipai|kaipai|CapCut|capcut|templateId|recipeId|rawFormula|formulaJson|recognizerOutput|androidWorker|AndroidWorker|dcoin)\b'
REMOTE_RUNTIME_PATTERN='https?://|remoteRenderUrl|renderUrl|signedUrl|cdnUrl|downloadUrl'
LIVE_PROVIDER_PATTERN='api\.kaipai|kaipai\.com|\b(?:accessToken|authorizationToken|providerToken|signedUrl|cookieHeader|AndroidWorker|androidWorker|adb|emulator)\b'
CANONICAL_RAW_PATTERN='\b(?:templateId|recipeId|rawFormula|formulaJson|formulaBundle|recognizerOutput|providerRenderSemantic|safeArea)\b'
FALLBACK_SUCCESS_PATTERN='\b(?:mockPreviewSuccess|artifactPreviewSuccess|cpuPreviewSuccess|androidOracleSuccess|fallbackPreviewSuccess)\b'

assert_pattern_rejects \
  "provider-specific render leakage" \
  "$PROVIDER_LEAKAGE_PATTERN" \
  'const templateId = "provider-render-semantic";'
assert_pattern_rejects \
  "remote runtime dependency" \
  "$REMOTE_RUNTIME_PATTERN" \
  'const renderUrl = "https://provider.invalid/render.mp4";'
assert_pattern_rejects \
  "live provider API dependency" \
  "$LIVE_PROVIDER_PATTERN" \
  'const accessToken = "provider-token";'
assert_pattern_rejects \
  "android worker dependency" \
  "$LIVE_PROVIDER_PATTERN" \
  'const androidWorker = "adb";'
assert_pattern_rejects \
  "raw formula canonical field" \
  "$CANONICAL_RAW_PATTERN" \
  'const rawFormula = {};'
assert_pattern_rejects \
  "fallback success evidence" \
  "$FALLBACK_SUCCESS_PATTERN" \
  'const mockPreviewSuccess = true;'

require_file "crates/draft_import/src/adaptation_report.rs"
require_file "crates/draft_import/tests/adaptation_report.rs"
require_file "crates/draft_import/tests/schema_exports.rs"
require_file "schemas/adaptation-report.schema.json"
require_file "apps/desktop-electron/src/generated/TemplateImport.ts"
require_file "scripts/no-product-fallback-guards.sh"

require_fixed "crates/draft_import/src/adaptation_report.rs" "pub enum AdaptationStatus"
require_fixed "crates/draft_import/src/adaptation_report.rs" "NeedsNativeEffect"
require_fixed "crates/draft_import/src/adaptation_report.rs" "ExternalProvenanceRef"
require_fixed "crates/draft_import/tests/adaptation_report.rs" "adaptation_report_summary_counts_every_status"
require_fixed "crates/draft_import/tests/schema_exports.rs" "schema_exports_generated_adaptation_report_contracts_from_rust"
require_fixed "schemas/adaptation-report.schema.json" "\"missingResource\""
require_fixed "schemas/adaptation-report.schema.json" "\"needsNativeEffect\""
require_fixed "schemas/adaptation-report.schema.json" "\"additionalProperties\": false"
require_fixed "apps/desktop-electron/src/generated/TemplateImport.ts" "export type AdaptationReport"
require_fixed "package.json" "\"test:phase17-source-guards\""
require_fixed "package.json" "bash scripts/phase17-source-guards.sh"
require_fixed "package.json" "\"test:phase17-rust\""
require_fixed "package.json" "cargo test -p draft_import adaptation_report -- --nocapture"
require_fixed "package.json" "cargo test -p draft_import schema_exports -- --nocapture"

provider_leakage_matches="$(
  matches_for_pattern "$PROVIDER_LEAKAGE_PATTERN" "${CORE_BOUNDARY_PATHS[@]}" --glob '!*.svg' \
    | rg -v --pcre2 '^crates/bindings_node/src/(project_session_service|lib)\.rs:[0-9]+:.*(?:import|Import|formula|Formula|adapter_kaipai|KaipaiFormulaBundle|KaipaiImportOptions|map_kaipai_bundle_to_import_plan)' \
    || true
)"
if [ -n "$provider_leakage_matches" ]; then
  printf '%s\n' "$provider_leakage_matches" >&2
  fail "core/render/export/session paths must not contain provider-specific render semantics"
fi

fail_matches \
  "runtime paths must not depend on remote template/render URLs" \
  "$REMOTE_RUNTIME_PATTERN" \
  "${CORE_BOUNDARY_PATHS[@]}" \
  --glob '!*.svg'

fail_matches \
  "product/core paths must not depend on live provider APIs, credentials, or Android worker tooling" \
  "$LIVE_PROVIDER_PATTERN" \
  "${CORE_BOUNDARY_PATHS[@]}" \
  --glob '!*.svg'

fail_matches \
  "canonical import/report artifacts must not expose raw provider formula fields as semantics" \
  "$CANONICAL_RAW_PATTERN" \
  "${CANONICAL_CONTRACT_PATHS[@]}"

fail_matches \
  "product success paths must not claim fallback/mock/artifact/CPU/Android success evidence" \
  "$FALLBACK_SUCCESS_PATTERN" \
  "apps/desktop-electron/src" \
  "apps/desktop-electron/tests" \
  "crates/bindings_node/src" \
  "crates/realtime_preview_runtime/src" \
  --glob '!*.svg'

bash scripts/no-product-fallback-guards.sh

echo "phase17 source guards passed"

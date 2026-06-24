#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase17.1 source guard violation: $1" >&2
  exit 1
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

assert_pattern_rejects() {
  local description="$1"
  local pattern="$2"
  local source="$3"
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' RETURN
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase171Violation.tsx"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase171Violation.tsx" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "// $source" >"$tmp_dir/CommentOnly.tsx"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.tsx" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

HIGH_FREQUENCY_CANONICAL_INTENT_PATTERN='(?:drag|Drag|slider|Slider|scrub|Scrub|pointer|Pointer|PreviewChange|queue[A-Za-z]+Update|flush[A-Za-z]+Update).*executeProject(?:Timeline)?Intent\s*\('
INTENT_KIND_INVALIDATION_PATTERN='previewAffectingIntentNeedsRefresh|\bswitch\s*\([^)]*(?:intent|intentKind|intent\.kind)[^)]*\)'
DOM_ONLY_PREVIEW_EVIDENCE_PATTERN='\b(?:domOnly|DOMOnly|ghostPreview|previewGhost|overlayOnly|domOverlayOnly)(?:Preview)?(?:Success|Evidence|Moved)?\b|toHaveCSS\s*\(\s*["'\''](?:transform|left|top)["'\'']'
UNSUPPORTED_VISIBLE_HIGH_FREQUENCY_CONTROL_PATTERN='\b(?:segment-transition-handle|segment-fade-handle|transition-drag-handle|fade-drag-handle|crop-drag-handle|text-box-resize-handle)\b'

assert_pattern_rejects \
  "high-frequency canonical timeline intent loop" \
  "$HIGH_FREQUENCY_CANONICAL_INTENT_PATTERN" \
  'function handleDragMove() { executeProjectTimelineIntent({ kind: "moveSelectedSegmentIntent" }, "拖拽"); }'
assert_pattern_rejects \
  "intent-kind preview invalidation switch" \
  "$INTENT_KIND_INVALIDATION_PATTERN" \
  'function previewAffectingIntentNeedsRefresh(intentKind: string) { switch (intentKind) { case "moveSelectedSegmentIntent": return true; } }'
assert_pattern_rejects \
  "DOM-only ghost preview success evidence" \
  "$DOM_ONLY_PREVIEW_EVIDENCE_PATTERN" \
  'const domOnlyPreviewSuccess = await previewOverlay.locator(".ghost").isVisible();'
assert_pattern_rejects \
  "unsupported visible high-frequency control" \
  "$UNSUPPORTED_VISIBLE_HIGH_FREQUENCY_CONTROL_PATTERN" \
  '<button className="segment-transition-handle" aria-label="转场拖拽" />'

fail "phase17.1 source guard implementation pending"

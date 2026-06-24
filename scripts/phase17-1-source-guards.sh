#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase17.1 source guard violation: $1" >&2
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

HIGH_FREQUENCY_INTERACTION_CLASSES=(
  "preview canvas move"
  "preview canvas rotate"
  "preview canvas scale"
  "preview canvas crop"
  "preview canvas text-box"
  "timeline move"
  "timeline trim"
  "timeline cross-track"
  "timeline transition"
  "timeline fade"
  "playhead scrub"
  "ruler scrub"
  "timecode scrub"
  "inspector visual drag"
  "inspector text drag"
  "inspector audio drag"
  "keyframe marker edit"
  "keyframe value edit"
  "template report navigation"
)

WORKSPACE_HIGH_FREQUENCY_FILES=(
  "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"
  "apps/desktop-electron/src/renderer/workspace/Timeline.tsx"
  "apps/desktop-electron/src/renderer/workspace/Inspector.tsx"
  "apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx"
)

INTERACTION_E2E_FILES=(
  "apps/desktop-electron/tests/interaction-preview-inspector.spec.ts"
  "apps/desktop-electron/tests/interaction-timeline-keyframe.spec.ts"
  "apps/desktop-electron/tests/template-import.spec.ts"
  "apps/desktop-electron/tests/ui-regression.spec.ts"
)

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

for file in \
  "apps/desktop-electron/src/renderer/App.tsx" \
  "apps/desktop-electron/src/renderer/workspace/projectInteraction.ts" \
  "${WORKSPACE_HIGH_FREQUENCY_FILES[@]}" \
  "${INTERACTION_E2E_FILES[@]}" \
  "docs/no-product-fallback-policy.md" \
  "package.json"; do
  require_file "$file"
done

for interaction_class in "${HIGH_FREQUENCY_INTERACTION_CLASSES[@]}"; do
  case "$interaction_class" in
    "preview canvas move"|"preview canvas rotate"|"preview canvas scale")
      require_fixed "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx" "selectedSegmentVisual"
      ;;
    "preview canvas crop")
      require_fixed "apps/desktop-electron/src/renderer/workspace/Inspector.tsx" "visual-crop-grid"
      require_fixed "apps/desktop-electron/tests/interaction-timeline-keyframe.spec.ts" ".segment-transition-handle, .segment-fade-handle"
      ;;
    "preview canvas text-box")
      require_fixed "apps/desktop-electron/src/renderer/workspace/Inspector.tsx" "selectedText"
      ;;
    "timeline move"|"timeline trim"|"timeline cross-track")
      require_fixed "apps/desktop-electron/src/renderer/workspace/Timeline.tsx" "timelineMoveTrim"
      ;;
    "timeline transition"|"timeline fade")
      require_fixed "apps/desktop-electron/tests/interaction-timeline-keyframe.spec.ts" ".segment-transition-handle, .segment-fade-handle"
      ;;
    "playhead scrub"|"ruler scrub")
      require_fixed "apps/desktop-electron/src/renderer/workspace/Timeline.tsx" "playheadScrub"
      ;;
    "timecode scrub")
      require_fixed "apps/desktop-electron/src/renderer/App.tsx" "handleSeekPlayhead"
      ;;
    "inspector visual drag")
      require_fixed "apps/desktop-electron/src/renderer/workspace/Inspector.tsx" "selectedSegmentVisual"
      ;;
    "inspector text drag")
      require_fixed "apps/desktop-electron/src/renderer/workspace/Inspector.tsx" "selectedText"
      ;;
    "inspector audio drag")
      require_fixed "apps/desktop-electron/src/renderer/workspace/Inspector.tsx" "selectedSegmentAudio"
      ;;
    "keyframe marker edit"|"keyframe value edit")
      require_fixed "apps/desktop-electron/src/renderer/workspace/Timeline.tsx" "keyframeEdit"
      require_fixed "apps/desktop-electron/src/renderer/workspace/Inspector.tsx" "keyframeEdit"
      ;;
    "template report navigation")
      require_fixed "apps/desktop-electron/src/renderer/App.tsx" "scheduleTemplateReportNavigationFlush"
      require_fixed "apps/desktop-electron/tests/template-import.spec.ts" "rapid report row navigation should coalesce preview seeks"
      ;;
    *)
      fail "unhandled high-frequency interaction inventory row: ${interaction_class}"
      ;;
  esac
done

require_fixed "package.json" "\"test:phase17-1:guards\""
require_fixed "package.json" "bash scripts/phase17-1-source-guards.sh"
require_fixed "docs/no-product-fallback-policy.md" "generated colors, screenshots, or DOM overlays that are not the presented video"

require_fixed "apps/desktop-electron/src/renderer/App.tsx" "ProjectInteractionController"
require_fixed "apps/desktop-electron/src/renderer/App.tsx" "beginProjectInteractionSession"
require_fixed "apps/desktop-electron/src/renderer/App.tsx" "updateProjectInteractionSession"
require_fixed "apps/desktop-electron/src/renderer/App.tsx" "commitProjectInteractionSession"
require_fixed "apps/desktop-electron/src/renderer/App.tsx" "cancelProjectInteractionSession"
require_fixed "apps/desktop-electron/src/renderer/App.tsx" "previewRefreshTargetFromDelta"
require_fixed "apps/desktop-electron/src/renderer/App.tsx" "commandDeltaAffectsRealtimePreview"
require_fixed "apps/desktop-electron/src/renderer/App.tsx" "commandDeltaInvalidatesDerivedState"
require_fixed "apps/desktop-electron/src/renderer/App.tsx" "updateRealtimePreviewProjectSessionSnapshot({"
require_fixed "apps/desktop-electron/src/renderer/App.tsx" "interactionId: update.interactionId"
require_fixed "apps/desktop-electron/src/renderer/App.tsx" "generation: result.data.generation"
require_fixed "apps/desktop-electron/src/renderer/workspace/projectInteraction.ts" "ProjectInteractionController"
require_fixed "apps/desktop-electron/src/renderer/workspace/projectInteraction.ts" "ProjectInteractionEvidence"

require_fixed "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx" 'projectInteractions.begin("selectedSegmentVisual")'
require_fixed "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx" "requestAnimationFrame"
require_fixed "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx" "data-interaction-source"
require_fixed "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx" "data-interaction-generation"
require_fixed "apps/desktop-electron/src/renderer/workspace/Timeline.tsx" 'projectInteractions.begin("timelineMoveTrim")'
require_fixed "apps/desktop-electron/src/renderer/workspace/Timeline.tsx" 'projectInteractions.begin("playheadScrub")'
require_fixed "apps/desktop-electron/src/renderer/workspace/Timeline.tsx" 'projectInteractions.begin("keyframeEdit")'
require_fixed "apps/desktop-electron/src/renderer/workspace/Timeline.tsx" "requestAnimationFrame"
require_fixed "apps/desktop-electron/src/renderer/workspace/Timeline.tsx" "data-interaction-source"
require_fixed "apps/desktop-electron/src/renderer/workspace/Inspector.tsx" 'projectInteractions.begin("selectedText")'
require_fixed "apps/desktop-electron/src/renderer/workspace/Inspector.tsx" 'projectInteractions.begin("selectedSegmentAudio")'
require_fixed "apps/desktop-electron/src/renderer/workspace/Inspector.tsx" "shouldUseKeyframeValueInteraction"
require_fixed "apps/desktop-electron/src/renderer/workspace/Inspector.tsx" "onPreviewChange"
require_fixed "apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx" 'projectInteractions.begin("selectedSegmentAudio")'
require_fixed "apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx" 'aria-disabled="true"'
require_fixed "apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx" "暂不可用"

require_fixed "apps/desktop-electron/tests/interaction-preview-inspector.spec.ts" "beginProjectInteraction"
require_fixed "apps/desktop-electron/tests/interaction-preview-inspector.spec.ts" "updateProjectInteraction"
require_fixed "apps/desktop-electron/tests/interaction-preview-inspector.spec.ts" "commitProjectInteraction"
require_fixed "apps/desktop-electron/tests/interaction-preview-inspector.spec.ts" "revisionUnchanged"
require_fixed "apps/desktop-electron/tests/interaction-timeline-keyframe.spec.ts" "coalescedThrough"
require_fixed "apps/desktop-electron/tests/interaction-timeline-keyframe.spec.ts" "acceptedSequence"
require_fixed "apps/desktop-electron/tests/interaction-timeline-keyframe.spec.ts" "keyed property value drag must use keyframeEdit interactions"
require_fixed "apps/desktop-electron/tests/template-import.spec.ts" "waitForCompositedPreviewEvidence"
require_fixed "apps/desktop-electron/tests/template-import.spec.ts" "renderGraphGpuComposited"
require_fixed "apps/desktop-electron/tests/template-import.spec.ts" "fallbackActive"
require_fixed "apps/desktop-electron/tests/ui-regression.spec.ts" "ui-reference-regression.spec"

fail_matches \
  "high-frequency workspace paths must not route drag/slider/scrub samples through canonical executeProjectIntent loops" \
  "$HIGH_FREQUENCY_CANONICAL_INTENT_PATTERN" \
  "${WORKSPACE_HIGH_FREQUENCY_FILES[@]}"

fail_matches \
  "preview invalidation must be driven by CommandDelta, not intent-kind switches" \
  "$INTENT_KIND_INVALIDATION_PATTERN" \
  "apps/desktop-electron/src/renderer/App.tsx" \
  "apps/desktop-electron/src/renderer/commandHelpers.ts"

fail_matches \
  "product E2E must not accept DOM-only ghost overlay movement as preview success" \
  "$DOM_ONLY_PREVIEW_EVIDENCE_PATTERN" \
  "apps/desktop-electron/tests/interaction-preview-inspector.spec.ts" \
  "apps/desktop-electron/tests/interaction-timeline-keyframe.spec.ts" \
  "apps/desktop-electron/tests/template-import.spec.ts" \
  "apps/desktop-electron/tests/ui-regression.spec.ts"

fail_matches \
  "unsupported high-frequency transition/fade/crop/text-box handles must remain hidden or product-gated until wired to sessions" \
  "$UNSUPPORTED_VISIBLE_HIGH_FREQUENCY_CONTROL_PATTERN" \
  "apps/desktop-electron/src/renderer"

echo "phase17.1 source guards passed"

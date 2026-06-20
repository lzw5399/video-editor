#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase3-source-guards: rg is required" >&2
  exit 1
fi

failed=0

fail_if_matches() {
  local description="$1"
  local pattern="$2"
  shift 2

  local output
  output="$(rg -n --pcre2 "$pattern" "$@" 2>/dev/null || true)"
  if [ -n "$output" ]; then
    echo "phase3-source-guards: ${description}" >&2
    echo "$output" >&2
    failed=1
  fi
}

fail_if_diff() {
  if [ "${VE_PHASE3_SOURCE_GUARDS_CHECK_GIT_DIFF:-0}" != "1" ]; then
    return
  fi
  if ! git diff --exit-code "$@" >/dev/null; then
    echo "phase3-source-guards: generated schemas/contracts are dirty" >&2
    git diff -- "$@" >&2
    failed=1
  fi
}

fail_if_matches \
  "draft_commands must not depend on runtime, storage, render, Electron, Node, filesystem, or process layers" \
  '\b(?:media_runtime|media_runtime_desktop|project_store|preview_service|render_graph|ffmpeg_compiler|bindings_node|ffmpeg|ffprobe|electron|napi|node|std::fs|fs::|std::process)\b' \
  crates/draft_commands/src crates/draft_commands/Cargo.toml

fail_if_matches \
  "renderer/preload must not drive primary editing through low-level segment command builders" \
  '\bbuild(?:AddSegment|AddAudioSegment|AddTextSegment|MoveSegment|SplitSegment|TrimSegment)Command\b' \
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/preload

fail_if_matches \
  "renderer/preload must not allocate segment/track IDs for primary timeline edits" \
  '\bconst[[:space:]]+(?:segmentId|rightSegmentId|trackId)[[:space:]]*=' \
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/preload

fail_if_matches \
  "renderer/preload must not construct segment source timeranges or main-track magnet semantics" \
  '(?:sourceTimerange|mainTrackMagnet)[[:space:]]*:' \
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/preload

fail_if_matches \
  "renderer/preload must not derive trim/move target timeranges or mutate tracks/segments directly" \
  'targetTimerange[[:space:]]*=|command\.payload\.(?:targetTimerange|sourceTimerange)|\.tracks[[:space:]]*=|tracks\.(push|splice|sort)|\.segments[[:space:]]*=|segments\.(push|splice|sort)' \
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/preload

fail_if_matches \
  "draft/schema/generated command contracts must not use floating-point or seconds-based persisted time" \
  'durationSeconds|duration_seconds|source.*Seconds|target.*Seconds|seconds: f32|seconds: f64' \
  crates/draft_model/src crates/draft_commands/src schemas/command.schema.json schemas/draft.schema.json apps/desktop-electron/src/generated

fail_if_matches \
  "draft command terminology must use material/track/segment vocabulary, not Asset/Clip" \
  '\b(Asset|Clip)\b' \
  crates/draft_model/src crates/draft_commands/src schemas/command.schema.json schemas/draft.schema.json apps/desktop-electron/src/generated

fail_if_matches \
  "draft fixtures must not persist command state, undo/redo stacks, history limits, or snapping runtime state" \
  'commandState|undoStack|redoStack|maxHistoryEntries|snapping' \
  fixtures/draft/positive fixtures/draft/negative

fail_if_diff schemas apps/desktop-electron/src/generated

exit "$failed"

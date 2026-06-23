#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase16 source guard violation: $1" >&2
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

assert_pattern_rejects() {
  local description="$1"
  local pattern="$2"
  local source="$3"
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' RETURN
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase16Violation.ts"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase16Violation.ts" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "// $source" >"$tmp_dir/CommentOnly.ts"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.ts" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

BINDINGS_DIR="crates/bindings_node/src"
RENDERER_DIR="apps/desktop-electron/src/renderer"
PRELOAD_FILE="apps/desktop-electron/src/preload/index.ts"
PRODUCT_SCHEDULER_STRESS_SPEC="apps/desktop-electron/tests/product-scheduler-stress.spec.ts"

RENDERER_POLICY_MUTATION_PATTERN='\b(?:applyTaskRuntimeDevConfig|resourceBudgets|queuePolicies|maxInFlight|maxQueued|telemetrySampleLimit|QueueOverflowPolicy|schedulerPolicy|queuePriority|priorityQueue|fallbackPolicy|retryPolicy|JobPriority|ResourceClass)\b'
PRODUCT_MOCK_SUCCESS_PATTERN='VIDEO_EDITOR_TEST_MOCK_(?:EXPORT_COMMANDS|ARTIFACT_COMMANDS|AUDIO_COMMANDS)|VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES[[:space:]]*:[[:space:]]*["'\'']1["'\'']|mockSchedulerSuccess|artifactSchedulerSuccess|cpuProbeSchedulerSuccess|domOnlySchedulerSuccess'

assert_pattern_rejects \
  "renderer scheduler queue-depth mutation" \
  "$RENDERER_POLICY_MUTATION_PATTERN" \
  "const maxQueued = 128;"
assert_pattern_rejects \
  "product scheduler stress mock success" \
  "$PRODUCT_MOCK_SUCCESS_PATTERN" \
  "const env = { VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS: \"1\" };"

require_file "crates/task_runtime/src/lib.rs"
require_file "crates/task_runtime/src/freshness.rs"
require_file "crates/task_runtime/src/job.rs"
require_file "crates/task_runtime/src/config.rs"
require_file "crates/task_runtime/src/telemetry.rs"
require_file "crates/bindings_node/src/task_runtime_service.rs"
require_file "scripts/no-product-fallback-guards.sh"

require_fixed "crates/task_runtime/src/lib.rs" "Rust-owned task scheduling boundary contracts"
require_fixed "crates/task_runtime/src/job.rs" "pub enum JobDomain"
require_fixed "crates/task_runtime/src/job.rs" "ArtifactGeneration"
require_fixed "crates/task_runtime/src/job.rs" "MediaProbe"
require_fixed "crates/task_runtime/src/job.rs" "FilesystemIo"
require_fixed "crates/task_runtime/src/config.rs" "MAX_DEV_QUEUE_DEPTH"
require_fixed "crates/task_runtime/src/telemetry.rs" "resource_saturation_count"
require_fixed "crates/bindings_node/src/task_runtime_service.rs" "TaskRuntimeStatusResponse"
require_fixed "crates/bindings_node/src/task_runtime_service.rs" "TaskRuntimeTelemetryResponse"

for driver in \
  task-runtime-preview-driver \
  task-runtime-audio-driver \
  task-runtime-export-driver \
  task-runtime-export-validation \
  task-runtime-artifact-driver \
  task-runtime-media-probe; do
  if ! rg -q --fixed-strings "$driver" "$BINDINGS_DIR"; then
    fail "missing scheduler-owned binding driver name ${driver}"
  fi
done

playback_generation_definitions="$(rg -n 'pub struct PlaybackGeneration' crates --glob '*.rs' | wc -l | tr -d '[:space:]')"
if [ "$playback_generation_definitions" != "1" ]; then
  rg -n 'pub struct PlaybackGeneration' crates --glob '*.rs' >&2 || true
  fail "PlaybackGeneration must have exactly one canonical definition"
fi
require_fixed "crates/realtime_preview_runtime/src/clock.rs" "pub use task_runtime"
require_fixed "crates/realtime_preview_runtime/src/lib.rs" "pub use task_runtime"

fail_matches \
  "realtime preview binding must not keep legacy worker maps or idle poll cadence" \
  '\b(?:still_frame_workers|playback_workers|rt-preview-still|rt-preview-playback|REALTIME_PLAYBACK_IDLE_POLL_INTERVAL|presentPlaybackTick|schedulerCompositedEvidence)\b' \
  "$BINDINGS_DIR/realtime_preview_service.rs"

fail_matches \
  "export binding must not reintroduce a binding-owned export registry or raw export thread" \
  '\b(?:ExportJobRegistry|run_export_thread|export_thread_registry|thread::spawn[[:space:]]*\([[:space:]]*move[[:space:]]*\|\|[^{]*run_export)\b' \
  "$BINDINGS_DIR/preview_export_service.rs"

fail_matches \
  "audio binding must not reintroduce a binding-owned refill poll loop" \
  '\b(?:audio-preview-refill|AUDIO_PREVIEW_REFILL_POLL_INTERVAL|run_audio_refill_loop)\b' \
  "$BINDINGS_DIR/audio_service.rs"

fail_matches \
  "bindings must not call direct FFmpeg timeout helpers instead of scheduler adapters" \
  '\bDesktopFfmpegExecutor::with_timeout\b' \
  "$BINDINGS_DIR"

fail_matches \
  "bindings must not call raw material probing outside the scheduled probe adapter" \
  '\bprobe_material_metadata\s*\(' \
  "$BINDINGS_DIR"

fail_matches \
  "bindings must not use direct thumbnail generation helpers without scheduler cancellation/freshness" \
  '\bgenerate_thumbnail_artifact\s*\(' \
  "$BINDINGS_DIR"

fail_matches \
  "renderer and preload must not mutate scheduler capacities, queue policy, priority, freshness, retry, fallback, or resource budgets" \
  "$RENDERER_POLICY_MUTATION_PATTERN" \
  "$RENDERER_DIR" "$PRELOAD_FILE" \
  --glob '!commandHelpers.ts'

fail_matches \
  "preload must not expose scheduler dev-config mutation to renderer code" \
  'applyTaskRuntimeDevConfig|diagnostics:applyTaskRuntimeDevConfig' \
  "$PRELOAD_FILE"

require_fixed "$PRELOAD_FILE" "getTaskRuntimeStatus"
require_fixed "$PRELOAD_FILE" "getTaskRuntimeTelemetry"

if [ -f "$PRODUCT_SCHEDULER_STRESS_SPEC" ]; then
  fail_matches \
    "product scheduler stress success must not use test mock runtime/export/artifact/audio responses" \
    "$PRODUCT_MOCK_SUCCESS_PATTERN" \
    "$PRODUCT_SCHEDULER_STRESS_SPEC"
  require_fixed "$PRODUCT_SCHEDULER_STRESS_SPEC" "renderGraphGpuComposited"
  require_fixed "$PRODUCT_SCHEDULER_STRESS_SPEC" "captureVisiblePreviewEvidence"
  require_fixed "$PRODUCT_SCHEDULER_STRESS_SPEC" "requestProjectSessionPreviewFrameCount"
  require_fixed "$PRODUCT_SCHEDULER_STRESS_SPEC" "getTaskRuntimeTelemetry"
  require_fixed "$PRODUCT_SCHEDULER_STRESS_SPEC" "queueLatencyUs"
  require_fixed "$PRODUCT_SCHEDULER_STRESS_SPEC" "resourceSaturationCount"
  require_fixed "$PRODUCT_SCHEDULER_STRESS_SPEC" "fallbackActive"
fi

require_fixed "scripts/no-product-fallback-guards.sh" "scheduler stress success"

echo "phase16 source guards passed"

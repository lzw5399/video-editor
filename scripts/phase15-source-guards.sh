#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase15-source-guards: rg is required" >&2
  exit 1
fi

RENDERER_DIR="apps/desktop-electron/src/renderer"
RENDERER_PRODUCTION_DIRS=(
  "apps/desktop-electron/src/renderer/App.tsx"
  "apps/desktop-electron/src/renderer/viewModel.ts"
  "apps/desktop-electron/src/renderer/workspace"
)
PACKAGE_JSON="package.json"
UI_SPEC=".planning/phases/15-audio-engine-and-dsp-timeline-pipeline/15-UI-SPEC.md"
PHASE15_TARGETS=(
  "crates/audio_engine/tests/audio_session_generation.rs"
  "crates/audio_engine/tests/dsp_timeline.rs"
  "crates/audio_output_desktop/tests/audio_output_capabilities.rs"
  "crates/audio_output_desktop/tests/native_audio.rs"
  "crates/bindings_node/tests/audio_service.rs"
  "crates/render_graph/tests/render_graph_snapshots.rs"
  "crates/ffmpeg_compiler/tests/ffmpeg_job_snapshots.rs"
  "crates/testkit/tests/audio_preview_export_parity.rs"
  "apps/desktop-electron/tests/workspace.spec.ts"
)

fail() {
  echo "phase15-source-guards: $1" >&2
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
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase15Violation.ts"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase15Violation.ts" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "// $source" >"$tmp_dir/CommentOnly.ts"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.ts" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

RENDERER_AUDIO_GRAPH_PATTERN='\b(?:AudioGraph|audioGraph|DspTimeline|DspTimelinePlan|dspTimeline|AudioMixIntent|audioMixIntent|gainCurve|panLaw|fadeEnvelope|effectGraph|sampleIndex|targetSample|sourceSample|sampleRateToTimeline|TimelineClock|PlaybackGeneration)\b'
RENDERER_AUDIO_BUFFER_PATTERN='\b(?:mixBuffer|ringBuffer|audioBufferQueue|sampleBuffer|rawAudioBuffer|pcmBuffer|pcmSamples|interleavedSamples|deinterleavedSamples|channelSamples|outputDeviceHandle|nativeDeviceHandle|nativeStreamHandle|outputStreamHandle|CoreAudio|WASAPI|cpal|rubato)\b'
RENDERER_AUDIO_FFMPEG_PATTERN='\b(?:ffmpegAudioFilters?|audioFilterGraph|afade|adelay|amix|atrim|asetpts|volume=|pan=|filter_complex|filterComplex|ffmpegArgs|ffprobeArgs|FfmpegJob|FfmpegExecutor|FFmpeg|ffprobe)\b'
RENDERER_WAVEFORM_ARTIFACT_PATTERN='\b(?:waveformBlobPath|waveformPath|waveformArtifactRoot|artifactRoot|artifactStoreRoot|artifactStorePath|artifact-store\.sqlite|\.sqlite|\.veproj/derived|SQLite|sqlite3?|rusqlite|CREATE TABLE|SELECT .*waveform|INSERT INTO artifact|UPDATE artifact)\b'
RENDERER_CACHE_TIMELINE_PATTERN='\b(?:cacheKey|previewCacheKey|artifactKey|fingerprint|sourceFingerprint|graphFingerprint|blobFingerprint|RenderGraphNodeId|graphNode|dirtyRange|dirtyRanges|DirtyRange|dirtyDomains?|TimelineState|timelineStateMutation|playbackGeneration\s*(?:=|\+\+|--)|timelineClock\s*(?:=|\.))\b'
PRODUCTION_FORBIDDEN_COPY_PATTERN='\b(?:AudioGraph|DSP|TimelineClock|PlaybackGeneration|sampleIndex|mixBuffer|ringBuffer|WASAPI|CoreAudio|cpal|rubato|FFmpeg|ffprobe|SQLite|\.sqlite|\.veproj/derived|cacheKey|fingerprint|graphNode|dirtyRange|outputDeviceHandle|deviceHandle|session ID|native backend|raw logs|raw buffer)\b'

assert_pattern_rejects \
  "renderer AudioGraph ownership" \
  "$RENDERER_AUDIO_GRAPH_PATTERN" \
  "const graph = new AudioGraph({ sampleIndex: 42, gainCurve });"
assert_pattern_rejects \
  "renderer mixBuffer ownership" \
  "$RENDERER_AUDIO_BUFFER_PATTERN" \
  "const mixBuffer = new Float32Array(4096);"
assert_pattern_rejects \
  "renderer outputDeviceHandle ownership" \
  "$RENDERER_AUDIO_BUFFER_PATTERN" \
  "const outputDeviceHandle = native.openOutputDevice();"
assert_pattern_rejects \
  "renderer FFmpeg audio filter string" \
  "$RENDERER_AUDIO_FFMPEG_PATTERN" \
  "const audioFilterGraph = 'atrim=start=0,volume=1.0,amix=inputs=2';"
assert_pattern_rejects \
  "renderer waveform SQLite/blob path access" \
  "$RENDERER_WAVEFORM_ARTIFACT_PATTERN" \
  "const waveformBlobPath = path.join(projectRoot, '.veproj/derived/artifact-store.sqlite');"
assert_pattern_rejects \
  "production UI forbidden copy" \
  "$PRODUCTION_FORBIDDEN_COPY_PATTERN" \
  "return <span>AudioGraph DSP PlaybackGeneration FFmpeg SQLite</span>;"

for target in "${PHASE15_TARGETS[@]}"; do
  [ -f "$target" ] || fail "missing Phase 15 validation target ${target}"
done

require_fixed "$PACKAGE_JSON" "\"test:phase15-rust\""
require_fixed "$PACKAGE_JSON" "cargo test -p draft_model audio -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p draft_commands audio -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p audio_engine -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p audio_output_desktop audio_output_capabilities -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p bindings_node audio_service -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p render_graph audio -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p ffmpeg_compiler audio -- --nocapture"
require_fixed "$PACKAGE_JSON" "cargo test -p testkit audio_preview_export_parity -- --nocapture"
require_fixed "$PACKAGE_JSON" "\"test:phase15-source-guards\""
require_fixed "$PACKAGE_JSON" "bash scripts/phase15-source-guards.sh"
require_fixed "$PACKAGE_JSON" "\"test:phase15-workspace\""
require_fixed "$PACKAGE_JSON" "音频预览|波形|播放状态|五大区域"
require_fixed "$PACKAGE_JSON" "\"test:phase15\""
require_fixed "$PACKAGE_JSON" "pnpm run test:phase15-rust && pnpm run test:phase15-source-guards && pnpm run test:phase15-workspace && pnpm run test:contracts"
require_fixed "$PACKAGE_JSON" "\"test:contracts\""

require_fixed "crates/audio_engine/src/session.rs" "AudioPreviewRuntime"
require_fixed "crates/audio_engine/src/session.rs" "TimelineClock"
require_fixed "crates/audio_engine/src/session.rs" "PlaybackGeneration"
require_fixed "crates/audio_engine/src/session.rs" "AudioBufferRequest"
require_fixed "crates/audio_engine/src/session.rs" "max_buffer_duration_microseconds"
require_fixed "crates/audio_engine/src/dsp_timeline.rs" "DspTimelinePlan"
require_fixed "crates/audio_engine/src/dsp_timeline.rs" "gain_envelope"
require_fixed "crates/audio_engine/src/dsp_timeline.rs" "pan_envelope"
require_fixed "crates/audio_engine/src/dsp_timeline.rs" "fade_envelope"
require_fixed "crates/audio_engine/src/mix_intent.rs" "AudioMixIntent"
require_fixed "crates/audio_output_desktop/tests/native_audio.rs" "VIDEO_EDITOR_TEST_NATIVE_AUDIO"
require_fixed "crates/audio_output_desktop/src/cpal_output.rs" "CoreAudio"
require_fixed "crates/audio_output_desktop/src/cpal_output.rs" "WASAPI"
require_fixed "crates/bindings_node/src/audio_service.rs" "audio-session-"
require_fixed "apps/desktop-electron/src/generated/CommandEnvelope.ts" "export type AudioPreviewCommandPayload ="
require_fixed "apps/desktop-electron/src/generated/CommandResultEnvelope.ts" "export type AudioPreviewStatusResponse ="
require_fixed "apps/desktop-electron/src/generated/CommandResultEnvelope.ts" "export type WaveformDisplayPeaksResponse ="
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "音频预览 controls send generated command envelopes"
require_fixed "apps/desktop-electron/tests/workspace.spec.ts" "波形 display uses Rust-shaped peak payloads"

for label in \
  "音频就绪" \
  "正在播放" \
  "已暂停" \
  "音频缓冲中" \
  "正在定位声音" \
  "声音已同步到最新播放头" \
  "音频请求已取消" \
  "输出设备就绪" \
  "未找到输出设备" \
  "输出设备降级" \
  "音频暂不可用" \
  "波形就绪" \
  "波形生成中" \
  "暂无波形" \
  "波形生成失败"; do
  require_fixed "apps/desktop-electron/src/renderer/viewModel.ts" "$label"
done

for label in \
  "播放预览" \
  "暂无音频素材" \
  "导入音频素材后，可添加到时间线并预览声音。" \
  "音频预览失败：请检查素材是否可用，或重新连接输出设备后重试。" \
  "音频导出可能与预览不同"; do
  require_fixed "$UI_SPEC" "$label"
done

fail_matches \
  "renderer must not construct audio graphs, DSP plans, gain curves, pan laws, fade envelopes, sample indices, TimelineClock, or PlaybackGeneration" \
  "$RENDERER_AUDIO_GRAPH_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts' \
  --glob '!viewModel.ts'

fail_matches \
  "renderer must not own mix buffers, ring buffers, native output device handles, native backend handles, or raw audio samples" \
  "$RENDERER_AUDIO_BUFFER_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!viewModel.ts'

fail_matches \
  "renderer must not construct FFmpeg audio filters, FFmpeg args, or ffprobe args" \
  "$RENDERER_AUDIO_FFMPEG_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts' \
  --glob '!viewModel.ts'

fail_matches \
  "renderer must not read waveform blob paths, artifact roots, SQLite paths, or waveform SQL" \
  "$RENDERER_WAVEFORM_ARTIFACT_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!viewModel.ts'

fail_matches \
  "renderer must not compute cache keys, fingerprints, graph nodes, dirty ranges, or mutate timeline playback generation state" \
  "$RENDERER_CACHE_TIMELINE_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts' \
  --glob '!viewModel.ts'

fail_matches \
  "production renderer copy must hide audio graph, DSP, backend, FFmpeg, artifact-store, cache, fingerprint, dirty-range, session/device, raw log, and raw buffer internals" \
  "$PRODUCTION_FORBIDDEN_COPY_PATTERN" \
  "${RENDERER_PRODUCTION_DIRS[@]}" \
  --glob '!viewModel.ts'

#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase12-source-guards: rg is required" >&2
  exit 1
fi

RENDERER_DIR="apps/desktop-electron/src/renderer"
CONTRACT_DIRS=(
  "crates/draft_model/src"
  "schemas/command.schema.json"
  "apps/desktop-electron/src/generated/CommandResultEnvelope.ts"
)
PACKAGE_JSON="package.json"

fail() {
  echo "phase12-source-guards: $1" >&2
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
  printf '%s\n' "$source" >"$tmp_dir/InjectedBoundaryViolation.ts"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedBoundaryViolation.ts" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "// $source" >"$tmp_dir/CommentOnly.ts"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.ts" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}

RENDERER_PLATFORM_MEDIA_PATTERN='\b(?:MediaFoundation|MF_SOURCE_READER|DXVA|D3D11VideoDecoder|D3D12VideoDecoder|AVFoundation|AVAssetReader|VideoToolbox|VTDecompressionSession|CoreVideo|CVPixelBuffer|CVMetalTexture|MetalTexture|MTLTexture)\b'
RENDERER_DECODE_SELECTION_PATTERN='\b(?:RuntimeSelectedDecodePath|SelectedDecodePath|RuntimeMediaIoFallbackReason|MediaIoFallbackReason|NativeHardwareTexture|NativeHardwareCpuCopy|NativeSoftwareCpuFrame|FfmpegCpuFrame|FfmpegPreviewArtifact|chooseMediaIoFallback|selectMediaIoFallback|routeMediaIoFallback|mediaIoFallbackLadder)\b'
RENDERER_FFMPEG_PROCESS_PATTERN='\b(?:FfmpegExecutor|ffmpegArgs|ffprobeArgs|filter_complex|filterComplex|child_process|execFile|exec\s*\(|spawn\s*\()\b'
RAW_HANDLE_PATTERN='\b(?:nativePointer|rawHandle|ArrayBuffer|Uint8Array)\b|(^|[^A-Za-z0-9_])(?:bytes|pixels|rgba|bgra)[[:space:]]*[?:=]'

assert_pattern_rejects \
  "renderer-owned platform media API" \
  "$RENDERER_PLATFORM_MEDIA_PATTERN" \
  "const reader = new VideoToolbox.VTDecompressionSession();"
assert_pattern_rejects \
  "renderer-owned media IO fallback selection" \
  "$RENDERER_DECODE_SELECTION_PATTERN" \
  "const path = RuntimeSelectedDecodePath.FfmpegCpuFrame;"
assert_pattern_rejects \
  "renderer-owned FFmpeg process command" \
  "$RENDERER_FFMPEG_PROCESS_PATTERN" \
  "const ffmpegArgs = ['-filter_complex', graph];"
assert_pattern_rejects \
  "raw decoded-frame or native-handle payload contract" \
  "$RAW_HANDLE_PATTERN" \
  "export type BadDecodedFrame = { pixels: Uint8Array };"

require_fixed "$PACKAGE_JSON" "\"test:phase12-source-guards\""
require_fixed "$PACKAGE_JSON" "bash scripts/phase12-source-guards.sh"

fail_matches \
  "renderer must not import or construct native platform media APIs" \
  "$RENDERER_PLATFORM_MEDIA_PATTERN" \
  "$RENDERER_DIR"

fail_matches \
  "renderer must not choose native/FFmpeg media IO fallback decode paths" \
  "$RENDERER_DECODE_SELECTION_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "renderer must not construct FFmpeg/ffprobe process commands for media IO fallback" \
  "$RENDERER_FFMPEG_PROCESS_PATTERN" \
  "$RENDERER_DIR" \
  --glob '!commandHelpers.ts'

fail_matches \
  "binding-facing handle-capable contracts must not expose native pointers or raw frame byte/pixel payloads" \
  "$RAW_HANDLE_PATTERN" \
  "${CONTRACT_DIRS[@]}"

git diff --exit-code schemas apps/desktop-electron/src/generated >/dev/null

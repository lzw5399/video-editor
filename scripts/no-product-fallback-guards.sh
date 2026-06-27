#!/usr/bin/env bash
set -euo pipefail

fail_if_matches() {
  local label="$1"
  local pattern="$2"
  shift 2

  if rg -n "$pattern" "$@"; then
    echo "no-product-fallback violation: ${label}" >&2
    exit 1
  fi
}

require_in_file() {
  local file="$1"
  local required="$2"
  local label="$3"

  if ! rg -q "$required" "$file"; then
    echo "no-product-fallback violation: ${label} must require ${required}" >&2
    exit 1
  fi
}

fail_if_matches \
  "Electron realtime preview host must not request decoded/FFmpeg content evidence or expose mock/fallback playback displays" \
  'requestRealtimePreviewContentEvidence|shouldCollectContentEvidence|requestContentEvidence|mockFrameDisplay|VIDEO_EDITOR_TEST_EXPOSE_MOCK_FRAME_DISPLAY|VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_FFMPEG_FALLBACK|requestFallbackFrame|ffmpegArtifactGenerated' \
  apps/desktop-electron/src/main/realtimePreviewHost.ts

fail_if_matches \
  "Product tests and app launch must not keep the removed structural command mock switch alive" \
  'VIDEO_EDITOR_TEST_COMMAND_MOCKS|video-editor-test-command-mocks' \
  apps/desktop-electron/src apps/desktop-electron/tests

fail_if_matches \
  "Electron native binding must not expose decoded/FFmpeg content evidence as realtime preview evidence" \
  'requestRealtimePreviewContentEvidence|RealtimePreviewContentEvidenceRequest|RealtimePreviewContentEvidenceResponse' \
  apps/desktop-electron/src/main/nativeBinding.ts

fail_if_matches \
  "Rust realtime preview binding must not compute FFmpeg CPU fingerprints for product playback evidence" \
  'decode_ffmpeg_cpu_frame_fingerprint|FfmpegCpuFrameFingerprintRequest|request_content_evidence|RealtimePreviewContentEvidenceSource::Decoded|RealtimePreviewContentEvidenceBindingRequest|RealtimePreviewContentEvidenceBindingResponse' \
  crates/bindings_node/src/realtime_preview_service.rs crates/bindings_node/src/lib.rs

fail_if_matches \
  "Product user journey types must not accept decoded CPU evidence as playback proof" \
  'source:\s*"decoded"\s*\|\s*"composited"|source:\s*"decoded"' \
  apps/desktop-electron/tests/helpers/userJourney.ts apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx

fail_if_matches \
  "Product host state must not expose runtime/mock/artifact/native bridge values as the product backend" \
  'backend:\s*RealtimePreviewBackendUsed|this\.lastFrame\?\.backend\s*\?\?|backend:\s*"mock"|backend:\s*"gpu"|backend:\s*"offscreen"|backend:\s*"previewArtifact"|backend:\s*"ffmpegArtifact"|backend:\s*"nativeVideo' \
  apps/desktop-electron/src/main/realtimePreviewHost.ts \
  apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx \
  apps/desktop-electron/tests/helpers/userJourney.ts

fail_if_matches \
  "Native video bridge must not use a generic available product presentation constructor" \
  'NativePreviewPresentationState::available\(|pub fn available\(' \
  crates/bindings_node/src/native_preview_presenter.rs

if ! rg -q 'model\.backend !== "renderGraphGpu"' apps/desktop-electron/src/renderer/viewModel.ts; then
  echo "no-product-fallback violation: product realtime preview summary must reject every backend except renderGraphGpu" >&2
  exit 1
fi

if ! rg -q 'backend: "renderGraphGpu" \| "none"' apps/desktop-electron/tests/helpers/userJourney.ts; then
  echo "no-product-fallback violation: product user journey host backend must be renderGraphGpu or none only" >&2
  exit 1
fi

if ! rg -q '"test:phase15-2"' package.json; then
  echo "no-product-fallback violation: Phase 15.2 aggregate gate must stay wired in package.json" >&2
  exit 1
fi

if ! rg -q 'waitForProductPlaybackSuccess' apps/desktop-electron/tests/product-user-journey.spec.ts; then
  echo "no-product-fallback violation: product playback success tests must use the shared product playback helper" >&2
  exit 1
fi

if ! rg -q 'waitForVisiblePreviewCenterChange' apps/desktop-electron/tests/helpers/userJourney.ts; then
  echo "no-product-fallback violation: product playback helper must assert visible preview-region motion" >&2
  exit 1
fi

if ! rg -q 'visibleMotion\.visibleCenterHash' apps/desktop-electron/tests/helpers/userJourney.ts || \
  ! rg -q 'visibleBefore\.visibleCenterHash' apps/desktop-electron/tests/helpers/userJourney.ts; then
  echo "no-product-fallback violation: product playback helper must reject playhead-only motion" >&2
  exit 1
fi

if ! rg -q 'renderGraphGpuComposited' apps/desktop-electron/tests/product-user-journey.spec.ts apps/desktop-electron/tests/helpers/userJourney.ts; then
  echo "no-product-fallback violation: product playback must require renderGraphGpuComposited evidence" >&2
  exit 1
fi

SCHEDULER_STRESS_SPEC="apps/desktop-electron/tests/product-scheduler-stress.spec.ts"
if [ -f "$SCHEDULER_STRESS_SPEC" ]; then
  fail_if_matches \
    "Product scheduler stress success must not be satisfied by test runtime/export/artifact/audio mocks" \
    'VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS|VIDEO_EDITOR_TEST_MOCK_ARTIFACT_COMMANDS|VIDEO_EDITOR_TEST_MOCK_AUDIO_COMMANDS|VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES:\s*"1"|mockSchedulerSuccess|artifactSchedulerSuccess|cpuProbeSchedulerSuccess|domOnlySchedulerSuccess' \
    "$SCHEDULER_STRESS_SPEC"

  for required in \
    'renderGraphGpuComposited' \
    'captureVisiblePreviewEvidence' \
    'requestProjectSessionPreviewFrameCount' \
    'getTaskRuntimeTelemetry' \
    'queueLatencyUs' \
    'resourceSaturationCount' \
    'fallbackActive' \
    'visibleCenterHash'; do
    if ! rg -q "$required" "$SCHEDULER_STRESS_SPEC"; then
      echo "no-product-fallback violation: scheduler stress success must require ${required}" >&2
      exit 1
    fi
  done
fi

PHASE20_LONG_UAT_SPEC="apps/desktop-electron/tests/product-long-timeline-uat.spec.ts"
PHASE20_LONG_EVIDENCE_HELPER="apps/desktop-electron/tests/helpers/longTimelineEvidence.ts"
if [ -f "$PHASE20_LONG_UAT_SPEC" ]; then
  [ -f "$PHASE20_LONG_EVIDENCE_HELPER" ] || {
    echo "no-product-fallback violation: Phase 20 long UAT requires ${PHASE20_LONG_EVIDENCE_HELPER}" >&2
    exit 1
  }

  fail_if_matches \
    "Phase 20 long UAT must not be satisfied by mock runtime/export/artifact/audio switches or fake success helpers" \
    'VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS:\s*"1"|VIDEO_EDITOR_TEST_MOCK_ARTIFACT_COMMANDS:\s*"1"|VIDEO_EDITOR_TEST_MOCK_AUDIO_COMMANDS:\s*"1"|VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES:\s*"1"|mock(?:Runtime|Export|Artifact|Audio|Scheduler)Success|artifact(?:Export|Scheduler|Preview)Success|audioMockSuccess|runtimeMockSuccess|fileExistsOnlyExportSuccess|sourceOnlyExportSuccess' \
    "$PHASE20_LONG_UAT_SPEC" \
    "$PHASE20_LONG_EVIDENCE_HELPER"

  for required in \
    'launchPackagedApp' \
    'expectPhase20PreviewProductionEvidence' \
    'renderGraphGpuComposited' \
    'captureVisiblePreviewEvidence' \
    'requestProjectSessionPreviewFrameCount' \
    'waitForCompositedPreviewEvidence' \
    'waitForProductPlaybackSuccess' \
    'fallbackCount'; do
    require_in_file "$PHASE20_LONG_UAT_SPEC" "$required" "Phase 20 production preview evidence"
  done

  for required in \
    'readTaskRuntimeTelemetry' \
    'getTaskRuntimeTelemetry' \
    'queueLatencyUs' \
    'rejectedCount' \
    'fallbackCount' \
    'staleRejectedCount' \
    'commitProjectInteraction' \
    'cancelProjectInteraction'; do
    require_in_file "$PHASE20_LONG_UAT_SPEC" "$required" "Phase 20 scheduler pressure evidence"
  done

  for required in \
    'expectPhase20ExportMedia' \
    'exportAndValidatePhase20Media' \
    'firstExport' \
    'secondExport' \
    'sampleTimesSeconds' \
    'editPointSeconds' \
    'reopenCycles: 2' \
    'exportValidations: 2' \
    'firstReopen' \
    'secondReopen'; do
    require_in_file "$PHASE20_LONG_UAT_SPEC" "$required" "Phase 20 two-cycle export evidence"
  done

  for required in \
    'writePhase20EvidenceSummary' \
    'collectPhase20FailureEvidence' \
    'productSummary' \
    'developerDetails' \
    'evidenceDir'; do
    require_in_file "$PHASE20_LONG_UAT_SPEC" "$required" "Phase 20 evidence bundle"
  done

  for required in \
    'expectPhase20PreviewProductionEvidence' \
    'fallbackActive' \
    'renderGraphGpuComposited' \
    'requestProjectSessionPreviewFrameCount' \
    'expectPhase20ExportMedia' \
    'probeMediaRuntime' \
    'bundled' \
    'readFfprobeJson' \
    'sampleExportFrames' \
    'sampledFrames' \
    'sampledFramesJsonPath' \
    'minDistinctSampleHashes' \
    'writePhase20EvidenceSummary' \
    'collectPhase20FailureEvidence'; do
    require_in_file "$PHASE20_LONG_EVIDENCE_HELPER" "$required" "Phase 20 evidence helper"
  done
fi

echo "no-product-fallback guards passed"

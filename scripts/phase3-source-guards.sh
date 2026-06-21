#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase3-source-guards: rg is required" >&2
  exit 1
fi

failed=0

LOW_LEVEL_TIMELINE_EDIT_PATTERN='\b(?:AddSegmentCommandPayload|MoveSegmentCommandPayload|SplitSegmentCommandPayload|TrimSegmentCommandPayload|DeleteSegmentCommandPayload|AddTextSegmentCommandPayload|EditTextSegmentCommandPayload|ImportSubtitleSrtCommandPayload|AddAudioSegmentCommandPayload|AddTrackCommandPayload|build(?:AddSegment|AddAudioSegment|AddTextSegment|MoveSegment|SplitSegment|TrimSegment|ImportSubtitleSrt)Command|AddSegmentOptions|TextCommandOptions|ImportSubtitleSrtOptions|AudioCommandOptions|segmentIdPrefix|materialIdPrefix)\b|"\s*(?:addSegment|moveSegment|splitSegment|trimSegment|deleteSegment|addTextSegment|editTextSegment|importSubtitleSrt|addAudioSegment|addTrack)"'
LOW_LEVEL_EDIT_OBJECT_PATTERN='kind:[[:space:]]*"(?:addSegment|moveSegment|splitSegment|trimSegment|deleteSegment|addTextSegment|editTextSegment|importSubtitleSrt|addAudioSegment|addTrack)"(?s:.{0,800})\b(?:segmentId|rightSegmentId|trackId|targetTrackId|sourceTimerange|targetTimerange|mainTrackMagnet)[[:space:]]*:'
MUTATING_TRACK_ID_INTENT_PATTERN='kind:[[:space:]]*"(?:renameTrack|setTrackLock|setTrackVisibility|setTrackMute)"(?s:.{0,400})\btrackId[[:space:]]*(?::|,)'
LEGACY_SELECTION_INTENT_PATTERN='\bselectTimelineSegments\b|kind:[[:space:]]*"selectTimelineSegments"'
SELECTION_INTENT_LEGACY_FIELD_PATTERN='kind:[[:space:]]*"selectTimelineItemIntent"(?s:.{0,500})\b(?:segmentIds|trackIds)[[:space:]]*:'
RENDERER_TIMELINE_VIEW_PROJECTION_PATTERN='\b(?:deriveTimelineRows|getSelectedSegmentView|getSelectedTrackView|timelineTrackSelectionHandle|timelineSegmentSelectionHandle)\b'
RENDERER_TIMELINE_HANDLE_ENCODING_PATTERN='\bencodeURIComponent[[:space:]]*\([[:space:]]*(?:trackId|segmentId|selectedTrackId|selectedSegmentId)[[:space:]]*\)'
RENDERER_PROJECT_SUMMARY_DRAFT_PATTERN='\bworkspace\.draft\.(?:metadata|canvasConfig|tracks|materials)\b|\b(?:getSequenceDuration|getSequenceDurationUs)\s*\(|\bdraft\.tracks\.(?:reduce|flatMap|map|forEach)\s*\('
RENDERER_PRODUCT_EDIT_STATE_PATTERN='\bworkspace\.(?:commandState|selection)\b|\bcommandState\.(?:undoStack|redoStack|snapping)\b|\bselection\.(?:segmentIds|trackIds)\b'
ADD_INTENT_LEGACY_PLACEMENT_PATTERN='kind:[[:space:]]*"addTimelineSegmentIntent"(?s:.{0,500})\b(?:targetStart|targetTimerange|sourceTimerange|trackId|segmentId)[[:space:]]*:'
MEDIA_ADD_INTENT_LEGACY_TIMING_PATTERN='kind:[[:space:]]*"(?:addTextSegmentIntent|addAudioSegmentIntent|importSubtitleSrtIntent)"(?s:.{0,700})\b(?:duration|timeOffset|targetStart|targetTimerange|sourceTimerange|trackId|segmentId|segmentIdPrefix|materialIdPrefix)[[:space:]]*:'
MEDIA_ADD_CALLBACK_LEGACY_TIMING_PATTERN='on(?:AddTextSegment|AddAudioSegment|ImportSubtitleSrt)[^;\n]*\b(?:durationUs|timeOffsetUs)\b|function[[:space:]]+handle(?:AddTextSegment|AddAudioSegment|ImportSubtitleSrt)[[:space:]]*\([^)]*\b(?:durationUs|timeOffsetUs)\b'
TEXT_ADD_INTENT_LEGACY_PRESET_PATTERN='kind:[[:space:]]*"addTextSegmentIntent"(?s:.{0,220})\btext[[:space:]]*:|kind:[[:space:]]*"importSubtitleSrtIntent"(?s:.{0,420})\b(?:style|textBox|layoutRegion|wrapping|fontRef)[[:space:]]*:'
TEXT_ADD_NATIVE_INTENT_LEGACY_PRESET_PATTERN='kind:[[:space:]]*"addTextSegmentIntent";[^\n|]*\btext[[:space:]]*:|kind:[[:space:]]*"importSubtitleSrtIntent";(?s:.{0,260})\b(?:style|textBox|layoutRegion|wrapping)[[:space:]]*:'
TEXT_ADD_CALLBACK_LEGACY_PRESET_PATTERN='\bcreateDefaultTextSegment\b|on(?:AddTextSegment|ImportSubtitleSrt)[^;\n]*\b(?:TextSegment|textTemplate)\b|function[[:space:]]+handle(?:AddTextSegment|ImportSubtitleSrt)[[:space:]]*\([^)]*\b(?:TextSegment|textTemplate)\b'
EXPORT_CONTROL_LEGACY_COMMAND_PATTERN='\b(?:buildGetExportJobStatusCommand|buildCancelExportCommand|executeExportCommand)\b'
AUDIO_PREVIEW_LEGACY_COMMAND_BUILDER_PATTERN='\bbuild(?:CreateAudioPreviewSession|PlayAudioPreview|PauseAudioPreview|StopAudioPreview|SeekAudioPreview|CancelAudioPreview|GetAudioPreviewStatus|ListAudioOutputDevices|SelectAudioOutputDevice|GetWaveformDisplayPeaks|RefreshWaveformStatus)Command\b'
AUDIO_PREVIEW_GENERIC_EXECUTE_PATTERN='window\.videoEditorCore\.executeCommand<[^>]*(?:Audio|Waveform)|executeAudioCommand<[^>]*>\([[:space:]]*\([^)]*\)[[:space:]]*=>[[:space:]]*build(?:CreateAudioPreviewSession|PlayAudioPreview|PauseAudioPreview|StopAudioPreview|SeekAudioPreview|CancelAudioPreview|GetAudioPreviewStatus|ListAudioOutputDevices|SelectAudioOutputDevice|GetWaveformDisplayPeaks|RefreshWaveformStatus)Command'
AUDIO_PREVIEW_EXECUTE_ALLOWLIST_PATTERN='\|[[:space:]]*"(?:createAudioPreviewSession|playAudioPreview|pauseAudioPreview|stopAudioPreview|seekAudioPreview|cancelAudioPreview|getAudioPreviewStatus|listAudioOutputDevices|selectAudioOutputDevice|getWaveformDisplayPeaks|refreshWaveformStatus)"'
MOVE_INTENT_LEGACY_DELTA_PATTERN='kind:[[:space:]]*"moveSelectedSegmentIntent"(?s:.{0,300})\bdelta[[:space:]]*:'
MOVE_CALLBACK_DELTA_PATTERN='onMoveSelectedSegment\?\.\([[:space:]]*deltaUs[[:space:]]*\)'
TRIM_INTENT_LEGACY_DELTA_PATTERN='kind:[[:space:]]*"trimSelectedSegmentIntent"(?s:.{0,400})\bdelta[[:space:]]*:'
TRIM_CALLBACK_DELTA_PATTERN='onTrimSelectedSegment\?\.\([[:space:]]*"(?:left|right)"[[:space:]]*,[[:space:]]*(?:deltaUs|Math\.abs\([[:space:]]*deltaUs[[:space:]]*\))'
SPLIT_INTENT_LEGACY_SPLIT_AT_PATTERN='kind:[[:space:]]*"splitSelectedSegmentIntent"(?s:.{0,300})\bsplitAt[[:space:]]*:'
SPLIT_CALLBACK_PLAYHEAD_PATTERN='onSplitSelectedSegment\?\.\([[:space:]]*playheadUs[[:space:]]*\)'
KEYFRAME_INTENT_LEGACY_AT_PATTERN='kind:[[:space:]]*"(?:setSelectedSegmentKeyframe|removeSelectedSegmentKeyframe)"(?s:.{0,500})\bat[[:space:]]*:'
KEYFRAME_REMOVE_CALLBACK_AT_PATTERN='onRemoveKeyframe[[:space:]]*\((?s:.{0,160})\bkeyframe\.at\b'
PROJECT_SESSION_RAW_VIEW_MODEL_PATTERN='(?:struct|type)[[:space:]]+(?:SelectedSegmentViewModel|TimelineTrackRowViewModel|TimelineSegmentViewModel)(?s:.{0,900})\b(?:track:[[:space:]]*Track|segment:[[:space:]]*Segment)\b'
RENDERER_PRODUCT_RAW_TIMELINE_VM_PATTERN='\b(?:row\.track|segment\.segment|selected\.segment|selectedSegment\.segment)\b'
PROJECT_SESSION_RESPONSE_STATE_PATTERN='(?:struct|type)[[:space:]]+ProjectSession(?:Open|TimelineIntent|Intent)Response(?s:.{0,500})\b(?:draft|commandState|command_state|selection)\b'
RENDERER_WORKSPACE_STATE_PROJECT_STATE_PATTERN='export[[:space:]]+type[[:space:]]+WorkspaceState[[:space:]]*=[[:space:]]*\{(?s:.{0,800})\b(?:draft|commandState|selection)[[:space:]]*:'
RENDERER_SESSION_RESPONSE_READ_PATTERN='\b(?:result|openedProject)\.data\.(?:draft|commandState|selection)\b|\b(?:result|openedProject)\?\.data\?\.(?:draft|commandState|selection)\b'
RENDERER_PREVIEW_DRAFT_COMMAND_HELPER_PATTERN='\b(?:buildRequestPreviewFrameCommand|buildRequestPreviewSegmentCommand|RequestPreviewFrameCommandPayload|RequestPreviewSegmentCommandPayload)\b'

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

fail_if_matches_multiline() {
  local description="$1"
  local pattern="$2"
  shift 2

  local output
  output="$(rg -n -U --pcre2 "$pattern" "$@" 2>/dev/null || true)"
  if [ -n "$output" ]; then
    echo "phase3-source-guards: ${description}" >&2
    echo "$output" >&2
    failed=1
  fi
}

require_matches() {
  local description="$1"
  local pattern="$2"
  shift 2

  local output
  output="$(rg -n --pcre2 "$pattern" "$@" 2>/dev/null || true)"
  if [ -z "$output" ]; then
    echo "phase3-source-guards: missing required pattern: ${description}" >&2
    failed=1
  fi
}

require_matches_multiline() {
  local description="$1"
  local pattern="$2"
  shift 2

  local output
  output="$(rg -n -U --pcre2 "$pattern" "$@" 2>/dev/null || true)"
  if [ -z "$output" ]; then
    echo "phase3-source-guards: missing required pattern: ${description}" >&2
    failed=1
  fi
}

assert_pattern_rejects() {
  local description="$1"
  local pattern="$2"
  local sample="$3"
  local sample_file
  sample_file="$(mktemp "${TMPDIR:-/tmp}/phase3-source-guard.XXXXXX")"
  printf "%s\n" "$sample" > "$sample_file"
  if ! rg -n -U --pcre2 "$pattern" "$sample_file" >/dev/null 2>&1; then
    echo "phase3-source-guards: guard self-test failed: ${description}" >&2
    echo "$sample" >&2
    failed=1
  fi
  rm -f "$sample_file"
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
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/commandHelpers.ts apps/desktop-electron/src/preload

fail_if_matches \
  "renderer/main/preload must not construct low-level timeline edit commands or payload types" \
  "$LOW_LEVEL_TIMELINE_EDIT_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/renderer/commandHelpers.ts \
  apps/desktop-electron/src/preload \
  apps/desktop-electron/src/main/nativeBinding.ts \
  apps/desktop-electron/src/main/index.ts

fail_if_matches_multiline \
  "renderer/main/preload must not construct semantic fields inside low-level timeline edit payloads" \
  "$LOW_LEVEL_EDIT_OBJECT_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/renderer/commandHelpers.ts \
  apps/desktop-electron/src/preload \
  apps/desktop-electron/src/main/nativeBinding.ts \
  apps/desktop-electron/src/main/index.ts

fail_if_matches \
  "renderer display/view modules must not bypass App-owned command dispatch" \
  '\bwindow\.videoEditorCore\b|\bipcRenderer\b|\bexecuteProjectIntent[[:space:]]*\(' \
  apps/desktop-electron/src/renderer \
  --glob '!App.tsx'

fail_if_matches \
  "renderer must not read materials through legacy draft-bearing executeCommand payloads; use project-session read APIs" \
  'buildListMaterialsCommand|buildListMissingMaterialsCommand|command:[[:space:]]*"listMaterials"|command:[[:space:]]*"listMissingMaterials"|kind:[[:space:]]*"listMaterials"|kind:[[:space:]]*"listMissingMaterials"' \
  apps/desktop-electron/src/renderer

fail_if_matches_multiline \
  "project session importMaterial response must not expose full Draft payloads" \
  '(?:struct|type)[[:space:]]+ProjectSessionImportMaterialResponse(?s:.{0,360})\bdraft\b' \
  crates/bindings_node/src/project_session_service.rs \
  apps/desktop-electron/src/main/nativeBinding.ts

fail_if_matches_multiline \
  "project session open/edit responses must not expose draft, commandState, or selection" \
  "$PROJECT_SESSION_RESPONSE_STATE_PATTERN" \
  crates/bindings_node/src/project_session_service.rs \
  apps/desktop-electron/src/main/nativeBinding.ts

fail_if_matches_multiline \
  "renderer WorkspaceState must not store canonical draft, commandState, or selection" \
  "$RENDERER_WORKSPACE_STATE_PROJECT_STATE_PATTERN" \
  apps/desktop-electron/src/renderer/viewModel.ts

fail_if_matches \
  "renderer must not read project session draft, commandState, or selection response fields" \
  "$RENDERER_SESSION_RESPONSE_READ_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx

fail_if_matches_multiline \
  "renderer import material session result must use returned material data, not a returned full Draft" \
  'executeProjectSessionIntent<ProjectSessionImportMaterialResponse>(?s:.{0,1400})result\.data\.draft' \
  apps/desktop-electron/src/renderer/App.tsx

require_matches_multiline \
  "project session importMaterial response returns Rust session view model" \
  'ProjectSessionImportMaterialResponse(?s:.{0,360})\bviewModel\b' \
  apps/desktop-electron/src/main/nativeBinding.ts

require_matches_multiline \
  "project session importMaterial response returns Rust command delta" \
  'ProjectSessionImportMaterialResponse(?s:.{0,420})\bdelta\b' \
  apps/desktop-electron/src/main/nativeBinding.ts

require_matches_multiline \
  "project session importMaterial response returns Rust-owned material list" \
  'ProjectSessionImportMaterialResponse(?s:.{0,420})\bmaterials\b' \
  apps/desktop-electron/src/main/nativeBinding.ts

fail_if_matches_multiline \
  "renderer importMaterial must refresh canonical session materials instead of locally reconciling material arrays" \
  'executeProjectSessionIntent<ProjectSessionImportMaterialResponse>(?s:.{0,1800})current\.materials\.filter|executeProjectSessionIntent<ProjectSessionImportMaterialResponse>(?s:.{0,1800})\.\.\.current\.materials' \
  apps/desktop-electron/src/renderer/App.tsx

fail_if_matches_multiline \
  "renderer importMaterial must apply atomic response materials instead of issuing a second session material read" \
  'async function importMaterialPath(?s:(?!\n  async function handleCreateProject).)*listProjectSessionMaterials' \
  apps/desktop-electron/src/renderer/App.tsx

require_matches_multiline \
  "renderer importMaterial consumes Rust session view model" \
  'executeProjectSessionIntent<ProjectSessionImportMaterialResponse>(?s:.{0,1800})viewModel:[[:space:]]*result\.data\.viewModel' \
  apps/desktop-electron/src/renderer/App.tsx

require_matches \
  "renderer importMaterial applies Rust-owned response material list" \
  'materials:[[:space:]]*result\.data\.materials' \
  apps/desktop-electron/src/renderer/App.tsx

fail_if_matches \
  "renderer must not start export through renderer-owned draft payloads; use startProjectSessionExport" \
  'buildStartExportCommand|command:[[:space:]]*"startExport"|kind:[[:space:]]*"startExport"' \
  apps/desktop-electron/src/renderer apps/desktop-electron/src/preload

fail_if_matches \
  "renderer preview commands must use project-session preview APIs, not draft-bearing preview helpers" \
  "$RENDERER_PREVIEW_DRAFT_COMMAND_HELPER_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/renderer/commandHelpers.ts \
  apps/desktop-electron/src/preload

require_matches \
  "renderer still preview uses project-session preview frame API" \
  '\brequestProjectSessionPreviewFrame\b' \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/preload/index.ts \
  apps/desktop-electron/src/main/nativeBinding.ts

require_matches \
  "renderer segment preview uses project-session preview segment API" \
  '\brequestProjectSessionPreviewSegment\b' \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/preload/index.ts \
  apps/desktop-electron/src/main/nativeBinding.ts

fail_if_matches \
  "renderer command helpers must not expose legacy low-level timeline command payloads" \
  '\b(?:AddSegmentCommandPayload|MoveSegmentCommandPayload|SplitSegmentCommandPayload|TrimSegmentCommandPayload|DeleteSegmentCommandPayload|AddTextSegmentCommandPayload|EditTextSegmentCommandPayload|ImportSubtitleSrtCommandPayload|AddAudioSegmentCommandPayload|SetSegmentVolumeCommandPayload|UpdateSegmentAudioCommandPayload|AddTrackCommandPayload|RenameTrackCommandPayload|SetTrackLockCommandPayload|SetTrackVisibilityCommandPayload|UpdateSegmentVisualCommandPayload|SetSegmentKeyframeCommandPayload|RemoveSegmentKeyframeCommandPayload|SourceTimerange|SegmentId|TrackId|AddSegmentOptions|TextCommandOptions|ImportSubtitleSrtOptions|AudioCommandOptions|UpdateSegmentAudioOptions|segmentIdPrefix|materialIdPrefix)\b' \
  apps/desktop-electron/src/renderer/commandHelpers.ts

fail_if_matches \
  "bindings_node public executeCommand must not expose Rust timeline edit routes; use executeProjectIntent sessions instead" \
  'draft_commands::timeline::execute_timeline_edit|fn[[:space:]]+timeline_command\b|\btimeline_command[[:space:]]*\(|"\s*(?:addSegment|addTimelineSegmentIntent|selectTimelineSegments|moveSegment|moveSelectedSegmentIntent|splitSegment|splitSelectedSegmentIntent|trimSegment|trimSelectedSegmentIntent|deleteSegment|undoTimelineEdit|redoTimelineEdit|addTextSegment|addTextSegmentIntent|editTextSegment|importSubtitleSrt|importSubtitleSrtIntent|addAudioSegment|addAudioSegmentIntent|setSegmentVolume|updateSegmentAudio|addTrack|addTrackIntent|renameTrack|renameSelectedTrack|setTrackLock|setSelectedTrackLock|setTrackVisibility|setSelectedTrackVisibility|setTrackMute|setSelectedTrackMute|updateDraftCanvasConfig|updateSegmentVisual|setSegmentKeyframe|removeSegmentKeyframe)"' \
  crates/bindings_node/src/lib.rs

fail_if_matches \
  "bindings_node realtime preview must not own playback cadence/drop/backpressure policy; use realtime_preview_runtime scheduler contracts" \
  '\b(?:PLAYBACK_FRAME_DURATION_US|PLAYBACK_WORKER_IDLE_SLEEP|MAX_IN_FLIGHT_SURFACE_PRESENTATIONS|SURFACE_PRESENT_BACKPRESSURE_TIMEOUT|BindingPlaybackAnchor|BindingPlaybackDueTick|BindingPlaybackFrame|playback_due_tick|advance_next_playback_tick|next_tick_time|playback_anchor)\b' \
  crates/bindings_node/src/realtime_preview_service.rs

fail_if_matches \
  "renderer realtime preview monitor must subscribe to host telemetry instead of polling getTelemetry on an interval" \
  'setInterval\(|bridge\.getTelemetry\(' \
  apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx

fail_if_matches \
  "preload realtime preview host must not expose renderer getTelemetry polling APIs" \
  'getTelemetry|realtimePreviewHost:getTelemetry' \
  apps/desktop-electron/src/preload/index.ts

fail_if_matches \
  "renderer/main/preload/native binding must sync realtime preview from project session snapshots, not renderer-owned Draft payloads" \
  'updateDraftSnapshot|updateRealtimePreviewDraftSnapshot|RealtimePreviewDraftSnapshotRequest|realtimePreviewHost:updateDraftSnapshot|bridge\.updateDraftSnapshot' \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx \
  apps/desktop-electron/src/preload/index.ts \
  apps/desktop-electron/src/main/realtimePreviewHost.ts \
  apps/desktop-electron/src/main/nativeBinding.ts \
  crates/bindings_node/src/lib.rs

fail_if_matches \
  "renderer/preload must not send mutating track intents with renderer-owned track IDs; select the track, then use selected-track intents" \
  'kind:[[:space:]]*"(?:renameTrack|setTrackLock|setTrackVisibility|setTrackMute)"' \
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/preload

fail_if_matches_multiline \
  "renderer/preload must not send mutating track intents with renderer-owned track IDs; select the track, then use selected-track intents" \
  "$MUTATING_TRACK_ID_INTENT_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/preload apps/desktop-electron/src/main/nativeBinding.ts

fail_if_matches \
  "project session intent contract must not accept trackId on mutating track intents" \
  '\|\s*\{[[:space:]]*kind:[[:space:]]*"(?:renameTrack|setTrackLock|setTrackVisibility|setTrackMute)";[^}]*trackId' \
  apps/desktop-electron/src/main/nativeBinding.ts

fail_if_matches \
  "renderer/main/preload/native binding must not expose legacy selectTimelineSegments project intent; use selectTimelineItemIntent item handles" \
  "$LEGACY_SELECTION_INTENT_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/renderer/commandHelpers.ts \
  apps/desktop-electron/src/preload \
  apps/desktop-electron/src/main/nativeBinding.ts \
  apps/desktop-electron/src/main/index.ts

fail_if_matches_multiline \
  "renderer/main/preload/native binding must not attach renderer-owned segmentIds/trackIds to selectTimelineItemIntent" \
  "$SELECTION_INTENT_LEGACY_FIELD_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/renderer/commandHelpers.ts \
  apps/desktop-electron/src/preload \
  apps/desktop-electron/src/main/nativeBinding.ts \
  apps/desktop-electron/src/main/index.ts

require_matches \
  "project session selection intent uses Rust-resolved item handles" \
  '\bselectTimelineItemIntent\b' \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/main/nativeBinding.ts

require_matches \
  "project session selection intent carries itemHandle" \
  '\bitemHandle\b' \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/main/nativeBinding.ts

fail_if_matches \
  "renderer must not derive timeline rows, selected timeline views, or timeline selection handles from Draft" \
  "$RENDERER_TIMELINE_VIEW_PROJECTION_PATTERN" \
  apps/desktop-electron/src/renderer

fail_if_matches \
  "renderer must not encode track/segment IDs into timeline selection handles; consume Rust session handles" \
  "$RENDERER_TIMELINE_HANDLE_ENCODING_PATTERN" \
  apps/desktop-electron/src/renderer apps/desktop-electron/src/preload

require_matches \
  "renderer timeline consumes Rust session timeline view model" \
  '\bworkspace\.viewModel\.timeline\b' \
  apps/desktop-electron/src/renderer/workspace/Timeline.tsx

require_matches \
  "renderer selection display consumes Rust session selected-segment view model" \
  '\bworkspace\.viewModel\.selectedSegment\b' \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx \
  apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx \
  apps/desktop-electron/src/renderer/workspace/Inspector.tsx

fail_if_matches \
  "renderer product views must not derive project summary, canvas, or sequence duration from workspace.draft" \
  "$RENDERER_PROJECT_SUMMARY_DRAFT_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/renderer/workspace

require_matches \
  "renderer product views consume Rust session project summary view model" \
  '\bworkspace\.viewModel\.project\b' \
  apps/desktop-electron/src/renderer/App.tsx \
  apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx \
  apps/desktop-electron/src/renderer/workspace/Inspector.tsx

fail_if_matches \
  "renderer product workspace must not read legacy commandState or selection for edit controls; consume Rust session editControls view model" \
  "$RENDERER_PRODUCT_EDIT_STATE_PATTERN" \
  apps/desktop-electron/src/renderer/workspace

require_matches \
  "renderer timeline consumes Rust session edit controls view model" \
  '\bworkspace\.viewModel\.editControls\b' \
  apps/desktop-electron/src/renderer/workspace/Timeline.tsx

require_matches \
  "renderer inspector consumes Rust session edit controls view model" \
  '\bworkspace\.viewModel\.editControls\b' \
  apps/desktop-electron/src/renderer/workspace/Inspector.tsx

fail_if_matches_multiline \
  "project session view model must not expose raw Track or Segment payloads" \
  "$PROJECT_SESSION_RAW_VIEW_MODEL_PATTERN" \
  crates/bindings_node/src/project_session_service.rs \
  apps/desktop-electron/src/main/nativeBinding.ts

fail_if_matches \
  "renderer product workspace must not read raw Track/Segment objects from project session view models" \
  "$RENDERER_PRODUCT_RAW_TIMELINE_VM_PATTERN" \
  apps/desktop-electron/src/renderer/workspace \
  apps/desktop-electron/src/renderer/App.tsx

require_matches \
  "renderer feature panels consume Rust session timeline capabilities" \
  '\bworkspace\.viewModel\.timeline\.capabilities\b' \
  apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx

fail_if_matches \
  "renderer/preload product path must import SRT through Rust-owned intent, not renderer-owned subtitle IDs" \
  '\bbuildImportSubtitleSrtCommand\b|segmentIdPrefix[[:space:]]*:|materialIdPrefix[[:space:]]*:|trackId[[:space:]]*:[[:space:]]*"track-subtitle"' \
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

fail_if_matches_multiline \
  "renderer/native binding must not pass placement fields for addTimelineSegmentIntent" \
  "$ADD_INTENT_LEGACY_PLACEMENT_PATTERN" \
  apps/desktop-electron/src/main/nativeBinding.ts apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/workspace/Timeline.tsx

fail_if_matches_multiline \
  "renderer/native binding must not pass timing or placement fields for text/audio/subtitle add intents" \
  "$MEDIA_ADD_INTENT_LEGACY_TIMING_PATTERN" \
  apps/desktop-electron/src/main/nativeBinding.ts apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx

fail_if_matches \
  "renderer feature callbacks must not pass legacy text/audio/subtitle timing values" \
  "$MEDIA_ADD_CALLBACK_LEGACY_TIMING_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx

fail_if_matches_multiline \
  "renderer/native binding must not pass text preset fields for add text/subtitle intents" \
  "$TEXT_ADD_INTENT_LEGACY_PRESET_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx

fail_if_matches_multiline \
  "native binding must not type text preset fields for add text/subtitle intents" \
  "$TEXT_ADD_NATIVE_INTENT_LEGACY_PRESET_PATTERN" \
  apps/desktop-electron/src/main/nativeBinding.ts

fail_if_matches \
  "renderer feature callbacks must not construct add-time text presets" \
  "$TEXT_ADD_CALLBACK_LEGACY_PRESET_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx

fail_if_matches \
  "renderer must not construct export status/cancel command envelopes; use explicit export APIs" \
  "$EXPORT_CONTROL_LEGACY_COMMAND_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/commandHelpers.ts

fail_if_matches \
  "renderer must not construct audio preview command envelopes; use explicit audio preview APIs" \
  "$AUDIO_PREVIEW_LEGACY_COMMAND_BUILDER_PATTERN|$AUDIO_PREVIEW_GENERIC_EXECUTE_PATTERN" \
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/commandHelpers.ts

fail_if_matches \
  "bindings_node executeCommand must not allow audio preview command names; use explicit audio preview APIs" \
  "$AUDIO_PREVIEW_EXECUTE_ALLOWLIST_PATTERN" \
  crates/bindings_node/src/lib.rs

fail_if_matches_multiline \
  "renderer/native binding must not pass legacy move delta for selected-segment move" \
  "$MOVE_INTENT_LEGACY_DELTA_PATTERN" \
  apps/desktop-electron/src/main/nativeBinding.ts apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/workspace/Timeline.tsx

fail_if_matches \
  "timeline UI must pass move target start intent, not raw move delta" \
  "$MOVE_CALLBACK_DELTA_PATTERN" \
  apps/desktop-electron/src/renderer/workspace/Timeline.tsx

fail_if_matches_multiline \
  "renderer/native binding must not pass legacy trim delta for selected-segment trim" \
  "$TRIM_INTENT_LEGACY_DELTA_PATTERN" \
  apps/desktop-electron/src/main/nativeBinding.ts apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/workspace/Timeline.tsx

fail_if_matches \
  "timeline UI must pass trim boundary intent, not raw trim delta" \
  "$TRIM_CALLBACK_DELTA_PATTERN" \
  apps/desktop-electron/src/renderer/workspace/Timeline.tsx

fail_if_matches_multiline \
  "renderer/native binding must not pass legacy splitAt timestamps for selected-segment split" \
  "$SPLIT_INTENT_LEGACY_SPLIT_AT_PATTERN" \
  apps/desktop-electron/src/main/nativeBinding.ts apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/workspace/Timeline.tsx

fail_if_matches \
  "timeline UI must not pass renderer playhead as split command payload" \
  "$SPLIT_CALLBACK_PLAYHEAD_PATTERN" \
  apps/desktop-electron/src/renderer/workspace/Timeline.tsx

fail_if_matches \
  "renderer must not derive selected-segment keyframe time/value for product session intents" \
  '\b(?:keyframeValueForSegmentProperty|resolveSegmentRelativePlayhead)\b|kind:[[:space:]]*"setSelectedSegmentKeyframe",[[:space:]]*keyframe\b|onRemoveKeyframe\(keyframe\.property,[[:space:]]*keyframe\.at\)' \
  apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/workspace/Inspector.tsx

fail_if_matches_multiline \
  "renderer/native binding must not pass legacy keyframe at timestamps" \
  "$KEYFRAME_INTENT_LEGACY_AT_PATTERN" \
  apps/desktop-electron/src/main/nativeBinding.ts apps/desktop-electron/src/renderer/App.tsx apps/desktop-electron/src/renderer/workspace/Inspector.tsx

fail_if_matches_multiline \
  "renderer must not delete keyframes by row-derived keyframe time" \
  "$KEYFRAME_REMOVE_CALLBACK_AT_PATTERN" \
  apps/desktop-electron/src/renderer/workspace/Inspector.tsx

fail_if_matches \
  "project session intent contract must not accept renderer-built complete keyframes" \
  'setSelectedSegmentKeyframe";[[:space:]]*keyframe[[:space:]]*:' \
  apps/desktop-electron/src/main/nativeBinding.ts

require_matches \
  "native binding must expose session-owned playhead sync intent" \
  '\bsetSessionPlayhead\b' \
  apps/desktop-electron/src/main/nativeBinding.ts

require_matches \
  "renderer must sync session-owned playhead before keyframe intents" \
  '\bsetSessionPlayhead\b' \
  apps/desktop-electron/src/renderer/App.tsx

require_matches \
  "Rust project session must own playhead sync intent" \
  '\bSetSessionPlayhead\b' \
  crates/bindings_node/src/project_session_service.rs

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

assert_pattern_rejects \
  "low-level addSegment payload with renderer-owned semantic keys" \
  "$LOW_LEVEL_EDIT_OBJECT_PATTERN" \
  'const command = {
    kind: "addSegment",
    segmentId: "renderer-segment",
    trackId: "renderer-track",
    sourceTimerange: { start: 0, duration: 1000 },
    targetTimerange: { start: 0, duration: 1000 }
  };'

assert_pattern_rejects \
  "low-level trimSegment payload with renderer-owned timerange" \
  "$LOW_LEVEL_EDIT_OBJECT_PATTERN" \
  'const command = {
    kind: "trimSegment",
    segmentId: selectedId,
    targetTimerange: { start: nextStart, duration: nextDuration }
  };'

assert_pattern_rejects \
  "mutating track intent with renderer-owned trackId" \
  "$MUTATING_TRACK_ID_INTENT_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "setTrackVisibility",
    trackId,
    visible: false
  }, "hide track");'

assert_pattern_rejects \
  "legacy renderer selection intent" \
  "$LEGACY_SELECTION_INTENT_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "selectTimelineSegments",
    segmentIds: [segmentId],
    trackIds: [trackId]
  }, "select");'

assert_pattern_rejects \
  "new selection intent with legacy renderer-owned selection arrays" \
  "$SELECTION_INTENT_LEGACY_FIELD_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "selectTimelineItemIntent",
    itemHandle,
    segmentIds: [segmentId],
    trackIds: [trackId]
  }, "select");'

assert_pattern_rejects \
  "renderer-owned timeline view projection helper" \
  "$RENDERER_TIMELINE_VIEW_PROJECTION_PATTERN" \
  'export function deriveTimelineRows(draft: Draft, selection: TimelineSelection): TimelineTrackRow[] {
    return draft.tracks.map((track) => ({ track, segments: track.segments }));
  }'

assert_pattern_rejects \
  "renderer-owned timeline selection handle encoding" \
  "$RENDERER_TIMELINE_HANDLE_ENCODING_PATTERN" \
  'const handle = `segment:${encodeURIComponent(trackId)}:${encodeURIComponent(segmentId)}`;'

assert_pattern_rejects \
  "renderer-owned project summary derivation from workspace draft" \
  "$RENDERER_PROJECT_SUMMARY_DRAFT_PATTERN" \
  'const sequenceDuration = workspace.draft.tracks.reduce((duration, track) => duration + track.segments.length, 0);'

assert_pattern_rejects \
  "renderer-owned product edit controls from legacy commandState and selection" \
  "$RENDERER_PRODUCT_EDIT_STATE_PATTERN" \
  'const canUndo = workspace.commandState.undoStack.length > 0;
const hasSelection = workspace.selection.segmentIds.length > 0;
const snappingLabel = commandState.snapping.enabled ? "吸附 开" : "吸附 关";'

assert_pattern_rejects \
  "legacy addTimelineSegment placement field" \
  "$ADD_INTENT_LEGACY_PLACEMENT_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "addTimelineSegmentIntent",
    materialId,
    targetStart: normalizePlayheadTime(playheadUs)
  }, "add");'

assert_pattern_rejects \
  "legacy text add duration field" \
  "$MEDIA_ADD_INTENT_LEGACY_TIMING_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "addTextSegmentIntent",
    text,
    duration: safeDurationUs
  }, "text");'

assert_pattern_rejects \
  "legacy audio add duration field" \
  "$MEDIA_ADD_INTENT_LEGACY_TIMING_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "addAudioSegmentIntent",
    materialId,
    duration: safeDurationUs
  }, "audio");'

assert_pattern_rejects \
  "legacy subtitle import time offset field" \
  "$MEDIA_ADD_INTENT_LEGACY_TIMING_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "importSubtitleSrtIntent",
    srtContent,
    timeOffset: Math.max(0, Math.round(timeOffsetUs))
  }, "subtitle");'

assert_pattern_rejects \
  "legacy text add timing callback" \
  "$MEDIA_ADD_CALLBACK_LEGACY_TIMING_PATTERN" \
  'function handleAddTextSegment(text: TextSegment, durationUs: number): void {
    onAddTextSegment(text, durationUs);
  }'

assert_pattern_rejects \
  "legacy text add full preset field" \
  "$TEXT_ADD_INTENT_LEGACY_PRESET_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "addTextSegmentIntent",
    text: createDefaultTextSegment(content, "text")
  }, "text");'

assert_pattern_rejects \
  "legacy subtitle add style fields" \
  "$TEXT_ADD_INTENT_LEGACY_PRESET_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "importSubtitleSrtIntent",
    srtContent,
    style: textTemplate.style,
    textBox: textTemplate.textBox,
    layoutRegion: textTemplate.layoutRegion,
    wrapping: textTemplate.wrapping
  }, "subtitle");'

assert_pattern_rejects \
  "legacy renderer add-time text preset helper" \
  "$TEXT_ADD_CALLBACK_LEGACY_PRESET_PATTERN" \
  'function createDefaultTextSegment(content: string, source: TextSegment["source"]): TextSegment {
    return {} as TextSegment;
  }'

assert_pattern_rejects \
  "legacy export control command builder" \
  "$EXPORT_CONTROL_LEGACY_COMMAND_PATTERN" \
  'return buildGetExportJobStatusCommand(current.export.jobId);'

assert_pattern_rejects \
  "legacy audio preview command builder" \
  "$AUDIO_PREVIEW_LEGACY_COMMAND_BUILDER_PATTERN|$AUDIO_PREVIEW_GENERIC_EXECUTE_PATTERN" \
  'return buildPlayAudioPreviewCommand({ projectSessionId, expectedRevision, sessionId });'

assert_pattern_rejects \
  "legacy audio preview executeCommand allowlist" \
  "$AUDIO_PREVIEW_EXECUTE_ALLOWLIST_PATTERN" \
  '| "playAudioPreview"'

assert_pattern_rejects \
  "legacy selected-segment move delta" \
  "$MOVE_INTENT_LEGACY_DELTA_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "moveSelectedSegmentIntent",
    delta: Math.max(0, Math.round(deltaUs))
  }, "move");'

assert_pattern_rejects \
  "raw move delta callback" \
  "$MOVE_CALLBACK_DELTA_PATTERN" \
  'onMoveSelectedSegment?.(deltaUs)'

assert_pattern_rejects \
  "legacy selected-segment trim delta" \
  "$TRIM_INTENT_LEGACY_DELTA_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "trimSelectedSegmentIntent",
    direction: "left",
    delta: Math.max(1, Math.round(deltaUs))
  }, "trim");'

assert_pattern_rejects \
  "raw trim delta callback" \
  "$TRIM_CALLBACK_DELTA_PATTERN" \
  'onTrimSelectedSegment?.("right", Math.abs(deltaUs))'

assert_pattern_rejects \
  "legacy selected-segment splitAt timestamp" \
  "$SPLIT_INTENT_LEGACY_SPLIT_AT_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "splitSelectedSegmentIntent",
    splitAt: Math.max(0, Math.round(playheadUs))
  }, "split");'

assert_pattern_rejects \
  "renderer playhead passed through split callback" \
  "$SPLIT_CALLBACK_PLAYHEAD_PATTERN" \
  'onClick={() => onSplitSelectedSegment?.(playheadUs)}'

assert_pattern_rejects \
  "legacy selected-segment keyframe at timestamp" \
  "$KEYFRAME_INTENT_LEGACY_AT_PATTERN" \
  'void executeProjectTimelineIntent({
    kind: "setSelectedSegmentKeyframe",
    property,
    at: normalizePlayheadTime(playheadUs),
    interpolation: "linear",
    easing: "none"
  }, "keyframe");'

assert_pattern_rejects \
  "row-derived keyframe deletion time" \
  "$KEYFRAME_REMOVE_CALLBACK_AT_PATTERN" \
  'onRemoveKeyframe(keyframe.property, selected.targetTimerange.start + keyframe.at)'

assert_pattern_rejects \
  "project session response exposing renderer-owned state" \
  "$PROJECT_SESSION_RESPONSE_STATE_PATTERN" \
  'export type ProjectSessionTimelineIntentResponse = {
  sessionId: string;
  revision: number;
  draft: Draft;
  commandState: CommandState;
  selection: TimelineSelection;
};'

assert_pattern_rejects \
  "renderer workspace storing canonical project state" \
  "$RENDERER_WORKSPACE_STATE_PROJECT_STATE_PATTERN" \
  'export type WorkspaceState = {
  projectState: ProjectEntryState;
  draft: Draft;
  commandState: CommandState;
};'

assert_pattern_rejects \
  "renderer reading project session draft response" \
  "$RENDERER_SESSION_RESPONSE_READ_PATTERN" \
  'const nextDraft = result.data.draft;
const selected = openedProject?.data?.selection;'

assert_pattern_rejects \
  "renderer draft-bearing preview helper" \
  "$RENDERER_PREVIEW_DRAFT_COMMAND_HELPER_PATTERN" \
  'buildRequestPreviewFrameCommand({ draft: current.draft, targetTime });
type X = RequestPreviewSegmentCommandPayload;'

assert_pattern_rejects \
  "raw Track/Segment project session view model field" \
  "$PROJECT_SESSION_RAW_VIEW_MODEL_PATTERN" \
  'export type TimelineTrackRowViewModel = {
  track: Track;
  segments: TimelineSegmentViewModel[];
};

struct TimelineSegmentViewModel {
    segment: Segment,
}'

assert_pattern_rejects \
  "renderer raw timeline view model consumption" \
  "$RENDERER_PRODUCT_RAW_TIMELINE_VM_PATTERN" \
  'const active = row.track.kind === "audio" ? !row.track.muted : row.track.visible;
const id = segment.segment.segmentId;
const text = selectedSegment.segment.text;'

fail_if_diff schemas apps/desktop-electron/src/generated

exit "$failed"

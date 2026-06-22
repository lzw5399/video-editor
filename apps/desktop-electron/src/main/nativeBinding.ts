import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";

import type { ExportPreset } from "../generated/CommandEnvelope";
import type {
  AudioOutputDeviceSummary,
  AudioPreviewCommandResponse,
  AudioPreviewStatusResponse,
  ArtifactMaintenanceResult,
  ArtifactQuotaStatus,
  ArtifactStatusSummary,
  CommandDelta,
  CommandEvent,
  CommandResultEnvelope,
  ExportJobStatusResponse,
  MissingMaterialCommandDiagnostic,
  RuntimeCapabilityReport,
  WaveformDisplayPeaksResponse
} from "../generated/CommandResultEnvelope";
import type {
  AudioEffectSlot,
  AudioFade,
  AudioPanBalance,
  DraftCanvasConfig,
  KeyframeEasing,
  KeyframeInterpolation,
  KeyframeProperty,
  Keyframe,
  Material,
  MaterialId,
  MaterialKind,
  Microseconds,
  SegmentAudio,
  SourceTimerange,
  SegmentVisual,
  SegmentVolume,
  TargetTimerange,
  TextSegment,
  TrackKind
} from "../generated/Draft";

type PingResponse = { pong: boolean };
type VersionResponse = { coreVersion: string; contractVersion: string };
export type RuntimeDiscoverySource = { kind: "bundled"; directory: string };
export type RuntimeDiscoveredBinary = {
  kind: "ffmpeg" | "ffprobe";
  path: string;
  source: RuntimeDiscoverySource;
  version: string;
};
export type RuntimeConfigResponse = {
  ffmpeg: RuntimeDiscoveredBinary;
  ffprobe: RuntimeDiscoveredBinary;
};

type NativeBinding = {
  ping: () => CommandResultEnvelope<PingResponse>;
  version: () => CommandResultEnvelope<VersionResponse>;
  configureBundledRuntimeDirectory: (directory: string) => void;
  probeMediaRuntime: () => CommandResultEnvelope<RuntimeConfigResponse>;
  probeRuntimeCapabilities: () => CommandResultEnvelope<RuntimeCapabilityReport>;
  createProjectSession: (request: CreateProjectSessionRequest) => CommandResultEnvelope<ProjectSessionOpenResponse>;
  openProjectSession: (request: OpenProjectSessionRequest) => CommandResultEnvelope<ProjectSessionOpenResponse>;
  closeProjectSession: (request: ProjectSessionRequest) => CommandResultEnvelope<ProjectSessionClosedResponse>;
  executeProjectIntent: (request: ExecuteProjectIntentRequest) => CommandResultEnvelope<ProjectSessionIntentResponse>;
  listProjectSessionMaterials: (request: ProjectSessionReadRequest) => CommandResultEnvelope<ProjectSessionMaterialsResponse>;
  listProjectSessionMissingMaterials: (
    request: ProjectSessionReadRequest
  ) => CommandResultEnvelope<ProjectSessionMissingMaterialsResponse>;
  startProjectSessionExport: (request: StartProjectSessionExportRequest) => CommandResultEnvelope<ExportJobStatusResponse>;
  getExportJobStatus: (request: ExportJobRequest) => CommandResultEnvelope<ExportJobStatusResponse>;
  cancelExport: (request: ExportJobRequest) => CommandResultEnvelope<ExportJobStatusResponse>;
  createAudioPreviewSession: (request: AudioPreviewRequest) => CommandResultEnvelope<AudioPreviewCommandResponse>;
  playAudioPreview: (request: AudioPreviewRequest) => CommandResultEnvelope<AudioPreviewCommandResponse>;
  pauseAudioPreview: (request: AudioPreviewRequest) => CommandResultEnvelope<AudioPreviewCommandResponse>;
  stopAudioPreview: (request: AudioPreviewRequest) => CommandResultEnvelope<AudioPreviewCommandResponse>;
  seekAudioPreview: (request: AudioPreviewRequest) => CommandResultEnvelope<AudioPreviewCommandResponse>;
  cancelAudioPreview: (request: AudioPreviewRequest) => CommandResultEnvelope<AudioPreviewCommandResponse>;
  getAudioPreviewStatus: (request: AudioPreviewRequest) => CommandResultEnvelope<AudioPreviewStatusResponse>;
  listAudioOutputDevices: (request: AudioPreviewRequest) => CommandResultEnvelope<AudioOutputDeviceSummary[]>;
  selectAudioOutputDevice: (request: AudioPreviewRequest) => CommandResultEnvelope<AudioPreviewCommandResponse>;
  getWaveformDisplayPeaks: (request: AudioPreviewRequest) => CommandResultEnvelope<WaveformDisplayPeaksResponse>;
  refreshWaveformStatus: (request: AudioPreviewRequest) => CommandResultEnvelope<WaveformDisplayPeaksResponse>;
  getArtifactStatus: (request: ArtifactStatusRequest) => CommandResultEnvelope<ArtifactStatusSummary>;
  refreshArtifactStatus: (request: ArtifactStatusRequest) => CommandResultEnvelope<ArtifactStatusSummary>;
  retryArtifactGeneration: (request: ArtifactGenerationActionRequest) => CommandResultEnvelope<ArtifactStatusSummary>;
  resumeArtifactGeneration: (request: ArtifactGenerationActionRequest) => CommandResultEnvelope<ArtifactStatusSummary>;
  cancelArtifactGeneration: (request: ArtifactGenerationActionRequest) => CommandResultEnvelope<ArtifactStatusSummary>;
  getArtifactQuotaStatus: (request: ArtifactQuotaRequest) => CommandResultEnvelope<ArtifactQuotaStatus>;
  runArtifactGarbageCollection: (
    request: ArtifactGarbageCollectionRequest
  ) => CommandResultEnvelope<ArtifactMaintenanceResult>;
  createRealtimePreviewSession: (config: RealtimePreviewSessionConfig) => RealtimePreviewSessionResponse;
  subscribeRealtimePreviewEvents: (
    callback: (errorOrEventJson: unknown, eventJson?: string) => void
  ) => RealtimePreviewEventSubscriptionResponse;
  unsubscribeRealtimePreviewEvents: () => RealtimePreviewEventSubscriptionResponse;
  closeRealtimePreviewSession: (request: RealtimePreviewSessionRequest) => RealtimePreviewClosedResponse;
  attachRealtimePreviewSurface: (request: RealtimePreviewSurfaceRequest) => RealtimePreviewGenerationResponse;
  updateRealtimePreviewSurfaceBounds: (request: RealtimePreviewSurfaceBoundsRequest) => RealtimePreviewGenerationResponse;
  detachRealtimePreviewSurface: (request: RealtimePreviewSessionRequest) => RealtimePreviewGenerationResponse;
  updateRealtimePreviewProjectSessionSnapshot: (
    request: RealtimePreviewProjectSessionSnapshotRequest
  ) => RealtimePreviewGenerationResponse;
  seekRealtimePreview: (request: RealtimePreviewSeekRequest) => RealtimePreviewGenerationResponse;
  playRealtimePreview: (request: RealtimePreviewSessionRequest) => RealtimePreviewGenerationResponse;
  pauseRealtimePreview: (request: RealtimePreviewSessionRequest) => RealtimePreviewGenerationResponse;
  stopRealtimePreview: (request: RealtimePreviewSessionRequest) => RealtimePreviewGenerationResponse;
  requestRealtimePreviewFrame: (request: RealtimePreviewFrameRequest) => RealtimePreviewFrameResponse;
  nextRealtimePreviewCancellationToken: (request: RealtimePreviewSessionRequest) => number;
  cancelRealtimePreviewRequest: (request: RealtimePreviewCancellationRequest) => RealtimePreviewCanceledResponse;
  getRealtimePreviewTelemetry: (request: RealtimePreviewSessionRequest) => RealtimePreviewTelemetryResponse;
  getRealtimePreviewPresentationState: (
    request: RealtimePreviewSessionRequest
  ) => RealtimePreviewPresentationStateResponse;
};

export type OpenProjectSessionRequest = {
  bundlePath: string;
  sessionId?: string;
};

export type CreateProjectSessionRequest = {
  bundlePath: string;
  sessionId?: string;
  draftId?: string;
  draftName?: string;
  fixture?: "demo";
};

export type ProjectSessionRequest = {
  sessionId: string;
};

export type ProjectSessionReadRequest = {
  sessionId: string;
  expectedRevision: number;
};

export type StartProjectSessionExportRequest = {
  sessionId: string;
  expectedRevision: number;
  outputPath: string;
  preset: ExportPreset;
};

export type ExportJobRequest = {
  jobId: string;
};

export type AudioPreviewRequest = {
  projectSessionId?: string | null;
  expectedRevision?: number | null;
  sessionId?: string | null;
  materialId?: MaterialId | null;
  targetTime?: Microseconds | null;
  targetTimerange?: TargetTimerange | null;
  playbackGeneration?: number | null;
  deviceSelectionId?: string | null;
  maxPeakBins?: number | null;
};

export type ArtifactStatusRequest = {
  sessionId: string;
  bundlePath: string;
  materialId?: MaterialId | null;
};

export type ArtifactGenerationActionRequest = {
  sessionId: string;
  bundlePath: string;
  jobId: string;
};

export type ArtifactQuotaRequest = {
  sessionId: string;
  bundlePath: string;
};

export type ArtifactGarbageCollectionRequest = {
  sessionId: string;
  bundlePath: string;
  dryRun: boolean;
};

export type ProjectIntent =
  | {
      kind: "importMaterial";
      materialPath: string;
      materialId?: MaterialId | null;
      displayName?: string | null;
      materialKindHint?: MaterialKind | null;
    }
  | { kind: "addTimelineSegmentIntent"; materialId: MaterialId }
  | { kind: "selectTimelineItemIntent"; itemHandle: string }
  | { kind: "moveSelectedSegmentIntent"; startAt: Microseconds }
  | { kind: "splitSelectedSegmentIntent" }
  | { kind: "trimSelectedSegmentIntent"; direction: "left" | "right"; trimAt: Microseconds }
  | { kind: "deleteSelectedSegment" }
  | { kind: "addTextSegmentIntent"; content: string }
  | { kind: "editSelectedText"; text: TextSegment }
  | {
      kind: "importSubtitleSrtIntent";
      srtContent: string;
    }
  | { kind: "addAudioSegmentIntent"; materialId?: MaterialId | null }
  | { kind: "setSelectedSegmentVolume"; volume: SegmentVolume }
  | {
      kind: "updateSelectedSegmentAudio";
      gainMillis?: number | null;
      panBalanceMillis?: AudioPanBalance | null;
      fadeInDuration?: AudioFade | null;
      fadeOutDuration?: AudioFade | null;
      effectSlots?: AudioEffectSlot[] | null;
    }
  | { kind: "addTrackIntent"; trackKind: TrackKind }
  | { kind: "renameSelectedTrack"; name: string }
  | { kind: "setSelectedTrackLock"; locked: boolean }
  | { kind: "setSelectedTrackVisibility"; visible: boolean }
  | { kind: "setSelectedTrackMute"; muted: boolean }
  | { kind: "setSessionPlayhead"; playhead: Microseconds }
  | { kind: "updateDraftCanvasConfig"; canvasConfig: DraftCanvasConfig }
  | { kind: "updateSelectedSegmentVisual"; visual: SegmentVisual }
  | {
      kind: "setSelectedSegmentKeyframe";
      property: KeyframeProperty;
      interpolation: KeyframeInterpolation;
      easing: KeyframeEasing;
    }
  | { kind: "removeSelectedSegmentKeyframe"; property: KeyframeProperty }
  | { kind: "undoTimelineEdit" }
  | { kind: "redoTimelineEdit" };

export type ExecuteProjectIntentRequest = {
  sessionId: string;
  expectedRevision: number;
  intent: ProjectIntent;
};

export type ProjectSessionOpenResponse = {
  sessionId: string;
  revision: number;
  viewModel: ProjectSessionViewModel;
  bundlePath: string;
  projectJsonPath: string;
  warnings: string[];
};

export type ProjectSessionMaterialsResponse = {
  sessionId: string;
  revision: number;
  bundlePath: string;
  projectJsonPath: string;
  materials: Material[];
};

export type ProjectSessionMissingMaterialsResponse = {
  sessionId: string;
  revision: number;
  bundlePath: string;
  projectJsonPath: string;
  diagnostics: MissingMaterialCommandDiagnostic[];
};

export type ProjectSessionClosedResponse = {
  sessionId: string;
  closed: boolean;
};

export type ProjectSessionTimelineIntentResponse = {
  sessionId: string;
  revision: number;
  viewModel: ProjectSessionViewModel;
  events: CommandEvent[];
  delta: CommandDelta;
  bundlePath: string;
  projectJsonPath: string;
};

export type ProjectSessionImportMaterialResponse = {
  sessionId: string;
  revision: number;
  material: Material;
  materials: Material[];
  diagnostic?: MissingMaterialCommandDiagnostic | null;
  viewModel: ProjectSessionViewModel;
  events: CommandEvent[];
  delta: CommandDelta;
  bundlePath: string;
  projectJsonPath: string;
};

export type ProjectSessionIntentResponse = ProjectSessionTimelineIntentResponse | ProjectSessionImportMaterialResponse;

export type ProjectSessionViewModel = {
  project: ProjectSummaryViewModel;
  editControls: EditControlsViewModel;
  timeline: TimelineViewModel;
  selectedTrack: SelectedTrackViewModel | null;
  selectedSegment: SelectedSegmentViewModel | null;
};

export type EditControlsViewModel = {
  canUndo: boolean;
  canRedo: boolean;
  snappingEnabled: boolean;
  snappingLabel: string;
  hasSelectedSegment: boolean;
  hasSelectedTrack: boolean;
};

export type ProjectSummaryViewModel = {
  draftName: string;
  canvasConfig: DraftCanvasConfig;
  sequenceDuration: Microseconds;
  frameDuration: Microseconds;
  trackCount: number;
  materialCount: number;
};

export type SelectedTrackViewModel = {
  trackId: string;
  selectionHandle: string;
  name: string;
  kindLabel: string;
  muted: boolean;
  locked: boolean;
  visible: boolean;
};

export type SelectedSegmentViewModel = {
  segmentKey: string;
  selectionHandle: string;
  track: SelectedTrackViewModel;
  material: Material | null;
  sourceTimerange: SourceTimerange;
  targetTimerange: TargetTimerange;
  sourceLabel: string;
  targetLabel: string;
  visual: SegmentVisual;
  volume: SegmentVolume;
  audio: SegmentAudio;
  text: TextSegment | null;
  keyframes: Keyframe[];
  hasText: boolean;
  hasAudioControls: boolean;
};

export type TimelineViewModel = {
  rows: TimelineTrackRowViewModel[];
  duration: Microseconds;
  rulerTicks: Microseconds[];
  capabilities: TimelineCapabilitiesViewModel;
};

export type TimelineCapabilitiesViewModel = {
  hasTextTrack: boolean;
  hasAudioTrack: boolean;
};

export type TimelineTrackRowViewModel = {
  rowKey: string;
  selectionHandle: string;
  name: string;
  symbol: string;
  kindLabel: string;
  statusLabel: string;
  lockLabel: string;
  visibilityLabel: string;
  muteLabel: string;
  rowClassName: string;
  selected: boolean;
  lockActive: boolean;
  visibilityActive: boolean;
  muteActive: boolean;
  canToggleVisibility: boolean;
  canToggleMute: boolean;
  nextLocked: boolean;
  nextVisible: boolean;
  nextMuted: boolean;
  visibilitySymbol: string;
  segments: TimelineSegmentViewModel[];
};

export type TimelineSegmentViewModel = {
  segmentKey: string;
  selectionHandle: string;
  waveformMaterialId: MaterialId | null;
  material: Material | null;
  label: string;
  sourceLabel: string;
  targetLabel: string;
  visualKind: TimelineSegmentVisualKind;
  start: Microseconds;
  duration: Microseconds;
  selected: boolean;
  keyframeMarkers: TimelineKeyframeMarkerViewModel[];
};

export type TimelineKeyframeMarkerViewModel = {
  markerKey: string;
  positionPerMille: number;
  title: string;
  ariaLabel: string;
};

export type TimelineSegmentVisualKind = "video" | "image" | "audio" | "text" | "sticker" | "filter";

export type RealtimePreviewSessionConfig = {
  sessionLabel: string;
  frameRateNumerator: number;
  frameRateDenominator: number;
  playbackRateNumerator: number;
  playbackRateDenominator: number;
};

export type RealtimePreviewSessionRequest = {
  sessionId: string;
};

export type RealtimePreviewSessionResponse = {
  sessionId: string;
  playbackGeneration: number;
};

export type RealtimePreviewEventSubscriptionResponse = {
  subscribed: boolean;
};

export type RealtimePreviewBindingEvent = {
  sessionId: string;
  kind: "sessionCreated" | "sessionClosed" | "controlChanged" | "framePresented" | "playbackEnded" | "playbackError";
  playbackGeneration: number;
  targetTimeMicroseconds?: number | null;
  droppedFrameCount?: number | null;
  errorMessage?: string | null;
};

export type RealtimePreviewClosedResponse = {
  sessionId: string;
  closed: boolean;
};

export type RealtimePreviewSurfaceDescriptor = {
  kind: "windowsHwnd" | "macosNsView" | "mock" | "offscreen";
  parentHandle?: number;
  parentHandleHex?: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactorMillis: number;
};

export type RealtimePreviewSurfaceRequest = {
  sessionId: string;
  surface: RealtimePreviewSurfaceDescriptor;
};

export type RealtimePreviewSurfaceBounds = {
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactorMillis: number;
};

export type RealtimePreviewSurfaceBoundsRequest = {
  sessionId: string;
  bounds: RealtimePreviewSurfaceBounds;
};

export type RealtimePreviewGenerationResponse = {
  playbackGeneration: number;
};

export type RealtimePreviewProjectSessionSnapshotRequest = {
  sessionId: string;
  projectSessionId: string;
  expectedRevision: number;
};

export type RealtimePreviewSeekRequest = {
  sessionId: string;
  targetTimeMicroseconds: number;
};

export type RealtimePreviewBackendUsed = "mock" | "gpu" | "offscreen" | "previewArtifact" | "ffmpegArtifact" | "none";

export type RealtimePreviewFallbackReason =
  | "noGpuAdapter"
  | "surfaceUnavailable"
  | "surfaceLost"
  | "unsupportedGraphIntent"
  | "frameProviderUnavailable"
  | "textParityUnsupported"
  | "nativeChildWindowFailed"
  | "offscreenReadbackRequired"
  | "previewArtifactCacheHit"
  | "ffmpegArtifactGenerated"
  | "canceled"
  | "staleGeneration";

export type RealtimePreviewDiagnostic = {
  entityId?: string;
  domain: string;
  support: string | { degraded?: { reason: string }; unsupported?: { reason: string } };
  reason: string;
  fallback?: RealtimePreviewFallbackReason;
  fallbackUsed: boolean;
  canceled: boolean;
  cancellationToken?: number;
};

export type RealtimePreviewRequestMode = "seek" | "scrub" | "playbackTick" | "firstFrame";

export type RealtimePreviewFrameRequest = {
  sessionId: string;
  frame: {
    targetTimeMicroseconds: number;
    playbackGeneration: number;
    queueLatencyMs: number;
    renderDurationMs: number;
    mode: RealtimePreviewRequestMode;
    cancellationToken?: number;
    fallbackReason?: RealtimePreviewFallbackReason;
    cacheHit: boolean;
  };
};

export type RealtimePreviewFrameResponse = {
  targetTimeMicroseconds: number;
  playbackGeneration: number;
  presented: boolean;
  staleRejected: boolean;
  canceled: boolean;
  cancellationToken?: number;
  backend: RealtimePreviewBackendUsed;
  fallback?: RealtimePreviewFallbackReason;
  diagnostics: RealtimePreviewDiagnostic[];
  telemetry: RealtimePreviewTelemetryResponse;
};

export type RealtimePreviewCancellationRequest = {
  sessionId: string;
  cancellationToken: number;
};

export type RealtimePreviewCanceledResponse = {
  cancellationToken: number;
  canceled: boolean;
};

export type RealtimePreviewTelemetryResponse = {
  firstFrameLatencyMs: number | null;
  seekLatencyMs: number | null;
  queueLatencyMs: number;
  renderDurationMs: number;
  presentedFrameCount: number;
  droppedFrameCount: number;
  repeatedFrameCount: number;
  staleRejectedCount: number;
  canceledRequestCount: number;
  fallbackCount: number;
  cacheHitCount: number;
  targetTimeMicroseconds: number;
  playbackGeneration: number;
  framePacing: RealtimePreviewFramePacingTelemetry;
};

export type RealtimePreviewFramePacingTelemetry = {
  sampleCount: number;
  intervalP50Ms: number | null;
  intervalP95Ms: number | null;
  intervalMaxMs: number | null;
  scheduleLatenessP95Ms: number | null;
  scheduleLatenessMaxMs: number | null;
  samples: RealtimePreviewFramePacingSample[];
};

export type RealtimePreviewFramePacingSample = {
  targetTimeMicroseconds: number;
  intervalMs?: number | null;
  scheduleLatenessMs: number;
  renderDurationMs: number;
  droppedFrameCount: number;
};

export type RealtimePreviewPresentationStateResponse = {
  available: boolean;
  backend: "nativeVideoBridge" | "renderGraphGpu" | "none";
  unsupportedReason?: string | null;
  evidence?: RealtimePreviewPresentationEvidence | null;
  surfacePlacement?: RealtimePreviewSurfacePlacementEvidence | null;
};

export type RealtimePreviewPresentationEvidence = {
  source: "nativeVideoBridge" | "renderGraphGpuComposited";
  digest: string;
  width: number;
  height: number;
  byteCount: number;
  targetTimeMicroseconds: number;
  presentedFrames: number;
  submittedDraws: number;
  activeTextOverlays?: RealtimePreviewTextOverlayEvidence[];
};

export type RealtimePreviewTextOverlayEvidence = {
  source: "text" | "subtitle";
  content: string;
};

export type RealtimePreviewScreenRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type RealtimePreviewSurfacePlacementEvidence = {
  nativeScreenRect: RealtimePreviewScreenRect;
  drawableLifecycleDiagnostic?: string | null;
};

const requireNative = createRequire(__filename);
const MAX_LOAD_ERROR_LENGTH = 600;

let cachedBinding: NativeBinding | null | undefined;
let cachedLoadError: string | null = null;

export function ping(): CommandResultEnvelope<PingResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("ping");
  }
  return binding.ping();
}

export function version(): CommandResultEnvelope<VersionResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("version");
  }
  return binding.version();
}

export function configureBundledRuntimeDirectory(directory: string): void {
  requireLoadedBinding().configureBundledRuntimeDirectory(directory);
}

export function probeMediaRuntime(): CommandResultEnvelope<RuntimeConfigResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("probeMediaRuntime");
  }
  return binding.probeMediaRuntime();
}

export function probeRuntimeCapabilities(): CommandResultEnvelope<RuntimeCapabilityReport> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("probeRuntimeCapabilities");
  }
  return binding.probeRuntimeCapabilities();
}

export function createProjectSession(request: CreateProjectSessionRequest): CommandResultEnvelope<ProjectSessionOpenResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("createProjectSession");
  }
  return binding.createProjectSession(request);
}

export function openProjectSession(request: OpenProjectSessionRequest): CommandResultEnvelope<ProjectSessionOpenResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("openProjectSession");
  }
  return binding.openProjectSession(request);
}

export function closeProjectSession(request: ProjectSessionRequest): CommandResultEnvelope<ProjectSessionClosedResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("closeProjectSession");
  }
  return binding.closeProjectSession(request);
}

export function executeProjectIntent(
  request: ExecuteProjectIntentRequest
): CommandResultEnvelope<ProjectSessionIntentResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("executeProjectIntent");
  }
  return binding.executeProjectIntent(request);
}

export function listProjectSessionMaterials(
  request: ProjectSessionReadRequest
): CommandResultEnvelope<ProjectSessionMaterialsResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("listProjectSessionMaterials");
  }
  return binding.listProjectSessionMaterials(request);
}

export function listProjectSessionMissingMaterials(
  request: ProjectSessionReadRequest
): CommandResultEnvelope<ProjectSessionMissingMaterialsResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("listProjectSessionMissingMaterials");
  }
  return binding.listProjectSessionMissingMaterials(request);
}

export function startProjectSessionExport(
  request: StartProjectSessionExportRequest
): CommandResultEnvelope<ExportJobStatusResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("startProjectSessionExport");
  }
  return binding.startProjectSessionExport(request);
}

export function getExportJobStatus(request: ExportJobRequest): CommandResultEnvelope<ExportJobStatusResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("getExportJobStatus");
  }
  return binding.getExportJobStatus(request);
}

export function cancelExport(request: ExportJobRequest): CommandResultEnvelope<ExportJobStatusResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("cancelExport");
  }
  return binding.cancelExport(request);
}

export function createAudioPreviewSession(request: AudioPreviewRequest): CommandResultEnvelope<AudioPreviewCommandResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("createAudioPreviewSession");
  }
  return binding.createAudioPreviewSession(request);
}

export function playAudioPreview(request: AudioPreviewRequest): CommandResultEnvelope<AudioPreviewCommandResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("playAudioPreview");
  }
  return binding.playAudioPreview(request);
}

export function pauseAudioPreview(request: AudioPreviewRequest): CommandResultEnvelope<AudioPreviewCommandResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("pauseAudioPreview");
  }
  return binding.pauseAudioPreview(request);
}

export function stopAudioPreview(request: AudioPreviewRequest): CommandResultEnvelope<AudioPreviewCommandResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("stopAudioPreview");
  }
  return binding.stopAudioPreview(request);
}

export function seekAudioPreview(request: AudioPreviewRequest): CommandResultEnvelope<AudioPreviewCommandResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("seekAudioPreview");
  }
  return binding.seekAudioPreview(request);
}

export function cancelAudioPreview(request: AudioPreviewRequest): CommandResultEnvelope<AudioPreviewCommandResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("cancelAudioPreview");
  }
  return binding.cancelAudioPreview(request);
}

export function getAudioPreviewStatus(request: AudioPreviewRequest): CommandResultEnvelope<AudioPreviewStatusResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("getAudioPreviewStatus");
  }
  return binding.getAudioPreviewStatus(request);
}

export function listAudioOutputDevices(request: AudioPreviewRequest): CommandResultEnvelope<AudioOutputDeviceSummary[]> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("listAudioOutputDevices");
  }
  return binding.listAudioOutputDevices(request);
}

export function selectAudioOutputDevice(request: AudioPreviewRequest): CommandResultEnvelope<AudioPreviewCommandResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("selectAudioOutputDevice");
  }
  return binding.selectAudioOutputDevice(request);
}

export function getWaveformDisplayPeaks(request: AudioPreviewRequest): CommandResultEnvelope<WaveformDisplayPeaksResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("getWaveformDisplayPeaks");
  }
  return binding.getWaveformDisplayPeaks(request);
}

export function refreshWaveformStatus(request: AudioPreviewRequest): CommandResultEnvelope<WaveformDisplayPeaksResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("refreshWaveformStatus");
  }
  return binding.refreshWaveformStatus(request);
}

export function getArtifactStatus(request: ArtifactStatusRequest): CommandResultEnvelope<ArtifactStatusSummary> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("getArtifactStatus");
  }
  return binding.getArtifactStatus(request);
}

export function refreshArtifactStatus(request: ArtifactStatusRequest): CommandResultEnvelope<ArtifactStatusSummary> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("refreshArtifactStatus");
  }
  return binding.refreshArtifactStatus(request);
}

export function retryArtifactGeneration(
  request: ArtifactGenerationActionRequest
): CommandResultEnvelope<ArtifactStatusSummary> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("retryArtifactGeneration");
  }
  return binding.retryArtifactGeneration(request);
}

export function resumeArtifactGeneration(
  request: ArtifactGenerationActionRequest
): CommandResultEnvelope<ArtifactStatusSummary> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("resumeArtifactGeneration");
  }
  return binding.resumeArtifactGeneration(request);
}

export function cancelArtifactGeneration(
  request: ArtifactGenerationActionRequest
): CommandResultEnvelope<ArtifactStatusSummary> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("cancelArtifactGeneration");
  }
  return binding.cancelArtifactGeneration(request);
}

export function getArtifactQuotaStatus(request: ArtifactQuotaRequest): CommandResultEnvelope<ArtifactQuotaStatus> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("getArtifactQuotaStatus");
  }
  return binding.getArtifactQuotaStatus(request);
}

export function runArtifactGarbageCollection(
  request: ArtifactGarbageCollectionRequest
): CommandResultEnvelope<ArtifactMaintenanceResult> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("runArtifactGarbageCollection");
  }
  return binding.runArtifactGarbageCollection(request);
}

export function createRealtimePreviewSession(config: RealtimePreviewSessionConfig): RealtimePreviewSessionResponse {
  return requireLoadedBinding().createRealtimePreviewSession(config);
}

export function subscribeRealtimePreviewEvents(
  callback: (errorOrEventJson: unknown, eventJson?: string) => void
): RealtimePreviewEventSubscriptionResponse {
  return requireLoadedBinding().subscribeRealtimePreviewEvents(callback);
}

export function unsubscribeRealtimePreviewEvents(): RealtimePreviewEventSubscriptionResponse {
  return requireLoadedBinding().unsubscribeRealtimePreviewEvents();
}

export function closeRealtimePreviewSession(request: RealtimePreviewSessionRequest): RealtimePreviewClosedResponse {
  return requireLoadedBinding().closeRealtimePreviewSession(request);
}

export function attachRealtimePreviewSurface(request: RealtimePreviewSurfaceRequest): RealtimePreviewGenerationResponse {
  return requireLoadedBinding().attachRealtimePreviewSurface(request);
}

export function updateRealtimePreviewSurfaceBounds(
  request: RealtimePreviewSurfaceBoundsRequest
): RealtimePreviewGenerationResponse {
  return requireLoadedBinding().updateRealtimePreviewSurfaceBounds(request);
}

export function detachRealtimePreviewSurface(request: RealtimePreviewSessionRequest): RealtimePreviewGenerationResponse {
  return requireLoadedBinding().detachRealtimePreviewSurface(request);
}

export function updateRealtimePreviewProjectSessionSnapshot(
  request: RealtimePreviewProjectSessionSnapshotRequest
): RealtimePreviewGenerationResponse {
  return requireLoadedBinding().updateRealtimePreviewProjectSessionSnapshot(request);
}

export function seekRealtimePreview(request: RealtimePreviewSeekRequest): RealtimePreviewGenerationResponse {
  return requireLoadedBinding().seekRealtimePreview(request);
}

export function playRealtimePreview(request: RealtimePreviewSessionRequest): RealtimePreviewGenerationResponse {
  return requireLoadedBinding().playRealtimePreview(request);
}

export function pauseRealtimePreview(request: RealtimePreviewSessionRequest): RealtimePreviewGenerationResponse {
  return requireLoadedBinding().pauseRealtimePreview(request);
}

export function stopRealtimePreview(request: RealtimePreviewSessionRequest): RealtimePreviewGenerationResponse {
  return requireLoadedBinding().stopRealtimePreview(request);
}

export function requestRealtimePreviewFrame(request: RealtimePreviewFrameRequest): RealtimePreviewFrameResponse {
  return requireLoadedBinding().requestRealtimePreviewFrame(request);
}

export function nextRealtimePreviewCancellationToken(request: RealtimePreviewSessionRequest): number {
  return requireLoadedBinding().nextRealtimePreviewCancellationToken(request);
}

export function cancelRealtimePreviewRequest(request: RealtimePreviewCancellationRequest): RealtimePreviewCanceledResponse {
  return requireLoadedBinding().cancelRealtimePreviewRequest(request);
}

export function getRealtimePreviewTelemetry(request: RealtimePreviewSessionRequest): RealtimePreviewTelemetryResponse {
  return requireLoadedBinding().getRealtimePreviewTelemetry(request);
}

export function getRealtimePreviewPresentationState(
  request: RealtimePreviewSessionRequest
): RealtimePreviewPresentationStateResponse {
  return requireLoadedBinding().getRealtimePreviewPresentationState(request);
}

function loadNativeBinding(): NativeBinding | null {
  if (cachedBinding !== undefined) {
    return cachedBinding;
  }

  const bindingPath = resolveNativeBindingPath();
  try {
    const loaded = requireNative(bindingPath) as Partial<NativeBinding>;
    if (
      typeof loaded.ping !== "function" ||
      typeof loaded.version !== "function" ||
      typeof loaded.configureBundledRuntimeDirectory !== "function" ||
      typeof loaded.probeMediaRuntime !== "function" ||
      typeof loaded.probeRuntimeCapabilities !== "function" ||
      typeof loaded.createProjectSession !== "function" ||
      typeof loaded.openProjectSession !== "function" ||
      typeof loaded.closeProjectSession !== "function" ||
      typeof loaded.executeProjectIntent !== "function" ||
      typeof loaded.listProjectSessionMaterials !== "function" ||
      typeof loaded.listProjectSessionMissingMaterials !== "function" ||
      typeof loaded.startProjectSessionExport !== "function" ||
      typeof loaded.getExportJobStatus !== "function" ||
      typeof loaded.cancelExport !== "function" ||
      typeof loaded.createAudioPreviewSession !== "function" ||
      typeof loaded.playAudioPreview !== "function" ||
      typeof loaded.pauseAudioPreview !== "function" ||
      typeof loaded.stopAudioPreview !== "function" ||
      typeof loaded.seekAudioPreview !== "function" ||
      typeof loaded.cancelAudioPreview !== "function" ||
      typeof loaded.getAudioPreviewStatus !== "function" ||
      typeof loaded.listAudioOutputDevices !== "function" ||
      typeof loaded.selectAudioOutputDevice !== "function" ||
      typeof loaded.getWaveformDisplayPeaks !== "function" ||
      typeof loaded.refreshWaveformStatus !== "function" ||
      typeof loaded.getArtifactStatus !== "function" ||
      typeof loaded.refreshArtifactStatus !== "function" ||
      typeof loaded.retryArtifactGeneration !== "function" ||
      typeof loaded.resumeArtifactGeneration !== "function" ||
      typeof loaded.cancelArtifactGeneration !== "function" ||
      typeof loaded.getArtifactQuotaStatus !== "function" ||
      typeof loaded.runArtifactGarbageCollection !== "function" ||
      typeof loaded.createRealtimePreviewSession !== "function" ||
      typeof loaded.subscribeRealtimePreviewEvents !== "function" ||
      typeof loaded.unsubscribeRealtimePreviewEvents !== "function" ||
      typeof loaded.closeRealtimePreviewSession !== "function" ||
      typeof loaded.attachRealtimePreviewSurface !== "function" ||
      typeof loaded.updateRealtimePreviewSurfaceBounds !== "function" ||
      typeof loaded.detachRealtimePreviewSurface !== "function" ||
      typeof loaded.updateRealtimePreviewProjectSessionSnapshot !== "function" ||
      typeof loaded.seekRealtimePreview !== "function" ||
      typeof loaded.playRealtimePreview !== "function" ||
      typeof loaded.pauseRealtimePreview !== "function" ||
      typeof loaded.stopRealtimePreview !== "function" ||
      typeof loaded.requestRealtimePreviewFrame !== "function" ||
      typeof loaded.nextRealtimePreviewCancellationToken !== "function" ||
      typeof loaded.cancelRealtimePreviewRequest !== "function" ||
      typeof loaded.getRealtimePreviewTelemetry !== "function" ||
      typeof loaded.getRealtimePreviewPresentationState !== "function"
    ) {
      throw new Error("Native binding does not expose the required editor and realtime preview functions");
    }

    cachedBinding = {
      ping: loaded.ping,
      version: loaded.version,
      configureBundledRuntimeDirectory: loaded.configureBundledRuntimeDirectory,
      probeMediaRuntime: loaded.probeMediaRuntime,
      probeRuntimeCapabilities: loaded.probeRuntimeCapabilities,
      createProjectSession: loaded.createProjectSession,
      openProjectSession: loaded.openProjectSession,
      closeProjectSession: loaded.closeProjectSession,
      executeProjectIntent: loaded.executeProjectIntent,
      listProjectSessionMaterials: loaded.listProjectSessionMaterials,
      listProjectSessionMissingMaterials: loaded.listProjectSessionMissingMaterials,
      startProjectSessionExport: loaded.startProjectSessionExport,
      getExportJobStatus: loaded.getExportJobStatus,
      cancelExport: loaded.cancelExport,
      createAudioPreviewSession: loaded.createAudioPreviewSession,
      playAudioPreview: loaded.playAudioPreview,
      pauseAudioPreview: loaded.pauseAudioPreview,
      stopAudioPreview: loaded.stopAudioPreview,
      seekAudioPreview: loaded.seekAudioPreview,
      cancelAudioPreview: loaded.cancelAudioPreview,
      getAudioPreviewStatus: loaded.getAudioPreviewStatus,
      listAudioOutputDevices: loaded.listAudioOutputDevices,
      selectAudioOutputDevice: loaded.selectAudioOutputDevice,
      getWaveformDisplayPeaks: loaded.getWaveformDisplayPeaks,
      refreshWaveformStatus: loaded.refreshWaveformStatus,
      getArtifactStatus: loaded.getArtifactStatus,
      refreshArtifactStatus: loaded.refreshArtifactStatus,
      retryArtifactGeneration: loaded.retryArtifactGeneration,
      resumeArtifactGeneration: loaded.resumeArtifactGeneration,
      cancelArtifactGeneration: loaded.cancelArtifactGeneration,
      getArtifactQuotaStatus: loaded.getArtifactQuotaStatus,
      runArtifactGarbageCollection: loaded.runArtifactGarbageCollection,
      createRealtimePreviewSession: loaded.createRealtimePreviewSession,
      subscribeRealtimePreviewEvents: loaded.subscribeRealtimePreviewEvents,
      unsubscribeRealtimePreviewEvents: loaded.unsubscribeRealtimePreviewEvents,
      closeRealtimePreviewSession: loaded.closeRealtimePreviewSession,
      attachRealtimePreviewSurface: loaded.attachRealtimePreviewSurface,
      updateRealtimePreviewSurfaceBounds: loaded.updateRealtimePreviewSurfaceBounds,
      detachRealtimePreviewSurface: loaded.detachRealtimePreviewSurface,
      updateRealtimePreviewProjectSessionSnapshot: loaded.updateRealtimePreviewProjectSessionSnapshot,
      seekRealtimePreview: loaded.seekRealtimePreview,
      playRealtimePreview: loaded.playRealtimePreview,
      pauseRealtimePreview: loaded.pauseRealtimePreview,
      stopRealtimePreview: loaded.stopRealtimePreview,
      requestRealtimePreviewFrame: loaded.requestRealtimePreviewFrame,
      nextRealtimePreviewCancellationToken: loaded.nextRealtimePreviewCancellationToken,
      cancelRealtimePreviewRequest: loaded.cancelRealtimePreviewRequest,
      getRealtimePreviewTelemetry: loaded.getRealtimePreviewTelemetry,
      getRealtimePreviewPresentationState: loaded.getRealtimePreviewPresentationState
    };
    cachedLoadError = null;
    return cachedBinding;
  } catch (error) {
    cachedBinding = null;
    cachedLoadError = boundErrorMessage(error);
    return null;
  }
}

function requireLoadedBinding(): NativeBinding {
  const binding = loadNativeBinding();
  if (binding === null) {
    throw new Error(`剪辑核心加载失败：${cachedLoadError ?? "unknown load failure"}`);
  }
  return binding;
}

export function resolveNativeBindingPath(): string {
  const candidates = resolveNativeBindingCandidates();
  const firstExistingCandidate = candidates.find((candidate) => existsSync(candidate));
  if (firstExistingCandidate !== undefined) {
    return firstExistingCandidate;
  }
  return candidates[0] ?? join(__dirname, "../../native/index.cjs");
}

export function resolveNativeBindingCandidates(): string[] {
  if (process.env.VE_NATIVE_BINDING_PATH !== undefined) {
    return [process.env.VE_NATIVE_BINDING_PATH];
  }

  const candidates = [
    join(__dirname, "../../native/index.cjs"),
    join(__dirname, "../native/index.cjs")
  ];

  const resourcesPath = readElectronResourcesPath();
  if (resourcesPath !== null) {
    candidates.unshift(
      join(resourcesPath, "app.asar.unpacked", "native", "index.cjs"),
      join(resourcesPath, "native", "index.cjs")
    );
  }

  return candidates;
}

function bindingLoadError(command: string): CommandResultEnvelope<never> {
  return {
    ok: false,
    data: null,
    error: {
      kind: "internal",
      command,
      message: `剪辑核心加载失败：${cachedLoadError ?? "unknown load failure"}`
    },
    events: []
  };
}

function readElectronResourcesPath(): string | null {
  const resourcesPath = (process as NodeJS.Process & { resourcesPath?: string }).resourcesPath;
  return typeof resourcesPath === "string" && resourcesPath.length > 0 ? resourcesPath : null;
}

function boundErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return message.length > MAX_LOAD_ERROR_LENGTH
    ? `${message.slice(0, MAX_LOAD_ERROR_LENGTH)}...`
    : message;
}

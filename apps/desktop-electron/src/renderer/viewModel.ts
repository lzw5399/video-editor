import type { CSSProperties } from "react";

import type { CommandState, ExportPreset, TimelineSelection } from "../generated/CommandEnvelope";
import type {
  AudioOutputDeviceStatus,
  AudioOutputDeviceSummary,
  AudioPreviewCommandResponse,
  AudioPreviewPlaybackStatus,
  AudioPreviewStatusResponse,
  ArtifactMaintenanceResult,
  ArtifactQuotaStatus,
  ArtifactStatusSummary,
  ArtifactTaskStatus,
  ExportDiagnosticKind,
  ExportJobPhase,
  ExportValidationReport,
  MaterialArtifactStatus,
  MissingMaterialCommandDiagnostic,
  PreviewStatus,
  WaveformDisplayPeaksResponse,
  WaveformDisplayStatus
} from "../generated/CommandResultEnvelope";
import type {
  CanvasAspectRatio,
  CanvasAspectRatioPreset,
  CanvasBackground,
  Draft,
  DraftCanvasConfig,
  KeyframeEasing,
  KeyframeInterpolation,
  KeyframeProperty,
  KeyframeValue,
  Material,
  MaterialKind,
  MaterialStatus,
  Microseconds,
  Segment,
  SegmentVisual,
  Track,
  TrackKind
} from "../generated/Draft";

export type WorkspaceCategory = "媒体" | "音频" | "文字" | "贴纸" | "特效" | "转场" | "字幕" | "滤镜" | "调节" | "模板" | "数字人";

export const WORKSPACE_CATEGORIES: readonly WorkspaceCategory[] = [
  "媒体",
  "音频",
  "文字",
  "贴纸",
  "特效",
  "转场",
  "字幕",
  "滤镜",
  "调节",
  "模板",
  "数字人"
];

export type WorkspaceCategoryMetadata = {
  label: WorkspaceCategory;
  symbol: string;
};

export const WORKSPACE_CATEGORY_META: Record<WorkspaceCategory, WorkspaceCategoryMetadata> = {
  媒体: { label: "媒体", symbol: "▣" },
  音频: { label: "音频", symbol: "♪" },
  文字: { label: "文字", symbol: "T" },
  贴纸: { label: "贴纸", symbol: "◇" },
  特效: { label: "特效", symbol: "✦" },
  转场: { label: "转场", symbol: "⇄" },
  字幕: { label: "字幕", symbol: "CC" },
  滤镜: { label: "滤镜", symbol: "◐" },
  调节: { label: "调节", symbol: "☷" },
  模板: { label: "模板", symbol: "▤" },
  数字人: { label: "数字人", symbol: "人" }
};

export type BindingStatus =
  | { kind: "checking"; label: string }
  | { kind: "ready"; label: string }
  | { kind: "error"; label: string };

export type WorkspaceState = {
  draft: Draft;
  commandState: CommandState;
  selection: TimelineSelection;
  materials: Material[];
  materialDiagnostics: MissingMaterialCommandDiagnostic[];
  resourcePanel: ResourcePanelState;
  preview: PreviewDisplayState;
  audioPreview: AudioPreviewDisplayModel;
  audioDevices: AudioDeviceDisplayModel;
  waveform: WaveformDisplayModel;
  audioParity: AudioParityDisplayModel;
  export: ExportDisplayState;
  runtimeDiagnostics: RuntimeDiagnosticsDisplayState;
  bindingStatus: BindingStatus;
  pendingCommand: string | null;
  pendingAudioCommand: string | null;
  commandError: string | null;
};

export type ResourceStatusTone = "ready" | "active" | "warning" | "error" | "muted";

export type MaterialResourceChipView = {
  key: string;
  label: string;
  statusLabel: string;
  tone: ResourceStatusTone;
  progressPerMille: number | null;
};

export type MaterialResourceStatusView = {
  materialId: string;
  materialLabel: string;
  chips: MaterialResourceChipView[];
};

export type ResourceTaskView = {
  jobId: string;
  label: string;
  statusLabel: string;
  tone: ResourceStatusTone;
  progressPerMille: number | null;
  canCancel: boolean;
  canRetry: boolean;
  canResume: boolean;
};

export type ResourceMaintenanceView = {
  statusLabel: string;
  severity: ResourceStatusTone;
  usedLabel: string;
  reclaimableLabel: string;
  releasedLabel: string;
  cleanupAvailable: boolean;
  resultLabel: string | null;
  errorLabel: string | null;
};

export type ResourcePanelState = {
  sessionId: string;
  statusLabel: string;
  materials: MaterialResourceStatusView[];
  tasks: ResourceTaskView[];
  maintenance: ResourceMaintenanceView;
  refreshAvailable: boolean;
  cleanupConfirming: boolean;
  cleanupRunning: boolean;
  pendingJobId: string | null;
  notice: string | null;
};

export type PreviewDisplayState = {
  frameArtifactPath: string | null;
  frameDisplayUrl: string | null;
  frameStatusLabel: string;
  frameMetadataLabel: string;
  segmentArtifactPath: string | null;
  segmentStatusLabel: string;
  segmentMetadataLabel: string;
  error: string | null;
  lastRequestedPlayhead: Microseconds | null;
  lastRequestedRangeLabel: string | null;
};

export type AudioPreviewDisplayModel = {
  sessionId: string | null;
  generation: number;
  status: AudioPreviewPlaybackStatus;
  statusLabel: string;
  targetTime: Microseconds;
  bufferedUntil: Microseconds;
  deviceStatusLabel: string;
  warningLabel: string | null;
  errorLabel: string | null;
};

export type AudioDeviceDisplayItem = {
  selectionId: string;
  displayName: string;
  status: AudioOutputDeviceStatus;
  statusLabel: string;
  isDefault: boolean;
};

export type AudioDeviceDisplayModel = {
  devices: AudioDeviceDisplayItem[];
  selectedDeviceId: string;
  statusLabel: string;
};

export type WaveformDisplayModel = {
  status: WaveformDisplayStatus;
  statusLabel: string;
  materialId: string | null;
  requestedPeakBins: number;
  returnedPeakBins: number;
  peaks: Array<{ minMillis: number; maxMillis: number }>;
};

export type AudioParityDisplayModel = {
  warningLabel: string | null;
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

export type RealtimePreviewDisplayModel = {
  backend: RealtimePreviewBackendUsed;
  firstFrameLatencyMs: number | null;
  seekLatencyMs: number | null;
  queueLatencyMs: number;
  renderDurationMs: number;
  presentedFrameCount: number;
  droppedFrameCount: number;
  repeatedFrameCount: number;
  staleRejectedCount: number;
  canceledRequestCount: number;
  currentRequestCanceled: boolean;
  fallbackReason: RealtimePreviewFallbackReason | null;
  fallbackCount: number;
  cacheHitCount: number;
  targetTimeMicroseconds: Microseconds;
  playbackGeneration: number;
  fallbackArtifactVisible: boolean;
};

export type ExportDisplayState = {
  outputPath: string;
  preset: ExportPreset;
  jobId: string | null;
  phase: ExportJobPhase | null;
  progressPerMille: number | null;
  outTime: Microseconds | null;
  logSummary: string;
  validation: ExportValidationReport | null;
  diagnosticLabel: string | null;
  error: string | null;
};

export type RuntimeDiagnosticsStatus = "idle" | "checking" | "ready" | "warning" | "error";
export type RuntimeDiagnosticsTone = "ready" | "warning" | "error" | "muted";

export type RuntimeDiagnosticsRow = {
  label: string;
  value: string;
  detail: string;
  tone: RuntimeDiagnosticsTone;
};

export type RuntimeDiagnosticsDisplayState = {
  status: RuntimeDiagnosticsStatus;
  statusLabel: string;
  statusDetail: string;
  packageStatusLabel: string;
  rows: RuntimeDiagnosticsRow[];
  diagnostics: string[];
  canPreview: boolean;
  canExport: boolean;
  checkedAtLabel: string;
};

export type SelectedTrackView = {
  trackId: string;
  name: string;
  kindLabel: string;
  muted: boolean;
  locked: boolean;
};

export type SelectedSegmentView = {
  segment: Segment;
  track: SelectedTrackView;
  material: Material | null;
};

export type TimelineSegmentView = {
  segment: Segment;
  material: Material | null;
  label: string;
  sourceLabel: string;
  targetLabel: string;
  visualKind: TimelineSegmentVisualKind;
  start: Microseconds;
  duration: Microseconds;
  selected: boolean;
};

export type TimelineTrackRow = {
  track: Track;
  symbol: string;
  kindLabel: string;
  statusLabel: string;
  lockLabel: string;
  visibilityLabel: string;
  muteLabel: string;
  rowClassName: string;
  segments: TimelineSegmentView[];
};

export type TimelineView = {
  rows: TimelineTrackRow[];
  duration: Microseconds;
  rulerTicks: Microseconds[];
};

export type TimelineSegmentVisualKind = "video" | "image" | "audio" | "text" | "sticker" | "filter";
export type WorkspaceStartupFixture = "blank" | "demo";

function defaultSegmentVisual(): SegmentVisual {
  return {
    visible: true,
    transform: {
      position: { x: 0, y: 0 },
      scale: { xMillis: 1000, yMillis: 1000 },
      rotation: { degrees: 0 },
      opacity: { valueMillis: 1000 },
      crop: { leftMillis: 0, rightMillis: 0, topMillis: 0, bottomMillis: 0 },
      anchor: { xMillis: 500, yMillis: 500 }
    },
    fitMode: "fit",
    backgroundFilling: { kind: "none" },
    blendMode: { kind: "normal" },
    mask: { kind: "none" }
  };
}

function defaultSegmentAudio() {
  return {
    gainMillis: 1000,
    panBalanceMillis: 0,
    fadeInDuration: { duration: 0 },
    fadeOutDuration: { duration: 0 },
    effectSlots: []
  };
}

export const blankWorkspaceDraft: Draft = {
  schemaVersion: 1,
  draftId: "draft-blank-workspace",
  metadata: {
    name: "未命名草稿",
    description: "空白桌面编辑草稿"
  },
  canvasConfig: {
    aspectRatio: {
      kind: "preset",
      preset: "ratio16x9"
    },
    width: 1920,
    height: 1080,
    frameRate: {
      numerator: 30,
      denominator: 1
    },
    background: {
      kind: "black"
    }
  },
  materials: [],
  tracks: [
    {
      trackId: "track-main-video",
      kind: "video",
      name: "视频轨道 1",
      muted: false,
      locked: false,
      segments: []
    },
    {
      trackId: "track-bgm",
      kind: "audio",
      name: "音频轨道 1",
      muted: false,
      locked: false,
      segments: []
    },
    {
      trackId: "track-title",
      kind: "text",
      name: "文字轨道 1",
      muted: false,
      locked: false,
      segments: []
    }
  ]
};

export const demoWorkspaceDraft: Draft = {
  schemaVersion: 1,
  draftId: "draft-phase-04-workspace",
  metadata: {
    name: "未命名草稿",
    description: "阶段四桌面工作区展示草稿"
  },
  canvasConfig: {
    aspectRatio: {
      kind: "preset",
      preset: "ratio16x9"
    },
    width: 1920,
    height: 1080,
    frameRate: {
      numerator: 30,
      denominator: 1
    },
    background: {
      kind: "black"
    }
  },
  materials: [
    {
      materialId: "material-workspace-video",
      kind: "video",
      uri: "media/workspace-video.mp4",
      displayName: "城市街景.mp4",
      metadata: {
        duration: 12_000_000,
        width: 1920,
        height: 1080,
        frameRate: {
          numerator: 30,
          denominator: 1
        },
        hasVideo: true,
        hasAudio: true,
        audioSampleRate: 48_000,
        audioChannels: 2
      },
      status: "available"
    },
    {
      materialId: "material-workspace-audio",
      kind: "audio",
      uri: "media/bgm.wav",
      displayName: "背景音乐.wav",
      metadata: {
        duration: 18_000_000,
        hasVideo: false,
        hasAudio: true,
        audioSampleRate: 44_100,
        audioChannels: 2
      },
      status: "available"
    },
    {
      materialId: "material-workspace-missing",
      kind: "image",
      uri: "media/missing-cover.png",
      displayName: "封面图.png",
      metadata: {
        duration: 3_000_000,
        width: 1280,
        height: 720,
        hasVideo: true,
        hasAudio: false
      },
      status: "missing"
    },
    {
      materialId: "material-workspace-sticker-failed",
      kind: "sticker",
      uri: "media/sticker.webp",
      displayName: "贴纸素材.webp",
      metadata: {
        hasVideo: true,
        hasAudio: false,
        probeError: "无法读取素材头信息"
      },
      status: "probeFailed"
    },
    {
      materialId: "material-workspace-title",
      kind: "text",
      uri: "text://material-workspace-title",
      displayName: "标题文字",
      metadata: {
        hasVideo: false,
        hasAudio: false
      },
      status: "available"
    }
  ],
  tracks: [
    {
      trackId: "track-main-video",
      kind: "video",
      name: "视频轨道 1",
      muted: false,
      locked: false,
      segments: [
        {
          segmentId: "segment-main-video",
          materialId: "material-workspace-video",
          sourceTimerange: {
            start: 0,
            duration: 8_000_000
          },
          targetTimerange: {
            start: 0,
            duration: 8_000_000
          },
          mainTrackMagnet: {
            enabled: true
          },
          keyframes: [],
          filters: [],
          transition: null,
          volume: {
            levelMillis: 1000
          },
          audio: defaultSegmentAudio(),
          visual: defaultSegmentVisual()
        }
      ]
    },
    {
      trackId: "track-bgm",
      kind: "audio",
      name: "音频轨道 1",
      muted: false,
      locked: false,
      segments: [
        {
          segmentId: "segment-bgm",
          materialId: "material-workspace-audio",
          sourceTimerange: {
            start: 0,
            duration: 8_000_000
          },
          targetTimerange: {
            start: 0,
            duration: 8_000_000
          },
          mainTrackMagnet: {
            enabled: false
          },
          keyframes: [],
          filters: [],
          transition: null,
          volume: {
            levelMillis: 800
          },
          audio: {
            ...defaultSegmentAudio(),
            gainMillis: 800
          },
          visual: defaultSegmentVisual()
        }
      ]
    },
    {
      trackId: "track-title",
      kind: "text",
      name: "文字轨道 1",
      muted: false,
      locked: false,
      segments: []
    }
  ]
};

export function resolveWorkspaceStartupDraft(fixture: WorkspaceStartupFixture = "blank"): Draft {
  return fixture === "demo" ? demoWorkspaceDraft : blankWorkspaceDraft;
}

export const initialCommandState: CommandState = {
  undoStack: [],
  redoStack: [],
  maxHistoryEntries: 50,
  snapping: {
    enabled: true,
    threshold: 120_000
  }
};

export const initialTimelineSelection: TimelineSelection = {
  segmentIds: [],
  trackIds: []
};

export function createInitialWorkspaceState(draft: Draft = blankWorkspaceDraft): WorkspaceState {
  return {
    draft,
    commandState: initialCommandState,
    selection: initialTimelineSelection,
    materials: draft.materials,
    materialDiagnostics: [],
    resourcePanel: createInitialResourcePanelState(),
    preview: {
      frameArtifactPath: null,
      frameDisplayUrl: null,
      frameStatusLabel: "等待请求预览帧",
      frameMetadataLabel: "帧预览尚未生成",
      segmentArtifactPath: null,
      segmentStatusLabel: "等待生成预览片段",
      segmentMetadataLabel: "片段预览尚未生成",
      error: null,
      lastRequestedPlayhead: null,
      lastRequestedRangeLabel: null
    },
    audioPreview: createInitialAudioPreviewDisplayModel(),
    audioDevices: createInitialAudioDeviceDisplayModel(),
    waveform: createInitialWaveformDisplayModel(),
    audioParity: {
      warningLabel: null
    },
    export: {
      outputPath: "/tmp/video-editor-export.mp4",
      preset: "h264AacBalanced",
      jobId: null,
      phase: null,
      progressPerMille: null,
      outTime: null,
      logSummary: "等待开始导出",
      validation: null,
      diagnosticLabel: null,
      error: null
    },
    runtimeDiagnostics: createWaitingRuntimeDiagnosticsState(),
    bindingStatus: {
      kind: "checking",
      label: "正在连接剪辑核心"
    },
    pendingCommand: null,
    pendingAudioCommand: null,
    commandError: null
  };
}

export function createInitialAudioPreviewDisplayModel(): AudioPreviewDisplayModel {
  return {
    sessionId: null,
    generation: 0,
    status: "ready",
    statusLabel: "音频就绪",
    targetTime: 0,
    bufferedUntil: 0,
    deviceStatusLabel: "输出设备就绪",
    warningLabel: null,
    errorLabel: null
  };
}

export function createInitialAudioDeviceDisplayModel(): AudioDeviceDisplayModel {
  return {
    selectedDeviceId: "system-default",
    statusLabel: "输出设备就绪",
    devices: [
      {
        selectionId: "system-default",
        displayName: "系统默认",
        status: "ready",
        statusLabel: "输出设备就绪",
        isDefault: true
      }
    ]
  };
}

export function createInitialWaveformDisplayModel(): WaveformDisplayModel {
  return {
    status: "missing",
    statusLabel: "暂无波形",
    materialId: null,
    requestedPeakBins: 0,
    returnedPeakBins: 0,
    peaks: []
  };
}

export function createInitialResourcePanelState(): ResourcePanelState {
  return {
    sessionId: "desktop-artifact-session",
    statusLabel: "资源待刷新",
    materials: [],
    tasks: [],
    maintenance: {
      statusLabel: "缓存空间正常",
      severity: "ready",
      usedLabel: "待统计",
      reclaimableLabel: "待统计",
      releasedLabel: "0 MB",
      cleanupAvailable: false,
      resultLabel: null,
      errorLabel: null
    },
    refreshAvailable: true,
    cleanupConfirming: false,
    cleanupRunning: false,
    pendingJobId: null,
    notice: null
  };
}

export function resourcePanelFromArtifactStatus(summary: ArtifactStatusSummary): ResourcePanelState {
  return {
    sessionId: summary.sessionId,
    statusLabel: summary.statusLabel,
    materials: materialResourceViews(summary.materials),
    tasks: summary.tasks.map((task) => ({
      jobId: task.jobId,
      label: `${artifactKindLabel(task.artifactKind)} · ${task.displayLabel}`,
      statusLabel: safeArtifactStatusLabel(task.statusLabel, task.status),
      tone: artifactStatusTone(task.status),
      progressPerMille: normalizeProgress(task.progressPerMille),
      canCancel: task.canCancel,
      canRetry: task.canRetry,
      canResume: task.canResume
    })),
    maintenance: maintenanceFromQuota(summary.quota, null, null),
    refreshAvailable: summary.refreshAvailable,
    cleanupConfirming: false,
    cleanupRunning: false,
    pendingJobId: null,
    notice: null
  };
}

export function resourcePanelWithQuota(current: ResourcePanelState, quota: ArtifactQuotaStatus): ResourcePanelState {
  return {
    ...current,
    maintenance: maintenanceFromQuota(quota, current.maintenance.resultLabel, null)
  };
}

export function resourcePanelWithMaintenanceResult(
  current: ResourcePanelState,
  result: ArtifactMaintenanceResult
): ResourcePanelState {
  return {
    ...current,
    cleanupConfirming: false,
    cleanupRunning: false,
    notice: result.completed ? "缓存清理完成" : result.statusLabel,
    maintenance: {
      ...current.maintenance,
      resultLabel: `${result.statusLabel} · 已释放 ${result.releasedLabel}`,
      releasedLabel: result.releasedLabel,
      reclaimableLabel: result.reclaimableLabel,
      errorLabel: null
    }
  };
}

export function resourcePanelWithError(current: ResourcePanelState, message: string): ResourcePanelState {
  return {
    ...current,
    cleanupRunning: false,
    pendingJobId: null,
    maintenance: {
      ...current.maintenance,
      errorLabel: message
    }
  };
}

export function audioPreviewFromCommandResponse(
  current: AudioPreviewDisplayModel,
  response: AudioPreviewCommandResponse
): AudioPreviewDisplayModel {
  return {
    ...current,
    sessionId: response.sessionId,
    generation: response.generation,
    status: response.status,
    statusLabel: safeAudioPlaybackStatusLabel(response.statusLabel, response.status),
    targetTime: response.targetTime,
    warningLabel: audioWarningFromPlaybackStatus(response.status),
    errorLabel: response.status === "failed" ? "音频预览失败：请检查素材是否可用，或重新连接输出设备后重试。" : null
  };
}

export function audioPreviewFromStatusResponse(response: AudioPreviewStatusResponse): AudioPreviewDisplayModel {
  return {
    sessionId: response.sessionId,
    generation: response.generation,
    status: response.status,
    statusLabel: safeAudioPlaybackStatusLabel(response.statusLabel, response.status),
    targetTime: response.targetTime,
    bufferedUntil: response.bufferedUntil,
    deviceStatusLabel: safeAudioDeviceStatusLabel(response.device.statusLabel, response.device.status),
    warningLabel: audioWarningFromPlaybackStatus(response.status),
    errorLabel: response.status === "failed" ? "音频预览失败：请检查素材是否可用，或重新连接输出设备后重试。" : null
  };
}

export function audioDevicesFromSummaries(
  summaries: AudioOutputDeviceSummary[],
  selectedDeviceId: string
): AudioDeviceDisplayModel {
  const devices = summaries.map((device) => ({
    selectionId: device.selectionId,
    displayName: device.isDefault ? "系统默认" : device.displayName,
    status: device.status,
    statusLabel: safeAudioDeviceStatusLabel(device.statusLabel, device.status),
    isDefault: device.isDefault
  }));
  const resolvedDevices = devices.length > 0 ? devices : createInitialAudioDeviceDisplayModel().devices;
  const selected = resolvedDevices.some((device) => device.selectionId === selectedDeviceId)
    ? selectedDeviceId
    : resolvedDevices.find((device) => device.isDefault)?.selectionId ?? resolvedDevices[0].selectionId;
  const selectedDevice = resolvedDevices.find((device) => device.selectionId === selected) ?? resolvedDevices[0];

  return {
    devices: resolvedDevices,
    selectedDeviceId: selected,
    statusLabel: selectedDevice.statusLabel
  };
}

export function waveformDisplayFromResponse(response: WaveformDisplayPeaksResponse): WaveformDisplayModel {
  return {
    status: response.status,
    statusLabel: safeWaveformStatusLabel(response.statusLabel, response.status),
    materialId: response.materialId ?? null,
    requestedPeakBins: Math.max(0, Math.round(response.requestedPeakBins)),
    returnedPeakBins: Math.max(0, Math.round(response.returnedPeakBins)),
    peaks: response.peaks.slice(0, 64).map((peak) => ({
      minMillis: Math.max(-1000, Math.min(1000, Math.round(peak.minMillis))),
      maxMillis: Math.max(-1000, Math.min(1000, Math.round(peak.maxMillis)))
    }))
  };
}

function safeAudioPlaybackStatusLabel(label: string, status: AudioPreviewPlaybackStatus): string {
  const allowed: Record<AudioPreviewPlaybackStatus, string> = {
    ready: "音频就绪",
    playing: "正在播放",
    paused: "已暂停",
    stopped: "已暂停",
    buffering: "音频缓冲中",
    seeking: "正在定位声音",
    canceled: "音频请求已取消",
    staleRejected: "声音已同步到最新播放头",
    unavailable: "音频暂不可用",
    failed: "音频预览失败：请检查素材是否可用，或重新连接输出设备后重试。"
  };

  return Object.values(allowed).includes(label) ? label : allowed[status];
}

function safeAudioDeviceStatusLabel(label: string, status: AudioOutputDeviceStatus): string {
  const allowed: Record<AudioOutputDeviceStatus, string> = {
    ready: "输出设备就绪",
    degraded: "输出设备降级",
    missing: "未找到输出设备",
    unavailable: "音频暂不可用"
  };

  return Object.values(allowed).includes(label) ? label : allowed[status];
}

function safeWaveformStatusLabel(label: string, status: WaveformDisplayStatus): string {
  const allowed: Record<WaveformDisplayStatus, string> = {
    ready: "波形就绪",
    pending: "波形生成中",
    missing: "暂无波形",
    failed: "波形生成失败"
  };

  return Object.values(allowed).includes(label) ? label : allowed[status];
}

function audioWarningFromPlaybackStatus(status: AudioPreviewPlaybackStatus): string | null {
  if (status === "buffering") {
    return "音频缓冲中";
  }
  if (status === "staleRejected") {
    return "声音已同步到最新播放头";
  }
  if (status === "unavailable") {
    return "音频暂不可用";
  }
  return null;
}

export function artifactPreviewStatusLabel(resourcePanel: ResourcePanelState): string | null {
  const previewTasks = resourcePanel.tasks.filter((task) => task.label.startsWith("预览"));

  if (previewTasks.some((task) => task.tone === "active")) {
    return "预览资源生成中";
  }

  if (previewTasks.some((task) => task.tone === "error")) {
    return "生成失败";
  }

  if (previewTasks.some((task) => task.statusLabel === "已取消")) {
    return "已取消";
  }

  if (resourcePanel.materials.some((material) => material.chips.some((chip) => chip.statusLabel === "待刷新"))) {
    return "预览待刷新";
  }

  if (resourcePanel.materials.some((material) => material.chips.length > 0)) {
    return "预览就绪";
  }

  return null;
}

function materialResourceViews(statuses: MaterialArtifactStatus[]): MaterialResourceStatusView[] {
  const grouped = new Map<string, MaterialResourceStatusView>();

  for (const status of statuses) {
    const view =
      grouped.get(status.materialId) ??
      {
        materialId: status.materialId,
        materialLabel: status.materialLabel,
        chips: []
      };
    view.chips.push({
      key: `${status.materialId}-${status.artifactKind}`,
      label: artifactKindLabel(status.artifactKind),
      statusLabel: safeArtifactStatusLabel(status.statusLabel, status.status),
      tone: artifactStatusTone(status.status),
      progressPerMille: normalizeProgress(status.progressPerMille)
    });
    grouped.set(status.materialId, view);
  }

  return Array.from(grouped.values());
}

function maintenanceFromQuota(
  quota: ArtifactQuotaStatus,
  resultLabel: string | null,
  errorLabel: string | null
): ResourceMaintenanceView {
  return {
    statusLabel: quota.statusLabel,
    severity: quota.severity === "warning" ? "warning" : quota.severity === "error" ? "error" : "ready",
    usedLabel: quota.usedLabel,
    reclaimableLabel: quota.reclaimableLabel,
    releasedLabel: quota.releasedLabel,
    cleanupAvailable: quota.cleanupAvailable,
    resultLabel,
    errorLabel
  };
}

function artifactKindLabel(kind: string): string {
  if (kind === "thumbnail") {
    return "缩略图";
  }
  if (kind === "waveform") {
    return "波形";
  }
  if (kind === "proxy") {
    return "代理";
  }
  if (kind === "preview") {
    return "预览";
  }
  return "资源";
}

function safeArtifactStatusLabel(label: string, status: ArtifactTaskStatus): string {
  const allowed: Record<ArtifactTaskStatus, string> = {
    waiting: "等待生成",
    running: "生成中",
    ready: "资源就绪",
    dirty: "待刷新",
    resumable: "可继续",
    cancelRequested: "正在取消",
    cancelled: "已取消",
    failed: "生成失败"
  };

  return Object.values(allowed).includes(label) ? label : allowed[status];
}

function artifactStatusTone(status: ArtifactTaskStatus): ResourceStatusTone {
  if (status === "ready") {
    return "ready";
  }
  if (status === "running" || status === "cancelRequested") {
    return "active";
  }
  if (status === "failed") {
    return "error";
  }
  if (status === "waiting" || status === "dirty" || status === "resumable" || status === "cancelled") {
    return "warning";
  }
  return "muted";
}

function normalizeProgress(value: number | null | undefined): number | null {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return null;
  }

  return Math.max(0, Math.min(1000, Math.round(value)));
}

export function createWaitingRuntimeDiagnosticsState(): RuntimeDiagnosticsDisplayState {
  return {
    status: "idle",
    statusLabel: "等待运行环境检测",
    statusDetail: "打包应用启动后会检测剪辑核心、媒体运行环境、编码器和字幕能力。",
    packageStatusLabel: "打包状态待检测",
    rows: [],
    diagnostics: [],
    canPreview: false,
    canExport: false,
    checkedAtLabel: "尚未检测"
  };
}

export function createCheckingRuntimeDiagnosticsState(): RuntimeDiagnosticsDisplayState {
  return {
    status: "checking",
    statusLabel: "正在检测运行环境",
    statusDetail: "正在检测剪辑核心、编码器、字幕和字体环境。",
    packageStatusLabel: "打包状态检测中",
    rows: [],
    diagnostics: [],
    canPreview: false,
    canExport: false,
    checkedAtLabel: "检测中"
  };
}

export function formatMicroseconds(duration: Microseconds | null | undefined): string {
  if (duration === null || duration === undefined) {
    return "时长未知";
  }

  const totalMilliseconds = Math.max(0, Math.floor(duration / 1000));
  const milliseconds = totalMilliseconds % 1000;
  const totalSeconds = Math.floor(totalMilliseconds / 1000);
  const seconds = totalSeconds % 60;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const minutes = totalMinutes % 60;
  const hours = Math.floor(totalMinutes / 60);

  return `${padTime(hours)}:${padTime(minutes)}:${padTime(seconds)}.${milliseconds.toString().padStart(3, "0")}`;
}

export function formatTimelineTime(time: Microseconds | null | undefined): string {
  return formatMicroseconds(time);
}

export function formatRealtimePreviewBackendLabel(backend: RealtimePreviewBackendUsed): string {
  const labels: Record<RealtimePreviewBackendUsed, string> = {
    mock: "实时后端：Mock",
    gpu: "实时后端：GPU",
    offscreen: "实时后端：离屏",
    previewArtifact: "备用产物：预览缓存",
    ffmpegArtifact: "备用产物：媒体运行环境",
    none: "实时后端：未呈现"
  };

  return labels[backend];
}

export function formatRealtimePreviewFallbackReason(reason: RealtimePreviewFallbackReason): string {
  const labels: Record<RealtimePreviewFallbackReason, string> = {
    noGpuAdapter: "未检测到 GPU 适配器",
    surfaceUnavailable: "预览表面不可用",
    surfaceLost: "预览表面已丢失",
    unsupportedGraphIntent: "当前画面超出实时支持范围",
    frameProviderUnavailable: "素材帧暂不可用",
    textParityUnsupported: "文字实时一致性未通过",
    nativeChildWindowFailed: "原生预览窗口接入失败",
    offscreenReadbackRequired: "需要离屏回读",
    previewArtifactCacheHit: "命中预览缓存",
    ffmpegArtifactGenerated: "已生成媒体备用产物",
    canceled: "请求已取消",
    staleGeneration: "旧一代请求已拒绝"
  };

  return labels[reason];
}

export function formatRealtimePreviewProductFallbackReason(reason: RealtimePreviewFallbackReason): string {
  const labels: Record<RealtimePreviewFallbackReason, string> = {
    noGpuAdapter: "实时预览受限：未检测到可用 GPU",
    surfaceUnavailable: "实时预览受限：预览窗口暂不可用",
    surfaceLost: "实时预览受限：预览窗口已失效",
    unsupportedGraphIntent: "实时预览受限：当前画面超出实时支持范围",
    frameProviderUnavailable: "实时预览受限：素材帧暂不可用",
    textParityUnsupported: "实时预览受限：文字实时一致性未通过",
    nativeChildWindowFailed: "实时预览受限：原生预览窗口接入失败",
    offscreenReadbackRequired: "实时预览受限：需要离屏回读",
    previewArtifactCacheHit: "实时预览受限：当前使用缓存画面",
    ffmpegArtifactGenerated: "实时预览受限：当前画面暂不能实时播放",
    canceled: "实时预览受限：当前请求已取消",
    staleGeneration: "实时预览受限：旧画面请求已拒绝"
  };

  return labels[reason];
}

export function summarizeRealtimePreviewDisplay(model: RealtimePreviewDisplayModel): string {
  const latency = [
    model.firstFrameLatencyMs === null ? "首帧 -" : `首帧 ${model.firstFrameLatencyMs} ms`,
    model.seekLatencyMs === null ? "寻帧 -" : `寻帧 ${model.seekLatencyMs} ms`,
    `排队 ${model.queueLatencyMs} ms`,
    `渲染 ${model.renderDurationMs} ms`
  ];
  const pacing = [
    `已呈现 ${model.presentedFrameCount} 帧`,
    `丢帧 ${model.droppedFrameCount}`,
    `重复 ${model.repeatedFrameCount}`,
    `拒绝旧帧 ${model.staleRejectedCount}`
  ];
  const runtime = [
    `取消 ${model.canceledRequestCount}`,
    `缓存 ${model.cacheHitCount}`,
    `降级 ${model.fallbackCount}`,
    `世代 ${model.playbackGeneration}`
  ];
  const requestState = model.currentRequestCanceled ? ["当前请求已取消"] : [];
  const fallback =
    model.fallbackReason === null ? [] : [`原因 ${formatRealtimePreviewFallbackReason(model.fallbackReason)}`];

  return [
    formatRealtimePreviewBackendLabel(model.backend),
    ...latency,
    ...pacing,
    ...runtime,
    ...requestState,
    ...fallback
  ].join(" · ");
}

export function summarizeRealtimePreviewProductDisplay(model: RealtimePreviewDisplayModel): string {
  if (model.fallbackReason !== null) {
    return formatRealtimePreviewProductFallbackReason(model.fallbackReason);
  }

  if (model.fallbackArtifactVisible || model.backend === "previewArtifact" || model.backend === "ffmpegArtifact") {
    return "实时预览受限：正在使用降级画面";
  }

  return summarizeRealtimePreviewDisplay(model);
}

export function formatMaterialKind(kind: MaterialKind): string {
  const labels: Record<MaterialKind, string> = {
    video: "视频",
    image: "图片",
    audio: "音频",
    text: "文字",
    sticker: "贴纸"
  };

  return labels[kind];
}

export function formatMaterialStatus(status: MaterialStatus): string {
  const labels: Record<MaterialStatus, string> = {
    available: "可用",
    missing: "素材丢失",
    probeFailed: "解析失败"
  };

  return labels[status];
}

export function formatTrackKind(kind: TrackKind): string {
  const labels: Record<TrackKind, string> = {
    video: "视频",
    audio: "音频",
    text: "文字",
    sticker: "贴纸",
    filter: "滤镜"
  };

  return labels[kind];
}

export function formatKeyframeProperty(property: KeyframeProperty): string {
  const labels: Record<KeyframeProperty, string> = {
    visualPositionX: "位置 X",
    visualPositionY: "位置 Y",
    visualScaleX: "缩放 X",
    visualScaleY: "缩放 Y",
    visualRotation: "旋转",
    visualOpacity: "不透明度",
    textFontSize: "字号",
    textColor: "颜色",
    textLineHeight: "行高",
    textLetterSpacing: "字间距",
    textLayoutX: "布局 X",
    textLayoutY: "布局 Y",
    textLayoutWidth: "布局宽",
    textLayoutHeight: "布局高",
    volume: "音量",
    stickerPositionX: "贴纸位置 X",
    stickerPositionY: "贴纸位置 Y",
    stickerScaleX: "贴纸缩放 X",
    stickerScaleY: "贴纸缩放 Y",
    filterParameterUnsupported: "滤镜参数"
  };

  return labels[property];
}

export function formatKeyframeValue(value: KeyframeValue): string {
  if (value.kind === "color") {
    return value.value;
  }

  return String(value.value);
}

export function formatKeyframeInterpolation(interpolation: KeyframeInterpolation): string {
  const labels: Record<KeyframeInterpolation, string> = {
    hold: "保持",
    linear: "线性"
  };

  return labels[interpolation];
}

export function formatKeyframeEasing(easing: KeyframeEasing): string {
  const labels: Record<KeyframeEasing, string> = {
    none: "无",
    easeIn: "缓入",
    easeOut: "缓出",
    easeInOut: "缓入缓出"
  };

  return labels[easing];
}

export function formatMaterialDetail(material: Material): string {
  const { metadata } = material;

  if (metadata.width !== null && metadata.width !== undefined && metadata.height !== null && metadata.height !== undefined) {
    return `${metadata.width} x ${metadata.height}`;
  }

  if (
    metadata.audioSampleRate !== null &&
    metadata.audioSampleRate !== undefined &&
    metadata.audioChannels !== null &&
    metadata.audioChannels !== undefined
  ) {
    return `${metadata.audioSampleRate} Hz / ${metadata.audioChannels} 声道`;
  }

  return metadata.probeError ?? "素材信息待解析";
}

export function formatCommandError(message: string): string {
  return `操作失败：${message}。请检查素材或撤销上一步后重试。`;
}

export function formatCanvasAspectRatio(config: DraftCanvasConfig): string {
  if (config.aspectRatio.kind === "preset") {
    return canvasPresetLabel(config.aspectRatio.preset);
  }

  const [numerator, denominator] = reduceRatio(config.aspectRatio.numerator, config.aspectRatio.denominator);
  return `${numerator}:${denominator}`;
}

export function formatCanvasSize(config: DraftCanvasConfig): string {
  return `${config.width} x ${config.height}`;
}

export function formatCanvasFrameRate(config: DraftCanvasConfig): string {
  const { numerator, denominator } = config.frameRate;
  if (denominator === 1) {
    return `${numerator} fps`;
  }
  return `${numerator}/${denominator} fps`;
}

export function formatCanvasBackground(config: DraftCanvasConfig): string {
  const labels: Record<CanvasBackground["kind"], string> = {
    black: "黑色",
    solidColor: "纯色",
    blurFill: "模糊填充",
    image: "图片背景"
  };
  return labels[config.background.kind];
}

export function formatCanvasBackgroundStatus(config: DraftCanvasConfig): string {
  if (config.background.kind === "blurFill") {
    return "模糊填充 · 降级";
  }
  if (config.background.kind === "image") {
    return "图片背景 · 未接入";
  }
  if (config.background.kind === "solidColor") {
    return `纯色 · ${config.background.color}`;
  }
  return "黑色";
}

export function canvasBackgroundTone(config: DraftCanvasConfig): "ready" | "warning" | "muted" {
  if (config.background.kind === "blurFill" || config.background.kind === "image") {
    return "warning";
  }
  return config.background.kind === "black" ? "muted" : "ready";
}

export function formatCanvasReadout(config: DraftCanvasConfig): string {
  return `画布 ${formatCanvasAspectRatio(config)} · ${formatCanvasSize(config)} · ${formatCanvasFrameRate(config)}`;
}

export function canvasPresetLabel(preset: CanvasAspectRatioPreset): string {
  const labels: Record<CanvasAspectRatioPreset, string> = {
    ratio16x9: "16:9",
    ratio9x16: "9:16",
    ratio1x1: "1:1",
    ratio4x3: "4:3",
    ratio3x4: "3:4"
  };
  return labels[preset];
}

export function canvasAspectRatioFromSize(width: number, height: number): CanvasAspectRatio {
  const [numerator, denominator] = reduceRatio(Math.max(1, Math.round(width)), Math.max(1, Math.round(height)));
  return { kind: "custom", numerator, denominator };
}

export function formatPreviewStatus(status: PreviewStatus): string {
  const labels: Record<PreviewStatus, string> = {
    generated: "已生成",
    cached: "命中缓存",
    invalidated: "已失效"
  };

  return labels[status];
}

export function formatExportPreset(preset: ExportPreset): string {
  const labels: Record<ExportPreset, string> = {
    h264AacDraft: "草稿质量",
    h264AacBalanced: "标准质量"
  };

  return labels[preset];
}

export function formatExportPhase(phase: ExportJobPhase | null | undefined): string {
  if (phase === null || phase === undefined) {
    return "未开始";
  }

  const labels: Record<ExportJobPhase, string> = {
    queued: "排队中",
    running: "导出中",
    validating: "校验中",
    completed: "已完成",
    cancelled: "已取消",
    failed: "导出失败",
    validationFailed: "校验失败"
  };

  return labels[phase];
}

export function formatExportDiagnostic(kind: ExportDiagnosticKind | null | undefined): string | null {
  if (kind === null || kind === undefined) {
    return null;
  }

  const labels: Record<ExportDiagnosticKind, string> = {
    invalidOutputPath: "输出路径无效",
    engineFailed: "剪辑语义失败",
    renderGraphFailed: "渲染图失败",
    compileFailed: "导出编译失败",
    runtimeUnavailable: "运行时不可用",
    runtimeFailed: "运行时失败",
    cancelled: "导出已取消",
    validationFailed: "输出校验失败"
  };

  return labels[kind];
}

export function formatExportProgress(progressPerMille: number | null | undefined): string {
  if (progressPerMille === null || progressPerMille === undefined) {
    return "0%";
  }

  const percent = Math.max(0, Math.min(100, Math.round(progressPerMille / 10)));
  return `${percent}%`;
}

export function materialStatusMessage(material: Material): string | null {
  if (material.status === "missing") {
    return "素材丢失：请重新定位文件后继续编辑。";
  }

  if (material.status === "probeFailed") {
    return "素材解析失败：请检查文件格式或重新导入。";
  }

  return null;
}

export function formatMaterialDiagnostic(diagnostic: MissingMaterialCommandDiagnostic): string {
  return `${diagnostic.materialId}：${diagnostic.message}`;
}

export function getSelectedTrackView(draft: Draft, selection: TimelineSelection): SelectedTrackView | null {
  const selectedTrackId = selection.trackIds[0];
  const selectedSegmentId = selection.segmentIds[0];
  const track =
    draft.tracks.find((candidate) => candidate.trackId === selectedTrackId) ??
    draft.tracks.find((candidate) =>
      candidate.segments.some((segment) => segment.segmentId === selectedSegmentId)
    ) ??
    null;

  if (track === null) {
    return null;
  }

  return {
    trackId: track.trackId,
    name: track.name,
    kindLabel: formatTrackKind(track.kind),
    muted: track.muted,
    locked: track.locked
  };
}

export function getSelectedSegmentView(draft: Draft, selection: TimelineSelection): SelectedSegmentView | null {
  const selectedSegmentId = selection.segmentIds[0];

  if (selectedSegmentId === undefined) {
    return null;
  }

  for (const track of draft.tracks) {
    const segment = track.segments.find((candidate) => candidate.segmentId === selectedSegmentId);

    if (segment !== undefined) {
      const material = draft.materials.find((candidate) => candidate.materialId === segment.materialId) ?? null;
      return {
        segment,
        track: {
          trackId: track.trackId,
          name: track.name,
          kindLabel: formatTrackKind(track.kind),
          muted: track.muted,
          locked: track.locked
        },
        material
      };
    }
  }

  return null;
}

export function deriveTimelineRows(draft: Draft, selection: TimelineSelection): TimelineView {
  const duration = Math.max(
    10_000_000,
    ...draft.tracks.flatMap((track) =>
      track.segments.map((segment) => segment.targetTimerange.start + segment.targetTimerange.duration)
    )
  );
  const rows = draft.tracks.map((track) => {
    const kindLabel = formatTrackKind(track.kind);

    return {
      track,
      symbol: timelineTrackSymbol(track.kind),
      kindLabel,
      statusLabel: `${kindLabel} · ${track.segments.length} 片段`,
      lockLabel: track.locked ? "已锁定" : "未锁定",
      visibilityLabel: track.kind === "audio" ? "听觉开启" : "画面可见",
      muteLabel: track.muted ? "已静音" : "未静音",
      rowClassName: `track-row ${track.kind}`,
      segments: track.segments.map((segment) => {
        const material = draft.materials.find((candidate) => candidate.materialId === segment.materialId) ?? null;
        const selected = selection.segmentIds.includes(segment.segmentId);
        return {
          segment,
          material,
          label: material?.displayName ?? `片段 ${segment.segmentId}`,
          sourceLabel: `源 ${formatTimelineTime(segment.sourceTimerange.start)} / ${formatTimelineTime(
            segment.sourceTimerange.duration
          )}`,
          targetLabel: `目标 ${formatTimelineTime(segment.targetTimerange.start)} / ${formatTimelineTime(
            segment.targetTimerange.duration
          )}`,
          visualKind: timelineSegmentVisualKind(track.kind, material),
          start: segment.targetTimerange.start,
          duration: segment.targetTimerange.duration,
          selected
        };
      })
    };
  });

  return {
    rows,
    duration,
    rulerTicks: buildRulerTicks(duration)
  };
}

function timelineTrackSymbol(kind: TrackKind): string {
  const symbols: Record<TrackKind, string> = {
    video: "▣",
    audio: "♪",
    text: "T",
    sticker: "◇",
    filter: "◐"
  };

  return symbols[kind];
}

function timelineSegmentVisualKind(trackKind: TrackKind, material: Material | null): TimelineSegmentVisualKind {
  if (material?.kind === "video" || material?.kind === "image" || material?.kind === "audio" || material?.kind === "text") {
    return material.kind;
  }

  if (trackKind === "video" || trackKind === "audio" || trackKind === "text" || trackKind === "sticker" || trackKind === "filter") {
    return trackKind;
  }

  return "video";
}

export function segmentBlockStyle(segment: TimelineSegmentView, timelineDuration: Microseconds): CSSProperties {
  const safeDuration = Math.max(1, timelineDuration);
  return {
    left: `${(Math.max(0, segment.start) / safeDuration) * 100}%`,
    width: `${(Math.max(1, segment.duration) / safeDuration) * 100}%`
  };
}

export function findTrackByKind(draft: Draft, kind: TrackKind) {
  return draft.tracks.find((track) => track.kind === kind) ?? null;
}

export function findFirstMaterialByKind(draft: Draft, kind: MaterialKind) {
  return draft.materials.find((material) => material.kind === kind && material.status === "available") ?? null;
}

export function nextTrackStart(track: { segments: Segment[] }): Microseconds {
  return track.segments.reduce(
    (latest, segment) => Math.max(latest, segment.targetTimerange.start + segment.targetTimerange.duration),
    0
  );
}

function padTime(value: number): string {
  return value.toString().padStart(2, "0");
}

function reduceRatio(numerator: number, denominator: number): [number, number] {
  const safeNumerator = Math.max(1, Math.round(numerator));
  const safeDenominator = Math.max(1, Math.round(denominator));
  const divisor = greatestCommonDivisor(safeNumerator, safeDenominator);
  return [safeNumerator / divisor, safeDenominator / divisor];
}

function greatestCommonDivisor(left: number, right: number): number {
  let a = Math.abs(Math.round(left));
  let b = Math.abs(Math.round(right));
  while (b !== 0) {
    const remainder = a % b;
    a = b;
    b = remainder;
  }
  return Math.max(1, a);
}

function buildRulerTicks(duration: Microseconds): Microseconds[] {
  const tickCount = 5;
  const lastTickIndex = tickCount - 1;

  return Array.from({ length: tickCount }, (_value, index) => Math.round((duration * index) / lastTickIndex));
}

import type { CSSProperties } from "react";

import type { CommandState, ExportPreset, TimelineSelection } from "../generated/CommandEnvelope";
import type {
  ExportDiagnosticKind,
  ExportJobPhase,
  ExportValidationReport,
  MissingMaterialCommandDiagnostic,
  PreviewStatus
} from "../generated/CommandResultEnvelope";
import type {
  CanvasAspectRatio,
  CanvasAspectRatioPreset,
  CanvasBackground,
  Draft,
  DraftCanvasConfig,
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
  preview: PreviewDisplayState;
  export: ExportDisplayState;
  runtimeDiagnostics: RuntimeDiagnosticsDisplayState;
  bindingStatus: BindingStatus;
  pendingCommand: string | null;
  commandError: string | null;
};

export type PreviewDisplayState = {
  frameArtifactPath: string | null;
  frameStatusLabel: string;
  frameMetadataLabel: string;
  segmentArtifactPath: string | null;
  segmentStatusLabel: string;
  segmentMetadataLabel: string;
  error: string | null;
  lastRequestedPlayhead: Microseconds | null;
  lastRequestedRangeLabel: string | null;
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
    fitMode: "stretch",
    backgroundFilling: { kind: "none" },
    blendMode: { kind: "normal" },
    mask: { kind: "none" }
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
    preview: {
      frameArtifactPath: null,
      frameStatusLabel: "等待请求预览帧",
      frameMetadataLabel: "帧预览尚未生成",
      segmentArtifactPath: null,
      segmentStatusLabel: "等待生成预览片段",
      segmentMetadataLabel: "片段预览尚未生成",
      error: null,
      lastRequestedPlayhead: null,
      lastRequestedRangeLabel: null
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
    commandError: null
  };
}

export function createWaitingRuntimeDiagnosticsState(): RuntimeDiagnosticsDisplayState {
  return {
    status: "idle",
    statusLabel: "等待运行环境检测",
    statusDetail: "打包应用启动后会检测剪辑核心、FFmpeg、编码器和字幕能力。",
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

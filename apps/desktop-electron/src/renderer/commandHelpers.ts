import type {
  AddSegmentCommandPayload,
  AddAudioSegmentCommandPayload,
  AddTextSegmentCommandPayload,
  CancelExportCommandPayload,
  CommandEnvelope,
  CommandState,
  DeleteSegmentCommandPayload,
  EditTextSegmentCommandPayload,
  ExportPreset,
  GetExportJobStatusCommandPayload,
  ImportMaterialCommandPayload,
  InvalidatePreviewCacheCommandPayload,
  ListMissingMaterialsCommandPayload,
  MoveSegmentCommandPayload,
  ProbeRuntimeCapabilitiesCommandPayload,
  PreviewCacheEntryRef,
  RedoTimelineEditCommandPayload,
  RequestPreviewFrameCommandPayload,
  RequestPreviewSegmentCommandPayload,
  SelectTimelineSegmentsCommandPayload,
  SplitSegmentCommandPayload,
  StartExportCommandPayload,
  TimelineSelection,
  TrimSegmentCommandPayload,
  TrackId,
  UndoTimelineEditCommandPayload
} from "../generated/CommandEnvelope";
import type {
  CommandResultEnvelope,
  RuntimeBinaryCapability,
  RuntimeCapabilityReport,
  RuntimeFeatureCapability,
  RuntimeFontCapability,
  TimelineCommandResponse
} from "../generated/CommandResultEnvelope";
import type {
  Draft,
  MaterialId,
  MaterialKind,
  Microseconds,
  SegmentId,
  SegmentVolume,
  SourceTimerange,
  TargetTimerange,
  TextSegment
} from "../generated/Draft";
import type {
  RuntimeDiagnosticsDisplayState,
  RuntimeDiagnosticsRow,
  RuntimeDiagnosticsTone
} from "./viewModel";

export type CommandContext = {
  draft: Draft;
  commandState: CommandState;
  selection: TimelineSelection;
};

type ImportMaterialOptions = {
  draft: Draft;
  bundlePath: string;
  materialPath: string;
  materialId?: MaterialId | null;
  displayName?: string | null;
  materialKindHint?: MaterialKind | null;
};

export function buildImportMaterialCommand(options: ImportMaterialOptions): CommandEnvelope {
  const payload = {
    kind: "importMaterial",
    draft: options.draft,
    bundlePath: options.bundlePath,
    materialPath: options.materialPath,
    materialId: options.materialId ?? null,
    displayName: options.displayName ?? null,
    materialKindHint: options.materialKindHint ?? null
  } satisfies ImportMaterialCommandPayload & { kind: "importMaterial" };

  return envelope("importMaterial", payload);
}

export function buildListMaterialsCommand(draft: Draft): CommandEnvelope {
  return envelope("listMaterials", {
    kind: "listMaterials",
    draft
  });
}

export function buildListMissingMaterialsCommand(draft: Draft, bundlePath: string): CommandEnvelope {
  const payload = {
    kind: "listMissingMaterials",
    draft,
    bundlePath
  } satisfies ListMissingMaterialsCommandPayload & { kind: "listMissingMaterials" };

  return envelope("listMissingMaterials", payload);
}

export function buildProbeRuntimeCapabilitiesCommand(): CommandEnvelope {
  const payload = {
    kind: "probeRuntimeCapabilities"
  } satisfies ProbeRuntimeCapabilitiesCommandPayload & { kind: "probeRuntimeCapabilities" };

  return envelope("probeRuntimeCapabilities", payload);
}

type AddSegmentOptions = {
  context: CommandContext;
  trackId: TrackId;
  segmentId: SegmentId;
  materialId: MaterialId;
  sourceTimerange: SourceTimerange;
  targetTimerange: TargetTimerange;
};

export function buildAddSegmentCommand(options: AddSegmentOptions): CommandEnvelope {
  const payload = {
    kind: "addSegment",
    draft: options.context.draft,
    commandState: options.context.commandState,
    selection: options.context.selection,
    trackId: options.trackId,
    segmentId: options.segmentId,
    materialId: options.materialId,
    sourceTimerange: options.sourceTimerange,
    targetTimerange: options.targetTimerange
  } satisfies AddSegmentCommandPayload & { kind: "addSegment" };

  return envelope("addSegment", payload);
}

export function buildSelectTimelineSegmentsCommand(
  context: CommandContext,
  segmentIds: SegmentId[],
  trackIds: TrackId[]
): CommandEnvelope {
  const payload = {
    kind: "selectTimelineSegments",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentIds,
    trackIds
  } satisfies SelectTimelineSegmentsCommandPayload & { kind: "selectTimelineSegments" };

  return envelope("selectTimelineSegments", payload);
}

export function buildMoveSegmentCommand(
  context: CommandContext,
  segmentId: SegmentId,
  targetTrackId: TrackId,
  targetStart: Microseconds
): CommandEnvelope {
  const payload = {
    kind: "moveSegment",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    targetTrackId,
    targetStart
  } satisfies MoveSegmentCommandPayload & { kind: "moveSegment" };

  return envelope("moveSegment", payload);
}

export function buildSplitSegmentCommand(
  context: CommandContext,
  segmentId: SegmentId,
  rightSegmentId: SegmentId,
  splitAt: Microseconds
): CommandEnvelope {
  const payload = {
    kind: "splitSegment",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    rightSegmentId,
    splitAt
  } satisfies SplitSegmentCommandPayload & { kind: "splitSegment" };

  return envelope("splitSegment", payload);
}

export function buildTrimSegmentCommand(
  context: CommandContext,
  segmentId: SegmentId,
  direction: "left" | "right",
  targetTimerange: TargetTimerange
): CommandEnvelope {
  const payload = {
    kind: "trimSegment",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    direction,
    targetTimerange
  } satisfies TrimSegmentCommandPayload & { kind: "trimSegment" };

  return envelope("trimSegment", payload);
}

export function buildDeleteSegmentCommand(context: CommandContext, segmentId: SegmentId): CommandEnvelope {
  const payload = {
    kind: "deleteSegment",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId
  } satisfies DeleteSegmentCommandPayload & { kind: "deleteSegment" };

  return envelope("deleteSegment", payload);
}

export function buildUndoTimelineEditCommand(context: CommandContext): CommandEnvelope {
  const payload = {
    kind: "undoTimelineEdit",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection
  } satisfies UndoTimelineEditCommandPayload & { kind: "undoTimelineEdit" };

  return envelope("undoTimelineEdit", payload);
}

export function buildRedoTimelineEditCommand(context: CommandContext): CommandEnvelope {
  const payload = {
    kind: "redoTimelineEdit",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection
  } satisfies RedoTimelineEditCommandPayload & { kind: "redoTimelineEdit" };

  return envelope("redoTimelineEdit", payload);
}

type TextCommandOptions = {
  context: CommandContext;
  trackId: TrackId;
  segmentId: SegmentId;
  materialId: MaterialId;
  sourceTimerange: SourceTimerange;
  targetTimerange: TargetTimerange;
  text: TextSegment;
};

export function buildAddTextSegmentCommand(options: TextCommandOptions): CommandEnvelope {
  const payload = {
    kind: "addTextSegment",
    draft: options.context.draft,
    commandState: options.context.commandState,
    selection: options.context.selection,
    trackId: options.trackId,
    segmentId: options.segmentId,
    materialId: options.materialId,
    sourceTimerange: options.sourceTimerange,
    targetTimerange: options.targetTimerange,
    text: options.text
  } satisfies AddTextSegmentCommandPayload & { kind: "addTextSegment" };

  return envelope("addTextSegment", payload);
}

export function buildEditTextSegmentCommand(
  context: CommandContext,
  segmentId: SegmentId,
  text: TextSegment
): CommandEnvelope {
  const payload = {
    kind: "editTextSegment",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    text
  } satisfies EditTextSegmentCommandPayload & { kind: "editTextSegment" };

  return envelope("editTextSegment", payload);
}

type AudioCommandOptions = {
  context: CommandContext;
  trackId: TrackId;
  segmentId: SegmentId;
  materialId: MaterialId;
  sourceTimerange: SourceTimerange;
  targetTimerange: TargetTimerange;
};

export function buildAddAudioSegmentCommand(options: AudioCommandOptions): CommandEnvelope {
  const payload = {
    kind: "addAudioSegment",
    draft: options.context.draft,
    commandState: options.context.commandState,
    selection: options.context.selection,
    trackId: options.trackId,
    segmentId: options.segmentId,
    materialId: options.materialId,
    sourceTimerange: options.sourceTimerange,
    targetTimerange: options.targetTimerange
  } satisfies AddAudioSegmentCommandPayload & { kind: "addAudioSegment" };

  return envelope("addAudioSegment", payload);
}

export function buildSetSegmentVolumeCommand(
  context: CommandContext,
  segmentId: SegmentId,
  volume: SegmentVolume
): CommandEnvelope {
  return envelope("setSegmentVolume", {
    kind: "setSegmentVolume",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    volume
  });
}

export function buildSetTrackMuteCommand(context: CommandContext, trackId: TrackId, muted: boolean): CommandEnvelope {
  return envelope("setTrackMute", {
    kind: "setTrackMute",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    trackId,
    muted
  });
}

type RequestPreviewFrameOptions = {
  draft: Draft;
  cacheRoot: string;
  targetTime: Microseconds;
};

export function buildRequestPreviewFrameCommand(options: RequestPreviewFrameOptions): CommandEnvelope {
  const payload = {
    kind: "requestPreviewFrame",
    draft: options.draft,
    cacheRoot: options.cacheRoot,
    targetTime: options.targetTime
  } satisfies RequestPreviewFrameCommandPayload & { kind: "requestPreviewFrame" };

  return envelope("requestPreviewFrame", payload);
}

type RequestPreviewSegmentOptions = {
  draft: Draft;
  cacheRoot: string;
  targetTimerange: TargetTimerange;
};

export function buildRequestPreviewSegmentCommand(options: RequestPreviewSegmentOptions): CommandEnvelope {
  const payload = {
    kind: "requestPreviewSegment",
    draft: options.draft,
    cacheRoot: options.cacheRoot,
    targetTimerange: options.targetTimerange
  } satisfies RequestPreviewSegmentCommandPayload & { kind: "requestPreviewSegment" };

  return envelope("requestPreviewSegment", payload);
}

type InvalidatePreviewCacheOptions = {
  entries: PreviewCacheEntryRef[];
  changedRanges: TargetTimerange[];
  changedMaterialIds: MaterialId[];
  reason: string;
};

export function buildInvalidatePreviewCacheCommand(options: InvalidatePreviewCacheOptions): CommandEnvelope {
  const payload = {
    kind: "invalidatePreviewCache",
    entries: options.entries,
    changedRanges: options.changedRanges,
    changedMaterialIds: options.changedMaterialIds,
    reason: options.reason
  } satisfies InvalidatePreviewCacheCommandPayload & { kind: "invalidatePreviewCache" };

  return envelope("invalidatePreviewCache", payload);
}

type StartExportOptions = {
  draft: Draft;
  outputPath: string;
  preset: ExportPreset;
};

export function buildStartExportCommand(options: StartExportOptions): CommandEnvelope {
  const payload = {
    kind: "startExport",
    draft: options.draft,
    outputPath: options.outputPath,
    preset: options.preset
  } satisfies StartExportCommandPayload & { kind: "startExport" };

  return envelope("startExport", payload);
}

export function buildGetExportJobStatusCommand(jobId: string): CommandEnvelope {
  const payload = {
    kind: "getExportJobStatus",
    jobId
  } satisfies GetExportJobStatusCommandPayload & { kind: "getExportJobStatus" };

  return envelope("getExportJobStatus", payload);
}

export function buildCancelExportCommand(jobId: string): CommandEnvelope {
  const payload = {
    kind: "cancelExport",
    jobId
  } satisfies CancelExportCommandPayload & { kind: "cancelExport" };

  return envelope("cancelExport", payload);
}

export function applyTimelineCommandResult(
  current: CommandContext,
  result: CommandResultEnvelope<TimelineCommandResponse>
): { state: CommandContext; errorMessage: string | null } {
  if (!result.ok || result.data === null) {
    return {
      state: current,
      errorMessage: commandErrorMessage(result)
    };
  }

  return {
    state: {
      draft: result.data.draft,
      commandState: result.data.commandState,
      selection: result.data.selection
    },
    errorMessage: null
  };
}

export function commandErrorMessage(resultOrMessage: CommandResultEnvelope<unknown> | string): string {
  const message =
    typeof resultOrMessage === "string"
      ? resultOrMessage
      : resultOrMessage.error?.message ?? "剪辑核心返回未知错误";

  return `操作失败：${message}。请检查素材或撤销上一步后重试。`;
}

export function runtimeDiagnosticsFromReport(report: RuntimeCapabilityReport): RuntimeDiagnosticsDisplayState {
  const encoderReady = report.h264Encoder.available && report.aacEncoder.available;
  const subtitleReady = report.assFilter.available && report.subtitlesFilter.available;
  const hasBlockingRuntime = report.status === "unavailable";
  const canPreview = !hasBlockingRuntime && report.ffmpeg.status !== "unavailable" && report.ffprobe.status !== "unavailable";
  const canExport = canPreview && encoderReady;
  const status = report.status === "ready" ? "ready" : report.status === "unavailable" ? "error" : "warning";
  const diagnostics = [
    ...report.diagnostics,
    report.licensePosture.message
  ].filter((message, index, all) => message.length > 0 && all.indexOf(message) === index);

  return {
    status,
    statusLabel:
      status === "ready"
        ? "运行环境就绪"
        : status === "error"
          ? "运行环境检测失败，请检查 FFmpeg/ffprobe 路径后重试。"
          : "部分能力不可用，可继续编辑，但预览或导出可能受限。",
    statusDetail:
      status === "ready"
        ? "预览和导出能力已通过剪辑核心检测。"
        : status === "error"
          ? "运行环境检测失败，请检查 FFmpeg/ffprobe 路径后重试。"
        : "部分能力不可用，可继续编辑，但预览或导出可能受限。",
    packageStatusLabel: report.licensePosture.externalRuntime ? "外部运行环境" : "打包应用已就绪",
    rows: [
      binaryRow("FFmpeg 状态", report.ffmpeg),
      binaryRow("ffprobe 状态", report.ffprobe),
      featurePairRow("编码能力", report.h264Encoder, report.aacEncoder),
      featurePairRow("字幕能力", report.assFilter, report.subtitlesFilter),
      fontRow("字体环境", report.fontReadiness),
      {
        label: "打包状态",
        value: report.licensePosture.redistributableBuild ? "可再发行构建" : "本机外部运行环境",
        detail: report.licensePosture.message,
        tone: report.licensePosture.redistributableBuild ? "ready" : "warning"
      }
    ],
    diagnostics,
    canPreview,
    canExport,
    checkedAtLabel: "刚刚检测"
  };
}

export function runtimeDiagnosticsFromError(message: string): RuntimeDiagnosticsDisplayState {
  return {
    status: "error",
    statusLabel: "运行环境检测失败，请检查 FFmpeg/ffprobe 路径后重试。",
    statusDetail: message,
    packageStatusLabel: "运行环境不可用",
    rows: [
      {
        label: "FFmpeg 状态",
        value: message.includes("FFmpeg") ? "未找到" : "待检测",
        detail: message,
        tone: "error"
      },
      {
        label: "ffprobe 状态",
        value: message.includes("ffprobe") ? "未找到" : "待检测",
        detail: message,
        tone: "error"
      }
    ],
    diagnostics: [message],
    canPreview: false,
    canExport: false,
    checkedAtLabel: "检测失败"
  };
}

function binaryRow(label: string, capability: RuntimeBinaryCapability): RuntimeDiagnosticsRow {
  return {
    label,
    value: statusValue(capability.status),
    detail: [capability.path, capability.version, capability.source, capability.configureSummary, capability.diagnostic]
      .filter((value): value is string => value !== null && value !== undefined && value.length > 0)
      .join(" · "),
    tone: statusTone(capability.status)
  };
}

function featurePairRow(
  label: string,
  first: RuntimeFeatureCapability,
  second: RuntimeFeatureCapability
): RuntimeDiagnosticsRow {
  const ready = first.available && second.available;
  const detail = [featureDetail(first), featureDetail(second)].join(" · ");

  return {
    label,
    value: ready ? "可用" : "能力受限",
    detail,
    tone: ready ? "ready" : "warning"
  };
}

function fontRow(label: string, capability: RuntimeFontCapability): RuntimeDiagnosticsRow {
  return {
    label,
    value: statusValue(capability.status),
    detail:
      capability.availableFontPaths.length > 0
        ? capability.availableFontPaths.join(" · ")
        : capability.diagnostic ?? "字体环境未完全就绪，文字渲染可能与导出结果不一致。",
    tone: statusTone(capability.status)
  };
}

function featureDetail(feature: RuntimeFeatureCapability): string {
  return `${feature.name} ${feature.available ? "可用" : "不可用"}`;
}

function statusValue(status: RuntimeBinaryCapability["status"]): string {
  const labels: Record<RuntimeBinaryCapability["status"], string> = {
    ready: "可用",
    warning: "不可用",
    unavailable: "未找到"
  };

  return labels[status];
}

function statusTone(status: RuntimeBinaryCapability["status"]): RuntimeDiagnosticsTone {
  const tones: Record<RuntimeBinaryCapability["status"], RuntimeDiagnosticsTone> = {
    ready: "ready",
    warning: "warning",
    unavailable: "error"
  };

  return tones[status];
}

function envelope(command: CommandEnvelope["command"], payload: CommandEnvelope["payload"]): CommandEnvelope {
  return {
    command,
    payload,
    requestId: `${command}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`
  };
}

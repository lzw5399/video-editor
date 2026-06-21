import type {
  AudioPreviewCommandPayload,
  CancelExportCommandPayload,
  CommandEnvelope,
  CommandState,
  DirtyRange,
  ExportPreset,
  ArtifactGenerationActionCommandPayload,
  GetArtifactQuotaStatusCommandPayload,
  GetArtifactStatusCommandPayload,
  GetExportJobStatusCommandPayload,
  ImportMaterialCommandPayload,
  InvalidatePreviewCacheCommandPayload,
  ListMissingMaterialsCommandPayload,
  OpenProjectBundleCommandPayload,
  ProbeRuntimeCapabilitiesCommandPayload,
  PreviewCacheEntryRef,
  RequestPreviewFrameCommandPayload,
  RequestPreviewSegmentCommandPayload,
  RefreshArtifactStatusCommandPayload,
  SaveProjectBundleCommandPayload,
  StartExportCommandPayload,
  RunArtifactGarbageCollectionCommandPayload,
  TimelineSelection
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
  TargetTimerange
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

export function buildOpenProjectBundleCommand(bundlePath: string): CommandEnvelope {
  const payload = {
    kind: "openProjectBundle",
    bundlePath
  } satisfies OpenProjectBundleCommandPayload & { kind: "openProjectBundle" };

  return envelope("openProjectBundle", payload);
}

export function buildSaveProjectBundleCommand(draft: Draft, bundlePath: string): CommandEnvelope {
  const payload = {
    kind: "saveProjectBundle",
    draft,
    bundlePath
  } satisfies SaveProjectBundleCommandPayload & { kind: "saveProjectBundle" };

  return envelope("saveProjectBundle", payload);
}

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

type RequestPreviewFrameOptions = {
  draft: Draft;
  cacheRoot?: string;
  bundlePath?: string;
  targetTime: Microseconds;
};

export function buildRequestPreviewFrameCommand(options: RequestPreviewFrameOptions): CommandEnvelope {
  const payload = {
    kind: "requestPreviewFrame",
    draft: options.draft,
    ...(options.cacheRoot === undefined ? {} : { cacheRoot: options.cacheRoot }),
    ...(options.bundlePath === undefined ? {} : { bundlePath: options.bundlePath }),
    targetTime: options.targetTime
  } satisfies RequestPreviewFrameCommandPayload & { kind: "requestPreviewFrame" };

  return envelope("requestPreviewFrame", payload);
}

type RequestPreviewSegmentOptions = {
  draft: Draft;
  cacheRoot?: string;
  bundlePath?: string;
  targetTimerange: TargetTimerange;
};

export function buildRequestPreviewSegmentCommand(options: RequestPreviewSegmentOptions): CommandEnvelope {
  const payload = {
    kind: "requestPreviewSegment",
    draft: options.draft,
    ...(options.cacheRoot === undefined ? {} : { cacheRoot: options.cacheRoot }),
    ...(options.bundlePath === undefined ? {} : { bundlePath: options.bundlePath }),
    targetTimerange: options.targetTimerange
  } satisfies RequestPreviewSegmentCommandPayload & { kind: "requestPreviewSegment" };

  return envelope("requestPreviewSegment", payload);
}

type InvalidatePreviewCacheOptions = {
  entries: PreviewCacheEntryRef[];
  changedRanges: DirtyRange[];
  changedMaterialIds: MaterialId[];
  changedGraphNodeIds?: string[];
  changedDomains?: InvalidatePreviewCacheCommandPayload["changedDomains"];
  runtimeCapabilityFingerprint?: string | null;
  outputProfileFingerprint?: string | null;
  fullDraft?: boolean;
  reason: string;
  artifactSchemaVersion?: number;
  generatorVersion?: string;
};

export function buildInvalidatePreviewCacheCommand(options: InvalidatePreviewCacheOptions): CommandEnvelope {
  const payload = {
    kind: "invalidatePreviewCache",
    entries: options.entries,
    changedRanges: options.changedRanges,
    changedMaterialIds: options.changedMaterialIds,
    changedGraphNodeIds: options.changedGraphNodeIds,
    changedDomains: options.changedDomains,
    runtimeCapabilityFingerprint: options.runtimeCapabilityFingerprint,
    outputProfileFingerprint: options.outputProfileFingerprint,
    fullDraft: options.fullDraft,
    reason: options.reason,
    artifactSchemaVersion: options.artifactSchemaVersion,
    generatorVersion: options.generatorVersion
  } satisfies InvalidatePreviewCacheCommandPayload & { kind: "invalidatePreviewCache" };

  return envelope("invalidatePreviewCache", payload);
}

type AudioPreviewCommandKind =
  | "createAudioPreviewSession"
  | "playAudioPreview"
  | "pauseAudioPreview"
  | "stopAudioPreview"
  | "seekAudioPreview"
  | "cancelAudioPreview"
  | "getAudioPreviewStatus"
  | "listAudioOutputDevices"
  | "selectAudioOutputDevice"
  | "getWaveformDisplayPeaks"
  | "refreshWaveformStatus";

type AudioPreviewCommandOptions = {
  draft?: Draft | null;
  sessionId?: string | null;
  materialId?: MaterialId | null;
  targetTime?: Microseconds | null;
  targetTimerange?: TargetTimerange | null;
  playbackGeneration?: number | null;
  deviceSelectionId?: string | null;
  maxPeakBins?: number | null;
};

export function buildCreateAudioPreviewSessionCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("createAudioPreviewSession", options);
}

export function buildPlayAudioPreviewCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("playAudioPreview", options);
}

export function buildPauseAudioPreviewCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("pauseAudioPreview", options);
}

export function buildStopAudioPreviewCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("stopAudioPreview", options);
}

export function buildSeekAudioPreviewCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("seekAudioPreview", options);
}

export function buildCancelAudioPreviewCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("cancelAudioPreview", options);
}

export function buildGetAudioPreviewStatusCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("getAudioPreviewStatus", options);
}

export function buildListAudioOutputDevicesCommand(options: AudioPreviewCommandOptions = {}): CommandEnvelope {
  return buildAudioPreviewCommand("listAudioOutputDevices", options);
}

export function buildSelectAudioOutputDeviceCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("selectAudioOutputDevice", options);
}

export function buildGetWaveformDisplayPeaksCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("getWaveformDisplayPeaks", options);
}

export function buildRefreshWaveformStatusCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("refreshWaveformStatus", options);
}

function buildAudioPreviewCommand(kind: AudioPreviewCommandKind, options: AudioPreviewCommandOptions): CommandEnvelope {
  const payload = {
    kind,
    draft: options.draft ?? null,
    sessionId: options.sessionId ?? null,
    materialId: options.materialId ?? null,
    targetTime: options.targetTime ?? null,
    targetTimerange: options.targetTimerange ?? null,
    playbackGeneration: options.playbackGeneration ?? null,
    deviceSelectionId: options.deviceSelectionId ?? null,
    maxPeakBins: options.maxPeakBins ?? null
  } satisfies AudioPreviewCommandPayload & { kind: AudioPreviewCommandKind };

  return envelope(kind, payload);
}

type ArtifactStatusCommandOptions = {
  sessionId: string;
  bundlePath: string;
  materialId?: MaterialId | null;
};

export function buildGetArtifactStatusCommand(options: ArtifactStatusCommandOptions): CommandEnvelope {
  const payload = {
    kind: "getArtifactStatus",
    sessionId: options.sessionId,
    bundlePath: options.bundlePath,
    materialId: options.materialId ?? null
  } satisfies GetArtifactStatusCommandPayload & { kind: "getArtifactStatus" };

  return envelope("getArtifactStatus", payload);
}

export function buildRefreshArtifactStatusCommand(options: ArtifactStatusCommandOptions): CommandEnvelope {
  const payload = {
    kind: "refreshArtifactStatus",
    sessionId: options.sessionId,
    bundlePath: options.bundlePath,
    materialId: options.materialId ?? null
  } satisfies RefreshArtifactStatusCommandPayload & { kind: "refreshArtifactStatus" };

  return envelope("refreshArtifactStatus", payload);
}

type ArtifactGenerationActionOptions = {
  sessionId: string;
  bundlePath: string;
  jobId: string;
};

export function buildRetryArtifactGenerationCommand(options: ArtifactGenerationActionOptions): CommandEnvelope {
  return buildArtifactGenerationActionCommand("retryArtifactGeneration", options);
}

export function buildResumeArtifactGenerationCommand(options: ArtifactGenerationActionOptions): CommandEnvelope {
  return buildArtifactGenerationActionCommand("resumeArtifactGeneration", options);
}

export function buildCancelArtifactGenerationCommand(options: ArtifactGenerationActionOptions): CommandEnvelope {
  return buildArtifactGenerationActionCommand("cancelArtifactGeneration", options);
}

function buildArtifactGenerationActionCommand(
  kind: "retryArtifactGeneration" | "resumeArtifactGeneration" | "cancelArtifactGeneration",
  options: ArtifactGenerationActionOptions
): CommandEnvelope {
  const payload = {
    kind,
    sessionId: options.sessionId,
    bundlePath: options.bundlePath,
    jobId: options.jobId
  } satisfies ArtifactGenerationActionCommandPayload & { kind: typeof kind };

  return envelope(kind, payload);
}

export function buildGetArtifactQuotaStatusCommand(sessionId: string, bundlePath: string): CommandEnvelope {
  const payload = {
    kind: "getArtifactQuotaStatus",
    sessionId,
    bundlePath
  } satisfies GetArtifactQuotaStatusCommandPayload & { kind: "getArtifactQuotaStatus" };

  return envelope("getArtifactQuotaStatus", payload);
}

export function buildRunArtifactGarbageCollectionCommand(
  sessionId: string,
  bundlePath: string,
  dryRun: boolean
): CommandEnvelope {
  const payload = {
    kind: "runArtifactGarbageCollection",
    sessionId,
    bundlePath,
    dryRun
  } satisfies RunArtifactGarbageCollectionCommandPayload & { kind: "runArtifactGarbageCollection" };

  return envelope("runArtifactGarbageCollection", payload);
}

type StartExportOptions = {
  draft: Draft;
  outputPath: string;
  preset: ExportPreset;
  dirtyFacts?: StartExportCommandPayload["dirtyFacts"];
};

export function buildStartExportCommand(options: StartExportOptions): CommandEnvelope {
  const payload = {
    kind: "startExport",
    draft: options.draft,
    outputPath: options.outputPath,
    preset: options.preset,
    dirtyFacts: options.dirtyFacts
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
          ? "运行环境检测失败，请检查媒体运行环境后重试。"
          : "部分能力不可用，可继续编辑，但预览或导出可能受限。",
    statusDetail:
      status === "ready"
        ? "预览和导出能力已通过剪辑核心检测。"
        : status === "error"
          ? "运行环境检测失败，请检查媒体运行环境后重试。"
        : "部分能力不可用，可继续编辑，但预览或导出可能受限。",
    packageStatusLabel: report.licensePosture.externalRuntime ? "外部运行环境" : "打包应用已就绪",
    rows: [
      binaryRow("媒体运行环境", report.ffmpeg),
      binaryRow("媒体探测环境", report.ffprobe),
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
    statusLabel: "运行环境检测失败，请检查媒体运行环境后重试。",
    statusDetail: message,
    packageStatusLabel: "运行环境不可用",
    rows: [
      {
        label: "媒体运行环境",
        value: message.includes("媒体运行环境") ? "未找到" : "待检测",
        detail: message,
        tone: "error"
      },
      {
        label: "媒体探测环境",
        value: message.includes("媒体探测环境") ? "未找到" : "待检测",
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

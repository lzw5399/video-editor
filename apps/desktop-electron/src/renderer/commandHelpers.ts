import type {
  CommandResultEnvelope,
  RuntimeBinaryCapability,
  RuntimeCapabilityReport,
  RuntimeFeatureCapability,
  RuntimeFontCapability
} from "../generated/CommandResultEnvelope";
import type {
  TaskRuntimeStatusResponse,
  TaskRuntimeTelemetryResponse,
  TaskRuntimeTelemetrySummary
} from "../main/nativeBinding";
import type {
  RuntimeDiagnosticsDisplayState,
  RuntimeDiagnosticsRow,
  RuntimeDiagnosticsTone
} from "./viewModel";

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
    packageStatusLabel: report.licensePosture.externalRuntime ? "运行环境异常" : "内置运行环境",
    schedulerStatusLabel: null,
    rows: [
      binaryRow("媒体运行环境", report.ffmpeg),
      binaryRow("媒体探测环境", report.ffprobe),
      featurePairRow("编码能力", report.h264Encoder, report.aacEncoder),
      featurePairRow("字幕能力", report.assFilter, report.subtitlesFilter),
      fontRow("字体环境", report.fontReadiness),
      {
        label: "打包状态",
        value: report.licensePosture.redistributableBuild ? "可再发行构建" : "内置运行环境待审查",
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
    schedulerStatusLabel: null,
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

export function runtimeDiagnosticsWithSchedulerEvidence(
  base: RuntimeDiagnosticsDisplayState,
  statusResult: CommandResultEnvelope<TaskRuntimeStatusResponse>,
  telemetryResult: CommandResultEnvelope<TaskRuntimeTelemetryResponse> | null
): RuntimeDiagnosticsDisplayState {
  const rows = [...base.rows];
  const diagnostics = [...base.diagnostics];
  const status = statusResult.ok ? statusResult.data : null;
  const statusError = statusResult.ok ? null : statusResult.error?.message ?? "调度服务暂不可用";
  const schedulerStatusLabel = status === null ? "调度暂不可用" : productSchedulerStatusLabel(status);
  const schedulerWorkAvailable = status?.workAvailable === true;
  const schedulerSeverity =
    status === null ? "error" : status.status === "unavailable" ? "error" : status.status === "degraded" ? "warning" : "ready";

  rows.push({
    label: "调度服务",
    value: schedulerStatusLabel,
    detail:
      status === null
        ? statusError ?? "调度服务暂不可用"
        : status.telemetryAvailable
          ? `观测已接入 · 修订 ${status.configRevision}`
          : `观测暂不可用 · 修订 ${status.configRevision}`,
    tone: schedulerStatusTone(schedulerSeverity)
  });

  if (statusError !== null) {
    diagnostics.push(statusError);
  }

  if (telemetryResult !== null) {
    const telemetry = telemetryResult.ok ? telemetryResult.data : null;
    const telemetryError = telemetryResult.ok ? null : telemetryResult.error?.message ?? "调度观测暂不可用";
    rows.push({
      label: "调度统计",
      value: telemetry === null ? "观测暂不可用" : schedulerTelemetryValue(telemetry),
      detail: telemetry === null ? telemetryError ?? "调度观测暂不可用" : schedulerTelemetryDetail(telemetry),
      tone: telemetry === null ? "warning" : schedulerTelemetryTone(telemetry)
    });
    if (telemetryError !== null) {
      diagnostics.push(telemetryError);
    }
  }

  const statusDetail =
    schedulerSeverity === "error"
      ? statusError ?? "调度暂不可用，预览或导出暂不可用。"
      : schedulerSeverity === "warning" && base.status !== "error"
        ? "调度服务受限，可继续编辑，但后台任务可能排队。"
        : base.statusDetail;

  return {
    ...base,
    status: mergeRuntimeStatus(base.status, schedulerSeverity),
    statusLabel: mergeRuntimeStatusLabel(base.status, schedulerSeverity, base.statusLabel),
    statusDetail,
    schedulerStatusLabel,
    rows,
    diagnostics: diagnostics.filter((message, index, all) => message.length > 0 && all.indexOf(message) === index),
    canPreview: base.canPreview && schedulerWorkAvailable,
    canExport: base.canExport && schedulerWorkAvailable
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

function productSchedulerStatusLabel(status: TaskRuntimeStatusResponse): string {
  if (status.status === "ready" && status.workAvailable) {
    return status.statusLabel.length > 0 ? status.statusLabel : "调度服务就绪";
  }
  if (status.status === "degraded") {
    return "调度服务受限";
  }
  return "调度暂不可用";
}

function schedulerStatusTone(status: RuntimeDiagnosticsTone): RuntimeDiagnosticsTone {
  return status;
}

function schedulerTelemetryValue(telemetry: TaskRuntimeTelemetryResponse): string {
  return `已完成 ${formatCount(telemetry.completedCount)} · 已取消 ${formatCount(telemetry.canceledCount)}`;
}

function schedulerTelemetryDetail(telemetry: TaskRuntimeTelemetryResponse): string {
  return [
    `已提交 ${formatCount(telemetry.submittedCount)}`,
    `已开始 ${formatCount(telemetry.startedCount)}`,
    `已拒绝 ${formatCount(telemetry.rejectedCount)}`,
    `已合并 ${formatCount(telemetry.coalescedCount)}`,
    `旧请求 ${formatCount(telemetry.staleRejectedCount)}`,
    `不可用 ${formatCount(telemetry.unavailableCount)}`,
    `饱和 ${formatCount(telemetry.resourceSaturationCount)}`,
    `等待 P95 ${formatSummaryP95(telemetry.waitTimeUs)}`
  ].join(" · ");
}

function schedulerTelemetryTone(telemetry: TaskRuntimeTelemetryResponse): RuntimeDiagnosticsTone {
  if (telemetry.status === "unavailable" || telemetry.unavailableCount > 0) {
    return "error";
  }
  if (telemetry.status === "degraded" || telemetry.rejectedCount > 0 || telemetry.resourceSaturationCount > 0) {
    return "warning";
  }
  return "ready";
}

function formatSummaryP95(summary: TaskRuntimeTelemetrySummary): string {
  return summary.p95 === null || summary.p95 === undefined ? "-" : `${Math.round(summary.p95 / 1000)} ms`;
}

function formatCount(value: number): string {
  return Math.max(0, Math.round(value)).toString();
}

function mergeRuntimeStatus(
  base: RuntimeDiagnosticsDisplayState["status"],
  schedulerSeverity: RuntimeDiagnosticsTone
): RuntimeDiagnosticsDisplayState["status"] {
  if (base === "checking") {
    return base;
  }
  if (base === "error" || schedulerSeverity === "error") {
    return "error";
  }
  if (base === "warning" || schedulerSeverity === "warning") {
    return "warning";
  }
  return base;
}

function mergeRuntimeStatusLabel(
  base: RuntimeDiagnosticsDisplayState["status"],
  schedulerSeverity: RuntimeDiagnosticsTone,
  baseLabel: string
): string {
  if (base === "error") {
    return baseLabel;
  }
  if (schedulerSeverity === "error") {
    return "调度暂不可用，预览或导出暂不可用。";
  }
  if (schedulerSeverity === "warning" && base !== "warning") {
    return "部分能力不可用，可继续编辑，但预览或导出可能受限。";
  }
  return baseLabel;
}

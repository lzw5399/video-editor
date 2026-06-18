import type { ExportPreset } from "../../generated/CommandEnvelope";
import type { DraftCanvasConfig } from "../../generated/Draft";
import {
  canvasBackgroundTone,
  formatCanvasAspectRatio,
  formatCanvasBackgroundStatus,
  formatCanvasReadout,
  formatExportPhase,
  formatExportPreset,
  formatExportProgress,
  formatMicroseconds,
  type BindingStatus,
  type ExportDisplayState,
  type PreviewDisplayState,
  type RuntimeDiagnosticsDisplayState,
  type RuntimeDiagnosticsRow,
  type RuntimeDiagnosticsTone
} from "../viewModel";

import "./preview-inspector.css";

type PreviewMonitorProps = {
  draftName: string;
  canvasConfig: DraftCanvasConfig;
  bindingStatus: BindingStatus;
  preview: PreviewDisplayState;
  exportState: ExportDisplayState;
  runtimeDiagnostics: RuntimeDiagnosticsDisplayState;
  pending: boolean;
  playheadUs?: number;
  onPlayheadChange: (value: number) => void;
  onRequestPreviewFrame: () => void;
  onRequestPreviewSegment: () => void;
  onProbeRuntimeCapabilities: () => void;
  onExportOutputPathChange: (value: string) => void;
  onExportPresetChange: (value: ExportPreset) => void;
  onStartExport: () => void;
  onRefreshExportStatus: () => void;
  onCancelExport: () => void;
};

type MonitorControl = {
  label: string;
  symbol: string;
};

const MONITOR_CONTROLS: readonly MonitorControl[] = [
  { label: "播放", symbol: "▶" },
  { label: "停止", symbol: "■" },
  { label: "上一帧", symbol: "‹" },
  { label: "下一帧", symbol: "›" },
  { label: "适应窗口", symbol: "□" },
  { label: "画面比例", symbol: "16:9" },
  { label: "全屏", symbol: "⛶" }
];

export function PreviewMonitor({
  draftName,
  canvasConfig,
  bindingStatus,
  preview,
  exportState,
  runtimeDiagnostics,
  pending,
  playheadUs = 0,
  onPlayheadChange,
  onRequestPreviewFrame,
  onRequestPreviewSegment,
  onProbeRuntimeCapabilities,
  onExportOutputPathChange,
  onExportPresetChange,
  onStartExport,
  onRefreshExportStatus,
  onCancelExport
}: PreviewMonitorProps): React.ReactElement {
  const safePlayheadUs = Math.max(0, Math.round(playheadUs));
  const frameStepUs = frameDurationUs(canvasConfig);
  const canvasReadout = formatCanvasReadout(canvasConfig);
  const canvasRatio = formatCanvasAspectRatio(canvasConfig);
  const backgroundStatus = formatCanvasBackgroundStatus(canvasConfig);
  const backgroundTone = canvasBackgroundTone(canvasConfig);
  const canvasStyle = {
    aspectRatio: `${Math.max(1, canvasConfig.width)} / ${Math.max(1, canvasConfig.height)}`,
    background: canvasConfig.background.kind === "solidColor" ? canvasConfig.background.color : "#070707"
  };
  const exportCanCancel =
    exportState.jobId !== null &&
    (exportState.phase === "queued" || exportState.phase === "running" || exportState.phase === "validating");
  const previewFrameLabel = runtimeDiagnostics.canPreview ? "请求预览帧" : "预览暂不可用";
  const previewSegmentLabel = runtimeDiagnostics.canPreview ? "生成预览片段" : "预览暂不可用";
  const startExportLabel = runtimeDiagnostics.canExport ? "开始导出" : "导出暂不可用";

  return (
    <div className="preview-shell">
      <div className="preview-titlebar">
        <strong>{draftName}</strong>
        <span title={canvasReadout}>预览命令已接入 · {canvasReadout}</span>
      </div>

      <div
        className={`preview-canvas canvas-background-${backgroundTone}`}
        aria-label="预览画面"
        style={canvasStyle}
      >
        {preview.frameDisplayUrl === null ? (
          <div className="preview-placeholder">
            <span>{preview.frameArtifactPath === null ? "等待请求预览帧" : "预览帧已返回，正在准备显示"}</span>
          </div>
        ) : (
          <img className="preview-frame-image" src={preview.frameDisplayUrl} alt="当前预览帧" aria-label="当前预览帧" />
        )}
      </div>

      <div className="preview-transport" aria-label="预览控制">
        <div className="preview-timecode" aria-label="当前时间码">
          {formatMicroseconds(safePlayheadUs)}
        </div>
        <div className="preview-control-group" role="group" aria-label="预览播放控制">
          {MONITOR_CONTROLS.map((control) => (
            <button
              key={control.label}
              type="button"
              className={control.label === "画面比例" ? "preview-icon-button ratio-button" : "preview-icon-button"}
              aria-label={control.label}
              title={control.label}
              onClick={() => {
                if (control.label === "停止") {
                  onPlayheadChange(0);
                } else if (control.label === "上一帧") {
                  onPlayheadChange(Math.max(0, safePlayheadUs - frameStepUs));
                } else if (control.label === "下一帧") {
                  onPlayheadChange(safePlayheadUs + frameStepUs);
                }
              }}
              disabled={pending || control.label === "播放" || control.label === "适应窗口" || control.label === "画面比例" || control.label === "全屏"}
            >
              <span aria-hidden="true">{control.label === "画面比例" ? canvasRatio : control.symbol}</span>
            </button>
          ))}
        </div>
        <label className="preview-seek-control">
          <span>预览时间</span>
          <input
            aria-label="预览时间"
            type="number"
            min="0"
            step="100000"
            value={safePlayheadUs}
            onChange={(event) => onPlayheadChange(Math.max(0, Math.round(event.currentTarget.valueAsNumber || 0)))}
          />
        </label>
        <div className="preview-command-group" role="group" aria-label="预览生成">
          <button
            type="button"
            className="preview-command-button"
            aria-label={previewFrameLabel}
            title={previewFrameLabel}
            onClick={onRequestPreviewFrame}
            disabled={pending || !runtimeDiagnostics.canPreview}
          >
            帧
          </button>
          <button
            type="button"
            className="preview-command-button"
            aria-label={previewSegmentLabel}
            title={previewSegmentLabel}
            onClick={onRequestPreviewSegment}
            disabled={pending || !runtimeDiagnostics.canPreview}
          >
            片段
          </button>
        </div>
      </div>

      <div className="preview-artifact-panel" aria-label="预览产物">
        <PreviewArtifactLine title="预览帧" status={preview.frameStatusLabel} metadata={preview.frameMetadataLabel} path={preview.frameArtifactPath} />
        <PreviewArtifactLine
          title="预览片段"
          status={preview.segmentStatusLabel}
          metadata={preview.segmentMetadataLabel}
          path={preview.segmentArtifactPath}
        />
      </div>

      <RuntimeDiagnosticsPanel diagnostics={runtimeDiagnostics} pending={pending} onProbe={onProbeRuntimeCapabilities} />

      <div className="export-panel" aria-label="导出面板">
        <label className="export-path-control">
          <span>输出路径</span>
          <input
            aria-label="输出路径"
            type="text"
            value={exportState.outputPath}
            onChange={(event) => onExportOutputPathChange(event.currentTarget.value)}
            disabled={pending}
          />
        </label>
        <label className="export-preset-control">
          <span>导出预设</span>
          <select
            aria-label="导出预设"
            value={exportState.preset}
            onChange={(event) => onExportPresetChange(event.currentTarget.value as ExportPreset)}
            disabled={pending}
          >
            <option value="h264AacBalanced">{formatExportPreset("h264AacBalanced")}</option>
            <option value="h264AacDraft">{formatExportPreset("h264AacDraft")}</option>
          </select>
        </label>
        <div className="export-actions" role="group" aria-label="导出操作">
          <button type="button" aria-label={startExportLabel} title={startExportLabel} onClick={onStartExport} disabled={pending || !runtimeDiagnostics.canExport}>
            导出
          </button>
          <button
            type="button"
            aria-label="查询导出状态"
            title="查询导出状态"
            onClick={onRefreshExportStatus}
            disabled={pending || exportState.jobId === null}
          >
            状态
          </button>
          <button
            type="button"
            aria-label="取消导出"
            title="取消导出"
            onClick={onCancelExport}
            disabled={pending || !exportCanCancel}
          >
            取消
          </button>
        </div>
        <div className="export-progress" aria-label="导出进度">
          <span>{formatExportPhase(exportState.phase)}</span>
          <progress max={1000} value={exportState.progressPerMille ?? 0} />
          <strong>{formatExportProgress(exportState.progressPerMille)}</strong>
        </div>
        <div className="export-log" aria-label="导出日志">
          {exportState.error ?? exportState.diagnosticLabel ?? exportState.logSummary}
        </div>
        <div className="export-validation" aria-label="输出校验">
          {exportState.validation === null
            ? "输出校验待完成"
            : `${exportState.validation.width ?? "-"}x${exportState.validation.height ?? "-"} · ${
                exportState.validation.hasAudio ? "含音频" : "无音频"
              }`}
        </div>
      </div>

      <div className="preview-status-line" aria-live="polite">
        <span className={`status-dot ${bindingStatus.kind}`} aria-hidden="true" />
        <span aria-label="预览状态">{preview.error ?? preview.frameStatusLabel}</span>
        <span className="canvas-readout-chip" title={canvasReadout}>
          {canvasReadout}
        </span>
        <span className={`canvas-background-chip ${backgroundTone}`} title={backgroundStatus}>
          {backgroundStatus}
        </span>
      </div>
    </div>
  );
}

function frameDurationUs(canvasConfig: DraftCanvasConfig): number {
  const numerator = Math.max(1, Math.round(canvasConfig.frameRate.numerator));
  const denominator = Math.max(1, Math.round(canvasConfig.frameRate.denominator));
  return Math.max(1, Math.round((denominator * 1_000_000) / numerator));
}

function RuntimeDiagnosticsPanel({
  diagnostics,
  pending,
  onProbe
}: {
  diagnostics: RuntimeDiagnosticsDisplayState;
  pending: boolean;
  onProbe: () => void;
}): React.ReactElement {
  const actionLabel = diagnostics.status === "idle" ? "检查运行环境" : "重新检测运行环境";
  const busy = diagnostics.status === "checking";
  const detail = diagnostics.diagnostics[0] ?? diagnostics.statusDetail;

  return (
    <section className={`runtime-diagnostics-panel ${diagnostics.status}`} aria-label="运行环境诊断">
      <div className="runtime-diagnostics-header">
        <div className="runtime-status-summary" aria-label="运行环境状态">
          <span className={`runtime-status-dot ${diagnostics.status}`} aria-hidden="true" />
          <strong>{diagnostics.statusLabel}</strong>
          <span title={detail}>{detail}</span>
        </div>
        <button
          type="button"
          aria-label={actionLabel}
          title={actionLabel}
          onClick={onProbe}
          disabled={pending || busy}
        >
          {actionLabel === "检查运行环境" ? "检测" : "重检"}
        </button>
      </div>
      <div className="runtime-capability-grid" aria-label="运行能力列表">
        {diagnostics.rows.length === 0 ? (
          <div className="runtime-capability-row muted" aria-label="打包状态">
            <strong>打包状态</strong>
            <span>{diagnostics.packageStatusLabel}</span>
            <em>{diagnostics.checkedAtLabel}</em>
          </div>
        ) : (
          diagnostics.rows.map((row) => <RuntimeDiagnosticsRowView key={row.label} row={row} />)
        )}
      </div>
    </section>
  );
}

function RuntimeDiagnosticsRowView({ row }: { row: RuntimeDiagnosticsRow }): React.ReactElement {
  return (
    <div className={`runtime-capability-row ${runtimeToneClass(row.tone)}`} aria-label={row.label} title={row.detail}>
      <strong>{row.label}</strong>
      <span>{row.value}</span>
      <em>{row.detail || "无额外诊断"}</em>
    </div>
  );
}

function runtimeToneClass(tone: RuntimeDiagnosticsTone): string {
  return tone;
}

function PreviewArtifactLine({
  title,
  status,
  metadata,
  path
}: {
  title: string;
  status: string;
  metadata: string;
  path: string | null;
}): React.ReactElement {
  return (
    <div className="preview-artifact-line">
      <strong>{title}</strong>
      <span>{status}</span>
      <span>{metadata}</span>
      {path === null ? null : <code>{path}</code>}
    </div>
  );
}

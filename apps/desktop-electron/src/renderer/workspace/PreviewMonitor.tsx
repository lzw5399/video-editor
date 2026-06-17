import type { ExportPreset } from "../../generated/CommandEnvelope";
import {
  formatExportPhase,
  formatExportPreset,
  formatExportProgress,
  formatMicroseconds,
  type BindingStatus,
  type ExportDisplayState,
  type PreviewDisplayState
} from "../viewModel";

import "./preview-inspector.css";

type PreviewMonitorProps = {
  draftName: string;
  bindingStatus: BindingStatus;
  preview: PreviewDisplayState;
  exportState: ExportDisplayState;
  pending: boolean;
  playheadUs?: number;
  onPlayheadChange: (value: number) => void;
  onRequestPreviewFrame: () => void;
  onRequestPreviewSegment: () => void;
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
  bindingStatus,
  preview,
  exportState,
  pending,
  playheadUs = 0,
  onPlayheadChange,
  onRequestPreviewFrame,
  onRequestPreviewSegment,
  onExportOutputPathChange,
  onExportPresetChange,
  onStartExport,
  onRefreshExportStatus,
  onCancelExport
}: PreviewMonitorProps): React.ReactElement {
  const safePlayheadUs = Math.max(0, Math.round(playheadUs));
  const exportCanCancel =
    exportState.jobId !== null &&
    (exportState.phase === "queued" || exportState.phase === "running" || exportState.phase === "validating");

  return (
    <div className="preview-shell">
      <div className="preview-titlebar">
        <strong>{draftName}</strong>
        <span>预览命令已接入</span>
      </div>

      <div className="preview-canvas" aria-label="预览画面">
        <div className="preview-placeholder">
          <span>{preview.frameArtifactPath === null ? "等待请求预览帧" : "预览帧已返回"}</span>
          {preview.frameArtifactPath === null ? null : <code>{preview.frameArtifactPath}</code>}
        </div>
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
              className="preview-icon-button"
              aria-label={control.label}
              title={control.label}
              onClick={() => {
                if (control.label === "停止") {
                  onPlayheadChange(0);
                } else if (control.label === "上一帧") {
                  onPlayheadChange(Math.max(0, safePlayheadUs - 33_333));
                } else if (control.label === "下一帧") {
                  onPlayheadChange(safePlayheadUs + 33_333);
                }
              }}
              disabled={pending || control.label === "播放" || control.label === "适应窗口" || control.label === "画面比例" || control.label === "全屏"}
            >
              <span aria-hidden="true">{control.symbol}</span>
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
            aria-label="请求预览帧"
            title="请求预览帧"
            onClick={onRequestPreviewFrame}
            disabled={pending}
          >
            帧
          </button>
          <button
            type="button"
            className="preview-command-button"
            aria-label="生成预览片段"
            title="生成预览片段"
            onClick={onRequestPreviewSegment}
            disabled={pending}
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
          <button type="button" aria-label="开始导出" title="开始导出" onClick={onStartExport} disabled={pending}>
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
        <span>16:9</span>
        <span>30 fps</span>
      </div>
    </div>
  );
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

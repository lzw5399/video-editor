import { formatMicroseconds, type BindingStatus, type PreviewDisplayState } from "../viewModel";

import "./preview-inspector.css";

type PreviewMonitorProps = {
  draftName: string;
  bindingStatus: BindingStatus;
  preview: PreviewDisplayState;
  pending: boolean;
  playheadUs?: number;
  onPlayheadChange: (value: number) => void;
  onRequestPreviewFrame: () => void;
  onRequestPreviewSegment: () => void;
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
  pending,
  playheadUs = 0,
  onPlayheadChange,
  onRequestPreviewFrame,
  onRequestPreviewSegment
}: PreviewMonitorProps): React.ReactElement {
  const safePlayheadUs = Math.max(0, Math.round(playheadUs));

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

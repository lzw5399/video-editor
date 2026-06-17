import { formatMicroseconds, type BindingStatus } from "../viewModel";

import "./preview-inspector.css";

type PreviewMonitorProps = {
  draftName: string;
  bindingStatus: BindingStatus;
  playheadUs?: number;
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

export function PreviewMonitor({ draftName, bindingStatus, playheadUs = 0 }: PreviewMonitorProps): React.ReactElement {
  return (
    <div className="preview-shell">
      <div className="preview-titlebar">
        <strong>{draftName}</strong>
        <span>预览待接入</span>
      </div>

      <div className="preview-canvas" aria-label="预览画面">
        <div className="preview-placeholder">
          <span>预览画面将在下一阶段接入</span>
        </div>
      </div>

      <div className="preview-transport" aria-label="预览控制">
        <div className="preview-timecode" aria-label="当前时间码">
          {formatMicroseconds(playheadUs ?? 0)}
        </div>
        <div className="preview-control-group" role="group" aria-label="预览播放控制">
          {MONITOR_CONTROLS.map((control) => (
            <button
              key={control.label}
              type="button"
              className="preview-icon-button"
              aria-label={control.label}
              title={control.label}
              disabled
            >
              <span aria-hidden="true">{control.symbol}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="preview-status-line" aria-live="polite">
        <span className={`status-dot ${bindingStatus.kind}`} aria-hidden="true" />
        <span>等待预览帧接入</span>
        <span>16:9</span>
        <span>30 fps</span>
      </div>
    </div>
  );
}

import type { BindingStatus } from "../viewModel";

type PreviewMonitorProps = {
  draftName: string;
  bindingStatus: BindingStatus;
};

export function PreviewMonitor({ draftName, bindingStatus }: PreviewMonitorProps): React.ReactElement {
  return (
    <div className="preview-shell">
      <div className="preview-stage" aria-label="预览画面">
        <div className="preview-placeholder">
          <strong>预览将在下一阶段接入</strong>
          <span>{draftName}</span>
        </div>
      </div>
      <div className="preview-status" aria-live="polite">
        <span className={`status-dot ${bindingStatus.kind}`} />
        <span>{bindingStatus.label}</span>
      </div>
    </div>
  );
}

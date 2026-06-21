import { useState } from "react";

import {
  WORKSPACE_CATEGORIES,
  WORKSPACE_CATEGORY_META,
  artifactPreviewStatusLabel,
  formatExportPhase,
  formatExportPreset,
  formatExportProgress,
  isProductSafeStatusCopy,
  type WorkspaceCategory,
  type WorkspaceState
} from "../viewModel";
import type { ExportPreset } from "../../generated/CommandEnvelope";
import type { DraftCanvasConfig, KeyframeEasing, KeyframeInterpolation, KeyframeProperty, SegmentVisual } from "../../generated/Draft";
import { FeaturePanel } from "./FeaturePanel";
import { Inspector } from "./Inspector";
import { PreviewMonitor, type RealtimePreviewHostState } from "./PreviewMonitor";
import { Timeline } from "./Timeline";

type WorkspaceShellProps = {
  workspace: WorkspaceState;
  activeCategory: WorkspaceCategory;
  showDeveloperDiagnostics: boolean;
  bundlePath: string;
  materialPath: string;
  playheadUs: number;
  playbackRunning: boolean;
  onRealtimePreviewHostStateChange: (state: RealtimePreviewHostState) => void;
  onCategoryChange: (category: WorkspaceCategory) => void;
  onBundlePathChange: (value: string) => void;
  onMaterialPathChange: (value: string) => void;
  onPlayheadChange: (value: number) => void;
  onTogglePlayback: () => void;
  onStopPlayback: () => void;
  onRequestPreviewFrame: () => void;
  onRequestPreviewSegment: () => void;
  onProbeRuntimeCapabilities: () => void;
  onExportOutputPathChange: (value: string) => void;
  onExportPresetChange: (value: ExportPreset) => void;
  onStartExport: () => void;
  onRefreshExportStatus: () => void;
  onCancelExport: () => void;
  onRetryAudioPreview: () => void;
  onSelectAudioOutputDevice: (deviceSelectionId: string) => void;
  onImportMaterial: () => void;
  onImportMaterialFromPath: () => void;
  onRefreshMaterials: () => void;
  onListMissingMaterials: () => void;
  onRefreshArtifactStatus: () => void;
  onCancelArtifactGeneration: (jobId: string) => void;
  onRetryArtifactGeneration: (jobId: string) => void;
  onResumeArtifactGeneration: (jobId: string) => void;
  onPrepareArtifactCleanup: () => void;
  onConfirmArtifactCleanup: () => void;
  onDismissResourceNotice: () => void;
  onAddTextSegment: Parameters<typeof FeaturePanel>[0]["onAddTextSegment"];
  onImportSubtitleSrt: Parameters<typeof FeaturePanel>[0]["onImportSubtitleSrt"];
  onAddAudioSegment: Parameters<typeof FeaturePanel>[0]["onAddAudioSegment"];
  onEditSelectedText: Parameters<typeof Inspector>[0]["onEditSelectedText"];
  onUpdateDraftCanvasConfig: (canvasConfig: DraftCanvasConfig) => void;
  onUpdateSelectedSegmentVisual: (visual: SegmentVisual) => void;
  onSetSelectedSegmentKeyframe: (
    property: KeyframeProperty,
    interpolation?: KeyframeInterpolation,
    easing?: KeyframeEasing
  ) => void;
  onRemoveSelectedSegmentKeyframe: (property: KeyframeProperty, at: number) => void;
  onSetSelectedSegmentVolume: Parameters<typeof FeaturePanel>[0]["onSetSelectedSegmentVolume"];
  onUpdateSelectedSegmentAudio: Parameters<typeof FeaturePanel>[0]["onUpdateSelectedSegmentAudio"];
  onSetSelectedTrackMute: Parameters<typeof FeaturePanel>[0]["onSetSelectedTrackMute"];
  onSelectTimelineSegment: Parameters<typeof Timeline>[0]["onSelectSegment"];
  onSelectTimelineTrack: Parameters<typeof Timeline>[0]["onSelectTrack"];
  onAddTimelineSegment: Parameters<typeof Timeline>[0]["onAddSegment"];
  onAddTimelineTrack: Parameters<typeof Timeline>[0]["onAddTrack"];
  onRenameTimelineTrack: Parameters<typeof Timeline>[0]["onRenameTrack"];
  onSetTimelineTrackLock: Parameters<typeof Timeline>[0]["onSetTrackLock"];
  onSetTimelineTrackVisibility: Parameters<typeof Timeline>[0]["onSetTrackVisibility"];
  onMoveSelectedSegment: Parameters<typeof Timeline>[0]["onMoveSelectedSegment"];
  onSplitSelectedSegment: Parameters<typeof Timeline>[0]["onSplitSelectedSegment"];
  onTrimSelectedSegment: Parameters<typeof Timeline>[0]["onTrimSelectedSegment"];
  onDeleteSelectedSegment: Parameters<typeof Timeline>[0]["onDeleteSelectedSegment"];
  onSetTimelineTrackMute: Parameters<typeof Timeline>[0]["onSetTrackMute"];
  onUndoTimelineEdit: Parameters<typeof Timeline>[0]["onUndo"];
  onRedoTimelineEdit: Parameters<typeof Timeline>[0]["onRedo"];
};

export function WorkspaceShell({
  workspace,
  activeCategory,
  showDeveloperDiagnostics,
  bundlePath,
  materialPath,
  playheadUs,
  playbackRunning,
  onRealtimePreviewHostStateChange,
  onCategoryChange,
  onBundlePathChange,
  onMaterialPathChange,
  onPlayheadChange,
  onTogglePlayback,
  onStopPlayback,
  onRequestPreviewFrame,
  onRequestPreviewSegment,
  onProbeRuntimeCapabilities,
  onExportOutputPathChange,
  onExportPresetChange,
  onStartExport,
  onRefreshExportStatus,
  onCancelExport,
  onRetryAudioPreview,
  onSelectAudioOutputDevice,
  onImportMaterial,
  onImportMaterialFromPath,
  onRefreshMaterials,
  onListMissingMaterials,
  onRefreshArtifactStatus,
  onCancelArtifactGeneration,
  onRetryArtifactGeneration,
  onResumeArtifactGeneration,
  onPrepareArtifactCleanup,
  onConfirmArtifactCleanup,
  onDismissResourceNotice,
  onAddTextSegment,
  onImportSubtitleSrt,
  onAddAudioSegment,
  onEditSelectedText,
  onUpdateDraftCanvasConfig,
  onUpdateSelectedSegmentVisual,
  onSetSelectedSegmentKeyframe,
  onRemoveSelectedSegmentKeyframe,
  onSetSelectedSegmentVolume,
  onUpdateSelectedSegmentAudio,
  onSetSelectedTrackMute,
  onSelectTimelineSegment,
  onSelectTimelineTrack,
  onAddTimelineSegment,
  onAddTimelineTrack,
  onRenameTimelineTrack,
  onSetTimelineTrackLock,
  onSetTimelineTrackVisibility,
  onMoveSelectedSegment,
  onSplitSelectedSegment,
  onTrimSelectedSegment,
  onDeleteSelectedSegment,
  onSetTimelineTrackMute,
  onUndoTimelineEdit,
  onRedoTimelineEdit
}: WorkspaceShellProps): React.ReactElement {
  const selectedSegment = workspace.viewModel.selectedSegment;
  const [exportModalOpen, setExportModalOpen] = useState(false);

  return (
    <main className="workspace" aria-label="剪映风格编辑工作区">
      <header className="top-feature-bar" aria-label="顶部功能区">
        <h1 className="product-mark">视频剪辑</h1>
        <nav className="category-nav" aria-label="顶部功能区">
          {WORKSPACE_CATEGORIES.map((category) => {
            const metadata = WORKSPACE_CATEGORY_META[category];

            return (
              <button
                key={category}
                type="button"
                className={category === activeCategory ? "category-button active" : "category-button"}
                aria-label={metadata.label}
                aria-pressed={category === activeCategory}
                title={metadata.label}
                onClick={() => onCategoryChange(category)}
              >
                <span className="category-symbol" aria-hidden="true">
                  {metadata.symbol}
                </span>
                <span className="category-label">{metadata.label}</span>
              </button>
            );
          })}
        </nav>
        <div className="product-action-bar" aria-label="产品操作">
          <button type="button" className="top-export-button" aria-label="导出" onClick={() => setExportModalOpen(true)}>
            导出
          </button>
        </div>
      </header>

      <section className="material-panel" aria-label="素材面板">
        <FeaturePanel
          category={activeCategory}
          workspace={workspace}
          showDeveloperDiagnostics={showDeveloperDiagnostics}
          bundlePath={bundlePath}
          materialPath={materialPath}
          onBundlePathChange={onBundlePathChange}
          onMaterialPathChange={onMaterialPathChange}
          onImportMaterial={onImportMaterial}
          onImportMaterialFromPath={onImportMaterialFromPath}
          onRefreshMaterials={onRefreshMaterials}
          onListMissingMaterials={onListMissingMaterials}
          onRefreshArtifactStatus={onRefreshArtifactStatus}
          onCancelArtifactGeneration={onCancelArtifactGeneration}
          onRetryArtifactGeneration={onRetryArtifactGeneration}
          onResumeArtifactGeneration={onResumeArtifactGeneration}
          onPrepareArtifactCleanup={onPrepareArtifactCleanup}
          onConfirmArtifactCleanup={onConfirmArtifactCleanup}
          onDismissResourceNotice={onDismissResourceNotice}
          onSelectAudioOutputDevice={onSelectAudioOutputDevice}
          onAddTimelineSegment={onAddTimelineSegment}
          onAddTextSegment={onAddTextSegment}
          onImportSubtitleSrt={onImportSubtitleSrt}
          onAddAudioSegment={onAddAudioSegment}
          onSetSelectedSegmentVolume={onSetSelectedSegmentVolume}
          onUpdateSelectedSegmentAudio={onUpdateSelectedSegmentAudio}
          onSetSelectedTrackMute={onSetSelectedTrackMute}
        />
      </section>

      <section className="preview-monitor" aria-label="预览窗口">
        <PreviewMonitor
          draftName={workspace.viewModel.project.draftName}
          canvasConfig={workspace.viewModel.project.canvasConfig}
          bindingStatus={workspace.bindingStatus}
          preview={workspace.preview}
          resourcePreviewStatusLabel={artifactPreviewStatusLabel(workspace.resourcePanel)}
          audioPreview={workspace.audioPreview}
          audioDevices={workspace.audioDevices}
          audioParity={workspace.audioParity}
          waveform={workspace.waveform}
          runtimeDiagnostics={workspace.runtimeDiagnostics}
          selectedSegment={selectedSegment}
          showDeveloperDiagnostics={showDeveloperDiagnostics}
          pending={workspace.pendingCommand !== null}
          playheadUs={playheadUs}
          playbackRunning={playbackRunning}
          onRealtimePreviewHostStateChange={onRealtimePreviewHostStateChange}
          onPlayheadChange={onPlayheadChange}
          onTogglePlayback={onTogglePlayback}
          onStopPlayback={onStopPlayback}
          onRequestPreviewFrame={onRequestPreviewFrame}
          onRequestPreviewSegment={onRequestPreviewSegment}
          onProbeRuntimeCapabilities={onProbeRuntimeCapabilities}
          onRetryAudioPreview={onRetryAudioPreview}
          onUpdateSelectedSegmentVisual={onUpdateSelectedSegmentVisual}
        />
      </section>

      <aside className="inspector-panel" aria-label="属性检查器">
        <Inspector
          workspace={workspace}
          playheadUs={playheadUs}
          showDeveloperDiagnostics={showDeveloperDiagnostics}
          onEditSelectedText={onEditSelectedText}
          onUpdateDraftCanvasConfig={onUpdateDraftCanvasConfig}
          onUpdateSelectedSegmentVisual={onUpdateSelectedSegmentVisual}
          onSetSelectedSegmentKeyframe={onSetSelectedSegmentKeyframe}
          onRemoveSelectedSegmentKeyframe={onRemoveSelectedSegmentKeyframe}
          onSetSelectedSegmentVolume={onSetSelectedSegmentVolume}
          onUpdateSelectedSegmentAudio={onUpdateSelectedSegmentAudio}
          onSetSelectedTrackMute={onSetSelectedTrackMute}
        />
      </aside>

      <section className="timeline-panel" aria-label="时间线">
        <Timeline
          workspace={workspace}
          playheadUs={playheadUs}
          playbackRunning={playbackRunning}
          onPlayheadChange={onPlayheadChange}
          onTogglePlayback={onTogglePlayback}
          onStopPlayback={onStopPlayback}
          onSelectSegment={onSelectTimelineSegment}
          onSelectTrack={onSelectTimelineTrack}
          onAddSegment={onAddTimelineSegment}
          onAddTrack={onAddTimelineTrack}
          onRenameTrack={onRenameTimelineTrack}
          onSetTrackLock={onSetTimelineTrackLock}
          onSetTrackVisibility={onSetTimelineTrackVisibility}
          onMoveSelectedSegment={onMoveSelectedSegment}
          onSplitSelectedSegment={onSplitSelectedSegment}
          onTrimSelectedSegment={onTrimSelectedSegment}
          onDeleteSelectedSegment={onDeleteSelectedSegment}
          onSetTrackMute={onSetTimelineTrackMute}
          onUndo={onUndoTimelineEdit}
          onRedo={onRedoTimelineEdit}
        />
      </section>

      {exportModalOpen ? (
        <ExportModal
          workspace={workspace}
          showDeveloperDiagnostics={showDeveloperDiagnostics}
          onClose={() => setExportModalOpen(false)}
          onExportOutputPathChange={onExportOutputPathChange}
          onExportPresetChange={onExportPresetChange}
          onStartExport={onStartExport}
          onRefreshExportStatus={onRefreshExportStatus}
          onCancelExport={onCancelExport}
        />
      ) : null}
    </main>
  );
}

type ExportModalProps = {
  workspace: WorkspaceState;
  showDeveloperDiagnostics: boolean;
  onClose: () => void;
  onExportOutputPathChange: (value: string) => void;
  onExportPresetChange: (value: ExportPreset) => void;
  onStartExport: () => void;
  onRefreshExportStatus: () => void;
  onCancelExport: () => void;
};

const EXPORT_SAMPLE_RATES = ["48 kHz", "44.1 kHz", "96 kHz"] as const;

function ExportModal({
  workspace,
  showDeveloperDiagnostics,
  onClose,
  onExportOutputPathChange,
  onExportPresetChange,
  onStartExport,
  onRefreshExportStatus,
  onCancelExport
}: ExportModalProps): React.ReactElement {
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [sampleRateOpen, setSampleRateOpen] = useState(false);
  const [sampleRate, setSampleRate] = useState<(typeof EXPORT_SAMPLE_RATES)[number]>("48 kHz");
  const [includeAudio, setIncludeAudio] = useState(true);
  const { export: exportState, runtimeDiagnostics } = workspace;
  const pending = workspace.pendingCommand !== null;
  const exportCanCancel =
    exportState.jobId !== null &&
    (exportState.phase === "queued" || exportState.phase === "running" || exportState.phase === "validating");
  const exportCompleted = exportState.phase === "completed";
  const startExportLabel = runtimeDiagnostics.canExport ? "开始导出" : "导出暂不可用";
  const exportMessage = showDeveloperDiagnostics
    ? exportState.error ?? exportState.diagnosticLabel ?? exportState.logSummary
    : productExportStatusMessage(exportState);

  return (
    <div className="modal-backdrop" role="presentation">
      <section className="export-modal" role="dialog" aria-modal="true" aria-labelledby="export-modal-title">
        <header className="export-modal-header">
          <h2 id="export-modal-title">导出</h2>
          <button type="button" className="modal-icon-button" aria-label="关闭" onClick={onClose}>
            ×
          </button>
        </header>

        <div className="export-modal-body">
          <label className="export-modal-field wide-field">
            <span>输出路径</span>
            <input
              aria-label="输出路径"
              type="text"
              value={exportState.outputPath}
              onChange={(event) => onExportOutputPathChange(event.currentTarget.value)}
              disabled={pending}
            />
          </label>

          <div className="export-modal-grid">
            <label className="export-modal-field">
              <span>分辨率</span>
              <select aria-label="分辨率" defaultValue="draft" disabled={pending}>
                <option value="draft">跟随草稿</option>
                <option value="1080p">1080p</option>
                <option value="720p">720p</option>
              </select>
            </label>
            <label className="export-modal-field">
              <span>帧率</span>
              <select aria-label="帧率" defaultValue="draft" disabled={pending}>
                <option value="draft">跟随草稿</option>
                <option value="30">30 fps</option>
                <option value="60">60 fps</option>
              </select>
            </label>
            <label className="export-modal-field">
              <span>视频码率</span>
              <select aria-label="视频码率" defaultValue="auto" disabled={pending}>
                <option value="auto">智能推荐</option>
                <option value="8m">8 Mbps</option>
                <option value="16m">16 Mbps</option>
              </select>
            </label>
            <label className="export-modal-field">
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
          </div>

          <label className="export-audio-toggle">
            <input
              type="checkbox"
              aria-label="导出音频"
              checked={includeAudio}
              onChange={(event) => setIncludeAudio(event.currentTarget.checked)}
              disabled={pending}
            />
            <span>导出音频</span>
          </label>

          <button
            type="button"
            className="export-advanced-toggle"
            aria-expanded={advancedOpen}
            aria-controls="export-advanced-settings"
            onClick={() => setAdvancedOpen((current) => !current)}
          >
            高级设置
          </button>

          {advancedOpen ? (
            <section id="export-advanced-settings" className="export-advanced-panel" aria-label="高级导出设置">
              <label className="export-modal-field">
                <span>编码格式</span>
                <select aria-label="编码格式" defaultValue="h264" disabled={pending}>
                  <option value="h264">H.264</option>
                </select>
              </label>
              <div className="export-modal-field">
                <span>音频采样率</span>
                <button
                  type="button"
                  className="export-sample-rate-combobox"
                  role="combobox"
                  aria-label="音频采样率"
                  aria-expanded={sampleRateOpen}
                  aria-controls="export-sample-rate-options"
                  onClick={() => setSampleRateOpen((current) => !current)}
                  disabled={pending || !includeAudio}
                >
                  {sampleRate}
                </button>
                {sampleRateOpen ? (
                  <div id="export-sample-rate-options" className="export-sample-rate-list" role="listbox" aria-label="音频采样率选项">
                    {EXPORT_SAMPLE_RATES.map((rate) => (
                      <button
                        key={rate}
                        type="button"
                        role="option"
                        aria-selected={rate === sampleRate}
                        onClick={() => {
                          setSampleRate(rate);
                          setSampleRateOpen(false);
                        }}
                      >
                        {rate}
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
            </section>
          ) : null}

          <div className="export-progress" aria-label="导出进度">
            <span>{formatExportPhase(exportState.phase)}</span>
            <progress max={1000} value={exportState.progressPerMille ?? 0} />
            <strong>{formatExportProgress(exportState.progressPerMille)}</strong>
          </div>
          <div className="export-log" aria-label="导出状态">
            {exportMessage}
          </div>
          <div className="export-validation" aria-label="输出校验">
            {exportState.validation === null
              ? "输出校验待完成"
              : `${exportState.validation.width ?? "-"}x${exportState.validation.height ?? "-"} · ${
                  exportState.validation.hasAudio ? "含音频" : "无音频"
                }`}
          </div>
        </div>

        <footer className="export-modal-actions" role="group" aria-label="导出操作">
          <button type="button" className="secondary-action" aria-label="打开位置" disabled={!exportCompleted}>
            打开位置
          </button>
          <button
            type="button"
            className="secondary-action"
            aria-label="查询导出状态"
            onClick={onRefreshExportStatus}
            disabled={pending || exportState.jobId === null}
          >
            查询导出状态
          </button>
          <button
            type="button"
            className="secondary-action"
            aria-label="取消导出"
            onClick={onCancelExport}
            disabled={pending || !exportCanCancel}
          >
            取消导出
          </button>
          <button
            type="button"
            className="primary-action"
            aria-label={startExportLabel}
            onClick={onStartExport}
            disabled={pending || !runtimeDiagnostics.canExport}
          >
            导出
          </button>
        </footer>
      </section>
    </div>
  );
}

function productExportStatusMessage(exportState: WorkspaceState["export"]): string {
  if (exportState.error !== null || exportState.diagnosticLabel !== null) {
    if (exportState.phase === "cancelled") {
      return "导出已取消";
    }
    if (exportState.phase === "validationFailed") {
      return "输出校验未通过，请重新导出";
    }
    return "导出失败，请检查输出设置后重试";
  }

  if (isProductSafeStatusCopy(exportState.logSummary)) {
    return exportState.logSummary;
  }

  switch (exportState.phase) {
    case "queued":
      return "等待导出";
    case "running":
      return "正在导出";
    case "validating":
      return "正在校验输出";
    case "completed":
      return "导出完成";
    case "cancelled":
      return "导出已取消";
    case "failed":
      return "导出失败，请重试";
    case "validationFailed":
      return "输出校验未通过，请重新导出";
    case null:
      return "等待开始导出";
  }
}

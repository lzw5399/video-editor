import { useState, type CSSProperties } from "react";

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
import type {
  DraftCanvasConfig,
  EffectParameterUpdate,
  Filter,
  KeyframeEasing,
  KeyframeInterpolation,
  KeyframeProperty,
  SegmentBlendMode,
  SegmentMask,
  SegmentRetiming,
  TransitionReference
} from "../../generated/Draft";
import type { AdaptationReport } from "../../generated/TemplateImport";
import { appIconUrls, type AppIconName } from "../assets/icons";
import { FeaturePanel, type TemplateReportRowNavigationTarget } from "./FeaturePanel";
import { Inspector } from "./Inspector";
import { PreviewMonitor, type RealtimePreviewHostState } from "./PreviewMonitor";
import type { ProjectInteractionController } from "./projectInteraction";
import { Timeline } from "./Timeline";

type WorkspaceShellProps = {
  workspace: WorkspaceState;
  activeCategory: WorkspaceCategory;
  templateImportReport: AdaptationReport | null;
  showDeveloperDiagnostics: boolean;
  bundlePath: string;
  materialPath: string;
  playheadUs: number;
  playbackRunning: boolean;
  projectInteractions: ProjectInteractionController;
  onRealtimePreviewHostStateChange: (state: RealtimePreviewHostState) => void;
  onCategoryChange: (category: WorkspaceCategory) => void;
  onBundlePathChange: (value: string) => void;
  onMaterialPathChange: (value: string) => void;
  onPlayheadChange: (value: number) => void;
  onTogglePlayback: () => void;
  onStopPlayback: () => void;
  onProbeRuntimeCapabilities: () => void;
  onExportOutputPathChange: (value: string) => void;
  onExportPresetChange: (value: ExportPreset) => void;
  onSuspendRealtimePreviewSurface: () => Promise<void>;
  onStartExport: () => void;
  onRefreshExportStatus: () => void;
  onCancelExport: () => void;
  onRetryAudioPreview: () => void;
  onSelectAudioOutputDevice: (deviceSelectionId: string) => void;
  onImportMaterial: () => void;
  onImportTemplateBundle: () => void;
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
  onUpdateSelectedSegmentVisual: Parameters<typeof Inspector>[0]["onUpdateSelectedSegmentVisual"];
  onSetSelectedSegmentRetime: (retiming: SegmentRetiming) => void;
  onApplySelectedSegmentEffect: (effect: Filter) => void;
  onUpdateSelectedSegmentEffectParameter: (effectIndex: number, parameter: EffectParameterUpdate) => void;
  onRemoveSelectedSegmentEffect: (effectIndex: number) => void;
  onSetSelectedSegmentMask: (mask: SegmentMask) => void;
  onSetSelectedSegmentBlendMode: (blendMode: SegmentBlendMode) => void;
  onAddSelectedSegmentTransition: (
    fromSegmentId: string,
    toSegmentId: string,
    reference: TransitionReference,
    duration: number
  ) => void;
  onSelectPreviewTextOverlay: (selectionHandle: string) => void;
  onEditPreviewTextOverlay: (selectionHandle: string) => void;
  onSetSelectedSegmentKeyframe: (
    property: KeyframeProperty,
    interpolation?: KeyframeInterpolation,
    easing?: KeyframeEasing
  ) => void;
  onRemoveSelectedSegmentKeyframe: (property: KeyframeProperty, at: number) => void;
  onSetSelectedSegmentVolume: Parameters<typeof FeaturePanel>[0]["onSetSelectedSegmentVolume"];
  onUpdateSelectedSegmentAudio: Parameters<typeof FeaturePanel>[0]["onUpdateSelectedSegmentAudio"];
  onSetSelectedTrackMute: Parameters<typeof FeaturePanel>[0]["onSetSelectedTrackMute"];
  onNavigateTemplateReportItem: (target: TemplateReportRowNavigationTarget) => void;
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

const CATEGORY_ICON_NAMES: Record<WorkspaceCategory, AppIconName> = {
  媒体: "categoryMedia",
  音频: "categoryAudio",
  文字: "categoryText",
  贴纸: "categorySticker",
  特效: "categoryEffect",
  转场: "categoryTransition",
  字幕: "categoryCaption",
  滤镜: "categoryFilter",
  调节: "categoryAdjust",
  模板: "categoryTemplate",
  数字人: "categoryDigitalHuman"
};

const PRIMARY_CATEGORY_COUNT = 7;
const PRIMARY_WORKSPACE_CATEGORIES = WORKSPACE_CATEGORIES.slice(0, PRIMARY_CATEGORY_COUNT);
const OVERFLOW_WORKSPACE_CATEGORIES = WORKSPACE_CATEGORIES.slice(PRIMARY_CATEGORY_COUNT);

export function WorkspaceShell({
  workspace,
  activeCategory,
  templateImportReport,
  showDeveloperDiagnostics,
  bundlePath,
  materialPath,
  playheadUs,
  playbackRunning,
  projectInteractions,
  onRealtimePreviewHostStateChange,
  onCategoryChange,
  onBundlePathChange,
  onMaterialPathChange,
  onPlayheadChange,
  onTogglePlayback,
  onStopPlayback,
  onProbeRuntimeCapabilities,
  onExportOutputPathChange,
  onExportPresetChange,
  onSuspendRealtimePreviewSurface,
  onStartExport,
  onRefreshExportStatus,
  onCancelExport,
  onRetryAudioPreview,
  onSelectAudioOutputDevice,
  onImportMaterial,
  onImportTemplateBundle,
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
  onSetSelectedSegmentRetime,
  onApplySelectedSegmentEffect,
  onUpdateSelectedSegmentEffectParameter,
  onRemoveSelectedSegmentEffect,
  onSetSelectedSegmentMask,
  onSetSelectedSegmentBlendMode,
  onAddSelectedSegmentTransition,
  onSelectPreviewTextOverlay,
  onEditPreviewTextOverlay,
  onSetSelectedSegmentKeyframe,
  onRemoveSelectedSegmentKeyframe,
  onSetSelectedSegmentVolume,
  onUpdateSelectedSegmentAudio,
  onSetSelectedTrackMute,
  onNavigateTemplateReportItem,
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
  const [topFeatureOverflowOpen, setTopFeatureOverflowOpen] = useState(false);
  const [previewTextEditFocusVersion, setPreviewTextEditFocusVersion] = useState(0);
  const overflowActive = OVERFLOW_WORKSPACE_CATEGORIES.includes(activeCategory);
  const handleEditPreviewTextOverlay = (selectionHandle: string): void => {
    setPreviewTextEditFocusVersion((current) => current + 1);
    onEditPreviewTextOverlay(selectionHandle);
  };
  const openExportModal = (): void => {
    void (async () => {
      try {
        await onSuspendRealtimePreviewSurface();
      } finally {
        setExportModalOpen(true);
      }
    })();
  };

  return (
    <main className="workspace" aria-label="剪映风格编辑工作区">
      <header className="product-titlebar" aria-label="项目标题栏">
        <div className="titlebar-status" aria-label="草稿保存状态">
          <span className="titlebar-window-controls" aria-hidden="true">
            <span className="titlebar-window-dot close" />
            <span className="titlebar-window-dot minimize" />
            <span className="titlebar-window-dot zoom" />
          </span>
          <strong>{formatAutosaveStatusLabel()}</strong>
        </div>
        <div className="workspace-title" aria-label="项目标题" title={workspace.viewModel.project.draftName}>
          {workspace.viewModel.project.draftName}
        </div>
        <div className="product-action-bar" aria-label="产品操作">
          <button type="button" className="top-export-button" aria-label="导出" onClick={openExportModal}>
            <span className="app-icon-mask" style={iconMaskStyle("topExport")} aria-hidden="true" />
            <span>导出</span>
          </button>
        </div>
      </header>
      <header className="top-feature-bar" aria-label="顶部功能区">
        <nav className="category-nav" aria-label="顶部功能区">
          {PRIMARY_WORKSPACE_CATEGORIES.map((category) => (
            <CategoryButton
              key={category}
              category={category}
              active={category === activeCategory}
              onSelect={(nextCategory) => {
                setTopFeatureOverflowOpen(false);
                onCategoryChange(nextCategory);
              }}
            />
          ))}
        </nav>
        <div className="top-feature-overflow-wrap">
          <button
            type="button"
            className={overflowActive ? "top-feature-overflow active" : "top-feature-overflow"}
            aria-label="更多功能"
            aria-haspopup="menu"
            aria-expanded={topFeatureOverflowOpen}
            title="更多功能"
            onClick={() => setTopFeatureOverflowOpen((current) => !current)}
          >
            <span className="app-icon-mask" style={iconMaskStyle("titlebarMenu")} aria-hidden="true" />
          </button>
          {topFeatureOverflowOpen ? (
            <div className="top-feature-overflow-menu" role="menu" aria-label="更多功能菜单">
              {OVERFLOW_WORKSPACE_CATEGORIES.map((category) => (
                <CategoryButton
                  key={category}
                  category={category}
                  active={category === activeCategory}
                  menuItem
                  onSelect={(nextCategory) => {
                    setTopFeatureOverflowOpen(false);
                    onCategoryChange(nextCategory);
                  }}
                />
              ))}
            </div>
          ) : null}
        </div>
      </header>

      <section className="material-panel" aria-label="素材面板">
        <FeaturePanel
          category={activeCategory}
          workspace={workspace}
          templateImportReport={templateImportReport}
          projectInteractions={projectInteractions}
          showDeveloperDiagnostics={showDeveloperDiagnostics}
          bundlePath={bundlePath}
          materialPath={materialPath}
          onBundlePathChange={onBundlePathChange}
          onMaterialPathChange={onMaterialPathChange}
          onImportMaterial={onImportMaterial}
          onImportTemplateBundle={onImportTemplateBundle}
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
          onApplySelectedSegmentEffect={onApplySelectedSegmentEffect}
          onAddSelectedSegmentTransition={onAddSelectedSegmentTransition}
          onNavigateTemplateReportItem={onNavigateTemplateReportItem}
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
          audioPending={workspace.pendingAudioCommand !== null}
          nativeSurfaceSuspended={exportModalOpen}
          playheadUs={playheadUs}
          timelineDurationUs={workspace.viewModel.timeline.duration}
          playbackRunning={playbackRunning}
          projectInteractions={projectInteractions}
          onRealtimePreviewHostStateChange={onRealtimePreviewHostStateChange}
          onPlayheadChange={onPlayheadChange}
          onTogglePlayback={onTogglePlayback}
          onStopPlayback={onStopPlayback}
          onProbeRuntimeCapabilities={onProbeRuntimeCapabilities}
          onRetryAudioPreview={onRetryAudioPreview}
          onSelectPreviewTextOverlay={onSelectPreviewTextOverlay}
          onEditPreviewTextOverlay={handleEditPreviewTextOverlay}
        />
      </section>

      <aside className="inspector-panel" aria-label="属性检查器">
        <Inspector
          workspace={workspace}
          playheadUs={playheadUs}
          textEditFocusRequest={previewTextEditFocusVersion}
          showDeveloperDiagnostics={showDeveloperDiagnostics}
          projectInteractions={projectInteractions}
          onEditSelectedText={onEditSelectedText}
          onUpdateDraftCanvasConfig={onUpdateDraftCanvasConfig}
          onUpdateSelectedSegmentVisual={onUpdateSelectedSegmentVisual}
          onSetSelectedSegmentRetime={onSetSelectedSegmentRetime}
          onApplySelectedSegmentEffect={onApplySelectedSegmentEffect}
          onUpdateSelectedSegmentEffectParameter={onUpdateSelectedSegmentEffectParameter}
          onRemoveSelectedSegmentEffect={onRemoveSelectedSegmentEffect}
          onSetSelectedSegmentMask={onSetSelectedSegmentMask}
          onSetSelectedSegmentBlendMode={onSetSelectedSegmentBlendMode}
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
          showDeveloperDiagnostics={showDeveloperDiagnostics}
          playheadUs={playheadUs}
          playbackRunning={playbackRunning}
          projectInteractions={projectInteractions}
          onPlayheadChange={onPlayheadChange}
          onTogglePlayback={onTogglePlayback}
          onStopPlayback={onStopPlayback}
          onSelectSegment={onSelectTimelineSegment}
          onSelectTrack={onSelectTimelineTrack}
          onAddSegment={onAddTimelineSegment}
          onAddTextSegment={onAddTextSegment}
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

function formatAutosaveStatusLabel(date = new Date()): string {
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  const seconds = String(date.getSeconds()).padStart(2, "0");
  return `${hours}:${minutes}:${seconds} 自动保存本地`;
}

function CategoryButton({
  category,
  active,
  menuItem = false,
  onSelect
}: {
  category: WorkspaceCategory;
  active: boolean;
  menuItem?: boolean;
  onSelect: (category: WorkspaceCategory) => void;
}): React.ReactElement {
  const metadata = WORKSPACE_CATEGORY_META[category];

  return (
    <button
      key={category}
      type="button"
      role={menuItem ? "menuitemradio" : undefined}
      className={`${menuItem ? "category-menu-button" : "category-button"}${active ? " active" : ""}`}
      aria-label={metadata.label}
      aria-pressed={menuItem ? undefined : active}
      aria-checked={menuItem ? active : undefined}
      title={metadata.label}
      onClick={() => onSelect(category)}
    >
      <span className="category-symbol app-icon-mask" style={iconMaskStyle(CATEGORY_ICON_NAMES[category])} aria-hidden="true" />
      <span className="category-label">{metadata.label}</span>
    </button>
  );
}

function iconMaskStyle(icon: AppIconName): CSSProperties {
  return { "--app-icon-url": `url("${appIconUrls[icon]}")` } as CSSProperties;
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
  const exportSettingsLocked = pending || exportCanCancel;
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
              disabled={exportSettingsLocked}
            />
          </label>

          <div className="export-modal-grid">
            <label className="export-modal-field">
              <span>分辨率</span>
              <select aria-label="分辨率" defaultValue="draft" disabled={exportSettingsLocked}>
                <option value="draft">跟随草稿</option>
                <option value="1080p">1080p</option>
                <option value="720p">720p</option>
              </select>
            </label>
            <label className="export-modal-field">
              <span>帧率</span>
              <select aria-label="帧率" defaultValue="draft" disabled={exportSettingsLocked}>
                <option value="draft">跟随草稿</option>
                <option value="30">30 fps</option>
                <option value="60">60 fps</option>
              </select>
            </label>
            <label className="export-modal-field">
              <span>视频码率</span>
              <select aria-label="视频码率" defaultValue="auto" disabled={exportSettingsLocked}>
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
                disabled={exportSettingsLocked}
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
              disabled={exportSettingsLocked}
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
                <select aria-label="编码格式" defaultValue="h264" disabled={exportSettingsLocked}>
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
                  disabled={exportSettingsLocked || !includeAudio}
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
            disabled={pending || exportCanCancel || !runtimeDiagnostics.canExport}
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

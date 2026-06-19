import {
  WORKSPACE_CATEGORIES,
  WORKSPACE_CATEGORY_META,
  artifactPreviewStatusLabel,
  getSelectedSegmentView,
  type WorkspaceCategory,
  type WorkspaceState
} from "../viewModel";
import type { ExportPreset } from "../../generated/CommandEnvelope";
import type { DraftCanvasConfig, KeyframeEasing, KeyframeInterpolation, KeyframeProperty, SegmentVisual } from "../../generated/Draft";
import { FeaturePanel } from "./FeaturePanel";
import { Inspector } from "./Inspector";
import { PreviewMonitor } from "./PreviewMonitor";
import { Timeline } from "./Timeline";

type WorkspaceShellProps = {
  workspace: WorkspaceState;
  activeCategory: WorkspaceCategory;
  showDeveloperDiagnostics: boolean;
  bundlePath: string;
  materialPath: string;
  playheadUs: number;
  playbackRunning: boolean;
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
  onSetSelectedTrackMute: Parameters<typeof FeaturePanel>[0]["onSetSelectedTrackMute"];
  onSelectTimelineSegment: Parameters<typeof Timeline>[0]["onSelectSegment"];
  onAddTimelineSegment: Parameters<typeof Timeline>[0]["onAddSegment"];
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
  onSetSelectedTrackMute,
  onSelectTimelineSegment,
  onAddTimelineSegment,
  onMoveSelectedSegment,
  onSplitSelectedSegment,
  onTrimSelectedSegment,
  onDeleteSelectedSegment,
  onSetTimelineTrackMute,
  onUndoTimelineEdit,
  onRedoTimelineEdit
}: WorkspaceShellProps): React.ReactElement {
  const selectedSegment = getSelectedSegmentView(workspace.draft, workspace.selection);

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
          onAddTextSegment={onAddTextSegment}
          onImportSubtitleSrt={onImportSubtitleSrt}
          onAddAudioSegment={onAddAudioSegment}
          onSetSelectedSegmentVolume={onSetSelectedSegmentVolume}
          onSetSelectedTrackMute={onSetSelectedTrackMute}
        />
      </section>

      <section className="preview-monitor" aria-label="预览窗口">
        <PreviewMonitor
          draftName={workspace.draft.metadata.name}
          canvasConfig={workspace.draft.canvasConfig}
          bindingStatus={workspace.bindingStatus}
          preview={workspace.preview}
          resourcePreviewStatusLabel={artifactPreviewStatusLabel(workspace.resourcePanel)}
          exportState={workspace.export}
          runtimeDiagnostics={workspace.runtimeDiagnostics}
          selectedSegment={selectedSegment}
          showDeveloperDiagnostics={showDeveloperDiagnostics}
          pending={workspace.pendingCommand !== null}
          playheadUs={playheadUs}
          playbackRunning={playbackRunning}
          onPlayheadChange={onPlayheadChange}
          onTogglePlayback={onTogglePlayback}
          onStopPlayback={onStopPlayback}
          onRequestPreviewFrame={onRequestPreviewFrame}
          onRequestPreviewSegment={onRequestPreviewSegment}
          onProbeRuntimeCapabilities={onProbeRuntimeCapabilities}
          onExportOutputPathChange={onExportOutputPathChange}
          onExportPresetChange={onExportPresetChange}
          onStartExport={onStartExport}
          onRefreshExportStatus={onRefreshExportStatus}
          onCancelExport={onCancelExport}
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
          onAddSegment={onAddTimelineSegment}
          onMoveSelectedSegment={onMoveSelectedSegment}
          onSplitSelectedSegment={onSplitSelectedSegment}
          onTrimSelectedSegment={onTrimSelectedSegment}
          onDeleteSelectedSegment={onDeleteSelectedSegment}
          onSetTrackMute={onSetTimelineTrackMute}
          onUndo={onUndoTimelineEdit}
          onRedo={onRedoTimelineEdit}
        />
      </section>
    </main>
  );
}

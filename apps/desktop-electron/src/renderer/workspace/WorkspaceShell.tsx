import { WORKSPACE_CATEGORIES, WORKSPACE_CATEGORY_META, type WorkspaceCategory, type WorkspaceState } from "../viewModel";
import type { ExportPreset } from "../../generated/CommandEnvelope";
import type { DraftCanvasConfig } from "../../generated/Draft";
import { FeaturePanel } from "./FeaturePanel";
import { Inspector } from "./Inspector";
import { PreviewMonitor } from "./PreviewMonitor";
import { Timeline } from "./Timeline";

type WorkspaceShellProps = {
  workspace: WorkspaceState;
  activeCategory: WorkspaceCategory;
  bundlePath: string;
  materialPath: string;
  playheadUs: number;
  onCategoryChange: (category: WorkspaceCategory) => void;
  onBundlePathChange: (value: string) => void;
  onMaterialPathChange: (value: string) => void;
  onPlayheadChange: (value: number) => void;
  onRequestPreviewFrame: () => void;
  onRequestPreviewSegment: () => void;
  onProbeRuntimeCapabilities: () => void;
  onExportOutputPathChange: (value: string) => void;
  onExportPresetChange: (value: ExportPreset) => void;
  onStartExport: () => void;
  onRefreshExportStatus: () => void;
  onCancelExport: () => void;
  onImportMaterial: () => void;
  onRefreshMaterials: () => void;
  onListMissingMaterials: () => void;
  onAddTextSegment: Parameters<typeof FeaturePanel>[0]["onAddTextSegment"];
  onAddAudioSegment: Parameters<typeof FeaturePanel>[0]["onAddAudioSegment"];
  onEditSelectedText: Parameters<typeof Inspector>[0]["onEditSelectedText"];
  onUpdateDraftCanvasConfig: (canvasConfig: DraftCanvasConfig) => void;
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
  bundlePath,
  materialPath,
  playheadUs,
  onCategoryChange,
  onBundlePathChange,
  onMaterialPathChange,
  onPlayheadChange,
  onRequestPreviewFrame,
  onRequestPreviewSegment,
  onProbeRuntimeCapabilities,
  onExportOutputPathChange,
  onExportPresetChange,
  onStartExport,
  onRefreshExportStatus,
  onCancelExport,
  onImportMaterial,
  onRefreshMaterials,
  onListMissingMaterials,
  onAddTextSegment,
  onAddAudioSegment,
  onEditSelectedText,
  onUpdateDraftCanvasConfig,
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
          bundlePath={bundlePath}
          materialPath={materialPath}
          onBundlePathChange={onBundlePathChange}
          onMaterialPathChange={onMaterialPathChange}
          onImportMaterial={onImportMaterial}
          onRefreshMaterials={onRefreshMaterials}
          onListMissingMaterials={onListMissingMaterials}
          onAddTextSegment={onAddTextSegment}
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
          exportState={workspace.export}
          runtimeDiagnostics={workspace.runtimeDiagnostics}
          pending={workspace.pendingCommand !== null}
          playheadUs={playheadUs}
          onPlayheadChange={onPlayheadChange}
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
          onEditSelectedText={onEditSelectedText}
          onUpdateDraftCanvasConfig={onUpdateDraftCanvasConfig}
          onSetSelectedSegmentVolume={onSetSelectedSegmentVolume}
          onSetSelectedTrackMute={onSetSelectedTrackMute}
        />
      </aside>

      <section className="timeline-panel" aria-label="时间线">
        <Timeline
          workspace={workspace}
          playheadUs={playheadUs}
          onPlayheadChange={onPlayheadChange}
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

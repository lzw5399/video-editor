import {
  formatMicroseconds,
  formatTrackKind,
  WORKSPACE_CATEGORIES,
  type WorkspaceCategory,
  type WorkspaceState
} from "../viewModel";
import { FeaturePanel } from "./FeaturePanel";
import { PreviewMonitor } from "./PreviewMonitor";

type WorkspaceShellProps = {
  workspace: WorkspaceState;
  activeCategory: WorkspaceCategory;
  bundlePath: string;
  materialPath: string;
  onCategoryChange: (category: WorkspaceCategory) => void;
  onBundlePathChange: (value: string) => void;
  onMaterialPathChange: (value: string) => void;
  onImportMaterial: () => void;
  onRefreshMaterials: () => void;
  onListMissingMaterials: () => void;
  onAddTextSegment: Parameters<typeof FeaturePanel>[0]["onAddTextSegment"];
  onAddAudioSegment: Parameters<typeof FeaturePanel>[0]["onAddAudioSegment"];
  onSetSelectedSegmentVolume: Parameters<typeof FeaturePanel>[0]["onSetSelectedSegmentVolume"];
  onSetSelectedTrackMute: Parameters<typeof FeaturePanel>[0]["onSetSelectedTrackMute"];
};

export function WorkspaceShell({
  workspace,
  activeCategory,
  bundlePath,
  materialPath,
  onCategoryChange,
  onBundlePathChange,
  onMaterialPathChange,
  onImportMaterial,
  onRefreshMaterials,
  onListMissingMaterials,
  onAddTextSegment,
  onAddAudioSegment,
  onSetSelectedSegmentVolume,
  onSetSelectedTrackMute
}: WorkspaceShellProps): React.ReactElement {
  return (
    <main className="workspace" aria-label="剪映风格编辑工作区">
      <header className="top-feature-bar" aria-label="顶部功能区">
        <h1 className="product-mark">视频剪辑</h1>
        <nav className="category-nav" aria-label="顶部功能区">
          {WORKSPACE_CATEGORIES.map((category) => (
            <button
              key={category}
              type="button"
              className={category === activeCategory ? "category-button active" : "category-button"}
              aria-pressed={category === activeCategory}
              onClick={() => onCategoryChange(category)}
            >
              {category}
            </button>
          ))}
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
        <PreviewMonitor draftName={workspace.draft.metadata.name} bindingStatus={workspace.bindingStatus} />
      </section>

      <aside className="inspector-panel" aria-label="属性检查器">
        <div className="panel-header">
          <h2>属性检查器</h2>
        </div>
        {workspace.selection.segmentIds.length === 0 ? (
          <div className="empty-state">
            <strong>未选择片段</strong>
            <span>在时间线中选择一个片段后，可在这里调整文字、音量和轨道状态。</span>
          </div>
        ) : (
          <dl className="inspector-list">
            <div>
              <dt>已选片段</dt>
              <dd>{workspace.selection.segmentIds.join("、")}</dd>
            </div>
          </dl>
        )}
        {workspace.commandError === null ? null : <p className="command-error">{workspace.commandError}</p>}
      </aside>

      <section className="timeline-panel" aria-label="时间线">
        <div className="timeline-toolbar">
          <span>时间线</span>
          <span>主轨磁吸已开启</span>
        </div>
        <div className="timeline-ruler">
          <span>00:00:00.000</span>
          <span>00:00:05.000</span>
          <span>00:00:10.000</span>
        </div>
        <div className="track-list">
          {workspace.draft.tracks.map((track) => (
            <div className={`track-row ${track.kind}`} key={track.trackId}>
              <div className="track-header">
                <strong>{track.name}</strong>
                <span>{formatTrackKind(track.kind)}轨道</span>
              </div>
              <div className="segment-lane">
                {track.segments.map((segment) => (
                  <div className="segment-block" key={segment.segmentId}>
                    <strong>{materialName(workspace.materials, segment.materialId)}</strong>
                    <span>{formatMicroseconds(segment.targetTimerange.duration)}</span>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </section>
    </main>
  );
}

function materialName(materials: Material[], materialId: string): string {
  return materials.find((material) => material.materialId === materialId)?.displayName ?? "素材";
}

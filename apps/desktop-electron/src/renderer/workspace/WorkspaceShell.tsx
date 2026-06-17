import type { Material } from "../../generated/Draft";
import {
  formatMaterialDetail,
  formatMaterialKind,
  formatMaterialStatus,
  formatMicroseconds,
  formatTrackKind,
  WORKSPACE_CATEGORIES,
  type WorkspaceCategory,
  type WorkspaceState
} from "../viewModel";
import { PreviewMonitor } from "./PreviewMonitor";

type WorkspaceShellProps = {
  workspace: WorkspaceState;
  activeCategory: WorkspaceCategory;
  onCategoryChange: (category: WorkspaceCategory) => void;
};

export function WorkspaceShell({
  workspace,
  activeCategory,
  onCategoryChange
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
        <div className="panel-header">
          <h2>{activeCategory}</h2>
          <button type="button" className="primary-action">
            导入素材
          </button>
        </div>
        {activeCategory === "媒体" ? <MaterialList materials={workspace.materials} /> : <DeferredPanel category={activeCategory} />}
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

function MaterialList({ materials }: { materials: Material[] }): React.ReactElement {
  if (materials.length === 0) {
    return (
      <div className="empty-state">
        <strong>还没有素材</strong>
        <span>导入视频、图片或音频后，可添加到时间线开始剪辑。</span>
      </div>
    );
  }

  return (
    <div className="material-list">
      {materials.map((material) => (
        <article className="material-row" aria-label={`素材 ${material.displayName}`} key={material.materialId}>
          <div className="material-thumb">{formatMaterialKind(material.kind)}</div>
          <div className="material-copy">
            <div className="material-title">
              <strong>{material.displayName}</strong>
              <span className={`material-status ${material.status}`}>{formatMaterialStatus(material.status)}</span>
            </div>
            <div className="material-metadata">
              <span>{formatMicroseconds(material.metadata.duration)}</span>
              <span>{formatMaterialDetail(material)}</span>
            </div>
          </div>
        </article>
      ))}
    </div>
  );
}

function DeferredPanel({ category }: { category: WorkspaceCategory }): React.ReactElement {
  return (
    <div className="empty-state">
      <strong>{category}面板已预留</strong>
      <span>后续阶段会接入对应的素材、效果与剪辑命令。</span>
    </div>
  );
}

function materialName(materials: Material[], materialId: string): string {
  return materials.find((material) => material.materialId === materialId)?.displayName ?? "素材";
}

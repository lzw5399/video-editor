import { useMemo, useState } from "react";

import type { Material, TextAlignment, TextSegment } from "../../generated/Draft";
import {
  findFirstMaterialByKind,
  findTrackByKind,
  formatMaterialDetail,
  formatMaterialDiagnostic,
  formatMaterialKind,
  formatMaterialStatus,
  formatMicroseconds,
  getSelectedSegmentView,
  getSelectedTrackView,
  materialStatusMessage,
  type WorkspaceCategory,
  type WorkspaceState
} from "../viewModel";

type FeaturePanelProps = {
  category: WorkspaceCategory;
  workspace: WorkspaceState;
  bundlePath: string;
  materialPath: string;
  onBundlePathChange: (value: string) => void;
  onMaterialPathChange: (value: string) => void;
  onImportMaterial: () => void;
  onRefreshMaterials: () => void;
  onListMissingMaterials: () => void;
  onAddTextSegment: (text: TextSegment, durationUs: number) => void;
  onAddAudioSegment: (materialId: string, durationUs: number) => void;
  onSetSelectedSegmentVolume: (levelMillis: number) => void;
  onSetSelectedTrackMute: (trackId: string, muted: boolean) => void;
};

export function FeaturePanel(props: FeaturePanelProps): React.ReactElement {
  if (props.category === "媒体") {
    return <MaterialPanel {...props} />;
  }

  if (props.category === "文字") {
    return <TextPanel {...props} />;
  }

  if (props.category === "音频") {
    return <AudioPanel {...props} />;
  }

  return <DeferredCategoryPanel category={props.category} />;
}

function MaterialPanel({
  workspace,
  bundlePath,
  materialPath,
  onBundlePathChange,
  onMaterialPathChange,
  onImportMaterial,
  onRefreshMaterials,
  onListMissingMaterials
}: FeaturePanelProps): React.ReactElement {
  return (
    <div className="feature-panel-content">
      <div className="panel-header">
        <h2>媒体</h2>
        <button type="button" className="primary-action" onClick={onImportMaterial} disabled={workspace.pendingCommand !== null}>
          导入素材
        </button>
      </div>

      <div className="field-stack">
        <label className="field-row">
          <span>草稿包路径</span>
          <input value={bundlePath} onChange={(event) => onBundlePathChange(event.currentTarget.value)} />
        </label>
        <label className="field-row">
          <span>素材路径</span>
          <input value={materialPath} onChange={(event) => onMaterialPathChange(event.currentTarget.value)} />
        </label>
        <div className="button-row">
          <button type="button" className="secondary-action" onClick={onRefreshMaterials}>
            刷新素材
          </button>
          <button type="button" className="secondary-action" onClick={onListMissingMaterials}>
            检查丢失素材
          </button>
        </div>
      </div>

      {workspace.materialDiagnostics.length === 0 ? null : (
        <div className="diagnostic-list" aria-label="素材诊断">
          {workspace.materialDiagnostics.map((diagnostic) => (
            <p key={`${diagnostic.materialId}-${diagnostic.kind}`}>{formatMaterialDiagnostic(diagnostic)}</p>
          ))}
        </div>
      )}

      <MaterialList materials={workspace.materials} />
    </div>
  );
}

function TextPanel({ workspace, onAddTextSegment }: FeaturePanelProps): React.ReactElement {
  const [content, setContent] = useState("输入文字");
  const [fontSize, setFontSize] = useState(36);
  const [color, setColor] = useState("#ffffff");
  const [alignment, setAlignment] = useState<TextAlignment>("center");
  const [strokeEnabled, setStrokeEnabled] = useState(true);
  const [strokeColor, setStrokeColor] = useState("#000000");
  const [strokeWidth, setStrokeWidth] = useState(2);
  const [shadowEnabled, setShadowEnabled] = useState(true);
  const [shadowColor, setShadowColor] = useState("#222222");
  const [backgroundEnabled, setBackgroundEnabled] = useState(false);
  const [backgroundColor, setBackgroundColor] = useState("#101010");
  const [durationSeconds, setDurationSeconds] = useState(3);
  const textTrack = findTrackByKind(workspace.draft, "text");

  const text: TextSegment = useMemo(
    () => ({
      content,
      style: {
        fontSize,
        color,
        alignment,
        stroke: strokeEnabled ? { color: strokeColor, width: strokeWidth } : null,
        shadow: shadowEnabled ? { color: shadowColor, offsetX: 2, offsetY: 2, blur: 4 } : null,
        background: backgroundEnabled ? { color: backgroundColor } : null
      }
    }),
    [
      alignment,
      backgroundColor,
      backgroundEnabled,
      color,
      content,
      fontSize,
      shadowColor,
      shadowEnabled,
      strokeColor,
      strokeEnabled,
      strokeWidth
    ]
  );

  return (
    <div className="feature-panel-content">
      <div className="panel-header">
        <h2>文字</h2>
        <button
          type="button"
          className="primary-action"
          onClick={() => onAddTextSegment(text, Math.max(1, durationSeconds) * 1_000_000)}
          disabled={workspace.pendingCommand !== null || textTrack === null}
        >
          添加文字
        </button>
      </div>

      <div className="field-stack">
        <label className="field-row">
          <span>文字内容</span>
          <textarea value={content} onChange={(event) => setContent(event.currentTarget.value)} />
        </label>
        <label className="field-row">
          <span>时长（秒）</span>
          <input type="number" min="1" value={durationSeconds} onChange={(event) => setDurationSeconds(event.currentTarget.valueAsNumber || 1)} />
        </label>
        <label className="field-row">
          <span>字号</span>
          <input type="number" min="1" value={fontSize} onChange={(event) => setFontSize(event.currentTarget.valueAsNumber || 1)} />
        </label>
        <label className="field-row">
          <span>颜色</span>
          <input type="color" value={color} onChange={(event) => setColor(event.currentTarget.value)} />
        </label>
        <div className="field-row">
          <span>对齐</span>
          <div className="segmented-control" role="group" aria-label="文字对齐">
            {(["left", "center", "right"] as const).map((value) => (
              <button
                key={value}
                type="button"
                className={alignment === value ? "active" : ""}
                onClick={() => setAlignment(value)}
              >
                {value === "left" ? "左" : value === "center" ? "中" : "右"}
              </button>
            ))}
          </div>
        </div>
        <label className="toggle-row">
          <input type="checkbox" checked={strokeEnabled} onChange={(event) => setStrokeEnabled(event.currentTarget.checked)} />
          <span>描边</span>
        </label>
        <label className="field-row">
          <span>描边颜色</span>
          <input type="color" value={strokeColor} onChange={(event) => setStrokeColor(event.currentTarget.value)} disabled={!strokeEnabled} />
        </label>
        <label className="field-row">
          <span>描边宽度</span>
          <input
            type="number"
            min="1"
            value={strokeWidth}
            onChange={(event) => setStrokeWidth(event.currentTarget.valueAsNumber || 1)}
            disabled={!strokeEnabled}
          />
        </label>
        <label className="toggle-row">
          <input type="checkbox" checked={shadowEnabled} onChange={(event) => setShadowEnabled(event.currentTarget.checked)} />
          <span>阴影</span>
        </label>
        <label className="field-row">
          <span>阴影颜色</span>
          <input type="color" value={shadowColor} onChange={(event) => setShadowColor(event.currentTarget.value)} disabled={!shadowEnabled} />
        </label>
        <label className="toggle-row">
          <input
            type="checkbox"
            checked={backgroundEnabled}
            onChange={(event) => setBackgroundEnabled(event.currentTarget.checked)}
          />
          <span>背景</span>
        </label>
        <label className="field-row">
          <span>背景颜色</span>
          <input
            type="color"
            value={backgroundColor}
            onChange={(event) => setBackgroundColor(event.currentTarget.value)}
            disabled={!backgroundEnabled}
          />
        </label>
      </div>
    </div>
  );
}

function AudioPanel({
  workspace,
  onAddAudioSegment,
  onSetSelectedSegmentVolume,
  onSetSelectedTrackMute
}: FeaturePanelProps): React.ReactElement {
  const audioMaterials = workspace.materials.filter((material) => material.kind === "audio" && material.status === "available");
  const firstAudioMaterial = findFirstMaterialByKind(workspace.draft, "audio");
  const [materialId, setMaterialId] = useState(firstAudioMaterial?.materialId ?? "");
  const [durationSeconds, setDurationSeconds] = useState(4);
  const [volume, setVolume] = useState(1000);
  const selectedSegment = getSelectedSegmentView(workspace.draft, workspace.selection);
  const selectedTrack = getSelectedTrackView(workspace.draft, workspace.selection);
  const audioTrack = findTrackByKind(workspace.draft, "audio");
  const selectedMaterialId = materialId || (audioMaterials[0]?.materialId ?? "");

  return (
    <div className="feature-panel-content">
      <div className="panel-header">
        <h2>音频</h2>
        <button
          type="button"
          className="primary-action"
          onClick={() => onAddAudioSegment(selectedMaterialId, Math.max(1, durationSeconds) * 1_000_000)}
          disabled={workspace.pendingCommand !== null || audioTrack === null || selectedMaterialId.length === 0}
        >
          添加音频
        </button>
      </div>

      <div className="field-stack">
        <label className="field-row">
          <span>BGM素材</span>
          <select value={selectedMaterialId} onChange={(event) => setMaterialId(event.currentTarget.value)}>
            {audioMaterials.map((material) => (
              <option key={material.materialId} value={material.materialId}>
                {material.displayName}
              </option>
            ))}
          </select>
        </label>
        <label className="field-row">
          <span>时长（秒）</span>
          <input type="number" min="1" value={durationSeconds} onChange={(event) => setDurationSeconds(event.currentTarget.valueAsNumber || 1)} />
        </label>
      </div>

      <div className="field-stack">
        <h3>音量与静音</h3>
        <label className="field-row">
          <span>音量（毫音量）</span>
          <input
            type="number"
            min="0"
            max="4000"
            step="50"
            value={volume}
            onChange={(event) => setVolume(event.currentTarget.valueAsNumber || 0)}
          />
        </label>
        <div className="button-row">
          <button
            type="button"
            className="secondary-action"
            onClick={() => onSetSelectedSegmentVolume(volume)}
            disabled={workspace.pendingCommand !== null || selectedSegment === null}
          >
            应用到所选片段
          </button>
          <button
            type="button"
            className="secondary-action"
            onClick={() => selectedTrack && onSetSelectedTrackMute(selectedTrack.trackId, !selectedTrack.muted)}
            disabled={workspace.pendingCommand !== null || selectedTrack === null}
          >
            {selectedTrack?.muted ? "取消轨道静音" : "轨道静音"}
          </button>
        </div>
      </div>
    </div>
  );
}

function DeferredCategoryPanel({ category }: { category: WorkspaceCategory }): React.ReactElement {
  return (
    <div className="feature-panel-content">
      <div className="panel-header">
        <h2>{category}</h2>
      </div>
      <div className="empty-state">
        <strong>{category}面板已预留</strong>
        <span>当前阶段暂不提供{category}编辑，后续会通过剪辑核心命令接入对应能力。</span>
      </div>
    </div>
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
      {materials.map((material) => {
        const statusMessage = materialStatusMessage(material);

        return (
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
              {statusMessage === null ? null : <p className="material-warning">{statusMessage}</p>}
            </div>
          </article>
        );
      })}
    </div>
  );
}

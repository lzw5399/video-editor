import { useEffect, useMemo, useState } from "react";

import type { TextAlignment, TextSegment } from "../../generated/Draft";
import { formatMicroseconds, getSelectedSegmentView, type WorkspaceState } from "../viewModel";

type InspectorProps = {
  workspace: WorkspaceState;
  onEditSelectedText: (text: TextSegment) => void;
  onSetSelectedSegmentVolume: (levelMillis: number) => void;
  onSetSelectedTrackMute: (trackId: string, muted: boolean) => void;
};

type TextFormState = {
  content: string;
  fontSize: number;
  color: string;
  alignment: TextAlignment;
  strokeColor: string;
  strokeWidth: number;
  strokeEnabled: boolean;
  shadowColor: string;
  shadowEnabled: boolean;
  backgroundColor: string;
  backgroundEnabled: boolean;
};

const DEFAULT_TEXT_STATE: TextFormState = {
  content: "",
  fontSize: 36,
  color: "#ffffff",
  alignment: "center",
  strokeColor: "#000000",
  strokeWidth: 2,
  strokeEnabled: false,
  shadowColor: "#222222",
  shadowEnabled: false,
  backgroundColor: "#101010",
  backgroundEnabled: false
};

export function Inspector({
  workspace,
  onEditSelectedText,
  onSetSelectedSegmentVolume,
  onSetSelectedTrackMute
}: InspectorProps): React.ReactElement {
  const selected = getSelectedSegmentView(workspace.draft, workspace.selection);
  const [textState, setTextState] = useState<TextFormState>(DEFAULT_TEXT_STATE);
  const [volume, setVolume] = useState(1000);

  useEffect(() => {
    if (selected === null) {
      setTextState(DEFAULT_TEXT_STATE);
      setVolume(1000);
      return;
    }

    setVolume(selected.segment.volume.levelMillis);

    if (selected.segment.text === null || selected.segment.text === undefined) {
      setTextState(DEFAULT_TEXT_STATE);
      return;
    }

    setTextState({
      content: selected.segment.text.content,
      fontSize: selected.segment.text.style.fontSize,
      color: selected.segment.text.style.color,
      alignment: selected.segment.text.style.alignment,
      strokeColor: selected.segment.text.style.stroke?.color ?? "#000000",
      strokeWidth: selected.segment.text.style.stroke?.width ?? 2,
      strokeEnabled: selected.segment.text.style.stroke !== null && selected.segment.text.style.stroke !== undefined,
      shadowColor: selected.segment.text.style.shadow?.color ?? "#222222",
      shadowEnabled: selected.segment.text.style.shadow !== null && selected.segment.text.style.shadow !== undefined,
      backgroundColor: selected.segment.text.style.background?.color ?? "#101010",
      backgroundEnabled:
        selected.segment.text.style.background !== null && selected.segment.text.style.background !== undefined
    });
  }, [
    selected?.segment.segmentId,
    selected?.segment.volume.levelMillis,
    selected?.segment.text?.content,
    selected?.segment.text?.style.fontSize,
    selected?.segment.text?.style.color,
    selected?.segment.text?.style.alignment,
    selected?.segment.text?.style.stroke !== null && selected?.segment.text?.style.stroke !== undefined,
    selected?.segment.text?.style.stroke?.color,
    selected?.segment.text?.style.stroke?.width,
    selected?.segment.text?.style.shadow !== null && selected?.segment.text?.style.shadow !== undefined,
    selected?.segment.text?.style.shadow?.color,
    selected?.segment.text?.style.background !== null && selected?.segment.text?.style.background !== undefined,
    selected?.segment.text?.style.background?.color
  ]);

  const text = useMemo<TextSegment>(
    () => ({
      content: textState.content,
      style: {
        fontSize: textState.fontSize,
        color: textState.color,
        alignment: textState.alignment,
        stroke: textState.strokeEnabled ? { color: textState.strokeColor, width: textState.strokeWidth } : null,
        shadow: textState.shadowEnabled
          ? { color: textState.shadowColor, offsetX: 2, offsetY: 2, blur: 4 }
          : null,
        background: textState.backgroundEnabled ? { color: textState.backgroundColor } : null
      }
    }),
    [textState]
  );

  if (selected === null) {
    return (
      <div className="inspector-content">
        <div className="panel-header">
          <h2>属性检查器</h2>
        </div>
        <div className="empty-state">
          <strong>未选择片段</strong>
          <span>在时间线中选择一个片段后，可在这里调整文字、音量和轨道状态。</span>
        </div>
        {workspace.commandError === null ? null : <p className="command-error">{workspace.commandError}</p>}
      </div>
    );
  }

  const hasText = selected.segment.text !== null && selected.segment.text !== undefined;

  return (
    <div className="inspector-content">
      <div className="panel-header">
        <h2>属性检查器</h2>
      </div>

      <dl className="inspector-list">
        <div>
          <dt>片段ID</dt>
          <dd>{selected.segment.segmentId}</dd>
        </div>
        <div>
          <dt>轨道</dt>
          <dd>
            {selected.track.name} / {selected.track.kindLabel}
          </dd>
        </div>
        <div>
          <dt>素材</dt>
          <dd>{selected.material?.displayName ?? selected.segment.materialId}</dd>
        </div>
        <div>
          <dt>源时间</dt>
          <dd>
            {formatMicroseconds(selected.segment.sourceTimerange.start)} /{" "}
            {formatMicroseconds(selected.segment.sourceTimerange.duration)}
          </dd>
        </div>
        <div>
          <dt>目标时间</dt>
          <dd>
            {formatMicroseconds(selected.segment.targetTimerange.start)} /{" "}
            {formatMicroseconds(selected.segment.targetTimerange.duration)}
          </dd>
        </div>
      </dl>

      <div className="field-stack">
        <h3>文字</h3>
        {hasText ? (
          <>
            <label className="field-row">
              <span>文字内容</span>
              <textarea
                value={textState.content}
                onChange={(event) => setTextState((current) => ({ ...current, content: event.currentTarget.value }))}
              />
            </label>
            <label className="field-row">
              <span>字号</span>
              <input
                type="number"
                min="1"
                value={textState.fontSize}
                onChange={(event) =>
                  setTextState((current) => ({ ...current, fontSize: event.currentTarget.valueAsNumber || 1 }))
                }
              />
            </label>
            <label className="field-row">
              <span>颜色</span>
              <input
                type="color"
                value={textState.color}
                onChange={(event) => setTextState((current) => ({ ...current, color: event.currentTarget.value }))}
              />
            </label>
            <div className="field-row">
              <span>对齐</span>
              <div className="segmented-control" role="group" aria-label="检查器文字对齐">
                {(["left", "center", "right"] as const).map((value) => (
                  <button
                    key={value}
                    type="button"
                    className={textState.alignment === value ? "active" : ""}
                    onClick={() => setTextState((current) => ({ ...current, alignment: value }))}
                  >
                    {value === "left" ? "左" : value === "center" ? "中" : "右"}
                  </button>
                ))}
              </div>
            </div>
            <label className="toggle-row">
              <input
                type="checkbox"
                checked={textState.strokeEnabled}
                onChange={(event) => setTextState((current) => ({ ...current, strokeEnabled: event.currentTarget.checked }))}
              />
              <span>描边</span>
            </label>
            <label className="field-row">
              <span>描边颜色</span>
              <input
                type="color"
                value={textState.strokeColor}
                disabled={!textState.strokeEnabled}
                onChange={(event) => setTextState((current) => ({ ...current, strokeColor: event.currentTarget.value }))}
              />
            </label>
            <label className="field-row">
              <span>描边宽度</span>
              <input
                type="number"
                min="1"
                value={textState.strokeWidth}
                disabled={!textState.strokeEnabled}
                onChange={(event) =>
                  setTextState((current) => ({ ...current, strokeWidth: event.currentTarget.valueAsNumber || 1 }))
                }
              />
            </label>
            <label className="toggle-row">
              <input
                type="checkbox"
                checked={textState.shadowEnabled}
                onChange={(event) => setTextState((current) => ({ ...current, shadowEnabled: event.currentTarget.checked }))}
              />
              <span>阴影</span>
            </label>
            <label className="field-row">
              <span>阴影颜色</span>
              <input
                type="color"
                value={textState.shadowColor}
                disabled={!textState.shadowEnabled}
                onChange={(event) => setTextState((current) => ({ ...current, shadowColor: event.currentTarget.value }))}
              />
            </label>
            <label className="toggle-row">
              <input
                type="checkbox"
                checked={textState.backgroundEnabled}
                onChange={(event) =>
                  setTextState((current) => ({ ...current, backgroundEnabled: event.currentTarget.checked }))
                }
              />
              <span>背景</span>
            </label>
            <label className="field-row">
              <span>背景颜色</span>
              <input
                type="color"
                value={textState.backgroundColor}
                disabled={!textState.backgroundEnabled}
                onChange={(event) =>
                  setTextState((current) => ({ ...current, backgroundColor: event.currentTarget.value }))
                }
              />
            </label>
            <button
              type="button"
              className="primary-action wide-action"
              onClick={() => onEditSelectedText(text)}
              disabled={workspace.pendingCommand !== null}
            >
              应用文字
            </button>
          </>
        ) : (
          <p className="inspector-note">当前片段没有文字语义。</p>
        )}
      </div>

      <div className="field-stack">
        <h3>音频</h3>
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
        <label className="toggle-row">
          <input
            type="checkbox"
            checked={selected.track.muted}
            onChange={(event) => onSetSelectedTrackMute(selected.track.trackId, event.currentTarget.checked)}
            disabled={workspace.pendingCommand !== null}
          />
          <span>轨道静音</span>
        </label>
        <button
          type="button"
          className="secondary-action wide-action"
          onClick={() => onSetSelectedSegmentVolume(volume)}
          disabled={workspace.pendingCommand !== null}
        >
          应用音量
        </button>
      </div>

      {workspace.commandError === null ? null : <p className="command-error">{workspace.commandError}</p>}
    </div>
  );
}

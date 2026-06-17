import { useEffect, useMemo, useState } from "react";

import type { TextAlignment, TextSegment } from "../../generated/Draft";
import { formatMicroseconds, getSelectedSegmentView, type WorkspaceState } from "../viewModel";

import "./preview-inspector.css";

type InspectorProps = {
  workspace: WorkspaceState;
  onEditSelectedText: (text: TextSegment) => void;
  onSetSelectedSegmentVolume: (levelMillis: number) => void;
  onSetSelectedTrackMute: (trackId: string, muted: boolean) => void;
};

type InspectorTab = "画面" | "音频" | "变速" | "动画" | "调节" | "AI效果";

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

const INSPECTOR_TABS: readonly InspectorTab[] = ["画面", "音频", "变速", "动画", "调节", "AI效果"];

export function Inspector({
  workspace,
  onEditSelectedText,
  onSetSelectedSegmentVolume,
  onSetSelectedTrackMute
}: InspectorProps): React.ReactElement {
  const selected = getSelectedSegmentView(workspace.draft, workspace.selection);
  const [activeTab, setActiveTab] = useState<InspectorTab>("画面");
  const [textState, setTextState] = useState<TextFormState>(DEFAULT_TEXT_STATE);
  const [volume, setVolume] = useState(1000);
  const sequenceDuration = getSequenceDuration(workspace);
  const hasText = selected?.segment.text !== null && selected?.segment.text !== undefined;

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

  return (
    <div className="inspector-content">
      <div className="panel-header">
        <h2>属性检查器</h2>
      </div>

      <div className="inspector-tabs" role="tablist" aria-label="检查器分类">
        {INSPECTOR_TABS.map((tab) => (
          <button
            key={tab}
            type="button"
            role="tab"
            aria-selected={activeTab === tab}
            className={activeTab === tab ? "active" : ""}
            onClick={() => setActiveTab(tab)}
          >
            {tab}
          </button>
        ))}
      </div>

      {selected === null ? (
        <>
          {activeTab === "画面" ? (
            <section className="inspector-section" aria-label="草稿参数" role="tabpanel">
              <div className="inspector-section-title">
                <h3>草稿参数</h3>
              </div>
              <div className="empty-state compact-empty">
                <strong>未选择片段</strong>
                <span>选择时间线片段后，可在这里调整画面、音频、文字和关键帧参数。</span>
              </div>
              <dl className="inspector-list compact">
                <InspectorDatum label="草稿名称" value={workspace.draft.metadata.name} />
                <InspectorDatum label="画布比例" value="16:9" />
                <InspectorDatum label="画布尺寸" value="1920 x 1080" />
                <InspectorDatum label="序列时长" value={formatMicroseconds(sequenceDuration)} />
                <InspectorDatum label="轨道数量" value={`${workspace.draft.tracks.length} 条`} />
                <InspectorDatum label="素材数量" value={`${workspace.draft.materials.length} 个`} />
                <InspectorDatum label="吸附状态" value={workspace.commandState.snapping.enabled ? "开启" : "关闭"} />
                <InspectorDatum label="核心状态" value={workspace.bindingStatus.label} />
              </dl>
            </section>
          ) : (
            <DeferredInspectorTab tab={activeTab} selected={false} />
          )}
          {workspace.commandError === null ? null : <p className="command-error">{workspace.commandError}</p>}
        </>
      ) : (
        <>
          {activeTab === "画面" ? (
            <div className="inspector-tab-panel" role="tabpanel" aria-label="画面参数">
              <section className="inspector-section" aria-label="片段信息">
                <div className="inspector-section-title">
                  <h3>片段参数</h3>
                  <KeyframeButton />
                </div>
                <dl className="inspector-list compact">
                  <InspectorDatum label="片段ID" value={selected.segment.segmentId} />
                  <InspectorDatum label="素材" value={selected.material?.displayName ?? selected.segment.materialId} />
                  <InspectorDatum label="轨道" value={`${selected.track.name} / ${selected.track.kindLabel}`} />
                  <InspectorDatum
                    label="源时间"
                    value={`${formatMicroseconds(selected.segment.sourceTimerange.start)} / ${formatMicroseconds(
                      selected.segment.sourceTimerange.duration
                    )}`}
                  />
                  <InspectorDatum
                    label="目标时间"
                    value={`${formatMicroseconds(selected.segment.targetTimerange.start)} / ${formatMicroseconds(
                      selected.segment.targetTimerange.duration
                    )}`}
                  />
                </dl>
              </section>

              <section className="inspector-section" aria-label="画面变换">
                <div className="inspector-section-title">
                  <h3>画面</h3>
                  <KeyframeButton />
                </div>
                <ShellControl label="位置" value="X 0 / Y 0" />
                <ShellControl label="缩放" value="100%" />
                <ShellControl label="旋转" value="0°" />
                <ShellControl label="不透明度" value="100%" />
              </section>

              <section className="inspector-section" aria-label="文字参数">
                <div className="inspector-section-title">
                  <h3>文字</h3>
                  <KeyframeButton />
                </div>
                {hasText ? (
                  <>
                    <label className="field-row compact-row textarea-row">
                      <span>文字内容</span>
                      <textarea
                        value={textState.content}
                        onChange={(event) => setTextState((current) => ({ ...current, content: event.currentTarget.value }))}
                      />
                    </label>
                    <label className="field-row compact-row">
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
                    <label className="field-row compact-row color-row">
                      <span>颜色</span>
                      <input
                        type="color"
                        value={textState.color}
                        onChange={(event) => setTextState((current) => ({ ...current, color: event.currentTarget.value }))}
                      />
                    </label>
                    <div className="field-row compact-row">
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
                    <label className="toggle-row compact-toggle">
                      <input
                        type="checkbox"
                        checked={textState.strokeEnabled}
                        onChange={(event) =>
                          setTextState((current) => ({ ...current, strokeEnabled: event.currentTarget.checked }))
                        }
                      />
                      <span>描边</span>
                    </label>
                    <label className="field-row compact-row color-row">
                      <span>描边颜色</span>
                      <input
                        type="color"
                        value={textState.strokeColor}
                        disabled={!textState.strokeEnabled}
                        onChange={(event) => setTextState((current) => ({ ...current, strokeColor: event.currentTarget.value }))}
                      />
                    </label>
                    <label className="field-row compact-row">
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
                    <label className="toggle-row compact-toggle">
                      <input
                        type="checkbox"
                        checked={textState.shadowEnabled}
                        onChange={(event) =>
                          setTextState((current) => ({ ...current, shadowEnabled: event.currentTarget.checked }))
                        }
                      />
                      <span>阴影</span>
                    </label>
                    <label className="field-row compact-row color-row">
                      <span>阴影颜色</span>
                      <input
                        type="color"
                        value={textState.shadowColor}
                        disabled={!textState.shadowEnabled}
                        onChange={(event) => setTextState((current) => ({ ...current, shadowColor: event.currentTarget.value }))}
                      />
                    </label>
                    <label className="toggle-row compact-toggle">
                      <input
                        type="checkbox"
                        checked={textState.backgroundEnabled}
                        onChange={(event) =>
                          setTextState((current) => ({ ...current, backgroundEnabled: event.currentTarget.checked }))
                        }
                      />
                      <span>背景</span>
                    </label>
                    <label className="field-row compact-row color-row">
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
              </section>
            </div>
          ) : null}

          {activeTab === "音频" ? (
            <section className="inspector-section" aria-label="音频参数" role="tabpanel">
              <div className="inspector-section-title">
                <h3>音频</h3>
                <KeyframeButton />
              </div>
              <label className="field-row compact-row">
                <span>音量</span>
                <input
                  type="range"
                  min="0"
                  max="4000"
                  step="50"
                  value={volume}
                  onChange={(event) => setVolume(event.currentTarget.valueAsNumber || 0)}
                />
              </label>
              <label className="field-row compact-row">
                <span>毫音量</span>
                <input
                  type="number"
                  min="0"
                  max="4000"
                  step="50"
                  value={volume}
                  onChange={(event) => setVolume(event.currentTarget.valueAsNumber || 0)}
                />
              </label>
              <label className="toggle-row compact-toggle">
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
            </section>
          ) : null}

          {activeTab !== "画面" && activeTab !== "音频" ? <DeferredInspectorTab tab={activeTab} selected /> : null}

          {workspace.commandError === null ? null : <p className="command-error">{workspace.commandError}</p>}
        </>
      )}
    </div>
  );
}

function DeferredInspectorTab({ tab, selected }: { tab: InspectorTab; selected: boolean }): React.ReactElement {
  return (
    <section className="inspector-section" aria-label={`${tab}参数`} role="tabpanel">
      <div className="inspector-section-title">
        <h3>{tab}</h3>
      </div>
      <div className="empty-state compact-empty">
        <strong>{tab}功能待接入</strong>
        <span>{selected ? `当前阶段暂不提供${tab}参数编辑。` : "选择时间线片段后，可查看对应参数。"}</span>
      </div>
    </section>
  );
}

function InspectorDatum({ label, value }: { label: string; value: string }): React.ReactElement {
  return (
    <div>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

function KeyframeButton(): React.ReactElement {
  return (
    <button
      type="button"
      className="keyframe-button"
      aria-label="关键帧功能待接入"
      title="关键帧功能待接入"
      disabled
    >
      <span aria-hidden="true">◇+</span>
    </button>
  );
}

function ShellControl({ label, value }: { label: string; value: string }): React.ReactElement {
  return (
    <div className="field-row compact-row shell-control-row">
      <span>{label}</span>
      <div className="shell-control">
        <input type="range" min="0" max="100" value="100" disabled readOnly aria-label={`${label}待接入`} />
        <input type="text" value={value} disabled readOnly aria-label={`${label}数值待接入`} />
      </div>
    </div>
  );
}

function getSequenceDuration(workspace: WorkspaceState): number {
  return Math.max(
    0,
    ...workspace.draft.tracks.flatMap((track) =>
      track.segments.map((segment) => segment.targetTimerange.start + segment.targetTimerange.duration)
    )
  );
}

import { useEffect, useMemo, useState } from "react";

import type {
  CanvasAspectRatioPreset,
  CanvasBackground,
  DraftCanvasConfig,
  TextAlignment,
  TextSegment
} from "../../generated/Draft";
import {
  canvasAspectRatioFromSize,
  canvasPresetLabel,
  formatCanvasAspectRatio,
  formatCanvasBackgroundStatus,
  formatCanvasFrameRate,
  formatCanvasReadout,
  formatCanvasSize,
  formatMicroseconds,
  getSelectedSegmentView,
  type WorkspaceState
} from "../viewModel";

import "./preview-inspector.css";

type InspectorProps = {
  workspace: WorkspaceState;
  onEditSelectedText: (text: TextSegment) => void;
  onUpdateDraftCanvasConfig: (canvasConfig: DraftCanvasConfig) => void;
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

type CanvasPresetChoice = CanvasAspectRatioPreset | "custom";

type CanvasFormState = {
  preset: CanvasPresetChoice;
  width: string;
  height: string;
  frameRate: string;
  backgroundKind: CanvasBackground["kind"];
  color: string;
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
const CANVAS_PRESETS: readonly CanvasPresetChoice[] = [
  "ratio16x9",
  "ratio9x16",
  "ratio1x1",
  "ratio4x3",
  "ratio3x4",
  "custom"
];
const CANVAS_PRESET_SIZES: Record<CanvasAspectRatioPreset, { width: number; height: number }> = {
  ratio16x9: { width: 1920, height: 1080 },
  ratio9x16: { width: 1080, height: 1920 },
  ratio1x1: { width: 1080, height: 1080 },
  ratio4x3: { width: 1440, height: 1080 },
  ratio3x4: { width: 1080, height: 1440 }
};
const CANVAS_FRAME_RATES = [24, 25, 30, 50, 60] as const;
const CANVAS_BACKGROUNDS: readonly { kind: CanvasBackground["kind"]; label: string }[] = [
  { kind: "black", label: "黑色" },
  { kind: "solidColor", label: "纯色" },
  { kind: "blurFill", label: "模糊填充" },
  { kind: "image", label: "图片背景" }
];

export function Inspector({
  workspace,
  onEditSelectedText,
  onUpdateDraftCanvasConfig,
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
            <CanvasDraftSettings
              workspace={workspace}
              sequenceDuration={sequenceDuration}
              onUpdateDraftCanvasConfig={onUpdateDraftCanvasConfig}
            />
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

function CanvasDraftSettings({
  workspace,
  sequenceDuration,
  onUpdateDraftCanvasConfig
}: {
  workspace: WorkspaceState;
  sequenceDuration: number;
  onUpdateDraftCanvasConfig: (canvasConfig: DraftCanvasConfig) => void;
}): React.ReactElement {
  const acceptedConfig = workspace.draft.canvasConfig;
  const [canvasState, setCanvasState] = useState<CanvasFormState>(() => canvasFormFromConfig(acceptedConfig));

  useEffect(() => {
    setCanvasState(canvasFormFromConfig(acceptedConfig));
  }, [acceptedConfig]);

  const candidate = buildCanvasConfigFromForm(canvasState);
  const validationMessage = validateCanvasForm(canvasState);
  const changed = candidate !== null && !canvasConfigsEqual(candidate, acceptedConfig);
  const pending = workspace.pendingCommand !== null;
  const canApply = candidate !== null && validationMessage === null && changed && !pending;
  const displayConfig = candidate ?? acceptedConfig;
  const backgroundStatus = formatCanvasBackgroundStatus(displayConfig);

  function selectPreset(preset: CanvasPresetChoice): void {
    if (preset === "custom") {
      setCanvasState((current) => ({ ...current, preset }));
      return;
    }

    const size = CANVAS_PRESET_SIZES[preset];
    setCanvasState((current) => ({
      ...current,
      preset,
      width: String(size.width),
      height: String(size.height)
    }));
  }

  function updateDimension(field: "width" | "height", value: string): void {
    setCanvasState((current) => ({
      ...current,
      preset: "custom",
      [field]: value
    }));
  }

  return (
    <section className="inspector-section canvas-settings-section" aria-label="草稿参数" role="tabpanel">
      <div className="inspector-section-title">
        <h3>草稿参数</h3>
      </div>
      <div className="empty-state compact-empty">
        <strong>未选择片段</strong>
        <span>这里显示草稿级画布参数。选择时间线片段后，可调整片段画面、音频、文字和关键帧参数。</span>
      </div>

      <div className="canvas-form" aria-label="画布参数表单">
        <div className="canvas-control-row">
          <span>画布比例</span>
          <div className="canvas-segmented" role="group" aria-label="画布比例">
            {CANVAS_PRESETS.map((preset) => {
              const label = preset === "custom" ? "自定义" : canvasPresetLabel(preset);
              return (
                <button
                  key={preset}
                  type="button"
                  className={canvasState.preset === preset ? "active" : ""}
                  aria-pressed={canvasState.preset === preset}
                  onClick={() => selectPreset(preset)}
                >
                  {label}
                </button>
              );
            })}
          </div>
        </div>

        <div className="canvas-control-row">
          <span>画布尺寸</span>
          <div className="canvas-size-fields">
            <label>
              <span>宽</span>
              <input
                aria-label="画布宽度"
                inputMode="numeric"
                type="number"
                min="1"
                step="1"
                value={canvasState.width}
                onChange={(event) => updateDimension("width", event.currentTarget.value)}
              />
            </label>
            <label>
              <span>高</span>
              <input
                aria-label="画布高度"
                inputMode="numeric"
                type="number"
                min="1"
                step="1"
                value={canvasState.height}
                onChange={(event) => updateDimension("height", event.currentTarget.value)}
              />
            </label>
          </div>
        </div>

        <label className="canvas-control-row">
          <span>帧率</span>
          <select
            aria-label="帧率"
            value={canvasState.frameRate}
            onChange={(event) => setCanvasState((current) => ({ ...current, frameRate: event.currentTarget.value }))}
          >
            {CANVAS_FRAME_RATES.map((frameRate) => (
              <option key={frameRate} value={String(frameRate)}>
                {frameRate} fps
              </option>
            ))}
          </select>
        </label>

        <div className="canvas-control-row">
          <span>画布背景</span>
          <div className="canvas-segmented background-modes" role="group" aria-label="画布背景">
            {CANVAS_BACKGROUNDS.map((background) => (
              <button
                key={background.kind}
                type="button"
                className={canvasState.backgroundKind === background.kind ? "active" : ""}
                aria-pressed={canvasState.backgroundKind === background.kind}
                onClick={() => setCanvasState((current) => ({ ...current, backgroundKind: background.kind }))}
              >
                {background.label}
              </button>
            ))}
          </div>
        </div>

        {canvasState.backgroundKind === "solidColor" ? (
          <label className="canvas-control-row canvas-color-row">
            <span>背景颜色</span>
            <span className="canvas-color-controls">
              <input
                aria-label="画布背景颜色"
                type="color"
                value={isHexColor(canvasState.color) ? canvasState.color : "#000000"}
                onChange={(event) => setCanvasState((current) => ({ ...current, color: event.currentTarget.value }))}
              />
              <input
                aria-label="画布背景色值"
                type="text"
                value={canvasState.color}
                onChange={(event) => setCanvasState((current) => ({ ...current, color: event.currentTarget.value }))}
              />
            </span>
          </label>
        ) : null}

        <div className={`canvas-background-status ${canvasBackgroundToneClass(displayConfig.background.kind)}`}>
          <span>{backgroundStatus}</span>
          {displayConfig.background.kind === "blurFill" ? <em>降级</em> : null}
          {displayConfig.background.kind === "image" ? <em>未接入</em> : null}
        </div>

        <button
          type="button"
          className="canvas-image-button"
          aria-label="图片背景未接入"
          title="图片背景未接入"
          disabled
        >
          图片背景 <span>未接入</span>
        </button>

        <p className="canvas-coordinate-help">坐标以画布中心为原点，X 向右，Y 向上</p>
        <p className="canvas-readout" aria-label="画布读数">
          {formatCanvasReadout(displayConfig)}
        </p>
        {validationMessage === null ? null : <p className="canvas-validation-error">{validationMessage}</p>}

        <button
          type="button"
          className="primary-action wide-action"
          disabled={!canApply}
          onClick={() => {
            if (candidate !== null && validationMessage === null) {
              onUpdateDraftCanvasConfig(candidate);
            }
          }}
        >
          应用草稿参数
        </button>
      </div>

      <dl className="inspector-list compact">
        <InspectorDatum label="草稿名称" value={workspace.draft.metadata.name} />
        <InspectorDatum label="画布比例" value={formatCanvasAspectRatio(acceptedConfig)} />
        <InspectorDatum label="画布尺寸" value={formatCanvasSize(acceptedConfig)} />
        <InspectorDatum label="帧率" value={formatCanvasFrameRate(acceptedConfig)} />
        <InspectorDatum label="画布背景" value={formatCanvasBackgroundStatus(acceptedConfig)} />
        <InspectorDatum label="序列时长" value={formatMicroseconds(sequenceDuration)} />
        <InspectorDatum label="轨道数量" value={`${workspace.draft.tracks.length} 条`} />
        <InspectorDatum label="素材数量" value={`${workspace.draft.materials.length} 个`} />
        <InspectorDatum label="吸附状态" value={workspace.commandState.snapping.enabled ? "开启" : "关闭"} />
        <InspectorDatum label="核心状态" value={workspace.bindingStatus.label} />
      </dl>
    </section>
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

function canvasFormFromConfig(config: DraftCanvasConfig): CanvasFormState {
  return {
    preset: config.aspectRatio.kind === "preset" ? config.aspectRatio.preset : "custom",
    width: String(config.width),
    height: String(config.height),
    frameRate: frameRateControlValue(config),
    backgroundKind: config.background.kind,
    color: config.background.kind === "solidColor" ? config.background.color : "#000000"
  };
}

function buildCanvasConfigFromForm(state: CanvasFormState): DraftCanvasConfig | null {
  const width = parsePositiveInteger(state.width);
  const height = parsePositiveInteger(state.height);
  const frameRate = parsePositiveInteger(state.frameRate);

  if (width === null || height === null || frameRate === null) {
    return null;
  }

  return {
    aspectRatio:
      state.preset === "custom"
        ? canvasAspectRatioFromSize(width, height)
        : {
            kind: "preset",
            preset: state.preset
          },
    width,
    height,
    frameRate: {
      numerator: frameRate,
      denominator: 1
    },
    background: canvasBackgroundFromForm(state)
  };
}

function canvasBackgroundFromForm(state: CanvasFormState): CanvasBackground {
  if (state.backgroundKind === "solidColor") {
    return {
      kind: "solidColor",
      color: state.color.trim()
    };
  }

  if (state.backgroundKind === "blurFill") {
    return { kind: "blurFill" };
  }

  if (state.backgroundKind === "image") {
    return {
      kind: "image",
      materialId: null
    };
  }

  return { kind: "black" };
}

function validateCanvasForm(state: CanvasFormState): string | null {
  const width = parsePositiveInteger(state.width);
  const height = parsePositiveInteger(state.height);
  const frameRate = parsePositiveInteger(state.frameRate);

  if (width === null || height === null) {
    return "画布尺寸必须是大于 0 的整数。";
  }

  if (frameRate === null) {
    return "帧率必须是大于 0 的整数。";
  }

  if (state.backgroundKind === "solidColor" && !isHexColor(state.color)) {
    return "纯色背景必须使用 #RRGGBB 色值。";
  }

  if (state.backgroundKind === "image") {
    return "图片背景素材选择未接入。";
  }

  return null;
}

function canvasConfigsEqual(left: DraftCanvasConfig, right: DraftCanvasConfig): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function frameRateControlValue(config: DraftCanvasConfig): string {
  const { numerator, denominator } = config.frameRate;
  const fps = denominator === 0 ? 30 : Math.round(numerator / denominator);
  return CANVAS_FRAME_RATES.includes(fps as (typeof CANVAS_FRAME_RATES)[number]) ? String(fps) : "30";
}

function parsePositiveInteger(value: string): number | null {
  if (!/^\d+$/.test(value.trim())) {
    return null;
  }

  const parsed = Number.parseInt(value, 10);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : null;
}

function isHexColor(value: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(value.trim());
}

function canvasBackgroundToneClass(kind: CanvasBackground["kind"]): "ready" | "warning" | "muted" {
  if (kind === "blurFill" || kind === "image") {
    return "warning";
  }

  return kind === "solidColor" ? "ready" : "muted";
}

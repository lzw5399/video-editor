import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import type {
  CanvasAspectRatioPreset,
  CanvasBackground,
  DraftCanvasConfig,
  Keyframe,
  KeyframeEasing,
  KeyframeInterpolation,
  KeyframeProperty,
  SegmentBackgroundFilling,
  SegmentFitMode,
  SegmentVisual,
  TextAlignment,
  TextSegment
} from "../../generated/Draft";
import type { SegmentVisualPatch, TextSegmentPatch } from "../../main/nativeBinding";
import {
  canvasAspectRatioFromSize,
  canvasPresetLabel,
  formatCanvasAspectRatio,
  formatCanvasBackgroundStatus,
  formatCanvasFrameRate,
  formatCanvasReadout,
  formatCanvasSize,
  formatKeyframeEasing,
  formatKeyframeInterpolation,
  formatKeyframeProperty,
  formatKeyframeValue,
  formatMicroseconds,
  type SelectedSegmentView,
  type WorkspaceState
} from "../viewModel";

import "./preview-inspector.css";

type InspectorProps = {
  workspace: WorkspaceState;
  playheadUs: number;
  textEditFocusRequest: number;
  showDeveloperDiagnostics: boolean;
  onEditSelectedText: (patch: TextSegmentPatch) => void;
  onUpdateDraftCanvasConfig: (canvasConfig: DraftCanvasConfig) => void;
  onUpdateSelectedSegmentVisual: (patch: SegmentVisualPatch) => void;
  onSetSelectedSegmentKeyframe: (
    property: KeyframeProperty,
    interpolation?: KeyframeInterpolation,
    easing?: KeyframeEasing
  ) => void;
  onRemoveSelectedSegmentKeyframe: (property: KeyframeProperty) => void;
  onSetSelectedSegmentVolume: (levelMillis: number) => void;
  onUpdateSelectedSegmentAudio: (options: {
    gainMillis: number;
    panBalanceMillis: number;
    fadeInDuration: number;
    fadeOutDuration: number;
  }) => void;
  onSetSelectedTrackMute: (itemHandle: string, muted: boolean) => void;
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
  fontFamily: string;
  fontRef: string | null;
  lineHeightMillis: number;
  letterSpacingMillis: number;
  textBoxWidthMillis: number;
  textBoxHeightMillis: number;
  layoutXMillis: number;
  layoutYMillis: number;
  layoutWidthMillis: number;
  layoutHeightMillis: number;
  wrapping: TextSegment["wrapping"];
  source: TextSegment["source"];
};

const BUNDLED_TEXT_FONTS = [
  {
    family: "Noto Sans CJK SC",
    fontRef: "font://bundled/noto-sans-cjk-sc-regular"
  },
  {
    family: "Noto Serif CJK SC",
    fontRef: "font://bundled/noto-serif-cjk-sc-regular"
  }
] as const;

type CanvasPresetChoice = CanvasAspectRatioPreset | "custom";

type CanvasFormState = {
  preset: CanvasPresetChoice;
  width: string;
  height: string;
  frameRatePreset: string;
  frameRateNumerator: string;
  frameRateDenominator: string;
  backgroundKind: CanvasBackground["kind"];
  color: string;
};

type VisualBackgroundChoice = SegmentBackgroundFilling["kind"];

type VisualFormState = {
  visible: boolean;
  positionX: string;
  positionY: string;
  scaleXMillis: string;
  scaleYMillis: string;
  rotationDegrees: string;
  opacityMillis: string;
  fitMode: SegmentFitMode;
  cropLeftMillis: string;
  cropRightMillis: string;
  cropTopMillis: string;
  cropBottomMillis: string;
  backgroundKind: VisualBackgroundChoice;
  backgroundColor: string;
};

type AudioEditOptions = {
  gainMillis: number;
  panBalanceMillis: number;
  fadeInDuration: number;
  fadeOutDuration: number;
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
  backgroundEnabled: false,
  fontFamily: BUNDLED_TEXT_FONTS[0].family,
  fontRef: BUNDLED_TEXT_FONTS[0].fontRef,
  lineHeightMillis: 1200,
  letterSpacingMillis: 0,
  textBoxWidthMillis: 800,
  textBoxHeightMillis: 200,
  layoutXMillis: 100,
  layoutYMillis: 100,
  layoutWidthMillis: 800,
  layoutHeightMillis: 800,
  wrapping: "auto",
  source: "text"
};

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
  { kind: "blurFill", label: "模糊填充" }
];
const FIT_MODE_LABELS: Record<SegmentFitMode, string> = {
  fit: "适应",
  fill: "填充",
  stretch: "拉伸"
};
const VISUAL_BACKGROUND_LABELS: Record<VisualBackgroundChoice, string> = {
  none: "无",
  black: "黑色",
  solidColor: "纯色",
  blur: "模糊",
  image: "图片"
};
const VISUAL_BACKGROUND_CHOICES: readonly VisualBackgroundChoice[] = ["none", "black", "solidColor", "blur", "image"];
const KEYFRAME_INTERPOLATIONS: readonly KeyframeInterpolation[] = ["hold", "linear"];
const KEYFRAME_EASINGS: readonly KeyframeEasing[] = ["none", "easeIn", "easeOut", "easeInOut"];
const TEXT_KEYFRAME_PROPERTIES: readonly KeyframeProperty[] = [
  "textFontSize",
  "textColor",
  "textLineHeight",
  "textLetterSpacing",
  "textLayoutX",
  "textLayoutY",
  "textLayoutWidth",
  "textLayoutHeight"
];
const KEYFRAME_PROPERTY_GROUPS: readonly {
  name: string;
  properties: readonly KeyframeProperty[];
}[] = [
  {
    name: "画面",
    properties: ["visualPositionX", "visualPositionY", "visualScaleX", "visualScaleY", "visualRotation", "visualOpacity"]
  },
  {
    name: "文本",
    properties: TEXT_KEYFRAME_PROPERTIES
  },
  {
    name: "音频",
    properties: ["volume"]
  }
];

export function Inspector({
  workspace,
  playheadUs,
  textEditFocusRequest,
  showDeveloperDiagnostics,
  onEditSelectedText,
  onUpdateDraftCanvasConfig,
  onUpdateSelectedSegmentVisual,
  onSetSelectedSegmentKeyframe,
  onRemoveSelectedSegmentKeyframe,
  onSetSelectedSegmentVolume,
  onUpdateSelectedSegmentAudio,
  onSetSelectedTrackMute
}: InspectorProps): React.ReactElement {
  const selected = workspace.viewModel.selectedSegment;
  const [activeTab, setActiveTab] = useState<InspectorTab>("画面");
  const [focusedKeyframeProperty, setFocusedKeyframeProperty] = useState<KeyframeProperty>("visualPositionX");
  const [textState, setTextState] = useState<TextFormState>(DEFAULT_TEXT_STATE);
  const [volumePercent, setVolumePercent] = useState(100);
  const [panPercent, setPanPercent] = useState(0);
  const [fadeInUs, setFadeInUs] = useState(0);
  const [fadeOutUs, setFadeOutUs] = useState(0);
  const textContentRef = useRef<HTMLTextAreaElement | null>(null);
  const textCommitKeyRef = useRef<string | null>(null);
  const audioCommitKeyRef = useRef<string | null>(null);
  const audioHydrationSelectionRef = useRef<string | null>(null);
  const sequenceDuration = workspace.viewModel.project.sequenceDuration;
  const hasText = selected?.text !== null && selected?.text !== undefined;
  const pendingKeyframe = workspace.pendingCommand === "设置关键帧" || workspace.pendingCommand === "删除关键帧";
  const inspectorFieldsDisabled = workspace.pendingCommand !== null;
  const visibleTabs = useMemo(() => inspectorTabsForSelection(selected), [selected]);
  const effectiveActiveTab =
    selected === null || visibleTabs.includes(activeTab) ? activeTab : visibleTabs[0];
  const renderKeyframeButton = (property: KeyframeProperty, label: string): React.ReactElement => (
    <KeyframeButton
      property={property}
      propertyLabel={label}
      selected={selected}
      playheadAt={playheadUs}
      pending={workspace.pendingCommand !== null}
      onSet={() => onSetSelectedSegmentKeyframe(property)}
      onRemove={() => onRemoveSelectedSegmentKeyframe(property)}
      onFocusProperty={() => {
        setFocusedKeyframeProperty(property);
        setActiveTab("动画");
      }}
    />
  );

  useEffect(() => {
    if (selected === null) {
      setTextState(DEFAULT_TEXT_STATE);
      textCommitKeyRef.current = null;
      audioCommitKeyRef.current = null;
      audioHydrationSelectionRef.current = null;
      setVolumePercent(100);
      setPanPercent(0);
      setFadeInUs(0);
      setFadeOutUs(0);
      return;
    }

    const selectedAudioOptions = audioOptionsFromSelected(selected);
    setVolumePercent(Math.round(selectedAudioOptions.gainMillis / 10));
    setPanPercent(Math.round(selectedAudioOptions.panBalanceMillis / 10));
    setFadeInUs(selectedAudioOptions.fadeInDuration);
    setFadeOutUs(selectedAudioOptions.fadeOutDuration);
    audioCommitKeyRef.current = audioOptionsKey(selectedAudioOptions);
    audioHydrationSelectionRef.current = selected.selectionHandle;

    if (selected.text === null || selected.text === undefined) {
      setTextState(DEFAULT_TEXT_STATE);
      textCommitKeyRef.current = null;
      return;
    }

    const nextTextState = {
      content: selected.text.content,
      fontSize: selected.text.style.fontSize,
      color: selected.text.style.color,
      alignment: selected.text.style.alignment,
      strokeColor: selected.text.style.stroke?.color ?? "#000000",
      strokeWidth: selected.text.style.stroke?.width ?? 2,
      strokeEnabled: selected.text.style.stroke !== null && selected.text.style.stroke !== undefined,
      shadowColor: selected.text.style.shadow?.color ?? "#222222",
      shadowEnabled: selected.text.style.shadow !== null && selected.text.style.shadow !== undefined,
      backgroundColor: selected.text.style.background?.color ?? "#101010",
      backgroundEnabled:
        selected.text.style.background !== null && selected.text.style.background !== undefined,
      fontFamily: selected.text.style.font.family,
      fontRef: selected.text.style.font.fontRef ?? null,
      lineHeightMillis: selected.text.style.lineHeightMillis,
      letterSpacingMillis: selected.text.style.letterSpacingMillis,
      textBoxWidthMillis: selected.text.textBox.widthMillis,
      textBoxHeightMillis: selected.text.textBox.heightMillis,
      layoutXMillis: selected.text.layoutRegion.xMillis,
      layoutYMillis: selected.text.layoutRegion.yMillis,
      layoutWidthMillis: selected.text.layoutRegion.widthMillis,
      layoutHeightMillis: selected.text.layoutRegion.heightMillis,
      wrapping: selected.text.wrapping,
      source: selected.text.source
    };
    setTextState(nextTextState);
    textCommitKeyRef.current = textPatchKey(textPatchFromState(nextTextState));
  }, [
    selected?.segmentKey,
    selected?.volume.levelMillis,
    selected?.audio?.gainMillis,
    selected?.audio?.panBalanceMillis,
    selected?.audio?.fadeInDuration.duration,
    selected?.audio?.fadeOutDuration.duration,
    selected?.text?.content,
    selected?.text?.style.fontSize,
    selected?.text?.style.color,
    selected?.text?.style.alignment,
    selected?.text?.style.stroke !== null && selected?.text?.style.stroke !== undefined,
    selected?.text?.style.stroke?.color,
    selected?.text?.style.stroke?.width,
    selected?.text?.style.shadow !== null && selected?.text?.style.shadow !== undefined,
    selected?.text?.style.shadow?.color,
    selected?.text?.style.background !== null && selected?.text?.style.background !== undefined,
    selected?.text?.style.background?.color,
    selected?.text?.style.font.family,
    selected?.text?.style.font.fontRef,
    selected?.text?.style.lineHeightMillis,
    selected?.text?.style.letterSpacingMillis,
    selected?.text?.textBox.widthMillis,
    selected?.text?.textBox.heightMillis,
    selected?.text?.layoutRegion.xMillis,
    selected?.text?.layoutRegion.yMillis,
    selected?.text?.layoutRegion.widthMillis,
    selected?.text?.layoutRegion.heightMillis,
    selected?.text?.wrapping,
    selected?.text?.source
  ]);

  useEffect(() => {
    if (selected === null) {
      return;
    }

    const nextTabs = inspectorTabsForSelection(selected);
    if (!nextTabs.includes(activeTab)) {
      setActiveTab(nextTabs[0]);
    }
  }, [activeTab, selected]);

  useEffect(() => {
    if (textEditFocusRequest <= 0 || selected?.text === null || selected?.text === undefined) {
      return;
    }
    setActiveTab("画面");
    window.requestAnimationFrame(() => {
      textContentRef.current?.focus();
      textContentRef.current?.select();
    });
  }, [selected?.selectionHandle, selected?.text, textEditFocusRequest]);

  const textPatch = useMemo<TextSegmentPatch>(() => textPatchFromState(textState), [textState]);
  const textValidationMessage = validateTextForm(textState);
  const audioOptions = useMemo(
    () => audioOptionsFromState(volumePercent, panPercent, fadeInUs, fadeOutUs),
    [fadeInUs, fadeOutUs, panPercent, volumePercent]
  );

  useEffect(() => {
    if (!hasText || selected?.text === null || selected?.text === undefined || textValidationMessage !== null) {
      return undefined;
    }
    if (workspace.pendingCommand !== null) {
      return undefined;
    }
    const nextKey = textPatchKey(textPatch);
    if (nextKey === textCommitKeyRef.current) {
      return undefined;
    }
    const timeout = window.setTimeout(() => {
      textCommitKeyRef.current = nextKey;
      onEditSelectedText(textPatch);
    }, 160);
    return () => window.clearTimeout(timeout);
  }, [hasText, onEditSelectedText, selected?.selectionHandle, selected?.text, textPatch, textValidationMessage, workspace.pendingCommand]);

  useEffect(() => {
    if (selected === null) {
      return undefined;
    }
    if (audioHydrationSelectionRef.current === selected.selectionHandle) {
      audioHydrationSelectionRef.current = null;
      return undefined;
    }
    if (workspace.pendingCommand !== null) {
      return undefined;
    }
    const nextKey = audioOptionsKey(audioOptions);
    if (nextKey === audioCommitKeyRef.current) {
      return undefined;
    }
    const timeout = window.setTimeout(() => {
      audioCommitKeyRef.current = nextKey;
      onUpdateSelectedSegmentAudio(audioOptions);
    }, 160);
    return () => window.clearTimeout(timeout);
  }, [audioOptions, onUpdateSelectedSegmentAudio, selected, workspace.pendingCommand]);

  return (
    <div className="inspector-content">
      <div className="panel-header">
        <h2>{selected === null ? "草稿参数" : "属性检查器"}</h2>
      </div>

      {selected === null ? null : (
        <div className="inspector-tabs" role="tablist" aria-label="检查器分类">
          {visibleTabs.map((tab) => (
            <button
              key={tab}
              type="button"
              role="tab"
              aria-selected={effectiveActiveTab === tab}
              className={effectiveActiveTab === tab ? "active" : ""}
              onClick={() => setActiveTab(tab)}
            >
              {tab}
            </button>
          ))}
        </div>
      )}

      {selected === null ? (
        <>
          <CanvasDraftSettings
            workspace={workspace}
            sequenceDuration={sequenceDuration}
            showDeveloperDiagnostics={showDeveloperDiagnostics}
            onUpdateDraftCanvasConfig={onUpdateDraftCanvasConfig}
          />
          {workspace.commandError === null ? null : <p className="command-error">{workspace.commandError}</p>}
        </>
      ) : (
        <>
          {effectiveActiveTab === "画面" ? (
            <div className="inspector-tab-panel" role="tabpanel" aria-label="画面参数">
              <section className="inspector-section" aria-label="片段信息">
                <div className="inspector-section-title">
                  <h3>片段参数</h3>
                </div>
                <dl className="inspector-list compact">
                  {showDeveloperDiagnostics ? <InspectorDatum label="片段ID" value={selected.segmentKey} /> : null}
                  <InspectorDatum label="素材" value={selected.material?.displayName ?? "未关联素材"} />
                  <InspectorDatum label="轨道" value={`${selected.track.name} / ${selected.track.kindLabel}`} />
                  <InspectorDatum
                    label="源时间"
                    value={`${formatMicroseconds(selected.sourceTimerange.start)} / ${formatMicroseconds(
                      selected.sourceTimerange.duration
                    )}`}
                  />
                  <InspectorDatum
                    label="目标时间"
                    value={`${formatMicroseconds(selected.targetTimerange.start)} / ${formatMicroseconds(
                      selected.targetTimerange.duration
                    )}`}
                  />
                </dl>
              </section>

              <section className="inspector-section" aria-label="画面变换">
                <div className="inspector-section-title">
                  <h3>基础</h3>
                  {renderKeyframeButton("visualPositionX", "位置 X")}
                </div>
                <SegmentVisualControls
                  visual={selected.visual}
                  pending={workspace.pendingCommand !== null}
                  renderKeyframeButton={renderKeyframeButton}
                  onUpdateVisual={onUpdateSelectedSegmentVisual}
                />
              </section>

              {hasText ? (
                <>
                  <section className="inspector-section" aria-label="文本">
                    <div className="inspector-section-title">
                      <h3>文本</h3>
                      {renderKeyframeButton("textFontSize", "字号")}
                    </div>
                    <label className="field-row compact-row textarea-row">
                      <span>文字内容</span>
                      <textarea
                        ref={textContentRef}
                        value={textState.content}
                        disabled={inspectorFieldsDisabled}
                        onChange={(event) => {
                          const content = event.currentTarget.value;
                          setTextState((current) => ({ ...current, content }));
                        }}
                      />
                    </label>
                    <dl className="inspector-list compact">
                      <InspectorDatum label="字幕来源" value={textState.source === "subtitle" ? "SRT 字幕" : "默认文字"} />
                    </dl>
                    <label className="field-row compact-row">
                      <span>字体</span>
                      <input
                        aria-label="字体"
                        list="inspector-bundled-fonts"
                        value={textState.fontFamily}
                        disabled={inspectorFieldsDisabled}
                        onChange={(event) => {
                          const fontFamily = event.currentTarget.value;
                          const font = BUNDLED_TEXT_FONTS.find((entry) => entry.family === fontFamily);
                          setTextState((current) => ({ ...current, fontFamily, fontRef: font?.fontRef ?? null }));
                        }}
                      />
                      <datalist id="inspector-bundled-fonts">
                        {BUNDLED_TEXT_FONTS.map((font) => (
                          <option key={font.fontRef} value={font.family} />
                        ))}
                      </datalist>
                    </label>
                    <TextNumberField
                      label="字号"
                      value={textState.fontSize}
                      min={1}
                      max={400}
                      step={1}
                      disabled={inspectorFieldsDisabled}
                      action={renderKeyframeButton("textFontSize", "字号")}
                      onChange={(fontSize) => setTextState((current) => ({ ...current, fontSize }))}
                    />
                    <label className="field-row compact-row color-row">
                      <span>颜色</span>
                      <span className="field-with-action">
                        <input
                          aria-label="颜色"
                          type="color"
                          value={textState.color}
                          disabled={inspectorFieldsDisabled}
                          onChange={(event) => {
                            const color = event.currentTarget.value;
                            setTextState((current) => ({ ...current, color }));
                          }}
                        />
                        {renderKeyframeButton("textColor", "颜色")}
                      </span>
                    </label>
                  </section>

                  <section className="inspector-section" aria-label="样式">
                    <div className="inspector-section-title">
                      <h3>样式</h3>
                    </div>
                    <label className="toggle-row compact-toggle">
                      <input
                        type="checkbox"
                        checked={textState.strokeEnabled}
                        disabled={inspectorFieldsDisabled}
                        onChange={(event) => {
                          const strokeEnabled = event.currentTarget.checked;
                          setTextState((current) => ({ ...current, strokeEnabled }));
                        }}
                      />
                      <span>描边</span>
                    </label>
                    <label className="field-row compact-row color-row">
                      <span>描边颜色</span>
                      <input
                        aria-label="描边颜色"
                        type="color"
                        value={textState.strokeColor}
                        disabled={inspectorFieldsDisabled || !textState.strokeEnabled}
                        onChange={(event) => {
                          const strokeColor = event.currentTarget.value;
                          setTextState((current) => ({ ...current, strokeColor }));
                        }}
                      />
                    </label>
                    <TextNumberField
                      label="描边宽度"
                      value={textState.strokeWidth}
                      min={1}
                      max={120}
                      step={1}
                      disabled={inspectorFieldsDisabled || !textState.strokeEnabled}
                      onChange={(strokeWidth) => setTextState((current) => ({ ...current, strokeWidth }))}
                    />
                    <label className="toggle-row compact-toggle">
                      <input
                        type="checkbox"
                        checked={textState.shadowEnabled}
                        disabled={inspectorFieldsDisabled}
                        onChange={(event) => {
                          const shadowEnabled = event.currentTarget.checked;
                          setTextState((current) => ({ ...current, shadowEnabled }));
                        }}
                      />
                      <span>阴影</span>
                    </label>
                    <label className="field-row compact-row color-row">
                      <span>阴影颜色</span>
                      <input
                        aria-label="阴影颜色"
                        type="color"
                        value={textState.shadowColor}
                        disabled={inspectorFieldsDisabled || !textState.shadowEnabled}
                        onChange={(event) => {
                          const shadowColor = event.currentTarget.value;
                          setTextState((current) => ({ ...current, shadowColor }));
                        }}
                      />
                    </label>
                    <label className="toggle-row compact-toggle">
                      <input
                        type="checkbox"
                        checked={textState.backgroundEnabled}
                        disabled={inspectorFieldsDisabled}
                        onChange={(event) => {
                          const backgroundEnabled = event.currentTarget.checked;
                          setTextState((current) => ({ ...current, backgroundEnabled }));
                        }}
                      />
                      <span>背景</span>
                    </label>
                    <label className="field-row compact-row color-row">
                      <span>背景颜色</span>
                      <input
                        aria-label="背景颜色"
                        type="color"
                        value={textState.backgroundColor}
                        disabled={inspectorFieldsDisabled || !textState.backgroundEnabled}
                        onChange={(event) => {
                          const backgroundColor = event.currentTarget.value;
                          setTextState((current) => ({ ...current, backgroundColor }));
                        }}
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
                            disabled={inspectorFieldsDisabled}
                            onClick={() => setTextState((current) => ({ ...current, alignment: value }))}
                          >
                            {value === "left" ? "左" : value === "center" ? "中" : "右"}
                          </button>
                        ))}
                      </div>
                    </div>
                  </section>

                  <section className="inspector-section" aria-label="文本框">
                    <div className="inspector-section-title">
                      <h3>文本框</h3>
                      {renderKeyframeButton("textLineHeight", "行高")}
                    </div>
                    <TextNumberField
                      label="宽度"
                      value={textState.textBoxWidthMillis}
                      min={1}
                      max={1000}
                      step={10}
                      disabled={inspectorFieldsDisabled}
                      onChange={(textBoxWidthMillis) => setTextState((current) => ({ ...current, textBoxWidthMillis }))}
                    />
                    <TextNumberField
                      label="高度"
                      value={textState.textBoxHeightMillis}
                      min={1}
                      max={1000}
                      step={10}
                      disabled={inspectorFieldsDisabled}
                      onChange={(textBoxHeightMillis) => setTextState((current) => ({ ...current, textBoxHeightMillis }))}
                    />
                    <label className="toggle-row compact-toggle">
                      <input
                        type="checkbox"
                        checked={textState.wrapping === "auto"}
                        disabled={inspectorFieldsDisabled}
                        onChange={(event) => {
                          const wrapping = event.currentTarget.checked ? "auto" : "none";
                          setTextState((current) => ({ ...current, wrapping }));
                        }}
                      />
                      <span>自动换行</span>
                    </label>
                    <TextNumberField
                      label="行高"
                      value={textState.lineHeightMillis}
                      min={500}
                      max={3000}
                      step={50}
                      disabled={inspectorFieldsDisabled}
                      action={renderKeyframeButton("textLineHeight", "行高")}
                      onChange={(lineHeightMillis) => setTextState((current) => ({ ...current, lineHeightMillis }))}
                    />
                    <TextNumberField
                      label="字间距"
                      value={textState.letterSpacingMillis}
                      min={0}
                      max={2000}
                      step={50}
                      disabled={inspectorFieldsDisabled}
                      action={renderKeyframeButton("textLetterSpacing", "字间距")}
                      onChange={(letterSpacingMillis) => setTextState((current) => ({ ...current, letterSpacingMillis }))}
                    />
                  </section>

                  <section className="inspector-section" aria-label="布局">
                    <div className="inspector-section-title">
                      <h3>布局</h3>
                      {renderKeyframeButton("textLayoutX", "布局 X")}
                    </div>
                    <p className="inspector-note">安全区域使用画布千分比坐标。</p>
                    <div className="text-layout-grid">
                      <TextNumberField
                        label="X"
                        value={textState.layoutXMillis}
                        min={0}
                        max={1000}
                        step={10}
                        disabled={inspectorFieldsDisabled}
                        action={renderKeyframeButton("textLayoutX", "布局 X")}
                        onChange={(layoutXMillis) => setTextState((current) => ({ ...current, layoutXMillis }))}
                      />
                      <TextNumberField
                        label="Y"
                        value={textState.layoutYMillis}
                        min={0}
                        max={1000}
                        step={10}
                        disabled={inspectorFieldsDisabled}
                        action={renderKeyframeButton("textLayoutY", "布局 Y")}
                        onChange={(layoutYMillis) => setTextState((current) => ({ ...current, layoutYMillis }))}
                      />
                      <TextNumberField
                        label="宽"
                        value={textState.layoutWidthMillis}
                        min={1}
                        max={1000}
                        step={10}
                        disabled={inspectorFieldsDisabled}
                        action={renderKeyframeButton("textLayoutWidth", "布局宽")}
                        onChange={(layoutWidthMillis) => setTextState((current) => ({ ...current, layoutWidthMillis }))}
                      />
                      <TextNumberField
                        label="高"
                        value={textState.layoutHeightMillis}
                        min={1}
                        max={1000}
                        step={10}
                        disabled={inspectorFieldsDisabled}
                        action={renderKeyframeButton("textLayoutHeight", "布局高")}
                        onChange={(layoutHeightMillis) => setTextState((current) => ({ ...current, layoutHeightMillis }))}
                      />
                    </div>
                    {textValidationMessage === null ? null : <p className="canvas-validation-error">{textValidationMessage}</p>}
                  </section>

                </>
              ) : (
                null
              )}
            </div>
          ) : null}

          {effectiveActiveTab === "音频" ? (
            <section className="inspector-section" aria-label="音频参数" role="tabpanel">
              <div className="inspector-section-title">
                <h3>音频</h3>
                {renderKeyframeButton("volume", "音量")}
              </div>
              <label className="field-row compact-row">
                <span>音量</span>
                <span className="field-with-action">
                  <input
                    aria-label="音量"
                    type="range"
                    min="0"
                    max="400"
                    step="5"
                    value={volumePercent}
                    onChange={(event) => setVolumePercent(toBoundedNumber(event.currentTarget.valueAsNumber, volumePercent, 0, 400))}
                  />
                  {renderKeyframeButton("volume", "音量")}
                </span>
              </label>
              <label className="field-row compact-row">
                <span>声像</span>
                <input
                  aria-label="声像"
                  type="range"
                  min="-100"
                  max="100"
                  step="5"
                  value={panPercent}
                  onChange={(event) => setPanPercent(toBoundedNumber(event.currentTarget.valueAsNumber, panPercent, -100, 100))}
                />
              </label>
              <label className="field-row compact-row">
                <span>淡入</span>
                <input
                  aria-label="淡入"
                  type="number"
                  min="0"
                  step="10000"
                  value={fadeInUs}
                  onChange={(event) => setFadeInUs(toBoundedNumber(event.currentTarget.valueAsNumber, fadeInUs, 0, 60_000_000))}
                />
              </label>
              <label className="field-row compact-row">
                <span>淡出</span>
                <input
                  aria-label="淡出"
                  type="number"
                  min="0"
                  step="10000"
                  value={fadeOutUs}
                  onChange={(event) => setFadeOutUs(toBoundedNumber(event.currentTarget.valueAsNumber, fadeOutUs, 0, 60_000_000))}
                />
              </label>
              <label className="toggle-row compact-toggle">
                <input
                  type="checkbox"
                  checked={selected.track.muted}
                  onChange={(event) => onSetSelectedTrackMute(selected.track.selectionHandle, event.currentTarget.checked)}
                  disabled={workspace.pendingCommand !== null}
                />
                <span>轨道静音</span>
              </label>
            </section>
          ) : null}

          {effectiveActiveTab === "动画" ? (
            <AnimationInspectorTab
              selected={selected}
              playheadAt={playheadUs}
              focusedProperty={focusedKeyframeProperty}
              pending={pendingKeyframe}
              onFocusProperty={setFocusedKeyframeProperty}
              onSetKeyframe={onSetSelectedSegmentKeyframe}
              onRemoveKeyframe={onRemoveSelectedSegmentKeyframe}
            />
          ) : null}

          {workspace.commandError === null ? null : <p className="command-error">{workspace.commandError}</p>}
        </>
      )}
    </div>
  );
}

function CanvasDraftSettings({
  workspace,
  sequenceDuration,
  showDeveloperDiagnostics,
  onUpdateDraftCanvasConfig
}: {
  workspace: WorkspaceState;
  sequenceDuration: number;
  showDeveloperDiagnostics: boolean;
  onUpdateDraftCanvasConfig: (canvasConfig: DraftCanvasConfig) => void;
}): React.ReactElement {
  const project = workspace.viewModel.project;
  const acceptedConfig = project.canvasConfig;
  const acceptedConfigKey = useMemo(() => JSON.stringify(acceptedConfig), [acceptedConfig]);
  const canvasCommitKeyRef = useRef<string>(acceptedConfigKey);
  const [canvasState, setCanvasState] = useState<CanvasFormState>(() => canvasFormFromConfig(acceptedConfig));
  const [modalOpen, setModalOpen] = useState(false);

  useEffect(() => {
    canvasCommitKeyRef.current = acceptedConfigKey;
    if (!modalOpen) {
      setCanvasState(canvasFormFromConfig(acceptedConfig));
    }
  }, [acceptedConfig, acceptedConfigKey, modalOpen]);

  const candidate = useMemo(() => buildCanvasConfigFromForm(canvasState), [canvasState]);
  const candidateKey = candidate === null ? null : JSON.stringify(candidate);
  const validationMessage = useMemo(() => validateCanvasForm(canvasState), [canvasState]);
  const changed = candidate !== null && !canvasConfigsEqual(candidate, acceptedConfig);
  const pending = workspace.pendingCommand !== null;
  const canFinish = validationMessage === null && !pending;
  const displayConfig = candidate ?? acceptedConfig;
  const backgroundStatus = formatCanvasBackgroundStatus(displayConfig);

  useEffect(() => {
    if (!modalOpen || candidate === null || candidateKey === null || validationMessage !== null || pending) {
      return undefined;
    }
    if (!changed || candidateKey === canvasCommitKeyRef.current) {
      return undefined;
    }
    const timeout = window.setTimeout(() => {
      canvasCommitKeyRef.current = candidateKey;
      onUpdateDraftCanvasConfig(candidate);
    }, 160);
    return () => window.clearTimeout(timeout);
  }, [candidate, candidateKey, changed, modalOpen, onUpdateDraftCanvasConfig, pending, validationMessage]);

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

  function updateFrameRatePart(field: "frameRateNumerator" | "frameRateDenominator", value: string): void {
    setCanvasState((current) => ({
      ...current,
      [field]: value
    }));
  }

  function updateCanvasColor(value: string): void {
    setCanvasState((current) => ({ ...current, color: value }));
  }

  function openModal(): void {
    setCanvasState(canvasFormFromConfig(acceptedConfig));
    setModalOpen(true);
  }

  function closeModal(): void {
    setCanvasState(canvasFormFromConfig(acceptedConfig));
    setModalOpen(false);
  }

  function finishModal(): void {
    if (
      candidate !== null &&
      candidateKey !== null &&
      validationMessage === null &&
      changed &&
      !pending &&
      candidateKey !== canvasCommitKeyRef.current
    ) {
      canvasCommitKeyRef.current = candidateKey;
      onUpdateDraftCanvasConfig(candidate);
    }
    setModalOpen(false);
  }

  return (
    <section
      className={
        showDeveloperDiagnostics
          ? "inspector-section canvas-settings-section developer-diagnostics"
          : "inspector-section canvas-settings-section product-draft-settings"
      }
      aria-label="草稿参数"
      role="tabpanel"
    >
      <div className="inspector-section-title">
        <h3>草稿参数</h3>
        <button type="button" className="secondary-action compact-action" onClick={openModal}>
          修改
        </button>
      </div>
      {showDeveloperDiagnostics ? (
        <div className="empty-state compact-empty">
          <strong>未选择片段</strong>
          <span>这里显示草稿级画布参数。选择时间线片段后，可调整片段画面、音频、文字和关键帧参数。</span>
        </div>
      ) : null}

      <dl className="inspector-list compact">
        <InspectorDatum label="草稿名称" value={project.draftName} />
        <InspectorDatum label="画布比例" value={formatCanvasAspectRatio(acceptedConfig)} />
        <InspectorDatum label="画布尺寸" value={formatCanvasSize(acceptedConfig)} />
        <InspectorDatum label="帧率" value={formatCanvasFrameRate(acceptedConfig)} />
        <InspectorDatum label="画布背景" value={formatCanvasBackgroundStatus(acceptedConfig)} />
        <InspectorDatum label="序列时长" value={formatMicroseconds(sequenceDuration)} />
        <InspectorDatum label="轨道数量" value={`${project.trackCount} 条`} />
        <InspectorDatum label="素材数量" value={`${project.materialCount} 个`} />
        <InspectorDatum label="吸附状态" value={workspace.viewModel.editControls.snappingEnabled ? "开启" : "关闭"} />
      </dl>

      {modalOpen ? (
        <div className="canvas-modal-backdrop">
          <div className="canvas-modal" role="dialog" aria-modal="true" aria-labelledby="canvas-modal-title">
            <div className="canvas-modal-header">
              <h3 id="canvas-modal-title">草稿参数</h3>
              <button type="button" className="canvas-modal-close" aria-label="关闭草稿参数" onClick={closeModal}>
                ×
              </button>
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
                <span className="canvas-frame-rate-controls">
                  <select
                    aria-label="帧率"
                    value={canvasState.frameRatePreset}
                    onChange={(event) => {
                      const nextFrameRate = event.currentTarget.value;
                      setCanvasState((current) => {
                        if (nextFrameRate === "custom") {
                          return { ...current, frameRatePreset: "custom" };
                        }

                        return {
                          ...current,
                          frameRatePreset: nextFrameRate,
                          frameRateNumerator: nextFrameRate,
                          frameRateDenominator: "1"
                        };
                      });
                    }}
                  >
                    {CANVAS_FRAME_RATES.map((frameRate) => (
                      <option key={frameRate} value={String(frameRate)}>
                        {frameRate} fps
                      </option>
                    ))}
                    <option value="custom">
                      {canvasState.frameRatePreset === "custom"
                        ? `当前 ${canvasState.frameRateNumerator}/${canvasState.frameRateDenominator} fps`
                        : "自定义"}
                    </option>
                  </select>
                  {canvasState.frameRatePreset === "custom" ? (
                    <span className="canvas-rate-fields">
                      <input
                        aria-label="帧率分子"
                        inputMode="numeric"
                        type="number"
                        min="1"
                        step="1"
                        value={canvasState.frameRateNumerator}
                        onChange={(event) => updateFrameRatePart("frameRateNumerator", event.currentTarget.value)}
                      />
                      <span aria-hidden="true">/</span>
                      <input
                        aria-label="帧率分母"
                        inputMode="numeric"
                        type="number"
                        min="1"
                        step="1"
                        value={canvasState.frameRateDenominator}
                        onChange={(event) => updateFrameRatePart("frameRateDenominator", event.currentTarget.value)}
                      />
                    </span>
                  ) : null}
                </span>
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
                      onChange={(event) => updateCanvasColor(event.currentTarget.value)}
                    />
                    <input
                      aria-label="画布背景色值"
                      type="text"
                      value={canvasState.color}
                      onChange={(event) => updateCanvasColor(event.currentTarget.value)}
                    />
                  </span>
                </label>
              ) : null}

              <div className={`canvas-background-status ${canvasBackgroundToneClass(displayConfig.background.kind)}`}>
                <span>{backgroundStatus}</span>
                {displayConfig.background.kind === "blurFill" ? <em>降级</em> : null}
              </div>

              <p className="canvas-coordinate-help">坐标以画布中心为原点，X 向右，Y 向上</p>
              <p className="canvas-readout" aria-label="画布读数">
                {formatCanvasReadout(displayConfig)}
              </p>
              {validationMessage === null ? null : <p className="canvas-validation-error">{validationMessage}</p>}
            </div>

            <div className="canvas-modal-actions">
              <button type="button" className="secondary-action" onClick={closeModal}>
                关闭
              </button>
              <button type="button" className="primary-action" disabled={!canFinish} onClick={finishModal}>
                完成
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}

function AnimationInspectorTab({
  selected,
  playheadAt,
  focusedProperty,
  pending,
  onFocusProperty,
  onSetKeyframe,
  onRemoveKeyframe
}: {
  selected: SelectedSegmentView | null;
  playheadAt: number;
  focusedProperty: KeyframeProperty;
  pending: boolean;
  onFocusProperty: (property: KeyframeProperty) => void;
  onSetKeyframe: (property: KeyframeProperty, interpolation?: KeyframeInterpolation, easing?: KeyframeEasing) => void;
  onRemoveKeyframe: (property: KeyframeProperty) => void;
}): React.ReactElement {
  if (selected === null) {
    return (
      <section className="inspector-section animation-panel" aria-label="动画参数" role="tabpanel">
        <div className="inspector-section-title">
          <h3>动画</h3>
        </div>
        <div className="empty-state compact-empty">
          <strong>未选择片段</strong>
          <span>选择时间线片段后，可查看动画参数和关键帧。</span>
        </div>
      </section>
    );
  }

  const visibleGroups = keyframeGroupsForSelection(selected);
  const visibleProperties = visibleGroups.flatMap((group) => [...group.properties]);
  const activeFocusedProperty = visibleProperties.includes(focusedProperty) ? focusedProperty : visibleProperties[0];
  const supportedFocused = isSupportedPropertyForSegment(selected, activeFocusedProperty);
  const focusedKeyframes = selected.keyframes.filter((keyframe) => keyframe.property === activeFocusedProperty);
  const segmentName = selected.material?.displayName ?? "未关联素材";

  return (
    <section className="inspector-section animation-panel" aria-label="动画参数" role="tabpanel">
      <div className="inspector-section-title">
        <h3>动画</h3>
      </div>

      <div className="animation-summary" aria-label="关键帧概览">
        <strong>{segmentName}</strong>
        <span>{selected.keyframes.length} 个关键帧</span>
        <span>播放头 {formatMicroseconds(playheadAt)}</span>
        <em>当前 {formatKeyframeProperty(activeFocusedProperty)}</em>
      </div>

      {pending ? <p className="keyframe-pending">关键帧命令处理中</p> : null}

      {selected.keyframes.length === 0 ? (
        <div className="empty-state compact-empty keyframe-empty">
          <strong>还没有关键帧</strong>
          <span>在画面、文本或音频参数旁点击菱形，可在当前播放头添加关键帧。</span>
        </div>
      ) : null}

      <div className="animation-property-groups" aria-label="属性关键帧">
        {visibleGroups.map((group) => (
          <div className="animation-property-group" key={group.name} aria-label={`${group.name}关键帧`}>
            <div className="animation-group-title">
              <strong>{group.name}</strong>
            </div>
            {group.properties.map((property) => {
              const count = selected.keyframes.filter((keyframe) => keyframe.property === property).length;
              const supported = isSupportedPropertyForSegment(selected, property);
              return (
                <button
                  key={property}
                  type="button"
                  className={activeFocusedProperty === property ? "animation-property-row active" : "animation-property-row"}
                  aria-label={`${formatKeyframeProperty(property)}关键帧`}
                  onClick={() => onFocusProperty(property)}
                >
                  <span>{formatKeyframeProperty(property)}</span>
                  <em>{supported ? `${count} 个` : "暂不支持"}</em>
                </button>
              );
            })}
          </div>
        ))}
      </div>

      <div className="animation-detail" aria-label={`${formatKeyframeProperty(activeFocusedProperty)}关键帧`}>
        <div className="animation-detail-title">
          <strong>{formatKeyframeProperty(activeFocusedProperty)}关键帧</strong>
          <KeyframeButton
            property={activeFocusedProperty}
            propertyLabel={formatKeyframeProperty(activeFocusedProperty)}
            selected={selected}
            playheadAt={playheadAt}
            pending={pending}
            onSet={() => onSetKeyframe(activeFocusedProperty)}
            onRemove={() => onRemoveKeyframe(activeFocusedProperty)}
            onFocusProperty={() => onFocusProperty(activeFocusedProperty)}
          />
        </div>
        {focusedKeyframes.length === 0 ? (
          <p className="inspector-note">当前属性还没有关键帧。</p>
        ) : (
          <div className="keyframe-row-list" aria-label="关键帧列表">
            {focusedKeyframes.map((keyframe) => (
              <KeyframeDetailRow
                key={`${keyframe.property}-${keyframe.at}`}
                keyframe={keyframe}
                active={selected.targetTimerange.start + keyframe.at === playheadAt}
                pending={pending}
                onRemove={() => onRemoveKeyframe(keyframe.property)}
              />
            ))}
          </div>
        )}

        <div className="animation-controls" aria-label="关键帧插值与缓动">
          <span>插值</span>
          <div className="segmented-control keyframe-segmented" role="group" aria-label="关键帧插值">
            {KEYFRAME_INTERPOLATIONS.map((interpolation) => (
              <button
                key={interpolation}
                type="button"
                onClick={() => onSetKeyframe(activeFocusedProperty, interpolation)}
                disabled={pending || !supportedFocused}
              >
                {formatKeyframeInterpolation(interpolation)}
              </button>
            ))}
          </div>
          <span>缓动</span>
          <div className="segmented-control keyframe-segmented" role="group" aria-label="关键帧缓动">
            {KEYFRAME_EASINGS.map((easing) => (
              <button
                key={easing}
                type="button"
                onClick={() => onSetKeyframe(activeFocusedProperty, "linear", easing)}
                disabled={pending || !supportedFocused}
              >
                {formatKeyframeEasing(easing)}
              </button>
            ))}
          </div>
        </div>
      </div>

    </section>
  );
}

function KeyframeDetailRow({
  keyframe,
  active,
  pending,
  onRemove
}: {
  keyframe: Keyframe;
  active: boolean;
  pending: boolean;
  onRemove: () => void;
}): React.ReactElement {
  const disabled = pending || !active;
  const label = active
    ? `删除${formatKeyframeProperty(keyframe.property)}关键帧`
    : `将播放头移动到${formatMicroseconds(keyframe.at)}后可删除`;

  return (
    <div className={active ? "keyframe-detail-row active" : "keyframe-detail-row"}>
      <span>{formatMicroseconds(keyframe.at)}</span>
      <span>{formatKeyframeValue(keyframe.value)}</span>
      <span>{formatKeyframeInterpolation(keyframe.interpolation)}</span>
      <span>{formatKeyframeEasing(keyframe.easing)}</span>
      <button type="button" onClick={onRemove} disabled={disabled} aria-label={label} title={label}>
        删除
      </button>
    </div>
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

function KeyframeButton({
  property,
  propertyLabel,
  selected = null,
  playheadAt = 0,
  pending = false,
  deferredLabel,
  onSet,
  onRemove,
  onFocusProperty
}: {
  property?: KeyframeProperty;
  propertyLabel?: string;
  selected?: SelectedSegmentView | null;
  playheadAt?: number;
  pending?: boolean;
  deferredLabel?: string;
  onSet?: () => void;
  onRemove?: () => void;
  onFocusProperty?: () => void;
}): React.ReactElement {
  if (deferredLabel !== undefined || property === undefined || propertyLabel === undefined) {
    const label = deferredLabel ?? "关键帧功能待接入";
    return (
      <button type="button" className="keyframe-button deferred" aria-label={label} title={label} disabled>
        <span aria-hidden="true">◇</span>
      </button>
    );
  }

  if (selected === null || !isSupportedPropertyForSegment(selected, property)) {
    const label = `${propertyLabel}关键帧暂不支持`;
    return (
      <button type="button" className="keyframe-button deferred" aria-label={label} title={label} disabled>
        <span aria-hidden="true">◇</span>
      </button>
    );
  }

  const propertyKeyframes = selected.keyframes.filter((keyframe) => keyframe.property === property);
  const activeKeyframe = propertyKeyframes.find(
    (keyframe) => selected.targetTimerange.start + keyframe.at === playheadAt
  );
  const disabled = pending;

  if (activeKeyframe !== undefined) {
    const label = `删除${propertyLabel}关键帧`;
    return (
      <button
        type="button"
        className="keyframe-button active"
        aria-label={label}
        title={label}
        disabled={disabled}
        onClick={() => onRemove?.()}
      >
        <span aria-hidden="true">◆</span>
      </button>
    );
  }

  if (propertyKeyframes.length > 0) {
    const label = `查看${propertyLabel}关键帧`;
    return (
      <button
        type="button"
        className="keyframe-button has-keyframes"
        aria-label={label}
        title={`已有${propertyKeyframes.length}个${propertyLabel}关键帧`}
        disabled={disabled}
        onClick={onFocusProperty}
      >
        <span aria-hidden="true">◇</span>
      </button>
    );
  }

  const label = `添加${propertyLabel}关键帧`;
  return (
    <button
      type="button"
      className="keyframe-button"
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onSet}
    >
      <span aria-hidden="true">◇+</span>
    </button>
  );
}

function TextNumberField({
  label,
  value,
  min,
  max,
  step,
  disabled = false,
  action,
  onChange
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  disabled?: boolean;
  action?: ReactNode;
  onChange: (value: number) => void;
}): React.ReactElement {
  return (
    <div className={action === undefined ? "field-row compact-row text-number-row" : "field-row compact-row text-number-row with-action"}>
      <span>{label}</span>
      <input
        aria-label={label}
        type="number"
        min={min}
        max={max}
        step={step}
        value={Number.isFinite(value) ? value : ""}
        disabled={disabled}
        onChange={(event) => onChange(event.currentTarget.valueAsNumber)}
      />
      {action}
    </div>
  );
}

function isSupportedPropertyForSegment(selected: SelectedSegmentView, property: KeyframeProperty): boolean {
  if (TEXT_KEYFRAME_PROPERTIES.includes(property)) {
    return selected.text !== null && selected.text !== undefined;
  }

  return true;
}

function inspectorTabsForSelection(selected: SelectedSegmentView | null): InspectorTab[] {
  if (selected === null) {
    return [];
  }

  const context = selectedSegmentContext(selected);

  if (context.hasAudioSemantics && !context.hasText && context.materialKind !== "video") {
    return ["音频", "动画"];
  }

  if (context.hasText) {
    return ["画面", "动画"];
  }

  if (context.hasAudioSemantics) {
    return ["画面", "音频", "动画"];
  }

  return ["画面", "动画"];
}

function keyframeGroupsForSelection(selected: SelectedSegmentView): typeof KEYFRAME_PROPERTY_GROUPS {
  const context = selectedSegmentContext(selected);

  return KEYFRAME_PROPERTY_GROUPS.filter((group) => {
    if (group.name === "文本") {
      return context.hasText;
    }

    if (group.name === "音频") {
      return context.hasAudioSemantics;
    }

    return true;
  });
}

function selectedSegmentContext(selected: SelectedSegmentView): {
  materialKind: string | undefined;
  hasText: boolean;
  hasAudioSemantics: boolean;
} {
  const materialKind = selected.material?.kind;
  const hasText = selected.hasText;
  const hasAudioSemantics = selected.hasAudioControls;

  return { materialKind, hasText, hasAudioSemantics };
}

function validateTextForm(state: TextFormState): string | null {
  if (state.content.trim().length === 0) {
    return "文字内容不能为空。";
  }

  if (state.fontFamily.trim().length === 0) {
    return "字体名称不能为空。";
  }

  if (!isIntegerInRange(state.fontSize, 1, 400)) {
    return "字号必须是 1 到 400 之间的整数。";
  }

  if (!isIntegerInRange(state.strokeWidth, 1, 120)) {
    return "描边宽度必须是 1 到 120 之间的整数。";
  }

  if (!isIntegerInRange(state.lineHeightMillis, 500, 3000)) {
    return "行高必须是 500 到 3000 之间的整数。";
  }

  if (!isIntegerInRange(state.letterSpacingMillis, 0, 2000)) {
    return "字间距必须是 0 到 2000 之间的整数。";
  }

  if (!isIntegerInRange(state.textBoxWidthMillis, 1, 1000) || !isIntegerInRange(state.textBoxHeightMillis, 1, 1000)) {
    return "文本框宽高必须是 1 到 1000 之间的整数。";
  }

  if (
    !isIntegerInRange(state.layoutXMillis, 0, 1000) ||
    !isIntegerInRange(state.layoutYMillis, 0, 1000) ||
    !isIntegerInRange(state.layoutWidthMillis, 1, 1000) ||
    !isIntegerInRange(state.layoutHeightMillis, 1, 1000)
  ) {
    return "布局安全区域必须使用 0 到 1000 之间的整数。";
  }

  if (state.layoutXMillis + state.layoutWidthMillis > 1000 || state.layoutYMillis + state.layoutHeightMillis > 1000) {
    return "布局安全区域不能超出画布范围。";
  }

  return null;
}

function textPatchFromState(state: TextFormState): TextSegmentPatch {
  return {
    content: state.content,
    fontFamily: state.fontFamily,
    ...(state.fontRef === null ? {} : { fontRef: state.fontRef }),
    fontSize: state.fontSize,
    color: state.color,
    alignment: state.alignment,
    lineHeightMillis: state.lineHeightMillis,
    letterSpacingMillis: state.letterSpacingMillis,
    strokeEnabled: state.strokeEnabled,
    strokeColor: state.strokeColor,
    strokeWidth: state.strokeWidth,
    shadowEnabled: state.shadowEnabled,
    shadowColor: state.shadowColor,
    backgroundEnabled: state.backgroundEnabled,
    backgroundColor: state.backgroundColor,
    textBoxWidthMillis: state.textBoxWidthMillis,
    textBoxHeightMillis: state.textBoxHeightMillis,
    layoutXMillis: state.layoutXMillis,
    layoutYMillis: state.layoutYMillis,
    layoutWidthMillis: state.layoutWidthMillis,
    layoutHeightMillis: state.layoutHeightMillis,
    wrapping: state.wrapping
  };
}

function textPatchKey(patch: TextSegmentPatch): string {
  return JSON.stringify(patch);
}

function audioOptionsFromSelected(selected: SelectedSegmentView): AudioEditOptions {
  return {
    gainMillis: selected.audio?.gainMillis ?? selected.volume.levelMillis,
    panBalanceMillis: selected.audio?.panBalanceMillis ?? 0,
    fadeInDuration: selected.audio?.fadeInDuration.duration ?? 0,
    fadeOutDuration: selected.audio?.fadeOutDuration.duration ?? 0
  };
}

function audioOptionsFromState(
  volumePercent: number,
  panPercent: number,
  fadeInUs: number,
  fadeOutUs: number
): AudioEditOptions {
  return {
    gainMillis: volumePercent * 10,
    panBalanceMillis: panPercent * 10,
    fadeInDuration: fadeInUs,
    fadeOutDuration: fadeOutUs
  };
}

function audioOptionsKey(options: AudioEditOptions): string {
  return JSON.stringify(options);
}

function isIntegerInRange(value: number, min: number, max: number): boolean {
  return Number.isSafeInteger(value) && value >= min && value <= max;
}

function SegmentVisualControls({
  visual,
  pending,
  renderKeyframeButton,
  onUpdateVisual
}: {
  visual: SegmentVisual;
  pending: boolean;
  renderKeyframeButton: (property: KeyframeProperty, label: string) => React.ReactElement;
  onUpdateVisual: (patch: SegmentVisualPatch) => void;
}): React.ReactElement {
  const visualKey = useMemo(() => JSON.stringify(visual), [visual]);
  const [visualState, setVisualState] = useState<VisualFormState>(() => visualFormFromVisual(visual));
  const visualCommitKeyRef = useRef<string | null>(null);

  useEffect(() => {
    const nextVisualState = visualFormFromVisual(visual);
    setVisualState(nextVisualState);
    const canonicalPatch = buildVisualPatchFromForm(nextVisualState);
    visualCommitKeyRef.current = canonicalPatch === null ? null : visualPatchKey(canonicalPatch);
  }, [visualKey]);

  const patch = buildVisualPatchFromForm(visualState);
  const validationMessage = validateVisualForm(visualState);
  const changed = patch !== null && visualPatchChangesVisual(visual, patch);

  useEffect(() => {
    if (patch === null || validationMessage !== null || !changed || pending) {
      return undefined;
    }
    const nextKey = visualPatchKey(patch);
    if (nextKey === visualCommitKeyRef.current) {
      return undefined;
    }
    const timeout = window.setTimeout(() => {
      visualCommitKeyRef.current = nextKey;
      onUpdateVisual(patch);
    }, 160);
    return () => window.clearTimeout(timeout);
  }, [changed, onUpdateVisual, patch, pending, validationMessage]);

  function updateVisualField(field: keyof VisualFormState, value: string | boolean): void {
    setVisualState((current) => ({ ...current, [field]: value }));
  }

  return (
    <div className="visual-controls" aria-label="画面基础表单">
      <label className="toggle-row compact-toggle visual-toggle-row">
        <input
          type="checkbox"
          checked={visualState.visible}
          onChange={(event) => updateVisualField("visible", event.currentTarget.checked)}
          disabled={pending}
        />
        <span>显示画面</span>
      </label>

      <VisualPairControl
        label="位置"
        firstLabel="X"
        secondLabel="Y"
        min={-1000}
        max={1000}
        step={10}
        firstValue={visualState.positionX}
        secondValue={visualState.positionY}
        disabled={pending}
        onFirstChange={(value) => updateVisualField("positionX", value)}
        onSecondChange={(value) => updateVisualField("positionY", value)}
        firstAction={renderKeyframeButton("visualPositionX", "位置 X")}
        secondAction={renderKeyframeButton("visualPositionY", "位置 Y")}
      />

      <VisualPairControl
        label="缩放"
        firstLabel="X"
        secondLabel="Y"
        min={1}
        max={3000}
        step={10}
        firstValue={visualState.scaleXMillis}
        secondValue={visualState.scaleYMillis}
        disabled={pending}
        onFirstChange={(value) => updateVisualField("scaleXMillis", value)}
        onSecondChange={(value) => updateVisualField("scaleYMillis", value)}
        firstAction={renderKeyframeButton("visualScaleX", "缩放 X")}
        secondAction={renderKeyframeButton("visualScaleY", "缩放 Y")}
      />

      <VisualSingleControl
        label="旋转"
        min={-360}
        max={360}
        step={1}
        value={visualState.rotationDegrees}
        disabled={pending}
        onChange={(value) => updateVisualField("rotationDegrees", value)}
        action={renderKeyframeButton("visualRotation", "旋转")}
      />

      <VisualSingleControl
        label="不透明度"
        min={0}
        max={1000}
        step={10}
        value={visualState.opacityMillis}
        disabled={pending}
        onChange={(value) => updateVisualField("opacityMillis", value)}
        action={renderKeyframeButton("visualOpacity", "不透明度")}
      />

      <div className="visual-control-row">
        <span>适应方式</span>
        <div className="visual-segmented" role="group" aria-label="适应方式">
          {(Object.keys(FIT_MODE_LABELS) as SegmentFitMode[]).map((fitMode) => (
            <button
              key={fitMode}
              type="button"
              className={visualState.fitMode === fitMode ? "active" : ""}
              aria-pressed={visualState.fitMode === fitMode}
              onClick={() => updateVisualField("fitMode", fitMode)}
              disabled={pending}
            >
              {FIT_MODE_LABELS[fitMode]}
            </button>
          ))}
        </div>
      </div>

      <div className="visual-control-row">
        <span>裁剪</span>
        <div className="visual-crop-grid" role="group" aria-label="裁剪">
          <VisualCompactNumber
            label="左"
            ariaLabel="裁剪 左"
            min={0}
            max={999}
            step={10}
            value={visualState.cropLeftMillis}
            disabled={pending}
            onChange={(value) => updateVisualField("cropLeftMillis", value)}
          />
          <VisualCompactNumber
            label="右"
            ariaLabel="裁剪 右"
            min={0}
            max={999}
            step={10}
            value={visualState.cropRightMillis}
            disabled={pending}
            onChange={(value) => updateVisualField("cropRightMillis", value)}
          />
          <VisualCompactNumber
            label="上"
            ariaLabel="裁剪 上"
            min={0}
            max={999}
            step={10}
            value={visualState.cropTopMillis}
            disabled={pending}
            onChange={(value) => updateVisualField("cropTopMillis", value)}
          />
          <VisualCompactNumber
            label="下"
            ariaLabel="裁剪 下"
            min={0}
            max={999}
            step={10}
            value={visualState.cropBottomMillis}
            disabled={pending}
            onChange={(value) => updateVisualField("cropBottomMillis", value)}
          />
        </div>
      </div>

      <div className="visual-control-row">
        <span>背景填充</span>
        <div className="visual-segmented background-fill" role="group" aria-label="背景填充">
          {VISUAL_BACKGROUND_CHOICES.map((backgroundKind) => (
            <button
              key={backgroundKind}
              type="button"
              className={visualState.backgroundKind === backgroundKind ? "active" : ""}
              aria-pressed={visualState.backgroundKind === backgroundKind}
              onClick={() => updateVisualField("backgroundKind", backgroundKind)}
              disabled={pending || backgroundKind === "image"}
            >
              {VISUAL_BACKGROUND_LABELS[backgroundKind]}
            </button>
          ))}
        </div>
      </div>

      {visualState.backgroundKind === "solidColor" ? (
        <label className="visual-control-row visual-color-row">
          <span>填充颜色</span>
          <span className="visual-color-controls">
            <input
              aria-label="背景填充颜色"
              type="color"
              value={isHexColor(visualState.backgroundColor) ? visualState.backgroundColor : "#000000"}
              onChange={(event) => updateVisualField("backgroundColor", event.currentTarget.value)}
              disabled={pending}
            />
            <input
              aria-label="背景填充色值"
              type="text"
              value={visualState.backgroundColor}
              onChange={(event) => updateVisualField("backgroundColor", event.currentTarget.value)}
              disabled={pending}
            />
          </span>
        </label>
      ) : null}

      {validationMessage === null ? null : <p className="canvas-validation-error">{validationMessage}</p>}

    </div>
  );
}

function VisualPairControl({
  label,
  firstLabel,
  secondLabel,
  min,
  max,
  step,
  firstValue,
  secondValue,
  disabled,
  onFirstChange,
  onSecondChange,
  firstAction,
  secondAction
}: {
  label: string;
  firstLabel: string;
  secondLabel: string;
  min: number;
  max: number;
  step: number;
  firstValue: string;
  secondValue: string;
  disabled: boolean;
  onFirstChange: (value: string) => void;
  onSecondChange: (value: string) => void;
  firstAction?: ReactNode;
  secondAction?: ReactNode;
}): React.ReactElement {
  return (
    <div className="visual-control-row" role="group" aria-label={label}>
      <span>{label}</span>
      <div className="visual-pair-grid">
        <VisualRangeNumber
          label={label}
          shortLabel={firstLabel}
          min={min}
          max={max}
          step={step}
          value={firstValue}
          disabled={disabled}
          onChange={onFirstChange}
          action={firstAction}
        />
        <VisualRangeNumber
          label={label}
          shortLabel={secondLabel}
          min={min}
          max={max}
          step={step}
          value={secondValue}
          disabled={disabled}
          onChange={onSecondChange}
          action={secondAction}
        />
      </div>
    </div>
  );
}

function VisualSingleControl({
  label,
  min,
  max,
  step,
  value,
  disabled,
  onChange,
  action
}: {
  label: string;
  min: number;
  max: number;
  step: number;
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
  action?: ReactNode;
}): React.ReactElement {
  return (
    <div className="visual-control-row" role="group" aria-label={label}>
      <span>{label}</span>
      <VisualRangeNumber
        label={label}
        shortLabel="数值"
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        onChange={onChange}
        action={action}
      />
    </div>
  );
}

function VisualRangeNumber({
  label,
  shortLabel,
  min,
  max,
  step,
  value,
  disabled,
  onChange,
  action
}: {
  label: string;
  shortLabel: string;
  min: number;
  max: number;
  step: number;
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
  action?: ReactNode;
}): React.ReactElement {
  const rangeValue = clamp(Number.parseInt(value, 10) || 0, min, max);
  const numberAriaLabel = shortLabel === "数值" ? label : `${label} ${shortLabel}`;

  return (
    <div className={action === undefined ? "visual-range-number" : "visual-range-number with-keyframe"}>
      <span>{shortLabel}</span>
      <input
        aria-label={`${numberAriaLabel}滑杆`}
        type="range"
        min={min}
        max={max}
        step={step}
        value={rangeValue}
        onChange={(event) => onChange(event.currentTarget.value)}
        disabled={disabled}
      />
      <input
        aria-label={numberAriaLabel}
        type="number"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(event) => onChange(event.currentTarget.value)}
        disabled={disabled}
      />
      {action}
    </div>
  );
}

function VisualCompactNumber({
  label,
  ariaLabel,
  min,
  max,
  step,
  value,
  disabled,
  onChange
}: {
  label: string;
  ariaLabel: string;
  min: number;
  max: number;
  step: number;
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
}): React.ReactElement {
  return (
    <label className="visual-compact-number">
      <span>{label}</span>
      <input
        aria-label={ariaLabel}
        type="number"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(event) => onChange(event.currentTarget.value)}
        disabled={disabled}
      />
    </label>
  );
}

function canvasFormFromConfig(config: DraftCanvasConfig): CanvasFormState {
  return {
    preset: config.aspectRatio.kind === "preset" ? config.aspectRatio.preset : "custom",
    width: String(config.width),
    height: String(config.height),
    frameRatePreset: frameRatePresetFromConfig(config),
    frameRateNumerator: String(config.frameRate.numerator),
    frameRateDenominator: String(config.frameRate.denominator),
    backgroundKind: config.background.kind,
    color: config.background.kind === "solidColor" ? config.background.color : "#000000"
  };
}

function buildCanvasConfigFromForm(state: CanvasFormState): DraftCanvasConfig | null {
  const width = parsePositiveInteger(state.width);
  const height = parsePositiveInteger(state.height);
  const frameRateNumerator = parsePositiveInteger(state.frameRateNumerator);
  const frameRateDenominator = parsePositiveInteger(state.frameRateDenominator);

  if (width === null || height === null || frameRateNumerator === null || frameRateDenominator === null) {
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
      numerator: frameRateNumerator,
      denominator: frameRateDenominator
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
  const frameRateNumerator = parsePositiveInteger(state.frameRateNumerator);
  const frameRateDenominator = parsePositiveInteger(state.frameRateDenominator);

  if (width === null || height === null) {
    return "画布尺寸必须是大于 0 的整数。";
  }

  if (frameRateNumerator === null || frameRateDenominator === null) {
    return "帧率分子和分母必须是大于 0 的整数。";
  }

  if (state.backgroundKind === "solidColor" && !isHexColor(state.color)) {
    return "纯色背景必须使用 #RRGGBB 色值。";
  }

  if (state.backgroundKind === "image") {
    return "图片背景素材选择未接入。";
  }

  return null;
}

function visualFormFromVisual(visual: SegmentVisual): VisualFormState {
  return {
    visible: visual.visible,
    positionX: String(visual.transform.position.x),
    positionY: String(visual.transform.position.y),
    scaleXMillis: String(visual.transform.scale.xMillis),
    scaleYMillis: String(visual.transform.scale.yMillis),
    rotationDegrees: String(visual.transform.rotation.degrees),
    opacityMillis: String(visual.transform.opacity.valueMillis),
    fitMode: visual.fitMode,
    cropLeftMillis: String(visual.transform.crop.leftMillis),
    cropRightMillis: String(visual.transform.crop.rightMillis),
    cropTopMillis: String(visual.transform.crop.topMillis),
    cropBottomMillis: String(visual.transform.crop.bottomMillis),
    backgroundKind: visual.backgroundFilling.kind,
    backgroundColor: visual.backgroundFilling.kind === "solidColor" ? visual.backgroundFilling.color : "#000000"
  };
}

function buildVisualPatchFromForm(state: VisualFormState): SegmentVisualPatch | null {
  const positionX = parseIntegerInRange(state.positionX, -1000, 1000);
  const positionY = parseIntegerInRange(state.positionY, -1000, 1000);
  const scaleXMillis = parseIntegerInRange(state.scaleXMillis, 1, 3000);
  const scaleYMillis = parseIntegerInRange(state.scaleYMillis, 1, 3000);
  const rotationDegrees = parseIntegerInRange(state.rotationDegrees, -360, 360);
  const opacityMillis = parseIntegerInRange(state.opacityMillis, 0, 1000);
  const cropLeftMillis = parseIntegerInRange(state.cropLeftMillis, 0, 999);
  const cropRightMillis = parseIntegerInRange(state.cropRightMillis, 0, 999);
  const cropTopMillis = parseIntegerInRange(state.cropTopMillis, 0, 999);
  const cropBottomMillis = parseIntegerInRange(state.cropBottomMillis, 0, 999);

  if (
    positionX === null ||
    positionY === null ||
    scaleXMillis === null ||
    scaleYMillis === null ||
    rotationDegrees === null ||
    opacityMillis === null ||
    cropLeftMillis === null ||
    cropRightMillis === null ||
    cropTopMillis === null ||
    cropBottomMillis === null
  ) {
    return null;
  }

  if (cropLeftMillis + cropRightMillis >= 1000 || cropTopMillis + cropBottomMillis >= 1000) {
    return null;
  }

  if (state.backgroundKind === "solidColor" && !isHexColor(state.backgroundColor)) {
    return null;
  }

  return {
    visible: state.visible,
    positionX,
    positionY,
    scaleXMillis,
    scaleYMillis,
    rotationDegrees,
    opacityMillis,
    cropLeftMillis,
    cropRightMillis,
    cropTopMillis,
    cropBottomMillis,
    fitMode: state.fitMode,
    backgroundKind: state.backgroundKind,
    ...(state.backgroundKind === "solidColor" ? { backgroundColor: state.backgroundColor.trim() } : {})
  };
}

function validateVisualForm(state: VisualFormState): string | null {
  if (
    parseIntegerInRange(state.positionX, -1000, 1000) === null ||
    parseIntegerInRange(state.positionY, -1000, 1000) === null
  ) {
    return "位置必须是 -1000 到 1000 之间的整数。";
  }

  if (
    parseIntegerInRange(state.scaleXMillis, 1, 3000) === null ||
    parseIntegerInRange(state.scaleYMillis, 1, 3000) === null
  ) {
    return "缩放必须是 1 到 3000 之间的整数。";
  }

  if (parseIntegerInRange(state.rotationDegrees, -360, 360) === null) {
    return "旋转必须是 -360 到 360 之间的整数角度。";
  }

  if (parseIntegerInRange(state.opacityMillis, 0, 1000) === null) {
    return "不透明度必须是 0 到 1000 之间的整数。";
  }

  const cropLeftMillis = parseIntegerInRange(state.cropLeftMillis, 0, 999);
  const cropRightMillis = parseIntegerInRange(state.cropRightMillis, 0, 999);
  const cropTopMillis = parseIntegerInRange(state.cropTopMillis, 0, 999);
  const cropBottomMillis = parseIntegerInRange(state.cropBottomMillis, 0, 999);

  if (
    cropLeftMillis === null ||
    cropRightMillis === null ||
    cropTopMillis === null ||
    cropBottomMillis === null
  ) {
    return "裁剪必须是 0 到 999 之间的整数。";
  }

  if (cropLeftMillis + cropRightMillis >= 1000 || cropTopMillis + cropBottomMillis >= 1000) {
    return "左右或上下裁剪总和必须小于 1000。";
  }

  if (state.backgroundKind === "solidColor" && !isHexColor(state.backgroundColor)) {
    return "背景填充纯色必须使用 #RRGGBB 色值。";
  }

  return null;
}

function canvasConfigsEqual(left: DraftCanvasConfig, right: DraftCanvasConfig): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function visualPatchChangesVisual(visual: SegmentVisual, patch: SegmentVisualPatch): boolean {
  return (
    patch.visible !== visual.visible ||
    patch.fitMode !== visual.fitMode ||
    patch.positionX !== visual.transform.position.x ||
    patch.positionY !== visual.transform.position.y ||
    patch.scaleXMillis !== visual.transform.scale.xMillis ||
    patch.scaleYMillis !== visual.transform.scale.yMillis ||
    patch.rotationDegrees !== visual.transform.rotation.degrees ||
    patch.opacityMillis !== visual.transform.opacity.valueMillis ||
    patch.cropLeftMillis !== visual.transform.crop.leftMillis ||
    patch.cropRightMillis !== visual.transform.crop.rightMillis ||
    patch.cropTopMillis !== visual.transform.crop.topMillis ||
    patch.cropBottomMillis !== visual.transform.crop.bottomMillis ||
    patch.backgroundKind !== visual.backgroundFilling.kind ||
    (patch.backgroundKind === "solidColor" &&
      visual.backgroundFilling.kind === "solidColor" &&
      patch.backgroundColor !== visual.backgroundFilling.color)
  );
}

function visualPatchKey(patch: SegmentVisualPatch): string {
  return JSON.stringify(patch);
}

function frameRatePresetFromConfig(config: DraftCanvasConfig): string {
  const { numerator, denominator } = config.frameRate;
  if (denominator === 1 && CANVAS_FRAME_RATES.includes(numerator as (typeof CANVAS_FRAME_RATES)[number])) {
    return String(numerator);
  }

  return "custom";
}

function parsePositiveInteger(value: string): number | null {
  if (!/^\d+$/.test(value.trim())) {
    return null;
  }

  const parsed = Number.parseInt(value, 10);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : null;
}

function parseIntegerInRange(value: string, min: number, max: number): number | null {
  if (!/^-?\d+$/.test(value.trim())) {
    return null;
  }

  const parsed = Number.parseInt(value, 10);
  return Number.isSafeInteger(parsed) && parsed >= min && parsed <= max ? parsed : null;
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

function toBoundedNumber(value: number, fallback: number, min: number, max: number): number {
  const rounded = Math.round(Number.isFinite(value) ? value : fallback);
  return clamp(rounded, min, max);
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

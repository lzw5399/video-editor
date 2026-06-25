import { useEffect, useMemo, useRef, useState, type ReactNode, type PointerEvent as ReactPointerEvent } from "react";

import type {
  CanvasAspectRatioPreset,
  CanvasBackground,
  CapabilityReportItem,
  CapabilitySupport,
  DraftCanvasConfig,
  EffectParameterUpdate,
  Filter,
  Keyframe,
  KeyframeEasing,
  KeyframeInterpolation,
  KeyframeProperty,
  KeyframeValue,
  SegmentBlendMode,
  SegmentBackgroundFilling,
  SegmentFitMode,
  SegmentMask,
  SegmentRetiming,
  SegmentVisual,
  TextAlignment,
  TextSegment
} from "../../generated/Draft";
import type { ProjectInteractionPayload, SegmentVisualPatch, TextSegmentPatch } from "../../main/nativeBinding";
import type { ProjectInteractionController } from "./projectInteraction";
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
  projectInteractions: ProjectInteractionController;
  onEditSelectedText: (patch: TextSegmentPatch) => void;
  onUpdateDraftCanvasConfig: (canvasConfig: DraftCanvasConfig) => void;
  onUpdateSelectedSegmentVisual: (patch: SegmentVisualPatch) => void;
  onSetSelectedSegmentRetime: (retiming: SegmentRetiming) => void;
  onApplySelectedSegmentEffect: (effect: Filter) => void;
  onUpdateSelectedSegmentEffectParameter: (effectIndex: number, parameter: EffectParameterUpdate) => void;
  onRemoveSelectedSegmentEffect: (effectIndex: number) => void;
  onSetSelectedSegmentMask: (mask: SegmentMask) => void;
  onSetSelectedSegmentBlendMode: (blendMode: SegmentBlendMode) => void;
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

type InspectorTab = "画面" | "音频" | "变速" | "动画" | "效果" | "滤镜" | "调节" | "蒙版" | "混合";

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

type VisualDisplayTransform = {
  min: number;
  max: number;
  step: number;
  suffix: string;
  toDisplay: (internalValue: string) => string;
  fromDisplay: (displayValue: string) => string;
};

type AudioEditOptions = {
  gainMillis: number;
  panBalanceMillis: number;
  fadeInDuration: number;
  fadeOutDuration: number;
};

type VisualInteractionState = {
  kind: "selectedSegmentVisual" | "keyframeEdit";
  interactionId: string | null;
  sequence: number;
  beginPromise: Promise<void>;
  updateInFlight: boolean;
  rafId: number | null;
  pendingPayload: ProjectInteractionPayload | null;
};

type TextInteractionState = {
  interactionId: string | null;
  sequence: number;
  beginPromise: Promise<void>;
  updateInFlight: boolean;
  rafId: number | null;
  pendingPatch: TextSegmentPatch | null;
};

type AudioInteractionState = {
  interactionId: string | null;
  sequence: number;
  beginPromise: Promise<void>;
  updateInFlight: boolean;
  rafId: number | null;
  pendingOptions: AudioEditOptions | null;
};

type ProductionEffectInteractionState = {
  kind: "selectedSegmentRetime" | "selectedSegmentEffect" | "selectedSegmentMask" | "selectedSegmentBlend";
  interactionId: string | null;
  sequence: number;
  beginPromise: Promise<void>;
  updateInFlight: boolean;
  finishing: boolean;
  rafId: number | null;
  pendingPayload: ProjectInteractionPayload | null;
  cleanupListeners: (() => void) | null;
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

const PERCENT_VALUE_DISPLAY: VisualDisplayTransform = {
  min: 0.1,
  max: 300,
  step: 1,
  suffix: "%",
  toDisplay: millisStringToPercentString,
  fromDisplay: percentStringToMillisString
};
const OPACITY_PERCENT_DISPLAY: VisualDisplayTransform = {
  min: 0,
  max: 100,
  step: 1,
  suffix: "%",
  toDisplay: millisStringToPercentString,
  fromDisplay: percentStringToMillisString
};
const CROP_PERCENT_DISPLAY: VisualDisplayTransform = {
  min: 0,
  max: 99.9,
  step: 0.1,
  suffix: "%",
  toDisplay: millisStringToPercentString,
  fromDisplay: percentStringToMillisString
};
const DEGREE_DISPLAY: VisualDisplayTransform = {
  min: -360,
  max: 360,
  step: 1,
  suffix: "°",
  toDisplay: (value) => value,
  fromDisplay: (value) => value
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
  projectInteractions,
  onEditSelectedText,
  onUpdateDraftCanvasConfig,
  onUpdateSelectedSegmentVisual,
  onSetSelectedSegmentRetime,
  onApplySelectedSegmentEffect,
  onUpdateSelectedSegmentEffectParameter,
  onRemoveSelectedSegmentEffect,
  onSetSelectedSegmentMask,
  onSetSelectedSegmentBlendMode,
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
  const textInteractionRef = useRef<TextInteractionState | null>(null);
  const audioCommitKeyRef = useRef<string | null>(null);
  const audioDraftOptionsRef = useRef<AudioEditOptions>(audioOptionsFromState(100, 0, 0, 0));
  const audioHydrationSelectionRef = useRef<string | null>(null);
  const audioInteractionRef = useRef<AudioInteractionState | null>(null);
  const productionInteractionRef = useRef<ProductionEffectInteractionState | null>(null);
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
    return () => {
      void cancelActiveProductionEffectInteraction();
    };
  }, [selected?.selectionHandle]);

  useEffect(() => {
    if (selected === null) {
      setTextState(DEFAULT_TEXT_STATE);
      textCommitKeyRef.current = null;
      textInteractionRef.current = null;
      audioCommitKeyRef.current = null;
      audioDraftOptionsRef.current = audioOptionsFromState(100, 0, 0, 0);
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
    audioDraftOptionsRef.current = selectedAudioOptions;
    audioCommitKeyRef.current = audioOptionsKey(selectedAudioOptions);
    audioHydrationSelectionRef.current = selected.selectionHandle;

    if (selected.text === null || selected.text === undefined) {
      setTextState(DEFAULT_TEXT_STATE);
      textCommitKeyRef.current = null;
      textInteractionRef.current = null;
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
  function commitTextState(state: TextFormState = textState): void {
    if (!hasText || selected?.text === null || selected?.text === undefined || validateTextForm(state) !== null) {
      return;
    }
    if (workspace.pendingCommand !== null) {
      return;
    }
    const nextPatch = textPatchFromState(state);
    const nextKey = textPatchKey(nextPatch);
    if (nextKey === textCommitKeyRef.current) {
      return;
    }
    textCommitKeyRef.current = nextKey;
    onEditSelectedText(nextPatch);
  }

  function updateTextState(
    patch: Partial<TextFormState>,
    options: { provisional?: boolean } = {}
  ): void {
    setTextState((current) => {
      const next = { ...current, ...patch };
      if (options.provisional) {
        queueInspectorTextUpdate(next);
      }
      return next;
    });
  }

  function updateAndCommitTextState(patch: Partial<TextFormState>): void {
    const next = { ...textState, ...patch };
    setTextState(next);
    commitTextState(next);
  }

  function finishOrCommitTextFieldEdit(): void {
    if (textInteractionRef.current !== null) {
      void finishInspectorTextInteraction("commit");
      return;
    }
    commitTextState();
  }

  function beginInspectorTextInteraction(): TextInteractionState {
    const existing = textInteractionRef.current;
    if (existing !== null) {
      return existing;
    }
    const interaction: TextInteractionState = {
      interactionId: null,
      sequence: 0,
      beginPromise: Promise.resolve(),
      updateInFlight: false,
      rafId: null,
      pendingPatch: null
    };
    interaction.beginPromise = projectInteractions.begin("selectedText").then((begin) => {
      if (textInteractionRef.current !== interaction || begin === null) {
        return;
      }
      interaction.interactionId = begin.interactionId;
      flushInspectorTextUpdate(interaction);
    });
    textInteractionRef.current = interaction;
    return interaction;
  }

  function queueInspectorTextUpdate(state: TextFormState): void {
    if (!hasText || selected === null || workspace.pendingCommand !== null || validateTextForm(state) !== null) {
      return;
    }
    const interaction = beginInspectorTextInteraction();
    interaction.pendingPatch = textPatchFromState(state);
    if (interaction.rafId !== null) {
      return;
    }
    interaction.rafId = window.requestAnimationFrame(() => {
      interaction.rafId = null;
      flushInspectorTextUpdate(interaction);
    });
  }

  function flushInspectorTextUpdate(interaction: TextInteractionState): void {
    if (interaction.updateInFlight || interaction.interactionId === null || interaction.pendingPatch === null) {
      return;
    }
    const nextPatch = interaction.pendingPatch;
    interaction.pendingPatch = null;
    interaction.updateInFlight = true;
    interaction.sequence += 1;
    void projectInteractions.update(interaction.interactionId, interaction.sequence, {
      kind: "selectedText",
      patch: nextPatch
    }).then(() => {
      interaction.updateInFlight = false;
      if (textInteractionRef.current !== interaction) {
        return;
      }
      flushInspectorTextUpdate(interaction);
    });
  }

  async function finishInspectorTextInteraction(action: "commit" | "cancel"): Promise<void> {
    const interaction = textInteractionRef.current;
    if (interaction === null) {
      return;
    }
    if (interaction.rafId !== null) {
      window.cancelAnimationFrame(interaction.rafId);
      interaction.rafId = null;
    }
    await interaction.beginPromise;
    while (interaction.updateInFlight) {
      await new Promise((resolve) => window.setTimeout(resolve, 0));
    }
    if (interaction.pendingPatch !== null) {
      flushInspectorTextUpdate(interaction);
      while (interaction.updateInFlight || interaction.pendingPatch !== null) {
        await new Promise((resolve) => window.setTimeout(resolve, 0));
      }
    }
    textInteractionRef.current = null;
    if (interaction.interactionId === null || interaction.sequence === 0) {
      if (action === "commit") {
        commitTextState();
      }
      return;
    }
    if (action === "commit") {
      textCommitKeyRef.current = textPatchKey(textPatchFromState(textState));
      await projectInteractions.commit(interaction.interactionId);
      return;
    }
    await projectInteractions.cancel(interaction.interactionId);
  }

  function updateAudioState(
    patch: Partial<{ volumePercent: number; panPercent: number; fadeInUs: number; fadeOutUs: number }>,
    options: { provisional?: boolean } = {}
  ): void {
    const nextVolumePercent = patch.volumePercent ?? volumePercent;
    const nextPanPercent = patch.panPercent ?? panPercent;
    const nextFadeInUs = patch.fadeInUs ?? fadeInUs;
    const nextFadeOutUs = patch.fadeOutUs ?? fadeOutUs;
    if (patch.volumePercent !== undefined) {
      setVolumePercent(patch.volumePercent);
    }
    if (patch.panPercent !== undefined) {
      setPanPercent(patch.panPercent);
    }
    if (patch.fadeInUs !== undefined) {
      setFadeInUs(patch.fadeInUs);
    }
    if (patch.fadeOutUs !== undefined) {
      setFadeOutUs(patch.fadeOutUs);
    }
    const nextOptions = audioOptionsFromState(nextVolumePercent, nextPanPercent, nextFadeInUs, nextFadeOutUs);
    audioDraftOptionsRef.current = nextOptions;
    if (options.provisional) {
      queueInspectorAudioUpdate(nextOptions);
    }
  }

  function commitAudioFieldEdit(options: AudioEditOptions = audioDraftOptionsRef.current): void {
    if (selected === null || workspace.pendingCommand !== null) {
      return;
    }
    const nextKey = audioOptionsKey(options);
    if (nextKey === audioCommitKeyRef.current) {
      return;
    }
    audioCommitKeyRef.current = nextKey;
    onUpdateSelectedSegmentAudio(options);
  }

  function beginInspectorAudioInteraction(): AudioInteractionState {
    const existing = audioInteractionRef.current;
    if (existing !== null) {
      return existing;
    }
    const interaction: AudioInteractionState = {
      interactionId: null,
      sequence: 0,
      beginPromise: Promise.resolve(),
      updateInFlight: false,
      rafId: null,
      pendingOptions: null
    };
    interaction.beginPromise = projectInteractions.begin("selectedSegmentAudio").then((begin) => {
      if (audioInteractionRef.current !== interaction || begin === null) {
        return;
      }
      interaction.interactionId = begin.interactionId;
      flushInspectorAudioUpdate(interaction);
    });
    audioInteractionRef.current = interaction;
    return interaction;
  }

  function queueInspectorAudioUpdate(options: AudioEditOptions): void {
    if (selected === null || workspace.pendingCommand !== null) {
      return;
    }
    const interaction = beginInspectorAudioInteraction();
    interaction.pendingOptions = options;
    if (interaction.rafId !== null) {
      return;
    }
    interaction.rafId = window.requestAnimationFrame(() => {
      interaction.rafId = null;
      flushInspectorAudioUpdate(interaction);
    });
  }

  function flushInspectorAudioUpdate(interaction: AudioInteractionState): void {
    if (interaction.updateInFlight || interaction.interactionId === null || interaction.pendingOptions === null) {
      return;
    }
    const nextOptions = interaction.pendingOptions;
    interaction.pendingOptions = null;
    interaction.updateInFlight = true;
    interaction.sequence += 1;
    void projectInteractions.update(interaction.interactionId, interaction.sequence, {
      kind: "selectedSegmentAudio",
      gainMillis: Math.max(0, Math.min(4000, Math.round(nextOptions.gainMillis))),
      panBalanceMillis: Math.max(-1000, Math.min(1000, Math.round(nextOptions.panBalanceMillis))),
      fadeInDuration: { duration: Math.max(0, Math.round(nextOptions.fadeInDuration)) },
      fadeOutDuration: { duration: Math.max(0, Math.round(nextOptions.fadeOutDuration)) },
      effectSlots: []
    }).then(() => {
      interaction.updateInFlight = false;
      if (audioInteractionRef.current !== interaction) {
        return;
      }
      flushInspectorAudioUpdate(interaction);
    });
  }

  async function finishInspectorAudioInteraction(action: "commit" | "cancel"): Promise<void> {
    const interaction = audioInteractionRef.current;
    if (interaction === null) {
      return;
    }
    if (interaction.rafId !== null) {
      window.cancelAnimationFrame(interaction.rafId);
      interaction.rafId = null;
    }
    await interaction.beginPromise;
    while (interaction.updateInFlight) {
      await new Promise((resolve) => window.setTimeout(resolve, 0));
    }
    if (interaction.pendingOptions !== null) {
      flushInspectorAudioUpdate(interaction);
      while (interaction.updateInFlight || interaction.pendingOptions !== null) {
        await new Promise((resolve) => window.setTimeout(resolve, 0));
      }
    }
    audioInteractionRef.current = null;
    if (interaction.interactionId === null) {
      return;
    }
    if (action === "commit") {
      audioCommitKeyRef.current = audioOptionsKey(audioDraftOptionsRef.current);
      await projectInteractions.commit(interaction.interactionId);
      return;
    }
    await projectInteractions.cancel(interaction.interactionId);
  }

  function beginProductionEffectInteraction(
    kind: ProductionEffectInteractionState["kind"]
  ): ProductionEffectInteractionState {
    const existing = productionInteractionRef.current;
    if (existing !== null && existing.kind === kind) {
      return existing;
    }
    const interaction: ProductionEffectInteractionState = {
      kind,
      interactionId: null,
      sequence: 0,
      beginPromise: Promise.resolve(),
      updateInFlight: false,
      finishing: false,
      rafId: null,
      pendingPayload: null,
      cleanupListeners: null
    };
    interaction.beginPromise = projectInteractions.begin(kind).then((begin) => {
      if (begin === null) {
        return;
      }
      interaction.interactionId = begin.interactionId;
      if (productionInteractionRef.current !== interaction) {
        return;
      }
      flushProductionEffectInteraction(interaction);
    });
    productionInteractionRef.current = interaction;
    interaction.cleanupListeners = armRangeFinishListeners(
      () => void finishProductionEffectInteraction("commit"),
      () => void finishProductionEffectInteraction("cancel")
    );
    return interaction;
  }

  function queueProductionEffectInteraction(
    kind: ProductionEffectInteractionState["kind"],
    payload: ProjectInteractionPayload
  ): void {
    if (selected === null || workspace.pendingCommand !== null) {
      return;
    }
    const interaction = beginProductionEffectInteraction(kind);
    interaction.pendingPayload = payload;
    if (interaction.rafId !== null) {
      return;
    }
    interaction.rafId = window.requestAnimationFrame(() => {
      interaction.rafId = null;
      flushProductionEffectInteraction(interaction);
    });
  }

  function flushProductionEffectInteraction(interaction: ProductionEffectInteractionState): void {
    if (interaction.updateInFlight || interaction.interactionId === null || interaction.pendingPayload === null) {
      return;
    }
    const payload = interaction.pendingPayload;
    interaction.pendingPayload = null;
    interaction.updateInFlight = true;
    interaction.sequence += 1;
    void projectInteractions.update(interaction.interactionId, interaction.sequence, payload).then(() => {
      interaction.updateInFlight = false;
      if (productionInteractionRef.current !== interaction) {
        return;
      }
      flushProductionEffectInteraction(interaction);
    });
  }

  async function finishProductionEffectInteraction(action: "commit" | "cancel"): Promise<void> {
    const interaction = productionInteractionRef.current;
    if (interaction === null) {
      return;
    }
    await closeProductionEffectInteraction(interaction, action);
  }

  async function cancelActiveProductionEffectInteraction(): Promise<void> {
    const interaction = productionInteractionRef.current;
    if (interaction === null) {
      return;
    }
    await closeProductionEffectInteraction(interaction, "cancel");
  }

  async function closeProductionEffectInteraction(
    interaction: ProductionEffectInteractionState,
    action: "commit" | "cancel"
  ): Promise<void> {
    if (interaction.finishing) {
      return;
    }
    interaction.finishing = true;
    interaction.cleanupListeners?.();
    interaction.cleanupListeners = null;
    if (interaction.rafId !== null) {
      window.cancelAnimationFrame(interaction.rafId);
      interaction.rafId = null;
    }
    await interaction.beginPromise;
    while (interaction.updateInFlight) {
      await new Promise((resolve) => window.setTimeout(resolve, 0));
    }
    if (action === "commit" && interaction.pendingPayload !== null) {
      flushProductionEffectInteraction(interaction);
      while (interaction.updateInFlight || interaction.pendingPayload !== null) {
        await new Promise((resolve) => window.setTimeout(resolve, 0));
      }
    } else {
      interaction.pendingPayload = null;
    }
    if (productionInteractionRef.current === interaction) {
      productionInteractionRef.current = null;
    }
    if (interaction.interactionId === null) {
      return;
    }
    if (action === "commit") {
      await projectInteractions.commit(interaction.interactionId);
      return;
    }
    await projectInteractions.cancel(interaction.interactionId);
  }

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
                  selected={selected}
                  visual={selected.visual}
                  playheadAt={playheadUs}
                  pending={workspace.pendingCommand !== null}
                  renderKeyframeButton={renderKeyframeButton}
                  projectInteractions={projectInteractions}
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
	                          updateTextState({ content });
	                        }}
	                        onBlur={() => commitTextState()}
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
	                          updateTextState({ fontFamily, fontRef: font?.fontRef ?? null });
	                        }}
	                        onBlur={() => commitTextState()}
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
	                      onChange={(fontSize) => updateTextState({ fontSize }, { provisional: true })}
	                      onCommit={finishOrCommitTextFieldEdit}
	                      onCancel={() => void finishInspectorTextInteraction("cancel")}
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
	                            updateTextState({ color }, { provisional: true });
	                          }}
	                          onPointerUp={finishOrCommitTextFieldEdit}
	                          onPointerCancel={() => void finishInspectorTextInteraction("cancel")}
	                          onBlur={finishOrCommitTextFieldEdit}
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
	                          updateAndCommitTextState({ strokeEnabled });
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
	                          updateTextState({ strokeColor }, { provisional: true });
	                        }}
	                        onPointerUp={finishOrCommitTextFieldEdit}
	                        onPointerCancel={() => void finishInspectorTextInteraction("cancel")}
	                        onBlur={finishOrCommitTextFieldEdit}
	                      />
                    </label>
                    <TextNumberField
                      label="描边宽度"
                      value={textState.strokeWidth}
                      min={1}
                      max={120}
	                      step={1}
	                      disabled={inspectorFieldsDisabled || !textState.strokeEnabled}
	                      onChange={(strokeWidth) => updateTextState({ strokeWidth }, { provisional: true })}
	                      onCommit={finishOrCommitTextFieldEdit}
	                      onCancel={() => void finishInspectorTextInteraction("cancel")}
	                    />
                    <label className="toggle-row compact-toggle">
                      <input
                        type="checkbox"
                        checked={textState.shadowEnabled}
	                        disabled={inspectorFieldsDisabled}
	                        onChange={(event) => {
	                          const shadowEnabled = event.currentTarget.checked;
	                          updateAndCommitTextState({ shadowEnabled });
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
	                          updateTextState({ shadowColor }, { provisional: true });
	                        }}
	                        onPointerUp={finishOrCommitTextFieldEdit}
	                        onPointerCancel={() => void finishInspectorTextInteraction("cancel")}
	                        onBlur={finishOrCommitTextFieldEdit}
	                      />
                    </label>
                    <label className="toggle-row compact-toggle">
                      <input
                        type="checkbox"
                        checked={textState.backgroundEnabled}
	                        disabled={inspectorFieldsDisabled}
	                        onChange={(event) => {
	                          const backgroundEnabled = event.currentTarget.checked;
	                          updateAndCommitTextState({ backgroundEnabled });
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
	                          updateTextState({ backgroundColor }, { provisional: true });
	                        }}
	                        onPointerUp={finishOrCommitTextFieldEdit}
	                        onPointerCancel={() => void finishInspectorTextInteraction("cancel")}
	                        onBlur={finishOrCommitTextFieldEdit}
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
	                            onClick={() => updateAndCommitTextState({ alignment: value })}
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
                      value={millisToPercentValue(textState.textBoxWidthMillis)}
                      min={0.1}
                      max={100}
	                      step={1}
                      suffix="%"
	                      disabled={inspectorFieldsDisabled}
	                      onChange={(value) => updateTextState({ textBoxWidthMillis: percentToMillisValue(value) }, { provisional: true })}
	                      onCommit={finishOrCommitTextFieldEdit}
	                      onCancel={() => void finishInspectorTextInteraction("cancel")}
	                    />
                    <TextNumberField
                      label="高度"
                      value={millisToPercentValue(textState.textBoxHeightMillis)}
                      min={0.1}
                      max={100}
	                      step={1}
                      suffix="%"
	                      disabled={inspectorFieldsDisabled}
	                      onChange={(value) => updateTextState({ textBoxHeightMillis: percentToMillisValue(value) }, { provisional: true })}
	                      onCommit={finishOrCommitTextFieldEdit}
	                      onCancel={() => void finishInspectorTextInteraction("cancel")}
	                    />
                    <label className="toggle-row compact-toggle">
                      <input
                        type="checkbox"
                        checked={textState.wrapping === "auto"}
	                        disabled={inspectorFieldsDisabled}
	                        onChange={(event) => {
	                          const wrapping = event.currentTarget.checked ? "auto" : "none";
	                          updateAndCommitTextState({ wrapping });
	                        }}
	                      />
                      <span>自动换行</span>
                    </label>
                    <TextNumberField
                      label="行高"
                      value={millisToPercentValue(textState.lineHeightMillis)}
                      min={50}
                      max={300}
                      step={5}
                      suffix="%"
	                      disabled={inspectorFieldsDisabled}
	                      action={renderKeyframeButton("textLineHeight", "行高")}
	                      onChange={(value) => updateTextState({ lineHeightMillis: percentToMillisValue(value) }, { provisional: true })}
	                      onCommit={finishOrCommitTextFieldEdit}
	                      onCancel={() => void finishInspectorTextInteraction("cancel")}
	                    />
                    <TextNumberField
                      label="字间距"
                      value={millisToPercentValue(textState.letterSpacingMillis)}
                      min={0}
                      max={200}
                      step={5}
                      suffix="%"
	                      disabled={inspectorFieldsDisabled}
	                      action={renderKeyframeButton("textLetterSpacing", "字间距")}
	                      onChange={(value) => updateTextState({ letterSpacingMillis: percentToMillisValue(value) }, { provisional: true })}
	                      onCommit={finishOrCommitTextFieldEdit}
	                      onCancel={() => void finishInspectorTextInteraction("cancel")}
	                    />
                  </section>

                  <section className="inspector-section" aria-label="布局">
                    <div className="inspector-section-title">
                      <h3>布局</h3>
                      {renderKeyframeButton("textLayoutX", "布局 X")}
                    </div>
                    <p className="inspector-note">安全区域使用画布百分比坐标。</p>
                    <div className="text-layout-grid">
                      <TextNumberField
                        label="X"
                        value={millisToPercentValue(textState.layoutXMillis)}
                        min={0}
                        max={100}
                        step={1}
                        suffix="%"
	                        disabled={inspectorFieldsDisabled}
	                        action={renderKeyframeButton("textLayoutX", "布局 X")}
	                        onChange={(value) => updateTextState({ layoutXMillis: percentToMillisValue(value) }, { provisional: true })}
	                        onCommit={finishOrCommitTextFieldEdit}
	                        onCancel={() => void finishInspectorTextInteraction("cancel")}
	                      />
                      <TextNumberField
                        label="Y"
                        value={millisToPercentValue(textState.layoutYMillis)}
                        min={0}
                        max={100}
                        step={1}
                        suffix="%"
	                        disabled={inspectorFieldsDisabled}
	                        action={renderKeyframeButton("textLayoutY", "布局 Y")}
	                        onChange={(value) => updateTextState({ layoutYMillis: percentToMillisValue(value) }, { provisional: true })}
	                        onCommit={finishOrCommitTextFieldEdit}
	                        onCancel={() => void finishInspectorTextInteraction("cancel")}
	                      />
                      <TextNumberField
                        label="宽"
                        value={millisToPercentValue(textState.layoutWidthMillis)}
                        min={0.1}
                        max={100}
                        step={1}
                        suffix="%"
	                        disabled={inspectorFieldsDisabled}
	                        action={renderKeyframeButton("textLayoutWidth", "布局宽")}
	                        onChange={(value) => updateTextState({ layoutWidthMillis: percentToMillisValue(value) }, { provisional: true })}
	                        onCommit={finishOrCommitTextFieldEdit}
	                        onCancel={() => void finishInspectorTextInteraction("cancel")}
	                      />
                      <TextNumberField
                        label="高"
                        value={millisToPercentValue(textState.layoutHeightMillis)}
                        min={0.1}
                        max={100}
                        step={1}
                        suffix="%"
	                        disabled={inspectorFieldsDisabled}
	                        action={renderKeyframeButton("textLayoutHeight", "布局高")}
	                        onChange={(value) => updateTextState({ layoutHeightMillis: percentToMillisValue(value) }, { provisional: true })}
	                        onCommit={finishOrCommitTextFieldEdit}
	                        onCancel={() => void finishInspectorTextInteraction("cancel")}
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
                    onPointerDown={beginInspectorAudioInteraction}
                    onPointerUp={() => void finishInspectorAudioInteraction("commit")}
                    onPointerCancel={() => void finishInspectorAudioInteraction("cancel")}
                    onBlur={() => void finishInspectorAudioInteraction("commit")}
                    onChange={(event) =>
                      updateAudioState(
                        { volumePercent: toBoundedNumber(event.currentTarget.valueAsNumber, volumePercent, 0, 400) },
                        { provisional: true }
                      )
                    }
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
                  onPointerDown={beginInspectorAudioInteraction}
                  onPointerUp={() => void finishInspectorAudioInteraction("commit")}
                  onPointerCancel={() => void finishInspectorAudioInteraction("cancel")}
                  onBlur={() => void finishInspectorAudioInteraction("commit")}
                  onChange={(event) =>
                    updateAudioState(
                      { panPercent: toBoundedNumber(event.currentTarget.valueAsNumber, panPercent, -100, 100) },
                      { provisional: true }
                    )
                  }
                />
              </label>
              <label className="field-row compact-row">
                <span>淡入</span>
                <input
                  aria-label="淡入"
                  type="number"
                  min="0"
                  step="0.1"
                  value={microsecondsToSeconds(fadeInUs)}
                  onChange={(event) =>
                    updateAudioState(
                      {
                        fadeInUs: secondsToNonNegativeMicroseconds(toBoundedFloat(event.currentTarget.valueAsNumber, 0, 0, 60))
                      },
                      { provisional: true }
                    )
                  }
                  onBlur={() => void finishInspectorAudioInteraction("commit")}
                />
              </label>
              <label className="field-row compact-row">
                <span>淡出</span>
                <input
                  aria-label="淡出"
                  type="number"
                  min="0"
                  step="0.1"
                  value={microsecondsToSeconds(fadeOutUs)}
                  onChange={(event) =>
                    updateAudioState(
                      {
                        fadeOutUs: secondsToNonNegativeMicroseconds(toBoundedFloat(event.currentTarget.valueAsNumber, 0, 0, 60))
                      },
                      { provisional: true }
                    )
                  }
                  onBlur={() => void finishInspectorAudioInteraction("commit")}
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

          {effectiveActiveTab === "变速" ? (
            <RetimeInspectorSection
              selected={selected}
              pending={inspectorFieldsDisabled}
              queueProductionEffectInteraction={queueProductionEffectInteraction}
              finishProductionEffectInteraction={finishProductionEffectInteraction}
              onSetSelectedSegmentRetime={onSetSelectedSegmentRetime}
            />
          ) : null}

          {effectiveActiveTab === "效果" || effectiveActiveTab === "滤镜" || effectiveActiveTab === "调节" ? (
            <EffectsInspectorSection
              tab={effectiveActiveTab}
              selected={selected}
              capabilities={workspace.viewModel.productionEffectCapabilities.entries}
              pending={inspectorFieldsDisabled}
              queueProductionEffectInteraction={queueProductionEffectInteraction}
              finishProductionEffectInteraction={finishProductionEffectInteraction}
              onApplySelectedSegmentEffect={onApplySelectedSegmentEffect}
              onUpdateSelectedSegmentEffectParameter={onUpdateSelectedSegmentEffectParameter}
              onRemoveSelectedSegmentEffect={onRemoveSelectedSegmentEffect}
            />
          ) : null}

          {effectiveActiveTab === "蒙版" ? (
            <MaskInspectorSection
              selected={selected}
              capabilities={workspace.viewModel.productionEffectCapabilities.entries}
              pending={inspectorFieldsDisabled}
              queueProductionEffectInteraction={queueProductionEffectInteraction}
              finishProductionEffectInteraction={finishProductionEffectInteraction}
              onSetSelectedSegmentMask={onSetSelectedSegmentMask}
            />
          ) : null}

          {effectiveActiveTab === "混合" ? (
            <BlendInspectorSection
              selected={selected}
              capabilities={workspace.viewModel.productionEffectCapabilities.entries}
              pending={inspectorFieldsDisabled}
              queueProductionEffectInteraction={queueProductionEffectInteraction}
              finishProductionEffectInteraction={finishProductionEffectInteraction}
              onSetSelectedSegmentBlendMode={onSetSelectedSegmentBlendMode}
            />
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

function RetimeInspectorSection({
  selected,
  pending,
  queueProductionEffectInteraction,
  finishProductionEffectInteraction,
  onSetSelectedSegmentRetime
}: {
  selected: SelectedSegmentView;
  pending: boolean;
  queueProductionEffectInteraction: (
    kind: "selectedSegmentRetime",
    payload: ProjectInteractionPayload
  ) => void;
  finishProductionEffectInteraction: (action: "commit" | "cancel") => Promise<void>;
  onSetSelectedSegmentRetime: (retiming: SegmentRetiming) => void;
}): React.ReactElement {
  const acceptedPercent = retimePercentFromSelected(selected);
  const [draftPercent, setDraftPercent] = useState(acceptedPercent);

  useEffect(() => {
    setDraftPercent(acceptedPercent);
  }, [acceptedPercent, selected.selectionHandle]);

  const audioFollows = selected.retiming.audioPolicy === "followVideoSpeed";
  const commitPercent = (percent: number): void => {
    onSetSelectedSegmentRetime(retimingFromPercent(percent, audioFollows));
  };

  return (
    <section className="inspector-section production-inspector-section" aria-label="变速" role="tabpanel">
      <div className="inspector-section-title">
        <h3>变速</h3>
        <span className="phase19-inline-status">{selected.phase19.retimeLabel}</span>
      </div>
      <div className="segmented-control phase19-segmented" role="group" aria-label="常规变速">
        {([50, 100, 200] as const).map((percent) => (
          <button
            key={percent}
            type="button"
            className={draftPercent === percent ? "active" : ""}
            disabled={pending}
            onClick={() => {
              setDraftPercent(percent);
              commitPercent(percent);
            }}
          >
            {speedPercentLabel(percent)}
          </button>
        ))}
        <button type="button" className={!([50, 100, 200] as const).includes(draftPercent as 50 | 100 | 200) ? "active" : ""} disabled>
          自定义
        </button>
      </div>
      <label className="field-row compact-row">
        <span>倍率</span>
        <input
          aria-label="变速倍率"
          type="range"
          min="25"
          max="300"
          step="5"
          value={draftPercent}
          disabled={pending}
          onPointerDown={(event) => {
            captureRangePointer(event);
            queueProductionEffectInteraction("selectedSegmentRetime", {
              kind: "selectedSegmentRetime",
              retiming: retimingFromPercent(draftPercent, audioFollows)
            });
          }}
          onPointerUp={() => void finishProductionEffectInteraction("commit")}
          onPointerCancel={() => void finishProductionEffectInteraction("cancel")}
          onMouseUp={() => void finishProductionEffectInteraction("commit")}
          onBlur={() => void finishProductionEffectInteraction("commit")}
          onChange={(event) => {
            const percent = toBoundedNumber(event.currentTarget.valueAsNumber, draftPercent, 25, 300);
            setDraftPercent(percent);
            queueProductionEffectInteraction("selectedSegmentRetime", {
              kind: "selectedSegmentRetime",
              retiming: retimingFromPercent(percent, audioFollows)
            });
          }}
        />
      </label>
      <div className="field-row compact-row phase19-readout-row">
        <span>当前倍率</span>
        <strong>{speedPercentLabel(draftPercent)}</strong>
      </div>
      <label className="toggle-row compact-toggle">
        <input
          type="checkbox"
          checked={audioFollows}
          disabled={pending}
          onChange={(event) => onSetSelectedSegmentRetime(retimingFromPercent(draftPercent, event.currentTarget.checked))}
        />
        <span>音频跟随变速</span>
      </label>
      <p className="phase19-compact-note">保持音调暂不支持</p>
    </section>
  );
}

function EffectsInspectorSection({
  tab,
  selected,
  capabilities,
  pending,
  queueProductionEffectInteraction,
  finishProductionEffectInteraction,
  onApplySelectedSegmentEffect,
  onUpdateSelectedSegmentEffectParameter,
  onRemoveSelectedSegmentEffect
}: {
  tab: "效果" | "滤镜" | "调节";
  selected: SelectedSegmentView;
  capabilities: CapabilityReportItem[];
  pending: boolean;
  queueProductionEffectInteraction: (
    kind: "selectedSegmentEffect",
    payload: ProjectInteractionPayload
  ) => void;
  finishProductionEffectInteraction: (action: "commit" | "cancel") => Promise<void>;
  onApplySelectedSegmentEffect: (effect: Filter) => void;
  onUpdateSelectedSegmentEffectParameter: (effectIndex: number, parameter: EffectParameterUpdate) => void;
  onRemoveSelectedSegmentEffect: (effectIndex: number) => void;
}): React.ReactElement {
  const available = productionEffectQuickAdds(tab, capabilities);
  const filters = selected.filters
    .map((filter, index) => ({ filter, index }))
    .filter(({ filter }) => filterVisibleInTab(filter, tab));

  return (
    <section className="inspector-section production-inspector-section" aria-label={tab} role="tabpanel">
      <div className="inspector-section-title">
        <h3>{tab}</h3>
        <span className="phase19-inline-status">{selected.phase19.effectCount} 个</span>
      </div>
      <ProductionCapabilityChips selected={selected} capabilities={capabilities} />
      <div className="phase19-quick-actions" role="group" aria-label={`${tab}快捷应用`}>
        {available.map((entry) => (
          <button
            key={entry.capabilityId}
            type="button"
            className="compact-action"
            disabled={pending}
            onClick={() => onApplySelectedSegmentEffect(effectForCapability(entry.capabilityId))}
          >
            {capabilityProductLabel(entry.capabilityId)}
          </button>
        ))}
      </div>
      {filters.length === 0 ? (
        <p className="phase19-empty-copy">未应用</p>
      ) : (
        <div className="phase19-effect-list">
          {filters.map(({ filter, index }) => (
            <AppliedEffectControls
              key={`${index}-${filterCapabilityId(filter)}`}
              filter={filter}
              effectIndex={index}
              pending={pending}
              queueProductionEffectInteraction={queueProductionEffectInteraction}
              finishProductionEffectInteraction={finishProductionEffectInteraction}
              onUpdateSelectedSegmentEffectParameter={onUpdateSelectedSegmentEffectParameter}
              onRemoveSelectedSegmentEffect={onRemoveSelectedSegmentEffect}
            />
          ))}
        </div>
      )}
    </section>
  );
}

function AppliedEffectControls({
  filter,
  effectIndex,
  pending,
  queueProductionEffectInteraction,
  finishProductionEffectInteraction,
  onUpdateSelectedSegmentEffectParameter,
  onRemoveSelectedSegmentEffect
}: {
  filter: Filter;
  effectIndex: number;
  pending: boolean;
  queueProductionEffectInteraction: (
    kind: "selectedSegmentEffect",
    payload: ProjectInteractionPayload
  ) => void;
  finishProductionEffectInteraction: (action: "commit" | "cancel") => Promise<void>;
  onUpdateSelectedSegmentEffectParameter: (effectIndex: number, parameter: EffectParameterUpdate) => void;
  onRemoveSelectedSegmentEffect: (effectIndex: number) => void;
}): React.ReactElement {
  const capabilityId = filterCapabilityId(filter);
  const title = capabilityProductLabel(capabilityId);
  const sliders = effectSliders(filter);
  const effectTargetKey = `${effectIndex}:${capabilityId}:${JSON.stringify(filter)}`;
  const [confirmRemoveTargetKey, setConfirmRemoveTargetKey] = useState<string | null>(null);
  const confirmRemove = confirmRemoveTargetKey === effectTargetKey;

  useEffect(() => {
    setConfirmRemoveTargetKey(null);
  }, [effectTargetKey]);

  return (
    <article className="phase19-effect-row" aria-label={title}>
      <div className="phase19-effect-row-header">
        <strong>{title}</strong>
        <label className="mini-toggle">
          <input
            type="checkbox"
            checked={filter.enabled}
            disabled={pending}
            onChange={(event) =>
              onUpdateSelectedSegmentEffectParameter(effectIndex, {
                parameter: "enabled",
                enabled: event.currentTarget.checked
              })
            }
          />
          <span>启用</span>
        </label>
        <button
          type="button"
          className="icon-text-action danger"
          aria-label={`移除${title}`}
          title={`移除${title}`}
          disabled={pending}
          onClick={() => setConfirmRemoveTargetKey(effectTargetKey)}
        >
          移除效果
        </button>
      </div>
      {confirmRemove ? (
        <div className="phase19-confirm-row" role="group" aria-label="移除效果确认">
          <button
            type="button"
            className="icon-text-action danger"
            disabled={pending}
            onClick={() => {
              if (confirmRemoveTargetKey !== effectTargetKey) {
                setConfirmRemoveTargetKey(null);
                return;
              }
              onRemoveSelectedSegmentEffect(effectIndex);
              setConfirmRemoveTargetKey(null);
            }}
          >
            确认移除效果
          </button>
          <button type="button" className="icon-text-action" disabled={pending} onClick={() => setConfirmRemoveTargetKey(null)}>
            保留效果
          </button>
        </div>
      ) : null}
      {sliders.map((slider) => (
        <label className="field-row compact-row" key={slider.label}>
          <span>{slider.label}</span>
          <input
            aria-label={slider.label}
            type="range"
            min={slider.min}
            max={slider.max}
            step={slider.step}
            value={slider.value}
            disabled={pending || !filter.enabled}
            onPointerDown={(event) => {
              captureRangePointer(event);
              queueProductionEffectInteraction("selectedSegmentEffect", {
                kind: "selectedSegmentEffect",
                effectIndex,
                parameter: slider.parameter(slider.value)
              });
            }}
            onPointerUp={() => void finishProductionEffectInteraction("commit")}
            onPointerCancel={() => void finishProductionEffectInteraction("cancel")}
            onMouseUp={() => void finishProductionEffectInteraction("commit")}
            onBlur={() => void finishProductionEffectInteraction("commit")}
            onChange={(event) => {
              const value = toBoundedNumber(event.currentTarget.valueAsNumber, slider.value, slider.min, slider.max);
              queueProductionEffectInteraction("selectedSegmentEffect", {
                kind: "selectedSegmentEffect",
                effectIndex,
                parameter: slider.parameter(value)
              });
            }}
          />
        </label>
      ))}
    </article>
  );
}

function MaskInspectorSection({
  selected,
  capabilities,
  pending,
  queueProductionEffectInteraction,
  finishProductionEffectInteraction,
  onSetSelectedSegmentMask
}: {
  selected: SelectedSegmentView;
  capabilities: CapabilityReportItem[];
  pending: boolean;
  queueProductionEffectInteraction: (
    kind: "selectedSegmentMask",
    payload: ProjectInteractionPayload
  ) => void;
  finishProductionEffectInteraction: (action: "commit" | "cancel") => Promise<void>;
  onSetSelectedSegmentMask: (mask: SegmentMask) => void;
}): React.ReactElement {
  const mask = selected.visual.mask.kind === "none" || selected.visual.mask.kind === "externalReference"
    ? defaultMask("rectangle")
    : selected.visual.mask;
  const shape = mask.kind === "ellipse" ? "ellipse" : "rectangle";
  const maskTargetKey = `${selected.selectionHandle}:${JSON.stringify(selected.visual.mask)}`;
  const [confirmResetTargetKey, setConfirmResetTargetKey] = useState<string | null>(null);
  const confirmReset = confirmResetTargetKey === maskTargetKey;

  useEffect(() => {
    setConfirmResetTargetKey(null);
  }, [maskTargetKey]);

  return (
    <section className="inspector-section production-inspector-section" aria-label="蒙版" role="tabpanel">
      <div className="inspector-section-title">
        <h3>蒙版</h3>
        <span className="phase19-inline-status">{selected.phase19.maskLabel}</span>
      </div>
      <ProductionCapabilityChips selected={selected} capabilities={capabilities} capabilityIds={["mask.rectangle", "mask.ellipse"]} />
      <div className="segmented-control phase19-segmented" role="group" aria-label="蒙版形状">
        {(["rectangle", "ellipse"] as const).map((nextShape) => (
          <button
            key={nextShape}
            type="button"
            className={shape === nextShape ? "active" : ""}
            disabled={pending}
            onClick={() => onSetSelectedSegmentMask(defaultMask(nextShape))}
          >
            {nextShape === "rectangle" ? "矩形" : "椭圆"}
          </button>
        ))}
      </div>
      <MaskSlider
        label="羽化"
        value={mask.featherMillis}
        max={1000}
        pending={pending}
        onChange={(value) =>
          queueProductionEffectInteraction("selectedSegmentMask", {
            kind: "selectedSegmentMask",
            mask: { ...mask, featherMillis: value }
          })
        }
        onFinish={finishProductionEffectInteraction}
      />
      <MaskSlider
        label="透明度"
        value={mask.opacityMillis}
        max={1000}
        pending={pending}
        onChange={(value) =>
          queueProductionEffectInteraction("selectedSegmentMask", {
            kind: "selectedSegmentMask",
            mask: { ...mask, opacityMillis: value }
          })
        }
        onFinish={finishProductionEffectInteraction}
      />
      <label className="toggle-row compact-toggle">
        <input
          type="checkbox"
          checked={mask.inverted}
          disabled={pending}
          onChange={(event) => onSetSelectedSegmentMask({ ...mask, inverted: event.currentTarget.checked })}
        />
        <span>反选蒙版</span>
      </label>
      <button type="button" className="compact-action" disabled={pending} onClick={() => setConfirmResetTargetKey(maskTargetKey)}>
        重置效果
      </button>
      {confirmReset ? (
        <div className="phase19-confirm-row" role="group" aria-label="重置效果确认">
          <button
            type="button"
            className="icon-text-action danger"
            disabled={pending}
            onClick={() => {
              if (confirmResetTargetKey !== maskTargetKey) {
                setConfirmResetTargetKey(null);
                return;
              }
              onSetSelectedSegmentMask({ kind: "none" });
              setConfirmResetTargetKey(null);
            }}
          >
            确认重置效果
          </button>
          <button type="button" className="icon-text-action" disabled={pending} onClick={() => setConfirmResetTargetKey(null)}>
            继续保留当前效果
          </button>
        </div>
      ) : null}
    </section>
  );
}

function MaskSlider({
  label,
  value,
  max,
  pending,
  onChange,
  onFinish
}: {
  label: string;
  value: number;
  max: number;
  pending: boolean;
  onChange: (value: number) => void;
  onFinish: (action: "commit" | "cancel") => Promise<void>;
}): React.ReactElement {
  return (
    <label className="field-row compact-row">
      <span>{label}</span>
      <input
        aria-label={label}
        type="range"
        min="0"
        max={max}
        step="10"
        value={value}
        disabled={pending}
        onPointerDown={(event) => {
          captureRangePointer(event);
          onChange(value);
        }}
        onPointerUp={() => void onFinish("commit")}
        onPointerCancel={() => void onFinish("cancel")}
        onMouseUp={() => void onFinish("commit")}
        onBlur={() => void onFinish("commit")}
        onChange={(event) => onChange(toBoundedNumber(event.currentTarget.valueAsNumber, value, 0, max))}
      />
    </label>
  );
}

function BlendInspectorSection({
  selected,
  capabilities,
  pending,
  queueProductionEffectInteraction,
  finishProductionEffectInteraction,
  onSetSelectedSegmentBlendMode
}: {
  selected: SelectedSegmentView;
  capabilities: CapabilityReportItem[];
  pending: boolean;
  queueProductionEffectInteraction: (
    kind: "selectedSegmentBlend",
    payload: ProjectInteractionPayload
  ) => void;
  finishProductionEffectInteraction: (action: "commit" | "cancel") => Promise<void>;
  onSetSelectedSegmentBlendMode: (blendMode: SegmentBlendMode) => void;
}): React.ReactElement {
  const opacityMillis = selected.visual.transform.opacity.valueMillis;
  return (
    <section className="inspector-section production-inspector-section" aria-label="混合" role="tabpanel">
      <div className="inspector-section-title">
        <h3>混合</h3>
        <span className="phase19-inline-status">{selected.phase19.blendLabel}</span>
      </div>
      <ProductionCapabilityChips selected={selected} capabilities={capabilities} capabilityIds={["blend.normal", "blend.multiply", "blend.screen"]} />
      <label className="field-row compact-row">
        <span>混合模式</span>
        <select
          aria-label="混合模式"
          value={selected.visual.blendMode.kind}
          disabled={pending}
          onChange={(event) => onSetSelectedSegmentBlendMode({ kind: event.currentTarget.value as "normal" | "multiply" | "screen" })}
        >
          <option value="normal">正常</option>
          <option value="multiply">正片叠底</option>
          <option value="screen">滤色</option>
        </select>
      </label>
      <label className="field-row compact-row">
        <span>透明度</span>
        <input
          aria-label="混合透明度"
          type="range"
          min="0"
          max="1000"
          step="10"
          value={opacityMillis}
          disabled={pending}
          onPointerDown={(event) => {
            captureRangePointer(event);
            queueProductionEffectInteraction("selectedSegmentBlend", {
              kind: "selectedSegmentBlend",
              opacityMillis
            });
          }}
          onPointerUp={() => void finishProductionEffectInteraction("commit")}
          onPointerCancel={() => void finishProductionEffectInteraction("cancel")}
          onMouseUp={() => void finishProductionEffectInteraction("commit")}
          onBlur={() => void finishProductionEffectInteraction("commit")}
          onChange={(event) =>
            queueProductionEffectInteraction("selectedSegmentBlend", {
              kind: "selectedSegmentBlend",
              opacityMillis: toBoundedNumber(event.currentTarget.valueAsNumber, opacityMillis, 0, 1000)
            })
          }
        />
      </label>
    </section>
  );
}

function ProductionCapabilityChips({
  selected,
  capabilities,
  capabilityIds
}: {
  selected: SelectedSegmentView;
  capabilities: CapabilityReportItem[];
  capabilityIds?: readonly string[];
}): React.ReactElement {
  const chips = (capabilityIds ?? selected.phase19.supportChips.map((chip) => chip.capabilityId))
    .map((capabilityId) => capabilities.find((entry) => entry.capabilityId === capabilityId) ?? null)
    .filter((entry): entry is CapabilityReportItem => entry !== null)
    .slice(0, 4);

  return (
    <div className="phase19-capability-chips" aria-label="能力支持">
      {chips.length === 0 ? (
        <span className="phase19-chip muted">暂不支持</span>
      ) : (
        chips.flatMap((entry) => [
          <span className={`phase19-chip ${supportChipTone(entry.preview)}`} key={`${entry.capabilityId}-preview`}>
            {supportLabel("预览", entry.preview)}
          </span>,
          <span className={`phase19-chip ${supportChipTone(entry.export)}`} key={`${entry.capabilityId}-export`}>
            {supportLabel("导出", entry.export)}
          </span>
        ])
      )}
    </div>
  );
}

function retimePercentFromSelected(selected: SelectedSegmentView): number {
  const mode = selected.retiming.mode;
  if (mode.kind !== "constant" || mode.speed.denominator <= 0) {
    return 100;
  }
  return Math.max(25, Math.min(300, Math.round((mode.speed.numerator * 100) / mode.speed.denominator)));
}

function retimingFromPercent(percent: number, audioFollows: boolean): SegmentRetiming {
  const bounded = Math.max(25, Math.min(300, Math.round(percent)));
  return {
    mode: {
      kind: "constant",
      speed: {
        numerator: bounded,
        denominator: 100
      }
    },
    audioPolicy: audioFollows ? "followVideoSpeed" : "muteUnsupported"
  };
}

function speedPercentLabel(percent: number): string {
  if (percent % 100 === 0) {
    return `${percent / 100}x`;
  }
  return `${(percent / 100).toFixed(2).replace(/0$/, "")}x`;
}

function captureRangePointer(event: ReactPointerEvent<HTMLInputElement>): void {
  event.currentTarget.setPointerCapture(event.pointerId);
}

function armRangeFinishListeners(onCommit: () => void, onCancel: () => void): () => void {
  let active = true;
  const cleanup = (): void => {
    active = false;
    window.removeEventListener("pointerup", commit);
    window.removeEventListener("mouseup", commit);
    window.removeEventListener("pointercancel", cancel);
    window.removeEventListener("keydown", cancelOnEscape);
  };
  const commit = (): void => {
    if (!active) {
      return;
    }
    cleanup();
    onCommit();
  };
  const cancel = (): void => {
    if (!active) {
      return;
    }
    cleanup();
    onCancel();
  };
  const cancelOnEscape = (event: KeyboardEvent): void => {
    if (event.key !== "Escape") {
      return;
    }
    event.preventDefault();
    cancel();
  };

  window.addEventListener("pointerup", commit, { once: true });
  window.addEventListener("mouseup", commit, { once: true });
  window.addEventListener("pointercancel", cancel, { once: true });
  window.addEventListener("keydown", cancelOnEscape);
  return cleanup;
}

function productionEffectQuickAdds(tab: "效果" | "滤镜" | "调节", capabilities: CapabilityReportItem[]): CapabilityReportItem[] {
  const ids =
    tab === "效果"
      ? ["effect.gaussianBlur", "effect.opacityAdjustment"]
      : tab === "滤镜"
        ? ["effect.basicColorAdjustment"]
        : ["effect.basicColorAdjustment", "effect.opacityAdjustment"];
  return ids
    .map((id) => capabilities.find((entry) => entry.capabilityId === id) ?? null)
    .filter((entry): entry is CapabilityReportItem => entry !== null && capabilityActionState(entry) !== "unsupported");
}

function filterVisibleInTab(filter: Filter, tab: "效果" | "滤镜" | "调节"): boolean {
  const capabilityId = filterCapabilityId(filter);
  if (tab === "效果") {
    return capabilityId === "effect.gaussianBlur" || capabilityId === "effect.opacityAdjustment";
  }
  if (tab === "滤镜") {
    return capabilityId === "effect.basicColorAdjustment";
  }
  return capabilityId === "effect.basicColorAdjustment" || capabilityId === "effect.opacityAdjustment";
}

function effectForCapability(capabilityId: string): Filter {
  if (capabilityId === "effect.basicColorAdjustment") {
    return {
      kind: {
        kind: "basicColorAdjustment",
        brightnessMillis: 0,
        contrastMillis: 1000,
        saturationMillis: 1000
      },
      enabled: true
    };
  }
  if (capabilityId === "effect.opacityAdjustment") {
    return { kind: { kind: "opacityAdjustment", opacityMillis: 1000 }, enabled: true };
  }
  return { kind: { kind: "gaussianBlur", radiusMillis: 1000 }, enabled: true };
}

function filterCapabilityId(filter: Filter): string {
  switch (filter.kind.kind) {
    case "gaussianBlur":
      return "effect.gaussianBlur";
    case "basicColorAdjustment":
      return "effect.basicColorAdjustment";
    case "opacityAdjustment":
      return "effect.opacityAdjustment";
    case "externalReference":
      return "externalReference";
  }
}

function capabilityProductLabel(capabilityId: string): string {
  switch (capabilityId) {
    case "effect.gaussianBlur":
      return "高斯模糊";
    case "effect.basicColorAdjustment":
      return "基础调色";
    case "effect.opacityAdjustment":
      return "不透明度";
    case "mask.rectangle":
      return "矩形蒙版";
    case "mask.ellipse":
      return "椭圆蒙版";
    case "blend.multiply":
      return "正片叠底";
    case "blend.screen":
      return "滤色";
    default:
      return "暂不支持";
  }
}

function effectSliders(filter: Filter): Array<{
  label: string;
  min: number;
  max: number;
  step: number;
  value: number;
  parameter: (value: number) => EffectParameterUpdate;
}> {
  switch (filter.kind.kind) {
    case "gaussianBlur":
      return [
        {
          label: "模糊",
          min: 0,
          max: 3000,
          step: 50,
          value: filter.kind.radiusMillis,
          parameter: (value) => ({ parameter: "gaussianBlurRadiusMillis", radiusMillis: Math.round(value) })
        }
      ];
    case "basicColorAdjustment":
      return [
        {
          label: "亮度",
          min: -1000,
          max: 1000,
          step: 25,
          value: filter.kind.brightnessMillis,
          parameter: (value) => ({ parameter: "basicColorBrightnessMillis", brightnessMillis: Math.round(value) })
        },
        {
          label: "对比度",
          min: 0,
          max: 3000,
          step: 25,
          value: filter.kind.contrastMillis,
          parameter: (value) => ({ parameter: "basicColorContrastMillis", contrastMillis: Math.max(0, Math.round(value)) })
        },
        {
          label: "饱和度",
          min: 0,
          max: 3000,
          step: 25,
          value: filter.kind.saturationMillis,
          parameter: (value) => ({ parameter: "basicColorSaturationMillis", saturationMillis: Math.max(0, Math.round(value)) })
        }
      ];
    case "opacityAdjustment":
      return [
        {
          label: "不透明度",
          min: 0,
          max: 1000,
          step: 10,
          value: filter.kind.opacityMillis,
          parameter: (value) => ({ parameter: "opacityMillis", opacityMillis: Math.max(0, Math.round(value)) })
        }
      ];
    case "externalReference":
      return [];
  }
}

function defaultMask(shape: "rectangle" | "ellipse"): Extract<SegmentMask, { kind: "rectangle" | "ellipse" }> {
  return {
    kind: shape,
    xMillis: 250,
    yMillis: 250,
    widthMillis: 500,
    heightMillis: 500,
    featherMillis: 0,
    opacityMillis: 1000,
    inverted: false
  };
}

function supportLabel(surface: "预览" | "导出", support: CapabilitySupport): string {
  switch (support.state) {
    case "supported":
      return `${surface}支持`;
    case "degraded":
      return `${surface}降级`;
    case "unsupported":
      return "暂不支持";
    case "externalReference":
      return "外部参考";
  }
}

function capabilityActionState(entry: CapabilityReportItem): "supported" | "degraded" | "unsupported" {
  if (entry.preview.state === "externalReference" || entry.export.state === "externalReference") {
    return "unsupported";
  }
  if (entry.preview.state === "unsupported" || entry.export.state === "unsupported") {
    return "unsupported";
  }
  if (entry.preview.state === "degraded" || entry.export.state === "degraded") {
    return "degraded";
  }
  return "supported";
}

function supportChipTone(support: CapabilitySupport): string {
  switch (support.state) {
    case "supported":
      return "ready";
    case "degraded":
      return "warning";
    case "unsupported":
    case "externalReference":
      return "error";
  }
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
  const nearestFocusedKeyframe = nearestKeyframeForProperty(selected, activeFocusedProperty, playheadAt);
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
            allowNearestRemove={true}
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
                active={nearestFocusedKeyframe?.at === keyframe.at}
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
  allowNearestRemove = false,
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
  allowNearestRemove?: boolean;
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
  ) ?? (allowNearestRemove ? nearestKeyframeForProperty(selected, property, playheadAt) ?? undefined : undefined);
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

function shouldUseKeyframeValueInteraction(
  selected: SelectedSegmentView,
  property: KeyframeProperty | undefined
): property is KeyframeProperty {
  return property !== undefined && selected.keyframes.some((keyframe) => keyframe.property === property);
}

function inspectorVisualInteractionPayload(
  selected: SelectedSegmentView,
  playheadAt: number,
  state: VisualFormState,
  property: KeyframeProperty | undefined
): ProjectInteractionPayload | null {
  if (shouldUseKeyframeValueInteraction(selected, property)) {
    const keyframe = nearestKeyframeForProperty(selected, property, playheadAt);
    const value = keyframeValueForVisualProperty(property, state);
    if (keyframe === null || value === null) {
      return null;
    }
    return {
      kind: "keyframeEdit",
      property,
      at: keyframe.at,
      fromAt: keyframe.at,
      value,
      interpolation: keyframe.interpolation,
      easing: keyframe.easing
    };
  }

  const patch = buildVisualPatchFromForm(state);
  return patch === null ? null : { kind: "selectedSegmentVisual", patch };
}

function nearestKeyframeForProperty(
  selected: SelectedSegmentView,
  property: KeyframeProperty,
  playheadAt: number
): Keyframe | null {
  const relativePlayhead = Math.max(
    0,
    Math.min(selected.targetTimerange.duration, playheadAt - selected.targetTimerange.start)
  );
  let nearest: Keyframe | null = null;
  let nearestDistance = Number.POSITIVE_INFINITY;
  for (const keyframe of selected.keyframes) {
    if (keyframe.property !== property) {
      continue;
    }
    const distance = Math.abs(keyframe.at - relativePlayhead);
    if (distance < nearestDistance) {
      nearest = keyframe;
      nearestDistance = distance;
    }
  }
  return nearest;
}

function keyframeValueForVisualProperty(property: KeyframeProperty, state: VisualFormState): KeyframeValue | null {
  switch (property) {
    case "visualPositionX": {
      const value = parseIntegerInRange(state.positionX, -1000, 1000);
      return value === null ? null : { kind: "int", value };
    }
    case "visualPositionY": {
      const value = parseIntegerInRange(state.positionY, -1000, 1000);
      return value === null ? null : { kind: "int", value };
    }
    case "visualScaleX": {
      const value = parseIntegerInRange(state.scaleXMillis, 1, 3000);
      return value === null ? null : { kind: "uint", value };
    }
    case "visualScaleY": {
      const value = parseIntegerInRange(state.scaleYMillis, 1, 3000);
      return value === null ? null : { kind: "uint", value };
    }
    case "visualRotation": {
      const value = parseIntegerInRange(state.rotationDegrees, -360, 360);
      return value === null ? null : { kind: "int", value };
    }
    case "visualOpacity": {
      const value = parseIntegerInRange(state.opacityMillis, 0, 1000);
      return value === null ? null : { kind: "uint", value };
    }
    default:
      return null;
  }
}

function TextNumberField({
  label,
  value,
  min,
  max,
  step,
  suffix,
  disabled = false,
  action,
  onChange,
  onCommit,
  onCancel
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  suffix?: string;
  disabled?: boolean;
  action?: ReactNode;
  onChange: (value: number) => void;
  onCommit?: () => void;
  onCancel?: () => void;
}): React.ReactElement {
  const numberInput = (
    <input
      aria-label={label}
      type="number"
      min={min}
      max={max}
      step={step}
      value={Number.isFinite(value) ? value : ""}
      disabled={disabled}
      onChange={(event) => onChange(event.currentTarget.valueAsNumber)}
      onPointerUp={onCommit}
      onPointerCancel={onCancel}
      onBlur={onCommit}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.currentTarget.blur();
        }
      }}
    />
  );

  return (
    <div className={action === undefined ? "field-row compact-row text-number-row" : "field-row compact-row text-number-row with-action"}>
      <span>{label}</span>
      {suffix === undefined ? (
        numberInput
      ) : (
        <span className="text-input-with-unit">
          {numberInput}
          <span aria-hidden="true">{suffix}</span>
        </span>
      )}
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
    return ["音频", "变速", "动画"];
  }

  if (context.hasText) {
    return ["画面", "变速", "效果", "蒙版", "混合", "动画"];
  }

  if (context.hasAudioSemantics) {
    return ["画面", "音频", "变速", "效果", "滤镜", "调节", "蒙版", "混合", "动画"];
  }

  return ["画面", "变速", "效果", "滤镜", "调节", "蒙版", "混合", "动画"];
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
    return "行高必须是 50% 到 300% 之间。";
  }

  if (!isIntegerInRange(state.letterSpacingMillis, 0, 2000)) {
    return "字间距必须是 0% 到 200% 之间。";
  }

  if (!isIntegerInRange(state.textBoxWidthMillis, 1, 1000) || !isIntegerInRange(state.textBoxHeightMillis, 1, 1000)) {
    return "文本框宽高必须是 0.1% 到 100% 之间。";
  }

  if (
    !isIntegerInRange(state.layoutXMillis, 0, 1000) ||
    !isIntegerInRange(state.layoutYMillis, 0, 1000) ||
    !isIntegerInRange(state.layoutWidthMillis, 1, 1000) ||
    !isIntegerInRange(state.layoutHeightMillis, 1, 1000)
  ) {
    return "布局安全区域必须使用画布百分比。";
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
  selected,
  visual,
  playheadAt,
  pending,
  renderKeyframeButton,
  projectInteractions,
  onUpdateVisual
}: {
  selected: SelectedSegmentView;
  visual: SegmentVisual;
  playheadAt: number;
  pending: boolean;
  renderKeyframeButton: (property: KeyframeProperty, label: string) => React.ReactElement;
  projectInteractions: ProjectInteractionController;
  onUpdateVisual: (patch: SegmentVisualPatch) => void;
}): React.ReactElement {
  const visualKey = useMemo(() => JSON.stringify(visual), [visual]);
  const [visualState, setVisualState] = useState<VisualFormState>(() => visualFormFromVisual(visual));
  const visualCommitKeyRef = useRef<string | null>(null);
  const visualInteractionRef = useRef<VisualInteractionState | null>(null);

  useEffect(() => {
    const nextVisualState = visualFormFromVisual(visual);
    setVisualState(nextVisualState);
    const canonicalPatch = buildVisualPatchFromForm(nextVisualState);
    visualCommitKeyRef.current = canonicalPatch === null ? null : visualPatchKey(canonicalPatch);
  }, [visualKey]);

  const patch = buildVisualPatchFromForm(visualState);
  const validationMessage = validateVisualForm(visualState);
  const changed = patch !== null && visualPatchChangesVisual(visual, patch);

  function updateVisualField(
    field: keyof VisualFormState,
    value: string | boolean,
    options: { provisional?: boolean; keyframeProperty?: KeyframeProperty } = {}
  ): void {
    setVisualState((current) => {
      const next = { ...current, [field]: value };
      if (options.provisional) {
        queueInspectorVisualUpdate(next, options.keyframeProperty);
      }
      return next;
    });
  }

  function updateVisualFieldDraft(field: keyof VisualFormState, value: string | boolean): void {
    setVisualState((current) => ({ ...current, [field]: value }));
  }

  function commitVisualState(state: VisualFormState = visualState): void {
    const nextPatch = buildVisualPatchFromForm(state);
    if (
      nextPatch === null ||
      validateVisualForm(state) !== null ||
      !visualPatchChangesVisual(visual, nextPatch) ||
      pending
    ) {
      return;
    }
    const nextKey = visualPatchKey(nextPatch);
    if (nextKey === visualCommitKeyRef.current) {
      return;
    }
    visualCommitKeyRef.current = nextKey;
    onUpdateVisual(nextPatch);
  }

  function commitVisualFieldEdit(): void {
    commitVisualState();
  }

  function updateAndCommitVisualField(field: keyof VisualFormState, value: string | boolean): void {
    setVisualState((current) => {
      const next = { ...current, [field]: value };
      commitVisualState(next);
      return next;
    });
  }

  function beginInspectorVisualInteraction(property?: KeyframeProperty): VisualInteractionState {
    const existing = visualInteractionRef.current;
    if (existing !== null) {
      return existing;
    }
    const kind = shouldUseKeyframeValueInteraction(selected, property) ? "keyframeEdit" : "selectedSegmentVisual";
    const interaction: VisualInteractionState = {
      kind,
      interactionId: null,
      sequence: 0,
      beginPromise: Promise.resolve(),
      updateInFlight: false,
      rafId: null,
      pendingPayload: null
    };
    interaction.beginPromise = projectInteractions.begin(kind).then((begin) => {
      if (visualInteractionRef.current !== interaction || begin === null) {
        return;
      }
      interaction.interactionId = begin.interactionId;
      flushInspectorVisualUpdate(interaction);
    });
    visualInteractionRef.current = interaction;
    return interaction;
  }

  function queueInspectorVisualUpdate(state: VisualFormState, property?: KeyframeProperty): void {
    const payload = inspectorVisualInteractionPayload(selected, playheadAt, state, property);
    if (payload === null || validateVisualForm(state) !== null || pending) {
      return;
    }
    const interaction = beginInspectorVisualInteraction(property);
    if (interaction.kind !== payload.kind) {
      return;
    }
    interaction.pendingPayload = payload;
    if (interaction.rafId !== null) {
      return;
    }
    interaction.rafId = window.requestAnimationFrame(() => {
      interaction.rafId = null;
      flushInspectorVisualUpdate(interaction);
    });
  }

  function flushInspectorVisualUpdate(interaction: VisualInteractionState): void {
    if (interaction.updateInFlight || interaction.interactionId === null || interaction.pendingPayload === null) {
      return;
    }
    const payload = interaction.pendingPayload;
    interaction.pendingPayload = null;
    interaction.updateInFlight = true;
    interaction.sequence += 1;
    void projectInteractions.update(interaction.interactionId, interaction.sequence, payload).then(() => {
      interaction.updateInFlight = false;
      if (visualInteractionRef.current !== interaction) {
        return;
      }
      flushInspectorVisualUpdate(interaction);
    });
  }

  async function finishInspectorVisualInteraction(action: "commit" | "cancel"): Promise<void> {
    const interaction = visualInteractionRef.current;
    if (interaction === null) {
      return;
    }
    if (interaction.rafId !== null) {
      window.cancelAnimationFrame(interaction.rafId);
      interaction.rafId = null;
    }
    await interaction.beginPromise;
    while (interaction.updateInFlight) {
      await new Promise((resolve) => window.setTimeout(resolve, 0));
    }
    if (interaction.pendingPayload !== null) {
      flushInspectorVisualUpdate(interaction);
      while (interaction.updateInFlight || interaction.pendingPayload !== null) {
        await new Promise((resolve) => window.setTimeout(resolve, 0));
      }
    }
    visualInteractionRef.current = null;
    if (interaction.interactionId === null) {
      return;
    }
    if (action === "commit") {
      const nextPatch = interaction.kind === "selectedSegmentVisual" ? buildVisualPatchFromForm(visualState) : null;
      if (nextPatch !== null) {
        visualCommitKeyRef.current = visualPatchKey(nextPatch);
      }
      await projectInteractions.commit(interaction.interactionId);
      return;
    }
    await projectInteractions.cancel(interaction.interactionId);
  }

  return (
    <div className="visual-controls" aria-label="画面基础表单">
      <label className="toggle-row compact-toggle visual-toggle-row">
        <input
          type="checkbox"
          checked={visualState.visible}
          onChange={(event) => updateAndCommitVisualField("visible", event.currentTarget.checked)}
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
        onFirstPreviewChange={(value) => updateVisualField("positionX", value, { provisional: true, keyframeProperty: "visualPositionX" })}
        onSecondPreviewChange={(value) => updateVisualField("positionY", value, { provisional: true, keyframeProperty: "visualPositionY" })}
        onFirstValueChange={(value) => updateVisualFieldDraft("positionX", value)}
        onSecondValueChange={(value) => updateVisualFieldDraft("positionY", value)}
        onFirstCommit={commitVisualFieldEdit}
        onSecondCommit={commitVisualFieldEdit}
        onFirstInteractionStart={() => beginInspectorVisualInteraction("visualPositionX")}
        onSecondInteractionStart={() => beginInspectorVisualInteraction("visualPositionY")}
        onInteractionCommit={() => void finishInspectorVisualInteraction("commit")}
        onInteractionCancel={() => void finishInspectorVisualInteraction("cancel")}
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
        display={PERCENT_VALUE_DISPLAY}
        firstValue={visualState.scaleXMillis}
        secondValue={visualState.scaleYMillis}
        disabled={pending}
        onFirstPreviewChange={(value) => updateVisualField("scaleXMillis", value, { provisional: true, keyframeProperty: "visualScaleX" })}
        onSecondPreviewChange={(value) => updateVisualField("scaleYMillis", value, { provisional: true, keyframeProperty: "visualScaleY" })}
        onFirstValueChange={(value) => updateVisualFieldDraft("scaleXMillis", value)}
        onSecondValueChange={(value) => updateVisualFieldDraft("scaleYMillis", value)}
        onFirstCommit={commitVisualFieldEdit}
        onSecondCommit={commitVisualFieldEdit}
        onFirstInteractionStart={() => beginInspectorVisualInteraction("visualScaleX")}
        onSecondInteractionStart={() => beginInspectorVisualInteraction("visualScaleY")}
        onInteractionCommit={() => void finishInspectorVisualInteraction("commit")}
        onInteractionCancel={() => void finishInspectorVisualInteraction("cancel")}
        firstAction={renderKeyframeButton("visualScaleX", "缩放 X")}
        secondAction={renderKeyframeButton("visualScaleY", "缩放 Y")}
      />

      <VisualSingleControl
        label="旋转"
        min={-360}
        max={360}
        step={1}
        display={DEGREE_DISPLAY}
        value={visualState.rotationDegrees}
        disabled={pending}
        onPreviewChange={(value) => updateVisualField("rotationDegrees", value, { provisional: true, keyframeProperty: "visualRotation" })}
        onValueChange={(value) => updateVisualFieldDraft("rotationDegrees", value)}
        onCommit={commitVisualFieldEdit}
        onInteractionStart={() => beginInspectorVisualInteraction("visualRotation")}
        onInteractionCommit={() => void finishInspectorVisualInteraction("commit")}
        onInteractionCancel={() => void finishInspectorVisualInteraction("cancel")}
        action={renderKeyframeButton("visualRotation", "旋转")}
      />

      <VisualSingleControl
        label="不透明度"
        min={0}
        max={1000}
        step={10}
        display={OPACITY_PERCENT_DISPLAY}
        value={visualState.opacityMillis}
        disabled={pending}
        onPreviewChange={(value) => updateVisualField("opacityMillis", value, { provisional: true, keyframeProperty: "visualOpacity" })}
        onValueChange={(value) => updateVisualFieldDraft("opacityMillis", value)}
        onCommit={commitVisualFieldEdit}
        onInteractionStart={() => beginInspectorVisualInteraction("visualOpacity")}
        onInteractionCommit={() => void finishInspectorVisualInteraction("commit")}
        onInteractionCancel={() => void finishInspectorVisualInteraction("cancel")}
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
              onClick={() => updateAndCommitVisualField("fitMode", fitMode)}
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
            display={CROP_PERCENT_DISPLAY}
            value={visualState.cropLeftMillis}
            disabled={pending}
            onChange={(value) => updateVisualFieldDraft("cropLeftMillis", value)}
            onCommit={commitVisualFieldEdit}
          />
          <VisualCompactNumber
            label="右"
            ariaLabel="裁剪 右"
            min={0}
            max={999}
            step={10}
            display={CROP_PERCENT_DISPLAY}
            value={visualState.cropRightMillis}
            disabled={pending}
            onChange={(value) => updateVisualFieldDraft("cropRightMillis", value)}
            onCommit={commitVisualFieldEdit}
          />
          <VisualCompactNumber
            label="上"
            ariaLabel="裁剪 上"
            min={0}
            max={999}
            step={10}
            display={CROP_PERCENT_DISPLAY}
            value={visualState.cropTopMillis}
            disabled={pending}
            onChange={(value) => updateVisualFieldDraft("cropTopMillis", value)}
            onCommit={commitVisualFieldEdit}
          />
          <VisualCompactNumber
            label="下"
            ariaLabel="裁剪 下"
            min={0}
            max={999}
            step={10}
            display={CROP_PERCENT_DISPLAY}
            value={visualState.cropBottomMillis}
            disabled={pending}
            onChange={(value) => updateVisualFieldDraft("cropBottomMillis", value)}
            onCommit={commitVisualFieldEdit}
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
              onClick={() => updateAndCommitVisualField("backgroundKind", backgroundKind)}
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
              onChange={(event) => updateVisualFieldDraft("backgroundColor", event.currentTarget.value)}
              onPointerUp={commitVisualFieldEdit}
              onBlur={commitVisualFieldEdit}
              disabled={pending}
            />
            <input
              aria-label="背景填充色值"
              type="text"
              value={visualState.backgroundColor}
              onChange={(event) => updateVisualFieldDraft("backgroundColor", event.currentTarget.value)}
              onBlur={commitVisualFieldEdit}
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
  display,
  firstValue,
  secondValue,
  disabled,
  onFirstPreviewChange,
  onSecondPreviewChange,
  onFirstValueChange,
  onSecondValueChange,
  onFirstCommit,
  onSecondCommit,
  onFirstInteractionStart,
  onSecondInteractionStart,
  onInteractionCommit,
  onInteractionCancel,
  firstAction,
  secondAction
}: {
  label: string;
  firstLabel: string;
  secondLabel: string;
  min: number;
  max: number;
  step: number;
  display?: VisualDisplayTransform;
  firstValue: string;
  secondValue: string;
  disabled: boolean;
  onFirstPreviewChange: (value: string) => void;
  onSecondPreviewChange: (value: string) => void;
  onFirstValueChange: (value: string) => void;
  onSecondValueChange: (value: string) => void;
  onFirstCommit: () => void;
  onSecondCommit: () => void;
  onFirstInteractionStart: () => void;
  onSecondInteractionStart: () => void;
  onInteractionCommit: () => void;
  onInteractionCancel: () => void;
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
          display={display}
          value={firstValue}
          disabled={disabled}
          onPreviewChange={onFirstPreviewChange}
          onValueChange={onFirstValueChange}
          onCommit={onFirstCommit}
          onInteractionStart={onFirstInteractionStart}
          onInteractionCommit={onInteractionCommit}
          onInteractionCancel={onInteractionCancel}
          action={firstAction}
        />
        <VisualRangeNumber
          label={label}
          shortLabel={secondLabel}
          min={min}
          max={max}
          step={step}
          display={display}
          value={secondValue}
          disabled={disabled}
          onPreviewChange={onSecondPreviewChange}
          onValueChange={onSecondValueChange}
          onCommit={onSecondCommit}
          onInteractionStart={onSecondInteractionStart}
          onInteractionCommit={onInteractionCommit}
          onInteractionCancel={onInteractionCancel}
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
  display,
  value,
  disabled,
  onPreviewChange,
  onValueChange,
  onCommit,
  onInteractionStart,
  onInteractionCommit,
  onInteractionCancel,
  action
}: {
  label: string;
  min: number;
  max: number;
  step: number;
  display?: VisualDisplayTransform;
  value: string;
  disabled: boolean;
  onPreviewChange: (value: string) => void;
  onValueChange: (value: string) => void;
  onCommit: () => void;
  onInteractionStart: () => void;
  onInteractionCommit: () => void;
  onInteractionCancel: () => void;
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
        display={display}
        value={value}
        disabled={disabled}
        onPreviewChange={onPreviewChange}
        onValueChange={onValueChange}
        onCommit={onCommit}
        onInteractionStart={onInteractionStart}
        onInteractionCommit={onInteractionCommit}
        onInteractionCancel={onInteractionCancel}
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
  display,
  value,
  disabled,
  onPreviewChange,
  onValueChange,
  onCommit,
  onInteractionStart,
  onInteractionCommit,
  onInteractionCancel,
  action
}: {
  label: string;
  shortLabel: string;
  min: number;
  max: number;
  step: number;
  display?: VisualDisplayTransform;
  value: string;
  disabled: boolean;
  onPreviewChange: (value: string) => void;
  onValueChange: (value: string) => void;
  onCommit: () => void;
  onInteractionStart: () => void;
  onInteractionCommit: () => void;
  onInteractionCancel: () => void;
  action?: ReactNode;
}): React.ReactElement {
  const controlMin = display?.min ?? min;
  const controlMax = display?.max ?? max;
  const controlStep = display?.step ?? step;
  const displayValue = display === undefined ? value : display.toDisplay(value);
  const rangeValue = clamp(Number.parseFloat(displayValue) || 0, controlMin, controlMax);
  const numberAriaLabel = shortLabel === "数值" ? label : `${label} ${shortLabel}`;
  const numberInput = (
    <input
      aria-label={numberAriaLabel}
      type="number"
      min={controlMin}
      max={controlMax}
      step={controlStep}
      value={displayValue}
      onChange={(event) => onValueChange(display === undefined ? event.currentTarget.value : display.fromDisplay(event.currentTarget.value))}
      onBlur={onCommit}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.currentTarget.blur();
        }
      }}
      disabled={disabled}
    />
  );

  return (
    <div className={action === undefined ? "visual-range-number" : "visual-range-number with-keyframe"}>
      <span>{shortLabel}</span>
      <input
        aria-label={`${numberAriaLabel}滑杆`}
        type="range"
        min={controlMin}
        max={controlMax}
        step={controlStep}
        value={rangeValue}
        onPointerDown={() => onInteractionStart()}
        onPointerUp={() => onInteractionCommit()}
        onPointerCancel={() => onInteractionCancel()}
        onChange={(event) => onPreviewChange(display === undefined ? event.currentTarget.value : display.fromDisplay(event.currentTarget.value))}
        disabled={disabled}
      />
      {display === undefined ? (
        numberInput
      ) : (
        <span className="visual-input-with-unit">
          {numberInput}
          <span aria-hidden="true">{display.suffix}</span>
        </span>
      )}
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
  display,
  value,
  disabled,
  onChange,
  onCommit
}: {
  label: string;
  ariaLabel: string;
  min: number;
  max: number;
  step: number;
  display?: VisualDisplayTransform;
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
  onCommit?: () => void;
}): React.ReactElement {
  const controlMin = display?.min ?? min;
  const controlMax = display?.max ?? max;
  const controlStep = display?.step ?? step;
  const displayValue = display === undefined ? value : display.toDisplay(value);
  const numberInput = (
    <input
      aria-label={ariaLabel}
      type="number"
      min={controlMin}
      max={controlMax}
      step={controlStep}
      value={displayValue}
      onChange={(event) => onChange(display === undefined ? event.currentTarget.value : display.fromDisplay(event.currentTarget.value))}
      onBlur={onCommit}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.currentTarget.blur();
        }
      }}
      disabled={disabled}
    />
  );

  return (
    <label className="visual-compact-number">
      <span>{label}</span>
      {display === undefined ? (
        numberInput
      ) : (
        <span className="visual-input-with-unit">
          {numberInput}
          <span aria-hidden="true">{display.suffix}</span>
        </span>
      )}
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
    return "缩放必须是 0.1% 到 300% 之间。";
  }

  if (parseIntegerInRange(state.rotationDegrees, -360, 360) === null) {
    return "旋转必须是 -360° 到 360° 之间。";
  }

  if (parseIntegerInRange(state.opacityMillis, 0, 1000) === null) {
    return "不透明度必须是 0% 到 100% 之间。";
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
    return "裁剪必须是 0% 到 99.9% 之间。";
  }

  if (cropLeftMillis + cropRightMillis >= 1000 || cropTopMillis + cropBottomMillis >= 1000) {
    return "左右或上下裁剪总和必须小于 100%。";
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

function millisStringToPercentString(value: string): string {
  if (value.trim().length === 0) {
    return "";
  }
  const parsed = Number.parseFloat(value);
  if (!Number.isFinite(parsed)) {
    return value;
  }
  return formatCompactNumber(parsed / 10);
}

function percentStringToMillisString(value: string): string {
  if (value.trim().length === 0) {
    return "";
  }
  const parsed = Number.parseFloat(value);
  if (!Number.isFinite(parsed)) {
    return value;
  }
  return String(Math.round(parsed * 10));
}

function millisToPercentValue(value: number): number {
  return Math.round((value / 10) * 10) / 10;
}

function percentToMillisValue(value: number): number {
  return Math.max(0, Math.round(value * 10));
}

function formatCompactNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "";
  }
  const rounded = Math.round(value * 10) / 10;
  return Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1);
}

function toBoundedNumber(value: number, fallback: number, min: number, max: number): number {
  const rounded = Math.round(Number.isFinite(value) ? value : fallback);
  return clamp(rounded, min, max);
}

function toBoundedFloat(value: number, fallback: number, min: number, max: number): number {
  return clamp(Number.isFinite(value) ? value : fallback, min, max);
}

function microsecondsToSeconds(value: number): number {
  return Math.round((Math.max(0, value) / 1_000_000) * 10) / 10;
}

function secondsToNonNegativeMicroseconds(value: number): number {
  return Math.max(0, Math.round(value * 1_000_000));
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

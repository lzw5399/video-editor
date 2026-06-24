import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent
} from "react";

import type { DraftCanvasConfig, SegmentVisual } from "../../generated/Draft";
import type { SegmentVisualPatch } from "../../main/nativeBinding";
import type { ProjectInteractionController, ProjectInteractionEvidence } from "./projectInteraction";
import { appIconUrls, type AppIconName } from "../assets/icons";
import {
  canvasBackgroundTone,
  formatCanvasAspectRatio,
  formatCanvasBackgroundStatus,
  formatCanvasReadout,
  formatMicroseconds,
  formatRealtimePreviewBackendLabel,
  formatRealtimePreviewFallbackReason,
  summarizeRealtimePreviewDisplay,
  summarizeRealtimePreviewProductDisplay,
  type SelectedSegmentView,
  type AudioParityDisplayModel,
  type AudioDeviceDisplayModel,
  type AudioPreviewDisplayModel,
  type BindingStatus,
  type PreviewDisplayState,
  type RealtimePreviewDisplayModel,
  type RealtimePreviewFallbackReason,
  type RuntimeDiagnosticsDisplayState,
  type RuntimeDiagnosticsRow,
  type RuntimeDiagnosticsTone,
  type WaveformDisplayModel
} from "../viewModel";

import "./preview-inspector.css";

type PreviewMonitorProps = {
  draftName: string;
  canvasConfig: DraftCanvasConfig;
  bindingStatus: BindingStatus;
  preview: PreviewDisplayState;
  resourcePreviewStatusLabel: string | null;
  audioPreview: AudioPreviewDisplayModel;
  audioDevices: AudioDeviceDisplayModel;
  audioParity: AudioParityDisplayModel;
  waveform: WaveformDisplayModel;
  runtimeDiagnostics: RuntimeDiagnosticsDisplayState;
  selectedSegment: SelectedSegmentView | null;
  showDeveloperDiagnostics: boolean;
  pending: boolean;
  audioPending: boolean;
  nativeSurfaceSuspended?: boolean;
  playheadUs?: number;
  timelineDurationUs: number;
  playbackRunning: boolean;
  projectInteractions: ProjectInteractionController;
  onRealtimePreviewHostStateChange: (state: RealtimePreviewHostState) => void;
  onPlayheadChange: (value: number) => void;
  onTogglePlayback: () => void;
  onStopPlayback: () => void;
  onProbeRuntimeCapabilities: () => void;
  onRetryAudioPreview: () => void;
  onSelectPreviewTextOverlay: (selectionHandle: string) => void;
  onEditPreviewTextOverlay: (selectionHandle: string) => void;
};

type MonitorControl = {
  label: string;
  icon?: AppIconName;
  imageIcon?: AppIconName;
  symbol?: string;
};

type RealtimePreviewHostRect = {
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactorMillis: number;
};

type PendingRealtimePreviewHostRect = {
  key: string;
  rect: RealtimePreviewHostRect;
};

type RealtimePreviewHostTelemetry = {
  firstFrameLatencyMs: number | null;
  seekLatencyMs: number | null;
  queueLatencyMs: number;
  renderDurationMs: number;
  presentedFrameCount: number;
  droppedFrameCount: number;
  repeatedFrameCount: number;
  staleRejectedCount: number;
  canceledRequestCount: number;
  fallbackCount: number;
  cacheHitCount: number;
  targetTimeMicroseconds: number;
  playbackGeneration: number;
  framePacing: RealtimePreviewFramePacingTelemetry;
};

type RealtimePreviewFramePacingTelemetry = {
  sampleCount: number;
  intervalP50Ms: number | null;
  intervalP95Ms: number | null;
  intervalMaxMs: number | null;
  scheduleLatenessP95Ms: number | null;
  scheduleLatenessMaxMs: number | null;
  samples: RealtimePreviewFramePacingSample[];
};

type RealtimePreviewFramePacingSample = {
  targetTimeMicroseconds: number;
  intervalMs?: number | null;
  scheduleLatenessMs: number;
  renderDurationMs: number;
  droppedFrameCount: number;
};

export type RealtimePreviewHostState = {
  ok: boolean;
  productReady: boolean;
  hostAttached: boolean;
  fallbackActive: boolean;
  statusLabel: string;
  fallbackLabel: string | null;
  unsupportedReason: string | null;
  playbackGeneration: number | null;
  backend: "renderGraphGpu" | "none";
  diagnosticSource: "nativeVideoBridge" | "runtimeFrameRequest" | "none";
  fallbackReason: RealtimePreviewFallbackReason | null;
  currentRequestCanceled: boolean;
  fallbackArtifactVisible: boolean;
  telemetry: RealtimePreviewHostTelemetry | null;
  frameDisplay: RealtimePreviewHostFrameDisplay | null;
  contentEvidence: RealtimePreviewHostContentEvidence | null;
  surfacePlacement: RealtimePreviewHostSurfacePlacement | null;
};

type RealtimePreviewScreenRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

type RealtimePreviewHostSurfacePlacement = {
  surfaceBoundsCoordinateSpace: "browserWindowContentLogicalPixels";
  screenRectCoordinateSpace: "electronScreenLogicalPixels";
  hostScreenRect: RealtimePreviewScreenRect;
  nativeScreenRect: RealtimePreviewScreenRect;
  nativeAppKitScreenRect: RealtimePreviewScreenRect;
  nativeDrawableLifecycleDiagnostic: string | null;
  deltaPx: RealtimePreviewScreenRect;
  maxDeltaPx: number;
  aligned: boolean;
};

type RealtimePreviewHostFrameDisplay = {
  surfaceKind: "mock";
  frameToken: string;
  targetTimeMicroseconds: number;
  dominantColor: string;
  accentColor: string;
};

type RealtimePreviewHostContentEvidence = {
  source: "nativeVideoBridge" | "renderGraphGpuComposited";
  digest: string;
  width: number;
  height: number;
  byteCount: number;
  targetTimeMicroseconds: number;
  presentedFrames: number;
  submittedDraws: number;
  activeTextOverlays?: RealtimePreviewTextOverlayEvidence[];
};

type RealtimePreviewTextOverlayEvidence = {
  trackId: string;
  segmentId: string;
  selectionHandle: string;
  source: "text" | "subtitle";
  content: string;
  fontFamily: string;
  fontRef?: string | null;
  fontSize: number;
  color: string;
  alignment: "left" | "center" | "right";
  lineHeightMillis: number;
  letterSpacingMillis: number;
  x: number;
  y: number;
  width: number;
  height: number;
  visualPositionX: number;
  visualPositionY: number;
  visualScaleXMillis: number;
  visualScaleYMillis: number;
  visualRotationDegrees: number;
  visualOpacityMillis: number;
  selected?: boolean;
};

type RealtimePreviewTextHitTestResponse = {
  hit: boolean;
  selectionHandle?: string | null;
};

export type RealtimePreviewHostApi = {
  updateHostRect: (rect: RealtimePreviewHostRect) => Promise<RealtimePreviewHostState>;
  detachSurface: () => Promise<RealtimePreviewHostState>;
  subscribeTelemetry: (listener: (state: RealtimePreviewHostState) => void) => () => void;
  updateProjectSessionSnapshot: (
    projectSessionId: string,
    expectedRevision: number,
    interactionId?: string | null
  ) => Promise<RealtimePreviewHostState>;
  seek: (targetTimeMicroseconds: number) => Promise<RealtimePreviewHostState>;
  play: () => Promise<RealtimePreviewHostState>;
  pause: () => Promise<RealtimePreviewHostState>;
  stop: () => Promise<RealtimePreviewHostState>;
  hitTestTextOverlay: (point: { x: number; y: number }) => Promise<RealtimePreviewTextHitTestResponse>;
};

type PreviewDragState = {
  mode: "move" | "rotate";
  pointerId: number;
  startClientX: number;
  startClientY: number;
  lastClientX: number;
  lastClientY: number;
  canvasWidth: number;
  canvasHeight: number;
  moved: boolean;
  sequence: number;
  beginPromise: Promise<void>;
  interactionId: string | null;
  interactionGeneration: number | null;
  updateInFlight: boolean;
  rafId: number | null;
  pendingPayload: SegmentVisualPatch | null;
  pendingMetrics: PreviewDragMetrics | null;
  acceptedMoveDeltaX: number;
  acceptedMoveDeltaY: number;
  acceptedRotationDeltaDegrees: number;
  centerClientX?: number;
  centerClientY?: number;
  startAngleDegrees?: number;
};

type PreviewDragMetrics =
  | {
      mode: "move";
      deltaClientX: number;
      deltaClientY: number;
    }
  | {
      mode: "rotate";
      deltaDegrees: number;
    };

type PreviewDragPreviewState =
  | {
      mode: "move";
      deltaClientX: number;
      deltaClientY: number;
    }
  | {
      mode: "rotate";
      deltaDegrees: number;
    };

type CanvasFitSize = {
  width: number;
  height: number;
};

type SelectionOverlayModel = {
  style: CSSProperties;
  source: "native-text" | "segment-visual";
  selectionHandle: string | null;
  rotateEnabled: boolean;
};

type MonitorViewControl = {
  label: string;
  className?: string;
  icon?: AppIconName;
  value?: string;
};

declare global {
  interface Window {
    videoEditorRealtimePreviewHost?: RealtimePreviewHostApi;
  }
}

const MICROSECONDS_PER_SECOND = 1_000_000;
const INITIAL_REALTIME_PREVIEW_HOST_STATE: RealtimePreviewHostState = {
  ok: false,
  productReady: false,
  hostAttached: false,
  fallbackActive: false,
  statusLabel: "实时预览等待接入",
  fallbackLabel: null,
  unsupportedReason: null,
  playbackGeneration: null,
  backend: "none",
  diagnosticSource: "none",
  fallbackReason: null,
  currentRequestCanceled: false,
  fallbackArtifactVisible: false,
  telemetry: null,
  frameDisplay: null,
  contentEvidence: null,
  surfacePlacement: null
};

const MONITOR_CONTROLS: readonly MonitorControl[] = [
  { label: "停止", imageIcon: "previewStop" },
  { label: "上一帧", icon: "previewPreviousFrame" },
  { label: "下一帧", icon: "previewNextFrame" }
];

const MONITOR_TITLE = "播放器-时间线01";

export function PreviewMonitor({
  draftName,
  canvasConfig,
  bindingStatus,
  preview,
  resourcePreviewStatusLabel,
  audioPreview,
  audioDevices,
  audioParity,
  waveform,
  runtimeDiagnostics,
  selectedSegment,
  showDeveloperDiagnostics,
  pending,
  audioPending,
  nativeSurfaceSuspended = false,
  playheadUs = 0,
  timelineDurationUs,
  playbackRunning,
  onRealtimePreviewHostStateChange,
  onPlayheadChange,
  onTogglePlayback,
  onStopPlayback,
  projectInteractions,
  onProbeRuntimeCapabilities,
  onRetryAudioPreview,
  onSelectPreviewTextOverlay,
  onEditPreviewTextOverlay
}: PreviewMonitorProps): React.ReactElement {
  const nativeHostRef = useRef<HTMLDivElement>(null);
  const previewStageRef = useRef<HTMLDivElement>(null);
  const lastSentHostRectRef = useRef<string | null>(null);
  const previewDragRef = useRef<PreviewDragState | null>(null);
  const [nativeHostState, setNativeHostState] = useState<RealtimePreviewHostState>(INITIAL_REALTIME_PREVIEW_HOST_STATE);
  const [canvasFitSize, setCanvasFitSize] = useState<CanvasFitSize | null>(null);
  const [previewDragPreview, setPreviewDragPreview] = useState<PreviewDragPreviewState | null>(null);
  const [previewInteractionEvidence, setPreviewInteractionEvidence] = useState<ProjectInteractionEvidence | null>(null);
  const safePlayheadUs = Math.max(0, Math.round(playheadUs));
  const safeTimelineDurationUs = Math.max(0, Math.round(timelineDurationUs));
  const frameStepUs = frameDurationUs(canvasConfig);
  const canvasReadout = formatCanvasReadout(canvasConfig);
  const canvasRatio = formatCanvasAspectRatio(canvasConfig);
  const backgroundStatus = formatCanvasBackgroundStatus(canvasConfig);
  const backgroundTone = canvasBackgroundTone(canvasConfig);
  const canvasStyle = {
    aspectRatio: `${Math.max(1, canvasConfig.width)} / ${Math.max(1, canvasConfig.height)}`,
    width: canvasFitSize === null ? undefined : `${canvasFitSize.width}px`,
    height: canvasFitSize === null ? undefined : `${canvasFitSize.height}px`,
    background: canvasConfig.background.kind === "solidColor" ? canvasConfig.background.color : "#070707"
  } as CSSProperties;
  const monitorViewControls: readonly MonitorViewControl[] = [
    { label: "原画", className: "original-button", value: "原画" },
    { label: "适应窗口", icon: "previewFit" },
    { label: "画布读数", value: "画布" },
    { label: "画面比例", className: "ratio-button", value: canvasRatio },
    { label: "全屏", value: "全屏" }
  ];
  const schedulerProductStatusLabel =
    !showDeveloperDiagnostics &&
    runtimeDiagnostics.schedulerStatusLabel !== null &&
    runtimeDiagnostics.schedulerStatusLabel !== "调度服务就绪"
      ? runtimeDiagnostics.schedulerStatusLabel
      : null;
  const previewPlaceholderLabel =
    schedulerProductStatusLabel ??
    (selectedSegment === null ? "添加素材到时间线后显示预览" : pending ? "正在准备预览画面" : "实时预览准备中");
  const showRealtimeSurface = !nativeSurfaceSuspended && nativeHostState.productReady && !nativeHostState.fallbackActive;
  const productPreviewStatusLabel = formatProductPreviewStatus(preview, previewPlaceholderLabel, pending);
  const previewStatusLabel = showDeveloperDiagnostics
    ? preview.error ?? preview.statusLabel
    : productPreviewStatusLabel === "画面已更新，预览待刷新"
      ? productPreviewStatusLabel
      : resourcePreviewStatusLabel ?? productPreviewStatusLabel;
  const runtimePreviewUnavailable = !runtimeDiagnostics.canPreview;
  const previewPlaybackLabel =
    runtimePreviewUnavailable && !playbackRunning ? "预览暂不可用" : previewControlLabel("播放", playbackRunning);
  const previewPlaybackTitle =
    runtimePreviewUnavailable && !playbackRunning
      ? runtimeDiagnostics.statusDetail || runtimeDiagnostics.statusLabel
      : previewPlaybackLabel;
  const selectionOverlay = buildSelectionOverlayModel(
    selectedSegment,
    nativeHostState.contentEvidence,
    previewDragPreview
  );
  const textOverlayStyle = !showRealtimeSurface ? buildTextOverlayStyle(selectedSegment, previewDragPreview) : null;

  function handlePreviewDragPointerDown(event: ReactPointerEvent<HTMLDivElement>): void {
    if (selectedSegment === null || !selectedSegment.visual.visible) {
      return;
    }

    const canvas = event.currentTarget.closest(".preview-canvas");
    if (!(canvas instanceof HTMLElement)) {
      return;
    }
    const canvasRect = canvas.getBoundingClientRect();
    if (canvasRect.width <= 0 || canvasRect.height <= 0) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    const drag: PreviewDragState = {
      mode: "move",
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      lastClientX: event.clientX,
      lastClientY: event.clientY,
      canvasWidth: canvasRect.width,
      canvasHeight: canvasRect.height,
      moved: false,
      sequence: 0,
      beginPromise: Promise.resolve(),
      interactionId: null,
      interactionGeneration: null,
      updateInFlight: false,
      rafId: null,
      pendingPayload: null,
      pendingMetrics: null,
      acceptedMoveDeltaX: 0,
      acceptedMoveDeltaY: 0,
      acceptedRotationDeltaDegrees: 0
    };
    drag.beginPromise = beginPreviewTransformInteraction(drag);
    previewDragRef.current = drag;
    setPreviewInteractionEvidence(null);
    setPreviewDragPreview({ mode: "move", deltaClientX: 0, deltaClientY: 0 });
  }

  function handlePreviewRotatePointerDown(event: ReactPointerEvent<HTMLButtonElement>): void {
    if (selectedSegment === null || !selectedSegment.visual.visible) {
      return;
    }

    const outline = event.currentTarget.closest(".preview-selection-outline");
    const canvas = event.currentTarget.closest(".preview-canvas");
    if (!(outline instanceof HTMLElement) || !(canvas instanceof HTMLElement)) {
      return;
    }
    const outlineRect = outline.getBoundingClientRect();
    const canvasRect = canvas.getBoundingClientRect();
    if (outlineRect.width <= 0 || outlineRect.height <= 0 || canvasRect.width <= 0 || canvasRect.height <= 0) {
      return;
    }

    const centerClientX = outlineRect.left + outlineRect.width / 2;
    const centerClientY = outlineRect.top + outlineRect.height / 2;
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    const drag: PreviewDragState = {
      mode: "rotate",
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      lastClientX: event.clientX,
      lastClientY: event.clientY,
      canvasWidth: canvasRect.width,
      canvasHeight: canvasRect.height,
      moved: false,
      sequence: 0,
      beginPromise: Promise.resolve(),
      interactionId: null,
      interactionGeneration: null,
      updateInFlight: false,
      rafId: null,
      pendingPayload: null,
      pendingMetrics: null,
      acceptedMoveDeltaX: 0,
      acceptedMoveDeltaY: 0,
      acceptedRotationDeltaDegrees: 0,
      centerClientX,
      centerClientY,
      startAngleDegrees: pointerAngleDegrees(event.clientX, event.clientY, centerClientX, centerClientY)
    };
    drag.beginPromise = beginPreviewTransformInteraction(drag);
    previewDragRef.current = drag;
    setPreviewInteractionEvidence(null);
    setPreviewDragPreview({ mode: "rotate", deltaDegrees: 0 });
  }

  function handlePreviewDragPointerMove(event: ReactPointerEvent<HTMLElement>): void {
    const drag = previewDragRef.current;
    if (drag === null || drag.pointerId !== event.pointerId) {
      return;
    }
    event.preventDefault();
    drag.lastClientX = event.clientX;
    drag.lastClientY = event.clientY;
    drag.moved =
      drag.moved ||
      Math.abs(event.clientX - drag.startClientX) + Math.abs(event.clientY - drag.startClientY) > 2;
    if (drag.mode === "rotate") {
      const metrics = previewRotateMetrics(drag, event.clientX, event.clientY);
      setPreviewDragPreview({
        mode: "rotate",
        deltaDegrees: normalizeRotationDegrees(metrics.deltaDegrees - drag.acceptedRotationDeltaDegrees)
      });
      queuePreviewTransformUpdate(drag, previewVisualPatchFromMetrics(drag, metrics), metrics);
      return;
    }
    const metrics = previewMoveMetrics(drag, event.clientX, event.clientY);
    setPreviewDragPreview({
      mode: "move",
      deltaClientX: metrics.deltaClientX - drag.acceptedMoveDeltaX,
      deltaClientY: metrics.deltaClientY - drag.acceptedMoveDeltaY
    });
    queuePreviewTransformUpdate(drag, previewVisualPatchFromMetrics(drag, metrics), metrics);
  }

  function handlePreviewDragPointerUp(event: ReactPointerEvent<HTMLElement>): void {
    const drag = previewDragRef.current;
    if (drag === null || drag.pointerId !== event.pointerId) {
      return;
    }
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    void finishPreviewTransformInteraction(drag, drag.moved ? "commit" : "cancel");
  }

  function handlePreviewDragPointerCancel(event: ReactPointerEvent<HTMLElement>): void {
    const drag = previewDragRef.current;
    if (drag !== null && drag.pointerId === event.pointerId) {
      void finishPreviewTransformInteraction(drag, "cancel");
    }
  }

  async function beginPreviewTransformInteraction(drag: PreviewDragState): Promise<void> {
    const begin = await projectInteractions.begin("selectedSegmentVisual");
    if (previewDragRef.current !== drag) {
      if (begin !== null) {
        void projectInteractions.cancel(begin.interactionId);
      }
      return;
    }
    if (begin === null) {
      if (drag.rafId !== null) {
        window.cancelAnimationFrame(drag.rafId);
        drag.rafId = null;
      }
      drag.pendingPayload = null;
      drag.pendingMetrics = null;
      previewDragRef.current = null;
      setPreviewDragPreview(null);
      setPreviewInteractionEvidence(null);
      return;
    }
    drag.interactionId = begin.interactionId;
    drag.interactionGeneration = begin.generation;
    flushPreviewTransformUpdate(drag);
  }

  function queuePreviewTransformUpdate(
    drag: PreviewDragState,
    payload: SegmentVisualPatch,
    metrics: PreviewDragMetrics
  ): void {
    drag.pendingPayload = payload;
    drag.pendingMetrics = metrics;
    if (drag.rafId !== null) {
      return;
    }
    drag.rafId = window.requestAnimationFrame(() => {
      drag.rafId = null;
      flushPreviewTransformUpdate(drag);
    });
  }

  function flushPreviewTransformUpdate(drag: PreviewDragState): void {
    if (drag.updateInFlight || drag.interactionId === null || drag.pendingPayload === null || drag.pendingMetrics === null) {
      return;
    }
    const payload = drag.pendingPayload;
    const metrics = drag.pendingMetrics;
    drag.pendingPayload = null;
    drag.pendingMetrics = null;
    drag.updateInFlight = true;
    const sequence = drag.sequence + 1;
    drag.sequence = sequence;
    void projectInteractions.update(drag.interactionId, sequence, {
      kind: "selectedSegmentVisual",
      patch: payload
    }).then((update) => {
      drag.updateInFlight = false;
      if (previewDragRef.current !== drag || update === null) {
        return;
      }
      drag.interactionGeneration = update.generation;
      acceptPreviewDragMetrics(drag, metrics);
      setPreviewInteractionEvidence({ kind: update.kind, generation: update.generation });
      reconcilePreviewDragAffordance(drag);
      flushPreviewTransformUpdate(drag);
    });
  }

  async function finishPreviewTransformInteraction(
    drag: PreviewDragState,
    action: "commit" | "cancel"
  ): Promise<void> {
    if (drag.rafId !== null) {
      window.cancelAnimationFrame(drag.rafId);
      drag.rafId = null;
    }
    setPreviewDragPreview(null);
    await drag.beginPromise;
    if (drag.interactionId === null) {
      if (previewDragRef.current === drag) {
        previewDragRef.current = null;
      }
      setPreviewInteractionEvidence(null);
      return;
    }
    if (action === "commit" && drag.moved) {
      const finalMetrics =
        drag.mode === "rotate"
          ? previewRotateMetrics(drag, drag.lastClientX, drag.lastClientY)
          : previewMoveMetrics(drag, drag.lastClientX, drag.lastClientY);
      const finalPayload = previewVisualPatchFromMetrics(drag, finalMetrics);
      drag.pendingPayload = finalPayload;
      drag.pendingMetrics = finalMetrics;
      while (drag.updateInFlight) {
        await new Promise((resolve) => window.setTimeout(resolve, 0));
      }
      flushPreviewTransformUpdate(drag);
      while (drag.updateInFlight || drag.pendingPayload !== null) {
        await new Promise((resolve) => window.setTimeout(resolve, 0));
      }
      await projectInteractions.commit(drag.interactionId);
      if (previewDragRef.current === drag) {
        previewDragRef.current = null;
      }
      setPreviewInteractionEvidence(null);
      return;
    }
    await projectInteractions.cancel(drag.interactionId);
    if (previewDragRef.current === drag) {
      previewDragRef.current = null;
    }
    setPreviewInteractionEvidence(null);
  }

  function acceptPreviewDragMetrics(drag: PreviewDragState, metrics: PreviewDragMetrics): void {
    if (metrics.mode === "move") {
      drag.acceptedMoveDeltaX = metrics.deltaClientX;
      drag.acceptedMoveDeltaY = metrics.deltaClientY;
      return;
    }
    drag.acceptedRotationDeltaDegrees = metrics.deltaDegrees;
  }

  function reconcilePreviewDragAffordance(drag: PreviewDragState): void {
    if (drag.mode === "rotate") {
      const metrics = previewRotateMetrics(drag, drag.lastClientX, drag.lastClientY);
      const deltaDegrees = normalizeRotationDegrees(metrics.deltaDegrees - drag.acceptedRotationDeltaDegrees);
      setPreviewDragPreview(Math.abs(deltaDegrees) > 0 ? { mode: "rotate", deltaDegrees } : null);
      return;
    }
    const metrics = previewMoveMetrics(drag, drag.lastClientX, drag.lastClientY);
    const deltaClientX = metrics.deltaClientX - drag.acceptedMoveDeltaX;
    const deltaClientY = metrics.deltaClientY - drag.acceptedMoveDeltaY;
    setPreviewDragPreview(
      Math.abs(deltaClientX) + Math.abs(deltaClientY) > 0
        ? { mode: "move", deltaClientX, deltaClientY }
        : null
    );
  }

  function handlePreviewCanvasClick(event: ReactMouseEvent<HTMLDivElement>): void {
    void selectPreviewTextAtClientPoint(event.clientX, event.clientY, false);
  }

  function handlePreviewCanvasDoubleClick(event: ReactMouseEvent<HTMLDivElement>): void {
    void selectPreviewTextAtClientPoint(event.clientX, event.clientY, true);
  }

  async function selectPreviewTextAtClientPoint(
    clientX: number,
    clientY: number,
    editAfterSelect: boolean
  ): Promise<void> {
    const bridge = window.videoEditorRealtimePreviewHost;
    const hostElement = nativeHostRef.current;
    if (bridge === undefined || hostElement === null || !showRealtimeSurface) {
      return;
    }
    const hostRect = hostElement.getBoundingClientRect();
    if (hostRect.width <= 0 || hostRect.height <= 0) {
      return;
    }
    const point = {
      x: Math.round(clientX - hostRect.left),
      y: Math.round(clientY - hostRect.top)
    };
    if (point.x < 0 || point.y < 0 || point.x > hostRect.width || point.y > hostRect.height) {
      return;
    }
    await bridge
      .hitTestTextOverlay(point)
      .then((hit) => {
        if (hit.hit && typeof hit.selectionHandle === "string" && hit.selectionHandle.length > 0) {
          if (editAfterSelect) {
            onEditPreviewTextOverlay(hit.selectionHandle);
          } else {
            onSelectPreviewTextOverlay(hit.selectionHandle);
          }
        }
      })
      .catch(() => undefined);
  }

  useEffect(() => {
    const stageElement = previewStageRef.current;
    if (stageElement === null) {
      return;
    }

    const calculateFitSize = () => {
      const box = stageElement.getBoundingClientRect();
      const availableWidth = Math.max(1, Math.floor(box.width));
      const availableHeight = Math.max(1, Math.floor(box.height));
      const sourceWidth = Math.max(1, canvasConfig.width);
      const sourceHeight = Math.max(1, canvasConfig.height);
      const sourceRatio = sourceWidth / sourceHeight;

      let width = Math.min(availableWidth, 840);
      let height = Math.round(width / sourceRatio);
      if (height > availableHeight) {
        height = availableHeight;
        width = Math.round(height * sourceRatio);
      }

      setCanvasFitSize((current) => {
        if (current !== null && current.width === width && current.height === height) {
          return current;
        }
        return { width, height };
      });
    };

    const observer = new ResizeObserver(calculateFitSize);
    observer.observe(stageElement);
    calculateFitSize();

    return () => {
      observer.disconnect();
    };
  }, [canvasConfig.height, canvasConfig.width]);

  useEffect(() => {
    const hostElement = nativeHostRef.current;
    const bridge = window.videoEditorRealtimePreviewHost;
    if (hostElement === null || bridge === undefined) {
      return;
    }

    let cancelled = false;
    let animationFrame: number | null = null;
    let updateInFlight = false;
    let pendingRect: PendingRealtimePreviewHostRect | null = null;

    const flushPendingRect = () => {
      if (cancelled || updateInFlight || pendingRect === null) {
        return;
      }
      const next = pendingRect;
      pendingRect = null;
      updateInFlight = true;
      void bridge
        .updateHostRect(next.rect)
        .then((state) => {
          if (!cancelled) {
            setNativeHostState(state);
            onRealtimePreviewHostStateChange(state);
          }
        })
        .catch(() => {
          if (!cancelled) {
            setNativeHostState({
              ...INITIAL_REALTIME_PREVIEW_HOST_STATE,
              fallbackActive: true,
              statusLabel: "实时预览不可用",
              fallbackLabel: "实时预览不可用：宿主通信暂不可用"
            });
          }
        })
        .finally(() => {
          updateInFlight = false;
          flushPendingRect();
        });
    };

    if (nativeSurfaceSuspended) {
      lastSentHostRectRef.current = null;
      void bridge
        .detachSurface()
        .then((state) => {
          if (!cancelled) {
            setNativeHostState(state);
            onRealtimePreviewHostStateChange(state);
          }
        })
        .catch(() => undefined);
      return () => {
        cancelled = true;
      };
    }

    const publishBounds = () => {
      animationFrame = null;
      const box = hostElement.getBoundingClientRect();
      const rect = {
        x: Math.round(box.x),
        y: Math.round(box.y),
        width: Math.round(box.width),
        height: Math.round(box.height),
        scaleFactorMillis: Math.round(window.devicePixelRatio * 1000)
      };
      if (rect.width <= 0 || rect.height <= 0 || rect.scaleFactorMillis <= 0) {
        return;
      }

      const rectKey = `${rect.x}:${rect.y}:${rect.width}:${rect.height}:${rect.scaleFactorMillis}`;
      if (lastSentHostRectRef.current === rectKey) {
        return;
      }
      lastSentHostRectRef.current = rectKey;
      pendingRect = { key: rectKey, rect };
      flushPendingRect();
    };

    const schedulePublish = () => {
      if (animationFrame !== null) {
        return;
      }
      animationFrame = window.requestAnimationFrame(publishBounds);
    };

    const observer = new ResizeObserver(schedulePublish);
    observer.observe(hostElement);
    window.addEventListener("resize", schedulePublish);
    schedulePublish();

    return () => {
      cancelled = true;
      observer.disconnect();
      window.removeEventListener("resize", schedulePublish);
      if (animationFrame !== null) {
        window.cancelAnimationFrame(animationFrame);
      }
    };
  }, [nativeSurfaceSuspended, onRealtimePreviewHostStateChange]);

  useEffect(() => {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return;
    }

    let cancelled = false;
    const unsubscribe = bridge.subscribeTelemetry((state) => {
      if (cancelled) {
        return;
      }
      setNativeHostState(state);
      onRealtimePreviewHostStateChange(state);
    });
    return () => {
      cancelled = true;
      unsubscribe();
    };
  }, [onRealtimePreviewHostStateChange]);

  return (
    <div className={showDeveloperDiagnostics ? "preview-shell developer-diagnostics" : "preview-shell"}>
      <div className="preview-titlebar">
        <strong title={`当前草稿：${draftName}`}>{MONITOR_TITLE}</strong>
        <button type="button" className="preview-title-menu" aria-label="播放器菜单" title="播放器菜单" disabled>
          <span className="app-icon-mask" style={iconMaskStyle("titlebarMenu")} aria-hidden="true" />
        </button>
      </div>

      <div ref={previewStageRef} className="preview-canvas-stage">
        <div
          className={`preview-canvas canvas-background-${backgroundTone}`}
          aria-label="预览画面"
          style={canvasStyle}
          onClick={handlePreviewCanvasClick}
          onDoubleClick={handlePreviewCanvasDoubleClick}
        >
          {!showRealtimeSurface ? (
            <div className="preview-placeholder">
              <span>{previewPlaceholderLabel}</span>
            </div>
          ) : null}
          {selectedSegment !== null && selectionOverlay !== null ? (
          <div
            className={`preview-selection-outline preview-selection-${selectionOverlay.source}`}
            aria-label="预览选中框"
            data-segment-id={selectedSegment.segmentKey}
            data-selection-handle={selectionOverlay.selectionHandle ?? selectedSegment.selectionHandle}
            data-overlay-source={selectionOverlay.source}
            data-fit-mode={selectedSegment.visual.fitMode}
            data-interaction-source={previewInteractionEvidence === null ? undefined : "rust-provisional"}
            data-interaction-kind={previewInteractionEvidence?.kind}
            data-interaction-generation={previewInteractionEvidence?.generation}
            style={selectionOverlay.style}
            onPointerDown={handlePreviewDragPointerDown}
            onPointerMove={handlePreviewDragPointerMove}
            onPointerUp={handlePreviewDragPointerUp}
            onPointerCancel={handlePreviewDragPointerCancel}
          >
            {selectionOverlay.rotateEnabled ? (
              <button
                type="button"
                className="preview-selection-rotate-handle"
                aria-label="旋转文字"
                title="旋转文字"
                onPointerDown={handlePreviewRotatePointerDown}
                onPointerMove={handlePreviewDragPointerMove}
                onPointerUp={handlePreviewDragPointerUp}
                onPointerCancel={handlePreviewDragPointerCancel}
              >
                <span aria-hidden="true">↻</span>
              </button>
            ) : null}
          </div>
          ) : null}
          {selectedSegment !== null && selectedSegment.text !== null && textOverlayStyle !== null ? (
          <div
            className="preview-text-overlay"
            aria-label="预览文字"
            data-segment-id={selectedSegment.segmentKey}
            data-text-source={selectedSegment.text.source}
            style={textOverlayStyle}
            onPointerDown={handlePreviewDragPointerDown}
            onPointerMove={handlePreviewDragPointerMove}
            onPointerUp={handlePreviewDragPointerUp}
            onPointerCancel={handlePreviewDragPointerCancel}
          >
            {selectedSegment.text.content}
          </div>
          ) : null}
          <div ref={nativeHostRef} className="preview-native-host" aria-label="实时预览画面">
          {showDeveloperDiagnostics ? (
            <div className="preview-native-host-readout">
              <span aria-label="实时预览状态">{formatRealtimePreviewHostStatus(nativeHostState)}</span>
              <span aria-label="实时预览数据">{formatRealtimePreviewTelemetry(nativeHostState, showDeveloperDiagnostics)}</span>
            </div>
          ) : null}
          {showDeveloperDiagnostics && nativeHostState.fallbackLabel !== null ? (
            <div className="preview-native-host-fallback" aria-label="实时预览不可用">
              {formatRealtimePreviewUnavailableLabel(nativeHostState, showDeveloperDiagnostics)}
            </div>
          ) : null}
          {showDeveloperDiagnostics && nativeHostState.fallbackArtifactVisible && nativeHostState.fallbackReason !== null ? (
            <div className="preview-native-host-fallback" aria-label="实时预览备用产物">
              {formatRealtimePreviewFallbackArtifact(nativeHostState, showDeveloperDiagnostics)}
            </div>
          ) : null}
          </div>
        </div>
      </div>

      <div
        className={showDeveloperDiagnostics ? "preview-transport developer-diagnostics" : "preview-transport"}
        aria-label="预览控制"
      >
        <div className="preview-timecode-cluster" aria-label="播放器时间">
          <span className="preview-timecode current" aria-label="当前时间码">
            {formatMicroseconds(safePlayheadUs)}
          </span>
          <span className="preview-time-divider" aria-hidden="true">
            /
          </span>
          <span className="preview-timecode duration" aria-label="总时长">
            {formatMicroseconds(safeTimelineDurationUs)}
          </span>
        </div>
        {showDeveloperDiagnostics ? (
        <div className="preview-frame-control-group" role="group" aria-label="逐帧预览控制">
          {MONITOR_CONTROLS.map((control) => (
            <button
              key={control.label}
              type="button"
              className="preview-icon-button"
              aria-label={previewControlLabel(control.label, playbackRunning)}
              title={previewControlLabel(control.label, playbackRunning)}
              onClick={() => {
                if (control.label === "停止") {
                  onStopPlayback();
                } else if (control.label === "上一帧") {
                  onPlayheadChange(Math.max(0, safePlayheadUs - frameStepUs));
                } else if (control.label === "下一帧") {
                  onPlayheadChange(safePlayheadUs + frameStepUs);
                }
              }}
              disabled={
                (runtimePreviewUnavailable || pending) && !(playbackRunning && control.label === "停止")
              }
            >
              <MonitorControlGlyph control={control} canvasRatio={canvasRatio} playbackRunning={playbackRunning} />
            </button>
          ))}
        </div>
        ) : null}
        <div className="preview-control-group" role="group" aria-label="预览播放控制">
          <button
            type="button"
            className="preview-play-button"
            aria-label={previewPlaybackLabel}
            title={previewPlaybackTitle}
            onClick={onTogglePlayback}
            disabled={(runtimePreviewUnavailable || pending) && !playbackRunning}
          >
            <MonitorControlGlyph control={{ label: "播放", icon: "play", symbol: "▶" }} canvasRatio={canvasRatio} playbackRunning={playbackRunning} />
          </button>
        </div>
        <div className="preview-view-control-group" role="group" aria-label="预览画面控制">
          {monitorViewControls.map((control) => (
            <button
              key={control.label}
              type="button"
              className={["preview-view-button", control.className].filter(Boolean).join(" ")}
              aria-label={control.label}
              title={
                control.label === "画布读数"
                  ? canvasReadout
                  : control.label === "画面比例"
                    ? `画面比例 ${canvasRatio}`
                    : control.value ?? control.label
              }
              disabled
            >
              {control.icon === undefined ? (
                <span aria-hidden="true">{control.value}</span>
              ) : (
                <span className="app-icon-mask" style={iconMaskStyle(control.icon)} aria-hidden="true" />
              )}
            </button>
          ))}
        </div>
        {showDeveloperDiagnostics ? (
          <>
            <label className="preview-seek-control">
              <span>预览时间</span>
              <input
                aria-label="预览时间"
                type="number"
                min="0"
                step="100000"
                value={safePlayheadUs}
                onChange={(event) => onPlayheadChange(Math.max(0, Math.round(event.currentTarget.valueAsNumber || 0)))}
              />
            </label>
          </>
        ) : null}
      </div>

      {showDeveloperDiagnostics ? (
        <>
          <RuntimeDiagnosticsPanel diagnostics={runtimeDiagnostics} pending={pending} onProbe={onProbeRuntimeCapabilities} />
        </>
      ) : null}

      {showDeveloperDiagnostics ? (
        <div className="preview-status-line" aria-live="polite">
          <span className={`status-dot ${bindingStatus.kind}`} aria-hidden="true" />
          <span aria-label="预览状态">{previewStatusLabel}</span>
          <span className={`audio-status-chip audio-status-${audioPreview.status}`} aria-label="音频预览状态">
            {audioStatusChipText(audioPreview, audioParity)}
          </span>
          <span className="audio-status-chip" aria-label="输出设备状态">
            {audioDeviceChipText(audioDevices)}
          </span>
          <span className={`audio-status-chip waveform-status-${waveform.status}`} aria-label="波形状态">
            {waveform.statusLabel}
          </span>
          <button
            type="button"
            className="audio-retry-button"
            aria-label="重试音频"
            title="重试音频"
            onClick={onRetryAudioPreview}
            disabled={pending || audioPending}
          >
            重试音频
          </button>
          <span className="canvas-readout-chip" title={canvasReadout}>
            {canvasReadout}
          </span>
          <span className={`canvas-background-chip ${backgroundTone}`} title={backgroundStatus}>
            {backgroundStatus}
          </span>
        </div>
      ) : null}
    </div>
  );
}

function formatProductPreviewStatus(preview: PreviewDisplayState, placeholderLabel: string, pending: boolean): string {
  if (preview.error !== null) {
    return preview.error;
  }

  if (pending) {
    return "正在准备预览画面";
  }

  if (preview.statusLabel.includes("预览待刷新")) {
    return "画面已更新，预览待刷新";
  }

  if (preview.statusLabel === "预览暂不可用") {
    return "预览暂不可用";
  }

  if (preview.statusLabel === "预览画面失败") {
    return "预览画面生成失败";
  }

  return preview.statusLabel === "实时预览就绪" ? "预览就绪" : placeholderLabel;
}

function previewControlLabel(label: string, playbackRunning: boolean): string {
  if (label === "播放") {
    return playbackRunning ? "暂停预览" : "播放预览";
  }
  if (label === "停止") {
    return "停止预览";
  }
  return label;
}

function MonitorControlGlyph({
  control,
  canvasRatio,
  playbackRunning
}: {
  control: MonitorControl;
  canvasRatio: string;
  playbackRunning: boolean;
}): React.ReactElement {
  if (control.label === "画面比例") {
    return <span aria-hidden="true">{canvasRatio}</span>;
  }

  const icon = control.label === "播放" && playbackRunning ? "pause" : control.icon;
  if (icon !== undefined) {
    return <span className="app-icon-mask" style={iconMaskStyle(icon)} aria-hidden="true" />;
  }
  if (control.imageIcon !== undefined) {
    return <img className="app-icon-image" src={appIconUrls[control.imageIcon]} alt="" aria-hidden="true" />;
  }

  return <span aria-hidden="true">{control.label === "画面比例" ? canvasRatio : control.symbol}</span>;
}

function iconMaskStyle(icon: AppIconName): CSSProperties {
  return { "--app-icon-url": `url("${appIconUrls[icon]}")` } as CSSProperties;
}

function audioStatusChipText(audioPreview: AudioPreviewDisplayModel, audioParity: AudioParityDisplayModel): string {
  const facts = [audioPreview.statusLabel];
  if (audioPreview.warningLabel !== null && audioPreview.warningLabel !== audioPreview.statusLabel) {
    facts.push(audioPreview.warningLabel);
  } else if (audioParity.warningLabel !== null) {
    facts.push(audioParity.warningLabel);
  } else if (
    audioPreview.deviceStatusLabel !== "输出设备就绪" &&
    audioPreview.deviceStatusLabel !== audioPreview.statusLabel
  ) {
    facts.push(audioPreview.deviceStatusLabel);
  }

  return facts.slice(0, 2).join(" · ");
}

function audioDeviceChipText(audioDevices: AudioDeviceDisplayModel): string {
  const selected =
    audioDevices.devices.find((device) => device.selectionId === audioDevices.selectedDeviceId) ??
    audioDevices.devices.find((device) => device.isDefault) ??
    audioDevices.devices[0];

  return selected === undefined ? "系统默认" : selected.displayName;
}

function frameDurationUs(canvasConfig: DraftCanvasConfig): number {
  const numerator = Math.max(1, Math.round(canvasConfig.frameRate.numerator));
  const denominator = Math.max(1, Math.round(canvasConfig.frameRate.denominator));
  return Math.max(1, Math.round((denominator * MICROSECONDS_PER_SECOND) / numerator));
}

function formatRealtimePreviewHostStatus(state: RealtimePreviewHostState): string {
  if (state.fallbackActive) {
    return state.statusLabel === "实时预览不可用" || state.statusLabel === "正在准备预览画面"
      ? state.statusLabel
      : "实时预览不可用";
  }
  if (!state.productReady) {
    return state.hostAttached ? "等待 GPU 合成" : "实时预览等待接入";
  }
  if (state.fallbackArtifactVisible || state.fallbackReason !== null) {
    return "实时预览受限";
  }
  return "实时预览已接入";
}

function formatRealtimePreviewUnavailableLabel(
  state: RealtimePreviewHostState,
  showDeveloperDiagnostics: boolean
): string {
  if (showDeveloperDiagnostics) {
    return state.fallbackLabel ?? state.statusLabel;
  }
  return formatRealtimePreviewTelemetry(state, false);
}

function formatRealtimePreviewTelemetry(state: RealtimePreviewHostState, showDeveloperDiagnostics: boolean): string {
  const { telemetry } = state;
  if (!showDeveloperDiagnostics && !state.productReady) {
    return "实时预览不可用：GPU 合成播放尚未接入";
  }
  if (telemetry === null) {
    return "等待首帧";
  }

  if (showDeveloperDiagnostics && !state.productReady) {
    const requestState = state.currentRequestCanceled ? ["当前请求已取消"] : [];
    const fallback =
      state.fallbackReason === null ? [] : [`原因 ${formatRealtimePreviewFallbackReason(state.fallbackReason)}`];
    return [
      formatRealtimePreviewDiagnosticSource(state.diagnosticSource),
      `运行时帧 ${telemetry.presentedFrameCount}`,
      `目标 ${formatMicroseconds(telemetry.targetTimeMicroseconds)}`,
      `取消 ${telemetry.canceledRequestCount}`,
      ...requestState,
      ...fallback
    ].join(" · ");
  }

  const model: RealtimePreviewDisplayModel = {
    backend: state.backend,
    firstFrameLatencyMs: telemetry.firstFrameLatencyMs,
    seekLatencyMs: telemetry.seekLatencyMs,
    queueLatencyMs: telemetry.queueLatencyMs,
    renderDurationMs: telemetry.renderDurationMs,
    presentedFrameCount: telemetry.presentedFrameCount,
    droppedFrameCount: telemetry.droppedFrameCount,
    repeatedFrameCount: telemetry.repeatedFrameCount,
    staleRejectedCount: telemetry.staleRejectedCount,
    canceledRequestCount: telemetry.canceledRequestCount,
    currentRequestCanceled: state.currentRequestCanceled,
    fallbackReason: state.fallbackReason,
    fallbackCount: telemetry.fallbackCount,
    cacheHitCount: telemetry.cacheHitCount,
    targetTimeMicroseconds: telemetry.targetTimeMicroseconds,
    playbackGeneration: telemetry.playbackGeneration,
    fallbackArtifactVisible: state.fallbackArtifactVisible
  };

  return showDeveloperDiagnostics ? summarizeRealtimePreviewDisplay(model) : summarizeRealtimePreviewProductDisplay(model);
}

function formatRealtimePreviewDiagnosticSource(source: RealtimePreviewHostState["diagnosticSource"]): string {
  const labels: Record<RealtimePreviewHostState["diagnosticSource"], string> = {
    nativeVideoBridge: "诊断来源：原生视频桥",
    runtimeFrameRequest: "诊断来源：运行时帧请求",
    none: "诊断来源：无"
  };
  return labels[source];
}

function formatRealtimePreviewFallbackArtifact(
  state: RealtimePreviewHostState,
  showDeveloperDiagnostics: boolean
): string {
  if (showDeveloperDiagnostics && state.fallbackReason !== null) {
    return `${formatRealtimePreviewBackendLabel(state.backend)} · ${formatRealtimePreviewFallbackReason(
      state.fallbackReason
    )}`;
  }
  return formatRealtimePreviewTelemetry(state, false);
}

function buildSelectionOverlayModel(
  selectedSegment: SelectedSegmentView | null,
  contentEvidence: RealtimePreviewHostContentEvidence | null,
  dragPreview: PreviewDragPreviewState | null
): SelectionOverlayModel | null {
  if (selectedSegment === null || !selectedSegment.visual.visible) {
    return null;
  }

  const selectedTextOverlay = selectedNativeTextOverlay(contentEvidence, selectedSegment);
  if (selectedTextOverlay !== null && contentEvidence !== null && contentEvidence.width > 0 && contentEvidence.height > 0) {
    const visual = selectedSegment.visual;
    const targetWidth = Math.max(1, contentEvidence.width);
    const targetHeight = Math.max(1, contentEvidence.height);
    const scaleX = Math.max(1, selectedTextOverlay.visualScaleXMillis) / 1000;
    const scaleY = Math.max(1, selectedTextOverlay.visualScaleYMillis) / 1000;
    const centerX =
      selectedTextOverlay.x +
      selectedTextOverlay.width / 2 +
      (targetWidth * selectedTextOverlay.visualPositionX) / 2000;
    const centerY =
      selectedTextOverlay.y +
      selectedTextOverlay.height / 2 -
      (targetHeight * selectedTextOverlay.visualPositionY) / 2000;
    const opacity = Math.max(0.28, Math.min(1, visual.transform.opacity.valueMillis / 1000));
    return {
      source: "native-text",
      selectionHandle: selectedTextOverlay.selectionHandle,
      rotateEnabled: true,
      style: {
        left: `${(centerX / targetWidth) * 100}%`,
        top: `${(centerY / targetHeight) * 100}%`,
        width: `${((Math.max(1, selectedTextOverlay.width) * scaleX) / targetWidth) * 100}%`,
        height: `${((Math.max(1, selectedTextOverlay.height) * scaleY) / targetHeight) * 100}%`,
        opacity,
        transform: buildCenteredPreviewTransform(selectedTextOverlay.visualRotationDegrees, dragPreview)
      }
    };
  }

  const visual = selectedSegment.visual;
  const crop = visual.transform.crop;
  const remainingWidthMillis = Math.max(1, 1000 - crop.leftMillis - crop.rightMillis);
  const remainingHeightMillis = Math.max(1, 1000 - crop.topMillis - crop.bottomMillis);
  const widthPercent = clampOverlayPercent((72 * visual.transform.scale.xMillis * remainingWidthMillis) / 1_000_000);
  const heightPercent = clampOverlayPercent((72 * visual.transform.scale.yMillis * remainingHeightMillis) / 1_000_000);
  const xPercent = clampOverlayOffsetPercent(visual.transform.position.x / 20);
  const yPercent = clampOverlayOffsetPercent(visual.transform.position.y / 20);
  const opacity = Math.max(0.28, Math.min(1, visual.transform.opacity.valueMillis / 1000));

  return {
    source: "segment-visual",
    selectionHandle: selectedSegment.selectionHandle,
    rotateEnabled: selectedSegment.text !== null,
    style: {
      left: `calc(50% + ${xPercent}%)`,
      top: `calc(50% - ${yPercent}%)`,
      width: `${widthPercent}%`,
      height: `${heightPercent}%`,
      opacity,
      transform: buildCenteredPreviewTransform(visual.transform.rotation.degrees, dragPreview)
    }
  };
}

function buildCenteredPreviewTransform(
  baseRotationDegrees: number,
  dragPreview: PreviewDragPreviewState | null
): string {
  const moveX = dragPreview?.mode === "move" ? dragPreview.deltaClientX : 0;
  const moveY = dragPreview?.mode === "move" ? dragPreview.deltaClientY : 0;
  const rotationDelta = dragPreview?.mode === "rotate" ? dragPreview.deltaDegrees : 0;
  return `translate(calc(-50% + ${Math.round(moveX)}px), calc(-50% + ${Math.round(moveY)}px)) rotate(${
    baseRotationDegrees + rotationDelta
  }deg)`;
}

function buildDirectPreviewTransform(
  baseRotationDegrees: number,
  dragPreview: PreviewDragPreviewState | null
): string {
  const moveX = dragPreview?.mode === "move" ? dragPreview.deltaClientX : 0;
  const moveY = dragPreview?.mode === "move" ? dragPreview.deltaClientY : 0;
  const rotationDelta = dragPreview?.mode === "rotate" ? dragPreview.deltaDegrees : 0;
  return `translate(${Math.round(moveX)}px, ${Math.round(moveY)}px) rotate(${baseRotationDegrees + rotationDelta}deg)`;
}

function selectedNativeTextOverlay(
  contentEvidence: RealtimePreviewHostContentEvidence | null,
  selectedSegment: SelectedSegmentView
): RealtimePreviewTextOverlayEvidence | null {
  if (selectedSegment.text === null || contentEvidence?.activeTextOverlays === undefined) {
    return null;
  }
  return (
    contentEvidence.activeTextOverlays.find((overlay) => overlay.selectionHandle === selectedSegment.selectionHandle) ??
    null
  );
}

function clampOverlayPercent(value: number): number {
  return Math.max(8, Math.min(98, value));
}

function clampOverlayOffsetPercent(value: number): number {
  return Math.max(-48, Math.min(48, value));
}

function pointerAngleDegrees(clientX: number, clientY: number, centerClientX: number, centerClientY: number): number {
  return (Math.atan2(clientY - centerClientY, clientX - centerClientX) * 180) / Math.PI;
}

function normalizeRotationDegrees(value: number): number {
  let normalized = Math.round(value);
  while (normalized > 180) {
    normalized -= 360;
  }
  while (normalized < -180) {
    normalized += 360;
  }
  return normalized;
}

function previewMoveMetrics(drag: PreviewDragState, clientX: number, clientY: number): PreviewDragMetrics {
  return {
    mode: "move",
    deltaClientX: clientX - drag.startClientX,
    deltaClientY: clientY - drag.startClientY
  };
}

function previewRotateMetrics(drag: PreviewDragState, clientX: number, clientY: number): PreviewDragMetrics {
  const centerClientX = drag.centerClientX ?? drag.startClientX;
  const centerClientY = drag.centerClientY ?? drag.startClientY;
  const startAngle =
    drag.startAngleDegrees ?? pointerAngleDegrees(drag.startClientX, drag.startClientY, centerClientX, centerClientY);
  const currentAngle = pointerAngleDegrees(clientX, clientY, centerClientX, centerClientY);
  return {
    mode: "rotate",
    deltaDegrees: normalizeRotationDegrees(currentAngle - startAngle)
  };
}

function previewVisualPatchFromMetrics(drag: PreviewDragState, metrics: PreviewDragMetrics): SegmentVisualPatch {
  if (metrics.mode === "rotate") {
    return {
      rotationDeltaDegrees: Math.round(metrics.deltaDegrees)
    };
  }
  const deltaX = Math.round((metrics.deltaClientX * 2000) / Math.max(1, drag.canvasWidth));
  const deltaY = Math.round((metrics.deltaClientY * 2000) / Math.max(1, drag.canvasHeight));
  return {
    positionDeltaX: deltaX,
    positionDeltaY: -deltaY
  };
}

function buildTextOverlayStyle(
  selectedSegment: SelectedSegmentView | null,
  dragPreview: PreviewDragPreviewState | null
): CSSProperties | null {
  if (
    selectedSegment === null ||
    !selectedSegment.visual.visible ||
    selectedSegment.text === null ||
    selectedSegment.text === undefined ||
    selectedSegment.text.content.trim().length === 0
  ) {
    return null;
  }

  const text = selectedSegment.text;
  const visual = selectedSegment.visual;
  const region = text.layoutRegion;
  const style = text.style;
  const textBoxWidth = clampMillis(text.textBox.widthMillis);
  const textBoxHeight = clampMillis(text.textBox.heightMillis);
  const widthMillis = Math.min(clampMillis(region.widthMillis), textBoxWidth);
  const heightMillis = Math.min(clampMillis(region.heightMillis), textBoxHeight);
  const leftMillis = clampMillis(region.xMillis);
  const topMillis = clampMillis(region.yMillis);
  const xPercent = clampOverlayOffsetPercent(visual.transform.position.x / 20);
  const yPercent = clampOverlayOffsetPercent(visual.transform.position.y / 20);
  const shadow = style.shadow;
  const stroke = style.stroke;

  return {
    left: `calc(${leftMillis / 10}% + ${xPercent}%)`,
    top: `calc(${topMillis / 10}% - ${yPercent}%)`,
    width: `${widthMillis / 10}%`,
    minHeight: `${Math.max(20, heightMillis / 10)}%`,
    opacity: Math.max(0.28, Math.min(1, visual.transform.opacity.valueMillis / 1000)),
    transform: buildDirectPreviewTransform(visual.transform.rotation.degrees, dragPreview),
    color: style.color,
    backgroundColor: style.background?.color ?? "transparent",
    fontFamily: quoteFontFamily(style.font.family),
    fontSize: `${style.fontSize}px`,
    lineHeight: style.lineHeightMillis / 1000,
    letterSpacing: `${style.letterSpacingMillis / 1000}px`,
    textAlign: style.alignment,
    whiteSpace: text.wrapping === "auto" ? "pre-wrap" : "pre",
    overflow: "hidden",
    textShadow: shadow === null || shadow === undefined ? "none" : `${shadow.offsetX}px ${shadow.offsetY}px ${shadow.blur}px ${shadow.color}`,
    WebkitTextStroke:
      stroke === null || stroke === undefined ? undefined : `${Math.max(0, stroke.width)}px ${stroke.color}`
  };
}

function clampMillis(value: number): number {
  return Math.max(0, Math.min(1000, Math.round(value)));
}

function quoteFontFamily(family: string): string {
  const trimmed = family.trim();
  if (trimmed.length === 0) {
    return "\"PingFang SC\", sans-serif";
  }

  return `"${trimmed.replace(/"/g, "")}", "PingFang SC", sans-serif`;
}

function RuntimeDiagnosticsPanel({
  diagnostics,
  pending,
  onProbe
}: {
  diagnostics: RuntimeDiagnosticsDisplayState;
  pending: boolean;
  onProbe: () => void;
}): React.ReactElement {
  const actionLabel = diagnostics.status === "idle" ? "检查运行环境" : "重新检测运行环境";
  const busy = diagnostics.status === "checking";
  const detail = diagnostics.diagnostics[0] ?? diagnostics.statusDetail;

  return (
    <section className={`runtime-diagnostics-panel ${diagnostics.status}`} aria-label="运行环境诊断">
      <div className="runtime-diagnostics-header">
        <div className="runtime-status-summary" aria-label="运行环境状态">
          <span className={`runtime-status-dot ${diagnostics.status}`} aria-hidden="true" />
          <strong>{diagnostics.statusLabel}</strong>
          <span title={detail}>{detail}</span>
        </div>
        <button
          type="button"
          aria-label={actionLabel}
          title={actionLabel}
          onClick={onProbe}
          disabled={pending || busy}
        >
          {actionLabel === "检查运行环境" ? "检测" : "重检"}
        </button>
      </div>
      <div className="runtime-capability-grid" aria-label="运行能力列表">
        {diagnostics.rows.length === 0 ? (
          <div className="runtime-capability-row muted" aria-label="打包状态">
            <strong>打包状态</strong>
            <span>{diagnostics.packageStatusLabel}</span>
            <em>{diagnostics.checkedAtLabel}</em>
          </div>
        ) : (
          diagnostics.rows.map((row) => <RuntimeDiagnosticsRowView key={row.label} row={row} />)
        )}
      </div>
    </section>
  );
}

function RuntimeDiagnosticsRowView({ row }: { row: RuntimeDiagnosticsRow }): React.ReactElement {
  return (
    <div className={`runtime-capability-row ${runtimeToneClass(row.tone)}`} aria-label={row.label} title={row.detail}>
      <strong>{row.label}</strong>
      <span>{row.value}</span>
      <em>{row.detail || "无额外诊断"}</em>
    </div>
  );
}

function runtimeToneClass(tone: RuntimeDiagnosticsTone): string {
  return tone;
}

import { useEffect, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent } from "react";

import type { DraftCanvasConfig, SegmentVisual } from "../../generated/Draft";
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
  playheadUs?: number;
  playbackRunning: boolean;
  onRealtimePreviewHostStateChange: (state: RealtimePreviewHostState) => void;
  onPlayheadChange: (value: number) => void;
  onTogglePlayback: () => void;
  onStopPlayback: () => void;
  onRequestPreviewFrame: () => void;
  onRequestPreviewSegment: () => void;
  onProbeRuntimeCapabilities: () => void;
  onRetryAudioPreview: () => void;
  onUpdateSelectedSegmentVisual: (visual: SegmentVisual) => void;
};

type MonitorControl = {
  label: string;
  icon?: AppIconName;
  symbol: string;
};

type RealtimePreviewHostRect = {
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactorMillis: number;
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
};

export type RealtimePreviewHostApi = {
  updateHostRect: (rect: RealtimePreviewHostRect) => Promise<RealtimePreviewHostState>;
  subscribeTelemetry: (listener: (state: RealtimePreviewHostState) => void) => () => void;
  updateProjectSessionSnapshot: (projectSessionId: string, expectedRevision: number) => Promise<RealtimePreviewHostState>;
  seek: (targetTimeMicroseconds: number) => Promise<RealtimePreviewHostState>;
  play: () => Promise<RealtimePreviewHostState>;
  pause: () => Promise<RealtimePreviewHostState>;
  stop: () => Promise<RealtimePreviewHostState>;
};

type PreviewDragState = {
  pointerId: number;
  startClientX: number;
  startClientY: number;
  lastClientX: number;
  lastClientY: number;
  startVisual: SegmentVisual;
  canvasWidth: number;
  canvasHeight: number;
  moved: boolean;
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
  { label: "播放", icon: "play", symbol: "▶" },
  { label: "停止", symbol: "■" },
  { label: "上一帧", symbol: "‹" },
  { label: "下一帧", symbol: "›" },
  { label: "适应窗口", symbol: "□" },
  { label: "画面比例", symbol: "16:9" },
  { label: "全屏", symbol: "⛶" }
];

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
  playheadUs = 0,
  playbackRunning,
  onRealtimePreviewHostStateChange,
  onPlayheadChange,
  onTogglePlayback,
  onStopPlayback,
  onRequestPreviewFrame,
  onRequestPreviewSegment,
  onProbeRuntimeCapabilities,
  onRetryAudioPreview,
  onUpdateSelectedSegmentVisual
}: PreviewMonitorProps): React.ReactElement {
  const nativeHostRef = useRef<HTMLDivElement>(null);
  const lastSentHostRectRef = useRef<string | null>(null);
  const previewDragRef = useRef<PreviewDragState | null>(null);
  const [nativeHostState, setNativeHostState] = useState<RealtimePreviewHostState>(INITIAL_REALTIME_PREVIEW_HOST_STATE);
  const safePlayheadUs = Math.max(0, Math.round(playheadUs));
  const frameStepUs = frameDurationUs(canvasConfig);
  const canvasReadout = formatCanvasReadout(canvasConfig);
  const canvasRatio = formatCanvasAspectRatio(canvasConfig);
  const backgroundStatus = formatCanvasBackgroundStatus(canvasConfig);
  const backgroundTone = canvasBackgroundTone(canvasConfig);
  const canvasStyle = {
    aspectRatio: `${Math.max(1, canvasConfig.width)} / ${Math.max(1, canvasConfig.height)}`,
    background: canvasConfig.background.kind === "solidColor" ? canvasConfig.background.color : "#070707"
  };
  const previewFrameLabel = runtimeDiagnostics.canPreview ? "请求预览帧" : "预览暂不可用";
  const previewSegmentLabel = runtimeDiagnostics.canPreview ? "生成预览片段" : "预览暂不可用";
  const previewPlaceholderLabel =
    selectedSegment === null ? "添加素材到时间线后显示预览" : pending ? "正在准备预览画面" : "实时预览准备中";
  const showRealtimeSurface = nativeHostState.productReady && !nativeHostState.fallbackActive;
  const showPreviewFrameImage = preview.frameDisplayUrl !== null && !showRealtimeSurface;
  const productPreviewStatusLabel = formatProductPreviewStatus(preview, previewPlaceholderLabel, pending);
  const previewStatusLabel = showDeveloperDiagnostics
    ? preview.error ?? preview.frameStatusLabel
    : productPreviewStatusLabel === "画面已更新，预览待刷新"
      ? productPreviewStatusLabel
      : resourcePreviewStatusLabel ?? productPreviewStatusLabel;
  const selectionOverlayStyle = buildSelectionOverlayStyle(selectedSegment);
  const textOverlayStyle =
    preview.frameDisplayUrl === null && !showRealtimeSurface ? buildTextOverlayStyle(selectedSegment) : null;

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
    previewDragRef.current = {
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      lastClientX: event.clientX,
      lastClientY: event.clientY,
      startVisual: selectedSegment.visual,
      canvasWidth: canvasRect.width,
      canvasHeight: canvasRect.height,
      moved: false
    };
  }

  function handlePreviewDragPointerMove(event: ReactPointerEvent<HTMLDivElement>): void {
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
  }

  function handlePreviewDragPointerUp(event: ReactPointerEvent<HTMLDivElement>): void {
    const drag = previewDragRef.current;
    if (drag === null || drag.pointerId !== event.pointerId) {
      return;
    }
    previewDragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    if (!drag.moved) {
      return;
    }

    const deltaX = Math.round(((drag.lastClientX - drag.startClientX) * 2000) / drag.canvasWidth);
    const deltaY = Math.round(((drag.lastClientY - drag.startClientY) * 2000) / drag.canvasHeight);
    onUpdateSelectedSegmentVisual({
      ...drag.startVisual,
      transform: {
        ...drag.startVisual.transform,
        position: {
          x: clampCanvasPosition(drag.startVisual.transform.position.x + deltaX),
          y: clampCanvasPosition(drag.startVisual.transform.position.y - deltaY)
        }
      }
    });
  }

  function handlePreviewDragPointerCancel(event: ReactPointerEvent<HTMLDivElement>): void {
    const drag = previewDragRef.current;
    if (drag !== null && drag.pointerId === event.pointerId) {
      previewDragRef.current = null;
    }
  }

  useEffect(() => {
    const hostElement = nativeHostRef.current;
    const bridge = window.videoEditorRealtimePreviewHost;
    if (hostElement === null || bridge === undefined) {
      return;
    }

    let cancelled = false;
    let animationFrame: number | null = null;

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

      void bridge
        .updateHostRect(rect)
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
        });
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
  }, [onRealtimePreviewHostStateChange]);

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
        <strong>{draftName}</strong>
        <span title={canvasReadout}>{canvasReadout}</span>
      </div>

      <div
        className={`preview-canvas canvas-background-${backgroundTone}`}
        aria-label="预览画面"
        style={canvasStyle}
      >
        {!showPreviewFrameImage && !showRealtimeSurface ? (
          <div className="preview-placeholder">
            <span>{preview.frameArtifactPath === null ? previewPlaceholderLabel : "预览帧已返回，正在准备显示"}</span>
          </div>
        ) : null}
        {showPreviewFrameImage ? (
          <img className="preview-frame-image" src={preview.frameDisplayUrl} alt="当前预览帧" aria-label="当前预览帧" />
        ) : null}
        {selectedSegment !== null && selectionOverlayStyle !== null ? (
          <div
            className="preview-selection-outline"
            aria-label="预览选中框"
            data-segment-id={selectedSegment.segmentKey}
            data-fit-mode={selectedSegment.visual.fitMode}
            style={selectionOverlayStyle}
            onPointerDown={handlePreviewDragPointerDown}
            onPointerMove={handlePreviewDragPointerMove}
            onPointerUp={handlePreviewDragPointerUp}
            onPointerCancel={handlePreviewDragPointerCancel}
          />
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

      <div
        className={showDeveloperDiagnostics ? "preview-transport developer-diagnostics" : "preview-transport"}
        aria-label="预览控制"
      >
        <div className="preview-timecode" aria-label="当前时间码">
          {formatMicroseconds(safePlayheadUs)}
        </div>
        <div className="preview-control-group" role="group" aria-label="预览播放控制">
          {MONITOR_CONTROLS.map((control) => (
            <button
              key={control.label}
              type="button"
              className={control.label === "画面比例" ? "preview-icon-button ratio-button" : "preview-icon-button"}
              aria-label={previewControlLabel(control.label, playbackRunning)}
              title={previewControlLabel(control.label, playbackRunning)}
              onClick={() => {
                if (control.label === "播放") {
                  onTogglePlayback();
                } else if (control.label === "停止") {
                  onStopPlayback();
                } else if (control.label === "上一帧") {
                  onPlayheadChange(Math.max(0, safePlayheadUs - frameStepUs));
                } else if (control.label === "下一帧") {
                  onPlayheadChange(safePlayheadUs + frameStepUs);
                }
              }}
              disabled={
                (pending && !(playbackRunning && (control.label === "播放" || control.label === "停止"))) ||
                control.label === "适应窗口" ||
                control.label === "画面比例" ||
                control.label === "全屏"
              }
            >
              <MonitorControlGlyph control={control} canvasRatio={canvasRatio} playbackRunning={playbackRunning} />
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
            <div className="preview-command-group" role="group" aria-label="预览生成">
              <button
                type="button"
                className="preview-command-button"
                aria-label={previewFrameLabel}
                title={previewFrameLabel}
                onClick={onRequestPreviewFrame}
                disabled={pending || !runtimeDiagnostics.canPreview}
              >
                帧
              </button>
              <button
                type="button"
                className="preview-command-button"
                aria-label={previewSegmentLabel}
                title={previewSegmentLabel}
                onClick={onRequestPreviewSegment}
                disabled={pending || !runtimeDiagnostics.canPreview}
              >
                片段
              </button>
            </div>
          </>
        ) : null}
      </div>

      {showDeveloperDiagnostics ? (
        <>
          <div className="preview-artifact-panel" aria-label="预览产物">
            <PreviewArtifactLine title="预览帧" status={preview.frameStatusLabel} metadata={preview.frameMetadataLabel} path={preview.frameArtifactPath} />
            <PreviewArtifactLine
              title="预览片段"
              status={preview.segmentStatusLabel}
              metadata={preview.segmentMetadataLabel}
              path={preview.segmentArtifactPath}
            />
          </div>

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

  if (preview.frameStatusLabel.includes("已更新，请重新请求预览帧")) {
    return "画面已更新，预览待刷新";
  }

  if (preview.frameStatusLabel === "预览暂不可用") {
    return "预览暂不可用";
  }

  if (preview.frameStatusLabel === "预览帧失败") {
    return "预览画面生成失败";
  }

  if (preview.frameDisplayUrl !== null) {
    return "预览就绪";
  }

  return preview.frameArtifactPath === null ? placeholderLabel : "正在准备预览画面";
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

  return <span aria-hidden="true">{control.symbol}</span>;
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

function buildSelectionOverlayStyle(selectedSegment: SelectedSegmentView | null): CSSProperties | null {
  if (selectedSegment === null || !selectedSegment.visual.visible) {
    return null;
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
    left: `calc(50% + ${xPercent}%)`,
    top: `calc(50% - ${yPercent}%)`,
    width: `${widthPercent}%`,
    height: `${heightPercent}%`,
    opacity,
    transform: `translate(-50%, -50%) rotate(${visual.transform.rotation.degrees}deg)`
  };
}

function clampOverlayPercent(value: number): number {
  return Math.max(8, Math.min(98, value));
}

function clampOverlayOffsetPercent(value: number): number {
  return Math.max(-48, Math.min(48, value));
}

function clampCanvasPosition(value: number): number {
  return Math.max(-960, Math.min(960, Math.round(value)));
}

function buildTextOverlayStyle(selectedSegment: SelectedSegmentView | null): CSSProperties | null {
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
    transform: `rotate(${visual.transform.rotation.degrees}deg)`,
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

function PreviewArtifactLine({
  title,
  status,
  metadata,
  path
}: {
  title: string;
  status: string;
  metadata: string;
  path: string | null;
}): React.ReactElement {
  return (
    <div className="preview-artifact-line">
      <strong>{title}</strong>
      <span>{status}</span>
      <span>{metadata}</span>
      {path === null ? null : <code>{path}</code>}
    </div>
  );
}

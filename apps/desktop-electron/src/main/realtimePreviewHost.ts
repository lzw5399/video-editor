import { app, BrowserWindow, ipcMain, screen, type IpcMainInvokeEvent, type WebContents } from "electron";

import {
  attachRealtimePreviewSurface,
  cancelRealtimePreviewRequest,
  closeRealtimePreviewSession,
  createRealtimePreviewSession,
  detachRealtimePreviewSurface,
  getRealtimePreviewTelemetry,
  getRealtimePreviewPresentationState,
  hitTestRealtimePreviewTextOverlay,
  nextRealtimePreviewCancellationToken,
  pauseRealtimePreview,
  playRealtimePreview,
  requestRealtimePreviewFrame,
  seekRealtimePreview,
  stopRealtimePreview,
  subscribeRealtimePreviewEvents,
  updateRealtimePreviewProjectSessionSnapshot,
  updateRealtimePreviewSurfaceBounds,
  type RealtimePreviewBindingEvent,
  type RealtimePreviewFallbackReason,
  type RealtimePreviewFrameResponse,
  type RealtimePreviewPresentationStateResponse,
  type RealtimePreviewScreenRect,
  type RealtimePreviewSurfaceBounds,
  type RealtimePreviewSurfaceDescriptor,
  type RealtimePreviewTextHitTestResponse,
  type RealtimePreviewTelemetryResponse
} from "./nativeBinding";

export type RealtimePreviewHostRectInput = {
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactorMillis: number;
};

export type RealtimePreviewHostDisplayState = {
  ok: boolean;
  productReady: boolean;
  hostAttached: boolean;
  fallbackActive: boolean;
  statusLabel: string;
  fallbackLabel: string | null;
  unsupportedReason: string | null;
  playbackGeneration: number | null;
  backend: RealtimePreviewHostProductBackend;
  diagnosticSource: RealtimePreviewHostDiagnosticSource;
  fallbackReason: RealtimePreviewFallbackReason | null;
  currentRequestCanceled: boolean;
  fallbackArtifactVisible: boolean;
  telemetry: RealtimePreviewTelemetryResponse | null;
  frameDisplay: RealtimePreviewHostFrameDisplay | null;
  contentEvidence: RealtimePreviewHostContentEvidence | null;
  surfacePlacement: RealtimePreviewHostSurfacePlacement | null;
};

export type RealtimePreviewHostTaskRuntimeTelemetry = {
  submittedCount: number;
  admittedCount: number;
  startedCount: number;
  completedCount: number;
  rejectedCount: number;
  canceledCount: number;
  staleRejectedCount: number;
  fallbackCount: number;
  cacheHitCount: number;
  firstFrameTimeUs: number | null;
  droppedFrameCount: number;
  repeatedFrameCount: number;
  resourceSaturationCount: number;
  queueLatencyUs: {
    sampleCount: number;
    p50?: number | null;
    p95?: number | null;
    max?: number | null;
  };
};

export type RealtimePreviewHostProductBackend = "renderGraphGpu" | "none";

export type RealtimePreviewHostDiagnosticSource = "nativeVideoBridge" | "runtimeFrameRequest" | "none";

export type RealtimePreviewHostFrameDisplay = {
  surfaceKind: "mock";
  frameToken: string;
  targetTimeMicroseconds: number;
  dominantColor: string;
  accentColor: string;
};

export type RealtimePreviewHostContentEvidence = {
  source: "nativeVideoBridge" | "renderGraphGpuComposited";
  digest: string;
  width: number;
  height: number;
  byteCount: number;
  targetTimeMicroseconds: number;
  presentedFrames: number;
  submittedDraws: number;
  activeTextOverlays?: RealtimePreviewHostTextOverlayEvidence[];
};

export type RealtimePreviewHostTextOverlayEvidence = {
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

export type RealtimePreviewHostTextHitTestPoint = {
  x: number;
  y: number;
};

export type RealtimePreviewHostTextHitTestResponse = RealtimePreviewTextHitTestResponse;

export type RealtimePreviewHostSurfacePlacement = {
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

type RealtimePreviewHostRecord = {
  kind: string;
  parentHandleByteLength?: number;
  surfaceKind?: string;
  bounds?: RealtimePreviewSurfaceBounds;
  reflowReason?: string;
  windowVisible?: boolean;
  windowFocused?: boolean;
  appFocused?: boolean;
  targetTimeMicroseconds?: number;
  playbackGeneration?: number;
  interactionId?: string | null;
  nativeEventKind?: string;
  droppedFrameCount?: number;
  durationMs?: number;
  presentedFrameCount?: number;
  errorMessage?: string;
  presentationAvailable?: boolean;
  presentationBackend?: string;
  unsupportedReason?: string | null;
};

declare global {
  var __videoEditorTestRealtimePreviewHostCalls: RealtimePreviewHostRecord[] | undefined;
}

type SenderAssertion = (event: IpcMainInvokeEvent) => void;

const hostsByWindowId = new Map<number, RealtimePreviewHost>();
const hostsBySessionId = new Map<string, RealtimePreviewHost>();
let realtimePreviewHostIpcInstalled = false;
let nativePreviewEventBridgeInstalled = false;
const TELEMETRY_STATE_CHANNEL = "realtimePreviewHost:telemetryState";
const PRESENTATION_EVENT_REFRESH_INTERVAL_MS = 250;
const STILL_FRAME_PRESENTATION_WAIT_TIMEOUT_MS = 3_000;
const PRESENTATION_TARGET_TOLERANCE_US = 5_000;
const WINDOW_SURFACE_REFLOW_DELAY_MS = 80;
const WINDOW_SURFACE_REFLOW_EVENTS = [
  "will-move",
  "move",
  "moved",
  "will-resize",
  "resize",
  "resized",
  "maximize",
  "unmaximize",
  "restore",
  "enter-full-screen",
  "leave-full-screen",
  "show"
] as const;

type PendingPresentationWaiter = {
  playbackGeneration: number;
  targetTimeMicroseconds: number;
  timeout: ReturnType<typeof setTimeout>;
  resolve: (presented: boolean) => void;
};

export function registerRealtimePreviewHost(window: BrowserWindow, assertAllowedSender: SenderAssertion): RealtimePreviewHost {
  installRealtimePreviewHostIpc(assertAllowedSender);
  const host = new RealtimePreviewHost(window);
  hostsByWindowId.set(window.id, host);
  window.on("closed", () => {
    hostsByWindowId.delete(window.id);
  });
  return host;
}

function installRealtimePreviewHostIpc(assertAllowedSender: SenderAssertion): void {
  if (realtimePreviewHostIpcInstalled) {
    return;
  }

  ipcMain.handle("realtimePreviewHost:updateRect", (event, rect: RealtimePreviewHostRectInput) => {
    assertAllowedSender(event);
    return hostForEvent(event).updateHostRect(rect);
  });
  ipcMain.handle("realtimePreviewHost:detachSurface", (event) => {
    assertAllowedSender(event);
    return hostForEvent(event).detachSurface();
  });
  ipcMain.handle("realtimePreviewHost:subscribeTelemetry", (event) => {
    assertAllowedSender(event);
    return hostForEvent(event).subscribeTelemetry(event.sender);
  });
  ipcMain.handle("realtimePreviewHost:unsubscribeTelemetry", (event) => {
    assertAllowedSender(event);
    hostForEvent(event).unsubscribeTelemetry(event.sender);
    return { ok: true };
  });
  ipcMain.handle(
    "realtimePreviewHost:updateProjectSessionSnapshot",
    (event, projectSessionId: string, expectedRevision: number, interactionId?: string | null) => {
      assertAllowedSender(event);
      return hostForEvent(event).updateProjectSessionSnapshot(projectSessionId, expectedRevision, interactionId ?? null);
    }
  );
  ipcMain.handle("realtimePreviewHost:seek", (event, targetTimeMicroseconds: number) => {
    assertAllowedSender(event);
    return hostForEvent(event).seek(targetTimeMicroseconds);
  });
  ipcMain.handle("realtimePreviewHost:play", (event) => {
    assertAllowedSender(event);
    return hostForEvent(event).play();
  });
  ipcMain.handle("realtimePreviewHost:pause", (event) => {
    assertAllowedSender(event);
    return hostForEvent(event).pause();
  });
  ipcMain.handle("realtimePreviewHost:stop", (event) => {
    assertAllowedSender(event);
    return hostForEvent(event).stop();
  });
  ipcMain.handle("realtimePreviewHost:hitTestTextOverlay", (event, point: RealtimePreviewHostTextHitTestPoint) => {
    assertAllowedSender(event);
    return hostForEvent(event).hitTestTextOverlay(point);
  });
  realtimePreviewHostIpcInstalled = true;
}

function hostForEvent(event: IpcMainInvokeEvent): RealtimePreviewHost {
  const window = BrowserWindow.fromWebContents(event.sender);
  const host = window === null ? undefined : hostsByWindowId.get(window.id);
  if (host === undefined) {
    throw new Error("实时预览宿主尚未就绪");
  }
  return host;
}

function ensureNativePreviewEventBridge(): void {
  if (nativePreviewEventBridgeInstalled) {
    return;
  }
  subscribeRealtimePreviewEvents((errorOrEventJson: unknown, maybeEventJson?: string) => {
    const eventJson = readRealtimePreviewEventJson(errorOrEventJson, maybeEventJson);
    if (eventJson === null) {
      const errorMessage = errorOrEventJson instanceof Error ? errorOrEventJson.message : String(errorOrEventJson);
      recordRealtimePreviewHostCall({
        kind: "nativePreviewEventInvalid",
        errorMessage: errorMessage.slice(0, 200)
      });
      return;
    }
    const event = parseRealtimePreviewBindingEvent(eventJson);
    if (event === null) {
      recordRealtimePreviewHostCall({
        kind: "nativePreviewEventInvalid",
        errorMessage: eventJson.slice(0, 200)
      });
      return;
    }
    const host = hostsBySessionId.get(event.sessionId);
    if (host === undefined) {
      recordRealtimePreviewHostCall({
        kind: "nativePreviewEventDropped",
        nativeEventKind: event.kind,
        playbackGeneration: event.playbackGeneration,
        targetTimeMicroseconds: event.targetTimeMicroseconds ?? undefined
      });
      return;
    }
    host.handleNativePreviewEvent(event);
  });
  nativePreviewEventBridgeInstalled = true;
  recordRealtimePreviewHostCall({ kind: "nativePreviewEventBridgeInstalled" });
}

function readRealtimePreviewEventJson(errorOrEventJson: unknown, maybeEventJson?: string): string | null {
  if (typeof maybeEventJson === "string") {
    return maybeEventJson;
  }
  if (typeof errorOrEventJson === "string") {
    return errorOrEventJson;
  }
  return null;
}

function parseRealtimePreviewBindingEvent(eventJson: string): RealtimePreviewBindingEvent | null {
  try {
    const value = JSON.parse(eventJson) as Partial<RealtimePreviewBindingEvent>;
    if (typeof value.sessionId !== "string" || typeof value.kind !== "string" || typeof value.playbackGeneration !== "number") {
      return null;
    }
    return value as RealtimePreviewBindingEvent;
  } catch {
    return null;
  }
}

export class RealtimePreviewHost {
  private sessionId: string | null = null;
  private playbackGeneration: number | null = null;
  private attached = false;
  private fallbackLabel: string | null = null;
  private telemetry: RealtimePreviewTelemetryResponse | null = null;
  private presentationState: RealtimePreviewPresentationStateResponse | null = null;
  private lastFrame: RealtimePreviewFrameResponse | null = null;
  private lastContentEvidence: RealtimePreviewHostContentEvidence | null = null;
  private lastPresentedEvent: { playbackGeneration: number; targetTimeMicroseconds: number } | null = null;
  private lastBounds: RealtimePreviewSurfaceBounds | null = null;
  private lastPresentationSnapshotRefreshAt = 0;
  private closed = false;
  private presentedNativeEventCount = 0;
  private droppedNativeFrameCount = 0;
  private telemetrySubscribers = new Map<number, WebContents>();
  private pendingPresentationWaiters = new Set<PendingPresentationWaiter>();
  private pendingGeometryReflow: ReturnType<typeof setTimeout> | null = null;
  private pendingGeometryReflowReason: string | null = null;
  private readonly windowGeometryListeners: Array<{ eventName: string; listener: () => void }> = [];

  constructor(private readonly window: BrowserWindow) {
    for (const eventName of WINDOW_SURFACE_REFLOW_EVENTS) {
      const listener = () => this.scheduleSurfaceReflow(`browserWindow:${eventName}`);
      window.on(eventName, listener);
      this.windowGeometryListeners.push({ eventName, listener });
    }
    window.on("close", () => {
      this.close();
    });
  }

  updateHostRect(rect: RealtimePreviewHostRectInput): RealtimePreviewHostDisplayState {
    if (this.closed) {
      return this.state("实时预览已关闭");
    }

    const bounds = normalizeHostRect(rect);
    if (bounds === null) {
      this.fallbackLabel = "实时预览区域暂不可用";
      return this.state("实时预览等待画面区域");
    }

    try {
      this.ensureSession();
      if (this.sessionId === null) {
        throw new Error("实时预览会话尚未创建");
      }

      if (!this.attached) {
        this.ensureNativeWindowVisible();
        const surface = this.buildSurfaceDescriptor(bounds);
        recordRealtimePreviewHostCall({
          kind: "attachSurface",
          surfaceKind: surface.kind,
          bounds
        });
        const response = attachRealtimePreviewSurface({
          sessionId: this.sessionId,
          surface
        });
        this.playbackGeneration = response.playbackGeneration;
        this.attached = true;
      }

      if (!sameBounds(this.lastBounds, bounds)) {
        recordRealtimePreviewHostCall({ kind: "updateSurfaceBounds", bounds });
        const response = updateRealtimePreviewSurfaceBounds({
          sessionId: this.sessionId,
          bounds
        });
        this.playbackGeneration = response.playbackGeneration;
        this.lastBounds = bounds;
      }

      this.fallbackLabel = null;
      this.refreshPreviewState();
      return this.state("实时预览已接入");
    } catch (error) {
      this.attached = false;
      this.fallbackLabel = attachFailureLabel(error);
      recordRealtimePreviewHostCall({ kind: "attachFailure", bounds });
      return this.state("实时预览不可用");
    }
  }

  getTelemetryState(refreshPresentation = true): RealtimePreviewHostDisplayState {
    try {
      this.ensureSession();
      this.mockRealtimeFrameForTest();
      this.refreshPreviewState(refreshPresentation);
    } catch (error) {
      this.fallbackLabel = attachFailureLabel(error);
    }

    return this.state(this.fallbackLabel === null ? "实时预览数据已更新" : "实时预览不可用");
  }

  subscribeTelemetry(sender: WebContents): RealtimePreviewHostDisplayState {
    ensureNativePreviewEventBridge();
    this.telemetrySubscribers.set(sender.id, sender);
    sender.once("destroyed", () => {
      this.telemetrySubscribers.delete(sender.id);
    });
    const state = this.getTelemetryState();
    recordRealtimePreviewHostCall({ kind: "subscribeTelemetry" });
    return state;
  }

  unsubscribeTelemetry(sender: WebContents): void {
    this.telemetrySubscribers.delete(sender.id);
    recordRealtimePreviewHostCall({ kind: "unsubscribeTelemetry" });
  }

  handleNativePreviewEvent(event: RealtimePreviewBindingEvent): void {
    if (this.closed || this.sessionId !== event.sessionId) {
      return;
    }
    this.playbackGeneration = event.playbackGeneration;
    if (event.kind === "framePresented") {
      this.presentedNativeEventCount += 1;
      this.droppedNativeFrameCount += event.droppedFrameCount ?? 0;
      if (typeof event.targetTimeMicroseconds === "number") {
        this.lastPresentedEvent = {
          playbackGeneration: event.playbackGeneration,
          targetTimeMicroseconds: event.targetTimeMicroseconds
        };
      }
    }
    recordRealtimePreviewHostCall({
      kind: "nativePreviewEvent",
      nativeEventKind: event.kind,
      targetTimeMicroseconds: event.targetTimeMicroseconds ?? undefined,
      playbackGeneration: event.playbackGeneration,
      errorMessage: event.errorMessage ?? undefined,
      droppedFrameCount: event.droppedFrameCount ?? undefined
    });
    this.applyNativeEventToCachedEvidence(event);
    this.resolvePresentationWaiters(event);
    this.publishTelemetryState(this.shouldRefreshPresentationForNativeEvent(event));
  }

  updateProjectSessionSnapshot(
    projectSessionId: string,
    expectedRevision: number,
    interactionId: string | null = null
  ): RealtimePreviewHostDisplayState {
    try {
      this.ensureSession();
      if (this.sessionId === null) {
        throw new Error("实时预览会话尚未创建");
      }
      this.lastFrame = null;
      this.lastContentEvidence = null;
      this.telemetry = null;
      const response = updateRealtimePreviewProjectSessionSnapshot({
        sessionId: this.sessionId,
        projectSessionId,
        expectedRevision: sanitizeExpectedRevision(expectedRevision),
        ...(interactionId === null ? {} : { interactionId })
      });
      this.playbackGeneration = response.playbackGeneration;
      recordRealtimePreviewHostCall({
        kind: "updateProjectSessionSnapshot",
        interactionId,
        playbackGeneration: response.playbackGeneration
      });
      this.fallbackLabel = null;
      this.refreshPreviewState();
      const state = this.state("实时预览会话快照已更新");
      this.publishCachedState(state);
      return state;
    } catch (error) {
      this.fallbackLabel = attachFailureLabel(error);
      return this.state("实时预览不可用");
    }
  }

  async seek(targetTimeMicroseconds: number): Promise<RealtimePreviewHostDisplayState> {
    try {
      this.ensureSession();
      if (this.sessionId === null) {
        throw new Error("实时预览会话尚未创建");
      }
      const targetTime = sanitizeTargetTimeMicroseconds(targetTimeMicroseconds);
      const response = seekRealtimePreview({
        sessionId: this.sessionId,
        targetTimeMicroseconds: targetTime
      });
      this.playbackGeneration = response.playbackGeneration;
      recordRealtimePreviewHostCall({
        kind: "seek",
        targetTimeMicroseconds: targetTime,
        playbackGeneration: response.playbackGeneration
      });
      this.fallbackLabel = null;
      this.refreshPreviewState();
      if (!this.hasPresentedTarget(response.playbackGeneration, targetTime)) {
        const presented = await this.waitForPresentedTarget(
          response.playbackGeneration,
          targetTime,
          STILL_FRAME_PRESENTATION_WAIT_TIMEOUT_MS
        );
        recordRealtimePreviewHostCall({
          kind: presented ? "seekStillFramePresented" : "seekStillFrameTimeout",
          targetTimeMicroseconds: targetTime,
          playbackGeneration: response.playbackGeneration
        });
        if (presented) {
          this.refreshPreviewState();
        } else {
          this.fallbackLabel = attachFailureLabel(
            new Error("render graph GPU compositor scheduler did not present the requested still frame")
          );
          this.presentationState = null;
          this.lastContentEvidence = null;
          this.telemetry = null;
          this.publishCachedState(this.state("实时预览不可用"));
        }
      }
      const state = this.state("实时预览已寻帧");
      this.publishCachedState(state);
      return state;
    } catch (error) {
      this.fallbackLabel = attachFailureLabel(error);
      return this.state("实时预览不可用");
    }
  }

  play(): RealtimePreviewHostDisplayState {
    try {
      this.ensureSession();
      if (this.sessionId === null) {
        throw new Error("实时预览会话尚未创建");
      }
      if (renderGraphCompositorDisabledForTest()) {
        throw new Error("render graph GPU compositor scheduler disabled by product test switch");
      }
      this.ensureNativeWindowVisible();
      const response = playRealtimePreview({ sessionId: this.sessionId });
      this.playbackGeneration = response.playbackGeneration;
      recordRealtimePreviewHostCall({ kind: "schedulerPlaybackWorkerStart" });
      recordRealtimePreviewHostCall({ kind: "play", playbackGeneration: response.playbackGeneration });
      this.fallbackLabel = null;
      this.refreshPreviewState();
      return this.state("实时预览播放中");
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      this.fallbackLabel = attachFailureLabel(error);
      if (isSurfaceOccludedAcquire(errorMessage)) {
        recordRealtimePreviewHostCall({
          kind: "surfaceAcquireOccluded",
          errorMessage
        });
      }
      recordRealtimePreviewHostCall({
        kind: "playRejectedMissingCompositor",
        errorMessage
      });
      try {
        this.refreshPreviewState();
      } catch {
        this.telemetry = null;
      }
      if (renderGraphCompositorDisabledForTest()) {
        this.presentationState = null;
        this.lastContentEvidence = null;
        this.telemetry = null;
      }
      const state = this.state("实时预览不可用");
      this.publishCachedState(state);
      return state;
    }
  }

  pause(): RealtimePreviewHostDisplayState {
    return this.applySessionPlaybackCommand("pause", () => {
      if (this.sessionId === null) {
        throw new Error("实时预览会话尚未创建");
      }
      return pauseRealtimePreview({ sessionId: this.sessionId }).playbackGeneration;
    }, "实时预览已暂停");
  }

  stop(): RealtimePreviewHostDisplayState {
    return this.applySessionPlaybackCommand("stop", () => {
      if (this.sessionId === null) {
        throw new Error("实时预览会话尚未创建");
      }
      return stopRealtimePreview({ sessionId: this.sessionId }).playbackGeneration;
    }, "实时预览已停止");
  }

  hitTestTextOverlay(point: RealtimePreviewHostTextHitTestPoint): RealtimePreviewHostTextHitTestResponse {
    if (this.sessionId === null || this.lastBounds === null || this.lastContentEvidence === null) {
      return { hit: false };
    }
    const targetWidth = Math.max(1, this.lastContentEvidence.width);
    const targetHeight = Math.max(1, this.lastContentEvidence.height);
    const hostWidth = Math.max(1, this.lastBounds.width);
    const hostHeight = Math.max(1, this.lastBounds.height);
    const targetX = clampHitTestCoordinate(
      Math.round((sanitizeHitTestCoordinate(point.x) * targetWidth) / hostWidth),
      targetWidth
    );
    const targetY = clampHitTestCoordinate(
      Math.round((sanitizeHitTestCoordinate(point.y) * targetHeight) / hostHeight),
      targetHeight
    );
    const result = hitTestRealtimePreviewTextOverlay({
      sessionId: this.sessionId,
      point: { x: targetX, y: targetY }
    });
    recordRealtimePreviewHostCall({
      kind: "hitTestTextOverlay",
      targetTimeMicroseconds: result.targetTimeMicroseconds ?? undefined
    });
    return result;
  }

  taskRuntimeTelemetry(): RealtimePreviewHostTaskRuntimeTelemetry | null {
    if (this.sessionId !== null) {
      try {
        this.refreshPreviewState();
      } catch {
        // Use the last native event counters below if the live snapshot is temporarily unavailable.
      }
    }
    const telemetry = this.telemetry;
    const evidence = this.presentationState?.evidence ?? this.lastContentEvidence;
    if (telemetry === null && evidence === null && this.presentedNativeEventCount <= 0) {
      return null;
    }
    const presentedFrameCount = Math.max(
      telemetry?.presentedFrameCount ?? 0,
      evidence?.presentedFrames ?? 0,
      this.presentedNativeEventCount
    );
    const sampleCount = Math.max(
      presentedFrameCount,
      telemetry?.framePacing.sampleCount ?? 0,
      evidence?.submittedDraws ?? 0
    );
    const queueLatencyUs = telemetry?.schedulerQueueLatencyP95Us ?? (telemetry?.queueLatencyMs ?? 0) * 1000;
    if (sampleCount <= 0 && queueLatencyUs <= 0) {
      return null;
    }
    return {
      submittedCount: sampleCount,
      admittedCount: sampleCount,
      startedCount: sampleCount,
      completedCount: presentedFrameCount,
      rejectedCount: telemetry?.schedulerRejectedCount ?? 0,
      canceledCount: telemetry?.schedulerCanceledCount ?? telemetry?.canceledRequestCount ?? 0,
      staleRejectedCount: telemetry?.schedulerStaleRejectedCount ?? telemetry?.staleRejectedCount ?? 0,
      fallbackCount: telemetry?.fallbackCount ?? 0,
      cacheHitCount: telemetry?.cacheHitCount ?? 0,
      firstFrameTimeUs: telemetry?.firstFrameLatencyMs === null || telemetry?.firstFrameLatencyMs === undefined
        ? null
        : telemetry.firstFrameLatencyMs * 1000,
      droppedFrameCount: Math.max(telemetry?.droppedFrameCount ?? 0, this.droppedNativeFrameCount),
      repeatedFrameCount: telemetry?.repeatedFrameCount ?? 0,
      resourceSaturationCount: telemetry?.schedulerResourceSaturationCount ?? 0,
      queueLatencyUs: {
        sampleCount,
        p50: queueLatencyUs,
        p95: queueLatencyUs,
        max: queueLatencyUs
      }
    };
  }

  detachSurface(): RealtimePreviewHostDisplayState {
    if (this.sessionId !== null && this.attached) {
      try {
        detachRealtimePreviewSurface({ sessionId: this.sessionId });
        recordRealtimePreviewHostCall({ kind: "detachSurface" });
      } catch (error) {
        this.fallbackLabel = attachFailureLabel(error);
        return this.state("实时预览不可用");
      }
    }

    this.attached = false;
    this.lastBounds = null;
    return this.state("实时预览表面已隐藏");
  }

  close(): void {
    if (this.closed) {
      return;
    }

    this.closed = true;
    for (const { eventName, listener } of this.windowGeometryListeners) {
      this.window.off(eventName, listener);
    }
    this.windowGeometryListeners.length = 0;
    if (this.pendingGeometryReflow !== null) {
      clearTimeout(this.pendingGeometryReflow);
      this.pendingGeometryReflow = null;
    }
    this.pendingGeometryReflowReason = null;
    for (const waiter of [...this.pendingPresentationWaiters]) {
      this.finishPresentationWaiter(waiter, false);
    }
    if (this.sessionId === null) {
      return;
    }

    hostsBySessionId.delete(this.sessionId);
    if (this.attached) {
      try {
        detachRealtimePreviewSurface({ sessionId: this.sessionId });
        recordRealtimePreviewHostCall({ kind: "detachSurface" });
      } catch {
        this.fallbackLabel = "实时预览关闭时不可用";
      }
    }

    try {
      closeRealtimePreviewSession({ sessionId: this.sessionId });
      recordRealtimePreviewHostCall({ kind: "closeSession" });
    } finally {
      this.sessionId = null;
      this.attached = false;
    }
  }

  private ensureSession(): void {
    if (this.sessionId !== null) {
      return;
    }

    const response = createRealtimePreviewSession({
      sessionLabel: `desktop-preview-${this.window.id}`,
      frameRateNumerator: 30,
      frameRateDenominator: 1,
      playbackRateNumerator: 1,
      playbackRateDenominator: 1
    });
    this.sessionId = response.sessionId;
    hostsBySessionId.set(response.sessionId, this);
    this.playbackGeneration = response.playbackGeneration;
    recordRealtimePreviewHostCall({ kind: "createSession" });
  }

  private scheduleSurfaceReflow(reason: string): void {
    if (this.closed || !this.attached || this.sessionId === null || this.lastBounds === null) {
      return;
    }

    this.pendingGeometryReflowReason = reason;
    if (this.pendingGeometryReflow !== null) {
      return;
    }

    this.pendingGeometryReflow = setTimeout(() => {
      const reflowReason = this.pendingGeometryReflowReason ?? reason;
      this.pendingGeometryReflow = null;
      this.pendingGeometryReflowReason = null;
      this.reflowSurfacePlacement(reflowReason);
    }, WINDOW_SURFACE_REFLOW_DELAY_MS);
  }

  private reflowSurfacePlacement(reason: string): void {
    if (this.closed || !this.attached || this.sessionId === null || this.lastBounds === null || this.window.isDestroyed()) {
      return;
    }

    try {
      const startedAt = performance.now();
      const response = updateRealtimePreviewSurfaceBounds({
        sessionId: this.sessionId,
        bounds: this.lastBounds
      });
      const durationMs = Math.round(performance.now() - startedAt);
      this.playbackGeneration = response.playbackGeneration;
      recordRealtimePreviewHostCall({
        kind: "reflowSurfaceBounds",
        bounds: this.lastBounds,
        reflowReason: reason,
        durationMs,
        playbackGeneration: response.playbackGeneration
      });
      this.fallbackLabel = null;
      this.refreshPreviewState();
      this.publishCachedState(this.state("实时预览表面已对齐"));
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      this.fallbackLabel = attachFailureLabel(error);
      recordRealtimePreviewHostCall({
        kind: "reflowSurfaceBoundsFailed",
        bounds: this.lastBounds,
        reflowReason: reason,
        errorMessage
      });
      this.publishCachedState(this.state("实时预览不可用"));
    }
  }

  private buildSurfaceDescriptor(bounds: RealtimePreviewSurfaceBounds): RealtimePreviewSurfaceDescriptor {
    const nativeHandle = this.window.getNativeWindowHandle();
    recordRealtimePreviewHostCall({
      kind: "acquireNativeWindowHandle",
      parentHandleByteLength: nativeHandle.byteLength
    });

    if (process.env.VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_ATTACH_FAILURE === "1") {
      throw new Error("实时预览测试不可用");
    }

    const kind = nativeSurfaceKind();
    if (kind === "offscreen") {
      return {
        kind,
        ...bounds
      };
    }
    if (kind === "macosNsView") {
      return {
        kind,
        parentHandleHex: nativeParentHandleToHex(nativeHandle),
        ...bounds
      };
    }

    const parentHandle = nativeParentHandleToNumber(nativeHandle);
    return {
      kind,
      parentHandle,
      ...bounds
    };
  }

  private ensureNativeWindowVisible(): void {
    if (this.window.isDestroyed()) {
      throw new Error("实时预览窗口已关闭");
    }
    if (this.window.isMinimized()) {
      this.window.restore();
    }
    if (!this.window.isVisible()) {
      this.window.show();
    }
    app.show();
    this.window.setFocusable(true);
    if (process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS === "1") {
      this.window.setAlwaysOnTop(true, "screen-saver");
    }
    this.window.focus();
    this.window.moveTop();
    app.focus({ steal: true });
    recordRealtimePreviewHostCall({
      kind: "prepareNativeWindowVisible",
      windowVisible: this.window.isVisible(),
      windowFocused: this.window.isFocused(),
      appFocused: BrowserWindow.getFocusedWindow()?.id === this.window.id
    });
  }

  private refreshPreviewState(refreshPresentation = true): void {
    if (this.sessionId === null) {
      return;
    }

    if (refreshPresentation) {
      this.refreshPresentationSnapshot();
    }
    this.telemetry = getRealtimePreviewTelemetry({ sessionId: this.sessionId });
    recordRealtimePreviewHostCall({ kind: "refreshTelemetrySnapshot" });
  }

  private publishTelemetryState(refreshPresentation = true): void {
    if (this.closed || this.telemetrySubscribers.size === 0) {
      return;
    }

    const state = this.getTelemetryState(refreshPresentation);
    for (const [senderId, sender] of this.telemetrySubscribers) {
      if (sender.isDestroyed()) {
        this.telemetrySubscribers.delete(senderId);
        continue;
      }
      this.sendTelemetryState(sender, state);
    }
    recordRealtimePreviewHostCall({
      kind: "pushTelemetry",
      presentedFrameCount: state.telemetry?.presentedFrameCount,
      playbackGeneration: state.playbackGeneration ?? undefined,
      presentationAvailable: state.productReady
    });
  }

  private sendTelemetryState(sender: WebContents, state: RealtimePreviewHostDisplayState): void {
    if (sender.isDestroyed()) {
      return;
    }
    sender.send(TELEMETRY_STATE_CHANNEL, state);
  }

  private publishCachedState(state: RealtimePreviewHostDisplayState): void {
    if (this.closed || this.telemetrySubscribers.size === 0) {
      return;
    }
    for (const [senderId, sender] of this.telemetrySubscribers) {
      if (sender.isDestroyed()) {
        this.telemetrySubscribers.delete(senderId);
        continue;
      }
      this.sendTelemetryState(sender, state);
    }
    recordRealtimePreviewHostCall({
      kind: "pushTelemetry",
      presentedFrameCount: state.telemetry?.presentedFrameCount,
      playbackGeneration: state.playbackGeneration ?? undefined,
      presentationAvailable: state.productReady
    });
  }

  private refreshPresentationSnapshot(): void {
    if (this.sessionId === null) {
      return;
    }

    const startedAt = performance.now();
    this.presentationState = getRealtimePreviewPresentationState({ sessionId: this.sessionId });
    const durationMs = Math.round(performance.now() - startedAt);
    this.lastPresentationSnapshotRefreshAt = performance.now();
    this.lastContentEvidence = this.presentationState.evidence ?? null;
    if (this.hasProductionCompositedPresenter()) {
      this.fallbackLabel = null;
    }
    recordRealtimePreviewHostCall({
      kind: "getPresentationState",
      durationMs,
      targetTimeMicroseconds: this.presentationState.evidence?.targetTimeMicroseconds,
      playbackGeneration: this.playbackGeneration ?? undefined,
      presentationAvailable: this.presentationState.available,
      presentationBackend: this.presentationState.backend,
      unsupportedReason: this.presentationState.unsupportedReason ?? null
    });
  }

  private shouldRefreshPresentationForNativeEvent(event: RealtimePreviewBindingEvent): boolean {
    if (event.kind !== "framePresented") {
      return true;
    }
    if (this.presentationState === null || !this.hasProductionCompositedPresenter()) {
      return true;
    }
    return performance.now() - this.lastPresentationSnapshotRefreshAt >= PRESENTATION_EVENT_REFRESH_INTERVAL_MS;
  }

  private applyNativeEventToCachedEvidence(event: RealtimePreviewBindingEvent): void {
    if (event.kind !== "framePresented" || event.targetTimeMicroseconds === undefined || this.lastContentEvidence === null) {
      return;
    }
    this.lastContentEvidence = {
      ...this.lastContentEvidence,
      targetTimeMicroseconds: event.targetTimeMicroseconds
    };
  }

  private hasPresentedTarget(playbackGeneration: number, targetTimeMicroseconds: number): boolean {
    if (this.playbackGeneration !== playbackGeneration || !this.hasProductionCompositedPresenter()) {
      return false;
    }
    if (
      this.lastPresentedEvent === null ||
      this.lastPresentedEvent.playbackGeneration !== playbackGeneration ||
      !presentationTargetMatches(this.lastPresentedEvent.targetTimeMicroseconds, targetTimeMicroseconds)
    ) {
      return false;
    }
    return presentationTargetMatches(this.lastContentEvidence?.targetTimeMicroseconds, targetTimeMicroseconds);
  }

  private waitForPresentedTarget(
    playbackGeneration: number,
    targetTimeMicroseconds: number,
    timeoutMs: number
  ): Promise<boolean> {
    if (this.hasPresentedTarget(playbackGeneration, targetTimeMicroseconds)) {
      return Promise.resolve(true);
    }

    return new Promise((resolve) => {
      const waiter: PendingPresentationWaiter = {
        playbackGeneration,
        targetTimeMicroseconds,
        timeout: setTimeout(() => {
          this.finishPresentationWaiter(waiter, false);
        }, timeoutMs),
        resolve
      };
      this.pendingPresentationWaiters.add(waiter);
    });
  }

  private resolvePresentationWaiters(event: RealtimePreviewBindingEvent): void {
    if (this.pendingPresentationWaiters.size === 0) {
      return;
    }

    for (const waiter of [...this.pendingPresentationWaiters]) {
      if (waiter.playbackGeneration !== this.playbackGeneration) {
        this.finishPresentationWaiter(waiter, false);
        continue;
      }
      if (
        event.kind === "framePresented" &&
        event.playbackGeneration === waiter.playbackGeneration &&
        presentationTargetMatches(event.targetTimeMicroseconds, waiter.targetTimeMicroseconds)
      ) {
        this.finishPresentationWaiter(waiter, true);
      }
    }
  }

  private finishPresentationWaiter(waiter: PendingPresentationWaiter, presented: boolean): void {
    if (!this.pendingPresentationWaiters.delete(waiter)) {
      return;
    }
    clearTimeout(waiter.timeout);
    waiter.resolve(presented);
  }

  private applySessionPlaybackCommand(
    kind: "play" | "pause" | "stop",
    command: () => number,
    statusLabel: string
  ): RealtimePreviewHostDisplayState {
    try {
      this.ensureSession();
      const playbackGeneration = command();
      this.playbackGeneration = playbackGeneration;
      recordRealtimePreviewHostCall({ kind, playbackGeneration });
      this.fallbackLabel = null;
      this.refreshPreviewState();
      return this.state(statusLabel);
    } catch (error) {
      this.fallbackLabel = attachFailureLabel(error);
      return this.state("实时预览不可用");
    }
  }

  private mockRealtimeFrameForTest(): void {
    if (this.sessionId === null || this.playbackGeneration === null || this.lastFrame !== null) {
      return;
    }

    if (process.env.VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_CANCELED === "1") {
      const cancellationToken = nextRealtimePreviewCancellationToken({ sessionId: this.sessionId });
      cancelRealtimePreviewRequest({
        sessionId: this.sessionId,
        cancellationToken
      });
      this.lastFrame = requestRealtimePreviewFrame({
        sessionId: this.sessionId,
        frame: {
          targetTimeMicroseconds: 1_200_000,
          playbackGeneration: this.playbackGeneration,
          queueLatencyMs: 1,
          renderDurationMs: 3,
          mode: "seek",
          cancellationToken,
          cacheHit: false
        }
      });
      recordRealtimePreviewHostCall({ kind: "requestCanceledFrame" });
      return;
    }

    if (process.env.VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_SEEK_FRAME === "1") {
      this.lastFrame = requestRealtimePreviewFrame({
        sessionId: this.sessionId,
        frame: {
          targetTimeMicroseconds: 1_200_000,
          playbackGeneration: this.playbackGeneration,
          queueLatencyMs: 2,
          renderDurationMs: 5,
          mode: "seek",
          cacheHit: false
        }
      });
      recordRealtimePreviewHostCall({ kind: "requestSeekFrame" });
      return;
    }

    if (process.env.VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_FIRST_FRAME !== "1") {
      return;
    }

    this.lastFrame = requestRealtimePreviewFrame({
      sessionId: this.sessionId,
      frame: {
        targetTimeMicroseconds: 0,
        playbackGeneration: this.playbackGeneration,
        queueLatencyMs: 4,
        renderDurationMs: 5,
        mode: "firstFrame",
        cacheHit: false
      }
    });
    recordRealtimePreviewHostCall({ kind: "requestFirstFrame" });
  }

  private hasProductionCompositedPresenter(): boolean {
    return (
      this.presentationState?.available === true &&
      this.presentationState.backend === "renderGraphGpu" &&
      this.presentationState.evidence?.source === "renderGraphGpuComposited"
    );
  }

  private state(statusLabel: string): RealtimePreviewHostDisplayState {
    const productReady = this.hasProductionCompositedPresenter();
    const fallbackLabel = productReady ? null : this.fallbackLabel;
    const backend: RealtimePreviewHostProductBackend = productReady ? "renderGraphGpu" : "none";
    const diagnosticSource: RealtimePreviewHostDiagnosticSource =
      this.presentationState?.backend === "nativeVideoBridge"
        ? "nativeVideoBridge"
        : this.lastFrame !== null
          ? "runtimeFrameRequest"
          : "none";
    const fallbackReason = this.lastFrame?.fallback ?? null;
    return {
      ok: fallbackLabel === null,
      productReady,
      hostAttached: this.attached,
      fallbackActive: fallbackLabel !== null,
      statusLabel,
      fallbackLabel,
      unsupportedReason: this.presentationState?.unsupportedReason ?? null,
      playbackGeneration: this.playbackGeneration,
      backend,
      diagnosticSource,
      fallbackReason,
      currentRequestCanceled: this.lastFrame?.canceled ?? false,
      fallbackArtifactVisible: this.lastFrame?.backend === "previewArtifact" || this.lastFrame?.backend === "ffmpegArtifact",
      telemetry: this.telemetry,
      frameDisplay: null,
      contentEvidence: this.lastContentEvidence,
      surfacePlacement: this.surfacePlacement()
    };
  }

  private surfacePlacement(): RealtimePreviewHostSurfacePlacement | null {
    const nativeAppKitScreenRect = this.presentationState?.surfacePlacement?.nativeScreenRect ?? null;
    const nativeDrawableLifecycleDiagnostic =
      this.presentationState?.surfacePlacement?.drawableLifecycleDiagnostic ?? null;
    if (nativeAppKitScreenRect === null || this.lastBounds === null || this.window.isDestroyed()) {
      return null;
    }

    const hostScreenRect = hostScreenRectForBounds(this.window, this.lastBounds);
    const nativeScreenRect = appKitScreenRectToElectronScreenRect(this.window, nativeAppKitScreenRect);
    const deltaPx = rectDelta(hostScreenRect, nativeScreenRect);
    const maxDeltaPx = maxRectDelta(hostScreenRect, nativeScreenRect);
    return {
      surfaceBoundsCoordinateSpace: "browserWindowContentLogicalPixels",
      screenRectCoordinateSpace: "electronScreenLogicalPixels",
      hostScreenRect,
      nativeScreenRect,
      nativeAppKitScreenRect,
      nativeDrawableLifecycleDiagnostic,
      deltaPx,
      maxDeltaPx,
      aligned: maxDeltaPx <= 2
    };
  }
}

export function getRealtimePreviewHostTaskRuntimeTelemetry(): RealtimePreviewHostTaskRuntimeTelemetry | null {
  let aggregate: RealtimePreviewHostTaskRuntimeTelemetry | null = null;
  for (const host of hostsByWindowId.values()) {
    const telemetry = host.taskRuntimeTelemetry();
    if (telemetry === null) {
      continue;
    }
    aggregate = aggregate === null ? telemetry : mergeRealtimePreviewHostTaskRuntimeTelemetry(aggregate, telemetry);
  }
  return aggregate;
}

function mergeRealtimePreviewHostTaskRuntimeTelemetry(
  first: RealtimePreviewHostTaskRuntimeTelemetry,
  second: RealtimePreviewHostTaskRuntimeTelemetry
): RealtimePreviewHostTaskRuntimeTelemetry {
  return {
    submittedCount: first.submittedCount + second.submittedCount,
    admittedCount: first.admittedCount + second.admittedCount,
    startedCount: first.startedCount + second.startedCount,
    completedCount: first.completedCount + second.completedCount,
    rejectedCount: first.rejectedCount + second.rejectedCount,
    canceledCount: first.canceledCount + second.canceledCount,
    staleRejectedCount: first.staleRejectedCount + second.staleRejectedCount,
    fallbackCount: first.fallbackCount + second.fallbackCount,
    cacheHitCount: first.cacheHitCount + second.cacheHitCount,
    firstFrameTimeUs: minNullable(first.firstFrameTimeUs, second.firstFrameTimeUs),
    droppedFrameCount: first.droppedFrameCount + second.droppedFrameCount,
    repeatedFrameCount: first.repeatedFrameCount + second.repeatedFrameCount,
    resourceSaturationCount: first.resourceSaturationCount + second.resourceSaturationCount,
    queueLatencyUs: {
      sampleCount: first.queueLatencyUs.sampleCount + second.queueLatencyUs.sampleCount,
      p50: maxNullable(first.queueLatencyUs.p50 ?? null, second.queueLatencyUs.p50 ?? null),
      p95: maxNullable(first.queueLatencyUs.p95 ?? null, second.queueLatencyUs.p95 ?? null),
      max: maxNullable(first.queueLatencyUs.max ?? null, second.queueLatencyUs.max ?? null)
    }
  };
}

function minNullable(first: number | null, second: number | null): number | null {
  if (first === null) {
    return second;
  }
  if (second === null) {
    return first;
  }
  return Math.min(first, second);
}

function maxNullable(first: number | null, second: number | null): number | null {
  if (first === null) {
    return second;
  }
  if (second === null) {
    return first;
  }
  return Math.max(first, second);
}

function sanitizeTargetTimeMicroseconds(value: number): number {
  return Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0;
}

function sanitizeHitTestCoordinate(value: number): number {
  return Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0;
}

function clampHitTestCoordinate(value: number, span: number): number {
  return Math.max(0, Math.min(Math.max(0, span - 1), value));
}

function sanitizeExpectedRevision(value: number): number {
  return Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0;
}

function normalizeHostRect(rect: RealtimePreviewHostRectInput): RealtimePreviewSurfaceBounds | null {
  const x = finiteRounded(rect.x);
  const y = finiteRounded(rect.y);
  const width = finiteRounded(rect.width);
  const height = finiteRounded(rect.height);
  const scaleFactorMillis = finiteRounded(rect.scaleFactorMillis);

  if (x === null || y === null || width === null || height === null || scaleFactorMillis === null) {
    return null;
  }

  if (width <= 0 || height <= 0 || scaleFactorMillis <= 0) {
    return null;
  }

  return { x, y, width, height, scaleFactorMillis };
}

function finiteRounded(value: number): number | null {
  return Number.isFinite(value) ? Math.round(value) : null;
}

function sameBounds(first: RealtimePreviewSurfaceBounds | null, second: RealtimePreviewSurfaceBounds): boolean {
  return (
    first !== null &&
    first.x === second.x &&
    first.y === second.y &&
    first.width === second.width &&
    first.height === second.height &&
    first.scaleFactorMillis === second.scaleFactorMillis
  );
}

function hostScreenRectForBounds(window: BrowserWindow, bounds: RealtimePreviewSurfaceBounds): RealtimePreviewScreenRect {
  const contentBounds = window.getContentBounds();
  return {
    x: contentBounds.x + bounds.x,
    y: contentBounds.y + bounds.y,
    width: bounds.width,
    height: bounds.height
  };
}

function appKitScreenRectToElectronScreenRect(
  window: BrowserWindow,
  rect: RealtimePreviewScreenRect
): RealtimePreviewScreenRect {
  const display = screen.getDisplayMatching(window.getBounds());
  return {
    x: rect.x,
    y: display.bounds.y + display.bounds.height - rect.y - rect.height,
    width: rect.width,
    height: rect.height
  };
}

function rectDelta(first: RealtimePreviewScreenRect, second: RealtimePreviewScreenRect): RealtimePreviewScreenRect {
  return {
    x: second.x - first.x,
    y: second.y - first.y,
    width: second.width - first.width,
    height: second.height - first.height
  };
}

function maxRectDelta(first: RealtimePreviewScreenRect, second: RealtimePreviewScreenRect): number {
  return Math.max(
    Math.abs(first.x - second.x),
    Math.abs(first.y - second.y),
    Math.abs(first.width - second.width),
    Math.abs(first.height - second.height)
  );
}

function presentationTargetMatches(actual: number | null | undefined, expected: number): boolean {
  return typeof actual === "number" && Math.abs(actual - expected) <= PRESENTATION_TARGET_TOLERANCE_US;
}

function nativeSurfaceKind(): RealtimePreviewSurfaceDescriptor["kind"] {
  if (process.env.VIDEO_EDITOR_TEST_REALTIME_PREVIEW_SURFACE_KIND === "mock") {
    return "mock";
  }
  if (process.platform === "win32") {
    return "windowsHwnd";
  }
  if (process.platform === "darwin") {
    return "macosNsView";
  }
  return "mock";
}

function nativeParentHandleToNumber(handle: Buffer): number {
  if (handle.byteLength === 0) {
    return 0;
  }

  const padded = Buffer.alloc(8);
  handle.copy(padded, 0, 0, Math.min(handle.byteLength, 8));
  const value = padded.readBigUInt64LE(0);
  const safeMask = BigInt(Number.MAX_SAFE_INTEGER);
  const safeValue = Number(value & safeMask);
  return safeValue === 0 ? 1 : safeValue;
}

function nativeParentHandleToHex(handle: Buffer): string {
  if (handle.byteLength === 0) {
    return "";
  }

  const padded = Buffer.alloc(8);
  handle.copy(padded, 0, 0, Math.min(handle.byteLength, 8));
  const value = padded.readBigUInt64LE(0);
  return value.toString(16);
}

function attachFailureLabel(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return `实时预览不可用：${message}`;
}

function isSurfaceOccludedAcquire(message: string): boolean {
  return message.includes("wgpu surface texture acquire failed: surface is occluded");
}

function renderGraphCompositorDisabledForTest(): boolean {
  return (
    process.env.VIDEO_EDITOR_TEST_DISABLE_RENDER_GRAPH_COMPOSITOR === "1" ||
    process.argv.some((value) => {
      const prefix = "--video-editor-test-disable-render-graph-compositor";
      return value === prefix || value === `${prefix}=1` || value.startsWith(`${prefix}=`);
    })
  );
}

function recordRealtimePreviewHostCall(call: RealtimePreviewHostRecord): void {
  if (process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS !== "1") {
    return;
  }

  globalThis.__videoEditorTestRealtimePreviewHostCalls ??= [];
  globalThis.__videoEditorTestRealtimePreviewHostCalls.push(call);
}

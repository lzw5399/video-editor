import { app, BrowserWindow, ipcMain, screen, type IpcMainInvokeEvent } from "electron";

import type { Draft } from "../generated/Draft";
import {
  attachRealtimePreviewSurface,
  cancelRealtimePreviewRequest,
  closeRealtimePreviewSession,
  createRealtimePreviewSession,
  detachRealtimePreviewSurface,
  getRealtimePreviewTelemetry,
  getRealtimePreviewPresentationState,
  nextRealtimePreviewCancellationToken,
  pauseRealtimePreview,
  playRealtimePreview,
  requestRealtimePreviewFrame,
  seekRealtimePreview,
  stopRealtimePreview,
  updateRealtimePreviewDraftSnapshot,
  updateRealtimePreviewSurfaceBounds,
  type RealtimePreviewFallbackReason,
  type RealtimePreviewFrameResponse,
  type RealtimePreviewPresentationStateResponse,
  type RealtimePreviewScreenRect,
  type RealtimePreviewSurfaceBounds,
  type RealtimePreviewSurfaceDescriptor,
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
};

export type RealtimePreviewHostSurfacePlacement = {
  hostScreenRect: RealtimePreviewScreenRect;
  nativeScreenRect: RealtimePreviewScreenRect;
  maxDeltaPx: number;
  aligned: boolean;
};

type RealtimePreviewHostRecord = {
  kind: string;
  parentHandleByteLength?: number;
  surfaceKind?: string;
  bounds?: RealtimePreviewSurfaceBounds;
  windowVisible?: boolean;
  windowFocused?: boolean;
  appFocused?: boolean;
  targetTimeMicroseconds?: number;
  playbackGeneration?: number;
  errorMessage?: string;
};

declare global {
  var __videoEditorTestRealtimePreviewHostCalls: RealtimePreviewHostRecord[] | undefined;
}

type SenderAssertion = (event: IpcMainInvokeEvent) => void;

const hostsByWindowId = new Map<number, RealtimePreviewHost>();
let realtimePreviewHostIpcInstalled = false;

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
  ipcMain.handle("realtimePreviewHost:getTelemetry", (event) => {
    assertAllowedSender(event);
    return hostForEvent(event).getTelemetryState();
  });
  ipcMain.handle("realtimePreviewHost:updateDraftSnapshot", (event, draft: Draft, bundlePath?: string) => {
    assertAllowedSender(event);
    return hostForEvent(event).updateDraftSnapshot(draft, bundlePath);
  });
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

export class RealtimePreviewHost {
  private sessionId: string | null = null;
  private playbackGeneration: number | null = null;
  private attached = false;
  private fallbackLabel: string | null = null;
  private telemetry: RealtimePreviewTelemetryResponse | null = null;
  private presentationState: RealtimePreviewPresentationStateResponse | null = null;
  private lastFrame: RealtimePreviewFrameResponse | null = null;
  private lastContentEvidence: RealtimePreviewHostContentEvidence | null = null;
  private lastBounds: RealtimePreviewSurfaceBounds | null = null;
  private draftSnapshot: Draft | null = null;
  private bundlePath: string | null = null;
  private playbackPositionMicroseconds = 0;
  private playbackTimer: ReturnType<typeof setInterval> | null = null;
  private playbackTickInFlight = false;
  private playbackRunning = false;
  private closed = false;

  constructor(private readonly window: BrowserWindow) {
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
      this.refreshTelemetry({ presentPlaybackTick: !this.playbackRunning });
      return this.state("实时预览已接入");
    } catch (error) {
      this.attached = false;
      this.fallbackLabel = attachFailureLabel(error);
      recordRealtimePreviewHostCall({ kind: "attachFailure", bounds });
      return this.state("实时预览不可用");
    }
  }

  getTelemetryState(): RealtimePreviewHostDisplayState {
    try {
      this.ensureSession();
      this.mockRealtimeFrameForTest();
      this.refreshTelemetry({ presentPlaybackTick: !this.playbackRunning });
    } catch (error) {
      this.fallbackLabel = attachFailureLabel(error);
    }

    return this.state(this.fallbackLabel === null ? "实时预览数据已更新" : "实时预览不可用");
  }

  updateDraftSnapshot(draft: Draft, bundlePath?: string): RealtimePreviewHostDisplayState {
    try {
      this.ensureSession();
      if (this.sessionId === null) {
        throw new Error("实时预览会话尚未创建");
      }
      this.draftSnapshot = draft;
      this.bundlePath = typeof bundlePath === "string" && bundlePath.trim().length > 0 ? bundlePath : null;
      this.lastFrame = null;
      this.lastContentEvidence = null;
      this.telemetry = null;
      this.stopPlaybackTimer();
      this.playbackPositionMicroseconds = Math.min(
        this.playbackPositionMicroseconds,
        sequenceDurationMicroseconds(draft)
      );
      const response = updateRealtimePreviewDraftSnapshot({
        sessionId: this.sessionId,
        draft,
        ...(this.bundlePath === null ? {} : { bundlePath: this.bundlePath })
      });
      this.playbackGeneration = response.playbackGeneration;
      recordRealtimePreviewHostCall({
        kind: "updateDraftSnapshot",
        playbackGeneration: response.playbackGeneration
      });
      this.fallbackLabel = null;
      this.refreshTelemetry({ presentPlaybackTick: false });
      return this.state("实时预览草稿已更新");
    } catch (error) {
      this.fallbackLabel = attachFailureLabel(error);
      return this.state("实时预览不可用");
    }
  }

  seek(targetTimeMicroseconds: number): RealtimePreviewHostDisplayState {
    try {
      this.ensureSession();
      if (this.sessionId === null) {
        throw new Error("实时预览会话尚未创建");
      }
      const targetTime = sanitizeTargetTimeMicroseconds(targetTimeMicroseconds);
      this.stopPlaybackTimer();
      this.playbackPositionMicroseconds = targetTime;
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
      this.refreshTelemetry({ presentPlaybackTick: false });
      return this.state("实时预览已寻帧");
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
      this.stopPlaybackTimer();
      this.ensureNativeWindowVisible();
      const response = playRealtimePreview({ sessionId: this.sessionId });
      this.playbackGeneration = response.playbackGeneration;
      recordRealtimePreviewHostCall({ kind: "schedulerDecodeCurrentFrame" });
      recordRealtimePreviewHostCall({ kind: "schedulerBuildRenderGraph" });
      recordRealtimePreviewHostCall({ kind: "schedulerPresentSurface" });
      recordRealtimePreviewHostCall({ kind: "play", playbackGeneration: response.playbackGeneration });
      this.fallbackLabel = null;
      this.refreshTelemetry({ presentPlaybackTick: true });
      if (this.hasProductionCompositedPresenter()) {
        this.startPlaybackTimer();
        recordRealtimePreviewHostCall({
          kind: "schedulerCompositedEvidence",
          targetTimeMicroseconds: this.lastContentEvidence?.targetTimeMicroseconds,
          playbackGeneration: response.playbackGeneration
        });
        return this.state("实时预览播放中");
      }
      this.playbackRunning = false;
      this.fallbackLabel = `实时预览不可用：${
        this.presentationState?.unsupportedReason?.slice(0, 120) ?? "GPU 合成播放尚未接入"
      }`;
      recordRealtimePreviewHostCall({
        kind: "playRejectedMissingCompositor",
        errorMessage: this.fallbackLabel
      });
      return this.state("实时预览不可用");
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
      this.stopPlaybackTimer();
      try {
        this.refreshTelemetry({ presentPlaybackTick: false });
      } catch {
        this.telemetry = null;
      }
      return this.state("实时预览不可用");
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
    this.playbackPositionMicroseconds = 0;
    return this.applySessionPlaybackCommand("stop", () => {
      if (this.sessionId === null) {
        throw new Error("实时预览会话尚未创建");
      }
      return stopRealtimePreview({ sessionId: this.sessionId }).playbackGeneration;
    }, "实时预览已停止");
  }

  close(): void {
    if (this.closed) {
      return;
    }

    this.closed = true;
    this.stopPlaybackTimer();
    if (this.sessionId === null) {
      return;
    }

    if (this.attached) {
      try {
        this.stopPlaybackTimer();
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
    this.playbackGeneration = response.playbackGeneration;
    recordRealtimePreviewHostCall({ kind: "createSession" });
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

    const parentHandle = nativeParentHandleToNumber(nativeHandle);
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

  private refreshTelemetry({ presentPlaybackTick }: { presentPlaybackTick: boolean }): void {
    if (this.sessionId === null) {
      return;
    }

    if (presentPlaybackTick) {
      this.refreshPresentationState();
    }
    this.telemetry = getRealtimePreviewTelemetry({ sessionId: this.sessionId });
    recordRealtimePreviewHostCall({ kind: "getTelemetry" });
  }

  private refreshPresentationState(): void {
    if (this.sessionId === null) {
      return;
    }

    this.presentationState = getRealtimePreviewPresentationState({ sessionId: this.sessionId });
    this.lastContentEvidence = this.presentationState.evidence ?? null;
    recordRealtimePreviewHostCall({ kind: "getPresentationState" });
  }

  private applySessionPlaybackCommand(
    kind: "play" | "pause" | "stop",
    command: () => number,
    statusLabel: string
  ): RealtimePreviewHostDisplayState {
    try {
      this.ensureSession();
      if (kind !== "play") {
        this.stopPlaybackTimer();
      }
      const playbackGeneration = command();
      this.playbackGeneration = playbackGeneration;
      recordRealtimePreviewHostCall({ kind, playbackGeneration });
      this.fallbackLabel = null;
      this.refreshTelemetry({ presentPlaybackTick: false });
      return this.state(statusLabel);
    } catch (error) {
      this.fallbackLabel = attachFailureLabel(error);
      return this.state("实时预览不可用");
    }
  }

  private startPlaybackTimer(): void {
    if (this.playbackTimer !== null || this.sessionId === null) {
      this.playbackRunning = true;
      return;
    }

    this.playbackRunning = true;
    this.playbackTimer = setInterval(() => {
      this.presentPlaybackTimerTick();
    }, 33);
  }

  private stopPlaybackTimer(): void {
    this.playbackRunning = false;
    this.playbackTickInFlight = false;
    if (this.playbackTimer === null) {
      return;
    }
    clearInterval(this.playbackTimer);
    this.playbackTimer = null;
  }

  private presentPlaybackTimerTick(): void {
    if (this.closed || this.sessionId === null || !this.playbackRunning || this.playbackTickInFlight) {
      return;
    }

    this.playbackTickInFlight = true;
    try {
      this.refreshTelemetry({ presentPlaybackTick: true });
      const sequenceDuration = this.draftSnapshot === null ? 0 : sequenceDurationMicroseconds(this.draftSnapshot);
      const targetTime = Math.max(
        this.telemetry?.targetTimeMicroseconds ?? 0,
        this.lastContentEvidence?.targetTimeMicroseconds ?? 0
      );
      if (sequenceDuration > 0 && targetTime >= sequenceDuration) {
        this.pauseAtSequenceEnd();
      }
    } catch (error) {
      this.fallbackLabel = attachFailureLabel(error);
      recordRealtimePreviewHostCall({
        kind: "playbackTimerError",
        errorMessage: error instanceof Error ? error.message : String(error)
      });
      this.stopPlaybackTimer();
    } finally {
      this.playbackTickInFlight = false;
    }
  }

  private pauseAtSequenceEnd(): void {
    if (this.sessionId === null) {
      this.stopPlaybackTimer();
      return;
    }

    this.playbackRunning = false;
    if (this.playbackTimer !== null) {
      clearInterval(this.playbackTimer);
      this.playbackTimer = null;
    }
    const response = pauseRealtimePreview({ sessionId: this.sessionId });
    this.playbackGeneration = response.playbackGeneration;
    recordRealtimePreviewHostCall({ kind: "pauseAtSequenceEnd", playbackGeneration: response.playbackGeneration });
    this.refreshTelemetry({ presentPlaybackTick: false });
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
    const backend: RealtimePreviewHostProductBackend = productReady ? "renderGraphGpu" : "none";
    const diagnosticSource: RealtimePreviewHostDiagnosticSource =
      this.presentationState?.backend === "nativeVideoBridge"
        ? "nativeVideoBridge"
        : this.lastFrame !== null
          ? "runtimeFrameRequest"
          : "none";
    const fallbackReason = this.lastFrame?.fallback ?? null;
    return {
      ok: this.fallbackLabel === null,
      productReady,
      hostAttached: this.attached,
      fallbackActive: this.fallbackLabel !== null,
      statusLabel,
      fallbackLabel: this.fallbackLabel,
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
    const nativeScreenRect = this.presentationState?.surfacePlacement?.nativeScreenRect ?? null;
    if (nativeScreenRect === null || this.lastBounds === null || this.window.isDestroyed()) {
      return null;
    }

    const hostScreenRect = hostScreenRectForBounds(this.window, this.lastBounds, nativeScreenRect);
    const maxDeltaPx = maxRectDelta(hostScreenRect, nativeScreenRect);
    return {
      hostScreenRect,
      nativeScreenRect,
      maxDeltaPx,
      aligned: maxDeltaPx <= 2
    };
  }
}

function sanitizeTargetTimeMicroseconds(value: number): number {
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

function hostScreenRectForBounds(
  window: BrowserWindow,
  bounds: RealtimePreviewSurfaceBounds,
  nativeScreenRect: RealtimePreviewScreenRect
): RealtimePreviewScreenRect {
  const contentBounds = window.getContentBounds();
  const directRect = {
    x: contentBounds.x + bounds.x,
    y: contentBounds.y + bounds.y,
    width: bounds.width,
    height: bounds.height
  };
  const display = screen.getDisplayMatching(window.getBounds());
  const flippedRect = {
    ...directRect,
    y: display.bounds.y + display.bounds.height - directRect.y - directRect.height
  };

  return maxRectDelta(flippedRect, nativeScreenRect) < maxRectDelta(directRect, nativeScreenRect)
    ? flippedRect
    : directRect;
}

function maxRectDelta(first: RealtimePreviewScreenRect, second: RealtimePreviewScreenRect): number {
  return Math.max(
    Math.abs(first.x - second.x),
    Math.abs(first.y - second.y),
    Math.abs(first.width - second.width),
    Math.abs(first.height - second.height)
  );
}

function sequenceDurationMicroseconds(draft: Draft | null): number {
  if (draft === null) {
    return 0;
  }

  return draft.tracks.reduce((duration, track) => {
    const trackEnd = track.segments.reduce((end, segment) => {
      return Math.max(end, segment.targetTimerange.start + segment.targetTimerange.duration);
    }, 0);
    return Math.max(duration, trackEnd);
  }, 0);
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

function recordRealtimePreviewHostCall(call: RealtimePreviewHostRecord): void {
  if (process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS !== "1") {
    return;
  }

  globalThis.__videoEditorTestRealtimePreviewHostCalls ??= [];
  globalThis.__videoEditorTestRealtimePreviewHostCalls.push(call);
}

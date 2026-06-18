import { BrowserWindow, ipcMain, type IpcMainInvokeEvent } from "electron";

import {
  attachRealtimePreviewSurface,
  closeRealtimePreviewSession,
  createRealtimePreviewSession,
  detachRealtimePreviewSurface,
  getRealtimePreviewTelemetry,
  requestRealtimePreviewFrame,
  updateRealtimePreviewSurfaceBounds,
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
  hostAttached: boolean;
  fallbackActive: boolean;
  statusLabel: string;
  fallbackLabel: string | null;
  playbackGeneration: number | null;
  telemetry: RealtimePreviewTelemetryResponse | null;
};

type RealtimePreviewHostRecord = {
  kind: string;
  parentHandleByteLength?: number;
  surfaceKind?: string;
  bounds?: RealtimePreviewSurfaceBounds;
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
  private lastBounds: RealtimePreviewSurfaceBounds | null = null;
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
      this.refreshTelemetry();
      return this.state("实时预览已接入");
    } catch (error) {
      this.attached = false;
      this.fallbackLabel = attachFailureLabel(error);
      recordRealtimePreviewHostCall({ kind: "attachFailure", bounds });
      return this.state("实时预览降级显示");
    }
  }

  getTelemetryState(): RealtimePreviewHostDisplayState {
    try {
      this.ensureSession();
      this.mockFirstFrameForTest();
      this.refreshTelemetry();
    } catch (error) {
      this.fallbackLabel = attachFailureLabel(error);
    }

    return this.state(this.fallbackLabel === null ? "实时预览数据已更新" : "实时预览降级显示");
  }

  close(): void {
    if (this.closed) {
      return;
    }

    this.closed = true;
    if (this.sessionId === null) {
      return;
    }

    if (this.attached) {
      try {
        detachRealtimePreviewSurface({ sessionId: this.sessionId });
        recordRealtimePreviewHostCall({ kind: "detachSurface" });
      } catch {
        this.fallbackLabel = "实时预览关闭时已降级";
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
      throw new Error("实时预览测试降级");
    }

    const parentHandle = nativeParentHandleToNumber(nativeHandle);
    const kind = nativeSurfaceKind();
    if (kind === "offscreen") {
      return {
        kind,
        ...bounds
      };
    }

    return {
      kind,
      parentHandle,
      ...bounds
    };
  }

  private refreshTelemetry(): void {
    if (this.sessionId === null) {
      return;
    }

    this.telemetry = getRealtimePreviewTelemetry({ sessionId: this.sessionId });
    recordRealtimePreviewHostCall({ kind: "getTelemetry" });
  }

  private mockFirstFrameForTest(): void {
    if (
      process.env.VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_FIRST_FRAME !== "1" ||
      this.sessionId === null ||
      this.playbackGeneration === null ||
      (this.telemetry?.presentedFrameCount ?? 0) > 0
    ) {
      return;
    }

    requestRealtimePreviewFrame({
      sessionId: this.sessionId,
      frame: {
        targetTimeMicroseconds: 0,
        playbackGeneration: this.playbackGeneration,
        queueLatencyMs: 4,
        renderDurationMs: 9
      }
    });
    recordRealtimePreviewHostCall({ kind: "requestFirstFrame" });
  }

  private state(statusLabel: string): RealtimePreviewHostDisplayState {
    return {
      ok: this.fallbackLabel === null,
      hostAttached: this.attached,
      fallbackActive: this.fallbackLabel !== null,
      statusLabel,
      fallbackLabel: this.fallbackLabel,
      playbackGeneration: this.playbackGeneration,
      telemetry: this.telemetry
    };
  }
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

function nativeSurfaceKind(): RealtimePreviewSurfaceDescriptor["kind"] {
  if (
    process.env.VIDEO_EDITOR_TEST_REALTIME_PREVIEW_SURFACE_KIND === "mock" ||
    process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS === "1"
  ) {
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

function attachFailureLabel(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return `实时预览降级：${message.slice(0, 120)}`;
}

function recordRealtimePreviewHostCall(call: RealtimePreviewHostRecord): void {
  if (process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS !== "1") {
    return;
  }

  globalThis.__videoEditorTestRealtimePreviewHostCalls ??= [];
  globalThis.__videoEditorTestRealtimePreviewHostCalls.push(call);
}

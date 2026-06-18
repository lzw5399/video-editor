import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type { CommandResultEnvelope } from "../generated/CommandResultEnvelope";

type PingResponse = { pong: boolean };
type VersionResponse = { coreVersion: string; contractVersion: string };

type NativeBinding = {
  ping: () => CommandResultEnvelope<PingResponse>;
  version: () => CommandResultEnvelope<VersionResponse>;
  executeCommand: (command: CommandEnvelope) => CommandResultEnvelope<unknown>;
  createRealtimePreviewSession: (config: RealtimePreviewSessionConfig) => RealtimePreviewSessionResponse;
  closeRealtimePreviewSession: (request: RealtimePreviewSessionRequest) => RealtimePreviewClosedResponse;
  attachRealtimePreviewSurface: (request: RealtimePreviewSurfaceRequest) => RealtimePreviewGenerationResponse;
  updateRealtimePreviewSurfaceBounds: (request: RealtimePreviewSurfaceBoundsRequest) => RealtimePreviewGenerationResponse;
  detachRealtimePreviewSurface: (request: RealtimePreviewSessionRequest) => RealtimePreviewGenerationResponse;
  requestRealtimePreviewFrame: (request: RealtimePreviewFrameRequest) => RealtimePreviewFrameResponse;
  getRealtimePreviewTelemetry: (request: RealtimePreviewSessionRequest) => RealtimePreviewTelemetryResponse;
};

export type RealtimePreviewSessionConfig = {
  sessionLabel: string;
  frameRateNumerator: number;
  frameRateDenominator: number;
  playbackRateNumerator: number;
  playbackRateDenominator: number;
};

export type RealtimePreviewSessionRequest = {
  sessionId: string;
};

export type RealtimePreviewSessionResponse = {
  sessionId: string;
  playbackGeneration: number;
};

export type RealtimePreviewClosedResponse = {
  sessionId: string;
  closed: boolean;
};

export type RealtimePreviewSurfaceDescriptor = {
  kind: "windowsHwnd" | "macosNsView" | "mock" | "offscreen";
  parentHandle?: number;
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactorMillis: number;
};

export type RealtimePreviewSurfaceRequest = {
  sessionId: string;
  surface: RealtimePreviewSurfaceDescriptor;
};

export type RealtimePreviewSurfaceBounds = {
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactorMillis: number;
};

export type RealtimePreviewSurfaceBoundsRequest = {
  sessionId: string;
  bounds: RealtimePreviewSurfaceBounds;
};

export type RealtimePreviewGenerationResponse = {
  playbackGeneration: number;
};

export type RealtimePreviewFrameRequest = {
  sessionId: string;
  frame: {
    targetTimeMicroseconds: number;
    playbackGeneration: number;
    queueLatencyMs: number;
    renderDurationMs: number;
  };
};

export type RealtimePreviewFrameResponse = {
  targetTimeMicroseconds: number;
  playbackGeneration: number;
  presented: boolean;
  staleRejected: boolean;
  canceled: boolean;
  backend: string;
};

export type RealtimePreviewTelemetryResponse = {
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
};

const requireNative = createRequire(__filename);
const MAX_LOAD_ERROR_LENGTH = 600;

let cachedBinding: NativeBinding | null | undefined;
let cachedLoadError: string | null = null;

export function ping(): CommandResultEnvelope<PingResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("ping");
  }
  return binding.ping();
}

export function version(): CommandResultEnvelope<VersionResponse> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError("version");
  }
  return binding.version();
}

export function executeCommand(command: CommandEnvelope): CommandResultEnvelope<unknown> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError(command.command);
  }
  return binding.executeCommand(command);
}

export function createRealtimePreviewSession(config: RealtimePreviewSessionConfig): RealtimePreviewSessionResponse {
  return requireLoadedBinding().createRealtimePreviewSession(config);
}

export function closeRealtimePreviewSession(request: RealtimePreviewSessionRequest): RealtimePreviewClosedResponse {
  return requireLoadedBinding().closeRealtimePreviewSession(request);
}

export function attachRealtimePreviewSurface(request: RealtimePreviewSurfaceRequest): RealtimePreviewGenerationResponse {
  return requireLoadedBinding().attachRealtimePreviewSurface(request);
}

export function updateRealtimePreviewSurfaceBounds(
  request: RealtimePreviewSurfaceBoundsRequest
): RealtimePreviewGenerationResponse {
  return requireLoadedBinding().updateRealtimePreviewSurfaceBounds(request);
}

export function detachRealtimePreviewSurface(request: RealtimePreviewSessionRequest): RealtimePreviewGenerationResponse {
  return requireLoadedBinding().detachRealtimePreviewSurface(request);
}

export function requestRealtimePreviewFrame(request: RealtimePreviewFrameRequest): RealtimePreviewFrameResponse {
  return requireLoadedBinding().requestRealtimePreviewFrame(request);
}

export function getRealtimePreviewTelemetry(request: RealtimePreviewSessionRequest): RealtimePreviewTelemetryResponse {
  return requireLoadedBinding().getRealtimePreviewTelemetry(request);
}

function loadNativeBinding(): NativeBinding | null {
  if (cachedBinding !== undefined) {
    return cachedBinding;
  }

  const bindingPath = resolveNativeBindingPath();
  try {
    const loaded = requireNative(bindingPath) as Partial<NativeBinding>;
    if (
      typeof loaded.ping !== "function" ||
      typeof loaded.version !== "function" ||
      typeof loaded.executeCommand !== "function" ||
      typeof loaded.createRealtimePreviewSession !== "function" ||
      typeof loaded.closeRealtimePreviewSession !== "function" ||
      typeof loaded.attachRealtimePreviewSurface !== "function" ||
      typeof loaded.updateRealtimePreviewSurfaceBounds !== "function" ||
      typeof loaded.detachRealtimePreviewSurface !== "function" ||
      typeof loaded.requestRealtimePreviewFrame !== "function" ||
      typeof loaded.getRealtimePreviewTelemetry !== "function"
    ) {
      throw new Error("Native binding does not expose the required editor and realtime preview functions");
    }

    cachedBinding = {
      ping: loaded.ping,
      version: loaded.version,
      executeCommand: loaded.executeCommand,
      createRealtimePreviewSession: loaded.createRealtimePreviewSession,
      closeRealtimePreviewSession: loaded.closeRealtimePreviewSession,
      attachRealtimePreviewSurface: loaded.attachRealtimePreviewSurface,
      updateRealtimePreviewSurfaceBounds: loaded.updateRealtimePreviewSurfaceBounds,
      detachRealtimePreviewSurface: loaded.detachRealtimePreviewSurface,
      requestRealtimePreviewFrame: loaded.requestRealtimePreviewFrame,
      getRealtimePreviewTelemetry: loaded.getRealtimePreviewTelemetry
    };
    cachedLoadError = null;
    return cachedBinding;
  } catch (error) {
    cachedBinding = null;
    cachedLoadError = boundErrorMessage(error);
    return null;
  }
}

function requireLoadedBinding(): NativeBinding {
  const binding = loadNativeBinding();
  if (binding === null) {
    throw new Error(`剪辑核心加载失败：${cachedLoadError ?? "unknown load failure"}`);
  }
  return binding;
}

export function resolveNativeBindingPath(): string {
  const candidates = resolveNativeBindingCandidates();
  const firstExistingCandidate = candidates.find((candidate) => existsSync(candidate));
  if (firstExistingCandidate !== undefined) {
    return firstExistingCandidate;
  }
  return candidates[0] ?? join(__dirname, "../../native/index.cjs");
}

export function resolveNativeBindingCandidates(): string[] {
  if (process.env.VE_NATIVE_BINDING_PATH !== undefined) {
    return [process.env.VE_NATIVE_BINDING_PATH];
  }

  const candidates = [
    join(__dirname, "../../native/index.cjs"),
    join(__dirname, "../native/index.cjs")
  ];

  const resourcesPath = readElectronResourcesPath();
  if (resourcesPath !== null) {
    candidates.unshift(
      join(resourcesPath, "app.asar.unpacked", "native", "index.cjs"),
      join(resourcesPath, "native", "index.cjs")
    );
  }

  return candidates;
}

function bindingLoadError(command: string): CommandResultEnvelope<never> {
  return {
    ok: false,
    data: null,
    error: {
      kind: "internal",
      command,
      message: `剪辑核心加载失败：${cachedLoadError ?? "unknown load failure"}`
    },
    events: []
  };
}

function readElectronResourcesPath(): string | null {
  const resourcesPath = (process as NodeJS.Process & { resourcesPath?: string }).resourcesPath;
  return typeof resourcesPath === "string" && resourcesPath.length > 0 ? resourcesPath : null;
}

function boundErrorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return message.length > MAX_LOAD_ERROR_LENGTH
    ? `${message.slice(0, MAX_LOAD_ERROR_LENGTH)}...`
    : message;
}

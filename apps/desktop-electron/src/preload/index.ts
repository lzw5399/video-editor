import { contextBridge, ipcRenderer, type IpcRendererEvent } from "electron";

import type {
  AudioPreviewRequest,
  ArtifactGarbageCollectionRequest,
  ArtifactGenerationActionRequest,
  ArtifactQuotaRequest,
  ArtifactStatusRequest,
  CreateProjectSessionRequest,
  ExportJobRequest,
  ExecuteProjectIntentRequest,
  OpenProjectSessionRequest,
  ProjectSessionReadRequest,
  ProjectSessionRequest,
  RequestProjectSessionPreviewFrameRequest,
  RequestProjectSessionPreviewSegmentRequest,
  StartProjectSessionExportRequest
} from "../main/nativeBinding";

type RealtimePreviewHostRect = {
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactorMillis: number;
};
type RealtimePreviewTelemetryListener = (state: unknown) => void;
type ProjectBundlePickerResponse = {
  canceled: boolean;
  bundlePath: string | null;
};

const allowedRendererUrl = readAllowedRendererUrl();
const realtimePreviewTelemetryChannel = "realtimePreviewHost:telemetryState";
const realtimePreviewTelemetryListeners = new Set<RealtimePreviewTelemetryListener>();
let realtimePreviewTelemetrySubscribed = false;
let realtimePreviewTelemetryState: unknown = null;
const realtimePreviewTelemetryListener = (_event: IpcRendererEvent, state: unknown) => {
  realtimePreviewTelemetryState = state;
  for (const listener of realtimePreviewTelemetryListeners) {
    listener(state);
  }
};

if (allowedRendererUrl !== undefined && isAllowedRendererLocation(window.location.href, allowedRendererUrl)) {
  contextBridge.exposeInMainWorld("videoEditorAppConfig", {
    workspaceFixture: readWorkspaceFixture(),
    openProjectBundlePath: readOpenProjectBundlePath(),
    showDeveloperDiagnostics: process.argv.includes("--video-editor-developer-diagnostics=1")
  });
  contextBridge.exposeInMainWorld("videoEditorCore", {
    ping: () => ipcRenderer.invoke("core:ping"),
    version: () => ipcRenderer.invoke("core:version"),
    probeMediaRuntime: () => ipcRenderer.invoke("core:probeMediaRuntime"),
    probeRuntimeCapabilities: () => ipcRenderer.invoke("core:probeRuntimeCapabilities"),
    createProjectSession: (request: CreateProjectSessionRequest) => ipcRenderer.invoke("core:createProjectSession", request),
    openProjectSession: (request: OpenProjectSessionRequest) => ipcRenderer.invoke("core:openProjectSession", request),
    executeProjectIntent: (request: ExecuteProjectIntentRequest) => ipcRenderer.invoke("core:executeProjectIntent", request),
    listProjectSessionMaterials: (request: ProjectSessionReadRequest) =>
      ipcRenderer.invoke("core:listProjectSessionMaterials", request),
    listProjectSessionMissingMaterials: (request: ProjectSessionReadRequest) =>
      ipcRenderer.invoke("core:listProjectSessionMissingMaterials", request),
    startProjectSessionExport: (request: StartProjectSessionExportRequest) =>
      ipcRenderer.invoke("core:startProjectSessionExport", request),
    getExportJobStatus: (request: ExportJobRequest) => ipcRenderer.invoke("core:getExportJobStatus", request),
    cancelExport: (request: ExportJobRequest) => ipcRenderer.invoke("core:cancelExport", request),
    createAudioPreviewSession: (request: AudioPreviewRequest) =>
      ipcRenderer.invoke("core:createAudioPreviewSession", request),
    playAudioPreview: (request: AudioPreviewRequest) => ipcRenderer.invoke("core:playAudioPreview", request),
    pauseAudioPreview: (request: AudioPreviewRequest) => ipcRenderer.invoke("core:pauseAudioPreview", request),
    stopAudioPreview: (request: AudioPreviewRequest) => ipcRenderer.invoke("core:stopAudioPreview", request),
    seekAudioPreview: (request: AudioPreviewRequest) => ipcRenderer.invoke("core:seekAudioPreview", request),
    cancelAudioPreview: (request: AudioPreviewRequest) => ipcRenderer.invoke("core:cancelAudioPreview", request),
    getAudioPreviewStatus: (request: AudioPreviewRequest) => ipcRenderer.invoke("core:getAudioPreviewStatus", request),
    listAudioOutputDevices: (request: AudioPreviewRequest) => ipcRenderer.invoke("core:listAudioOutputDevices", request),
    selectAudioOutputDevice: (request: AudioPreviewRequest) =>
      ipcRenderer.invoke("core:selectAudioOutputDevice", request),
    getWaveformDisplayPeaks: (request: AudioPreviewRequest) => ipcRenderer.invoke("core:getWaveformDisplayPeaks", request),
    refreshWaveformStatus: (request: AudioPreviewRequest) => ipcRenderer.invoke("core:refreshWaveformStatus", request),
    getArtifactStatus: (request: ArtifactStatusRequest) => ipcRenderer.invoke("core:getArtifactStatus", request),
    refreshArtifactStatus: (request: ArtifactStatusRequest) => ipcRenderer.invoke("core:refreshArtifactStatus", request),
    retryArtifactGeneration: (request: ArtifactGenerationActionRequest) =>
      ipcRenderer.invoke("core:retryArtifactGeneration", request),
    resumeArtifactGeneration: (request: ArtifactGenerationActionRequest) =>
      ipcRenderer.invoke("core:resumeArtifactGeneration", request),
    cancelArtifactGeneration: (request: ArtifactGenerationActionRequest) =>
      ipcRenderer.invoke("core:cancelArtifactGeneration", request),
    getArtifactQuotaStatus: (request: ArtifactQuotaRequest) => ipcRenderer.invoke("core:getArtifactQuotaStatus", request),
    runArtifactGarbageCollection: (request: ArtifactGarbageCollectionRequest) =>
      ipcRenderer.invoke("core:runArtifactGarbageCollection", request),
    requestProjectSessionPreviewFrame: (request: RequestProjectSessionPreviewFrameRequest) =>
      ipcRenderer.invoke("core:requestProjectSessionPreviewFrame", request),
    requestProjectSessionPreviewSegment: (request: RequestProjectSessionPreviewSegmentRequest) =>
      ipcRenderer.invoke("core:requestProjectSessionPreviewSegment", request),
    closeProjectSession: (request: ProjectSessionRequest) => ipcRenderer.invoke("core:closeProjectSession", request)
  });
  contextBridge.exposeInMainWorld("videoEditorPlatform", {
    createProjectBundle: (): Promise<ProjectBundlePickerResponse> => ipcRenderer.invoke("platform:createProjectBundle"),
    openProjectBundle: (): Promise<ProjectBundlePickerResponse> => ipcRenderer.invoke("platform:openProjectBundle"),
    openMaterialFiles: () => ipcRenderer.invoke("platform:openMaterialFiles"),
    pathToFileUrl: (path: string) => ipcRenderer.invoke("platform:pathToFileUrl", path)
  });
  contextBridge.exposeInMainWorld("videoEditorRealtimePreviewHost", {
    updateHostRect: (rect: RealtimePreviewHostRect) => ipcRenderer.invoke("realtimePreviewHost:updateRect", sanitizeHostRect(rect)),
    subscribeTelemetry: subscribeRealtimePreviewTelemetry,
    updateProjectSessionSnapshot: (projectSessionId: string, expectedRevision: number) =>
      ipcRenderer.invoke(
        "realtimePreviewHost:updateProjectSessionSnapshot",
        sanitizeProjectSessionId(projectSessionId),
        sanitizeExpectedRevision(expectedRevision)
      ),
    seek: (targetTimeMicroseconds: number) =>
      ipcRenderer.invoke("realtimePreviewHost:seek", sanitizeTargetTimeMicroseconds(targetTimeMicroseconds)),
    play: () => ipcRenderer.invoke("realtimePreviewHost:play"),
    pause: () => ipcRenderer.invoke("realtimePreviewHost:pause"),
    stop: () => ipcRenderer.invoke("realtimePreviewHost:stop")
  });
  if (process.argv.includes("--video-editor-test-observations=1")) {
    contextBridge.exposeInMainWorld("videoEditorTestObservations", {
      getNativeCommandObservations: () => ipcRenderer.invoke("test:getNativeCommandObservations"),
      getProjectSessionCalls: () => ipcRenderer.invoke("test:getProjectSessionCalls"),
      getRealtimePreviewHostCalls: () => ipcRenderer.invoke("test:getRealtimePreviewHostCalls"),
      getWindowMetrics: () => ipcRenderer.invoke("test:getWindowMetrics")
    });
  }
}

function subscribeRealtimePreviewTelemetry(listener: RealtimePreviewTelemetryListener): () => void {
  if (typeof listener !== "function") {
    throw new TypeError("realtime preview telemetry listener must be a function");
  }

  realtimePreviewTelemetryListeners.add(listener);
  if (realtimePreviewTelemetryState !== null) {
    queueMicrotask(() => listener(realtimePreviewTelemetryState));
  }
  if (!realtimePreviewTelemetrySubscribed) {
    realtimePreviewTelemetrySubscribed = true;
    ipcRenderer.on(realtimePreviewTelemetryChannel, realtimePreviewTelemetryListener);
    void ipcRenderer.invoke("realtimePreviewHost:subscribeTelemetry").then((state) => {
      realtimePreviewTelemetryState = state;
      for (const telemetryListener of realtimePreviewTelemetryListeners) {
        telemetryListener(state);
      }
    }).catch(() => undefined);
  }

  return () => {
    realtimePreviewTelemetryListeners.delete(listener);
    if (realtimePreviewTelemetryListeners.size > 0 || !realtimePreviewTelemetrySubscribed) {
      return;
    }
    realtimePreviewTelemetrySubscribed = false;
    ipcRenderer.removeListener(realtimePreviewTelemetryChannel, realtimePreviewTelemetryListener);
    void ipcRenderer.invoke("realtimePreviewHost:unsubscribeTelemetry");
  };
}

function readAllowedRendererUrl(): string | undefined {
  const prefix = "--video-editor-allowed-renderer-url=";
  return process.argv.find((argument) => argument.startsWith(prefix))?.slice(prefix.length);
}

function readWorkspaceFixture(): "demo" | "blank" | undefined {
  const prefix = "--video-editor-workspace-fixture=";
  const raw = process.argv.find((argument) => argument.startsWith(prefix))?.slice(prefix.length);
  if (raw === "demo" || raw === "blank") {
    return raw;
  }
  return undefined;
}

function readOpenProjectBundlePath(): string | undefined {
  const prefix = "--video-editor-test-open-project-bundle=";
  const raw = process.argv.find((argument) => argument.startsWith(prefix))?.slice(prefix.length);
  if (raw === undefined || raw.trim().length === 0) {
    return undefined;
  }
  return decodeURIComponent(raw);
}

function isAllowedRendererLocation(targetHref: string, allowedHref: string): boolean {
  try {
    const target = new URL(targetHref);
    const allowed = new URL(allowedHref);

    if (allowed.protocol === "file:") {
      return target.protocol === "file:" && target.host === allowed.host && target.pathname === allowed.pathname;
    }

    return target.origin === allowed.origin;
  } catch {
    return false;
  }
}

function sanitizeHostRect(rect: RealtimePreviewHostRect): RealtimePreviewHostRect {
  return {
    x: finiteRounded(rect.x),
    y: finiteRounded(rect.y),
    width: finiteRounded(rect.width),
    height: finiteRounded(rect.height),
    scaleFactorMillis: finiteRounded(rect.scaleFactorMillis)
  };
}

function finiteRounded(value: number): number {
  return Number.isFinite(value) ? Math.round(value) : 0;
}

function sanitizeTargetTimeMicroseconds(value: number): number {
  return Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0;
}

function sanitizeExpectedRevision(value: number): number {
  return Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0;
}

function sanitizeProjectSessionId(value: string): string {
  return typeof value === "string" ? value : "";
}

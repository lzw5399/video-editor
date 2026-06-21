import { contextBridge, ipcRenderer } from "electron";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type { Draft } from "../generated/Draft";
import type {
  CreateProjectSessionRequest,
  ExecuteProjectIntentRequest,
  OpenProjectSessionRequest,
  ProjectSessionReadRequest,
  ProjectSessionRequest
} from "../main/nativeBinding";

type RealtimePreviewHostRect = {
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactorMillis: number;
};
type ProjectBundlePickerResponse = {
  canceled: boolean;
  bundlePath: string | null;
};

const allowedRendererUrl = readAllowedRendererUrl();

if (allowedRendererUrl !== undefined && isAllowedRendererLocation(window.location.href, allowedRendererUrl)) {
  contextBridge.exposeInMainWorld("videoEditorAppConfig", {
    workspaceFixture: readWorkspaceFixture(),
    openProjectBundlePath: readOpenProjectBundlePath(),
    showDeveloperDiagnostics: process.argv.includes("--video-editor-developer-diagnostics=1")
  });
  contextBridge.exposeInMainWorld("videoEditorCore", {
    ping: () => ipcRenderer.invoke("core:ping"),
    version: () => ipcRenderer.invoke("core:version"),
    executeCommand: (command: CommandEnvelope) => ipcRenderer.invoke("core:executeCommand", command),
    createProjectSession: (request: CreateProjectSessionRequest) => ipcRenderer.invoke("core:createProjectSession", request),
    openProjectSession: (request: OpenProjectSessionRequest) => ipcRenderer.invoke("core:openProjectSession", request),
    executeProjectIntent: (request: ExecuteProjectIntentRequest) => ipcRenderer.invoke("core:executeProjectIntent", request),
    listProjectSessionMaterials: (request: ProjectSessionReadRequest) =>
      ipcRenderer.invoke("core:listProjectSessionMaterials", request),
    listProjectSessionMissingMaterials: (request: ProjectSessionReadRequest) =>
      ipcRenderer.invoke("core:listProjectSessionMissingMaterials", request),
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
    getTelemetry: () => ipcRenderer.invoke("realtimePreviewHost:getTelemetry"),
    updateDraftSnapshot: (draft: Draft, bundlePath?: string) =>
      ipcRenderer.invoke("realtimePreviewHost:updateDraftSnapshot", draft, bundlePath),
    seek: (targetTimeMicroseconds: number) =>
      ipcRenderer.invoke("realtimePreviewHost:seek", sanitizeTargetTimeMicroseconds(targetTimeMicroseconds)),
    play: () => ipcRenderer.invoke("realtimePreviewHost:play"),
    pause: () => ipcRenderer.invoke("realtimePreviewHost:pause"),
    stop: () => ipcRenderer.invoke("realtimePreviewHost:stop")
  });
  if (process.argv.includes("--video-editor-test-observations=1")) {
    contextBridge.exposeInMainWorld("videoEditorTestObservations", {
      getExecuteCommandCalls: () => ipcRenderer.invoke("test:getExecuteCommandCalls"),
      getProjectSessionCalls: () => ipcRenderer.invoke("test:getProjectSessionCalls"),
      getRealtimePreviewHostCalls: () => ipcRenderer.invoke("test:getRealtimePreviewHostCalls"),
      getWindowMetrics: () => ipcRenderer.invoke("test:getWindowMetrics")
    });
  }
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

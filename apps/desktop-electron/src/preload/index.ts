import { contextBridge, ipcRenderer } from "electron";

import type { CommandEnvelope } from "../generated/CommandEnvelope";

type RealtimePreviewHostRect = {
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactorMillis: number;
};

const allowedRendererUrl = readAllowedRendererUrl();

if (allowedRendererUrl !== undefined && isAllowedRendererLocation(window.location.href, allowedRendererUrl)) {
  contextBridge.exposeInMainWorld("videoEditorAppConfig", {
    workspaceFixture: readWorkspaceFixture(),
    showDeveloperDiagnostics: process.argv.includes("--video-editor-developer-diagnostics=1")
  });
  contextBridge.exposeInMainWorld("videoEditorCore", {
    ping: () => ipcRenderer.invoke("core:ping"),
    version: () => ipcRenderer.invoke("core:version"),
    executeCommand: (command: CommandEnvelope) => ipcRenderer.invoke("core:executeCommand", command)
  });
  contextBridge.exposeInMainWorld("videoEditorPlatform", {
    openMaterialFiles: () => ipcRenderer.invoke("platform:openMaterialFiles"),
    pathToFileUrl: (path: string) => ipcRenderer.invoke("platform:pathToFileUrl", path)
  });
  contextBridge.exposeInMainWorld("videoEditorRealtimePreviewHost", {
    updateHostRect: (rect: RealtimePreviewHostRect) => ipcRenderer.invoke("realtimePreviewHost:updateRect", sanitizeHostRect(rect)),
    getTelemetry: () => ipcRenderer.invoke("realtimePreviewHost:getTelemetry")
  });
}

function readAllowedRendererUrl(): string | undefined {
  const prefix = "--video-editor-allowed-renderer-url=";
  return process.argv.find((argument) => argument.startsWith(prefix))?.slice(prefix.length);
}

function readWorkspaceFixture(): "demo" | "blank" {
  return process.argv.includes("--video-editor-workspace-fixture=demo") ? "demo" : "blank";
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

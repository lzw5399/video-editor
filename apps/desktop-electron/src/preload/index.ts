import { contextBridge, ipcRenderer } from "electron";

import type { CommandEnvelope } from "../generated/CommandEnvelope";

const allowedRendererUrl = readAllowedRendererUrl();

if (allowedRendererUrl !== undefined && isAllowedRendererLocation(window.location.href, allowedRendererUrl)) {
  contextBridge.exposeInMainWorld("videoEditorCore", {
    ping: () => ipcRenderer.invoke("core:ping"),
    version: () => ipcRenderer.invoke("core:version"),
    executeCommand: (command: CommandEnvelope) => ipcRenderer.invoke("core:executeCommand", command)
  });
}

function readAllowedRendererUrl(): string | undefined {
  const prefix = "--video-editor-allowed-renderer-url=";
  return process.argv.find((argument) => argument.startsWith(prefix))?.slice(prefix.length);
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

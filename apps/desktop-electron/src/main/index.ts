import { app, BrowserWindow, ipcMain, type IpcMainInvokeEvent } from "electron";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import { executeCommand, ping, version } from "./nativeBinding";

const devServerUrl = process.env.VITE_DEV_SERVER_URL;
const isDevelopment = !app.isPackaged && isLoopbackUrl(devServerUrl);
const packagedRendererFile = join(__dirname, "../renderer/index.html");
const packagedRendererUrl = pathToFileURL(packagedRendererFile).toString();
const allowedRendererUrl = isDevelopment && devServerUrl !== undefined ? devServerUrl : packagedRendererUrl;
const allowedRendererUrlArgument = `--video-editor-allowed-renderer-url=${allowedRendererUrl}`;

ipcMain.handle("core:ping", (event) => {
  assertAllowedIpcSender(event);
  return ping();
});
ipcMain.handle("core:version", (event) => {
  assertAllowedIpcSender(event);
  return version();
});
ipcMain.handle("core:executeCommand", (event, command: CommandEnvelope) => {
  assertAllowedIpcSender(event);
  return executeCommand(command);
});

async function createWindow(): Promise<void> {
  const window = new BrowserWindow({
    width: 1280,
    height: 800,
    minWidth: 960,
    minHeight: 640,
    backgroundColor: "#171717",
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      preload: join(__dirname, "../preload/index.cjs"),
      additionalArguments: [allowedRendererUrlArgument]
    }
  });

  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  window.webContents.on("will-navigate", (event, targetUrl) => {
    if (!isAllowedRendererUrl(targetUrl)) {
      event.preventDefault();
    }
  });

  if (isDevelopment) {
    await window.loadURL(devServerUrl as string);
    return;
  }

  await window.loadFile(packagedRendererFile);
}

app.whenReady().then(createWindow);

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    void createWindow();
  }
});

function isLoopbackUrl(value: string | undefined): value is string {
  if (value === undefined) {
    return false;
  }

  try {
    const url = new URL(value);
    return (
      (url.protocol === "http:" || url.protocol === "https:") &&
      (url.hostname === "localhost" || url.hostname === "127.0.0.1" || url.hostname === "::1")
    );
  } catch {
    return false;
  }
}

function assertAllowedIpcSender(event: IpcMainInvokeEvent): void {
  const senderUrl = event.senderFrame.url;
  if (!isAllowedRendererUrl(senderUrl)) {
    throw new Error(`Rejected IPC from untrusted renderer: ${senderUrl}`);
  }
}

function isAllowedRendererUrl(targetUrl: string): boolean {
  try {
    const target = new URL(targetUrl);
    const allowed = new URL(allowedRendererUrl);

    if (isDevelopment && devServerUrl !== undefined) {
      return target.origin === allowed.origin;
    }

    return target.protocol === "file:" && target.host === allowed.host && target.pathname === allowed.pathname;
  } catch {
    return false;
  }
}

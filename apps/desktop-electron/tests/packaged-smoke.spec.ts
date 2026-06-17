import { expect, test } from "@playwright/test";

import type { CommandEnvelope } from "../src/generated/CommandEnvelope";
import type { CommandResultEnvelope } from "../src/generated/CommandResultEnvelope";
import { launchPackagedApp } from "./helpers/packagedApp";

type VideoEditorCoreApi = {
  ping: () => Promise<CommandResultEnvelope<{ pong: boolean }>>;
  version: () => Promise<CommandResultEnvelope<{ coreVersion: string; contractVersion: string }>>;
  executeCommand: (command: CommandEnvelope) => Promise<CommandResultEnvelope<unknown>>;
};

declare global {
  interface Window {
    videoEditorCore?: VideoEditorCoreApi;
    ipcRenderer?: unknown;
  }
}

function probeMediaRuntimeCommand(requestId: string): CommandEnvelope {
  return {
    command: "probeMediaRuntime",
    payload: { kind: "probeMediaRuntime" },
    requestId
  };
}

test("packaged app loads file renderer, preload bridge, native binding, and runtime probe", async () => {
  const { app, page, executablePath } = await launchPackagedApp();

  try {
    expect(executablePath).not.toContain(["dist", "main", "index.cjs"].join("/"));
    await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();

    const location = await page.evaluate(() => window.location.href);
    expect(location).toMatch(/^file:/);

    const bridgeShape = await page.evaluate(() => ({
      coreType: typeof window.videoEditorCore,
      ipcRendererType: typeof window.ipcRenderer,
      keys: Object.keys(window.videoEditorCore ?? {})
    }));
    expect(bridgeShape).toEqual({
      coreType: "object",
      ipcRendererType: "undefined",
      keys: ["ping", "version", "executeCommand"]
    });

    const ping = await page.evaluate(() => window.videoEditorCore?.ping());
    expect(ping).toEqual({
      ok: true,
      data: { pong: true },
      error: null,
      events: []
    });

    const version = await page.evaluate(() => window.videoEditorCore?.version());
    expect(version?.ok).toBe(true);
    expect(version?.data?.coreVersion).toMatch(/^\d+\.\d+\.\d+/);
    expect(version?.data?.contractVersion).toMatch(/^\d+\.\d+\.\d+/);
    expect(version?.error).toBeNull();

    const runtime = await page.evaluate((command) => {
      return window.videoEditorCore?.executeCommand(command);
    }, probeMediaRuntimeCommand("packaged-runtime-probe"));
    expect(runtime?.ok).toBe(true);
    expect(runtime?.error).toBeNull();
  } finally {
    await app.close();
  }
});

test("packaged app reports classified runtime discovery failures without crashing", async () => {
  const { app, page } = await launchPackagedApp({
    VE_FFMPEG_PATH: "/definitely-missing/video-editor/ffmpeg",
    VE_FFPROBE_PATH: "/definitely-missing/video-editor/ffprobe"
  });

  try {
    await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();

    const result = await page.evaluate((command) => {
      return window.videoEditorCore?.executeCommand(command);
    }, probeMediaRuntimeCommand("packaged-runtime-probe-missing"));

    expect(result?.ok).toBe(false);
    expect(result?.data).toBeNull();
    expect(result?.error?.kind).toBe("runtimeDiscoveryFailed");
    expect(result?.error?.command).toBe("probeMediaRuntime");
    expect(result?.error?.message).toMatch(/VE_FFMPEG_PATH|VE_FFPROBE_PATH/);
  } finally {
    await app.close();
  }
});

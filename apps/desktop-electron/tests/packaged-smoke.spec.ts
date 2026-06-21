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

function probeRuntimeCapabilitiesCommand(requestId: string): CommandEnvelope {
  return {
    command: "probeRuntimeCapabilities",
    payload: { kind: "probeRuntimeCapabilities" },
    requestId
  };
}

test("packaged app loads file renderer, preload bridge, native binding, and runtime probe", async () => {
  const { app, page, executablePath } = await launchPackagedApp();

  try {
    expect(executablePath).not.toContain(["dist", "main", "index.cjs"].join("/"));
    await expect(page.getByRole("main", { name: "项目入口" })).toBeVisible();

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
      keys: [
        "ping",
        "version",
        "executeCommand",
        "createProjectSession",
        "openProjectSession",
        "executeProjectIntent",
        "listProjectSessionMaterials",
        "listProjectSessionMissingMaterials",
        "closeProjectSession"
      ]
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
    const resourcesPath = await app.evaluate(() => process.resourcesPath);
    const ffmpeg = runtime?.data as {
      ffmpeg?: { path?: string; source?: { kind?: string; directory?: string } };
      ffprobe?: { path?: string; source?: { kind?: string; directory?: string } };
    } | undefined;
    expect(ffmpeg?.ffmpeg?.source?.kind).toBe("bundled");
    expect(ffmpeg?.ffprobe?.source?.kind).toBe("bundled");
    expect(ffmpeg?.ffmpeg?.source?.directory).toContain(resourcesPath);
    expect(ffmpeg?.ffprobe?.source?.directory).toContain(resourcesPath);
    expect(ffmpeg?.ffmpeg?.path).toContain(resourcesPath);
    expect(ffmpeg?.ffprobe?.path).toContain(resourcesPath);
    expect(ffmpeg?.ffmpeg?.path).not.toContain("/opt/homebrew");
    expect(ffmpeg?.ffprobe?.path).not.toContain("/opt/homebrew");

    const capabilities = await page.evaluate((command) => {
      return window.videoEditorCore?.executeCommand(command);
    }, probeRuntimeCapabilitiesCommand("packaged-runtime-capabilities"));
    const report = capabilities?.data as {
      ffmpeg?: { source?: string };
      ffprobe?: { source?: string };
      licensePosture?: { externalRuntime?: boolean; source?: string; redistributableBuild?: boolean };
    } | undefined;
    expect(capabilities?.ok).toBe(true);
    expect(report?.ffmpeg?.source).toBe("bundled");
    expect(report?.ffprobe?.source).toBe("bundled");
    expect(report?.licensePosture?.externalRuntime).toBe(false);
    expect(report?.licensePosture?.source).toBe("bundledRuntime");
    expect(report?.licensePosture?.redistributableBuild).toBe(false);
  } finally {
    await app.close();
  }
});

test("packaged app ignores external bundled runtime overrides", async () => {
  const { app, page } = await launchPackagedApp({
    VE_BUNDLED_FFMPEG_DIR: "/definitely-missing/video-editor/ffmpeg-runtime"
  });

  try {
    await expect(page.getByRole("main", { name: "项目入口" })).toBeVisible();

    const resourcesPath = await app.evaluate(() => process.resourcesPath);
    const result = await page.evaluate((command) => {
      return window.videoEditorCore?.executeCommand(command);
    }, probeMediaRuntimeCommand("packaged-runtime-probe-ignores-external-env"));

    const runtime = result?.data as {
      ffmpeg?: { path?: string; source?: { directory?: string } };
      ffprobe?: { path?: string; source?: { directory?: string } };
    } | undefined;
    expect(result?.ok).toBe(true);
    expect(result?.error).toBeNull();
    expect(runtime?.ffmpeg?.source?.directory).toContain(resourcesPath);
    expect(runtime?.ffprobe?.source?.directory).toContain(resourcesPath);
    expect(runtime?.ffmpeg?.path).not.toContain("/definitely-missing");
    expect(runtime?.ffprobe?.path).not.toContain("/definitely-missing");
  } finally {
    await app.close();
  }
});

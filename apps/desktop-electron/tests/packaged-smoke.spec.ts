import { expect, test } from "@playwright/test";

import type { CommandResultEnvelope, RuntimeCapabilityReport } from "../src/generated/CommandResultEnvelope";
import type { RuntimeConfigResponse } from "../src/main/nativeBinding";
import { launchPackagedApp } from "./helpers/packagedApp";

type VideoEditorCoreApi = {
  ping: () => Promise<CommandResultEnvelope<{ pong: boolean }>>;
  version: () => Promise<CommandResultEnvelope<{ coreVersion: string; contractVersion: string }>>;
  probeMediaRuntime: () => Promise<CommandResultEnvelope<RuntimeConfigResponse>>;
  probeRuntimeCapabilities: () => Promise<CommandResultEnvelope<RuntimeCapabilityReport>>;
  startProjectSessionExport?: (request: unknown) => Promise<CommandResultEnvelope<unknown>>;
};

declare global {
  interface Window {
    videoEditorCore?: VideoEditorCoreApi;
    ipcRenderer?: unknown;
  }
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
      hasExecuteCommand: Object.prototype.hasOwnProperty.call(window.videoEditorCore ?? {}, "executeCommand"),
      keys: Object.keys(window.videoEditorCore ?? {})
    }));
    expect(bridgeShape).toEqual({
      coreType: "object",
      ipcRendererType: "undefined",
      hasExecuteCommand: false,
      keys: expect.arrayContaining([
        "ping",
        "version",
        "probeMediaRuntime",
        "probeRuntimeCapabilities",
        "createProjectSession",
        "openProjectSession",
        "executeProjectIntent",
        "listProjectSessionMaterials",
        "listProjectSessionMissingMaterials",
        "startProjectSessionExport",
        "requestProjectSessionPreviewFrame",
        "requestProjectSessionPreviewSegment",
        "closeProjectSession"
      ])
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

    const runtime = await page.evaluate(() => window.videoEditorCore?.probeMediaRuntime());
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

    const capabilities = await page.evaluate(() => window.videoEditorCore?.probeRuntimeCapabilities());
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

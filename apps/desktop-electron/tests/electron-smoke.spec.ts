import { _electron as electron, expect, test, type ElectronApplication, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { join } from "node:path";

import type { CommandEnvelope } from "../src/generated/CommandEnvelope";
import type { CommandResultEnvelope, RuntimeCapabilityReport } from "../src/generated/CommandResultEnvelope";

type VideoEditorCoreApi = {
  ping: () => Promise<CommandResultEnvelope<{ pong: boolean }>>;
  version: () => Promise<CommandResultEnvelope<{ coreVersion: string; contractVersion: string }>>;
  probeRuntimeCapabilities: () => Promise<CommandResultEnvelope<RuntimeCapabilityReport>>;
  executeCommand: (command: CommandEnvelope) => Promise<CommandResultEnvelope<unknown>>;
};
type VideoEditorPlatformApi = {
  createProjectBundle: () => Promise<{ canceled: boolean; bundlePath: string | null }>;
  openProjectBundle: () => Promise<{ canceled: boolean; bundlePath: string | null }>;
  openMaterialFiles: () => Promise<{ canceled: boolean; filePaths: string[] }>;
  pathToFileUrl: (path: string) => Promise<string>;
};

declare global {
  interface Window {
    videoEditorAppConfig?: unknown;
    videoEditorCore?: VideoEditorCoreApi;
    videoEditorPlatform?: VideoEditorPlatformApi;
    ipcRenderer?: unknown;
  }
}

async function launchSmokeApp(): Promise<{ app: ElectronApplication; page: Page }> {
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")]
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  return { app, page };
}

async function expectVisibleWorkspaceRegions(page: Page): Promise<void> {
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  await expect(page.locator('[aria-label="顶部功能区"]').first()).toBeVisible();
  await expect(page.getByRole("navigation", { name: "顶部功能区" })).toBeVisible();
  await expect(page.locator('[aria-label="素材面板"]')).toBeVisible();
  await expect(page.locator('[aria-label="预览窗口"]')).toBeVisible();
  await expect(page.locator('[aria-label="属性检查器"]')).toBeVisible();
  await expect(page.locator('[aria-label="时间线"]')).toBeVisible();
}

async function expectProjectEntry(page: Page): Promise<void> {
  await expect(page.getByRole("main", { name: "项目入口" })).toBeVisible();
  await expect(page.getByRole("button", { name: "新建项目" })).toBeVisible();
  await expect(page.getByRole("button", { name: "打开项目" })).toBeVisible();
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toHaveCount(0);
}

async function launchSmokeAppWithEnv(
  env: NodeJS.ProcessEnv
): Promise<{ app: ElectronApplication; page: Page }> {
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")],
    env: {
      ...process.env,
      ...env
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  return { app, page };
}

async function startUntrustedHttpPage(): Promise<{ origin: string; url: string; close: () => Promise<void> }> {
  const server = createServer((_request, response) => {
    response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    response.end('<!doctype html><html><body><main aria-label="Untrusted page">Untrusted</main></body></html>');
  });

  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });

  const address = server.address();
  if (address === null || typeof address === "string") {
    throw new Error("Expected an ephemeral TCP port for the untrusted test page");
  }

  const origin = `http://127.0.0.1:${address.port}`;
  return {
    origin,
    url: `${origin}/untrusted`,
    close: () =>
      new Promise<void>((resolve, reject) => {
        server.close((error) => {
          if (error !== undefined) {
            reject(error);
            return;
          }
          resolve();
        });
      })
  };
}

test("renderer reaches Rust binding only through the typed preload bridge", async () => {
  const { app, page } = await launchSmokeApp();

  try {
    await expectProjectEntry(page);
    await expect(page.getByText("预览命令已接入")).toHaveCount(0);
    await expect(page.getByLabel("预览产物")).toHaveCount(0);
    await expect(page.getByLabel("运行环境诊断")).toHaveCount(0);

    const exposedKeys = await page.evaluate(() => Object.keys(window));
    expect(exposedKeys).toContain("videoEditorCore");
    expect(exposedKeys).toContain("videoEditorPlatform");
    expect(exposedKeys).not.toContain("ipcRenderer");

    const apiShape = await page.evaluate(() => ({
      ping: typeof window.videoEditorCore?.ping,
      version: typeof window.videoEditorCore?.version,
      probeRuntimeCapabilities: typeof window.videoEditorCore?.probeRuntimeCapabilities,
      executeCommand: typeof window.videoEditorCore?.executeCommand,
      keys: Object.keys(window.videoEditorCore ?? {}),
      platformKeys: Object.keys(window.videoEditorPlatform ?? {}),
      openMaterialFiles: typeof window.videoEditorPlatform?.openMaterialFiles,
      pathToFileUrl: typeof window.videoEditorPlatform?.pathToFileUrl
    }));
    expect(apiShape).toEqual({
      ping: "function",
      version: "function",
      probeRuntimeCapabilities: "function",
      executeCommand: "function",
      keys: [
        "ping",
        "version",
        "probeRuntimeCapabilities",
        "executeCommand",
        "createProjectSession",
        "openProjectSession",
        "executeProjectIntent",
        "listProjectSessionMaterials",
        "listProjectSessionMissingMaterials",
        "startProjectSessionExport",
        "closeProjectSession"
      ],
      platformKeys: ["createProjectBundle", "openProjectBundle", "openMaterialFiles", "pathToFileUrl"],
      openMaterialFiles: "function",
      pathToFileUrl: "function"
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
    expect(version?.events).toEqual([]);

    const command: CommandEnvelope = {
      command: "ping",
      payload: { kind: "ping" },
      requestId: "electron-smoke-ping"
    };
    const result = await page.evaluate((commandEnvelope) => {
      return window.videoEditorCore?.executeCommand(commandEnvelope);
    }, command);
    expect(result).toEqual({
      ok: true,
      data: { pong: true },
      error: null,
      events: []
    });

    await expect(page.getByRole("button", { name: "导入素材" })).toHaveCount(0);
    await expect(page.getByLabel("草稿包路径")).toHaveCount(0);
    await expect(page.getByLabel("素材路径")).toHaveCount(0);
    await expect(page.getByRole("article", { name: "素材 城市街景.mp4" })).toHaveCount(0);
    await expect(page.getByRole("article", { name: "素材 背景音乐.wav" })).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("test fixture opt-in loads demo workspace materials", async () => {
  const { app, page } = await launchSmokeAppWithEnv({
    VIDEO_EDITOR_TEST_WORKSPACE_FIXTURE: "demo",
    VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "1"
  });

  try {
    await expectVisibleWorkspaceRegions(page);
    await expect(page.getByLabel("草稿包路径")).toHaveValue("/tmp/phase-04-demo.veproj");
    await expect(page.getByLabel("素材路径")).toHaveValue("/tmp/demo-material.mp4");
    await expect(page.getByRole("article", { name: "素材 城市街景.mp4" })).toContainText("视频");
    await expect(page.getByRole("article", { name: "素材 城市街景.mp4" })).toContainText("可用");
    await expect(page.getByRole("article", { name: "素材 背景音乐.wav" })).toContainText("音频");
    await expect(page.getByRole("article", { name: "素材 封面图.png" })).toContainText("素材丢失");
    await expect(page.getByRole("article", { name: "素材 贴纸素材.webp" })).toContainText("解析失败");
  } finally {
    await app.close();
  }
});

test("renderer source does not construct FFmpeg or ffprobe commands", async () => {
  const source = await readFile(join(process.cwd(), "src/renderer/App.tsx"), "utf8");

  expect(source).not.toMatch(/ffmpeg|ffprobe/i);
});

test("main process ignores non-loopback dev server URLs", async () => {
  const { app, page } = await launchSmokeAppWithEnv({
    VITE_DEV_SERVER_URL: "https://example.com"
  });

  try {
    await expectProjectEntry(page);
    const location = await page.evaluate(() => window.location.href);
    expect(location).not.toContain("example.com");
  } finally {
    await app.close();
  }
});

test("untrusted navigation cannot access the native preload bridge", async () => {
  const untrustedPage = await startUntrustedHttpPage();
  const { app, page } = await launchSmokeApp();

  try {
    await expectProjectEntry(page);
    const initialLocation = await page.evaluate(() => window.location.href);

    await page.goto(untrustedPage.url).catch(() => undefined);

    const location = await page.evaluate(() => window.location.href);

    if (location === initialLocation) {
      await expectProjectEntry(page);
      expect(location).not.toContain(untrustedPage.origin);
      return;
    }

    expect(location).toContain(untrustedPage.origin);
    await expect(page.getByRole("main", { name: "Untrusted page" })).toBeVisible();
    const exposure = await page.evaluate(() => ({
      configType: typeof window.videoEditorAppConfig,
      coreType: typeof window.videoEditorCore,
      coreKeys: Object.keys(window.videoEditorCore ?? {}),
      platformType: typeof window.videoEditorPlatform,
      platformKeys: Object.keys(window.videoEditorPlatform ?? {}),
      ipcRendererType: typeof window.ipcRenderer
    }));
    expect(exposure).toEqual({
      configType: "undefined",
      coreType: "undefined",
      coreKeys: [],
      platformType: "undefined",
      platformKeys: [],
      ipcRendererType: "undefined"
    });
  } finally {
    await app.close();
    await untrustedPage.close();
  }
});

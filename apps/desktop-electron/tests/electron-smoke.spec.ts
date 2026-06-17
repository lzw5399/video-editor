import { _electron as electron, expect, test, type ElectronApplication, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { join } from "node:path";

import type { CommandEnvelope } from "../src/generated/CommandEnvelope";
import type { CommandResultEnvelope } from "../src/generated/CommandResultEnvelope";

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
    await expectVisibleWorkspaceRegions(page);
    await expect(page.getByText("预览命令已接入")).toBeVisible();
    await expect(page.getByText("等待请求预览帧").first()).toBeVisible();
    await expect(page.getByText("未选择片段")).toBeVisible();

    const exposedKeys = await page.evaluate(() => Object.keys(window));
    expect(exposedKeys).toContain("videoEditorCore");
    expect(exposedKeys).not.toContain("ipcRenderer");

    const apiShape = await page.evaluate(() => ({
      ping: typeof window.videoEditorCore?.ping,
      version: typeof window.videoEditorCore?.version,
      executeCommand: typeof window.videoEditorCore?.executeCommand,
      keys: Object.keys(window.videoEditorCore ?? {})
    }));
    expect(apiShape).toEqual({
      ping: "function",
      version: "function",
      executeCommand: "function",
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

    const topFeatureNav = page.getByRole("navigation", { name: "顶部功能区" });
    for (const category of ["媒体", "音频", "文字", "贴纸", "特效", "转场", "字幕", "滤镜", "调节", "模板", "数字人"]) {
      await expect(topFeatureNav.getByRole("button", { name: category })).toBeVisible();
    }

    await expect(page.getByRole("article", { name: "素材 城市街景.mp4" })).toContainText("视频");
    await expect(page.getByRole("article", { name: "素材 城市街景.mp4" })).toContainText("可用");
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
    await expectVisibleWorkspaceRegions(page);
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
    await expectVisibleWorkspaceRegions(page);
    const initialLocation = await page.evaluate(() => window.location.href);

    await page.goto(untrustedPage.url).catch(() => undefined);

    const location = await page.evaluate(() => window.location.href);

    if (location === initialLocation) {
      await expectVisibleWorkspaceRegions(page);
      expect(location).not.toContain(untrustedPage.origin);
      return;
    }

    expect(location).toContain(untrustedPage.origin);
    await expect(page.getByRole("main", { name: "Untrusted page" })).toBeVisible();
    const exposure = await page.evaluate(() => ({
      coreType: typeof window.videoEditorCore,
      coreKeys: Object.keys(window.videoEditorCore ?? {}),
      ipcRendererType: typeof window.ipcRenderer
    }));
    expect(exposure).toEqual({
      coreType: "undefined",
      coreKeys: [],
      ipcRendererType: "undefined"
    });
  } finally {
    await app.close();
    await untrustedPage.close();
  }
});

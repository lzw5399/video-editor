import { _electron as electron, expect, test, type ElectronApplication, type Page } from "@playwright/test";
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

test("renderer reaches Rust binding only through the typed preload bridge", async () => {
  const { app, page } = await launchSmokeApp();

  try {
    await expect(page.getByRole("main", { name: "Video editor smoke workbench" })).toBeVisible();

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
  } finally {
    await app.close();
  }
});

test("main process ignores non-loopback dev server URLs", async () => {
  const { app, page } = await launchSmokeAppWithEnv({
    VITE_DEV_SERVER_URL: "https://example.com"
  });

  try {
    await expect(page.getByRole("main", { name: "Video editor smoke workbench" })).toBeVisible();
    const location = await page.evaluate(() => window.location.href);
    expect(location).not.toContain("example.com");
  } finally {
    await app.close();
  }
});

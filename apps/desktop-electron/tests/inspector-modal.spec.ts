import { _electron as electron, expect, test, type ElectronApplication, type Page } from "@playwright/test";
import { join } from "node:path";

import type { CommandName } from "../src/generated/CommandEnvelope";

type ExecuteCommandCall = {
  command: CommandName;
  canvasConfig: {
    width: number;
    height: number;
    frameRate: { numerator: number; denominator: number };
  } | null;
};

declare global {
  interface Window {
    videoEditorCore?: {
      executeCommand: (command: unknown) => Promise<unknown>;
    };
  }
}

async function launchWorkspaceApp(): Promise<{ app: ElectronApplication; page: Page }> {
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")],
    env: {
      ...process.env,
      VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
      VIDEO_EDITOR_TEST_WORKSPACE_FIXTURE: "demo",
      VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_ARTIFACT_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_AUDIO_COMMANDS: "1",
      VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "0"
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  return { app, page };
}

async function spyExecuteCommandCalls(app: ElectronApplication, page: Page): Promise<void> {
  const hasBridge = await page.evaluate(() => typeof window.videoEditorCore?.executeCommand === "function");
  if (!hasBridge) {
    throw new Error("inspector modal test setup error: native videoEditorCore.executeCommand is unavailable");
  }

  await app.evaluate(() => {
    (globalThis as typeof globalThis & { __videoEditorTestExecuteCommandCalls?: ExecuteCommandCall[] })
      .__videoEditorTestExecuteCommandCalls = [];
  });
}

async function readExecuteCommandCalls(app: ElectronApplication): Promise<ExecuteCommandCall[]> {
  return app.evaluate(() => {
    return (
      (globalThis as typeof globalThis & { __videoEditorTestExecuteCommandCalls?: ExecuteCommandCall[] })
        .__videoEditorTestExecuteCommandCalls ?? []
    );
  });
}

async function openDraftParametersDialog(page: Page) {
  await page.getByLabel("草稿参数").getByRole("button", { name: "修改" }).click();
  const dialog = page.getByRole("dialog", { name: "草稿参数" });
  await expect(dialog).toBeVisible();
  return dialog;
}

test.describe("draft parameter inspector modal", () => {
  test("cancel does not mutate draft and apply records updateDraftCanvasConfig", async () => {
    const { app, page } = await launchWorkspaceApp();

    try {
      await spyExecuteCommandCalls(app, page);

      const inspector = page.getByLabel("草稿参数");
      await expect(inspector).toContainText("草稿参数");
      await expect(inspector.getByRole("button", { name: "修改" })).toBeVisible();
      await expect(inspector).toContainText("16:9");

      let dialog = await openDraftParametersDialog(page);
      await dialog.getByRole("group", { name: "画布比例" }).getByRole("button", { name: "9:16" }).click();
      await expect(dialog.getByLabel("画布宽度")).toHaveValue("1080");
      await expect(dialog.getByLabel("画布高度")).toHaveValue("1920");
      await dialog.getByRole("button", { name: "取消" }).click();
      await expect(page.getByRole("dialog", { name: "草稿参数" })).toHaveCount(0);
      await expect(inspector).toContainText("16:9");
      expect((await readExecuteCommandCalls(app)).some((call) => call.command === "updateDraftCanvasConfig")).toBe(false);

      dialog = await openDraftParametersDialog(page);
      await dialog.getByRole("group", { name: "画布比例" }).getByRole("button", { name: "9:16" }).click();
      await dialog.getByRole("group", { name: "画布背景" }).getByRole("button", { name: "模糊填充" }).click();
      await expect(dialog.getByRole("button", { name: "应用草稿参数" })).toBeEnabled();
      await dialog.getByRole("button", { name: "应用草稿参数" }).click();
      await expect(page.getByRole("dialog", { name: "草稿参数" })).toHaveCount(0);

      await expect
        .poll(async () => (await readExecuteCommandCalls(app)).some((call) => call.command === "updateDraftCanvasConfig"))
        .toBe(true);
      await expect(page.getByLabel("预览窗口")).toContainText("画布 9:16 · 1080 x 1920 · 30 fps");

      const canvasCall = (await readExecuteCommandCalls(app)).find((call) => call.command === "updateDraftCanvasConfig");
      expect(canvasCall?.canvasConfig).toMatchObject({
        width: 1080,
        height: 1920,
        frameRate: { numerator: 30, denominator: 1 }
      });
    } finally {
      await app.close();
    }
  });
});

import { _electron as electron, expect, test, type ElectronApplication, type Page } from "@playwright/test";
import { join } from "node:path";

import type { CommandName } from "../src/generated/CommandEnvelope";

type NativeCommandObservation = {
  command: CommandName | string;
  canvasConfig: {
    width: number;
    height: number;
    frameRate: { numerator: number; denominator: number };
  } | null;
};

type ProjectSessionCall = {
  command: "executeProjectIntent" | string;
  intentKind?: string | null;
  canvasConfig?: NativeCommandObservation["canvasConfig"];
};

declare global {
  interface Window {
    videoEditorTestObservations?: {
      getNativeCommandObservations: () => Promise<unknown[]>;
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

async function resetNativeCommandObservations(app: ElectronApplication, page: Page): Promise<void> {
  const hasBridge = await page.evaluate(() => typeof window.videoEditorTestObservations?.getNativeCommandObservations === "function");
  if (!hasBridge) {
    throw new Error("inspector modal test setup error: native test observation bridge is unavailable");
  }

  await app.evaluate(() => {
    (globalThis as typeof globalThis & { __videoEditorTestNativeCommandObservations?: NativeCommandObservation[] })
      .__videoEditorTestNativeCommandObservations = [];
    (globalThis as typeof globalThis & { __videoEditorTestProjectSessionCalls?: ProjectSessionCall[] })
      .__videoEditorTestProjectSessionCalls = [];
  });
}

async function readNativeCommandObservations(app: ElectronApplication): Promise<NativeCommandObservation[]> {
  const [directNativeObservations, projectCalls] = await Promise.all([
    app.evaluate(() => {
      return (
        (globalThis as typeof globalThis & { __videoEditorTestNativeCommandObservations?: NativeCommandObservation[] })
          .__videoEditorTestNativeCommandObservations ?? []
      );
    }),
    app.evaluate(() => {
      return (
        (globalThis as typeof globalThis & { __videoEditorTestProjectSessionCalls?: ProjectSessionCall[] })
          .__videoEditorTestProjectSessionCalls ?? []
      );
    })
  ]);
  return [
    ...directNativeObservations,
    ...projectCalls
      .filter((call) => call.command === "executeProjectIntent" && call.intentKind !== null)
      .map((call) => ({
        command: call.intentKind ?? "executeProjectIntent",
        canvasConfig: call.canvasConfig ?? null
      }))
  ];
}

async function openDraftParametersDialog(page: Page) {
  await page.getByLabel("草稿参数").getByRole("button", { name: "修改" }).click();
  const dialog = page.getByRole("dialog", { name: "草稿参数" });
  await expect(dialog).toBeVisible();
  return dialog;
}

test.describe("draft parameter inspector modal", () => {
  test("draft parameter edits are realtime and finish records updateDraftCanvasConfig", async () => {
    const { app, page } = await launchWorkspaceApp();

    try {
      await resetNativeCommandObservations(app, page);

      const inspector = page.getByLabel("草稿参数");
      await expect(inspector).toContainText("草稿参数");
      await expect(inspector.getByRole("button", { name: "修改" })).toBeVisible();
      await expect(inspector).toContainText("16:9");

      let dialog = await openDraftParametersDialog(page);
      await dialog.getByRole("group", { name: "画布比例" }).getByRole("button", { name: "9:16" }).click();
      await expect(dialog.getByLabel("画布宽度")).toHaveValue("1080");
      await expect(dialog.getByLabel("画布高度")).toHaveValue("1920");
      await dialog.getByRole("button", { name: "关闭", exact: true }).click();
      await expect(page.getByRole("dialog", { name: "草稿参数" })).toHaveCount(0);
      await expect(inspector).toContainText("16:9");
      expect((await readNativeCommandObservations(app)).some((call) => call.command === "updateDraftCanvasConfig")).toBe(false);

      dialog = await openDraftParametersDialog(page);
      await dialog.getByRole("group", { name: "画布比例" }).getByRole("button", { name: "9:16" }).click();
      await dialog.getByRole("group", { name: "画布背景" }).getByRole("button", { name: "模糊填充" }).click();
      await expect(dialog.getByRole("button", { name: "应用草稿参数" })).toHaveCount(0);
      await expect(dialog.getByRole("button", { name: "完成" })).toBeEnabled();
      await dialog.getByRole("button", { name: "完成" }).click();
      await expect(page.getByRole("dialog", { name: "草稿参数" })).toHaveCount(0);

      await expect
        .poll(async () => (await readNativeCommandObservations(app)).some((call) => call.command === "updateDraftCanvasConfig"))
        .toBe(true);
      await expect(page.getByLabel("预览窗口").getByRole("button", { name: "画布读数" })).toHaveAttribute(
        "title",
        "画布 9:16 · 1080 x 1920 · 30 fps"
      );

      const canvasCall = (await readNativeCommandObservations(app)).find((call) => call.command === "updateDraftCanvasConfig");
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

import { _electron as electron, expect, test, type ElectronApplication, type Page } from "@playwright/test";
import { join } from "node:path";

import type { CommandName } from "../src/generated/CommandEnvelope";

type ExecuteCommandCall = {
  command: CommandName;
  outputPath: string | null;
  preset: string | null;
  jobId: string | null;
};

async function launchWorkspaceApp(
  options: { showDeveloperDiagnostics?: boolean } = {}
): Promise<{ app: ElectronApplication; page: Page }> {
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
      VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: options.showDeveloperDiagnostics === true ? "1" : "0",
      VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify(["/tmp/demo-material.mp4"])
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  return { app, page };
}

async function readExecuteCommandCalls(app: ElectronApplication): Promise<ExecuteCommandCall[]> {
  return app.evaluate(() => {
    return (
      (globalThis as typeof globalThis & { __videoEditorTestExecuteCommandCalls?: ExecuteCommandCall[] })
        .__videoEditorTestExecuteCommandCalls ?? []
    );
  });
}

async function expectCommandCall(app: ElectronApplication, command: CommandName): Promise<void> {
  await expect
    .poll(async () => (await readExecuteCommandCalls(app)).some((call) => call.command === command))
    .toBe(true);
}

test("top-right export action opens an accessible modal and preview has no production export panel", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await expect(page.getByLabel("预览窗口").getByLabel("导出面板")).toHaveCount(0);

    await page.getByRole("button", { name: "导出", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "导出" });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByLabel("输出路径")).toHaveValue("video-editor-export.mp4");
    await expect(dialog.getByLabel("导出预设")).toHaveValue("h264AacBalanced");
    await expect(dialog.getByLabel("分辨率")).toBeVisible();
    await expect(dialog.getByLabel("帧率")).toBeVisible();
    await expect(dialog.getByLabel("视频码率")).toBeVisible();
    await expect(dialog.getByRole("checkbox", { name: "导出音频" })).toBeChecked();
    await expect(dialog.getByRole("button", { name: "取消导出" })).toBeDisabled();
    await expect(dialog.getByRole("button", { name: "打开位置" })).toBeDisabled();

    await dialog.getByRole("button", { name: "关闭" }).click();
    await expect(dialog).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("export modal starts, cancels, refreshes, and keeps command ownership in helpers", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await page.getByRole("button", { name: "导出", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "导出" });
    await dialog.getByLabel("输出路径").fill("/tmp/video-editor-export.mp4");
    await dialog.getByRole("button", { name: "开始导出" }).click();
    await expectCommandCall(app, "startExport");
    await expect(dialog.getByLabel("导出进度")).toContainText("导出中");
    await expect(dialog.getByLabel("导出进度")).toContainText("12%");
    await expect(dialog.getByRole("button", { name: "取消导出" })).toBeEnabled();

    await dialog.getByRole("button", { name: "取消导出" }).click();
    await expectCommandCall(app, "cancelExport");
    await expect(dialog.getByLabel("导出进度")).toContainText("已取消");

    await dialog.getByRole("button", { name: "开始导出" }).click();
    await dialog.getByRole("button", { name: "查询导出状态" }).click();
    await expectCommandCall(app, "getExportJobStatus");
    await expect(dialog.getByLabel("导出进度")).toContainText("已完成");
    await expect(dialog.getByLabel("输出校验")).toContainText("1920x1080");
    await expect(dialog.getByLabel("输出校验")).toContainText("含音频");
    await expect(dialog.getByRole("button", { name: "打开位置" })).toBeEnabled();

    const calls = await readExecuteCommandCalls(app);
    expect(calls.map((call) => call.command)).toEqual(
      expect.arrayContaining(["startExport", "cancelExport", "getExportJobStatus"])
    );
    const startCall = calls.find((call) => call.command === "startExport");
    expect(startCall?.outputPath).toBe("/tmp/video-editor-export.mp4");
    expect(startCall?.preset).toBe("h264AacBalanced");
  } finally {
    await app.close();
  }
});

test("advanced export settings expand, audio dropdown opens, and default modal copy stays product-safe", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await page.getByRole("button", { name: "导出", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "导出" });
    await expect(dialog.getByText(/FFmpeg|ffprobe|artifact|cache|\/tmp\//)).toHaveCount(0);

    const advancedToggle = dialog.getByRole("button", { name: "高级设置" });
    await expect(advancedToggle).toHaveAttribute("aria-expanded", "false");
    await advancedToggle.click();
    await expect(advancedToggle).toHaveAttribute("aria-expanded", "true");
    await expect(dialog.getByLabel("高级导出设置")).toBeVisible();
    await expect(dialog.getByLabel("编码格式")).toBeVisible();

    const advancedBox = await dialog.getByLabel("高级导出设置").boundingBox();
    const actionBox = await dialog.getByRole("group", { name: "导出操作" }).boundingBox();
    expect(advancedBox, "advanced settings box").not.toBeNull();
    expect(actionBox, "export action box").not.toBeNull();
    expect(advancedBox!.y + advancedBox!.height).toBeLessThanOrEqual(actionBox!.y);

    const sampleRate = dialog.getByRole("combobox", { name: "音频采样率" });
    await expect(sampleRate).toHaveAttribute("aria-expanded", "false");
    await sampleRate.click();
    await expect(sampleRate).toHaveAttribute("aria-expanded", "true");
    const listbox = dialog.getByRole("listbox", { name: "音频采样率选项" });
    await expect(listbox).toBeVisible();
    await expect(listbox.getByRole("option", { name: "48 kHz" })).toBeVisible();
    await listbox.getByRole("option", { name: "44.1 kHz" }).click();
    await expect(sampleRate).toHaveAttribute("aria-expanded", "false");
    await expect(sampleRate).toContainText("44.1 kHz");
  } finally {
    await app.close();
  }
});

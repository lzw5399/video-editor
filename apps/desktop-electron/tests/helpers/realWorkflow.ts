import { expect, type ElectronApplication, type Page } from "@playwright/test";
import { access } from "node:fs/promises";

import type { CommandName } from "../../src/generated/CommandEnvelope";
import type { Phase6MediaFixtures } from "./mediaFixtures";

type ExecuteCommandCall = {
  command: CommandName;
  kind: string;
};

export type RealWorkflowResult = {
  calls: ExecuteCommandCall[];
  framePath: string;
  segmentPath: string;
  outputPath: string;
};

export async function runRealImportPreviewExportWorkflow(
  app: ElectronApplication,
  page: Page,
  fixtures: Phase6MediaFixtures
): Promise<RealWorkflowResult> {
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  await expect(page.getByRole("button", { name: "请求预览帧" })).toBeEnabled({ timeout: 20_000 });

  await importMaterials(page, app, [
    { name: fixtures.videoName },
    { name: fixtures.audioName }
  ]);

  await addVideoSegment(page, app, fixtures.videoName);
  await addAudioSegment(page, app, fixtures.audioName);

  const framePath = await requestPreviewFrame(page, app);
  const segmentPath = await requestPreviewSegment(page, app);
  await exportDraft(page, app, fixtures.outputPath, fixtures.expectedResolutionLabel);

  const calls = await readExecuteCommandCalls(app);
  expect(calls.map((call) => call.command)).toEqual(
    expect.arrayContaining([
      "probeRuntimeCapabilities",
      "importMaterial",
      "addSegment",
      "addAudioSegment",
      "requestPreviewFrame",
      "requestPreviewSegment",
      "startExport",
      "getExportJobStatus"
    ])
  );

  return {
    calls,
    framePath,
    segmentPath,
    outputPath: fixtures.outputPath
  };
}

export async function readExecuteCommandCalls(app: ElectronApplication): Promise<ExecuteCommandCall[]> {
  return app.evaluate(() => {
    return (
      (globalThis as typeof globalThis & { __videoEditorTestExecuteCommandCalls?: ExecuteCommandCall[] })
        .__videoEditorTestExecuteCommandCalls ?? []
    );
  });
}

async function importMaterials(
  page: Page,
  app: ElectronApplication,
  materials: Array<{ name: string }>
): Promise<void> {
  const nextCount = (await countCommand(app, "importMaterial")) + materials.length;
  await page.getByRole("button", { name: "导入素材" }).click();
  await waitForCommandCount(page, app, "importMaterial", nextCount);
  for (const material of materials) {
    await expect(page.getByRole("article", { name: `素材 ${material.name}` })).toContainText("可用", { timeout: 20_000 });
  }
}

async function addVideoSegment(page: Page, app: ElectronApplication, videoName: string): Promise<void> {
  const nextCount = (await countCommand(app, "addSegment")) + 1;
  await page.locator(".compact-select select").selectOption({ label: videoName });
  await page.getByRole("button", { name: "添加片段" }).click();
  await waitForCommandCount(page, app, "addSegment", nextCount);
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(videoName)}`) })).toBeVisible();
}

async function addAudioSegment(page: Page, app: ElectronApplication, audioName: string): Promise<void> {
  const nextCount = (await countCommand(app, "addAudioSegment")) + 1;
  await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "音频" }).click();
  await page.getByLabel("BGM素材").selectOption({ label: audioName });
  await page.getByLabel("时长（微秒）").fill("2000000");
  await page.getByRole("button", { name: "添加音频", exact: true }).click();
  await waitForCommandCount(page, app, "addAudioSegment", nextCount);
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(audioName)}`) })).toBeVisible();
}

async function requestPreviewFrame(page: Page, app: ElectronApplication): Promise<string> {
  const nextCount = (await countCommand(app, "requestPreviewFrame")) + 1;
  await page.getByRole("button", { name: "请求预览帧" }).click();
  await waitForCommandCount(page, app, "requestPreviewFrame", nextCount);
  await expect(page.getByLabel("预览产物")).toContainText(/预览帧(已生成|命中缓存)/, { timeout: 30_000 });
  const path = await page.locator(".preview-artifact-line").filter({ hasText: "预览帧" }).locator("code").textContent();
  expect(path, "预览帧产物路径").not.toBeNull();
  await expectFileExists(path!);
  return path!;
}

async function requestPreviewSegment(page: Page, app: ElectronApplication): Promise<string> {
  const nextCount = (await countCommand(app, "requestPreviewSegment")) + 1;
  await page.getByRole("button", { name: "生成预览片段" }).click();
  await waitForCommandCount(page, app, "requestPreviewSegment", nextCount);
  await expect(page.getByLabel("预览产物")).toContainText(/预览片段(已生成|命中缓存)/, { timeout: 30_000 });
  const path = await page.locator(".preview-artifact-line").filter({ hasText: "预览片段" }).locator("code").textContent();
  expect(path, "预览片段产物路径").not.toBeNull();
  await expectFileExists(path!);
  return path!;
}

async function exportDraft(
  page: Page,
  app: ElectronApplication,
  outputPath: string,
  expectedResolutionLabel: string
): Promise<void> {
  const nextStartCount = (await countCommand(app, "startExport")) + 1;
  await page.getByLabel("输出路径").fill(outputPath);
  await expect(page.getByRole("button", { name: "开始导出" })).toBeEnabled({ timeout: 20_000 });
  await page.getByRole("button", { name: "开始导出" }).click();
  await waitForCommandCount(page, app, "startExport", nextStartCount);
  await expect(page.getByRole("button", { name: "查询导出状态" })).toBeEnabled({ timeout: 10_000 });

  for (let attempt = 0; attempt < 40; attempt += 1) {
    const progressText = (await page.getByLabel("导出进度").textContent()) ?? "";
    if (progressText.includes("已完成")) {
      break;
    }

    const nextStatusCount = (await countCommand(app, "getExportJobStatus")) + 1;
    await page.getByRole("button", { name: "查询导出状态" }).click();
    await waitForCommandCount(page, app, "getExportJobStatus", nextStatusCount);
    await page.waitForTimeout(500);
  }

  await expect(page.getByLabel("导出进度")).toContainText("已完成", { timeout: 5_000 });
  await expect(page.getByLabel("输出校验")).toContainText(expectedResolutionLabel);
  await expect(page.getByLabel("输出校验")).toContainText("含音频");
  await expectFileExists(outputPath);
}

async function waitForCommandCount(
  page: Page,
  app: ElectronApplication,
  command: CommandName,
  expectedCount: number
): Promise<void> {
  try {
    await expect.poll(async () => countCommand(app, command), { timeout: 30_000 }).toBeGreaterThanOrEqual(expectedCount);
  } catch (error) {
    const calls = await readExecuteCommandCalls(app);
    const materialCards = await page.getByRole("article").allTextContents();
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      [
        message,
        `Expected at least ${expectedCount} ${command} command(s).`,
        `Recorded commands: ${JSON.stringify(calls)}`,
        `Visible articles: ${JSON.stringify(materialCards)}`
      ].join("\n")
    );
  }
}

async function countCommand(app: ElectronApplication, command: CommandName): Promise<number> {
  return (await readExecuteCommandCalls(app)).filter((call) => call.command === command).length;
}

async function expectFileExists(path: string): Promise<void> {
  await expect(access(path).then(
    () => true,
    () => false
  )).resolves.toBe(true);
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

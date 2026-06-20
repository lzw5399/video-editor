import { expect, type ElectronApplication, type Page } from "@playwright/test";
import { execFile } from "node:child_process";
import { access } from "node:fs/promises";
import { join } from "node:path";
import { promisify } from "node:util";

import type { CommandName } from "../../src/generated/CommandEnvelope";
import type { Phase6MediaFixtures } from "./mediaFixtures";

const execFileAsync = promisify(execFile);

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
  await enterProjectFromProductEntryIfNeeded(page, app);
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  await expect(page.getByRole("button", { name: "请求预览帧" })).toBeEnabled({ timeout: 20_000 });
  await setBundlePath(page, fixtures.bundlePath);

  await importMaterials(page, app, [
    { name: fixtures.videoName },
    { name: fixtures.imageName },
    { name: fixtures.audioName }
  ]);

  await addVisualSegment(page, app, fixtures.videoName);
  await addTextSegment(page, app, fixtures.expectedTextContent);
  await addAudioSegment(page, app, fixtures.audioName);
  await addVisualSegment(page, app, fixtures.imageName);
  await waitForCommandCount(page, app, "saveProjectBundle", 4);
  await page.getByLabel("预览时间").fill("0");

  const framePath = await requestPreviewFrame(page, app);
  const segmentPath = await requestPreviewSegment(page, app);
  await exportDraft(page, app, fixtures);

  const calls = await readExecuteCommandCalls(app);
  expect(calls.map((call) => call.command)).toEqual(
    expect.arrayContaining([
      "probeRuntimeCapabilities",
      "importMaterial",
      "addSegment",
      "addTextSegment",
      "addAudioSegment",
      "saveProjectBundle",
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

export async function assertReopenedProjectState(page: Page, fixtures: Phase6MediaFixtures): Promise<void> {
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  await expect(page.getByLabel("草稿包路径")).toHaveValue(fixtures.bundlePath);
  await expect(page.getByRole("article", { name: `素材 ${fixtures.videoName}` })).toContainText("可用", { timeout: 20_000 });
  await expect(page.getByRole("article", { name: `素材 ${fixtures.imageName}` })).toContainText("可用", { timeout: 20_000 });
  await expect(page.getByRole("article", { name: `素材 ${fixtures.audioName}` })).toContainText("可用", { timeout: 20_000 });
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(fixtures.videoName)}`) })).toBeVisible();
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(fixtures.imageName)}`) })).toBeVisible();
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(fixtures.audioName)}`) })).toBeVisible();
  const textSegment = page.getByRole("button", { name: /片段 默认文字/ });
  await expect(textSegment).toBeVisible();
  await textSegment.click();
  await expect(page.getByRole("complementary", { name: "属性检查器" }).getByRole("textbox", { name: "文字内容" })).toHaveValue(
    fixtures.expectedTextContent
  );
}

async function enterProjectFromProductEntryIfNeeded(page: Page, app: ElectronApplication): Promise<void> {
  if ((await page.getByRole("main", { name: "项目入口" }).count()) === 0) {
    return;
  }

  const nextCount = (await countCommand(app, "saveProjectBundle")) + 1;
  await expect(page.getByRole("button", { name: "导入素材" })).toHaveCount(0);
  await page.getByRole("button", { name: "新建项目" }).click();
  await waitForCommandCount(page, app, "saveProjectBundle", nextCount);
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

async function setBundlePath(page: Page, bundlePath: string): Promise<void> {
  const input = page.getByLabel("草稿包路径");
  await expect(input).toBeVisible();
  await input.fill(bundlePath);
  await expect(input).toHaveValue(bundlePath);
}

async function addVisualSegment(page: Page, app: ElectronApplication, materialName: string): Promise<void> {
  const nextCount = (await countCommand(app, "addSegment")) + 1;
  await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "媒体" }).click();
  await page.locator(".compact-select select").selectOption({ label: materialName });
  await page.getByRole("button", { name: "添加片段" }).click();
  await waitForCommandCount(page, app, "addSegment", nextCount);
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(materialName)}`) })).toBeVisible();
}

async function addTextSegment(page: Page, app: ElectronApplication, content: string): Promise<void> {
  const nextCount = (await countCommand(app, "addTextSegment")) + 1;
  await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "文字" }).click();
  await page.getByLabel("默认文字").getByLabel("文字内容").fill(content);
  await page.getByLabel("默认文字").getByLabel("时长（微秒）").fill("3000000");
  await page.getByRole("button", { name: "添加文字", exact: true }).click();
  await waitForCommandCount(page, app, "addTextSegment", nextCount);
  await expect(page.getByRole("button", { name: /片段 默认文字/ })).toBeVisible();
  await expect(page.getByLabel("预览文字")).toContainText(content);
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
  fixtures: Phase6MediaFixtures
): Promise<void> {
  const nextStartCount = (await countCommand(app, "startExport")) + 1;
  const outputPath = fixtures.outputPath;
  await page.getByLabel("输出路径").fill(outputPath);
  await expect(page.getByRole("button", { name: "开始导出" })).toBeEnabled({ timeout: 20_000 });
  await page.getByRole("button", { name: "开始导出" }).click();
  await waitForCommandCount(page, app, "startExport", nextStartCount);
  const statusButton = page.getByRole("button", { name: "查询导出状态" });
  try {
    await expect(statusButton).toBeEnabled({ timeout: 10_000 });
  } catch (error) {
    const calls = await readExecuteCommandCalls(app);
    const progressText = (await page.getByLabel("导出进度").textContent()) ?? "";
    const logText = (await page.getByLabel("导出日志").textContent()) ?? "";
    const validationText = (await page.getByLabel("输出校验").textContent()) ?? "";
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      [
        message,
        `Export progress: ${progressText}`,
        `Export log: ${logText}`,
        `Export validation: ${validationText}`,
        `Recorded commands: ${JSON.stringify(calls)}`
      ].join("\n")
    );
  }

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
  await expect(page.getByLabel("输出校验")).toContainText(fixtures.expectedResolutionLabel);
  await expect(page.getByLabel("输出校验")).toContainText("含音频");
  await expectFileExists(outputPath);
  await expectExportMedia(outputPath, fixtures);
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

async function expectExportMedia(path: string, fixtures: Phase6MediaFixtures): Promise<void> {
  const ffprobePath = process.env.VE_FFPROBE_PATH ?? "ffprobe";
  const { stdout } = await execFileAsync(
    ffprobePath,
    ["-v", "error", "-print_format", "json", "-show_format", "-show_streams", path],
    {
      timeout: 20_000,
      maxBuffer: 1024 * 1024
    }
  );
  const probe = JSON.parse(stdout) as {
    format?: { duration?: string };
    streams?: Array<{ codec_type?: string; width?: number; height?: number; avg_frame_rate?: string }>;
  };
  const videoStream = probe.streams?.find((stream) => stream.codec_type === "video");
  const audioStream = probe.streams?.find((stream) => stream.codec_type === "audio");
  expect(videoStream?.width).toBe(fixtures.expectedWidth);
  expect(videoStream?.height).toBe(fixtures.expectedHeight);
  expect(videoStream?.avg_frame_rate).toBe(fixtures.expectedFrameRate);
  expect(audioStream, "export should contain an audio stream").toBeDefined();
  const duration = Number(probe.format?.duration ?? "0");
  expect(duration).toBeGreaterThan(fixtures.expectedDurationSeconds - 0.35);
  expect(duration).toBeLessThan(fixtures.expectedDurationSeconds + 0.35);
  await expectFileExists(join(fixtures.bundlePath, "project.json"));
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

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
  outputPath?: string | null;
  preset?: string | null;
};

type ProjectSessionCall = {
  command:
    | "createProjectSession"
    | "openProjectSession"
    | "executeProjectIntent"
    | "listProjectSessionMaterials"
    | "listProjectSessionMissingMaterials"
    | "startProjectSessionExport"
    | "closeProjectSession";
  intentKind: string | null;
  timelineSemanticKeys?: string[];
  outputPath?: string | null;
  preset?: string | null;
};

type RealtimePreviewHostCall = {
  kind: string;
  playbackGeneration?: number;
};

type RealtimePreviewHostState = {
  ok: boolean;
  productReady: boolean;
  fallbackActive: boolean;
  backend: "renderGraphGpu" | "none";
  diagnosticSource: "nativeVideoBridge" | "runtimeFrameRequest" | "none";
  telemetry: {
    presentedFrameCount: number;
    targetTimeMicroseconds: number;
    playbackGeneration: number;
  } | null;
  contentEvidence: {
    source: "nativeVideoBridge" | "renderGraphGpuComposited";
    digest: string;
    targetTimeMicroseconds: number;
  } | null;
};

export type RealWorkflowResult = {
  calls: ExecuteCommandCall[];
  realtimePreviewHostCalls: RealtimePreviewHostCall[];
  outputPath: string;
};

export async function runRealImportPreviewExportWorkflow(
  app: ElectronApplication,
  page: Page,
  fixtures: Phase6MediaFixtures
): Promise<RealWorkflowResult> {
  await enterProjectFromProductEntryIfNeeded(page, app);
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();

  await importMaterials(page, app, [
    { name: fixtures.videoName },
    { name: fixtures.imageName },
    { name: fixtures.audioName }
  ]);

  await addVisualSegment(page, app, fixtures.videoName);
  await addTextSegment(page, app, fixtures.expectedTextContent);
  await addAudioSegment(page, app, fixtures.audioName);
  await addVisualSegment(page, app, fixtures.imageName);
  await expectFileExists(join(fixtures.bundlePath, "project.json"));

  await verifyRealtimePreviewPlayback(page, app);
  await exportDraft(page, app, fixtures);

  const calls = await readExecuteCommandCalls(app);
  const projectCalls = await readProjectSessionCalls(app);
  const observedActions = [
    ...calls.map((call) => call.command),
    ...projectCalls
      .filter((call) => call.command === "executeProjectIntent" && call.intentKind !== null)
      .map((call) => call.intentKind as string)
  ];
  expect(calls.map((call) => call.command)).toEqual(
    expect.arrayContaining([
      "startExport",
      "getExportJobStatus"
    ])
  );
  expect(projectCalls.map((call) => call.command)).toContain("createProjectSession");
  expect(observedActions).toEqual(
    expect.arrayContaining(["addTimelineSegmentIntent", "addTextSegmentIntent", "addAudioSegmentIntent"])
  );
  expect(calls.filter((call) => call.command === "requestPreviewFrame")).toHaveLength(0);
  expect(calls.filter((call) => call.command === "requestPreviewSegment")).toHaveLength(0);

  return {
    calls,
    realtimePreviewHostCalls: await readRealtimePreviewHostCalls(app),
    outputPath: fixtures.outputPath
  };
}

export async function assertReopenedProjectState(page: Page, fixtures: Phase6MediaFixtures): Promise<void> {
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  await expect(page.getByRole("article", { name: `素材 ${fixtures.videoName}` })).toContainText("可用", { timeout: 20_000 });
  await expect(page.getByRole("article", { name: `素材 ${fixtures.imageName}` })).toContainText("可用", { timeout: 20_000 });
  await expect(page.getByRole("article", { name: `素材 ${fixtures.audioName}` })).toContainText("可用", { timeout: 20_000 });
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(fixtures.videoName)}`) })).toBeVisible();
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(fixtures.imageName)}`) })).toBeVisible();
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(fixtures.audioName)}`) })).toBeVisible();
  const textSegment = page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(fixtures.expectedTextContent)}`) });
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

  const nextSaveCount = (await countCommand(app, "saveProjectBundle")) + 1;
  const nextCreateSessionCount = (await countProjectSessionCommand(app, "createProjectSession")) + 1;
  await expect(page.getByRole("button", { name: "导入素材" })).toHaveCount(0);
  await page.getByRole("button", { name: "新建项目" }).click();
  await expect
    .poll(
      async () => {
        const saveReached = (await countCommand(app, "saveProjectBundle")) >= nextSaveCount;
        const sessionReached = (await countProjectSessionCommand(app, "createProjectSession")) >= nextCreateSessionCount;
        const workspaceVisible = (await page.getByRole("main", { name: "剪映风格编辑工作区" }).count()) > 0;
        return workspaceVisible && (saveReached || sessionReached);
      },
      { timeout: 30_000 }
    )
    .toBe(true);
}

export async function readExecuteCommandCalls(app: ElectronApplication): Promise<ExecuteCommandCall[]> {
  const [legacyCalls, projectCalls] = await Promise.all([
    app.evaluate(() => {
      return (
        (globalThis as typeof globalThis & { __videoEditorTestExecuteCommandCalls?: ExecuteCommandCall[] })
          .__videoEditorTestExecuteCommandCalls ?? []
      );
    }),
    readProjectSessionCalls(app)
  ]);
  return [
    ...legacyCalls,
    ...projectCalls
      .filter((call) => call.command === "startProjectSessionExport")
      .map((call) => ({
        command: "startExport" as CommandName,
        kind: "startExport",
        outputPath: call.outputPath ?? null,
        preset: call.preset ?? null
      }))
  ];
}

async function readLegacyExecuteCommandCalls(app: ElectronApplication): Promise<ExecuteCommandCall[]> {
  return app.evaluate(() => {
    return (
      (globalThis as typeof globalThis & { __videoEditorTestExecuteCommandCalls?: ExecuteCommandCall[] })
        .__videoEditorTestExecuteCommandCalls ?? []
    );
  });
}

async function readRealtimePreviewHostCalls(app: ElectronApplication): Promise<RealtimePreviewHostCall[]> {
  return app.evaluate(() => {
    return (
      (globalThis as typeof globalThis & { __videoEditorTestRealtimePreviewHostCalls?: RealtimePreviewHostCall[] })
        .__videoEditorTestRealtimePreviewHostCalls ?? []
    );
  });
}

async function readRealtimePreviewHostState(page: Page): Promise<RealtimePreviewHostState | null> {
  await page.evaluate(() => {
    const target = window as typeof window & {
      __videoEditorRealtimePreviewHostState?: RealtimePreviewHostState | null;
      __videoEditorRealtimePreviewHostObserverInstalled?: boolean;
      videoEditorRealtimePreviewHost?: {
        subscribeTelemetry: (listener: (state: RealtimePreviewHostState) => void) => () => void;
      };
    };
    if (target.__videoEditorRealtimePreviewHostObserverInstalled) {
      return;
    }
    target.__videoEditorRealtimePreviewHostObserverInstalled = true;
    target.__videoEditorRealtimePreviewHostState = null;
    target.videoEditorRealtimePreviewHost?.subscribeTelemetry((state) => {
      target.__videoEditorRealtimePreviewHostState = state;
    });
  });
  return page.evaluate(() => {
    return (
      (window as typeof window & {
        __videoEditorRealtimePreviewHostState?: RealtimePreviewHostState | null;
      }).__videoEditorRealtimePreviewHostState ?? null
    );
  });
}

async function importMaterials(
  page: Page,
  _app: ElectronApplication,
  materials: Array<{ name: string }>
): Promise<void> {
  await page.getByRole("button", { name: "导入素材" }).click();
  for (const material of materials) {
    await expect(page.getByRole("article", { name: `素材 ${material.name}` })).toContainText("可用", { timeout: 20_000 });
  }
}

async function addVisualSegment(page: Page, app: ElectronApplication, materialName: string): Promise<void> {
  const nextCount = (await countProjectSessionIntent(app, "addTimelineSegmentIntent")) + 1;
  await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "媒体" }).click();
  const materialRow = page.getByRole("article", { name: `素材 ${materialName}` });
  await expect(materialRow).toContainText("可用", { timeout: 20_000 });
  await materialRow.getByRole("button", { name: `添加 ${materialName} 到时间线` }).click();
  await waitForProjectSessionIntentCount(page, app, "addTimelineSegmentIntent", nextCount);
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(materialName)}`) })).toBeVisible();
}

async function addTextSegment(page: Page, app: ElectronApplication, content: string): Promise<void> {
  const nextCommandCount = (await countCommand(app, "addTextSegmentIntent")) + 1;
  const nextIntentCount = (await countProjectSessionIntent(app, "addTextSegmentIntent")) + 1;
  await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "文字" }).click();
  await page.getByLabel("默认文字").getByLabel("文字内容").fill(content);
  await page.getByLabel("文字时长（秒）").fill("3");
  await page.getByRole("button", { name: "添加文字", exact: true }).click();
  await waitForCommandOrProjectIntentCount(page, app, "addTextSegmentIntent", nextCommandCount, nextIntentCount);
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(content)}`) })).toBeVisible();
  await expect(page.getByLabel("预览文字")).toContainText(content);
}

async function addAudioSegment(page: Page, app: ElectronApplication, audioName: string): Promise<void> {
  const nextCommandCount = (await countCommand(app, "addAudioSegmentIntent")) + 1;
  const nextIntentCount = (await countProjectSessionIntent(app, "addAudioSegmentIntent")) + 1;
  await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "音频" }).click();
  await page.getByLabel("BGM素材").selectOption({ label: audioName });
  await page.getByLabel("音频时长（秒）").fill("2");
  await page.getByRole("button", { name: "添加音频", exact: true }).click();
  await waitForCommandOrProjectIntentCount(page, app, "addAudioSegmentIntent", nextCommandCount, nextIntentCount);
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(audioName)}`) })).toBeVisible();
}

async function verifyRealtimePreviewPlayback(page: Page, app: ElectronApplication): Promise<void> {
  const previewMonitor = page.getByLabel("预览窗口");
  await expect(previewMonitor.getByLabel("实时预览画面")).toBeVisible({ timeout: 20_000 });
  const frameRequestsBefore = await countCommand(app, "requestPreviewFrame");
  const segmentRequestsBefore = await countCommand(app, "requestPreviewSegment");
  const playCallsBefore = (await readRealtimePreviewHostCalls(app)).filter((call) => call.kind === "play").length;
  const stateBefore = await readRealtimePreviewHostState(page);
  const presentedBefore = stateBefore?.telemetry?.presentedFrameCount ?? 0;
  const targetBefore = stateBefore?.contentEvidence?.targetTimeMicroseconds ?? -1;

  await previewMonitor.getByRole("button", { name: "播放" }).click();
  await expect
    .poll(async () => (await readRealtimePreviewHostCalls(app)).filter((call) => call.kind === "play").length, {
      timeout: 10_000
    })
    .toBeGreaterThan(playCallsBefore);

  await expect
    .poll(
      async () => {
        const state = await readRealtimePreviewHostState(page);
        return (
          state?.ok === true &&
          state.productReady === true &&
          state.fallbackActive === false &&
          state.backend === "renderGraphGpu" &&
          state.diagnosticSource === "none" &&
          state.contentEvidence?.source === "renderGraphGpuComposited" &&
          (state.telemetry?.presentedFrameCount ?? 0) > presentedBefore &&
          (state.contentEvidence?.targetTimeMicroseconds ?? -1) > targetBefore
        );
      },
      { timeout: 15_000 }
    )
    .toBe(true);

  await previewMonitor.getByRole("button", { name: "暂停" }).click();
  expect(await countCommand(app, "requestPreviewFrame")).toBe(frameRequestsBefore);
  expect(await countCommand(app, "requestPreviewSegment")).toBe(segmentRequestsBefore);
}

async function exportDraft(
  page: Page,
  app: ElectronApplication,
  fixtures: Phase6MediaFixtures
): Promise<void> {
  const nextStartCount = (await countCommand(app, "startExport")) + 1;
  const outputPath = fixtures.outputPath;
  await page.getByLabel("产品操作").getByRole("button", { name: "导出", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "导出" });
  await expect(dialog).toBeVisible();
  await expect(page.getByLabel("预览窗口").getByLabel("导出面板")).toHaveCount(0);
  await dialog.getByLabel("输出路径").fill(outputPath);
  await expect(dialog.getByRole("button", { name: "开始导出" })).toBeEnabled({ timeout: 20_000 });
  await dialog.getByRole("button", { name: "开始导出" }).click();
  await waitForCommandCount(page, app, "startExport", nextStartCount);
  const statusButton = dialog.getByRole("button", { name: "查询导出状态" });
  try {
    await expect(statusButton).toBeEnabled({ timeout: 10_000 });
  } catch (error) {
    const calls = await readExecuteCommandCalls(app);
    const progressText = (await dialog.getByLabel("导出进度").textContent()) ?? "";
    const logText = (await dialog.getByLabel("导出状态").textContent()) ?? "";
    const validationText = (await dialog.getByLabel("输出校验").textContent()) ?? "";
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
    const progressText = (await dialog.getByLabel("导出进度").textContent()) ?? "";
    if (progressText.includes("已完成")) {
      break;
    }

    const nextStatusCount = (await countCommand(app, "getExportJobStatus")) + 1;
    await dialog.getByRole("button", { name: "查询导出状态" }).click();
    await waitForCommandCount(page, app, "getExportJobStatus", nextStatusCount);
    await page.waitForTimeout(500);
  }

  const finalProgressText = (await dialog.getByLabel("导出进度").textContent()) ?? "";
  if (!finalProgressText.includes("已完成")) {
    const calls = await readExecuteCommandCalls(app);
    const logText = (await dialog.getByLabel("导出状态").textContent()) ?? "";
    const validationText = (await dialog.getByLabel("输出校验").textContent()) ?? "";
    throw new Error(
      [
        `Export did not complete: ${finalProgressText}`,
        `Export log: ${logText}`,
        `Export validation: ${validationText}`,
        `Recorded commands: ${JSON.stringify(calls)}`
      ].join("\n")
    );
  }

  await expect(dialog.getByLabel("导出进度")).toContainText("已完成", { timeout: 5_000 });
  await expect(dialog.getByLabel("输出校验")).toContainText(fixtures.expectedResolutionLabel);
  await expect(dialog.getByLabel("输出校验")).toContainText("含音频");
  await expectFileExists(outputPath);
  await expectExportMedia(outputPath, fixtures, page);
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

async function readProjectSessionCalls(app: ElectronApplication): Promise<ProjectSessionCall[]> {
  return app.evaluate(() => {
    return (
      (globalThis as typeof globalThis & { __videoEditorTestProjectSessionCalls?: ProjectSessionCall[] })
        .__videoEditorTestProjectSessionCalls ?? []
    );
  });
}

async function countProjectSessionCommand(
  app: ElectronApplication,
  command: ProjectSessionCall["command"]
): Promise<number> {
  return (await readProjectSessionCalls(app)).filter((call) => call.command === command).length;
}

async function waitForProjectSessionIntentCount(
  page: Page,
  app: ElectronApplication,
  intentKind: string,
  expectedCount: number
): Promise<void> {
  try {
    await expect
      .poll(async () => countProjectSessionIntent(app, intentKind), { timeout: 30_000 })
      .toBeGreaterThanOrEqual(expectedCount);
  } catch (error) {
    const calls = await readProjectSessionCalls(app);
    const materialCards = await page.getByRole("article").allTextContents();
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      [
        message,
        `Expected at least ${expectedCount} ${intentKind} project intent(s).`,
        `Recorded project session calls: ${JSON.stringify(calls)}`,
        `Visible articles: ${JSON.stringify(materialCards)}`
      ].join("\n")
    );
  }
}

async function waitForCommandOrProjectIntentCount(
  page: Page,
  app: ElectronApplication,
  name: CommandName,
  expectedCommandCount: number,
  expectedIntentCount: number
): Promise<void> {
  try {
    await expect
      .poll(
        async () =>
          (await countCommand(app, name)) >= expectedCommandCount ||
          (await countProjectSessionIntent(app, name)) >= expectedIntentCount,
        { timeout: 30_000 }
      )
      .toBe(true);
  } catch (error) {
    const commandCalls = await readLegacyExecuteCommandCalls(app);
    const projectCalls = await readProjectSessionCalls(app);
    const materialCards = await page.getByRole("article").allTextContents();
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      [
        message,
        `Expected ${name} through executeCommand or project session intent.`,
        `Recorded commands: ${JSON.stringify(commandCalls)}`,
        `Recorded project session calls: ${JSON.stringify(projectCalls)}`,
        `Visible articles: ${JSON.stringify(materialCards)}`
      ].join("\n")
    );
  }
}

async function countProjectSessionIntent(app: ElectronApplication, intentKind: string): Promise<number> {
  return (await readProjectSessionCalls(app)).filter(
    (call) => call.command === "executeProjectIntent" && call.intentKind === intentKind
  ).length;
}

async function expectFileExists(path: string): Promise<void> {
  await expect(access(path).then(
    () => true,
    () => false
  )).resolves.toBe(true);
}

async function expectExportMedia(path: string, fixtures: Phase6MediaFixtures, page: Page): Promise<void> {
  const ffprobePath = await readBundledFfprobePath(page);
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

async function readBundledFfprobePath(page: Page): Promise<string> {
  const runtime = await page.evaluate(() => {
    const api = (window as unknown as {
      videoEditorCore?: {
        probeMediaRuntime: () => Promise<{
          ok: boolean;
          data: null | { ffprobe?: { path?: string; source?: string | { kind?: string } } };
          error: null | { message?: string };
        }>;
      };
    }).videoEditorCore;
    return api?.probeMediaRuntime();
  });

  if (runtime?.ok !== true || runtime.data?.ffprobe?.path === undefined) {
    throw new Error(`Unable to read bundled ffprobe path from app runtime: ${JSON.stringify(runtime)}`);
  }
  const source = runtime.data.ffprobe.source;
  expect(typeof source === "string" ? source : source?.kind).toBe("bundled");
  expect(runtime.data.ffprobe.path).not.toContain("/opt/homebrew");
  return runtime.data.ffprobe.path;
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

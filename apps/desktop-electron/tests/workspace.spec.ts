import { _electron as electron, expect, test, type ElectronApplication, type Locator, type Page } from "@playwright/test";
import { join } from "node:path";

import type { CommandEnvelope, CommandName } from "../src/generated/CommandEnvelope";
import type { CommandResultEnvelope } from "../src/generated/CommandResultEnvelope";

type ExecuteCommandCall = {
  command: CommandName;
  kind: string;
  requestId: string | null;
};

type RegionBox = {
  x: number;
  y: number;
  width: number;
  height: number;
};

type VideoEditorCoreApi = {
  executeCommand: (command: CommandEnvelope) => Promise<CommandResultEnvelope<unknown>>;
};

declare global {
  interface Window {
    videoEditorCore?: VideoEditorCoreApi;
    __executeCommandCalls?: ExecuteCommandCall[];
  }
}

async function launchWorkspaceApp(): Promise<{ app: ElectronApplication; page: Page }> {
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")]
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await expectVisibleWorkspaceRegions(page);
  return { app, page };
}

async function expectVisibleWorkspaceRegions(page: Page): Promise<void> {
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  await expect(page.locator('[aria-label="顶部功能区"]').first()).toBeVisible();
  await expect(page.locator('[aria-label="素材面板"]')).toBeVisible();
  await expect(page.locator('[aria-label="预览窗口"]')).toBeVisible();
  await expect(page.locator('[aria-label="属性检查器"]')).toBeVisible();
  await expect(page.locator('[aria-label="时间线"]')).toBeVisible();
}

async function spyExecuteCommandCalls(app: ElectronApplication, page: Page): Promise<void> {
  const hasBridge = await page.evaluate(() => typeof window.videoEditorCore?.executeCommand === "function");
  if (!hasBridge) {
    throw new Error("workspace test setup error: native videoEditorCore.executeCommand is unavailable");
  }

  await app.evaluate(({ ipcMain }) => {
    const ipc = ipcMain as unknown as {
      _invokeHandlers?: Map<string, (event: unknown, command: CommandEnvelope) => Promise<CommandResultEnvelope<unknown>>>;
    };
    const handlers = ipc._invokeHandlers;
    const originalHandler = handlers?.get("core:executeCommand");

    if (handlers === undefined || originalHandler === undefined) {
      throw new Error("workspace test setup error: core:executeCommand IPC handler is unavailable");
    }

    (globalThis as typeof globalThis & { __executeCommandCalls?: ExecuteCommandCall[] }).__executeCommandCalls = [];
    handlers.set("core:executeCommand", async (event, command) => {
      (globalThis as typeof globalThis & { __executeCommandCalls?: ExecuteCommandCall[] }).__executeCommandCalls?.push({
        command: command.command,
        kind: command.payload.kind,
        requestId: command.requestId ?? null
      });
      return originalHandler(event, command);
    });
  });
}

async function readExecuteCommandCalls(app: ElectronApplication): Promise<ExecuteCommandCall[]> {
  return app.evaluate(() => {
    return (globalThis as typeof globalThis & { __executeCommandCalls?: ExecuteCommandCall[] }).__executeCommandCalls ?? [];
  });
}

async function expectCommandCall(app: ElectronApplication, command: CommandName): Promise<void> {
  await expect
    .poll(async () => (await readExecuteCommandCalls(app)).some((call) => call.command === command))
    .toBe(true);
}

async function setViewportSizeAndVerifyLayout(app: ElectronApplication, page: Page, width: number, height: number): Promise<void> {
  await app.evaluate(
    async ({ BrowserWindow }, size) => {
      const window = BrowserWindow.getAllWindows()[0];
      window.setSize(size.width, size.height);
    },
    { width, height }
  );
  await page.setViewportSize({ width, height });
  await expectVisibleWorkspaceRegions(page);

  const boxes = {
    top: await expectStableBox(page.locator('[aria-label="顶部功能区"]').first(), `顶部功能区 ${width}x${height}`),
    left: await expectStableBox(page.locator('[aria-label="素材面板"]'), `素材面板 ${width}x${height}`),
    preview: await expectStableBox(page.locator('[aria-label="预览窗口"]'), `预览窗口 ${width}x${height}`),
    inspector: await expectStableBox(page.locator('[aria-label="属性检查器"]'), `属性检查器 ${width}x${height}`),
    timeline: await expectStableBox(page.locator('[aria-label="时间线"]'), `时间线 ${width}x${height}`)
  };

  for (const [name, box] of Object.entries(boxes)) {
    expect(box.x, `${name} left clipped`).toBeGreaterThanOrEqual(0);
    expect(box.y, `${name} top clipped`).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width, `${name} right clipped`).toBeLessThanOrEqual(width + 1);
    expect(box.y + box.height, `${name} bottom clipped`).toBeLessThanOrEqual(height + 1);
  }

  expectNoOverlap(boxes.left, boxes.preview, "素材面板", "预览窗口");
  expectNoOverlap(boxes.preview, boxes.inspector, "预览窗口", "属性检查器");
  expectNoOverlap(boxes.left, boxes.timeline, "素材面板", "时间线");
  expectNoOverlap(boxes.preview, boxes.timeline, "预览窗口", "时间线");
  expectNoOverlap(boxes.inspector, boxes.timeline, "属性检查器", "时间线");
}

async function expectStableBox(locator: Locator, label: string): Promise<RegionBox> {
  await expect(locator, `${label} visible`).toBeVisible();
  const box = await locator.boundingBox();
  expect(box, `${label} bounding box`).not.toBeNull();
  expect(box!.width, `${label} width`).toBeGreaterThan(0);
  expect(box!.height, `${label} height`).toBeGreaterThan(0);
  return box!;
}

function expectNoOverlap(first: RegionBox, second: RegionBox, firstName: string, secondName: string): void {
  const separated =
    first.x + first.width <= second.x ||
    second.x + second.width <= first.x ||
    first.y + first.height <= second.y ||
    second.y + second.height <= first.y;

  expect(separated, `${firstName} must not overlap ${secondName}`).toBe(true);
}

function expectSameSize(before: RegionBox, after: RegionBox, label: string): void {
  expect(Math.abs(before.width - after.width), `${label} width stable`).toBeLessThanOrEqual(1);
  expect(Math.abs(before.height - after.height), `${label} height stable`).toBeLessThanOrEqual(1);
}

test("Chinese editor workspace opens with required regions and material states", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await expectVisibleWorkspaceRegions(page);

    for (const category of ["媒体", "音频", "文字", "贴纸", "特效", "转场", "滤镜", "调节"]) {
      await expect(page.getByRole("button", { name: category })).toBeVisible();
    }

    await expect(page.getByText("预览将在下一阶段接入")).toBeVisible();
    await expect(page.getByText("未选择片段")).toBeVisible();

    await expect(page.getByRole("article", { name: "素材 城市街景.mp4" })).toContainText("视频");
    await expect(page.getByRole("article", { name: "素材 背景音乐.wav" })).toContainText("音频");
    await expect(page.getByRole("article", { name: "素材 封面图.png" })).toContainText("图片");
    await expect(page.getByRole("article", { name: "素材 城市街景.mp4" })).toContainText("可用");
    await expect(page.getByRole("article", { name: "素材 封面图.png" })).toContainText("素材丢失");
    await expect(page.getByRole("article", { name: "素材 贴纸素材.webp" })).toContainText("解析失败");
  } finally {
    await app.close();
  }
});

test("workspace panels switch categories without losing Chinese empty states", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await page.getByRole("button", { name: "文字" }).click();
    await expect(page.getByRole("heading", { name: "文字" })).toBeVisible();
    await expect(page.getByRole("button", { name: "添加文字" })).toBeVisible();
    await expect(page.getByLabel("文字对齐")).toBeVisible();

    await page.getByRole("button", { name: "音频" }).click();
    await expect(page.getByRole("heading", { name: "音频" })).toBeVisible();
    await expect(page.getByRole("button", { name: "添加音频" })).toBeVisible();
    await expect(page.getByText("音量与静音")).toBeVisible();

    for (const category of ["贴纸", "特效", "转场", "滤镜", "调节"]) {
      await page.getByRole("button", { name: category }).click();
      await expect(page.getByRole("heading", { name: category })).toBeVisible();
      await expect(page.getByText(`${category}面板已预留`)).toBeVisible();
      await expect(page.getByText(`当前阶段暂不提供${category}编辑，后续会通过剪辑核心命令接入对应能力。`)).toBeVisible();
      await expect(page.locator('[aria-label="素材面板"]')).toBeVisible();
    }
  } finally {
    await app.close();
  }
});

test("command-only timeline edit calls generated command and applies Rust response", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    const videoSegment = page.getByRole("button", { name: /片段 城市街景\.mp4/ });
    await videoSegment.click();
    await expectCommandCall(app, "selectTimelineSegments");
    await expect(page.getByText("片段ID")).toBeVisible();
    await expect(page.getByText("segment-main-video")).toBeVisible();

    await expect(page.getByRole("button", { name: /片段 城市街景\.mp4/ })).toHaveCount(1);
    await page.getByRole("button", { name: "添加片段" }).click();
    await expectCommandCall(app, "addSegment");
    await expect(page.getByRole("button", { name: /片段 城市街景\.mp4/ })).toHaveCount(2);
    await expect(page.locator('[aria-label="时间线"]')).toContainText("00:00:08.000 / 00:00:12.000");

    const calls = await readExecuteCommandCalls(app);
    expect(calls.map((call) => call.kind)).toEqual(expect.arrayContaining(["selectTimelineSegments", "addSegment"]));
  } finally {
    await app.close();
  }
});

test("layout stability keeps workspace regions visible and fixed at required sizes", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await setViewportSizeAndVerifyLayout(app, page, 1280, 800);
    await setViewportSizeAndVerifyLayout(app, page, 1120, 720);

    const previewBefore = await expectStableBox(page.locator('[aria-label="预览窗口"]'), "预览窗口 before state changes");
    const timelineBefore = await expectStableBox(page.locator('[aria-label="时间线"]'), "时间线 before state changes");
    const inspectorBefore = await expectStableBox(page.locator('[aria-label="属性检查器"]'), "属性检查器 before state changes");

    await page.getByRole("button", { name: /片段 城市街景\.mp4/ }).hover();
    await page.getByRole("button", { name: /片段 城市街景\.mp4/ }).click();
    await page.getByLabel("播放头").fill("1200000");
    await expect(page.getByLabel("播放头")).toHaveValue("1200000");

    const previewAfter = await expectStableBox(page.locator('[aria-label="预览窗口"]'), "预览窗口 after state changes");
    const timelineAfter = await expectStableBox(page.locator('[aria-label="时间线"]'), "时间线 after state changes");
    const inspectorAfter = await expectStableBox(page.locator('[aria-label="属性检查器"]'), "属性检查器 after state changes");

    expectSameSize(previewBefore, previewAfter, "预览窗口");
    expectSameSize(timelineBefore, timelineAfter, "时间线");
    expectSameSize(inspectorBefore, inspectorAfter, "属性检查器");
  } finally {
    await app.close();
  }
});

import { _electron as electron, expect, test, type ElectronApplication, type Locator, type Page } from "@playwright/test";
import { mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import type { CommandName } from "../src/generated/CommandEnvelope";

type ExecuteCommandCall = {
  command: CommandName;
  kind: string;
  requestId: string | null;
  targetTime: number | null;
  targetTimerange: { start: number; duration: number } | null;
  canvasConfig: {
    width: number;
    height: number;
    frameRate: { numerator: number; denominator: number };
  } | null;
  outputPath: string | null;
  preset: string | null;
  jobId: string | null;
};

type RegionBox = {
  x: number;
  y: number;
  width: number;
  height: number;
};

const WORKSPACE_CATEGORIES = ["媒体", "音频", "文字", "贴纸", "特效", "转场", "字幕", "滤镜", "调节", "模板", "数字人"] as const;
const DEFERRED_CATEGORIES = ["贴纸", "特效", "转场", "字幕", "滤镜", "调节", "模板", "数字人"] as const;
const REPO_ROOT = join(process.cwd(), "../..");
const PHASE5_SCREENSHOT_DIR = join(REPO_ROOT, "test-results/phase5");
const PHASE7_SCREENSHOT_DIR = join(REPO_ROOT, "test-results/phase7");

type VideoEditorCoreApi = {
  executeCommand: (command: unknown) => Promise<unknown>;
};

declare global {
  interface Window {
    videoEditorCore?: VideoEditorCoreApi;
  }
}

async function launchWorkspaceApp(
  options: { mockPreviewCommands?: boolean; mockExportCommands?: boolean; env?: NodeJS.ProcessEnv } = {}
): Promise<{ app: ElectronApplication; page: Page }> {
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")],
    env: {
      ...process.env,
      VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS: options.mockPreviewCommands === false ? "0" : "1",
      VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS: options.mockExportCommands === false ? "0" : "1",
      ...options.env
    }
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
  await expectTimelineControlsInsideStrip(page, `时间线控制 ${width}x${height}`);
}

async function expectProfessionalWorkspaceAtViewport(
  page: Page,
  app: ElectronApplication,
  width: number,
  height: number
): Promise<void> {
  await setViewportSizeAndVerifyLayout(app, page, width, height);
  await expectNoCategoryLabelWrap(page);
  await expectPreviewCanvasAspectRatio(page);
  await expectIconButtonsHaveAccessibleNames(page);
  await expectTimelineInputsFit(page);
  await expectPreviewControlsFit(page, `预览控制 ${width}x${height}`);
}

async function savePhase5PreviewScreenshot(page: Page, filename: string): Promise<void> {
  mkdirSync(PHASE5_SCREENSHOT_DIR, { recursive: true });
  await page.screenshot({ path: join(PHASE5_SCREENSHOT_DIR, filename), fullPage: true });
}

async function savePhase7CanvasScreenshot(page: Page, filename: string): Promise<void> {
  mkdirSync(PHASE7_SCREENSHOT_DIR, { recursive: true });
  await page.screenshot({ path: join(PHASE7_SCREENSHOT_DIR, filename), fullPage: true });
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
  const dividerTolerance = 1;
  const separated =
    first.x + first.width <= second.x + dividerTolerance ||
    second.x + second.width <= first.x + dividerTolerance ||
    first.y + first.height <= second.y + dividerTolerance ||
    second.y + second.height <= first.y + dividerTolerance;

  expect(separated, `${firstName} must not overlap ${secondName}`).toBe(true);
}

function expectSameSize(before: RegionBox, after: RegionBox, label: string): void {
  expect(Math.abs(before.width - after.width), `${label} width stable`).toBeLessThanOrEqual(1);
  expect(Math.abs(before.height - after.height), `${label} height stable`).toBeLessThanOrEqual(1);
}

async function expectTimelineControlsInsideStrip(page: Page, label: string): Promise<void> {
  const clippedControls = await page.locator('[aria-label="时间线控制"]').evaluate((strip) => {
    const stripBox = strip.getBoundingClientRect();
    return Array.from(strip.children)
      .map((child) => {
        const box = child.getBoundingClientRect();
        const style = window.getComputedStyle(child);
        return {
          label: child.textContent?.replace(/\s+/g, " ").trim() || child.getAttribute("aria-label") || child.tagName,
          visible: style.display !== "none" && style.visibility !== "hidden" && box.width > 0 && box.height > 0,
          left: box.left,
          top: box.top,
          right: box.right,
          bottom: box.bottom
        };
      })
      .filter(
        (box) =>
          box.visible &&
          (box.left < stripBox.left - 1 ||
            box.top < stripBox.top - 1 ||
            box.right > stripBox.right + 1 ||
            box.bottom > stripBox.bottom + 1)
      );
  });

  expect(clippedControls, `${label} controls clipped`).toEqual([]);
}

async function expectNoCategoryLabelWrap(page: Page): Promise<void> {
  const wrappedLabels = await page.locator(".category-button").evaluateAll((buttons) =>
    buttons
      .map((button) => {
        const label = button.querySelector(".category-label");
        const labelBox = label?.getBoundingClientRect();
        const buttonBox = button.getBoundingClientRect();
        const computed = label ? window.getComputedStyle(label) : null;
        const lineHeight = computed === null ? 16 : Number.parseFloat(computed.lineHeight);

        return {
          text: label?.textContent?.trim() ?? button.textContent?.trim() ?? button.getAttribute("aria-label") ?? "未知分类",
          wraps:
            labelBox === undefined ||
            labelBox.height > lineHeight * 1.35 ||
            labelBox.width > buttonBox.width - 4 ||
            buttonBox.height > 42
        };
      })
      .filter((item) => item.wraps)
  );

  expect(wrappedLabels, "顶部分类标签不能换行或溢出").toEqual([]);
}

async function expectPreviewCanvasAspectRatio(page: Page): Promise<void> {
  const canvas = await expectStableBox(page.locator(".preview-canvas"), "预览画面 16:9");
  const ratio = canvas.width / canvas.height;

  expect(Math.abs(ratio - 16 / 9), "预览画面保持 16:9").toBeLessThanOrEqual(0.04);
}

async function expectPreviewControlsFit(page: Page, label: string): Promise<void> {
  const clippedItems = await page.locator(".preview-shell").evaluate((shell) => {
    const shellBox = shell.getBoundingClientRect();
    return Array.from(
      shell.querySelectorAll(
        ".preview-canvas, .preview-transport, .preview-status-line, .preview-artifact-panel, .export-panel, .export-progress, .export-log, .export-validation, button, input, select, progress"
      )
    )
      .map((element) => {
        const box = element.getBoundingClientRect();
        const style = window.getComputedStyle(element);
        return {
          label: element.getAttribute("aria-label") || element.textContent?.replace(/\s+/g, " ").trim() || element.tagName,
          visible: style.display !== "none" && style.visibility !== "hidden" && box.width > 0 && box.height > 0,
          left: box.left,
          top: box.top,
          right: box.right,
          bottom: box.bottom
        };
      })
      .filter(
        (item) =>
          item.visible &&
          (item.left < shellBox.left - 1 ||
            item.top < shellBox.top - 1 ||
            item.right > shellBox.right + 1 ||
            item.bottom > shellBox.bottom + 1)
      );
  });

  expect(clippedItems, `${label} must stay inside preview shell`).toEqual([]);
}

function expectCompactScrollbarBaseline(): void {
  const source = readFileSync(join(process.cwd(), "src/renderer/styles.css"), "utf8");

  expect(source, "全局滚动条应保持紧凑深色基线").toContain("scrollbar-width: thin");
  expect(source, "全局滚动条应保持 webkit 深色滑块").toContain("::-webkit-scrollbar-thumb");
  expect(source, "滚动条宽度不能回退到默认宽度").toMatch(/::-webkit-scrollbar\s*\{[\s\S]*?width:\s*4px/);
}

async function expectNoLeftSecondaryMenu(page: Page): Promise<void> {
  await expect(page.locator(".secondary-category-rail")).toHaveCount(0);
  await expect(page.locator(".secondary-category-button")).toHaveCount(0);
  for (const category of WORKSPACE_CATEGORIES) {
    await expect(page.getByRole("navigation", { name: `${category}二级分类` })).toHaveCount(0);
  }
}

async function expectIconButtonsHaveAccessibleNames(page: Page): Promise<void> {
  const selector = [
    ".category-button",
    ".preview-icon-button",
    ".transport-button.icon-only",
    ".track-state-button",
    ".keyframe-button"
  ].join(",");
  const missingNames = await page.locator(selector).evaluateAll((buttons) =>
    buttons
      .map((button) => {
        const label = button.getAttribute("aria-label")?.trim() ?? "";
        const title = button.getAttribute("title")?.trim() ?? "";
        const hasChineseName = /[\u4e00-\u9fff]/.test(label) && /[\u4e00-\u9fff]/.test(title);

        return {
          className: button.getAttribute("class") ?? "",
          text: button.textContent?.replace(/\s+/g, " ").trim() ?? "",
          label,
          title,
          hasChineseName
        };
      })
      .filter((item) => !item.hasChineseName)
  );

  expect(missingNames, "图标/紧凑按钮需要中文 aria-label 和 title").toEqual([]);
}

async function expectTimelineInputsFit(page: Page): Promise<void> {
  const clippedInputs = await page.locator(".timeline-control input, .playhead-control input").evaluateAll((inputs) =>
    inputs
      .map((input) => {
        const element = input as HTMLInputElement;
        return {
          label: element.getAttribute("aria-label") ?? element.closest("label")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
          value: element.value,
          clientWidth: element.clientWidth,
          scrollWidth: element.scrollWidth
        };
      })
      .filter((item) => item.scrollWidth > item.clientWidth + 1)
  );

  expect(clippedInputs, "时间线数字输入不能裁切当前数值").toEqual([]);
}

test("Chinese editor workspace opens with required regions and material states", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await expectVisibleWorkspaceRegions(page);

    const topFeatureNav = page.getByRole("navigation", { name: "顶部功能区" });

    for (const category of WORKSPACE_CATEGORIES) {
      await expect(topFeatureNav.getByRole("button", { name: category })).toBeVisible();
    }
    await expectNoCategoryLabelWrap(page);
    await expectIconButtonsHaveAccessibleNames(page);
    await expectNoLeftSecondaryMenu(page);

    await expect(page.getByRole("button", { name: "导入素材" })).toBeVisible();
    await expect(page.getByRole("navigation", { name: "资源分类" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "刷新" })).toBeVisible();
    await expect(page.getByRole("button", { name: "检查丢失" })).toBeVisible();
    await expect(page.getByPlaceholder("搜索素材")).toBeVisible();
    const materialFilters = page.getByRole("group", { name: "素材筛选" });
    for (const filter of ["全部", "视频", "图片", "音频", "丢失"]) {
      await expect(materialFilters.getByRole("button", { name: filter })).toBeVisible();
    }

    await expect(page.getByText("预览命令已接入")).toBeVisible();
    await expect(page.getByText("等待请求预览帧").first()).toBeVisible();
    await expect(page.getByText("预览将在下一阶段接入")).toHaveCount(0);
    await expect(page.getByText("等待生成预览片段")).toBeVisible();
    await expect(page.getByRole("button", { name: "请求预览帧" })).toBeVisible();
    await expect(page.getByRole("button", { name: "生成预览片段" })).toBeVisible();
    await expect(page.getByLabel("预览时间")).toBeVisible();
    await expect(page.getByRole("button", { name: "适应窗口" })).toBeVisible();
    await expect(page.getByRole("button", { name: "画面比例" })).toBeVisible();
    await expect(page.getByRole("button", { name: "全屏" })).toBeVisible();
    await expectPreviewCanvasAspectRatio(page);

    await expect(page.getByText("未选择片段")).toBeVisible();
    await expect(page.getByRole("tab", { name: "画面" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "音频" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "变速" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "动画" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "调节" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "AI效果" })).toBeVisible();
    await expect(page.getByLabel("草稿参数")).toContainText("草稿参数");

    await expect(page.getByRole("article", { name: "素材 城市街景.mp4" })).toContainText("视频");
    await expect(page.getByRole("article", { name: "素材 背景音乐.wav" })).toContainText("音频");
    await expect(page.getByRole("article", { name: "素材 封面图.png" })).toContainText("图片");
    await expect(page.getByRole("article", { name: "素材 城市街景.mp4" })).toContainText("可用");
    await expect(page.getByRole("article", { name: "素材 封面图.png" })).toContainText("素材丢失");
    await expect(page.getByRole("article", { name: "素材 贴纸素材.webp" })).toContainText("解析失败");
    await materialFilters.getByRole("button", { name: "丢失" }).click();
    await expect(page.getByRole("article", { name: "素材 封面图.png" })).toContainText("素材丢失");
    await expect(page.getByRole("article", { name: "素材 贴纸素材.webp" })).toContainText("解析失败");
  } finally {
    await app.close();
  }
});

test("workspace panels switch categories without losing Chinese empty states", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    const topFeatureNav = page.getByRole("navigation", { name: "顶部功能区" });

    await topFeatureNav.getByRole("button", { name: "文字" }).click();
    await expect(page.getByRole("heading", { name: "文字" })).toBeVisible();
    await expectNoLeftSecondaryMenu(page);
    await expect(page.getByRole("button", { name: "添加文字" })).toBeVisible();
    await expect(page.getByLabel("文字对齐")).toBeVisible();

    await topFeatureNav.getByRole("button", { name: "音频" }).click();
    await expect(page.getByRole("heading", { name: "音频" })).toBeVisible();
    await expectNoLeftSecondaryMenu(page);
    await expect(page.getByRole("button", { name: "添加音频" })).toBeVisible();
    await expect(page.getByText("音量与静音")).toBeVisible();

    for (const category of DEFERRED_CATEGORIES) {
      await topFeatureNav.getByRole("button", { name: category }).click();
      await expect(page.getByRole("heading", { name: category })).toBeVisible();
      await expectNoLeftSecondaryMenu(page);
      await expect(page.getByText(`${category}功能已预留`)).toBeVisible();
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
    await expect(page.getByLabel("片段信息")).toContainText("片段参数");
    await expect(page.getByLabel("画面变换")).toContainText("位置");
    await expect(page.getByRole("button", { name: "关键帧功能待接入" })).toHaveCount(3);

    await page.getByRole("tab", { name: "音频" }).click();
    await expect(page.getByLabel("音频参数")).toContainText("应用音量");
    await expect(page.getByLabel("画面变换")).toHaveCount(0);
    await page.getByRole("tab", { name: "画面" }).click();
    await expect(page.getByLabel("画面变换")).toContainText("位置");

    await expect(page.getByRole("button", { name: /片段 城市街景\.mp4/ })).toHaveCount(1);
    const callsBeforeAdd = await readExecuteCommandCalls(app);
    await page.getByRole("button", { name: "添加片段" }).evaluate((button) => {
      (button as HTMLButtonElement).click();
      (button as HTMLButtonElement).click();
    });
    await expectCommandCall(app, "addSegment");
    await expect(page.getByRole("button", { name: /片段 城市街景\.mp4/ })).toHaveCount(2);
    await expect(page.locator('[aria-label="时间线"]')).toContainText("00:00:08.000 / 00:00:12.000");

    const calls = await readExecuteCommandCalls(app);
    const addSegmentCallsBefore = callsBeforeAdd.filter((call) => call.command === "addSegment").length;
    const addSegmentCallsAfter = calls.filter((call) => call.command === "addSegment").length;
    expect(addSegmentCallsAfter - addSegmentCallsBefore).toBe(1);
    expect(calls.map((call) => call.kind)).toEqual(expect.arrayContaining(["selectTimelineSegments", "addSegment"]));
  } finally {
    await app.close();
  }
});

test("material import routes through the same executeCommand bridge", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    await page.getByRole("button", { name: "导入素材" }).click();
    await expectCommandCall(app, "importMaterial");

    const calls = await readExecuteCommandCalls(app);
    expect(calls.map((call) => call.command)).toContain("importMaterial");
  } finally {
    await app.close();
  }
});

test("预览命令通过 executeCommand 更新帧和片段状态", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    await page.getByLabel("预览时间").fill("1200000");
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:01.200");

    await page.getByRole("button", { name: "请求预览帧" }).click();
    await expectCommandCall(app, "requestPreviewFrame");
    await expect(page.getByLabel("预览产物")).toContainText("预览帧已生成");
    await expect(page.getByLabel("预览产物")).toContainText("image/png");
    await expect(page.getByLabel("预览画面")).toContainText("/tmp/video-editor-preview-cache/test-frame-1200000.png");

    await page.getByRole("button", { name: "生成预览片段" }).click();
    await expectCommandCall(app, "requestPreviewSegment");
    await expect(page.getByLabel("预览产物")).toContainText("预览片段命中缓存");
    await expect(page.getByLabel("预览产物")).toContainText("video/mp4");
    await expect(page.getByLabel("预览产物")).toContainText("/tmp/video-editor-preview-cache/test-segment-1200000.mp4");

    const calls = await readExecuteCommandCalls(app);
    const frameCall = calls.find((call) => call.command === "requestPreviewFrame");
    const segmentCall = calls.find((call) => call.command === "requestPreviewSegment");
    expect(frameCall?.targetTime).toBe(1_200_000);
    expect(segmentCall?.targetTimerange).toEqual({ start: 1_200_000, duration: 2_000_000 });
  } finally {
    await app.close();
  }
});

test("草稿参数画布 UI 通过 Rust command 更新预览读数并保存截图", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);
    await expectNoLeftSecondaryMenu(page);

    const inspector = page.getByLabel("草稿参数");
    await expect(inspector).toContainText("草稿参数");
    for (const label of ["画布比例", "画布尺寸", "帧率", "画布背景", "黑色", "纯色", "模糊填充", "图片背景", "未接入"]) {
      await expect(inspector).toContainText(label);
    }
    await expect(inspector.getByRole("button", { name: "应用草稿参数" })).toBeDisabled();
    await expect(page.getByText("坐标以画布中心为原点")).toBeVisible();
    await expect(page.getByRole("button", { name: "图片背景未接入" })).toBeDisabled();

    await inspector.getByRole("group", { name: "画布比例" }).getByRole("button", { name: "9:16" }).click();
    await expect(page.getByLabel("画布宽度")).toHaveValue("1080");
    await expect(page.getByLabel("画布高度")).toHaveValue("1920");
    await inspector.getByRole("group", { name: "画布背景" }).getByRole("button", { name: "模糊填充" }).click();
    await expect(inspector).toContainText("模糊填充 · 降级");
    await inspector.getByRole("button", { name: "应用草稿参数" }).click();

    await expectCommandCall(app, "updateDraftCanvasConfig");
    await expect(page.getByText("画布 9:16 · 1080 x 1920 · 30 fps")).toBeVisible();
    await expect(page.getByText("模糊填充 · 降级").first()).toBeVisible();

    const calls = await readExecuteCommandCalls(app);
    const canvasCall = calls.find((call) => call.command === "updateDraftCanvasConfig");
    expect(canvasCall?.canvasConfig).toMatchObject({
      width: 1080,
      height: 1920,
      frameRate: { numerator: 30, denominator: 1 }
    });

    await setViewportSizeAndVerifyLayout(app, page, 1280, 800);
    await expectCompactScrollbarBaseline();
    await expectNoLeftSecondaryMenu(page);
    await savePhase7CanvasScreenshot(page, "canvas-1280x800.png");

    await setViewportSizeAndVerifyLayout(app, page, 1120, 720);
    await expectCompactScrollbarBaseline();
    await expectNoLeftSecondaryMenu(page);
    await savePhase7CanvasScreenshot(page, "canvas-1120x720.png");
  } finally {
    await app.close();
  }
});

test("预览失败显示中文分类错误且不改草稿", async () => {
  const { app, page } = await launchWorkspaceApp({
    mockPreviewCommands: false,
    env: {
      VE_FFMPEG_PATH: "/tmp/video-editor-missing-ffmpeg",
      VE_FFPROBE_PATH: "/tmp/video-editor-missing-ffprobe"
    }
  });

  try {
    await spyExecuteCommandCalls(app, page);

    await page.getByRole("button", { name: "请求预览帧" }).click();
    await expectCommandCall(app, "requestPreviewFrame");
    await expect(page.getByLabel("预览状态")).toContainText("请求预览帧失败");
    await expect(page.getByLabel("预览状态")).toContainText("预览服务失败");
    await expect(page.getByRole("button", { name: /片段 城市街景\.mp4/ })).toHaveCount(1);
    await expect(page.getByLabel("预览产物")).toContainText("预览帧失败");
  } finally {
    await app.close();
  }
});

test("concurrent material commands are blocked while a timeline edit is pending", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    await expect(page.getByRole("button", { name: /片段 城市街景\.mp4/ })).toHaveCount(1);
    await page.evaluate(() => {
      const findButton = (label: string): HTMLButtonElement => {
        const button = Array.from(document.querySelectorAll("button")).find(
          (candidate) => candidate.textContent?.trim() === label
        );

        if (!(button instanceof HTMLButtonElement)) {
          throw new Error(`找不到按钮：${label}`);
        }

        return button;
      };

      findButton("添加片段").click();
      findButton("导入素材").click();
    });

    await expectCommandCall(app, "addSegment");
    await expect(page.getByRole("button", { name: /片段 城市街景\.mp4/ })).toHaveCount(2);

    const draftMutatingCalls = (await readExecuteCommandCalls(app)).filter(
      (call) => call.command === "addSegment" || call.command === "importMaterial"
    );
    expect(draftMutatingCalls.map((call) => call.command)).toEqual(["addSegment"]);
  } finally {
    await app.close();
  }
});

test("layout stability keeps workspace regions visible and fixed at required sizes", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await expectProfessionalWorkspaceAtViewport(page, app, 1280, 800);
    await expectProfessionalWorkspaceAtViewport(page, app, 1120, 720);

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

test("预览区域在 1280x800 和 1120x720 保持比例并保存截图", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await expectProfessionalWorkspaceAtViewport(page, app, 1280, 800);
    await expectCompactScrollbarBaseline();
    await savePhase5PreviewScreenshot(page, "preview-1280x800.png");

    await expectProfessionalWorkspaceAtViewport(page, app, 1120, 720);
    await expectCompactScrollbarBaseline();
    await savePhase5PreviewScreenshot(page, "preview-1120x720.png");

    await expect(page.getByRole("button", { name: "请求预览帧" })).toBeVisible();
    await expect(page.getByRole("button", { name: "生成预览片段" })).toBeVisible();
    await expect(page.getByLabel("预览产物")).toBeVisible();
  } finally {
    await app.close();
  }
});

test("导出命令通过 executeCommand 更新导出状态并保存截图", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    await expect(page.getByLabel("导出面板")).toBeVisible();
    await expect(page.getByLabel("输出路径")).toHaveValue("/tmp/video-editor-export.mp4");
    await expect(page.getByLabel("导出预设")).toHaveValue("h264AacBalanced");
    await expect(page.getByRole("button", { name: "开始导出" })).toBeVisible();
    await expect(page.getByRole("button", { name: "查询导出状态" })).toBeDisabled();
    await expect(page.getByRole("button", { name: "取消导出" })).toBeDisabled();

    await page.getByLabel("输出路径").fill("/tmp/video-editor-export.mp4");
    await page.getByRole("button", { name: "开始导出" }).click();
    await expectCommandCall(app, "startExport");
    await expect(page.getByLabel("导出进度")).toContainText("导出中");
    await expect(page.getByLabel("导出进度")).toContainText("12%");
    await expect(page.getByLabel("导出日志")).toContainText("导出任务已启动");
    await expect(page.getByRole("button", { name: "取消导出" })).toBeEnabled();

    await page.getByRole("button", { name: "取消导出" }).click();
    await expectCommandCall(app, "cancelExport");
    await expect(page.getByLabel("导出进度")).toContainText("已取消");
    await expect(page.getByLabel("导出日志")).toContainText("导出已取消");

    await page.getByRole("button", { name: "开始导出" }).click();
    await page.getByRole("button", { name: "查询导出状态" }).click();
    await expectCommandCall(app, "getExportJobStatus");
    await expect(page.getByLabel("导出进度")).toContainText("已完成");
    await expect(page.getByLabel("导出进度")).toContainText("100%");
    await expect(page.getByLabel("导出日志")).toContainText("导出完成，输出校验通过");
    await expect(page.getByLabel("输出校验")).toContainText("1920x1080");
    await expect(page.getByLabel("输出校验")).toContainText("含音频");

    const calls = await readExecuteCommandCalls(app);
    expect(calls.map((call) => call.command)).toEqual(
      expect.arrayContaining(["startExport", "cancelExport", "getExportJobStatus"])
    );
    const startCall = calls.find((call) => call.command === "startExport");
    expect(startCall?.outputPath).toBe("/tmp/video-editor-export.mp4");
    expect(startCall?.preset).toBe("h264AacBalanced");

    await expectProfessionalWorkspaceAtViewport(page, app, 1280, 800);
    await expectCompactScrollbarBaseline();
    await savePhase5PreviewScreenshot(page, "export-1280x800.png");

    await expectProfessionalWorkspaceAtViewport(page, app, 1120, 720);
    await expectCompactScrollbarBaseline();
    await savePhase5PreviewScreenshot(page, "export-1120x720.png");
  } finally {
    await app.close();
  }
});

test("professional timeline exposes stable toolbar, track, segment, ruler, zoom, and snapping states", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await expectProfessionalWorkspaceAtViewport(page, app, 1280, 800);

    const timelineControls = page.getByLabel("时间线控制");
    for (const label of [
      "撤销",
      "重做",
      "播放",
      "停止",
      "左移所选片段",
      "右移所选片段",
      "分割所选片段",
      "左侧裁剪",
      "右侧裁剪",
      "删除所选片段",
      "缩小时间线",
      "放大时间线"
    ]) {
      await expect(timelineControls.getByRole("button", { name: label })).toBeVisible();
    }

    await expect(page.getByLabel("时间线标尺")).toContainText("00:00");
    await expect(page.getByLabel("时间线缩放", { exact: true })).toContainText("100%");
    await expect(page.locator(".snapping-status")).toHaveAttribute("aria-label", /吸附/);
    await expect(page.locator(".playhead")).toBeVisible();
    await expect(page.locator(".track-state-button")).toHaveCount(9);
    await expect(page.locator(".segment-kind-video")).toHaveCount(1);
    await expect(page.locator(".segment-kind-audio")).toHaveCount(1);

    await spyExecuteCommandCalls(app, page);
    await page.getByRole("button", { name: "音频轨道 1 静音状态：未静音" }).click();
    await expectCommandCall(app, "setTrackMute");
    await expect(page.getByRole("button", { name: "音频轨道 1 静音状态：已静音" })).toBeVisible();

    await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "文字" }).click();
    await page.getByRole("button", { name: "添加文字" }).click();
    await expect(page.locator(".segment-kind-text")).toHaveCount(1);

    const firstSegment = page.getByRole("button", { name: /片段 城市街景\.mp4/ });
    const before = await expectStableBox(firstSegment, "片段 hover 前");
    await firstSegment.hover();
    const afterHover = await expectStableBox(firstSegment, "片段 hover 后");
    await firstSegment.click();
    const afterSelection = await expectStableBox(firstSegment, "片段 selection 后");

    expectSameSize(before, afterHover, "片段 hover");
    expectSameSize(before, afterSelection, "片段 selection");
  } finally {
    await app.close();
  }
});

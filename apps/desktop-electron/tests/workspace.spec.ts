import { _electron as electron, expect, test, type ElectronApplication, type Locator, type Page } from "@playwright/test";
import { mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import type { CommandName } from "../src/generated/CommandEnvelope";
import type { Keyframe, SegmentVisual } from "../src/generated/Draft";
import {
  formatRealtimePreviewBackendLabel,
  formatRealtimePreviewFallbackReason,
  summarizeRealtimePreviewDisplay,
  type RealtimePreviewDisplayModel
} from "../src/renderer/viewModel";

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
  visual: SegmentVisual | null;
  keyframe: Keyframe | null;
  keyframeProperty: string | null;
  keyframeAt: number | null;
  textContent: string | null;
  textSource: string | null;
  srtContent: string | null;
  outputPath: string | null;
  preset: string | null;
  jobId: string | null;
};

type RealtimePreviewHostCall = {
  kind: string;
  parentHandleByteLength?: number;
  surfaceKind?: string;
  bounds?: {
    x: number;
    y: number;
    width: number;
    height: number;
    scaleFactorMillis: number;
  };
  targetTimeMicroseconds?: number;
  playbackGeneration?: number;
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

type VideoEditorRealtimePreviewHostApi = {
  updateHostRect: (rect: {
    x: number;
    y: number;
    width: number;
    height: number;
    scaleFactorMillis: number;
  }) => Promise<unknown>;
  getTelemetry: () => Promise<unknown>;
  updateDraftSnapshot: (draft: unknown) => Promise<unknown>;
  seek: (targetTimeMicroseconds: number) => Promise<unknown>;
  play: () => Promise<unknown>;
  pause: () => Promise<unknown>;
  stop: () => Promise<unknown>;
};

declare global {
  interface Window {
    videoEditorCore?: VideoEditorCoreApi;
    videoEditorRealtimePreviewHost?: VideoEditorRealtimePreviewHostApi;
  }
}

async function launchWorkspaceApp(
  options: {
    mockPreviewCommands?: boolean;
    mockExportCommands?: boolean;
    mockArtifactCommands?: boolean;
    mockAudioCommands?: boolean;
    showDeveloperDiagnostics?: boolean;
    env?: NodeJS.ProcessEnv;
  } = {}
): Promise<{ app: ElectronApplication; page: Page }> {
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")],
    env: {
      ...process.env,
      VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
      VIDEO_EDITOR_TEST_WORKSPACE_FIXTURE: "demo",
      VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS: options.mockPreviewCommands === false ? "0" : "1",
      VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS: options.mockExportCommands === false ? "0" : "1",
      VIDEO_EDITOR_TEST_MOCK_ARTIFACT_COMMANDS: options.mockArtifactCommands === false ? "0" : "1",
      VIDEO_EDITOR_TEST_MOCK_AUDIO_COMMANDS: options.mockAudioCommands === false ? "0" : "1",
      VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: options.showDeveloperDiagnostics === true ? "1" : "0",
      VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify(["/tmp/demo-material.mp4"]),
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

async function readRealtimePreviewHostCalls(app: ElectronApplication): Promise<RealtimePreviewHostCall[]> {
  return app.evaluate(() => {
    return (
      (globalThis as typeof globalThis & { __videoEditorTestRealtimePreviewHostCalls?: RealtimePreviewHostCall[] })
        .__videoEditorTestRealtimePreviewHostCalls ?? []
    );
  });
}

async function expectCommandCall(app: ElectronApplication, command: CommandName): Promise<void> {
  await expect
    .poll(async () => (await readExecuteCommandCalls(app)).some((call) => call.command === command))
    .toBe(true);
}

async function expectLatestPreviewFrameTarget(app: ElectronApplication, targetTime: number): Promise<void> {
  await expect
    .poll(async () => {
      const calls = (await readExecuteCommandCalls(app)).filter((call) => call.command === "requestPreviewFrame");
      return calls.at(-1)?.targetTime ?? null;
    })
    .toBe(targetTime);
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

async function expectNativePreviewHostLayout(
  app: ElectronApplication,
  page: Page,
  width: number,
  height: number,
  options: { requireBoundsUpdate?: boolean } = {}
): Promise<RegionBox> {
  await setViewportSizeAndVerifyLayout(app, page, width, height);
  const host = await expectStableBox(page.locator(".preview-native-host"), `实时预览宿主 ${width}x${height}`);
  const timeline = await expectStableBox(page.locator('[aria-label="时间线"]'), `时间线 ${width}x${height}`);
  const inspector = await expectStableBox(page.locator('[aria-label="属性检查器"]'), `属性检查器 ${width}x${height}`);

  expect(host.width, `实时预览宿主宽度 ${width}x${height}`).toBeGreaterThan(120);
  expect(host.height, `实时预览宿主高度 ${width}x${height}`).toBeGreaterThan(80);
  expectNoOverlap(host, timeline, "实时预览宿主", "时间线");
  expectNoOverlap(host, inspector, "实时预览宿主", "属性检查器");
  if (options.requireBoundsUpdate !== false) {
    await latestRealtimePreviewBounds(app);
  }
  return host;
}

async function latestRealtimePreviewBounds(app: ElectronApplication): Promise<NonNullable<RealtimePreviewHostCall["bounds"]>> {
  await expect
    .poll(async () => {
      const latestBounds = (await readRealtimePreviewHostCalls(app)).findLast((call) => call.kind === "updateSurfaceBounds")?.bounds;
      return latestBounds === undefined ? null : latestBounds;
    })
    .not.toBeNull();

  const latestBounds = (await readRealtimePreviewHostCalls(app)).findLast((call) => call.kind === "updateSurfaceBounds")?.bounds;
  expect(latestBounds, "实时预览宿主应上报 bounds").toBeDefined();
  return latestBounds!;
}

async function expectLocatorInsideHorizontalContainer(container: Locator, target: Locator, label: string): Promise<void> {
  await target.scrollIntoViewIfNeeded();
  const containerBox = await expectStableBox(container, `${label} container`);
  const targetBox = await expectStableBox(target, label);

  expect(targetBox.width, `${label} wider than container`).toBeLessThanOrEqual(containerBox.width + 1);
  expect(targetBox.x, `${label} left clipped`).toBeGreaterThanOrEqual(containerBox.x - 1);
  expect(targetBox.x + targetBox.width, `${label} right clipped`).toBeLessThanOrEqual(containerBox.x + containerBox.width + 1);
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
    await expect(page.getByLabel("草稿包路径")).toHaveCount(0);
    await expect(page.getByLabel("素材路径")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "导入路径" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "刷新" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "检查丢失" })).toHaveCount(0);
    await expect(page.getByPlaceholder("搜索素材")).toBeVisible();
    const materialFilters = page.getByRole("group", { name: "素材筛选" });
    for (const filter of ["全部", "视频", "图片", "音频", "丢失"]) {
      await expect(materialFilters.getByRole("button", { name: filter })).toBeVisible();
    }

    await expect(page.getByText("预览命令已接入")).toHaveCount(0);
    await expect(page.getByLabel("预览产物")).toHaveCount(0);
    await expect(page.getByLabel("运行环境诊断")).toHaveCount(0);
    await expect(page.getByText("添加素材到时间线后显示预览").first()).toBeVisible();
    await expect(page.getByText("预览将在下一阶段接入")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "请求预览帧" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "生成预览片段" })).toHaveCount(0);
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
    await expect(page.getByRole("heading", { name: "文字", exact: true })).toBeVisible();
    await expectNoLeftSecondaryMenu(page);
    await expect(page.getByRole("button", { name: "添加文字" })).toBeVisible();
    await expect(page.getByLabel("默认文字").getByText("字号")).toHaveCount(0);
    await expect(page.getByLabel("默认文字").getByText("描边")).toHaveCount(0);

    await topFeatureNav.getByRole("button", { name: "音频" }).click();
    await expect(page.getByRole("heading", { name: "音频", exact: true }).first()).toBeVisible();
    await expectNoLeftSecondaryMenu(page);
    await expect(page.getByRole("button", { name: "添加音频" })).toBeVisible();
    await expect(page.getByText("音量", { exact: true })).toBeVisible();
    await expect(page.getByText("声像", { exact: true })).toBeVisible();
    await expect(page.getByText("淡入", { exact: true })).toBeVisible();
    await expect(page.getByText("淡出", { exact: true })).toBeVisible();

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

test("文字 panel keeps contextual cards, deferred states, compact scrollbars, and no duplicate left primary menu", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "文字" }).click();

    await expectNoLeftSecondaryMenu(page);
    await expectCompactScrollbarBaseline();
    await expect(page.getByLabel("默认文字")).toContainText("默认文字");
    await expect(page.getByLabel("字幕 导入字幕")).toContainText("字幕 / 导入字幕");
    await expect(page.getByLabel("字幕 导入字幕")).toContainText("自动生成字幕片段");
    await expect(page.getByLabel("花字")).toContainText("暂未接入");
    await expect(page.getByLabel("气泡")).toContainText("暂未接入");
    await expect(page.getByRole("button", { name: "添加文字" })).toBeVisible();
    await expect(page.getByRole("button", { name: "导入字幕" })).toBeVisible();

    const resourcePanel = page.getByLabel("素材面板");
    for (const label of ["默认文字", "字幕 导入字幕", "花字", "气泡"]) {
      await expectLocatorInsideHorizontalContainer(resourcePanel, page.getByLabel(label), `文字面板 ${label}`);
    }
  } finally {
    await app.close();
  }
});

test("command-only text edit routes complete text inspector changes through executeCommand", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);
    await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "文字" }).click();
    await page.getByLabel("默认文字").getByLabel("文字内容").fill("开场标题");
    await page.getByRole("button", { name: "添加文字" }).click();
    await expectCommandCall(app, "addTextSegment");

    await expect(page.getByRole("button", { name: /片段 默认文字/ })).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByLabel("预览文字")).toContainText("开场标题");

    for (const section of ["文本", "样式", "文本框", "布局", "花字 / 气泡"]) {
      await expect(page.getByRole("heading", { name: section, exact: true })).toBeVisible();
    }
    const textSection = page.locator('section[aria-label="文本"]');
    const styleSection = page.locator('section[aria-label="样式"]');
    const textBoxSection = page.locator('section[aria-label="文本框"]');
    const layoutSection = page.locator('section[aria-label="布局"]');
    const inspector = page.getByLabel("属性检查器");
    for (const section of [textSection, styleSection, textBoxSection, layoutSection]) {
      await expectLocatorInsideHorizontalContainer(inspector, section, "文字检查器区块");
    }
    await expect(textSection).toContainText("字幕来源");
    await textSection.scrollIntoViewIfNeeded();
    await textSection.locator("textarea").fill("开场标题 已修改");
    await textSection.locator('input[aria-label="字体"]').fill("PingFang SC");
    await textSection.getByRole("spinbutton", { name: "字号", exact: true }).fill("48");
    await textSection.locator('input[aria-label="颜色"]').fill("#18c7ff");
    await styleSection.scrollIntoViewIfNeeded();
    await styleSection.getByRole("checkbox", { name: "描边", exact: true }).check();
    await styleSection.locator('input[aria-label="描边颜色"]').fill("#111111");
    await styleSection.getByRole("spinbutton", { name: "描边宽度", exact: true }).fill("5");
    await styleSection.getByRole("checkbox", { name: "阴影", exact: true }).check();
    await styleSection.locator('input[aria-label="阴影颜色"]').fill("#333333");
    await styleSection.getByRole("checkbox", { name: "背景", exact: true }).check();
    await styleSection.locator('input[aria-label="背景颜色"]').fill("#202020");
    await styleSection.getByRole("button", { name: "右", exact: true }).click();
    await textBoxSection.scrollIntoViewIfNeeded();
    await textBoxSection.getByRole("spinbutton", { name: "行高", exact: true }).fill("1300");
    await textBoxSection.getByRole("spinbutton", { name: "字间距", exact: true }).fill("120");
    await layoutSection.scrollIntoViewIfNeeded();
    await layoutSection.getByRole("spinbutton", { name: "X", exact: true }).fill("120");
    await layoutSection.getByRole("spinbutton", { name: "Y", exact: true }).fill("180");
    await layoutSection.getByRole("spinbutton", { name: "宽", exact: true }).fill("760");
    await layoutSection.getByRole("button", { name: "应用文字" }).click();
    await expectCommandCall(app, "editTextSegment");

    const previewText = page.getByLabel("预览文字");
    await expect(previewText).toContainText("开场标题 已修改");
    await expect(previewText).toHaveCSS("color", "rgb(24, 199, 255)");
    await expect(previewText).toHaveCSS("font-size", "48px");
    await expect(previewText).toHaveCSS("text-align", "right");
    await expect(previewText).toHaveCSS("letter-spacing", "0.12px");
    await expect(previewText).toHaveCSS("background-color", "rgb(32, 32, 32)");
    await expect(page.getByLabel("预览状态", { exact: true })).toContainText("画面已更新，预览待刷新");
    await expect(page.getByLabel("导出日志")).toContainText("文字已更新，请重新开始导出");

    const calls = await readExecuteCommandCalls(app);
    const addTextCall = calls.find((call) => call.command === "addTextSegment");
    const editTextCall = calls.find((call) => call.command === "editTextSegment");
    expect(addTextCall?.textSource).toBe("text");
    expect(addTextCall?.textContent).toBe("开场标题");
    expect(editTextCall?.textContent).toBe("开场标题 已修改");
    expect(calls.filter((call) => call.command === "editTextSegment")).toHaveLength(1);
  } finally {
    await app.close();
  }
});

test("音频 add/volume/mute commands update accepted timeline and inspector state", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "音频" }).click();
    await expect(page.getByRole("heading", { name: "音频", exact: true }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: /片段 背景音乐\.wav/ })).toHaveCount(1);

    await page.getByRole("button", { name: "添加音频" }).click();
    await expectCommandCall(app, "addAudioSegment");
    await expect(page.getByRole("button", { name: /片段 背景音乐\.wav/ })).toHaveCount(2);
    await expect(page.getByRole("button", { name: /片段 背景音乐\.wav/ }).last()).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByLabel("片段信息")).toContainText("音频轨道 1 / 音频");

    await page.getByRole("tab", { name: "音频" }).click();
    await page.getByLabel("音频参数").getByRole("slider", { name: "音量" }).fill("135");
    await page.getByLabel("音频参数").getByRole("slider", { name: "声像" }).fill("-20");
    await page.getByLabel("音频参数").getByRole("spinbutton", { name: "淡入" }).fill("450000");
    await page.getByLabel("音频参数").getByRole("spinbutton", { name: "淡出" }).fill("300000");
    await page.getByLabel("音频参数").getByRole("button", { name: "应用音频" }).click();
    await expectCommandCall(app, "updateSegmentAudio");
    await expect(page.getByLabel("音频参数").getByRole("slider", { name: "音量" })).toHaveValue("135");
    await expect(page.getByLabel("音频参数").getByRole("slider", { name: "声像" })).toHaveValue("-20");

    await page.getByLabel("音频参数").getByRole("checkbox", { name: "轨道静音" }).click();
    await expectCommandCall(app, "setTrackMute");
    await expect(page.getByRole("button", { name: "音频轨道 1 静音状态：已静音" })).toBeVisible();
    await expect(page.getByLabel("音频参数").getByRole("checkbox", { name: "轨道静音" })).toBeChecked();

    const calls = await readExecuteCommandCalls(app);
    expect(calls.map((call) => call.command)).toEqual(
      expect.arrayContaining(["addAudioSegment", "updateSegmentAudio", "setTrackMute"])
    );
  } finally {
    await app.close();
  }
});

test("audio segment blocks expose deterministic P0 waveform placeholder stripe", async () => {
  const { app, page } = await launchWorkspaceApp({ env: { VIDEO_EDITOR_TEST_AUDIO_WAVEFORM_STATUS: "missing" } });

  try {
    const audioSegment = page.getByRole("button", { name: /片段 背景音乐\.wav/ }).first();
    await expect(audioSegment.locator(".audio-waveform-placeholder")).toHaveAttribute("aria-label", "音频波形占位");
    await expect(audioSegment.locator(".audio-waveform-bar")).toHaveCount(12);
    await expect(audioSegment.locator(".audio-waveform-bar").nth(0)).toHaveAttribute("data-height", "short");
    await expect(audioSegment.locator(".audio-waveform-bar").nth(1)).toHaveAttribute("data-height", "medium");
    await expect(audioSegment.locator(".audio-waveform-bar").nth(2)).toHaveAttribute("data-height", "tall");
  } finally {
    await app.close();
  }
});

test("字幕 SRT import command path sends raw SRT once without renderer-created cue segments", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);
    await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "文字" }).click();
    await page.getByLabel("SRT 内容").fill("1\n00:00:00,000 --> 00:00:02,000\n第一句字幕\n\n2\n00:00:02,000 --> 00:00:04,000\n第二句字幕\n");
    await page.getByLabel("字幕时间偏移").fill("1000000");
    await page.getByRole("button", { name: "导入字幕" }).click();
    await expectCommandCall(app, "importSubtitleSrt");

    await expect(page.getByRole("button", { name: /片段 导入字幕/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /片段 导入字幕/ })).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByLabel("预览文字")).toContainText("测试字幕");
    await expect(page.getByLabel("片段信息")).toContainText("字幕 / 文字");
    const textSection = page.locator('section[aria-label="文本"]');
    await expect(textSection.getByRole("heading", { name: "文本", exact: true })).toBeVisible();
    await expect(textSection).toContainText("SRT 字幕");
    await expect(page.getByLabel("预览状态", { exact: true })).toContainText("画面已更新，预览待刷新");

    await textSection.locator("textarea").fill("第一句字幕 已校对");
    await page.getByRole("button", { name: "应用文字" }).click();
    await expectCommandCall(app, "editTextSegment");
    await expect(page.getByLabel("预览文字")).toContainText("第一句字幕 已校对");

    const visualForm = page.getByLabel("画面基础表单");
    await visualForm.getByLabel("位置 X", { exact: true }).fill("80");
    await visualForm.getByRole("button", { name: "应用画面" }).click();
    await expectCommandCall(app, "updateSegmentVisual");

    const calls = await readExecuteCommandCalls(app);
    const importCalls = calls.filter((call) => call.command === "importSubtitleSrt");
    expect(importCalls).toHaveLength(1);
    expect(importCalls[0].srtContent).toContain("第二句字幕");
    expect(importCalls[0].srtContent).toContain("00:00:02,000 --> 00:00:04,000");
    expect(importCalls[0].textSource).toBeNull();
    expect(calls.filter((call) => call.command === "addTextSegment")).toHaveLength(0);
    const editTextCall = calls.find((call) => call.command === "editTextSegment");
    expect(editTextCall?.textSource).toBe("subtitle");
    expect(editTextCall?.textContent).toBe("第一句字幕 已校对");
    expect(calls.find((call) => call.command === "updateSegmentVisual")?.visual?.transform.position.x).toBe(80);
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
    await expect(page.getByLabel("片段信息")).toContainText("片段参数");
    await expect(page.getByLabel("片段信息")).toContainText("城市街景.mp4");
    await expect(page.getByText("片段ID")).toHaveCount(0);
    await expect(page.getByLabel("画面变换")).toContainText("位置");
    await expect(page.getByRole("button", { name: "添加位置 X关键帧" }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: "添加缩放 X关键帧" }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: "添加不透明度关键帧" }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: "文本关键帧需要文字片段" })).toBeDisabled();

    await page.getByRole("tab", { name: "音频" }).click();
    await expect(page.getByLabel("音频参数")).toContainText("应用音频");
    await expect(page.getByRole("button", { name: "添加音量关键帧" }).first()).toBeVisible();
    await expect(page.getByLabel("画面变换")).toHaveCount(0);
    await page.getByRole("tab", { name: "动画" }).click();
    await expect(page.getByLabel("动画参数")).toContainText("还没有关键帧");
    await expect(page.getByLabel("属性关键帧")).toContainText("画面");
    await expect(page.getByLabel("属性关键帧")).toContainText("特效");
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
    await expectLatestPreviewFrameTarget(app, 8_000_000);
    await expect(page.getByRole("img", { name: "当前预览帧" })).toHaveAttribute("src", /test-frame-8000000\.png$/);

    const calls = await readExecuteCommandCalls(app);
    const addSegmentCallsBefore = callsBeforeAdd.filter((call) => call.command === "addSegment").length;
    const addSegmentCallsAfter = calls.filter((call) => call.command === "addSegment").length;
    expect(addSegmentCallsAfter - addSegmentCallsBefore).toBe(1);
    expect(calls.map((call) => call.kind)).toEqual(expect.arrayContaining(["selectTimelineSegments", "addSegment"]));
  } finally {
    await app.close();
  }
});

test("动画 tab and command-only keyframe add/remove update accepted timeline markers", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    await page.getByRole("tab", { name: "动画" }).click();
    await expect(page.getByLabel("动画参数")).toContainText("未选择片段");
    await expect(page.getByLabel("动画参数")).toContainText("选择时间线片段后，可查看动画参数和关键帧。");

    await page.getByRole("button", { name: /片段 城市街景\.mp4/ }).click();
    await expectCommandCall(app, "selectTimelineSegments");
    await expect(page.getByLabel("动画参数")).toContainText("还没有关键帧");
    await expect(page.getByLabel("动画参数")).toContainText("位置 X");
    await expect(page.getByLabel("动画参数")).toContainText("线性");
    await expect(page.getByLabel("动画参数")).toContainText("缓入缓出");

    await page.getByLabel("播放头").fill("1200000");
    await page.getByRole("button", { name: "添加位置 X关键帧" }).first().click();
    await expectCommandCall(app, "setSegmentKeyframe");

    await expect(page.locator(".segment-keyframe-marker")).toHaveCount(1);
    await expect(page.getByLabel("关键帧标记")).toBeVisible();
    await expect(page.getByLabel("关键帧列表")).toContainText("00:00:01.200");
    await expect(page.getByLabel("关键帧列表")).toContainText("线性");
    await expect(page.getByLabel("预览状态", { exact: true })).toContainText("画面已更新，预览待刷新");
    await expect(page.getByLabel("导出日志")).toContainText("关键帧已更新，请重新开始导出");

    const addCalls = await readExecuteCommandCalls(app);
    const addKeyframeCall = addCalls.find((call) => call.command === "setSegmentKeyframe");
    expect(addKeyframeCall?.kind).toBe("setSegmentKeyframe");
    expect(addKeyframeCall?.keyframeProperty).toBe("visualPositionX");
    expect(addKeyframeCall?.keyframeAt).toBe(1_200_000);
    expect(addKeyframeCall?.keyframe).toMatchObject({
      at: 1_200_000,
      property: "visualPositionX",
      value: { kind: "int", value: 0 },
      interpolation: "linear",
      easing: "none"
    });

    await setViewportSizeAndVerifyLayout(app, page, 1280, 800);
    await setViewportSizeAndVerifyLayout(app, page, 1120, 720);
    await expect(page.locator(".segment-keyframe-marker")).toHaveCount(1);

    await page.locator(".animation-detail").getByRole("button", { name: "删除位置 X关键帧" }).first().click();
    await expectCommandCall(app, "removeSegmentKeyframe");
    await expect(page.locator(".segment-keyframe-marker")).toHaveCount(0);
    await expect(page.getByLabel("动画参数")).toContainText("还没有关键帧");

    const removeCalls = await readExecuteCommandCalls(app);
    const removeKeyframeCall = removeCalls.find((call) => call.command === "removeSegmentKeyframe");
    expect(removeKeyframeCall?.kind).toBe("removeSegmentKeyframe");
    expect(removeKeyframeCall?.keyframeProperty).toBe("visualPositionX");
    expect(removeKeyframeCall?.keyframeAt).toBe(1_200_000);
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
  const { app, page } = await launchWorkspaceApp({ showDeveloperDiagnostics: true });

  try {
    await spyExecuteCommandCalls(app, page);

    await page.getByLabel("预览时间").fill("1200000");
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:01.200");

    await page.getByRole("button", { name: "请求预览帧" }).click();
    await expectCommandCall(app, "requestPreviewFrame");
    await expect(page.getByLabel("预览产物")).toContainText("预览帧已生成");
    await expect(page.getByLabel("预览产物")).toContainText("image/png");
    const previewImage = page.getByRole("img", { name: "当前预览帧" });
    await expect(previewImage).toBeVisible();
    await expect(previewImage).toHaveAttribute("src", /test-frame-1200000\.png$/);
    await expect(page.getByLabel("预览画面")).not.toContainText("/tmp/video-editor-preview-cache/test-frame-1200000.png");

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

test("播放头预览时间输入和逐帧按钮请求目标预览帧", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    await page.getByLabel("预览时间").fill("1200000");
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:01.200");
    await expectLatestPreviewFrameTarget(app, 1_200_000);
    await expect(page.getByRole("img", { name: "当前预览帧" })).toHaveAttribute("src", /test-frame-1200000\.png$/);

    const inspector = page.getByLabel("草稿参数");
    await page.getByLabel("帧率", { exact: true }).selectOption("custom");
    await page.getByLabel("帧率分子").fill("30000");
    await page.getByLabel("帧率分母").fill("1001");
    await inspector.getByRole("button", { name: "应用草稿参数" }).click();
    await expectCommandCall(app, "updateDraftCanvasConfig");
    await expect(page.getByLabel("预览窗口")).toContainText("30000/1001 fps");

    await spyExecuteCommandCalls(app, page);
    await page.getByLabel("预览时间").fill("0");
    await expectLatestPreviewFrameTarget(app, 0);
    await page.getByLabel("预览时间").fill("1200000");
    await expectLatestPreviewFrameTarget(app, 1_200_000);

    await page.getByRole("button", { name: "下一帧" }).click();
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:01.233");
    await expectLatestPreviewFrameTarget(app, 1_233_367);

    await page.getByRole("button", { name: "上一帧" }).click();
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:01.200");
    await expectLatestPreviewFrameTarget(app, 1_200_000);
  } finally {
    await app.close();
  }
});

test("预览播放按钮使用实时预览宿主而不是连续请求预览帧", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    const previewControls = page.getByRole("group", { name: "预览播放控制" });
    await expect(previewControls.getByRole("button", { name: "播放" })).toBeEnabled({ timeout: 20_000 });
    await previewControls.getByRole("button", { name: "播放" }).click();
    await expect(previewControls.getByRole("button", { name: "暂停" })).toBeEnabled();

    await expect
      .poll(async () => (await readRealtimePreviewHostCalls(app)).map((call) => call.kind), { timeout: 7_000 })
      .toEqual(expect.arrayContaining(["updateDraftSnapshot", "seek", "play"]));

    const playbackFrameRequests = (await readExecuteCommandCalls(app)).filter((call) => call.command === "requestPreviewFrame");
    expect(playbackFrameRequests).toHaveLength(0);

    await previewControls.getByRole("button", { name: "暂停" }).click();
    await expect
      .poll(async () => (await readRealtimePreviewHostCalls(app)).some((call) => call.kind === "pause"))
      .toBe(true);
    await expect(previewControls.getByRole("button", { name: "播放" })).toBeEnabled({ timeout: 10_000 });

    const timelineControls = page.getByRole("group", { name: "播放与历史" });
    await expect(timelineControls.getByRole("button", { name: "播放" })).toBeEnabled({ timeout: 10_000 });
    await timelineControls.getByRole("button", { name: "播放" }).click();
    await expect(timelineControls.getByRole("button", { name: "暂停" })).toBeEnabled();
    await timelineControls.getByRole("button", { name: "停止" }).click();
    await expect
      .poll(async () => (await readRealtimePreviewHostCalls(app)).some((call) => call.kind === "stop"))
      .toBe(true);
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:00.000");
  } finally {
    await app.close();
  }
});

test("音频预览 controls send generated command envelopes and preserve state after rejection", async () => {
  const { app, page } = await launchWorkspaceApp({
    env: {
      VIDEO_EDITOR_TEST_AUDIO_REJECT_COMMAND: "pauseAudioPreview"
    }
  });

  try {
    await spyExecuteCommandCalls(app, page);

    await expect(page.getByLabel("音频预览状态")).toContainText("音频就绪");
    await expect(page.getByLabel("输出设备状态")).toContainText("系统默认");

    await page.getByRole("button", { name: "播放预览" }).first().click();
    await expectCommandCall(app, "createAudioPreviewSession");
    await expectCommandCall(app, "playAudioPreview");
    await expect(page.getByLabel("音频预览状态")).toContainText("正在播放");

    await page.getByRole("button", { name: "暂停预览" }).first().click();
    await expectCommandCall(app, "pauseAudioPreview");
    await expect(page.getByLabel("音频预览状态")).toContainText("正在播放");

    await page.getByLabel("预览时间").fill("1200000");
    await expectCommandCall(app, "seekAudioPreview");
    await page.getByRole("button", { name: "停止预览" }).first().click();
    await expectCommandCall(app, "stopAudioPreview");
    await page.getByRole("button", { name: "重试音频" }).click();
    await expectCommandCall(app, "cancelAudioPreview");
    await expectCommandCall(app, "getAudioPreviewStatus");

    await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "音频" }).click();
    const audioPanel = page.getByRole("region", { name: "素材面板" });
    await audioPanel.getByLabel("输出设备").selectOption("desktop-output-secondary");
    await expectCommandCall(app, "selectAudioOutputDevice");
    await expect(audioPanel.getByLabel("输出设备")).toContainText("外接监听");

    const calls = await readExecuteCommandCalls(app);
    expect(calls.map((call) => call.command)).toEqual(
      expect.arrayContaining([
        "createAudioPreviewSession",
        "playAudioPreview",
        "pauseAudioPreview",
        "stopAudioPreview",
        "seekAudioPreview",
        "cancelAudioPreview",
        "getAudioPreviewStatus",
        "listAudioOutputDevices",
        "selectAudioOutputDevice"
      ])
    );
  } finally {
    await app.close();
  }
});

test("音频预览 panel and inspector expose production audio controls through updateSegmentAudio", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "音频" }).click();
    const audioPanel = page.getByRole("region", { name: "素材面板" });
    for (const label of ["音量", "声像", "淡入", "淡出", "轨道静音", "输出设备"]) {
      await expect(audioPanel.getByText(label, { exact: true }).first()).toBeVisible();
    }
    await expect(audioPanel.getByText("毫音量")).toHaveCount(0);
    await expect(audioPanel.getByRole("button", { name: "应用音频" })).toBeVisible();

    await page.getByRole("button", { name: /片段 背景音乐\.wav/ }).first().click();
    await page.getByRole("tab", { name: "音频" }).click();
    const audioInspector = page.getByLabel("音频参数");
    for (const label of ["音量", "声像", "淡入", "淡出", "轨道静音"]) {
      await expect(audioInspector.getByText(label, { exact: true }).first()).toBeVisible();
    }
    await expect(audioInspector.getByText("毫音量")).toHaveCount(0);

    await audioInspector.getByRole("slider", { name: "音量" }).fill("120");
    await audioInspector.getByRole("slider", { name: "声像" }).fill("-20");
    await audioInspector.getByRole("spinbutton", { name: "淡入" }).fill("300000");
    await audioInspector.getByRole("spinbutton", { name: "淡出" }).fill("500000");
    await audioInspector.getByRole("button", { name: "应用音频" }).click();
    await expectCommandCall(app, "updateSegmentAudio");

    const calls = await readExecuteCommandCalls(app);
    expect(calls.map((call) => call.command)).toContain("updateSegmentAudio");
  } finally {
    await app.close();
  }
});

test("波形 display uses Rust-shaped peak payloads and keeps fallback states stable", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    const audioSegment = page.getByRole("button", { name: /片段 背景音乐\.wav/ }).first();
    await expect(audioSegment.locator('[aria-label="音频波形"]')).toBeVisible();
    await expect(audioSegment.locator('[aria-label="音频波形"] .audio-waveform-bar')).toHaveCount(16);
    await expect(page.getByText("波形就绪")).toBeVisible();
    await expectCommandCall(app, "getWaveformDisplayPeaks");
    await expectCommandCall(app, "refreshWaveformStatus");

    const waveformBox = await expectStableBox(audioSegment.locator('[aria-label="音频波形"]'), "音频波形");
    expect(waveformBox.height, "音频波形固定 14px 高").toBeLessThanOrEqual(14);
    await setViewportSizeAndVerifyLayout(app, page, 1280, 800);
    await setViewportSizeAndVerifyLayout(app, page, 1120, 720);
  } finally {
    await app.close();
  }

  const pending = await launchWorkspaceApp({ env: { VIDEO_EDITOR_TEST_AUDIO_WAVEFORM_STATUS: "pending" } });
  try {
    await expect(pending.page.getByLabel("波形状态")).toContainText("波形生成中");
    await expect(pending.page.getByRole("button", { name: /片段 背景音乐\.wav/ }).first().locator('[aria-label="音频波形占位"]')).toBeVisible();
  } finally {
    await pending.app.close();
  }

  const failed = await launchWorkspaceApp({ env: { VIDEO_EDITOR_TEST_AUDIO_WAVEFORM_STATUS: "failed" } });
  try {
    await expect(failed.page.getByLabel("波形状态")).toContainText("波形生成失败");
    await expect(failed.page.getByRole("button", { name: /片段 背景音乐\.wav/ }).first().locator('[aria-label="音频波形占位"]')).toBeVisible();
  } finally {
    await failed.app.close();
  }
});

test("native preview host bridge keeps handles in main and exposes narrow telemetry APIs", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    const bridgeShape = await page.evaluate(() => {
      const bridge = window.videoEditorRealtimePreviewHost;
      return bridge === undefined ? [] : Object.keys(bridge).sort();
    });

    expect(bridgeShape).toEqual(["getTelemetry", "pause", "play", "seek", "stop", "updateDraftSnapshot", "updateHostRect"]);

    const updateResult = await page.evaluate(() =>
      window.videoEditorRealtimePreviewHost?.updateHostRect({
        x: 12.7,
        y: 34.2,
        width: 320.9,
        height: 180.1,
        scaleFactorMillis: 1250.6
      })
    );
    expect(JSON.stringify(updateResult)).not.toMatch(/native|handle|hwnd|nsview|gpu|surface|commandEncoder|cacheKey/i);

    const telemetry = await page.evaluate(() => window.videoEditorRealtimePreviewHost?.getTelemetry());
    expect(JSON.stringify(telemetry)).not.toMatch(/native|handle|hwnd|nsview|gpu|surface|commandEncoder|cacheKey/i);

    await expect
      .poll(async () => (await readRealtimePreviewHostCalls(app)).some((call) => call.kind === "attachSurface"))
      .toBe(true);

    const callsBeforeClose = await readRealtimePreviewHostCalls(app);
    expect(callsBeforeClose.some((call) => call.kind === "createSession")).toBe(true);
    expect(callsBeforeClose.some((call) => call.kind === "acquireNativeWindowHandle" && (call.parentHandleByteLength ?? 0) > 0)).toBe(true);
    expect(callsBeforeClose.some((call) => call.kind === "attachSurface" && call.surfaceKind !== undefined)).toBe(true);
    expect(callsBeforeClose.some((call) => call.kind === "updateSurfaceBounds")).toBe(true);
    expect(callsBeforeClose.at(-1)?.kind).not.toBe("closeSession");

    const latestBounds = callsBeforeClose.findLast((call) => call.kind === "updateSurfaceBounds")?.bounds;
    expect(latestBounds).toEqual({
      x: 13,
      y: 34,
      width: 321,
      height: 180,
      scaleFactorMillis: 1251
    });
    await app.evaluate(({ BrowserWindow }) => {
      BrowserWindow.getAllWindows()[0]?.close();
    });
    await expect
      .poll(async () => (await readRealtimePreviewHostCalls(app)).some((call) => call.kind === "closeSession"))
      .toBe(true);

    const callsAfterClose = await readRealtimePreviewHostCalls(app);
    expect(callsAfterClose.some((call) => call.kind === "detachSurface")).toBe(true);
    expect(callsAfterClose.some((call) => call.kind === "closeSession")).toBe(true);
  } finally {
    await app.close();
  }
});

test("实时预览 native preview host rectangle reports integer bounds and telemetry", async () => {
  const { app, page } = await launchWorkspaceApp({
    env: {
      VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_FIRST_FRAME: "1"
    }
  });

  try {
    const host1280 = await expectNativePreviewHostLayout(app, page, 1280, 800);
    const latest1280 = await latestRealtimePreviewBounds(app);
    expect(latest1280.width).toBe(Math.round(host1280.width));
    expect(latest1280.height).toBe(Math.round(host1280.height));
    expect(latest1280.x).toBe(Math.round(host1280.x));
    expect(latest1280.y).toBe(Math.round(host1280.y));

    await expect(page.getByLabel("实时预览状态")).toContainText("实时预览已接入");
    await expect(page.getByLabel("实时预览数据")).toContainText("首帧");
    await expect(page.getByLabel("实时预览数据")).toContainText("已呈现 1 帧");

    const host1120 = await expectNativePreviewHostLayout(app, page, 1120, 720);
    const latest1120 = await latestRealtimePreviewBounds(app);
    const deviceScaleMillis = await page.evaluate(() => Math.round(window.devicePixelRatio * 1000));
    expect(latest1120.width).toBe(Math.round(host1120.width));
    expect(latest1120.height).toBe(Math.round(host1120.height));
    expect(latest1120.scaleFactorMillis).toBe(deviceScaleMillis);
  } finally {
    await app.close();
  }
});

test("实时预览 native preview fallback displays main-provided attach diagnostics", async () => {
  const { app, page } = await launchWorkspaceApp({
    env: {
      VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_ATTACH_FAILURE: "1"
    }
  });

  try {
    await expectNativePreviewHostLayout(app, page, 1280, 800, { requireBoundsUpdate: false });
    await expect(page.getByLabel("实时预览状态")).toContainText("实时预览降级显示");
    await expect(page.getByLabel("实时预览降级")).toContainText("实时预览降级");
    await expect(page.getByLabel("实时预览降级")).not.toContainText("HWND");
    await expect(page.getByLabel("实时预览降级")).not.toContainText("NSView");
  } finally {
    await app.close();
  }
});

test("实时预览 telemetry shows supported backend without media fallback active label", async () => {
  const { app, page } = await launchWorkspaceApp({
    env: {
      VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_FIRST_FRAME: "1"
    }
  });

  try {
    await expectNativePreviewHostLayout(app, page, 1280, 800);
    await expect(page.getByLabel("实时预览状态")).toContainText("实时预览已接入");
    await expect(page.getByLabel("实时预览数据")).toContainText("实时后端：Mock");
    await expect(page.getByLabel("实时预览数据")).toContainText("首帧 9 ms");
    await expect(page.getByLabel("实时预览数据")).toContainText("寻帧 -");
    await expect(page.getByLabel("实时预览数据")).toContainText("拒绝旧帧 0");
    await expect(page.getByLabel("实时预览数据")).toContainText("缓存 0");
    await expect(page.getByLabel("实时预览数据")).not.toContainText("FFmpeg");
    await expect(page.getByLabel("实时预览备用产物")).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("实时预览 telemetry shows supported seek latency without fallback artifact", async () => {
  const { app, page } = await launchWorkspaceApp({
    env: {
      VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_SEEK_FRAME: "1"
    }
  });

  try {
    await expectNativePreviewHostLayout(app, page, 1120, 720);
    await expect(page.getByLabel("实时预览状态")).toContainText("实时预览已接入");
    await expect(page.getByLabel("实时预览数据")).toContainText("实时后端：Mock");
    await expect(page.getByLabel("实时预览数据")).toContainText("寻帧 7 ms");
    await expect(page.getByLabel("实时预览数据")).toContainText("已呈现 1 帧");
    await expect(page.getByLabel("实时预览备用产物")).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("实时预览 fallback artifact appears only when Rust reports fallback", async () => {
  const supported = await launchWorkspaceApp({
    env: {
      VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_FIRST_FRAME: "1"
    }
  });

  try {
    await expectNativePreviewHostLayout(supported.app, supported.page, 1280, 800);
    await expect(supported.page.getByLabel("实时预览备用产物")).toHaveCount(0);
  } finally {
    await supported.app.close();
  }

  const fallback = await launchWorkspaceApp({
    env: {
      VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_FFMPEG_FALLBACK: "1"
    }
  });

  try {
    await expectNativePreviewHostLayout(fallback.app, fallback.page, 1280, 800);
    await expect(fallback.page.getByLabel("实时预览数据")).toContainText("备用产物：媒体运行环境");
    await expect(fallback.page.getByLabel("实时预览数据")).toContainText("降级 1");
    await expect(fallback.page.getByLabel("实时预览备用产物")).toContainText("已生成媒体备用产物");
  } finally {
    await fallback.app.close();
  }
});

test("实时预览 telemetry displays Rust-reported cancellation counters", async () => {
  const { app, page } = await launchWorkspaceApp({
    env: {
      VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_CANCELED: "1"
    }
  });

  try {
    await expectNativePreviewHostLayout(app, page, 1280, 800);
    await expect(page.getByLabel("实时预览数据")).toContainText("当前请求已取消");
    await expect(page.getByLabel("实时预览数据")).toContainText("取消 1");
    await expect(page.getByLabel("实时预览数据")).toContainText("请求已取消");
    await expect(page.getByLabel("实时预览备用产物")).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("fallback source guard keeps renderer display-only for telemetry", () => {
  const previewMonitorSource = readFileSync(join(process.cwd(), "src/renderer/workspace/PreviewMonitor.tsx"), "utf8");
  const viewModelSource = readFileSync(join(process.cwd(), "src/renderer/viewModel.ts"), "utf8");

  expect(previewMonitorSource, "renderer must not build FFmpeg commands").not.toMatch(/ffmpeg\s*(?:-|\.|Command|Args)/i);
  expect(previewMonitorSource, "renderer must not create render graph/cache logic").not.toMatch(/renderGraph|cacheKey|fallbackLadder/i);
  expect(previewMonitorSource, "renderer must not assign fallback reasons").not.toMatch(/fallbackReason\s*=/i);
  expect(viewModelSource, "display model should not inspect drafts to infer support").not.toMatch(
    /if\s*\([^)]*(?:draft|material)[^)]*\)[\s\S]{0,160}fallback/i
  );
});

test("Phase 11 source guard and root scripts are wired", () => {
  const packageJson = JSON.parse(readFileSync(join(REPO_ROOT, "package.json"), "utf8")) as {
    scripts: Record<string, string>;
  };
  const guardSource = readFileSync(join(REPO_ROOT, "scripts/phase11-source-guards.sh"), "utf8");

  expect(packageJson.scripts["test:phase11-rust"]).toContain("realtime_preview_runtime");
  expect(packageJson.scripts["test:phase11-source-guards"]).toBe("bash scripts/phase11-source-guards.sh");
  expect(packageJson.scripts["test:phase11-workspace"]).toContain("实时预览|fallback|telemetry|五大区域");
  expect(packageJson.scripts["test:phase11"]).toContain("test:phase11-rust");
  expect(packageJson.scripts["test:phase11"]).toContain("test:phase11-source-guards");
  expect(packageJson.scripts["test:phase11"]).toContain("test:phase11-workspace");
  expect(packageJson.scripts["test:phase11"]).toContain("test:contracts");

  for (const forbiddenToken of [
    "GPUDevice",
    "build_render_graph",
    "RenderGraph",
    "compile_ffmpeg_job",
    "FfmpegExecutor",
    "previewCacheKey",
    "changedRanges",
    "evaluateKeyframes"
  ]) {
    expect(guardSource).toContain(forbiddenToken);
  }
  expect(guardSource).toContain("strip_comments");
});

test("Phase 11 runtime boundary docs include ownership, exclusions, and platform smoke commands", () => {
  const docs = readFileSync(join(REPO_ROOT, "docs/runtime-boundaries.md"), "utf8");
  const packageJson = JSON.parse(readFileSync(join(REPO_ROOT, "package.json"), "utf8")) as {
    scripts: Record<string, string>;
  };

  for (const requiredText of [
    "## Phase 11 Realtime Preview Runtime",
    "Rust-owned session, clock, generation, capability classification, telemetry, fallback routing, and GPU composition",
    "H.264 software video frame provider/cache",
    "Renderer responsibilities are UI-only",
    "TextParityUnsupported",
    "Phase 12 owns platform-native media IO and hardware decode",
    "Phase 15 owns realtime audio",
    "Phase 16 owns priority scheduling",
    "Phase 18 owns complex effects, retiming, filters, masks, and transitions",
    "Windows D3D12",
    "macOS Metal",
    "VIDEO_EDITOR_TEST_WGPU=1 cargo test -p realtime_preview_runtime real_wgpu_adapter -- --ignored --nocapture"
  ]) {
    expect(docs).toContain(requiredText);
  }

  expect(packageJson.scripts["test:phase11"]).toContain("test:phase11-workspace");
  expect(packageJson.scripts["test:phase11-workspace"]).toContain("实时预览|fallback|telemetry|五大区域");
});

test("telemetry display model represents Rust-owned realtime and fallback diagnostics", () => {
  const supported: RealtimePreviewDisplayModel = {
    backend: "mock",
    firstFrameLatencyMs: 18,
    seekLatencyMs: 7,
    queueLatencyMs: 2,
    renderDurationMs: 5,
    presentedFrameCount: 4,
    droppedFrameCount: 0,
    repeatedFrameCount: 1,
    staleRejectedCount: 0,
    canceledRequestCount: 0,
    currentRequestCanceled: false,
    fallbackReason: null,
    fallbackCount: 0,
    cacheHitCount: 2,
    targetTimeMicroseconds: 1_200_000,
    playbackGeneration: 3,
    fallbackArtifactVisible: false
  };
  const fallback: RealtimePreviewDisplayModel = {
    ...supported,
    backend: "ffmpegArtifact",
    presentedFrameCount: 0,
    currentRequestCanceled: true,
    fallbackReason: "ffmpegArtifactGenerated",
    fallbackCount: 1,
    fallbackArtifactVisible: true
  };

  expect(formatRealtimePreviewBackendLabel(supported.backend)).toBe("实时后端：Mock");
  expect(formatRealtimePreviewBackendLabel(fallback.backend)).toBe("备用产物：媒体运行环境");
  expect(formatRealtimePreviewFallbackReason("previewArtifactCacheHit")).toBe("命中预览缓存");
  expect(summarizeRealtimePreviewDisplay(supported)).toContain("首帧 18 ms");
  expect(summarizeRealtimePreviewDisplay(supported)).toContain("重复 1");
  expect(summarizeRealtimePreviewDisplay(supported)).toContain("缓存 2");
  expect(summarizeRealtimePreviewDisplay(fallback)).toContain("当前请求已取消");
  expect(fallback.fallbackArtifactVisible).toBe(true);
});

test("播放头支持时间线标尺点击和拖动请求预览帧", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    const rulerTrack = page.locator(".ruler-track");
    const rulerBox = await expectStableBox(rulerTrack, "时间线标尺轨道");
    await page.mouse.click(rulerBox.x + rulerBox.width * 0.5, rulerBox.y + rulerBox.height * 0.5);
    await expect(page.getByLabel("播放头")).toHaveValue("5000000");
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:05.000");
    await expectLatestPreviewFrameTarget(app, 5_000_000);

    await spyExecuteCommandCalls(app, page);
    const playhead = page.locator(".playhead");
    const playheadBox = await expectStableBox(playhead, "播放头拖动线");
    await page.mouse.move(playheadBox.x + playheadBox.width / 2, playheadBox.y + 4);
    await page.mouse.down();
    await page.mouse.move(rulerBox.x + rulerBox.width * 0.75, playheadBox.y + 4);
    await page.mouse.up();

    await expect(page.getByLabel("播放头")).toHaveValue("7500000");
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:07.500");
    await expectLatestPreviewFrameTarget(app, 7_500_000);
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
    await expect(
      page.getByLabel("预览窗口").getByText("画布 9:16 · 1080 x 1920 · 30 fps", { exact: true })
    ).toBeVisible();
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

test("自定义帧率在画布参数变更时保持有理数语义", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await spyExecuteCommandCalls(app, page);

    const inspector = page.getByLabel("草稿参数");
    await page.getByLabel("帧率", { exact: true }).selectOption("custom");
    await page.getByLabel("帧率分子").fill("30000");
    await page.getByLabel("帧率分母").fill("1001");
    await inspector.getByRole("button", { name: "应用草稿参数" }).click();

    await expectCommandCall(app, "updateDraftCanvasConfig");
    await expect(page.getByLabel("预览窗口")).toContainText("30000/1001 fps");

    await inspector.getByRole("group", { name: "画布背景" }).getByRole("button", { name: "纯色" }).click();
    await inspector.getByRole("button", { name: "应用草稿参数" }).click();

    await expect
      .poll(async () => (await readExecuteCommandCalls(app)).filter((call) => call.command === "updateDraftCanvasConfig").length)
      .toBe(2);

    const canvasCalls = (await readExecuteCommandCalls(app)).filter((call) => call.command === "updateDraftCanvasConfig");
    expect(canvasCalls.at(-1)?.canvasConfig?.frameRate).toEqual({ numerator: 30000, denominator: 1001 });
  } finally {
    await app.close();
  }
});

test("画布变更后旧预览和导出派生状态失效", async () => {
  const { app, page } = await launchWorkspaceApp({ showDeveloperDiagnostics: true });

  try {
    await spyExecuteCommandCalls(app, page);

    await page.getByRole("button", { name: "请求预览帧" }).click();
    await expectCommandCall(app, "requestPreviewFrame");
    await expect(page.getByRole("img", { name: "当前预览帧" })).toBeVisible();
    await expect(page.getByRole("img", { name: "当前预览帧" })).toHaveAttribute("src", /test-frame-0\.png$/);

    await page.getByRole("button", { name: "生成预览片段" }).click();
    await expectCommandCall(app, "requestPreviewSegment");
    await expect(page.getByLabel("预览产物")).toContainText("/tmp/video-editor-preview-cache/test-segment-0.mp4");

    await page.getByRole("button", { name: "开始导出" }).click();
    await expectCommandCall(app, "startExport");
    await page.getByRole("button", { name: "查询导出状态" }).click();
    await expectCommandCall(app, "getExportJobStatus");
    await expect(page.getByLabel("输出校验")).toContainText("1920x1080");
    await expect(page.getByRole("button", { name: "查询导出状态" })).toBeEnabled();

    const inspector = page.getByLabel("草稿参数");
    await inspector.getByRole("group", { name: "画布比例" }).getByRole("button", { name: "1:1" }).click();
    await inspector.getByRole("button", { name: "应用草稿参数" }).click();
    await expectCommandCall(app, "updateDraftCanvasConfig");

    await expect(page.getByRole("img", { name: "当前预览帧" })).toHaveCount(0);
    await expect(page.getByLabel("预览产物")).not.toContainText("/tmp/video-editor-preview-cache/test-segment-0.mp4");
    await expect(page.getByLabel("预览产物")).toContainText("画布已更新，请重新请求预览帧");
    await expect(page.getByLabel("预览产物")).toContainText("画布已更新，请重新生成预览片段");
    await expect(page.getByLabel("导出日志")).toContainText("草稿已更新，请重新开始导出");
    await expect(page.getByLabel("输出校验")).toContainText("输出校验待完成");
    await expect(page.getByRole("button", { name: "查询导出状态" })).toBeDisabled();
    await expect(page.getByRole("button", { name: "取消导出" })).toBeDisabled();
  } finally {
    await app.close();
  }
});

test("画面变换 command-only transform 通过 Rust command 更新 UI 并清理派生状态", async () => {
  const { app, page } = await launchWorkspaceApp({ showDeveloperDiagnostics: true });

  try {
    await spyExecuteCommandCalls(app, page);
    await expectNoLeftSecondaryMenu(page);

    await page.getByRole("button", { name: "请求预览帧" }).click();
    await expectCommandCall(app, "requestPreviewFrame");
    await expect(page.getByRole("img", { name: "当前预览帧" })).toBeVisible();
    await expect(page.getByRole("img", { name: "当前预览帧" })).toHaveAttribute("src", /test-frame-0\.png$/);

    await page.getByRole("button", { name: "生成预览片段" }).click();
    await expectCommandCall(app, "requestPreviewSegment");
    await expect(page.getByLabel("预览产物")).toContainText("/tmp/video-editor-preview-cache/test-segment-0.mp4");

    await page.getByRole("button", { name: "开始导出" }).click();
    await expectCommandCall(app, "startExport");
    await page.getByRole("button", { name: "查询导出状态" }).click();
    await expectCommandCall(app, "getExportJobStatus");
    await expect(page.getByLabel("输出校验")).toContainText("1920x1080");

    await page.getByRole("button", { name: /片段 城市街景\.mp4/ }).click();
    await expectCommandCall(app, "selectTimelineSegments");

    const visualForm = page.getByLabel("画面基础表单");
    await expect(page.getByLabel("画面变换")).toContainText("基础");
    for (const label of ["显示画面", "位置", "缩放", "旋转", "不透明度", "适应方式", "裁剪", "背景填充"]) {
      await expect(visualForm).toContainText(label);
    }
    await expect(visualForm).toContainText("混合模式");
    await expect(visualForm).toContainText("蒙版");
    await expect(visualForm.getByRole("button", { name: "应用画面" })).toBeDisabled();

    await visualForm.getByLabel("位置 X", { exact: true }).fill("160");
    await visualForm.getByLabel("位置 Y", { exact: true }).fill("-90");
    await visualForm.getByLabel("缩放 X", { exact: true }).fill("1250");
    await visualForm.getByLabel("缩放 Y", { exact: true }).fill("850");
    await visualForm.getByRole("spinbutton", { name: "旋转", exact: true }).fill("12");
    await visualForm.getByRole("spinbutton", { name: "不透明度", exact: true }).fill("760");
    await visualForm.getByLabel("裁剪 左", { exact: true }).fill("80");
    await visualForm.getByLabel("裁剪 右", { exact: true }).fill("40");
    await visualForm.getByLabel("裁剪 上", { exact: true }).fill("30");
    await visualForm.getByLabel("裁剪 下", { exact: true }).fill("20");
    await visualForm.getByRole("group", { name: "适应方式" }).getByRole("button", { name: "填充" }).click();
    await visualForm.getByRole("group", { name: "背景填充" }).getByRole("button", { name: "黑色" }).click();
    await expect(visualForm.getByRole("button", { name: "应用画面" })).toBeEnabled();
    await visualForm.getByRole("button", { name: "应用画面" }).click();

    await expectCommandCall(app, "updateSegmentVisual");
    await expectLatestPreviewFrameTarget(app, 0);
    await expect(visualForm.getByLabel("位置 X", { exact: true })).toHaveValue("160");
    await expect(visualForm.getByLabel("位置 Y", { exact: true })).toHaveValue("-90");
    await expect(visualForm.getByLabel("缩放 X", { exact: true })).toHaveValue("1250");
    await expect(visualForm.getByLabel("缩放 Y", { exact: true })).toHaveValue("850");

    await expect(page.getByRole("img", { name: "当前预览帧" })).toBeVisible();
    await expect(page.getByRole("img", { name: "当前预览帧" })).toHaveAttribute("src", /test-frame-0\.png$/);
    await expect(page.getByLabel("预览产物")).not.toContainText("/tmp/video-editor-preview-cache/test-segment-0.mp4");
    await expect(page.getByLabel("预览产物")).toContainText("预览帧已生成");
    await expect(page.getByLabel("预览产物")).toContainText("画面变换已更新，请重新生成预览片段");
    await expect(page.getByLabel("导出日志")).toContainText("画面变换已更新，请重新开始导出");
    await expect(page.getByLabel("输出校验")).toContainText("输出校验待完成");
    await expect(page.getByRole("button", { name: "查询导出状态" })).toBeDisabled();
    await expect(page.getByRole("button", { name: "取消导出" })).toBeDisabled();

    const visualCall = (await readExecuteCommandCalls(app)).find((call) => call.command === "updateSegmentVisual");
    expect(visualCall?.kind).toBe("updateSegmentVisual");
    expect(visualCall?.visual).toMatchObject({
      visible: true,
      fitMode: "fill",
      backgroundFilling: { kind: "black" },
      transform: {
        position: { x: 160, y: -90 },
        scale: { xMillis: 1250, yMillis: 850 },
        rotation: { degrees: 12 },
        opacity: { valueMillis: 760 },
        crop: { leftMillis: 80, rightMillis: 40, topMillis: 30, bottomMillis: 20 },
        anchor: { xMillis: 500, yMillis: 500 }
      },
      blendMode: { kind: "normal" },
      mask: { kind: "none" }
    });

    await setViewportSizeAndVerifyLayout(app, page, 1280, 800);
    await setViewportSizeAndVerifyLayout(app, page, 1120, 720);
  } finally {
    await app.close();
  }
});

test("selection preview overlay follows accepted visible segment without blocking preview image", async () => {
  const { app, page } = await launchWorkspaceApp({ showDeveloperDiagnostics: true });

  try {
    await spyExecuteCommandCalls(app, page);

    await page.getByRole("button", { name: "请求预览帧" }).click();
    await expectCommandCall(app, "requestPreviewFrame");
    await expect(page.getByRole("img", { name: "当前预览帧" })).toBeVisible();
    await expect(page.getByLabel("预览选中框")).toHaveCount(0);

    await page.getByRole("button", { name: /片段 城市街景\.mp4/ }).click();
    await expectCommandCall(app, "selectTimelineSegments");

    const overlay = page.getByLabel("预览选中框");
    await expect(overlay).toBeVisible();
    await expect(overlay).toHaveAttribute("data-segment-id", "segment-main-video");
    await expect(page.getByRole("img", { name: "当前预览帧" })).toBeVisible();

    const overlayPointerEvents = await overlay.evaluate((element) => window.getComputedStyle(element).pointerEvents);
    expect(overlayPointerEvents).toBe("none");

    await setViewportSizeAndVerifyLayout(app, page, 1280, 800);
    await expect(overlay).toBeVisible();
    await setViewportSizeAndVerifyLayout(app, page, 1120, 720);
    await expect(overlay).toBeVisible();
  } finally {
    await app.close();
  }
});

test("预览失败显示中文分类错误且不改草稿", async () => {
  const { app, page } = await launchWorkspaceApp({
    mockPreviewCommands: false,
    showDeveloperDiagnostics: true,
    env: {
      VE_FFMPEG_PATH: "/tmp/video-editor-missing-ffmpeg",
      VE_FFPROBE_PATH: "/tmp/video-editor-missing-ffprobe"
    }
  });

  try {
    await spyExecuteCommandCalls(app, page);

    await page.getByRole("button", { name: "请求预览帧" }).click();
    await expectCommandCall(app, "requestPreviewFrame");
    await expect(page.getByLabel("预览状态", { exact: true })).toContainText("请求预览帧失败");
    await expect(page.getByLabel("预览状态", { exact: true })).toContainText("预览服务失败");
    await expect(page.getByRole("button", { name: /片段 城市街景\.mp4/ })).toHaveCount(1);
    await expect(page.getByLabel("预览产物")).toContainText("预览帧失败");
    await expect(page.getByLabel("预览产物")).not.toContainText("/tmp/video-editor-preview-cache/test-frame");
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

    const draftMutatingCalls = (await readExecuteCommandCalls(app)).filter(
      (call) => call.command === "addSegment" || call.command === "importMaterial"
    );
    expect(draftMutatingCalls.map((call) => call.command)).toEqual(["addSegment"]);
  } finally {
    await app.close();
  }
});

test("五大区域 layout stability keeps workspace regions visible and fixed at required sizes", async () => {
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

    await expect(page.getByRole("button", { name: "请求预览帧" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "生成预览片段" })).toHaveCount(0);
    await expect(page.getByLabel("预览产物")).toHaveCount(0);
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

test("素材资源状态 uses generated artifact command envelopes", async () => {
  const { app, page } = await launchWorkspaceApp({ mockArtifactCommands: true });

  try {
    await spyExecuteCommandCalls(app, page);

    await expect(page.getByLabel("素材资源状态").first()).toBeVisible();
    await page.getByRole("button", { name: "更新状态" }).click();
    await page.getByRole("button", { name: "取消生成" }).click();
    await page.getByRole("button", { name: "重新生成" }).click();
    await page.getByRole("button", { name: "继续生成" }).click();
    await page.getByRole("button", { name: "清理缓存" }).click();
    await page.getByRole("button", { name: "确认清理缓存" }).click();

    await expect
      .poll(async () => (await readExecuteCommandCalls(app)).map((call) => call.command))
      .toEqual(
        expect.arrayContaining([
          "getArtifactStatus",
          "refreshArtifactStatus",
          "cancelArtifactGeneration",
          "retryArtifactGeneration",
          "resumeArtifactGeneration",
          "getArtifactQuotaStatus",
          "runArtifactGarbageCollection"
        ])
      );
  } finally {
    await app.close();
  }
});

test("资源任务 and 资源维护 update from Rust shaped artifact responses", async () => {
  const { app, page } = await launchWorkspaceApp({ mockArtifactCommands: true });

  try {
    await expect(page.getByLabel("资源任务")).toContainText("生成中");
    await expect(page.getByLabel("资源任务")).toContainText("正在取消");
    await expect(page.getByLabel("资源维护")).toContainText("缓存空间偏高");

    await page.getByRole("button", { name: "清理缓存" }).click();
    await expect(page.getByLabel("确认清理缓存")).toContainText("不会删除原始素材");
    await page.getByRole("button", { name: "确认清理缓存" }).click();
    await expect(page.getByLabel("资源维护")).toContainText("缓存清理完成");
  } finally {
    await app.close();
  }
});

test("资源任务 limits visible rows and 素材资源状态 keeps material row height stable", async () => {
  const { app, page } = await launchWorkspaceApp({
    mockArtifactCommands: true,
    env: {
      VIDEO_EDITOR_TEST_ARTIFACT_TASK_COUNT: "4"
    }
  });

  try {
    const firstMaterialRow = page.locator(".material-row").first();
    const before = await expectStableBox(firstMaterialRow, "资源状态刷新前素材行");

    await expect(page.locator(".resource-task-row")).toHaveCount(3);
    await expect(page.getByLabel("资源任务")).toContainText("另有 1 个资源任务");

    await page.getByRole("button", { name: "更新状态" }).click();
    await expectCommandCall(app, "refreshArtifactStatus");
    const after = await expectStableBox(firstMaterialRow, "资源状态刷新后素材行");
    expectSameSize(before, after, "素材资源状态刷新");
  } finally {
    await app.close();
  }
});

test("资源维护 and 素材资源状态 hide forbidden internal production copy", async () => {
  const { app, page } = await launchWorkspaceApp({ mockArtifactCommands: true });

  try {
    const resourceText = [
      await page.getByLabel("资源任务").textContent(),
      await page.getByLabel("资源维护").textContent(),
      await page.getByLabel("素材资源状态").first().textContent()
    ].join(" ");

    expect(resourceText).not.toMatch(
      /SQLite|\.sqlite|artifact-store\.sqlite|\.veproj\/derived|cacheRoot|fingerprint|graphNode|dirtyRange|FFmpeg|ffprobe|raw logs|cache key/i
    );
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

import { _electron as electron, expect, test, type ElectronApplication, type Locator, type Page } from "@playwright/test";
import { mkdirSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import type { Keyframe, SegmentVisual } from "../src/generated/Draft";
import {
  formatRealtimePreviewBackendLabel,
  formatRealtimePreviewFallbackReason,
  resourcePanelFromArtifactStatus,
  summarizeRealtimePreviewDisplay,
  summarizeRealtimePreviewProductDisplay,
  type RealtimePreviewDisplayModel
} from "../src/renderer/viewModel";
import type { RealtimePreviewHostApi } from "../src/renderer/workspace/PreviewMonitor";

type NativeCommandObservation = {
  command: string;
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
  textFontRef: string | null;
  srtContent: string | null;
  outputPath: string | null;
  preset: string | null;
  jobId: string | null;
  sessionId?: string | null;
  projectSessionId?: string | null;
  expectedRevision?: number | null;
  hasDraftField?: boolean;
};

type ProjectSessionCall = {
  command: "startProjectSessionExport" | string;
  sessionId?: string | null;
  projectSessionId?: string | null;
  expectedRevision?: number | null;
  intentKind?: string | null;
  targetTime?: number | null;
  targetTimerange?: { start: number; duration: number } | null;
  outputPath?: string | null;
  preset?: string | null;
  canvasConfig?: NativeCommandObservation["canvasConfig"];
  visual?: SegmentVisual | null;
  keyframeProperty?: string | null;
  keyframeAt?: number | null;
  textContent?: string | null;
  textSource?: string | null;
  textFontRef?: string | null;
  srtContent?: string | null;
  hasDraftField?: boolean;
};

type RealtimePreviewHostCall = {
  kind: string;
  nativeEventKind?: string;
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
const DEFERRED_CATEGORIES = ["贴纸", "特效", "转场", "滤镜", "调节", "模板", "数字人"] as const;
const REPO_ROOT = join(process.cwd(), "../..");
const PHASE5_SCREENSHOT_DIR = join(REPO_ROOT, "test-results/phase5");
const PHASE7_SCREENSHOT_DIR = join(REPO_ROOT, "test-results/phase7");
const PHASE15_3_SCREENSHOT_DIR = join(REPO_ROOT, "test-results/phase15-3");
const MEDIA_FIXTURE_DIR = join(REPO_ROOT, "apps/desktop-electron/tests/fixtures/media");
const PORTRAIT_VIDEO_FIXTURE = join(MEDIA_FIXTURE_DIR, "p0-portrait-testsrc.mp4");

declare global {
  interface Window {
    videoEditorTestObservations?: {
      getNativeCommandObservations: () => Promise<unknown[]>;
    };
    videoEditorRealtimePreviewHost?: RealtimePreviewHostApi;
  }
}

async function launchWorkspaceApp(
  options: {
    mockPreviewCommands?: boolean;
    mockExportCommands?: boolean;
    mockArtifactCommands?: boolean;
    mockAudioCommands?: boolean;
    showDeveloperDiagnostics?: boolean;
    startup?: "demoFixture" | "newProject";
    env?: NodeJS.ProcessEnv;
  } = {}
): Promise<{ app: ElectronApplication; page: Page }> {
  const useNewProject = options.startup === "newProject";
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")],
    env: {
      ...process.env,
      VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
      ...(useNewProject
        ? { VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE: testProjectPath("workspace") }
        : { VIDEO_EDITOR_TEST_WORKSPACE_FIXTURE: "demo" }),
      VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS: options.mockPreviewCommands === false ? "0" : "1",
      VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS: options.mockExportCommands === false ? "0" : "1",
      VIDEO_EDITOR_TEST_MOCK_ARTIFACT_COMMANDS: options.mockArtifactCommands === false ? "0" : "1",
      VIDEO_EDITOR_TEST_MOCK_AUDIO_COMMANDS: options.mockAudioCommands === false ? "0" : "1",
      VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "1",
      VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: options.showDeveloperDiagnostics === true ? "1" : "0",
      VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify(["/tmp/demo-material.mp4"]),
      ...options.env
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  if (useNewProject) {
    await expect(page.getByRole("main", { name: "项目入口" })).toBeVisible();
    await page.getByRole("button", { name: "新建项目" }).click();
  }
  await expectVisibleWorkspaceRegions(page);
  await expect
    .poll(
      async () =>
        (
          await app.evaluate(() => {
            return (
              (
                globalThis as typeof globalThis & {
                  __videoEditorTestProjectSessionCalls?: ProjectSessionCall[];
                }
              ).__videoEditorTestProjectSessionCalls ?? []
            );
          })
        ).some((call) => call.command === "createProjectSession" || call.command === "openProjectSession"),
      { timeout: 20_000 }
    )
    .toBe(true);
  return { app, page };
}

function testProjectPath(label: string): string {
  return join(tmpdir(), `video-editor-${label}-${process.pid}-${Date.now()}-${Math.random().toString(16).slice(2)}.veproj`);
}

async function expectVisibleWorkspaceRegions(page: Page): Promise<void> {
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  await expect(page.locator('[aria-label="顶部功能区"]').first()).toBeVisible();
  await expect(page.locator('[aria-label="素材面板"]')).toBeVisible();
  await expect(page.locator('[aria-label="预览窗口"]')).toBeVisible();
  await expect(page.locator('[aria-label="属性检查器"]')).toBeVisible();
  await expect(page.locator('[aria-label="时间线"]')).toBeVisible();
}

async function resetNativeCommandObservations(app: ElectronApplication, page: Page): Promise<void> {
  const hasBridge = await page.evaluate(() => typeof window.videoEditorTestObservations?.getNativeCommandObservations === "function");
  if (!hasBridge) {
    throw new Error("workspace test setup error: native test observation bridge is unavailable");
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
      .filter(
        (call) =>
          call.command === "startProjectSessionExport" ||
          call.intentKind !== null
      )
      .map((call) => {
        const command =
          call.command === "startProjectSessionExport"
            ? "startExport"
            : (call.intentKind ?? "executeProjectIntent");
        return {
          command,
          kind: command,
          requestId: null,
          targetTime: call.targetTime ?? null,
          targetTimerange: call.targetTimerange ?? null,
          canvasConfig: call.canvasConfig ?? null,
          visual: call.visual ?? null,
          keyframe: null,
          keyframeProperty: call.keyframeProperty ?? null,
          keyframeAt: call.keyframeAt ?? null,
          textContent: call.textContent ?? null,
          textSource: call.textSource ?? null,
          textFontRef: call.textFontRef ?? null,
          srtContent: call.srtContent ?? null,
          outputPath: call.outputPath ?? null,
          preset: call.preset ?? null,
          jobId: null,
          sessionId: call.sessionId ?? null,
          projectSessionId: call.projectSessionId ?? call.sessionId ?? null,
          expectedRevision: call.expectedRevision ?? null,
          hasDraftField: call.hasDraftField
        };
      })
  ];
}

async function readRealtimePreviewHostCalls(app: ElectronApplication): Promise<RealtimePreviewHostCall[]> {
  return app.evaluate(() => {
    return (
      (globalThis as typeof globalThis & { __videoEditorTestRealtimePreviewHostCalls?: RealtimePreviewHostCall[] })
        .__videoEditorTestRealtimePreviewHostCalls ?? []
    );
  });
}

async function expectCommandCall(app: ElectronApplication, command: string): Promise<void> {
  await expect
    .poll(async () => (await readNativeCommandObservations(app)).some((call) => call.command === command))
    .toBe(true);
}

async function openExportDialog(page: Page): Promise<Locator> {
  await page.getByLabel("产品操作").getByRole("button", { name: "导出", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "导出" });
  await expect(dialog).toBeVisible();
  return dialog;
}

async function openDraftParametersDialog(page: Page): Promise<Locator> {
  await page.getByLabel("草稿参数").getByRole("button", { name: "修改" }).click();
  const dialog = page.getByRole("dialog", { name: "草稿参数" });
  await expect(dialog).toBeVisible();
  return dialog;
}

async function expectLatestRealtimeHostSeekTarget(app: ElectronApplication, targetTime: number): Promise<void> {
  await expect
    .poll(async () => {
      const calls = (await readRealtimePreviewHostCalls(app)).filter((call) => call.kind === "seek");
      return calls.at(-1)?.targetTimeMicroseconds ?? null;
    })
    .toBe(targetTime);
}

async function expectNoPreviewFrameCommands(app: ElectronApplication): Promise<void> {
  const calls = await readNativeCommandObservations(app);
  expect(calls.filter((call) => call.command === "requestProjectSessionPreviewFrame")).toHaveLength(0);
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
        ".preview-canvas, .preview-transport, .preview-status-line, .preview-artifact-panel, button, input, select, progress"
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
  const host = await expectStableBox(page.locator(".preview-native-host"), `实时预览画面 ${width}x${height}`);
  const timeline = await expectStableBox(page.locator('[aria-label="时间线"]'), `时间线 ${width}x${height}`);
  const inspector = await expectStableBox(page.locator('[aria-label="属性检查器"]'), `属性检查器 ${width}x${height}`);

  expect(host.width, `实时预览画面宽度 ${width}x${height}`).toBeGreaterThan(120);
  expect(host.height, `实时预览画面高度 ${width}x${height}`).toBeGreaterThan(80);
  expectNoOverlap(host, timeline, "实时预览画面", "时间线");
  expectNoOverlap(host, inspector, "实时预览画面", "属性检查器");
  if (options.requireBoundsUpdate !== false) {
    await latestRealtimePreviewBounds(app);
  }
  return host;
}

async function latestRealtimePreviewBounds(app: ElectronApplication): Promise<NonNullable<RealtimePreviewHostCall["bounds"]>> {
  await expect
    .poll(async () => {
      const latestBounds = (await readRealtimePreviewHostCalls(app)).findLast(
        (call) => (call.kind === "updateSurfaceBounds" || call.kind === "attachSurface") && call.bounds !== undefined
      )?.bounds;
      return latestBounds === undefined ? null : latestBounds;
    })
    .not.toBeNull();

  const latestBounds = (await readRealtimePreviewHostCalls(app)).findLast(
    (call) => (call.kind === "updateSurfaceBounds" || call.kind === "attachSurface") && call.bounds !== undefined
  )?.bounds;
  expect(latestBounds, "实时预览画面应上报 bounds").toBeDefined();
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
  for (const name of ["播放头", "移动", "分割", "裁剪"]) {
    await expect(page.getByRole("spinbutton", { name, exact: true })).toHaveCount(0);
  }

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

async function seekWorkspaceTimelinePlayhead(page: Page, targetTimeUs: number): Promise<void> {
  const rulerTrack = page.locator(".ruler-track");
  const rulerBox = await expectStableBox(rulerTrack, "时间线标尺轨道");
  const ratio = Math.max(0, Math.min(1, targetTimeUs / 10_000_000));
  await page.mouse.click(rulerBox.x + rulerBox.width * ratio, rulerBox.y + rulerBox.height * 0.5);
}

async function dragWorkspaceMaterialToTimeline(page: Page, materialName: string): Promise<void> {
  const materialRow = page.getByRole("article", { name: `素材 ${materialName}` });
  const timelineDropTarget = page.locator('[data-material-drop-target="true"]');

  await expect(materialRow).toBeVisible({ timeout: 20_000 });
  await expect(timelineDropTarget).toBeVisible();
  await materialRow.dragTo(timelineDropTarget);
}

test("Chinese editor workspace opens with required regions and material states", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await expectVisibleWorkspaceRegions(page);

    const topFeatureNav = page.getByRole("navigation", { name: "顶部功能区" });

    await expect(page.getByLabel("项目标题栏")).toBeVisible();
    await expect(page.getByLabel("项目标题", { exact: true })).toContainText("未命名草稿");

    for (const category of WORKSPACE_CATEGORIES) {
      await expect(topFeatureNav.getByRole("button", { name: category })).toBeVisible();
    }
    await expectNoCategoryLabelWrap(page);
    await expectIconButtonsHaveAccessibleNames(page);
    await expectNoLeftSecondaryMenu(page);

    await expect(page.getByRole("button", { name: "导入素材" })).toBeVisible();
    const mediaTools = page.getByRole("group", { name: "媒体工具" });
    await expect(mediaTools).toBeVisible();
    await expect(mediaTools.getByRole("button", { name: "列表视图" })).toHaveAttribute("aria-pressed", "true");
    await expect(mediaTools.getByRole("button", { name: "高级筛选" })).toBeDisabled();
    const materialPanel = page.locator('[aria-label="素材面板"]');
    await expect(page.getByRole("navigation", { name: "资源分类" })).toHaveCount(0);
    const mediaSourceRail = page.getByRole("navigation", { name: "媒体来源" });
    await expect(mediaSourceRail).toBeVisible();
    await expect(mediaSourceRail.getByRole("button", { name: "导入" })).toHaveAttribute("aria-current", "page");
    for (const source of ["我的", "AI生成", "云素材", "官方素材"]) {
      await expect(mediaSourceRail.getByRole("button", { name: source })).toBeDisabled();
    }
    await expect(page.getByLabel("草稿包路径")).toHaveCount(0);
    await expect(page.getByLabel("素材路径")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "导入路径" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "刷新" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "检查丢失" })).toHaveCount(0);
    await expect(materialPanel.getByLabel("资源任务")).toHaveCount(0);
    await expect(materialPanel.getByLabel("资源维护")).toHaveCount(0);
    for (const label of ["更新状态", "清理缓存", "资源任务", "重试", "恢复"]) {
      await expect(materialPanel.getByText(label, { exact: true })).toHaveCount(0);
      await expect(materialPanel.getByRole("button", { name: label })).toHaveCount(0);
    }
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
    await expect(page.getByRole("spinbutton", { name: "预览时间" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "适应窗口" })).toBeVisible();
    await expect(page.getByRole("button", { name: "画面比例" })).toBeVisible();
    await expect(page.getByRole("button", { name: "全屏" })).toHaveCount(0);
    await expectPreviewCanvasAspectRatio(page);

    const timelineControls = page.getByLabel("时间线控制");
    await expect(timelineControls.getByText("素材", { exact: true })).toHaveCount(0);
    await expect(timelineControls.getByRole("button", { name: "添加片段" })).toHaveCount(0);

    await expect(page.getByText("未选择片段")).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "草稿参数" }).first()).toBeVisible();
    await expect(page.getByLabel("草稿参数")).toContainText("画布比例");
    await expect(page.getByRole("button", { name: "修改" })).toBeVisible();
    await expect(page.getByRole("tab")).toHaveCount(0);

    await expect(page.getByRole("article", { name: "素材 城市街景.mp4" })).toContainText("视频");
    await expect(page.getByRole("article", { name: "素材 背景音乐.wav" })).toContainText("音频");
    await expect(page.getByRole("article", { name: "素材 封面图.png" })).toContainText("图片");
    await expect(page.getByRole("article", { name: "素材 城市街景.mp4" })).not.toContainText("可用");
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
    await expect(page.getByRole("button", { name: "添加文字", exact: true })).toBeVisible();
    await expect(page.getByLabel("素材面板")).not.toContainText("微秒");
    await expect(page.getByLabel("默认文字").getByText("字号")).toHaveCount(0);
    await expect(page.getByLabel("默认文字").getByText("描边")).toHaveCount(0);

    await topFeatureNav.getByRole("button", { name: "音频" }).click();
    await expect(page.getByRole("heading", { name: "音频", exact: true }).first()).toBeVisible();
    await expectNoLeftSecondaryMenu(page);
    await expect(page.getByRole("button", { name: "添加音频", exact: true })).toBeVisible();
    await expect(page.getByLabel("素材面板")).not.toContainText("微秒");
    await expect(page.getByText("音量", { exact: true })).toBeVisible();
    await expect(page.getByText("声像", { exact: true })).toBeVisible();
    await expect(page.getByText("淡入", { exact: true })).toBeVisible();
    await expect(page.getByText("淡出", { exact: true })).toBeVisible();

    await topFeatureNav.getByRole("button", { name: "字幕" }).click();
    await expect(page.getByRole("heading", { name: "字幕", exact: true })).toBeVisible();
    await expectNoLeftSecondaryMenu(page);
    await expect(page.getByLabel("素材面板")).not.toContainText("字幕暂未开放");
    await expect(page.getByLabel("字幕 导入字幕")).toContainText("导入字幕");
    await expect(page.getByLabel("SRT 内容")).toBeVisible();

    for (const category of DEFERRED_CATEGORIES) {
      await topFeatureNav.getByRole("button", { name: category }).click();
      await expect(page.getByRole("heading", { name: category })).toBeVisible();
      await expectNoLeftSecondaryMenu(page);
      await expect(page.getByLabel(`${category}暂不可用`)).toContainText(
        category === "数字人" ? "能力暂未开放" : `${category}暂未开放`
      );
      await expect(page.getByLabel(`${category}暂不可用`)).toContainText("当前版本暂不提供该类编辑，切换分类不会修改草稿内容。");
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
    await expect(page.getByLabel("素材面板")).not.toContainText("SRT 内容");
    await expect(page.getByLabel("素材面板")).not.toContainText("导入字幕");
    await expect(page.getByLabel("花字")).toContainText("暂未接入");
    await expect(page.getByLabel("气泡")).toContainText("暂未接入");
    await expect(page.getByRole("button", { name: "添加文字", exact: true })).toBeVisible();

    const resourcePanel = page.getByLabel("素材面板");
    for (const label of ["默认文字", "花字", "气泡"]) {
      await expectLocatorInsideHorizontalContainer(resourcePanel, page.getByLabel(label), `文字面板 ${label}`);
    }
  } finally {
    await app.close();
  }
});

test("text edit routes complete text inspector changes through project session intent observations", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await resetNativeCommandObservations(app, page);
    await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "文字" }).click();
    await page.getByLabel("默认文字").getByLabel("文字内容").fill("开场标题");
    await page.getByRole("button", { name: "添加文字", exact: true }).click();
    await expectCommandCall(app, "addTextSegmentIntent");

    await expect(page.getByRole("button", { name: /片段 开场标题/ })).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByLabel("预览文字")).toContainText("开场标题");

    for (const section of ["文本", "样式", "文本框", "布局"]) {
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
    await expectCommandCall(app, "editSelectedText");

    const previewText = page.getByLabel("预览文字");
    await expect(previewText).toContainText("开场标题 已修改");
    await expect(previewText).toHaveCSS("color", "rgb(24, 199, 255)");
    await expect(previewText).toHaveCSS("font-size", "48px");
    await expect(previewText).toHaveCSS("text-align", "right");
    await expect(previewText).toHaveCSS("letter-spacing", "0.12px");
    await expect(previewText).toHaveCSS("background-color", "rgb(32, 32, 32)");
    const exportDialog = await openExportDialog(page);
    await expect(exportDialog.getByLabel("导出状态", { exact: true })).toContainText("文字已更新，请重新开始导出");

    const calls = await readNativeCommandObservations(app);
    const addTextCall = calls.find((call) => call.command === "addTextSegmentIntent");
    const editTextCall = calls.find((call) => call.command === "editSelectedText");
    expect(addTextCall?.hasDraftField).toBe(false);
    await expect(page.locator('[aria-label="时间线"]')).toContainText("00:00:00.000 / 00:00:03.000");
    expect(editTextCall?.textContent).toBe("开场标题 已修改");
    expect(calls.filter((call) => call.command === "editSelectedText")).toHaveLength(1);
  } finally {
    await app.close();
  }
});

test("bundled font is the default fontRef for new text segments", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await resetNativeCommandObservations(app, page);
    await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "文字" }).click();
    await page.getByLabel("默认文字").getByLabel("文字内容").fill("默认字体");
    await page.getByRole("button", { name: "添加文字", exact: true }).click();
    await expectCommandCall(app, "addTextSegmentIntent");

    await expect(page.getByRole("button", { name: /片段 默认字体/ })).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByLabel("预览文字")).toContainText("默认字体");
    await expect(page.getByLabel("预览文字")).toHaveCSS("font-family", /Noto Sans CJK SC/);
  } finally {
    await app.close();
  }
});

test("音频 add/volume/mute commands update accepted timeline and inspector state", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await resetNativeCommandObservations(app, page);

    await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "音频" }).click();
    await expect(page.getByRole("heading", { name: "音频", exact: true }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: /片段 背景音乐\.wav/ })).toHaveCount(1);

    await seekWorkspaceTimelinePlayhead(page, 8_000_000);
    await page.getByRole("button", { name: "添加音频", exact: true }).click();
    await expectCommandCall(app, "addAudioSegmentIntent");
    await expect(page.getByRole("button", { name: /片段 背景音乐\.wav/ })).toHaveCount(2);
    await expect(page.getByRole("button", { name: /片段 背景音乐\.wav/ }).last()).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByLabel("音频参数")).toBeVisible();
    await expect(page.getByLabel("画面基础表单")).toHaveCount(0);

    await page.getByRole("tab", { name: "音频" }).click();
    await page.getByLabel("音频参数").getByRole("slider", { name: "音量" }).fill("135");
    await page.getByLabel("音频参数").getByRole("slider", { name: "声像" }).fill("-20");
    await page.getByLabel("音频参数").getByRole("spinbutton", { name: "淡入" }).fill("450000");
    await page.getByLabel("音频参数").getByRole("spinbutton", { name: "淡出" }).fill("300000");
    await page.getByLabel("音频参数").getByRole("button", { name: "应用音频" }).click();
    await expectCommandCall(app, "updateSelectedSegmentAudio");
    await expect(page.getByLabel("音频参数").getByRole("slider", { name: "音量" })).toHaveValue("135");
    await expect(page.getByLabel("音频参数").getByRole("slider", { name: "声像" })).toHaveValue("-20");

    await page.getByLabel("音频参数").getByRole("checkbox", { name: "轨道静音" }).click();
    await expectCommandCall(app, "setSelectedTrackMute");
    await expect(page.getByRole("button", { name: "音频轨道 1 静音状态：已静音" })).toBeVisible();

    const calls = await readNativeCommandObservations(app);
    await expect(page.locator('[aria-label="时间线"]')).toContainText("00:00:08.000");
    expect(calls.map((call) => call.command)).toEqual(
      expect.arrayContaining(["addAudioSegmentIntent", "updateSelectedSegmentAudio", "setSelectedTrackMute"])
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

test("字幕 SRT import intent path sends raw SRT once without renderer-created cue segments", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await resetNativeCommandObservations(app, page);
    await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "字幕" }).click();
    await expect(page.getByRole("heading", { name: "字幕", exact: true })).toBeVisible();
    await expect(page.getByLabel("素材面板")).not.toContainText("字幕暂未开放");
    await expect(page.getByLabel("字幕 导入字幕")).toContainText("SRT 字幕");
    await page.getByLabel("SRT 内容").fill("1\n00:00:00,000 --> 00:00:02,000\n第一句字幕\n\n2\n00:00:02,000 --> 00:00:04,000\n第二句字幕\n");
    mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
    await page.screenshot({ path: join(PHASE15_3_SCREENSHOT_DIR, "captions-panel-1280x800.png"), fullPage: true });
    await page.getByRole("button", { name: "导入字幕" }).click();
    await expectCommandCall(app, "importSubtitleSrtIntent");

    await expect(page.getByRole("button", { name: /片段 第一句字幕/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /片段 第一句字幕/ })).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByLabel("预览文字")).toContainText("第一句字幕");
    await expect(page.getByLabel("片段信息")).toContainText("字幕 / 文字");
    const textSection = page.locator('section[aria-label="文本"]');
    await expect(textSection.getByRole("heading", { name: "文本", exact: true })).toBeVisible();
    await expect(textSection).toContainText("SRT 字幕");

    await textSection.locator("textarea").fill("第一句字幕 已校对");
    await page.getByRole("button", { name: "应用文字" }).click();
    await expectCommandCall(app, "editSelectedText");
    await expect(page.getByLabel("预览文字")).toContainText("第一句字幕 已校对");

    const visualForm = page.getByLabel("画面基础表单");
    await visualForm.getByLabel("位置 X", { exact: true }).fill("80");
    await visualForm.getByRole("button", { name: "应用画面" }).click();
    await expectCommandCall(app, "updateSelectedSegmentVisual");

    const calls = await readNativeCommandObservations(app);
    const importCalls = calls.filter((call) => call.command === "importSubtitleSrtIntent");
    expect(importCalls).toHaveLength(1);
    expect(importCalls[0].srtContent).toContain("第二句字幕");
    expect(importCalls[0].srtContent).toContain("00:00:02,000 --> 00:00:04,000");
    expect(importCalls[0].textSource).toBeNull();
    expect(calls.filter((call) => call.command === "addTextSegmentIntent")).toHaveLength(0);
    const editTextCall = calls.find((call) => call.command === "editSelectedText");
    expect(editTextCall?.textSource).toBe("subtitle");
    expect(editTextCall?.textContent).toBe("第一句字幕 已校对");
    expect(calls.find((call) => call.command === "updateSelectedSegmentVisual")?.visual?.transform.position.x).toBe(80);
  } finally {
    await app.close();
  }
});

test("command-only timeline edit calls generated command and applies Rust response", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await resetNativeCommandObservations(app, page);

    const videoSegment = page.getByRole("button", { name: /片段 城市街景\.mp4/ });
    await videoSegment.click();
    await expectCommandCall(app, "selectTimelineItemIntent");
    await expect(page.getByLabel("片段信息")).toContainText("片段参数");
    await expect(page.getByLabel("片段信息")).toContainText("城市街景.mp4");
    await expect(page.getByText("片段ID")).toHaveCount(0);
    await expect(page.getByLabel("画面变换")).toContainText("位置");
    await expect(page.getByRole("button", { name: "添加位置 X关键帧" }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: "添加缩放 X关键帧" }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: "添加不透明度关键帧" }).first()).toBeVisible();
    await expect(page.getByRole("button", { name: "文本关键帧需要文字片段" })).toHaveCount(0);

    await page.getByRole("tab", { name: "音频" }).click();
    await expect(page.getByLabel("音频参数")).toContainText("应用音频");
    await expect(page.getByRole("button", { name: "添加音量关键帧" }).first()).toBeVisible();
    await expect(page.getByLabel("画面变换")).toHaveCount(0);
    await page.getByRole("tab", { name: "动画" }).click();
    await expect(page.getByLabel("动画参数")).toContainText("还没有关键帧");
    await expect(page.getByLabel("属性关键帧")).toContainText("画面");
    await expect(page.getByLabel("属性关键帧")).toContainText("音频");
    await expect(page.getByLabel("属性关键帧")).not.toContainText("特效");
    await page.getByRole("tab", { name: "画面" }).click();
    await expect(page.getByLabel("画面变换")).toContainText("位置");

    await expect(page.getByRole("button", { name: /片段 城市街景\.mp4/ })).toHaveCount(1);
    const callsBeforeAdd = await readNativeCommandObservations(app);
    await seekWorkspaceTimelinePlayhead(page, 8_000_000);
    await page.getByRole("button", { name: "添加片段" }).evaluate((button) => {
      (button as HTMLButtonElement).click();
      (button as HTMLButtonElement).click();
    });
    await expectCommandCall(app, "addTimelineSegmentIntent");
    await expect(page.getByRole("button", { name: /片段 城市街景\.mp4/ })).toHaveCount(2);
    await expect(page.locator('[aria-label="时间线"]')).toContainText("00:00:08.000 / 00:00:12.000");
    await expectLatestRealtimeHostSeekTarget(app, 8_000_000);
    await expectNoPreviewFrameCommands(app);
    await expect(page.getByRole("img", { name: "当前预览帧" })).toHaveCount(0);

    const calls = await readNativeCommandObservations(app);
    const addSegmentCallsBefore = callsBeforeAdd.filter((call) => call.command === "addTimelineSegmentIntent").length;
    const addSegmentCallsAfter = calls.filter((call) => call.command === "addTimelineSegmentIntent").length;
    expect(addSegmentCallsAfter - addSegmentCallsBefore).toBe(1);
    expect(calls.map((call) => call.kind)).toEqual(expect.arrayContaining(["selectTimelineItemIntent", "addTimelineSegmentIntent"]));
  } finally {
    await app.close();
  }
});

test("multitrack controls add target rename lock visibility and mute through Rust commands", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await resetNativeCommandObservations(app, page);

    const trackControls = page.getByRole("group", { name: "添加轨道" });
    await expect(trackControls.getByRole("button", { name: "添加视频轨道" })).toBeVisible();
    await expect(trackControls.getByRole("button", { name: "添加音频轨道" })).toBeVisible();
    await expect(trackControls.getByRole("button", { name: "添加文字轨道" })).toBeVisible();

    await trackControls.getByRole("button", { name: "添加视频轨道" }).click();
    await expectCommandCall(app, "addTrackIntent");
    await expect(page.getByRole("button", { name: "选择轨道 视频轨道 2" })).toBeVisible();

    await page.getByRole("button", { name: "选择轨道 视频轨道 2" }).click();
    await expectCommandCall(app, "selectTimelineItemIntent");
    await expect(page.getByRole("button", { name: "选择轨道 视频轨道 2" })).toHaveAttribute("aria-pressed", "true");

    await seekWorkspaceTimelinePlayhead(page, 8_000_000);
    await page.getByRole("button", { name: "添加片段" }).click();
    await expectCommandCall(app, "addTimelineSegmentIntent");
    await expect(page.locator(".track-row.video").nth(1).getByRole("button", { name: /片段 城市街景\.mp4/ })).toBeVisible();

    const nameInput = page.getByRole("textbox", { name: "视频轨道 2 名称" });
    await nameInput.fill("叠加轨道");
    await nameInput.press("Enter");
    await expectCommandCall(app, "renameSelectedTrack");
    await expect(page.getByRole("button", { name: "选择轨道 叠加轨道" })).toBeVisible();

    await page.getByRole("button", { name: "叠加轨道 锁定状态：未锁定" }).click();
    await expectCommandCall(app, "setSelectedTrackLock");
    await expect(page.getByRole("button", { name: "叠加轨道 锁定状态：已锁定" })).toBeVisible();

    await page.getByRole("button", { name: "叠加轨道 可见状态：画面可见" }).click();
    await expectCommandCall(app, "setSelectedTrackVisibility");
    await expect(page.getByRole("button", { name: "叠加轨道 可见状态：画面隐藏" })).toBeVisible();

    await page.getByRole("button", { name: "音频轨道 1 静音状态：未静音" }).click();
    await expectCommandCall(app, "setSelectedTrackMute");
    await expect(page.getByRole("button", { name: "音频轨道 1 静音状态：已静音" })).toBeVisible();

    await trackControls.getByRole("button", { name: "添加音频轨道" }).click();
    await trackControls.getByRole("button", { name: "添加文字轨道" }).click();
    await expect(page.getByRole("button", { name: "选择轨道 音频轨道 2" })).toBeVisible();
    await expect(page.getByRole("button", { name: "选择轨道 文字轨道 2" })).toBeVisible();

    const calls = await readNativeCommandObservations(app);
    expect(calls.map((call) => call.command)).toEqual(
      expect.arrayContaining([
        "addTrackIntent",
        "selectTimelineItemIntent",
        "addTimelineSegmentIntent",
        "renameSelectedTrack",
        "setSelectedTrackLock",
        "setSelectedTrackVisibility",
        "setSelectedTrackMute"
      ])
    );
    expect(calls.filter((call) => call.command === "addTrackIntent")).toHaveLength(3);
  } finally {
    await app.close();
  }
});

test("material import routes through project session intent observations", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await resetNativeCommandObservations(app, page);

    await page.getByRole("button", { name: "导入素材" }).click();
    await expectCommandCall(app, "importMaterial");

    const calls = await readNativeCommandObservations(app);
    expect(calls.map((call) => call.command)).toContain("importMaterial");
  } finally {
    await app.close();
  }
});

test("auto canvas adopts the first imported portrait material without renderer-owned canvas math", async () => {
  const { app, page } = await launchWorkspaceApp({
    env: {
      VIDEO_EDITOR_TEST_WORKSPACE_FIXTURE: "blank",
      VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([PORTRAIT_VIDEO_FIXTURE])
    }
  });

  try {
    await resetNativeCommandObservations(app, page);

    await expect(page.getByText("还没有素材")).toBeVisible();
    await page.getByRole("button", { name: "导入素材" }).click();
    await expectCommandCall(app, "importMaterial");
    await expect(page.locator('[aria-label="素材 p0-portrait-testsrc.mp4"]')).toBeVisible();

    await dragWorkspaceMaterialToTimeline(page, "p0-portrait-testsrc.mp4");
    await expectCommandCall(app, "addTimelineSegmentIntent");
    await expect(page.getByRole("button", { name: /片段 p0-portrait-testsrc\.mp4/ })).toBeVisible();
    await expect(
      page.getByLabel("预览窗口").getByText("画布 9:16 · 180 x 320 · 30000/1001 fps", { exact: true })
    ).toBeVisible();
    await expect(page.getByLabel("预览选中框")).toHaveAttribute("data-fit-mode", "fit");
  } finally {
    await app.close();
  }
});

test("developer diagnostics preview time input seeks realtime host without artifact frame requests", async () => {
  const { app, page } = await launchWorkspaceApp({ showDeveloperDiagnostics: true });

  try {
    await resetNativeCommandObservations(app, page);

    await page.getByLabel("预览时间").fill("1200000");
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:01.200");
    await expectLatestRealtimeHostSeekTarget(app, 1_200_000);
    await expectNoPreviewFrameCommands(app);
    await expect(page.getByRole("img", { name: "当前预览帧" })).toHaveCount(0);

    const dialog = await openDraftParametersDialog(page);
    await dialog.getByLabel("帧率", { exact: true }).selectOption("custom");
    await dialog.getByLabel("帧率分子").fill("30000");
    await dialog.getByLabel("帧率分母").fill("1001");
    await dialog.getByRole("button", { name: "应用草稿参数" }).click();
    await expectCommandCall(app, "updateDraftCanvasConfig");
    await expect(page.getByLabel("预览窗口")).toContainText("30000/1001 fps");

    await resetNativeCommandObservations(app, page);
    await page.getByLabel("预览时间").fill("0");
    await expectLatestRealtimeHostSeekTarget(app, 0);
    await page.getByLabel("预览时间").fill("1200000");
    await expectLatestRealtimeHostSeekTarget(app, 1_200_000);

    await page.getByRole("button", { name: "下一帧" }).click();
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:01.233");
    await expectLatestRealtimeHostSeekTarget(app, 1_233_367);

    await page.getByRole("button", { name: "上一帧" }).click();
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:01.200");
    await expectLatestRealtimeHostSeekTarget(app, 1_200_000);
    await expectNoPreviewFrameCommands(app);
  } finally {
    await app.close();
  }
});

test("预览播放按钮使用实时预览画面而不是连续请求预览帧", async () => {
  const { app, page } = await launchWorkspaceApp({
    env: {
      VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([PORTRAIT_VIDEO_FIXTURE])
    }
  });

  try {
    await page.getByRole("button", { name: "导入素材" }).click();
    await expect(page.getByRole("article", { name: "素材 p0-portrait-testsrc.mp4" })).toContainText("视频", { timeout: 20_000 });
    await seekWorkspaceTimelinePlayhead(page, 8_000_000);
    await dragWorkspaceMaterialToTimeline(page, "p0-portrait-testsrc.mp4");
    await expect(page.getByRole("button", { name: /片段 p0-portrait-testsrc\.mp4/ })).toBeVisible();
    await resetNativeCommandObservations(app, page);

    const previewControls = page.getByRole("group", { name: "预览播放控制" });
    await expect(previewControls.getByRole("button", { name: "播放" })).toBeEnabled({ timeout: 20_000 });
    await previewControls.getByRole("button", { name: "播放" }).click();
    await page.waitForTimeout(500);

    const playbackFrameRequests = (await readNativeCommandObservations(app)).filter((call) => call.command === "requestProjectSessionPreviewFrame");
    expect(playbackFrameRequests).toHaveLength(0);
  } finally {
    await app.close();
  }
});

test("音频预览 controls call explicit native APIs and preserve state after rejection", async () => {
  const { app, page } = await launchWorkspaceApp({ showDeveloperDiagnostics: true });

  try {
    await resetNativeCommandObservations(app, page);

    await expect(page.getByLabel("音频预览状态")).toContainText("音频就绪");
    await expect(page.getByLabel("输出设备状态")).toContainText("系统默认");

    await page.getByRole("button", { name: "重试音频" }).click();
    await expectCommandCall(app, "createAudioPreviewSession");
    await expectCommandCall(app, "cancelAudioPreview");
    await expectCommandCall(app, "getAudioPreviewStatus");
    await expect(page.getByLabel("音频预览状态")).toContainText("音频就绪");

    await seekWorkspaceTimelinePlayhead(page, 1_200_000);
    await expectCommandCall(app, "seekAudioPreview");
    await page.getByRole("button", { name: "停止预览" }).first().click();
    await expectCommandCall(app, "stopAudioPreview");

    const calls = await readNativeCommandObservations(app);
    expect(calls.map((call) => call.command)).toEqual(
      expect.arrayContaining([
        "createAudioPreviewSession",
        "stopAudioPreview",
        "seekAudioPreview",
        "cancelAudioPreview",
        "getAudioPreviewStatus"
      ])
    );
    const audioCalls = calls.filter((call) =>
      [
        "createAudioPreviewSession",
        "cancelAudioPreview",
        "getAudioPreviewStatus",
        "seekAudioPreview",
        "stopAudioPreview",
        "listAudioOutputDevices",
        "selectAudioOutputDevice"
      ].includes(
        call.command
      )
    );
    expect(audioCalls.every((call) => call.hasDraftField === false)).toBe(true);
    expect(
      audioCalls.every((call) => typeof call.projectSessionId === "string" && typeof call.expectedRevision === "number")
    ).toBe(true);
  } finally {
    await app.close();
  }
});

test("音频预览 panel and inspector expose production audio controls through updateSelectedSegmentAudio intent", async () => {
  const { app, page } = await launchWorkspaceApp({ showDeveloperDiagnostics: true });

  try {
    await resetNativeCommandObservations(app, page);

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
    await expect
      .poll(async () => (await readNativeCommandObservations(app)).some((call) => call.command === "updateSelectedSegmentAudio"))
      .toBe(true);

    const calls = await readNativeCommandObservations(app);
    expect(calls.map((call) => call.command)).toContain("updateSelectedSegmentAudio");
  } finally {
    await app.close();
  }
});

test("波形 display uses Rust-shaped peak payloads and keeps fallback states stable", async () => {
  const { app, page } = await launchWorkspaceApp({ showDeveloperDiagnostics: true });

  try {
    await resetNativeCommandObservations(app, page);

    const audioSegment = page.getByRole("button", { name: /片段 背景音乐\.wav/ }).first();
    await expect(audioSegment.locator('[aria-label="音频波形"]')).toBeVisible();
    await expect(audioSegment.locator('[aria-label="音频波形"] .audio-waveform-bar')).toHaveCount(16);
    await expect(page.getByText("波形就绪")).toBeVisible();
    await expectCommandCall(app, "getWaveformDisplayPeaks");
    await expectCommandCall(app, "refreshWaveformStatus");
    const waveformCalls = (await readNativeCommandObservations(app)).filter(
      (call) => call.command === "getWaveformDisplayPeaks" || call.command === "refreshWaveformStatus"
    );
    expect(waveformCalls.every((call) => call.hasDraftField === false)).toBe(true);
    expect(
      waveformCalls.every((call) => typeof call.projectSessionId === "string" && typeof call.expectedRevision === "number")
    ).toBe(true);

    const waveformBox = await expectStableBox(audioSegment.locator('[aria-label="音频波形"]'), "音频波形");
    expect(waveformBox.height, "音频波形固定 14px 高").toBeLessThanOrEqual(14);
    await setViewportSizeAndVerifyLayout(app, page, 1280, 800);
    await setViewportSizeAndVerifyLayout(app, page, 1120, 720);
  } finally {
    await app.close();
  }

  const pending = await launchWorkspaceApp({
    showDeveloperDiagnostics: true,
    env: { VIDEO_EDITOR_TEST_AUDIO_WAVEFORM_STATUS: "pending" }
  });
  try {
    await expect(pending.page.getByLabel("波形状态")).toContainText("波形生成中");
    await expect(pending.page.getByRole("button", { name: /片段 背景音乐\.wav/ }).first().locator('[aria-label="音频波形占位"]')).toBeVisible();
  } finally {
    await pending.app.close();
  }

  const failed = await launchWorkspaceApp({
    showDeveloperDiagnostics: true,
    env: { VIDEO_EDITOR_TEST_AUDIO_WAVEFORM_STATUS: "failed" }
  });
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

    expect(bridgeShape).toEqual([
      "pause",
      "play",
      "seek",
      "stop",
      "subscribeTelemetry",
      "updateHostRect",
      "updateProjectSessionSnapshot"
    ]);

    const updateResult = await page.evaluate(() =>
      window.videoEditorRealtimePreviewHost?.updateHostRect({
        x: 12.7,
        y: 34.2,
        width: 320.9,
        height: 180.1,
        scaleFactorMillis: 1250.6
      })
    );
    expect(JSON.stringify(updateResult)).not.toMatch(/parentHandle|parentHandleHex|hwnd|nsview|commandEncoder|cacheKey/i);

    const telemetry = await page.evaluate(() => {
      return new Promise<unknown>((resolve) => {
        const bridge = window.videoEditorRealtimePreviewHost;
        if (bridge === undefined) {
          resolve(null);
          return;
        }
        let unsubscribe = () => undefined;
        const timer = window.setTimeout(() => {
          unsubscribe();
          resolve(null);
        }, 5_000);
        unsubscribe = bridge.subscribeTelemetry((state) => {
          window.clearTimeout(timer);
          unsubscribe();
          resolve(state);
        });
      });
    });
    expect(JSON.stringify(telemetry)).not.toMatch(/parentHandle|parentHandleHex|hwnd|nsview|commandEncoder|cacheKey/i);
    await expect
      .poll(async () => (await readRealtimePreviewHostCalls(app)).some((call) => call.kind === "subscribeTelemetry"))
      .toBe(true);
    await expect
      .poll(async () => (await readRealtimePreviewHostCalls(app)).some((call) => call.kind === "nativePreviewEventBridgeInstalled"))
      .toBe(true);

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
    showDeveloperDiagnostics: true,
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

    await expect(page.getByLabel("实时预览状态")).toContainText("等待 GPU 合成");
    await expect(page.getByLabel("实时预览数据")).toContainText("诊断来源：运行时帧请求");
    await expect(page.getByLabel("实时预览数据")).toContainText("运行时帧");
    expect((await readRealtimePreviewHostCalls(app)).some((call) => call.kind === "requestFirstFrame")).toBe(true);

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

test("实时预览 native preview attach failure displays unavailable diagnostics", async () => {
  const { app, page } = await launchWorkspaceApp({
    showDeveloperDiagnostics: true,
    env: {
      VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_ATTACH_FAILURE: "1"
    }
  });

  try {
    await expectNativePreviewHostLayout(app, page, 1280, 800, { requireBoundsUpdate: false });
    await expect(page.getByLabel("实时预览状态")).toContainText("实时预览不可用");
    await expect(page.getByLabel("实时预览不可用")).toContainText("实时预览不可用");
    await expect(page.getByLabel("实时预览不可用")).not.toContainText("HWND");
    await expect(page.getByLabel("实时预览不可用")).not.toContainText("NSView");
  } finally {
    await app.close();
  }
});

test("实时预览 telemetry keeps runtime frame diagnostics out of product-ready status", async () => {
  const { app, page } = await launchWorkspaceApp({
    showDeveloperDiagnostics: true,
    env: {
      VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_FIRST_FRAME: "1"
    }
  });

  try {
    await expectNativePreviewHostLayout(app, page, 1280, 800);
    await expect(page.getByLabel("实时预览状态")).toContainText("等待 GPU 合成");
    await expect(page.getByLabel("实时预览数据")).toContainText("诊断来源：运行时帧请求");
    await expect(page.getByLabel("实时预览数据")).toContainText("运行时帧");
    await expect(page.getByLabel("实时预览数据")).not.toContainText("FFmpeg");
    await expect(page.getByLabel("实时预览备用产物")).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("实时预览 telemetry shows runtime seek frame diagnostics without fallback artifact", async () => {
  const { app, page } = await launchWorkspaceApp({
    showDeveloperDiagnostics: true,
    env: {
      VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_SEEK_FRAME: "1"
    }
  });

  try {
    await expectNativePreviewHostLayout(app, page, 1120, 720);
    await expect(page.getByLabel("实时预览状态")).toContainText("等待 GPU 合成");
    await expect(page.getByLabel("实时预览数据")).toContainText("诊断来源：运行时帧请求");
    await expect(page.getByLabel("实时预览数据")).toContainText("目标 00:00:01.200");
    expect((await readRealtimePreviewHostCalls(app)).some((call) => call.kind === "requestSeekFrame")).toBe(true);
    await expect(page.getByLabel("实时预览备用产物")).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("实时预览 product host does not expose artifact fallback as playback", async () => {
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

  const ignoredFallback = await launchWorkspaceApp();

  try {
    await expectNativePreviewHostLayout(ignoredFallback.app, ignoredFallback.page, 1280, 800);
    const previewWindowText = (await ignoredFallback.page.getByLabel("预览窗口").textContent()) ?? "";
    expect(previewWindowText).not.toMatch(/当前画面暂不能实时播放|备用产物：媒体运行环境/);
    await expect(ignoredFallback.page.getByLabel("实时预览受限")).toHaveCount(0);
    await expect(ignoredFallback.page.getByLabel("实时预览备用产物")).toHaveCount(0);
  } finally {
    await ignoredFallback.app.close();
  }
});

test("baseline preview capability does not productize realtime fallback copy", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await expectNativePreviewHostLayout(app, page, 1280, 800);
    const previewWindowText = (await page.getByLabel("预览窗口").textContent()) ?? "";
    expect(previewWindowText).not.toMatch(
      /Mock|backend|fallback|cache|artifact|nativeVideoBridge|renderGraphGpu|requestProjectSessionPreviewFrame|备用产物|缓存|降级|排队|渲染|运行时帧/
    );
    await expect(page.getByLabel("实时预览备用产物")).toHaveCount(0);
    await expect(page.getByLabel("实时预览受限")).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("developer diagnostics do not expose artifact fallback as realtime playback", async () => {
  const { app, page } = await launchWorkspaceApp({
    showDeveloperDiagnostics: true
  });

  try {
    await expectNativePreviewHostLayout(app, page, 1280, 800);
    await expect(page.getByLabel("实时预览数据")).not.toContainText("备用产物：媒体运行环境");
    await expect(page.getByLabel("实时预览数据")).not.toContainText("降级 1");
    await expect(page.getByLabel("实时预览备用产物")).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("developer diagnostics display Rust-reported realtime cancellation counters", async () => {
  const { app, page } = await launchWorkspaceApp({
    showDeveloperDiagnostics: true,
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
  expect(previewMonitorSource, "renderer must not create render graph/cache logic").not.toMatch(
    /buildRenderGraph|RenderGraph\s*\(|cacheKey\s*[:=]|fallbackLadder\s*[:=]/i
  );
  expect(previewMonitorSource, "renderer must not assign fallback reasons").not.toMatch(/fallbackReason\s*=(?!=)/i);
  expect(viewModelSource, "display model should not inspect drafts to infer support").not.toMatch(
    /if\s*\([^)]*(?:draft|material)[^)]*\)[\s\S]{0,160}fallback/i
  );
});

test("resource panel carries ready thumbnail display refs from artifact status", () => {
  const panel = resourcePanelFromArtifactStatus({
    sessionId: "desktop-artifact-session",
    statusLabel: "资源已就绪",
    refreshAvailable: true,
    quota: {
      statusLabel: "缓存空间正常",
      severity: "ready",
      usedLabel: "1 MB",
      reclaimableLabel: "0 MB",
      releasedLabel: "0 MB",
      cleanupAvailable: false
    },
    tasks: [],
    materials: [
      {
        materialId: "material-video",
        materialLabel: "城市街景.mp4",
        artifactKind: "thumbnail",
        status: "ready",
        statusLabel: "已生成",
        progressPerMille: null,
        canRefresh: true,
        canRetry: false,
        canResume: false,
        canCancel: false,
        displayRef: {
          label: "缩略图",
          projectRelativeRef: "derived/thumbnails/material-video.jpg",
          artifactKind: "thumbnail"
        }
      },
      {
        materialId: "material-video",
        materialLabel: "城市街景.mp4",
        artifactKind: "waveform",
        status: "ready",
        statusLabel: "已生成",
        progressPerMille: null,
        canRefresh: true,
        canRetry: false,
        canResume: false,
        canCancel: false,
        displayRef: {
          label: "波形",
          projectRelativeRef: "derived/waveforms/material-video.json",
          artifactKind: "waveform"
        }
      },
      {
        materialId: "material-audio",
        materialLabel: "背景音乐.wav",
        artifactKind: "thumbnail",
        status: "running",
        statusLabel: "生成中",
        progressPerMille: 400,
        canRefresh: false,
        canRetry: false,
        canResume: false,
        canCancel: true,
        displayRef: {
          label: "未完成缩略图",
          projectRelativeRef: "derived/thumbnails/material-audio.jpg",
          artifactKind: "thumbnail"
        }
      }
    ]
  });

  expect(panel.materials.find((material) => material.materialId === "material-video")?.thumbnailRef).toEqual({
    label: "缩略图",
    projectRelativeRef: "derived/thumbnails/material-video.jpg",
    artifactKind: "thumbnail"
  });
  expect(panel.materials.find((material) => material.materialId === "material-audio")?.thumbnailRef).toBeNull();
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
    "Rust-owned session, clock, generation, capability classification,",
    "telemetry, and GPU composition",
    "H.264 software video frame provider/cache",
    "Renderer responsibilities are UI-only",
    "TextParityUnsupported",
    "No-Fallback Product Policy",
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
    backend: "mediaArtifact",
    presentedFrameCount: 0,
    currentRequestCanceled: true,
    fallbackReason: "mediaArtifactGenerated",
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
  expect(summarizeRealtimePreviewProductDisplay({ ...supported, backend: "renderGraphGpu" })).toBe("实时预览已接入");
  expect(summarizeRealtimePreviewProductDisplay(fallback)).toBe("实时预览不可用：GPU 合成播放尚未接入");
  expect(fallback.fallbackArtifactVisible).toBe(true);
});

test("播放头支持时间线标尺点击和拖动寻帧到实时预览画面", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await resetNativeCommandObservations(app, page);

    const rulerTrack = page.locator(".ruler-track");
    const rulerBox = await expectStableBox(rulerTrack, "时间线标尺轨道");
    await page.mouse.click(rulerBox.x + rulerBox.width * 0.5, rulerBox.y + rulerBox.height * 0.5);
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:05.000");
    await expectLatestRealtimeHostSeekTarget(app, 5_000_000);
    await expectNoPreviewFrameCommands(app);

    await resetNativeCommandObservations(app, page);
    const playhead = page.locator(".playhead");
    const playheadBox = await expectStableBox(playhead, "播放头拖动线");
    await page.mouse.move(playheadBox.x + playheadBox.width / 2, playheadBox.y + 4);
    await page.mouse.down();
    await page.mouse.move(rulerBox.x + rulerBox.width * 0.75, playheadBox.y + 4);
    await page.mouse.up();

    await expect(page.getByLabel("当前时间码")).toContainText(/00:00:07\.[4-6][0-9][0-9]/);
    await expect
      .poll(async () => {
        const latestSeek = (await readRealtimePreviewHostCalls(app)).filter((call) => call.kind === "seek").at(-1);
        const target = latestSeek?.targetTimeMicroseconds ?? 0;
        return target >= 7_400_000 && target <= 7_650_000;
      })
      .toBe(true);
    await expectNoPreviewFrameCommands(app);
  } finally {
    await app.close();
  }
});

test("草稿参数画布 UI 通过 Rust command 更新预览读数并保存截图", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await resetNativeCommandObservations(app, page);
    await expectNoLeftSecondaryMenu(page);

    const inspector = page.getByLabel("草稿参数");
    await expect(inspector).toContainText("草稿参数");
    for (const label of ["画布比例", "画布尺寸", "帧率", "画布背景"]) {
      await expect(inspector).toContainText(label);
    }
    await expect(inspector.getByRole("button", { name: "修改" })).toBeVisible();
    await expect(inspector.getByRole("button", { name: "应用草稿参数" })).toHaveCount(0);
    await expect(page.getByText("坐标以画布中心为原点")).toHaveCount(0);
    await expect(page.getByText("图片背景")).toHaveCount(0);

    const dialog = await openDraftParametersDialog(page);
    await dialog.getByRole("group", { name: "画布比例" }).getByRole("button", { name: "9:16" }).click();
    await expect(dialog.getByLabel("画布宽度")).toHaveValue("1080");
    await expect(dialog.getByLabel("画布高度")).toHaveValue("1920");
    await dialog.getByRole("group", { name: "画布背景" }).getByRole("button", { name: "模糊填充" }).click();
    await expect(dialog).toContainText("模糊填充 · 降级");
    await dialog.getByRole("button", { name: "应用草稿参数" }).click();

    await expectCommandCall(app, "updateDraftCanvasConfig");
    await expect(
      page.getByLabel("预览窗口").getByText("画布 9:16 · 1080 x 1920 · 30 fps", { exact: true })
    ).toBeVisible();
    await expect(page.getByText("模糊填充 · 降级").first()).toBeVisible();

    const calls = await readNativeCommandObservations(app);
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
    await resetNativeCommandObservations(app, page);

    let dialog = await openDraftParametersDialog(page);
    await dialog.getByLabel("帧率", { exact: true }).selectOption("custom");
    await dialog.getByLabel("帧率分子").fill("30000");
    await dialog.getByLabel("帧率分母").fill("1001");
    await dialog.getByRole("button", { name: "应用草稿参数" }).click();

    await expectCommandCall(app, "updateDraftCanvasConfig");
    await expect(page.getByLabel("预览窗口")).toContainText("30000/1001 fps");

    dialog = await openDraftParametersDialog(page);
    await dialog.getByRole("group", { name: "画布背景" }).getByRole("button", { name: "纯色" }).click();
    await dialog.getByRole("button", { name: "应用草稿参数" }).click();

    await expect
      .poll(async () => (await readNativeCommandObservations(app)).filter((call) => call.command === "updateDraftCanvasConfig").length)
      .toBe(2);

    const canvasCalls = (await readNativeCommandObservations(app)).filter((call) => call.command === "updateDraftCanvasConfig");
    expect(canvasCalls.at(-1)?.canvasConfig?.frameRate).toEqual({ numerator: 30000, denominator: 1001 });
  } finally {
    await app.close();
  }
});

test("画布变更后旧预览和导出派生状态失效", async () => {
  const { app, page } = await launchWorkspaceApp({ showDeveloperDiagnostics: true, startup: "newProject" });

  try {
    await resetNativeCommandObservations(app, page);

    let exportDialog = await openExportDialog(page);
    await exportDialog.getByRole("button", { name: "开始导出" }).click();
    await expectCommandCall(app, "startExport");
    await exportDialog.getByRole("button", { name: "查询导出状态" }).click();
    await expectCommandCall(app, "getExportJobStatus");
    await expect(exportDialog.getByLabel("输出校验")).toContainText("1920x1080");
    await expect(exportDialog.getByRole("button", { name: "查询导出状态" })).toBeEnabled();
    await exportDialog.getByRole("button", { name: "关闭" }).click();

    const dialog = await openDraftParametersDialog(page);
    await dialog.getByRole("group", { name: "画布比例" }).getByRole("button", { name: "1:1" }).click();
    await dialog.getByRole("button", { name: "应用草稿参数" }).click();
    await expectCommandCall(app, "updateDraftCanvasConfig");

    await expect(page.getByRole("img", { name: "当前预览帧" })).toHaveCount(0);
    await expect(page.getByLabel("预览产物")).toHaveCount(0);
    await expect(page.getByLabel("预览状态", { exact: true })).toContainText("画布已更新，预览待刷新");
    exportDialog = await openExportDialog(page);
    await expect(exportDialog.getByLabel("导出状态", { exact: true })).toContainText("草稿已更新，请重新开始导出");
    await expect(exportDialog.getByLabel("输出校验")).toContainText("输出校验待完成");
    await expect(exportDialog.getByRole("button", { name: "查询导出状态" })).toBeDisabled();
    await expect(exportDialog.getByRole("button", { name: "取消导出" })).toBeDisabled();
  } finally {
    await app.close();
  }
});

test("画面变换 command-only transform 通过 Rust command 更新 UI 并清理派生状态", async () => {
  const { app, page } = await launchWorkspaceApp({ showDeveloperDiagnostics: true });

  try {
    await resetNativeCommandObservations(app, page);
    await expectNoLeftSecondaryMenu(page);

    let exportDialog = await openExportDialog(page);
    await exportDialog.getByRole("button", { name: "开始导出" }).click();
    await expectCommandCall(app, "startExport");
    await exportDialog.getByRole("button", { name: "查询导出状态" }).click();
    await expectCommandCall(app, "getExportJobStatus");
    await expect(exportDialog.getByLabel("输出校验")).toContainText("1920x1080");
    await exportDialog.getByRole("button", { name: "关闭" }).click();

    await page.getByRole("button", { name: /片段 城市街景\.mp4/ }).click();
    await expectCommandCall(app, "selectTimelineItemIntent");

    const visualForm = page.getByLabel("画面基础表单");
    await expect(page.getByLabel("画面变换")).toContainText("基础");
    for (const label of ["显示画面", "位置", "缩放", "旋转", "不透明度", "适应方式", "裁剪", "背景填充"]) {
      await expect(visualForm).toContainText(label);
    }
    await expect(visualForm).not.toContainText("混合模式");
    await expect(visualForm).not.toContainText("蒙版");
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

    await expectCommandCall(app, "updateSelectedSegmentVisual");
    await expectLatestRealtimeHostSeekTarget(app, 0);
    await expect(visualForm.getByLabel("位置 X", { exact: true })).toHaveValue("160");
    await expect(visualForm.getByLabel("位置 Y", { exact: true })).toHaveValue("-90");
    await expect(visualForm.getByLabel("缩放 X", { exact: true })).toHaveValue("1250");
    await expect(visualForm.getByLabel("缩放 Y", { exact: true })).toHaveValue("850");

    await expect(page.getByRole("img", { name: "当前预览帧" })).toHaveCount(0);
    await expect(page.getByLabel("预览产物")).toHaveCount(0);
    await expect(page.getByLabel("预览状态", { exact: true })).toContainText("画面变换已更新，预览待刷新");
    exportDialog = await openExportDialog(page);
    await expect(exportDialog.getByLabel("导出状态", { exact: true })).toContainText("画面变换已更新，请重新开始导出");
    await expect(exportDialog.getByLabel("输出校验")).toContainText("输出校验待完成");
    await expect(exportDialog.getByRole("button", { name: "查询导出状态" })).toBeDisabled();
    await expect(exportDialog.getByRole("button", { name: "取消导出" })).toBeDisabled();
    await exportDialog.getByRole("button", { name: "关闭" }).click();

    const visualCall = (await readNativeCommandObservations(app)).find((call) => call.command === "updateSelectedSegmentVisual");
    expect(visualCall?.kind).toBe("updateSelectedSegmentVisual");
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

test("selection preview overlay follows accepted visible segment and allows direct canvas interaction", async () => {
  const { app, page } = await launchWorkspaceApp({ showDeveloperDiagnostics: true });

  try {
    await resetNativeCommandObservations(app, page);

    await expect(page.getByLabel("预览选中框")).toHaveCount(0);
    await expect(page.getByRole("img", { name: "当前预览帧" })).toHaveCount(0);

    await page.getByRole("button", { name: /片段 城市街景\.mp4/ }).click();
    await expectCommandCall(app, "selectTimelineItemIntent");

    const overlay = page.getByLabel("预览选中框");
    await expect(overlay).toBeVisible();
    await expect(overlay).toHaveAttribute("data-segment-id", "segment-main-video");
    await expect(page.getByRole("img", { name: "当前预览帧" })).toHaveCount(0);

    const overlayPointerEvents = await overlay.evaluate((element) => window.getComputedStyle(element).pointerEvents);
    expect(overlayPointerEvents).toBe("auto");

    await setViewportSizeAndVerifyLayout(app, page, 1280, 800);
    await expect(overlay).toBeVisible();
    await setViewportSizeAndVerifyLayout(app, page, 1120, 720);
    await expect(overlay).toBeVisible();
  } finally {
    await app.close();
  }
});

test("concurrent material commands are blocked while a timeline edit is pending", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await resetNativeCommandObservations(app, page);

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

    await expectCommandCall(app, "addTimelineSegmentIntent");

    const draftMutatingCalls = (await readNativeCommandObservations(app)).filter(
      (call) => call.command === "addTimelineSegmentIntent" || call.command === "importMaterial"
    );
    expect(draftMutatingCalls.map((call) => call.command)).toEqual(["addTimelineSegmentIntent"]);
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
    await seekWorkspaceTimelinePlayhead(page, 1_200_000);
    await expect(page.getByLabel("当前时间码")).toContainText("00:00:01.200");

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

test("right inspector hides production-forbidden diagnostics and fits required viewports", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    const inspector = page.getByLabel("属性检查器");
    const forbidden = /segmentId|trackId|material-workspace|media\/|\/tmp|cache|artifact|diagnostic|backend|debug|诊断|路径|缓存/i;

    for (const [width, height] of [
      [1280, 800],
      [1120, 720]
    ] as const) {
      await setViewportSizeAndVerifyLayout(app, page, width, height);
      await expect(inspector).not.toContainText(forbidden);
      await expect(inspector.getByLabel("草稿参数")).toContainText("草稿参数");

      const overflow = await inspector.evaluate((element) => ({
        horizontal: element.scrollWidth > element.clientWidth + 1,
        vertical: element.scrollHeight >= element.clientHeight
      }));
      expect(overflow.horizontal, `inspector must not widen at ${width}x${height}`).toBe(false);
    }

    await page.getByRole("button", { name: /片段 城市街景\.mp4/ }).click();
    await expect(page.getByLabel("画面基础表单")).toBeVisible();
    await expect(inspector).not.toContainText(forbidden);
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

test("导出控制通过显式导出 API 更新导出状态并保存截图", async () => {
  const { app, page } = await launchWorkspaceApp({ startup: "newProject" });

  try {
    await resetNativeCommandObservations(app, page);

    const exportDialog = await openExportDialog(page);
    await expect(page.getByLabel("预览窗口").getByLabel("导出面板")).toHaveCount(0);
    await expect(exportDialog.getByLabel("输出路径")).toHaveValue("video-editor-export.mp4");
    await expect(exportDialog.getByLabel("导出预设")).toHaveValue("h264AacBalanced");
    await expect(exportDialog.getByRole("button", { name: "开始导出" })).toBeVisible();
    await expect(exportDialog.getByRole("button", { name: "查询导出状态" })).toBeDisabled();
    await expect(exportDialog.getByRole("button", { name: "取消导出" })).toBeDisabled();

    await exportDialog.getByLabel("输出路径").fill("/tmp/video-editor-export.mp4");
    await exportDialog.getByRole("button", { name: "开始导出" }).click();
    await expectCommandCall(app, "startExport");
    await expect(exportDialog.getByLabel("导出进度")).toContainText("导出中");
    await expect(exportDialog.getByLabel("导出进度")).toContainText("12%");
    await expect(exportDialog.getByLabel("导出状态", { exact: true })).toContainText("导出任务已启动");
    await expect(exportDialog.getByRole("button", { name: "取消导出" })).toBeEnabled();

    await exportDialog.getByRole("button", { name: "取消导出" }).click();
    await expectCommandCall(app, "cancelExport");
    await expect(exportDialog.getByLabel("导出进度")).toContainText("已取消");
    await expect(exportDialog.getByLabel("导出状态", { exact: true })).toContainText("导出已取消");

    await exportDialog.getByRole("button", { name: "开始导出" }).click();
    await exportDialog.getByRole("button", { name: "查询导出状态" }).click();
    await expectCommandCall(app, "getExportJobStatus");
    await expect(exportDialog.getByLabel("导出进度")).toContainText("已完成");
    await expect(exportDialog.getByLabel("导出进度")).toContainText("100%");
    await expect(exportDialog.getByLabel("导出状态", { exact: true })).toContainText("导出完成，输出校验通过");
    await expect(exportDialog.getByLabel("输出校验")).toContainText("1920x1080");
    await expect(exportDialog.getByLabel("输出校验")).toContainText("含音频");

    const calls = await readNativeCommandObservations(app);
    expect(calls.map((call) => call.command)).toEqual(
      expect.arrayContaining(["startExport", "cancelExport", "getExportJobStatus"])
    );
    const startCall = calls.find((call) => call.command === "startExport");
    expect(startCall?.outputPath).toBe("/tmp/video-editor-export.mp4");
    expect(startCall?.preset).toBe("h264AacBalanced");
    await exportDialog.getByRole("button", { name: "关闭" }).click();

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

test("素材资源状态 uses explicit native artifact APIs", async () => {
  const { app, page } = await launchWorkspaceApp({ mockArtifactCommands: true, showDeveloperDiagnostics: true });

  try {
    await resetNativeCommandObservations(app, page);

    await expect(page.getByLabel("素材资源状态").first()).toBeVisible();
    await page.getByRole("button", { name: "更新状态" }).click();
    await page.getByRole("button", { name: "取消生成" }).click();
    await page.getByRole("button", { name: "重新生成" }).click();
    await page.getByRole("button", { name: "继续生成" }).click();
    await page.getByRole("button", { name: "清理缓存" }).click();
    await page.getByRole("button", { name: "确认清理缓存" }).click();

    await expect
      .poll(async () => (await readNativeCommandObservations(app)).map((call) => call.command))
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
  const { app, page } = await launchWorkspaceApp({ mockArtifactCommands: true, showDeveloperDiagnostics: true });

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
    showDeveloperDiagnostics: true,
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
  const { app, page } = await launchWorkspaceApp({ mockArtifactCommands: true, showDeveloperDiagnostics: true });

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
      "分割所选片段",
      "删除所选片段",
      "缩小时间线",
      "放大时间线"
    ]) {
      await expect(timelineControls.getByRole("button", { name: label })).toBeVisible();
    }

    await expect(page.getByLabel("时间线标尺")).toContainText("00:00");
    await expect(page.getByLabel("时间线缩放", { exact: true })).toContainText("100%");
    await expect(page.getByRole("button", { name: /吸附/ })).toHaveAttribute("aria-pressed", /true|false/);
    const contentWidthBefore = await page.locator(".track-scroll-content").evaluate((element) => element.getBoundingClientRect().width);
    await timelineControls.getByRole("button", { name: "放大时间线" }).click();
    await expect(page.getByLabel("时间线缩放", { exact: true })).toContainText("125%");
    await expect
      .poll(async () => page.locator(".track-scroll-content").evaluate((element) => element.getBoundingClientRect().width))
      .toBeGreaterThan(contentWidthBefore);
    await expect(page.locator(".playhead")).toBeVisible();
    await expect(page.locator(".track-state-button")).toHaveCount(9);
    await expect(page.locator(".segment-kind-video")).toHaveCount(1);
    await expect(page.locator(".segment-kind-audio")).toHaveCount(1);

    await resetNativeCommandObservations(app, page);
    await page.getByRole("button", { name: "音频轨道 1 静音状态：未静音" }).click();
    await expectCommandCall(app, "setSelectedTrackMute");
    await expect(page.getByRole("button", { name: "音频轨道 1 静音状态：已静音" })).toBeVisible();

    await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "文字" }).click();
    await page.getByRole("button", { name: "添加文字", exact: true }).click();
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

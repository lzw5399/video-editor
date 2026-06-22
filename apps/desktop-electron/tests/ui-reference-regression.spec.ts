import { _electron as electron, expect, test, type ElectronApplication, type Locator, type Page } from "@playwright/test";
import { mkdirSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

type RegionBox = {
  x: number;
  y: number;
  width: number;
  height: number;
};

type ReferenceManifest = {
  screenshots: Array<{
    file: string;
    intendedState: string;
    provisional: boolean;
  }>;
};

const REPO_ROOT = join(process.cwd(), "../..");
const REFERENCE_DIR = join(REPO_ROOT, "docs/ui-reference/jianying-pro");
const REFERENCE_SCREENSHOT_DIR = join(REFERENCE_DIR, "screenshots");
const PHASE15_3_SCREENSHOT_DIR = join(REPO_ROOT, "test-results/phase15-3");
const FORBIDDEN_DEFAULT_COPY =
  /FFmpeg|ffprobe|backend|Mock|runtime|fallback|telemetry|artifact|cache|diagnostic|debug|requestProjectSessionPreviewFrame|生成预览片段|运行环境|运行时|资源维护|草稿包路径|缓存|产物|诊断|日志|宿主|备用|渲染图|\/tmp\/|\.veproj\/derived/i;

test.describe.configure({ timeout: 90_000 });

test("reference manifest lists every provisional screenshot without pixel-golden claims", async () => {
  const manifest = readReferenceManifest();
  const screenshotFiles = readdirSync(REFERENCE_SCREENSHOT_DIR)
    .filter((file) => file.endsWith(".png"))
    .sort();
  const manifestFiles = manifest.screenshots.map((entry) => entry.file).sort();

  expect(manifestFiles).toEqual(screenshotFiles);
  for (const entry of manifest.screenshots) {
    expect(entry.provisional, `${entry.file} must stay provisional`).toBe(true);
    expect(entry.intendedState, `${entry.file} intended state`).not.toHaveLength(0);
  }
});

test("default launch captures project entry before any material import surface", async () => {
  const { app, page } = await launchProjectEntryApp();

  try {
    await setViewport(app, page, 1280, 800);
    await expect(page.getByRole("main", { name: "项目入口" })).toBeVisible();
    await expect(page.getByRole("button", { name: "新建项目" })).toBeVisible();
    await expect(page.getByRole("button", { name: "打开项目" })).toBeVisible();
    await expect(page.getByRole("button", { name: "导入素材" })).toHaveCount(0);
    await expect(page.locator('[aria-label="素材面板"]')).toHaveCount(0);
    await expectNoDebugCopy(page.locator("body"));
    await capturePhaseScreenshot(page, "project-entry-1280x800.png");
  } finally {
    await app.close();
  }
});

test("production workspace captures five-zone hierarchy at desktop viewports", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await expectWorkspaceHierarchy(app, page, 1280, 800);
    await capturePhaseScreenshot(page, "workspace-1280x800.png");
    await captureMaterialLibraryScreenshot(page, "material-library-1280x800.png");

    await expectWorkspaceHierarchy(app, page, 1120, 720);
    await capturePhaseScreenshot(page, "workspace-1120x720.png");
    await captureMaterialLibraryScreenshot(page, "material-library-1120x720.png");
  } finally {
    await app.close();
  }
});

test("top-right export modal and audio dropdown capture production modal states", async () => {
  const { app, page } = await launchWorkspaceApp();

  try {
    await setViewport(app, page, 1280, 800);
    const exportButton = page.getByLabel("产品操作").getByRole("button", { name: "导出", exact: true });
    await expect(exportButton).toBeVisible();
    await expect(page.getByLabel("预览窗口").getByLabel("导出面板")).toHaveCount(0);

    await exportButton.click();
    const dialog = page.getByRole("dialog", { name: "导出" });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByLabel("输出路径")).toBeVisible();
    await expect(dialog.getByLabel("分辨率")).toBeVisible();
    await expect(dialog.getByLabel("帧率")).toBeVisible();
    await expect(dialog.getByLabel("视频码率")).toBeVisible();
    await expect(dialog.getByRole("checkbox", { name: "导出音频" })).toBeChecked();

    const advancedToggle = dialog.getByRole("button", { name: "高级设置" });
    await expect(advancedToggle).toHaveAttribute("aria-expanded", "false");
    await advancedToggle.click();
    await expect(advancedToggle).toHaveAttribute("aria-expanded", "true");
    await expect(dialog.getByLabel("高级导出设置")).toBeVisible();

    const sampleRate = dialog.getByRole("combobox", { name: "音频采样率" });
    await expect(sampleRate).toHaveAttribute("aria-expanded", "false");
    await sampleRate.click();
    await expect(sampleRate).toHaveAttribute("aria-expanded", "true");
    await expect(dialog.getByRole("listbox", { name: "音频采样率选项" })).toBeVisible();

    const dialogBox = await stableBox(dialog, "导出弹窗");
    const listBox = await stableBox(dialog.getByRole("listbox", { name: "音频采样率选项" }), "音频采样率下拉");
    expect(listBox.x, "dropdown left clipped by modal").toBeGreaterThanOrEqual(dialogBox.x);
    expect(listBox.x + listBox.width, "dropdown right clipped by modal").toBeLessThanOrEqual(dialogBox.x + dialogBox.width + 1);
    await expectNoDebugCopy(dialog);
    await capturePhaseScreenshot(page, "export-advanced-dropdown-1280x800.png");
  } finally {
    await app.close();
  }
});

async function launchProjectEntryApp(): Promise<{ app: ElectronApplication; page: Page }> {
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")],
    env: {
      ...process.env,
      VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "1",
      VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "0",
      VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([])
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  return { app, page };
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
      VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "0",
      VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify(["/tmp/demo-material.mp4"])
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  return { app, page };
}

async function expectWorkspaceHierarchy(app: ElectronApplication, page: Page, width: number, height: number): Promise<void> {
  await setViewport(app, page, width, height);

  const boxes = {
    titlebar: await stableBox(page.locator('[aria-label="项目标题栏"]'), `项目标题栏 ${width}x${height}`),
    top: await stableBox(page.locator('[aria-label="顶部功能区"]').first(), `顶部功能区 ${width}x${height}`),
    left: await stableBox(page.locator('[aria-label="素材面板"]'), `素材面板 ${width}x${height}`),
    preview: await stableBox(page.locator('[aria-label="预览窗口"]'), `预览窗口 ${width}x${height}`),
    inspector: await stableBox(page.locator('[aria-label="属性检查器"]'), `属性检查器 ${width}x${height}`),
    timeline: await stableBox(page.locator('[aria-label="时间线"]'), `时间线 ${width}x${height}`)
  };

  expect(boxes.titlebar.y, "project titlebar starts at viewport top").toBeLessThanOrEqual(1);
  expect(boxes.top.y, "feature bar below project titlebar").toBeGreaterThanOrEqual(boxes.titlebar.y + boxes.titlebar.height - 1);
  expect(boxes.left.x, "left panel before preview").toBeLessThan(boxes.preview.x);
  expect(boxes.preview.x + boxes.preview.width, "preview before inspector").toBeLessThanOrEqual(boxes.inspector.x + 1);
  expect(boxes.timeline.y, "timeline below editor body").toBeGreaterThan(boxes.top.y + boxes.top.height);
  expect(boxes.timeline.width, "timeline spans workspace").toBeGreaterThan(width - 4);

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
  await expectMaterialLibraryGeometry(page, width);

  const previewCanvas = await stableBox(page.locator(".preview-canvas"), `预览画布 ${width}x${height}`);
  expect(previewCanvas.x, `预览画布左侧不能越界 ${width}x${height}`).toBeGreaterThanOrEqual(boxes.preview.x);
  expect(previewCanvas.y, `预览画布顶部不能越界 ${width}x${height}`).toBeGreaterThanOrEqual(boxes.preview.y);
  expect(previewCanvas.x + previewCanvas.width, `预览画布右侧不能越界 ${width}x${height}`).toBeLessThanOrEqual(
    boxes.preview.x + boxes.preview.width + 1
  );
  expect(previewCanvas.y + previewCanvas.height, `预览画布底部不能越界 ${width}x${height}`).toBeLessThanOrEqual(
    boxes.preview.y + boxes.preview.height + 1
  );
  if (previewCanvas.width >= previewCanvas.height) {
    expect(previewCanvas.width / boxes.preview.width, `横屏预览画布应填充预览窗口 ${width}x${height}`).toBeGreaterThanOrEqual(0.7);
  }
  await expect(page.getByLabel("项目标题", { exact: true })).toContainText("未命名草稿");

  const exportButtonBox = await stableBox(page.getByLabel("产品操作").getByRole("button", { name: "导出", exact: true }), "顶部导出");
  expect(exportButtonBox.x, "export action is top-right").toBeGreaterThan(width - 180);
  await expectNoDebugCopy(page.locator("body"));
}

async function setViewport(app: ElectronApplication, page: Page, width: number, height: number): Promise<void> {
  await app.evaluate(
    async ({ BrowserWindow }, size) => {
      const window = BrowserWindow.getAllWindows()[0];
      window.setSize(size.width, size.height);
    },
    { width, height }
  );
  await page.setViewportSize({ width, height });
}

async function stableBox(locator: Locator, label: string): Promise<RegionBox> {
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

async function expectNoDebugCopy(locator: Locator): Promise<void> {
  await expect(locator).not.toContainText(FORBIDDEN_DEFAULT_COPY);
  await expect(await collectProductSurfaceCopy(locator)).not.toMatch(FORBIDDEN_DEFAULT_COPY);
}

async function collectProductSurfaceCopy(locator: Locator): Promise<string> {
  return locator.evaluateAll((roots) => {
    const values: string[] = [];
    for (const root of roots) {
      const elements = [root, ...Array.from(root.querySelectorAll("[aria-label], [title]"))];
      for (const element of elements) {
        const ariaLabel = element.getAttribute("aria-label");
        if (ariaLabel !== null) {
          values.push(ariaLabel);
        }
        const title = element.getAttribute("title");
        if (title !== null) {
          values.push(title);
        }
      }
    }
    return values.join("\n");
  });
}

async function capturePhaseScreenshot(page: Page, filename: string): Promise<void> {
  mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
  await page.screenshot({ path: join(PHASE15_3_SCREENSHOT_DIR, filename), fullPage: true });
}

async function captureMaterialLibraryScreenshot(page: Page, filename: string): Promise<void> {
  mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
  await page.locator('[aria-label="素材面板"]').screenshot({ path: join(PHASE15_3_SCREENSHOT_DIR, filename) });
}

async function expectMaterialLibraryGeometry(page: Page, width: number): Promise<void> {
  const materialPanel = page.locator('[aria-label="素材面板"]');
  const sourceRail = page.locator(".media-source-rail");
  const libraryPane = page.locator(".media-library-pane");
  const materialCard = page.locator(".material-row").first();
  const thumbnail = materialCard.locator(".material-thumb");
  const copy = materialCard.locator(".material-copy");
  const panelBox = await stableBox(materialPanel, `素材面板 ${width}`);
  const railBox = await stableBox(sourceRail, `媒体来源 ${width}`);
  const paneBox = await stableBox(libraryPane, `素材库 ${width}`);

  expect(panelBox.width, `left material panel should keep Jianying-like workspace width ${width}`).toBeGreaterThanOrEqual(
    width <= 1199 ? 350 : 400
  );
  expect(railBox.width, `source rail should remain a real source column ${width}`).toBeGreaterThanOrEqual(width <= 1199 ? 98 : 118);
  expect(paneBox.width, `material library pane should not collapse ${width}`).toBeGreaterThanOrEqual(width <= 1199 ? 220 : 250);
  expectNoOverlap(railBox, paneBox, "媒体来源", "素材库");
  await expect(sourceRail.locator("button")).toHaveText(["导入", "我的", "AI生成", "云素材", "官方素材", "即梦AI"]);
  await expect(sourceRail.locator(".media-source-chevron")).toHaveCount(5);
  await expect(page.getByLabel("搜索素材")).toHaveAttribute("placeholder", "搜索文件名");

  const cardBox = await stableBox(materialCard, `素材卡片 ${width}`);
  const thumbBox = await stableBox(thumbnail, `素材缩略图 ${width}`);
  const copyBox = await stableBox(copy, `素材标题 ${width}`);
  expect(cardBox.height, `material card should not become a list row ${width}`).toBeGreaterThanOrEqual(120);
  expect(thumbBox.y, `thumbnail must stay above title ${width}`).toBeLessThan(copyBox.y);
  expect(Math.abs(thumbBox.x - cardBox.x), `thumbnail should align with card left edge ${width}`).toBeLessThanOrEqual(1);
}

function readReferenceManifest(): ReferenceManifest {
  return JSON.parse(readFileSync(join(REFERENCE_DIR, "manifest.json"), "utf8")) as ReferenceManifest;
}

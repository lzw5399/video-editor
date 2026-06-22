import { _electron as electron, expect, test, type ElectronApplication, type Locator, type Page } from "@playwright/test";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
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
const REFERENCE_MEDIA_DIR = join(process.cwd(), "tests/fixtures/media");
const REFERENCE_VIDEO = join(REFERENCE_MEDIA_DIR, "p0-moving-testsrc.mp4");
const REFERENCE_AUDIO = join(REFERENCE_MEDIA_DIR, "p0-tone.wav");
const REFERENCE_IMAGE = join(REFERENCE_MEDIA_DIR, "p0-overlay-testsrc.png");
const REFERENCE_MEDIA_FILES = [REFERENCE_VIDEO, REFERENCE_AUDIO, REFERENCE_IMAGE] as const;
const FORBIDDEN_DEFAULT_COPY =
  /FFmpeg|ffprobe|backend|Mock|runtime|fallback|telemetry|artifact|cache|diagnostic|debug|requestProjectSessionPreviewFrame|生成预览片段|运行环境|运行时|资源维护|草稿包路径|缓存|产物|诊断|日志|宿主|备用|渲染图|\/tmp\/|\.veproj\/derived/i;
const FORBIDDEN_REFERENCE_MEDIA_COPY = /素材丢失|解析失败|素材解析失败|素材解析失败，请检查文件格式或重新导入/;
const VISIBLE_TOP_CATEGORIES = ["素材", "音频", "文本", "贴纸", "特效", "转场", "字幕"] as const;
const OVERFLOW_TOP_CATEGORIES = ["智能包装", "滤镜", "调节", "数字人"] as const;
const ALL_TOP_CATEGORIES = [...VISIBLE_TOP_CATEGORIES, ...OVERFLOW_TOP_CATEGORIES] as const;

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
    await captureTimelineScreenshot(page, "timeline-bottom-1280x800.png");
    await captureTopFeatureOverflowScreenshot(page, "top-feature-overflow-1280x800.png");
    await expectTopFeatureCategoriesReachable(page);
    await captureMaterialLibraryScreenshot(page, "material-library-1280x800.png");
    await capturePreviewMonitorScreenshot(page, "preview-monitor-1280x800.png");

    await expectWorkspaceHierarchy(app, page, 1120, 720);
    await capturePhaseScreenshot(page, "workspace-1120x720.png");
    await captureTimelineScreenshot(page, "timeline-bottom-1120x720.png");
    await captureMaterialLibraryScreenshot(page, "material-library-1120x720.png");
    await capturePreviewMonitorScreenshot(page, "preview-monitor-1120x720.png");
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
      VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE: referenceProjectBundlePath(),
      VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_ARTIFACT_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_AUDIO_COMMANDS: "1",
      VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "0",
      VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify(REFERENCE_MEDIA_FILES)
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await expect(page.getByRole("main", { name: "项目入口" })).toBeVisible();
  await page.getByRole("button", { name: "新建项目" }).click();
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  await page.getByRole("button", { name: "导入素材" }).click();
  await expect(page.getByRole("article", { name: "素材 p0-moving-testsrc.mp4" })).toBeVisible();
  await expect(page.getByRole("article", { name: "素材 p0-tone.wav" })).toBeVisible();
  await expect(page.getByRole("article", { name: "素材 p0-overlay-testsrc.png" })).toBeVisible();
  await expect(page.getByLabel("素材面板")).not.toContainText(FORBIDDEN_REFERENCE_MEDIA_COPY);
  await page.getByRole("button", { name: "添加 p0-moving-testsrc.mp4 到时间线" }).click();
  await page.getByRole("button", { name: "添加 p0-tone.wav 到时间线" }).click();
  await expect(page.getByRole("button", { name: /片段 p0-moving-testsrc\.mp4/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /片段 p0-tone\.wav/ })).toBeVisible();
  await page.getByRole("button", { name: "选择轨道 视频轨道 1" }).click();
  await expect(page.getByLabel("属性检查器")).toContainText("草稿参数");
  return { app, page };
}

function referenceProjectBundlePath(): string {
  return join(mkdtempSync(join(tmpdir(), "video-editor-ui-reference-")), "未命名草稿.veproj");
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
  expect(Math.abs(boxes.preview.y - boxes.top.y), `preview title row must align with feature tabs ${width}x${height}`).toBeLessThanOrEqual(
    1
  );
  expect(Math.abs(boxes.inspector.y - boxes.top.y), `inspector header row must align with feature tabs ${width}x${height}`).toBeLessThanOrEqual(
    1
  );
  expect(boxes.left.y, `material library must start below feature tabs ${width}x${height}`).toBeGreaterThanOrEqual(
    boxes.top.y + boxes.top.height - 1
  );
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

  await expectTitlebarChrome(page, boxes.titlebar, width);
  expectNoOverlap(boxes.left, boxes.preview, "素材面板", "预览窗口");
  expectNoOverlap(boxes.preview, boxes.inspector, "预览窗口", "属性检查器");
  expectNoOverlap(boxes.left, boxes.timeline, "素材面板", "时间线");
  expectNoOverlap(boxes.preview, boxes.timeline, "预览窗口", "时间线");
  expectNoOverlap(boxes.inspector, boxes.timeline, "属性检查器", "时间线");
  await expectMaterialLibraryGeometry(page, width);
  await expect(page.getByLabel("素材面板")).not.toContainText(FORBIDDEN_REFERENCE_MEDIA_COPY);
  await expectPreviewMonitorChrome(page, boxes.preview, width);
  await expectTopFeatureNavigationChrome(page);
  await expectTimelineChrome(page, width);

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

async function expectTitlebarChrome(page: Page, titlebarBox: RegionBox, width: number): Promise<void> {
  const status = page.getByLabel("草稿保存状态");
  await expect(status).toContainText(/\d{2}:\d{2}:\d{2} 自动保存本地/);
  await expect(status.locator(".titlebar-window-dot")).toHaveCount(3);
  const statusBox = await stableBox(status, `草稿保存状态 ${width}`);
  expect(statusBox.x, `titlebar save status left clipped ${width}`).toBeGreaterThanOrEqual(titlebarBox.x);
  expect(statusBox.x + statusBox.width, `titlebar save status right clipped ${width}`).toBeLessThanOrEqual(
    titlebarBox.x + titlebarBox.width + 1
  );
}

async function expectTopFeatureNavigationChrome(page: Page): Promise<void> {
  const nav = page.getByRole("navigation", { name: "顶部功能区" });
  await expect(nav.locator(".category-button .category-label")).toHaveText([...VISIBLE_TOP_CATEGORIES]);
  const navBox = await stableBox(nav, "顶部功能区导航");
  for (const category of VISIBLE_TOP_CATEGORIES) {
    const buttonBox = await stableBox(nav.getByRole("button", { name: category }), `顶部功能 ${category}`);
    expect(buttonBox.x, `${category} must not be clipped by the nav left edge`).toBeGreaterThanOrEqual(navBox.x - 1);
    expect(buttonBox.x + buttonBox.width, `${category} must not be clipped by the nav right edge`).toBeLessThanOrEqual(
      navBox.x + navBox.width + 1
    );
  }
  const overflow = page.getByRole("button", { name: "更多功能" });
  await expect(overflow).toBeEnabled();
  await overflow.click();
  const menu = page.getByRole("menu", { name: "更多功能菜单" });
  await expect(menu).toBeVisible();
  await expect(menu.getByRole("menuitemradio")).toHaveText([...OVERFLOW_TOP_CATEGORIES]);
  await overflow.click();
  await expect(menu).toHaveCount(0);
}

async function captureTopFeatureOverflowScreenshot(page: Page, filename: string): Promise<void> {
  const overflow = page.getByRole("button", { name: "更多功能" });
  await overflow.click();
  await expect(page.getByRole("menu", { name: "更多功能菜单" })).toBeVisible();
  await capturePhaseScreenshot(page, filename);
  await overflow.click();
  await expect(page.getByRole("menu", { name: "更多功能菜单" })).toHaveCount(0);
}

async function expectTopFeatureCategoriesReachable(page: Page): Promise<void> {
  for (const category of ALL_TOP_CATEGORIES) {
    await selectTopFeatureCategory(page, category);
    if (category === "素材") {
      await expect(page.getByRole("navigation", { name: "媒体来源" })).toBeVisible();
      await expect(page.getByRole("group", { name: "媒体工具" })).toBeVisible();
    } else {
      await expect(page.getByLabel("素材面板").getByRole("heading", { name: category, exact: true }).first()).toBeVisible();
    }
    await expect(page.getByLabel("素材面板")).not.toContainText(/暂未开放|暂不可用|暂未接入/);
  }
  await selectTopFeatureCategory(page, "素材");
}

async function selectTopFeatureCategory(page: Page, category: (typeof ALL_TOP_CATEGORIES)[number]): Promise<void> {
  const nav = page.getByRole("navigation", { name: "顶部功能区" });
  if ((VISIBLE_TOP_CATEGORIES as readonly string[]).includes(category)) {
    await nav.getByRole("button", { name: category }).click();
    return;
  }
  const overflow = page.getByRole("button", { name: "更多功能" });
  await overflow.click();
  await page.getByRole("menu", { name: "更多功能菜单" }).getByRole("menuitemradio", { name: category }).click();
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

async function capturePreviewMonitorScreenshot(page: Page, filename: string): Promise<void> {
  mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
  await page.locator('[aria-label="预览窗口"]').screenshot({ path: join(PHASE15_3_SCREENSHOT_DIR, filename) });
}

async function captureTimelineScreenshot(page: Page, filename: string): Promise<void> {
  mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
  await page.locator('[aria-label="时间线"]').screenshot({ path: join(PHASE15_3_SCREENSHOT_DIR, filename) });
}

async function expectTimelineChrome(page: Page, width: number): Promise<void> {
  const toolbar = page.getByLabel("时间线控制");
  const ruler = page.getByLabel("时间线标尺");
  const header = page.locator(".track-header").first();
  const toolbarBox = await stableBox(toolbar, `时间线工具栏 ${width}`);
  const rulerBox = await stableBox(ruler, `时间线标尺 ${width}`);
  const headerBox = await stableBox(header, `时间线轨道头 ${width}`);

  expect(toolbarBox.height, `timeline toolbar should stay compact ${width}`).toBeLessThanOrEqual(44);
  expect(rulerBox.height, `timeline ruler should stay compact ${width}`).toBeLessThanOrEqual(26);
  expect(headerBox.width, `track header should be compact ${width}`).toBeGreaterThanOrEqual(118);
  expect(headerBox.width, `track header should not dominate timeline ${width}`).toBeLessThanOrEqual(132);

  const rowMetrics = await page.locator(".track-row").evaluateAll((rows) =>
    rows.map((row) => {
      const box = row.getBoundingClientRect();
      const viewportHeight = window.innerHeight;
      return {
        height: box.height,
        visible: box.bottom > 0 && box.top < viewportHeight
      };
    })
  );
  expect(rowMetrics.filter((row) => row.visible).length, `timeline should expose multiple rows ${width}`).toBeGreaterThanOrEqual(3);
  for (const row of rowMetrics) {
    expect(row.height, `timeline row should stay dense ${width}`).toBeGreaterThanOrEqual(38);
    expect(row.height, `timeline row should stay dense ${width}`).toBeLessThanOrEqual(46);
  }

  await expect(page.locator(".track-status-line")).toHaveCount(0);
  await expect(page.locator(".timeline-tool-divider")).toHaveCount(2);
  await expectTimelineToolbarContentsInside(page, width);
  await expect(page.locator(".segment-filmstrip").first()).toBeVisible();
  await expect(page.locator(".segment-wave-bed").first()).toBeVisible();
}

async function expectTimelineToolbarContentsInside(page: Page, width: number): Promise<void> {
  const clippedControls = await page.getByLabel("时间线控制").evaluate((strip) => {
    const stripBox = strip.getBoundingClientRect();
    return Array.from(strip.querySelectorAll("button, input, select, [role='group'], .timeline-edit-cluster, .timeline-zoom-shell"))
      .map((child) => {
        const box = child.getBoundingClientRect();
        const style = window.getComputedStyle(child);
        return {
          label:
            child.getAttribute("aria-label") ||
            child.getAttribute("title") ||
            child.textContent?.replace(/\s+/g, " ").trim() ||
            child.tagName,
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

  expect(clippedControls, `timeline toolbar controls clipped ${width}`).toEqual([]);
  const overlappingClusters = await page.getByLabel("时间线控制").evaluate((strip) => {
    const clusters = Array.from(strip.querySelectorAll(".timeline-edit-cluster"))
      .map((cluster) => {
        const box = cluster.getBoundingClientRect();
        const style = window.getComputedStyle(cluster);
        return {
          label: cluster.getAttribute("class") ?? "timeline cluster",
          visible: style.display !== "none" && style.visibility !== "hidden" && box.width > 0 && box.height > 0,
          left: box.left,
          top: box.top,
          right: box.right,
          bottom: box.bottom
        };
      })
      .filter((box) => box.visible);
    const overlaps: Array<{ first: string; second: string }> = [];
    for (let index = 0; index < clusters.length; index += 1) {
      for (let next = index + 1; next < clusters.length; next += 1) {
        const first = clusters[index];
        const second = clusters[next];
        const separated = first.right <= second.left + 1 || second.right <= first.left + 1 || first.bottom <= second.top + 1 || second.bottom <= first.top + 1;
        if (!separated) {
          overlaps.push({ first: first.label, second: second.label });
        }
      }
    }
    return overlaps;
  });
  expect(overlappingClusters, `timeline toolbar clusters overlap ${width}`).toEqual([]);
}

async function expectPreviewMonitorChrome(page: Page, previewBox: RegionBox, width: number): Promise<void> {
  const preview = page.locator('[aria-label="预览窗口"]');
  const titlebar = preview.locator(".preview-titlebar");
  const transport = preview.locator(".preview-transport");
  const timeCluster = preview.locator(".preview-timecode-cluster");
  const playButton = preview.getByRole("button", { name: "播放预览" });
  const viewControls = preview.getByRole("group", { name: "预览画面控制" });
  const titlebarBox = await stableBox(titlebar, `播放器标题栏 ${width}`);
  const transportBox = await stableBox(transport, `播放器控制栏 ${width}`);
  const timeBox = await stableBox(timeCluster, `播放器时间 ${width}`);
  const playBox = await stableBox(playButton, `播放器播放按钮 ${width}`);
  const viewBox = await stableBox(viewControls, `播放器画面控制 ${width}`);

  await expect(titlebar).toContainText("播放器-时间线01");
  await expect(titlebar).not.toContainText("未命名草稿");
  await expect(preview.getByRole("button", { name: "播放器菜单" })).toBeVisible();
  await expect(preview.getByLabel("当前时间码")).toBeVisible();
  await expect(preview.getByLabel("总时长")).toBeVisible();
  await expect(viewControls.getByRole("button", { name: "原画" })).toBeVisible();
  await expect(viewControls.getByRole("button", { name: "画面比例" })).toBeVisible();
  await expect(viewControls.getByRole("button", { name: "画布读数" })).toHaveAttribute("title", /画布/);

  expect(titlebarBox.width, `播放器标题栏应铺满预览面板 ${width}`).toBeGreaterThan(previewBox.width - 4);
  expect(transportBox.width, `播放器控制栏应铺满预览面板 ${width}`).toBeGreaterThan(previewBox.width - 4);
  expect(timeBox.x, `播放器时间应在左侧 ${width}`).toBeLessThan(playBox.x);
  expect(viewBox.x, `画面控制应在播放按钮右侧 ${width}`).toBeGreaterThan(playBox.x + playBox.width);
  expect(Math.abs(playBox.x + playBox.width / 2 - (previewBox.x + previewBox.width / 2)), `播放按钮应靠近预览面板中心 ${width}`).toBeLessThanOrEqual(24);
  expect(viewBox.x + viewBox.width, `画面控制应靠近预览面板右侧 ${width}`).toBeGreaterThan(previewBox.x + previewBox.width - 150);
}

async function expectMaterialLibraryGeometry(page: Page, width: number): Promise<void> {
  const materialPanel = page.locator('[aria-label="素材面板"]');
  const sourceRail = page.locator(".media-source-rail");
  const libraryPane = page.locator(".media-library-pane");
  const toolbar = page.locator(".media-toolbar");
  const importButton = toolbar.getByRole("button", { name: "导入素材" });
  const searchBox = page.getByLabel("搜索素材");
  const listButton = toolbar.getByRole("button", { name: "列表视图" });
  const materialCard = page.locator(".material-row").first();
  const thumbnail = materialCard.locator(".material-thumb");
  const copy = materialCard.locator(".material-copy");
  const panelBox = await stableBox(materialPanel, `素材面板 ${width}`);
  const railBox = await stableBox(sourceRail, `媒体来源 ${width}`);
  const paneBox = await stableBox(libraryPane, `素材库 ${width}`);
  const toolbarBox = await stableBox(toolbar, `素材工具栏 ${width}`);
  const importBox = await stableBox(importButton, `导入按钮 ${width}`);
  const searchBoxRect = await stableBox(searchBox, `素材搜索 ${width}`);
  const listBox = await stableBox(listButton, `素材视图按钮 ${width}`);

  expect(panelBox.width, `left material panel should keep Jianying-like workspace width ${width}`).toBeGreaterThanOrEqual(
    width <= 1199 ? 350 : 400
  );
  expect(railBox.width, `source rail should remain a compact source column ${width}`).toBeGreaterThanOrEqual(width <= 1199 ? 96 : 108);
  expect(railBox.width, `source rail should not dominate the material bin ${width}`).toBeLessThanOrEqual(width <= 1199 ? 116 : 128);
  expect(paneBox.width, `material library pane should not collapse ${width}`).toBeGreaterThanOrEqual(width <= 1199 ? 220 : 250);
  expectNoOverlap(railBox, paneBox, "媒体来源", "素材库");
  await expect(page.locator(".media-library-title-row")).toHaveCount(0);
  await expect(sourceRail.locator("button")).toHaveText(["导入", "我的", "AI生成", "云素材", "官方素材", "即梦AI"]);
  await expect(sourceRail.locator(".media-source-chevron")).toHaveCount(5);
  await expect(searchBox).toHaveAttribute("placeholder", "搜索文件名");
  expect(toolbarBox.y, `material toolbar should start the library pane ${width}`).toBeLessThanOrEqual(paneBox.y + 10);
  expect(Math.abs(importBox.y - searchBoxRect.y), `import and search should share one compact row ${width}`).toBeLessThanOrEqual(2);
  expect(Math.abs(listBox.y - searchBoxRect.y), `view action should align with search row ${width}`).toBeLessThanOrEqual(2);

  const cardBox = await stableBox(materialCard, `素材卡片 ${width}`);
  const thumbBox = await stableBox(thumbnail, `素材缩略图 ${width}`);
  const copyBox = await stableBox(copy, `素材标题 ${width}`);
  expect(cardBox.width, `material card should stay as a dense bin tile ${width}`).toBeLessThanOrEqual(122);
  expect(cardBox.height, `material card should not become a list row ${width}`).toBeGreaterThanOrEqual(112);
  expect(thumbBox.y, `thumbnail must stay above title ${width}`).toBeLessThan(copyBox.y);
  expect(Math.abs(thumbBox.x - cardBox.x), `thumbnail should align with card left edge ${width}`).toBeLessThanOrEqual(1);
}

function readReferenceManifest(): ReferenceManifest {
  return JSON.parse(readFileSync(join(REFERENCE_DIR, "manifest.json"), "utf8")) as ReferenceManifest;
}

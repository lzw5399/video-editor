import { _electron as electron, expect, test, type ElectronApplication, type Locator, type Page } from "@playwright/test";
import { join } from "node:path";

type ExecuteCommandCall = {
  command: string;
  kind: string;
};

type ProjectSessionCall = {
  command: string;
  intentKind: string | null;
};

type RegionBox = {
  x: number;
  y: number;
  width: number;
  height: number;
};

async function launchDiagnosticsApp(
  env: NodeJS.ProcessEnv = {}
): Promise<{ app: ElectronApplication; page: Page }> {
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")],
    env: {
      ...process.env,
      VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
      VIDEO_EDITOR_TEST_WORKSPACE_FIXTURE: "demo",
      VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "1",
      VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "1",
      ...env
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await expectVisibleWorkspaceRegions(page);
  await expect(page.getByLabel("运行环境诊断")).toBeVisible();
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

async function readExecuteCommandCalls(app: ElectronApplication): Promise<ExecuteCommandCall[]> {
  const [legacyCalls, projectCalls] = await Promise.all([
    app.evaluate(() => {
      return (
        (globalThis as typeof globalThis & { __videoEditorTestExecuteCommandCalls?: ExecuteCommandCall[] })
          .__videoEditorTestExecuteCommandCalls ?? []
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
    ...legacyCalls,
    ...projectCalls
      .filter((call) => call.command === "executeProjectIntent" && call.intentKind !== null)
      .map((call) => ({
        command: call.intentKind ?? "executeProjectIntent",
        kind: call.intentKind ?? "executeProjectIntent"
      }))
  ];
}

async function expectCommandCall(app: ElectronApplication, command: string): Promise<void> {
  await expect
    .poll(async () => (await readExecuteCommandCalls(app)).some((call) => call.command === command))
    .toBe(true);
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

async function expectWorkspaceLayoutAt(page: Page, app: ElectronApplication, width: number, height: number): Promise<void> {
  await setViewport(app, page, width, height);
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
  await expectPreviewDiagnosticsInsideShell(page, `${width}x${height}`);
}

async function expectPreviewDiagnosticsInsideShell(page: Page, label: string): Promise<void> {
  const previewShell = await expectStableBox(page.locator(".preview-shell"), `预览壳 ${label}`);
  const diagnostics = await expectStableBox(page.getByLabel("运行环境诊断"), `运行环境诊断 ${label}`);
  const canvas = await expectStableBox(page.locator(".preview-canvas"), `预览画面 ${label}`);
  const ratio = canvas.width / canvas.height;

  expect(Math.abs(ratio - 16 / 9), `预览画面比例 ${label}`).toBeLessThanOrEqual(0.04);
  expect(diagnostics.x, `诊断面板 left inside shell ${label}`).toBeGreaterThanOrEqual(previewShell.x - 1);
  expect(diagnostics.y, `诊断面板 top inside shell ${label}`).toBeGreaterThanOrEqual(previewShell.y - 1);
  expect(diagnostics.x + diagnostics.width, `诊断面板 right inside shell ${label}`).toBeLessThanOrEqual(
    previewShell.x + previewShell.width + 1
  );
  expect(diagnostics.y + diagnostics.height, `诊断面板 bottom inside shell ${label}`).toBeLessThanOrEqual(
    previewShell.y + previewShell.height + 1
  );

  const clippedItems = await page.locator(".preview-shell").evaluate((shell) => {
    const shellBox = shell.getBoundingClientRect();
    return Array.from(
      shell.querySelectorAll(
        ".preview-canvas, .preview-transport, .preview-artifact-panel, .runtime-diagnostics-panel, .preview-status-line, button, input, select, progress"
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

  expect(clippedItems, `预览壳内容不能裁切 ${label}`).toEqual([]);
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

test("运行环境诊断在 1280x800 和 1120x720 内显示就绪状态", async () => {
  const { app, page } = await launchDiagnosticsApp();

  try {
    await expectCommandCall(app, "probeRuntimeCapabilities");
    await expect(page.getByLabel("运行环境状态")).toContainText("运行环境就绪");
    await expect(page.getByLabel("运行能力列表")).toContainText("媒体运行环境");
    await expect(page.getByLabel("运行能力列表")).toContainText("媒体探测环境");
    await expect(page.getByLabel("运行能力列表")).toContainText("编码能力");
    await expect(page.getByLabel("运行能力列表")).toContainText("字幕能力");
    await expect(page.getByLabel("运行能力列表")).toContainText("字体环境");
    await expect(page.getByLabel("运行能力列表")).toContainText("打包状态");
    await expect(page.getByRole("button", { name: "重新检测运行环境" })).toBeVisible();

    await expectWorkspaceLayoutAt(page, app, 1280, 800);
    await expectWorkspaceLayoutAt(page, app, 1120, 720);
  } finally {
    await app.close();
  }
});

test("运行环境错误会禁用预览和导出入口", async () => {
  const { app, page } = await launchDiagnosticsApp({
    VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "error"
  });

  try {
    await expectCommandCall(app, "probeRuntimeCapabilities");
    await expect(page.getByLabel("运行环境状态")).toContainText(
      "运行环境检测失败，请检查内置 FFmpeg/ffprobe runtime 后重试。"
    );
    await expect(page.getByRole("button", { name: "预览暂不可用" }).first()).toBeDisabled();
    await page.getByRole("button", { name: "导出" }).click();
    await expect(page.getByRole("dialog", { name: "导出" }).getByRole("button", { name: "导出暂不可用" })).toBeDisabled();
    await page.getByRole("button", { name: "关闭" }).click();
    await expect(page.getByRole("button", { name: "重新检测运行环境" })).toBeVisible();

    await expectWorkspaceLayoutAt(page, app, 1280, 800);
    await expectWorkspaceLayoutAt(page, app, 1120, 720);
  } finally {
    await app.close();
  }
});

test("运行环境诊断不破坏时间线命令边界", async () => {
  const { app, page } = await launchDiagnosticsApp();

  try {
    await expectCommandCall(app, "probeRuntimeCapabilities");
    await page.getByRole("button", { name: /片段 城市街景\.mp4/ }).click();
    await expectCommandCall(app, "selectTimelineItemIntent");
    await expect(page.getByLabel("片段信息")).toContainText("segment-main-video");

    const calls = await readExecuteCommandCalls(app);
    expect(calls.map((call) => call.command)).toEqual(
      expect.arrayContaining(["probeRuntimeCapabilities", "selectTimelineItemIntent"])
    );
  } finally {
    await app.close();
  }
});

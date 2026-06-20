import { _electron as electron, expect, type ElectronApplication, type Page } from "@playwright/test";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { access, readFile, unlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { promisify } from "node:util";

import type { CommandName } from "../../src/generated/CommandEnvelope";
import {
  launchForegroundProductApp,
  type ForegroundProductAppController,
  type ForegroundProductAppDiagnostics,
  type ProductWindowMetrics
} from "./foregroundProductApp";

export const USER_JOURNEY_MEDIA_DIR = join(process.cwd(), "tests/fixtures/media");
export const USER_JOURNEY_MOVING_VIDEO = join(USER_JOURNEY_MEDIA_DIR, "p0-moving-testsrc.mp4");
export const USER_JOURNEY_AV_VIDEO = join(USER_JOURNEY_MEDIA_DIR, "p0-av-tone-testsrc.mp4");
export const USER_JOURNEY_OVERLAY_IMAGE = join(USER_JOURNEY_MEDIA_DIR, "p0-overlay-testsrc.png");
export const USER_JOURNEY_TONE_AUDIO = join(USER_JOURNEY_MEDIA_DIR, "p0-tone.wav");
const execFileAsync = promisify(execFile);

type ExecuteCommandCall = {
  command: CommandName;
  kind: string;
  targetTime?: number | null;
  targetTimerange?: { start: number; duration: number } | null;
  visual?: {
    visible: boolean;
    fitMode: string;
    transform: {
      position: { x: number; y: number };
      scale: { xMillis: number; yMillis: number };
      rotation: { degrees: number };
      opacity: { valueMillis: number };
    };
  } | null;
  textContent?: string | null;
  textSource?: string | null;
  textFontRef?: string | null;
};

type RealtimePreviewHostCall = {
  kind: string;
  parentHandleByteLength?: number;
  bounds?: {
    x: number;
    y: number;
    width: number;
    height: number;
    scaleFactorMillis: number;
  };
  targetTimeMicroseconds?: number;
  playbackGeneration?: number;
  errorMessage?: string;
};

type RealtimePreviewHostState = {
  ok: boolean;
  productReady: boolean;
  hostAttached: boolean;
  fallbackActive: boolean;
  statusLabel: string;
  fallbackLabel: string | null;
  playbackGeneration: number | null;
  backend: "renderGraphGpu" | "none";
  diagnosticSource: "nativeVideoBridge" | "runtimeFrameRequest" | "none";
  telemetry: {
    presentedFrameCount: number;
    targetTimeMicroseconds: number;
    playbackGeneration: number;
  } | null;
  frameDisplay: {
    frameToken: string;
    targetTimeMicroseconds: number;
    dominantColor: string;
    accentColor: string;
  } | null;
  contentEvidence: {
    source: "nativeVideoBridge" | "renderGraphGpuComposited";
    digest: string;
    width: number;
    height: number;
    targetTimeMicroseconds: number;
  } | null;
  surfacePlacement?: {
    hostScreenRect: { x: number; y: number; width: number; height: number };
    nativeScreenRect: { x: number; y: number; width: number; height: number };
    maxDeltaPx: number;
    aligned: boolean;
  } | null;
};

export type ProductJourneyAppController = {
  readonly kind: "electron-launch" | "foreground-cdp";
  close: () => Promise<void>;
  readExecuteCommandCalls: () => Promise<ExecuteCommandCall[]>;
  readRealtimePreviewHostCalls: () => Promise<RealtimePreviewHostCall[]>;
  readForegroundDiagnostics: () => Promise<ForegroundProductAppDiagnostics | null>;
  readWindowMetrics: () => Promise<ProductWindowMetrics | null>;
};

declare global {
  interface Window {
    videoEditorRealtimePreviewHost?: {
      getTelemetry: () => Promise<RealtimePreviewHostState>;
    };
  }
}

export type PreviewEvidence = {
  regionHash: string;
  visibleCenterHash: string;
  timecodeUs: number;
  placeholderText: string;
  imageSrc: string | null;
  hostState: RealtimePreviewHostState | null;
};

export async function waitForCompositedPreviewEvidence(
  page: Page,
  app?: ProductJourneyAppController,
  timeoutMs = 8_000,
  afterTargetTimeUs = -1
): Promise<PreviewEvidence> {
  const deadline = Date.now() + timeoutMs;
  let lastEvidence: PreviewEvidence | null = null;

  while (Date.now() < deadline) {
    lastEvidence = await capturePreviewEvidence(page);
    const evidence = lastEvidence.hostState?.contentEvidence;
    if (
      evidence?.source === "renderGraphGpuComposited" &&
      evidence.targetTimeMicroseconds > afterTargetTimeUs
    ) {
      return lastEvidence;
    }
    await page.waitForTimeout(250);
  }

  const hostCalls = app === undefined ? [] : await readRealtimePreviewHostCalls(app);
  const foregroundDiagnostics = app === undefined ? null : await app.readForegroundDiagnostics();
  throw new Error(
    `Timed out waiting for composited preview evidence after ${afterTargetTimeUs}us. Last host state: ${JSON.stringify(
      lastEvidence?.hostState ?? null
    )}. Host calls: ${JSON.stringify(hostCalls)}. Foreground diagnostics: ${JSON.stringify(foregroundDiagnostics)}`
  );
}

export async function waitForVisiblePreviewCenterChange(
  page: Page,
  app: ProductJourneyAppController | undefined,
  initialHash: string,
  timeoutMs = 5_000
): Promise<PreviewEvidence> {
  const deadline = Date.now() + timeoutMs;
  let lastEvidence: PreviewEvidence | null = null;

  while (Date.now() < deadline) {
    lastEvidence = await captureVisiblePreviewEvidence(page, app);
    if (lastEvidence.visibleCenterHash !== initialHash) {
      return lastEvidence;
    }
    await page.waitForTimeout(250);
  }

  throw new Error(
    `Timed out waiting for visible preview center pixels to change. Initial hash: ${initialHash}. Last evidence: ${JSON.stringify(
      lastEvidence
    )}`
  );
}

export async function captureVisiblePreviewEvidence(
  page: Page,
  app: ProductJourneyAppController | undefined
): Promise<PreviewEvidence> {
  const evidence = await capturePreviewEvidence(page);
  if (process.platform !== "darwin" || app === undefined) {
    return evidence;
  }
  return {
    ...evidence,
    visibleCenterHash: hashBuffer(await captureVisiblePreviewCenter(page, app))
  };
}

export function expectNoRejectedSurfaceAcquire(calls: RealtimePreviewHostCall[]): void {
  expect(
    calls,
    "product playback must not pass through an occluded WGPU surface acquire"
  ).not.toEqual(
    expect.arrayContaining([
      expect.objectContaining({
        kind: "surfaceAcquireOccluded"
      })
    ])
  );
}

export function expectOccludedSurfaceAcquireHasDrawableLifecycleDiagnostics(
  calls: RealtimePreviewHostCall[]
): void {
  const occluded = calls.find((call) => call.kind === "surfaceAcquireOccluded");
  expect(occluded, "occluded surface acquire must be recorded for fail-closed diagnosis").toBeDefined();
  expect(
    occluded?.errorMessage ?? "",
    "occluded acquire diagnostics must include AppKit/CoreAnimation drawable lifecycle state"
  ).toEqual(expect.stringContaining("drawableLifecycle{"));
  for (const field of [
    "parentWindowVisible=",
    "parentWindowOcclusionVisible=",
    "parentWindowOnActiveSpace=",
    "childWindowVisible=",
    "childWindowOcclusionVisible=",
    "childWindowOnActiveSpace=",
    "childHasParent=",
    "appActive=",
    "appHidden=",
    "runningAppActive=",
    "runningAppHidden=",
    "appActivationPolicy=",
    "appOcclusionVisible=",
    "childViewHidden=",
    "childViewHiddenOrAncestor=",
    "layerHidden=",
    "parentViewBounds=",
    "childWindowFrame=",
    "childViewFrame=",
    "layerBounds=",
    "drawableSize="
  ]) {
    expect(occluded?.errorMessage ?? "").toEqual(expect.stringContaining(field));
  }
}

export async function launchProductJourneyApp(
  openMaterialFiles: string[],
  env: NodeJS.ProcessEnv = {}
): Promise<{ app: ProductJourneyAppController; page: Page }> {
  await Promise.all(openMaterialFiles.map((filePath) => expectFileExists(filePath)));
  const productEnv = {
    VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
    VIDEO_EDITOR_TEST_COMMAND_MOCKS: "0",
    VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "0",
    VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "0",
    ...env
  };

  if (process.platform === "darwin") {
    const launch = await launchForegroundProductApp(openMaterialFiles, productEnv);
    await expectProductWorkspace(launch.page);
    return {
      app: wrapForegroundController(launch.app),
      page: launch.page
    };
  }

  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")],
    env: {
      ...process.env,
      ...productEnv,
      VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify(openMaterialFiles),
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await activateProductWindow(app, page);
  await expectProductWorkspace(page);
  return { app: wrapElectronApp(app), page };
}

export async function expectProductWorkspace(page: Page): Promise<void> {
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  await expect(page.getByRole("button", { name: "导入素材" })).toBeVisible();
  await expect(page.locator('[aria-label="素材面板"]')).toBeVisible();
  await expect(page.locator('[aria-label="预览窗口"]')).toBeVisible();
  await expect(page.locator('[aria-label="属性检查器"]')).toBeVisible();
  await expect(page.locator('[aria-label="时间线"]')).toBeVisible();

  await expect(page.getByLabel("预览产物")).toHaveCount(0);
  await expect(page.getByText("草稿包路径")).toHaveCount(0);
  await expect(page.getByText("素材路径")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "导入路径" })).toHaveCount(0);
}

export async function importMaterialThroughProductPicker(
  app: ProductJourneyAppController,
  page: Page,
  materialPath: string
): Promise<void> {
  const materialName = basename(materialPath);
  const nextCount = (await countCommand(app, "importMaterial")) + 1;
  await page.getByRole("button", { name: "导入素材" }).click();
  await waitForCommandCount(app, "importMaterial", nextCount);
  await expect(page.getByRole("article", { name: `素材 ${materialName}` })).toContainText("可用", {
    timeout: 30_000
  });
}

export async function importMaterialsThroughProductPicker(
  app: ProductJourneyAppController,
  page: Page,
  materialPaths: string[]
): Promise<void> {
  const nextCount = (await countCommand(app, "importMaterial")) + materialPaths.length;
  await page.getByRole("button", { name: "导入素材" }).click();
  await waitForCommandCount(app, "importMaterial", nextCount);
  for (const materialPath of materialPaths) {
    const materialName = basename(materialPath);
    await expect(page.getByRole("article", { name: `素材 ${materialName}` })).toContainText("可用", {
      timeout: 30_000
    });
  }
}

export async function addMaterialToTimeline(
  app: ProductJourneyAppController,
  page: Page,
  materialPath: string
): Promise<void> {
  const materialName = basename(materialPath);
  const nextCount = (await countCommand(app, "addSegment")) + 1;
  const timelineMaterialSelect = page.locator(".compact-select select");
  await expect(timelineMaterialSelect).toBeEnabled({ timeout: 10_000 });
  await timelineMaterialSelect.selectOption({ label: materialName });
  await page.getByRole("button", { name: "添加片段" }).click();
  await waitForCommandCount(app, "addSegment", nextCount);
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(materialName)}`) })).toBeVisible();
  await expect(page.getByLabel("预览选中框")).toBeVisible();
}

export async function addVideoTrack(page: Page, app: ProductJourneyAppController): Promise<void> {
  const nextCount = (await countCommand(app, "addTrack")) + 1;
  await page.getByRole("button", { name: "添加视频轨道" }).click();
  await waitForCommandCount(app, "addTrack", nextCount);
  await expect(page.getByRole("button", { name: /选择轨道 视频轨道 2/ })).toBeVisible();
}

export async function addTextThroughProductPanel(
  page: Page,
  app: ProductJourneyAppController,
  content: string,
  durationUs = 2_000_000
): Promise<void> {
  const nextCount = (await countCommand(app, "addTextSegment")) + 1;
  await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "文字" }).click();
  const textPanel = page.getByRole("region", { name: "素材面板" });
  await textPanel.getByLabel("默认文字").getByLabel("文字内容").fill(content);
  await textPanel.getByLabel("默认文字").getByLabel("时长（微秒）").fill(String(durationUs));
  await textPanel.getByRole("button", { name: "添加文字", exact: true }).click();
  await waitForCommandCount(app, "addTextSegment", nextCount);
  await expect(page.getByRole("complementary", { name: "属性检查器" }).getByRole("textbox", { name: "文字内容" })).toHaveValue(
    content
  );
}

export async function addAudioThroughProductPanel(
  page: Page,
  app: ProductJourneyAppController,
  audioPath: string,
  durationUs = 2_000_000
): Promise<void> {
  const nextCount = (await countCommand(app, "addAudioSegment")) + 1;
  await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "音频" }).click();
  const audioPanel = page.getByRole("region", { name: "素材面板" });
  await audioPanel.getByLabel("BGM素材").selectOption({ label: basename(audioPath) });
  await audioPanel.getByLabel("时长（微秒）").fill(String(durationUs));
  await audioPanel.getByRole("button", { name: "添加音频", exact: true }).click();
  await waitForCommandCount(app, "addAudioSegment", nextCount);
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(basename(audioPath))}`) })).toBeVisible();
}

type VisualInspectorEdit = {
  positionX?: number;
  positionY?: number;
  scaleX?: number;
  scaleY?: number;
  rotation?: number;
  opacity?: number;
  fitMode?: "适应" | "填充" | "拉伸";
};

export async function updateSelectedVisualThroughInspector(
  page: Page,
  app: ProductJourneyAppController,
  edit: VisualInspectorEdit = {}
): Promise<void> {
  const positionX = edit.positionX ?? 120;
  const positionY = edit.positionY ?? -40;
  const scaleX = edit.scaleX ?? 1250;
  const scaleY = edit.scaleY ?? 1250;
  const rotation = edit.rotation ?? 8;
  const opacity = edit.opacity ?? 820;
  const fitMode = edit.fitMode ?? "填充";
  const nextCount = (await countCommand(app, "updateSegmentVisual")) + 1;
  const visualForm = page.getByLabel("画面基础表单");
  await visualForm.getByLabel("位置 X", { exact: true }).fill(String(positionX));
  await visualForm.getByLabel("位置 Y", { exact: true }).fill(String(positionY));
  await visualForm.getByLabel("缩放 X", { exact: true }).fill(String(scaleX));
  await visualForm.getByLabel("缩放 Y", { exact: true }).fill(String(scaleY));
  await visualForm.getByRole("spinbutton", { name: "旋转", exact: true }).fill(String(rotation));
  await visualForm.getByRole("spinbutton", { name: "不透明度", exact: true }).fill(String(opacity));
  await visualForm.getByRole("group", { name: "适应方式" }).getByRole("button", { name: fitMode }).click();
  await expect(visualForm.getByRole("button", { name: "应用画面" })).toBeEnabled();
  await visualForm.getByRole("button", { name: "应用画面" }).click();
  await waitForCommandCount(app, "updateSegmentVisual", nextCount);
}

export async function seekTimelinePlayhead(page: Page, app: ProductJourneyAppController, targetTimeUs: number): Promise<void> {
  const frameRequestsBefore = requestPreviewFrameCount(await readExecuteCommandCalls(app));
  await page.getByRole("spinbutton", { name: "播放头", exact: true }).fill(String(targetTimeUs));
  await expect(page.getByLabel("当前时间码")).toContainText(formatExpectedTimecode(targetTimeUs), { timeout: 10_000 });
  expect(
    requestPreviewFrameCount(await readExecuteCommandCalls(app)),
    "product seek must not fall back to preview artifact frame requests"
  ).toBe(frameRequestsBefore);
}

export async function splitSelectedSegment(page: Page, app: ProductJourneyAppController, splitAtUs: number): Promise<void> {
  const nextCount = (await countCommand(app, "splitSegment")) + 1;
  await page.getByRole("spinbutton", { name: "分割", exact: true }).fill(String(splitAtUs));
  await page.getByRole("button", { name: "分割所选片段" }).click();
  await waitForCommandCount(app, "splitSegment", nextCount);
}

export async function moveSelectedSegmentRight(page: Page, app: ProductJourneyAppController, deltaUs: number): Promise<void> {
  const nextCount = (await countCommand(app, "moveSegment")) + 1;
  await page.getByRole("spinbutton", { name: "移动", exact: true }).fill(String(deltaUs));
  await page.getByRole("button", { name: "右移所选片段" }).click();
  await waitForCommandCount(app, "moveSegment", nextCount);
}

export async function deleteSelectedSegment(page: Page, app: ProductJourneyAppController): Promise<void> {
  const nextCount = (await countCommand(app, "deleteSegment")) + 1;
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "删除所选片段" }).click();
  await waitForCommandCount(app, "deleteSegment", nextCount);
}

export async function undoTimelineEdit(page: Page, app: ProductJourneyAppController): Promise<void> {
  const nextCount = (await countCommand(app, "undoTimelineEdit")) + 1;
  await page.getByRole("button", { name: "撤销" }).click();
  await waitForCommandCount(app, "undoTimelineEdit", nextCount);
}

export async function redoTimelineEdit(page: Page, app: ProductJourneyAppController): Promise<void> {
  const nextCount = (await countCommand(app, "redoTimelineEdit")) + 1;
  await page.getByRole("button", { name: "重做" }).click();
  await waitForCommandCount(app, "redoTimelineEdit", nextCount);
}

export function expectNoProductFallbackCalls(calls: RealtimePreviewHostCall[]): void {
  expectNoRejectedSurfaceAcquire(calls);
  expect(calls.map((call) => call.kind), "product journey must not accept missing-compositor fallback").not.toContain(
    "playRejectedMissingCompositor"
  );
}

export async function clickPreviewPlay(page: Page): Promise<void> {
  const controls = page.getByRole("group", { name: "预览播放控制" });
  const playButton = controls.getByRole("button", { name: "播放预览" });
  await expect(playButton).toBeEnabled({ timeout: 20_000 });
  await playButton.click();
  await expect(controls.getByRole("button", { name: "暂停预览" })).toBeEnabled({ timeout: 10_000 });
}

export async function activateProductJourneyApp(app: ProductJourneyAppController, page: Page): Promise<void> {
  await page.bringToFront();
  if (process.platform !== "darwin") {
    return;
  }
  const diagnostics = await app.readForegroundDiagnostics();
  if (diagnostics?.pid === null || diagnostics?.pid === undefined) {
    return;
  }
  await execFileAsync("osascript", ["-e", `tell application id "org.videoeditor.desktop" to activate`]).catch(
    () => undefined
  );
  await execFileAsync("osascript", [
    "-e",
    `tell application "System Events" to set frontmost of (first process whose unix id is ${diagnostics.pid}) to true`
  ]).catch(() => undefined);
  await page.waitForTimeout(750);
}

async function activateProductWindow(app: ElectronApplication, page: Page): Promise<void> {
  await page.bringToFront();
  await app.evaluate(({ app: electronApp, BrowserWindow }) => {
    if (process.platform === "darwin") {
      electronApp.setActivationPolicy("regular");
    }
    const window = BrowserWindow.getAllWindows()[0];
    window?.show();
    window?.setFocusable(true);
    window?.focus();
    window?.moveTop();
    electronApp.show();
    electronApp.focus({ steal: true });
  });

  if (process.platform !== "darwin") {
    return;
  }

  const pid = await app.evaluate(() => process.pid);
  await execFileAsync("osascript", [
    "-e",
    `tell application "System Events" to set frontmost of (first process whose unix id is ${pid}) to true`
  ]).catch(() => undefined);
  await page.waitForTimeout(250);
}

export async function capturePreviewEvidence(page: Page): Promise<PreviewEvidence> {
  const previewCanvas = page.getByLabel("预览画面", { exact: true });
  await expect(previewCanvas).toBeVisible();

  const screenshot = await previewCanvas.screenshot();
  const visibleCenterScreenshot = await captureVisiblePreviewCenter(page);
  const placeholder = page.locator(".preview-placeholder");
  const image = page.getByRole("img", { name: "当前预览帧" });

  return {
    regionHash: hashBuffer(screenshot),
    visibleCenterHash: hashBuffer(visibleCenterScreenshot),
    timecodeUs: parseTimecodeToMicroseconds((await page.getByLabel("当前时间码").textContent()) ?? ""),
    placeholderText: (await placeholder.textContent({ timeout: 100 }).catch(() => "")) ?? "",
    imageSrc: await image.getAttribute("src", { timeout: 100 }).catch(() => null),
    hostState: await readRealtimePreviewHostState(page)
  };
}

export async function readExecuteCommandCalls(app: ProductJourneyAppController): Promise<ExecuteCommandCall[]> {
  return app.readExecuteCommandCalls();
}

export async function readRealtimePreviewHostCalls(app: ProductJourneyAppController): Promise<RealtimePreviewHostCall[]> {
  return app.readRealtimePreviewHostCalls();
}

export function requestPreviewFrameCount(calls: ExecuteCommandCall[]): number {
  return calls.filter((call) => call.command === "requestPreviewFrame").length;
}

async function readRealtimePreviewHostState(page: Page): Promise<RealtimePreviewHostState | null> {
  return page.evaluate(async () => {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return null;
    }
    return (await bridge.getTelemetry()) as RealtimePreviewHostState;
  });
}

async function waitForCommandCount(app: ProductJourneyAppController, command: CommandName, expectedCount: number): Promise<void> {
  await expect.poll(async () => countCommand(app, command), { timeout: 30_000 }).toBeGreaterThanOrEqual(expectedCount);
}

async function countCommand(app: ProductJourneyAppController, command: CommandName): Promise<number> {
  return (await readExecuteCommandCalls(app)).filter((call) => call.command === command).length;
}

function wrapElectronApp(app: ElectronApplication): ProductJourneyAppController {
  return {
    kind: "electron-launch",
    close: () => app.close(),
    readForegroundDiagnostics: async () => null,
    readExecuteCommandCalls: () =>
      app.evaluate(() => {
        return (
          (globalThis as typeof globalThis & { __videoEditorTestExecuteCommandCalls?: ExecuteCommandCall[] })
            .__videoEditorTestExecuteCommandCalls ?? []
        );
      }),
    readRealtimePreviewHostCalls: () =>
      app.evaluate(() => {
        return (
          (globalThis as typeof globalThis & { __videoEditorTestRealtimePreviewHostCalls?: RealtimePreviewHostCall[] })
            .__videoEditorTestRealtimePreviewHostCalls ?? []
        );
      }),
    readWindowMetrics: async () =>
      app.evaluate(({ BrowserWindow, screen }) => {
        const window = BrowserWindow.getAllWindows()[0];
        if (window === undefined) {
          return null;
        }
        return {
          bounds: window.getBounds(),
          contentBounds: window.getContentBounds(),
          displayScaleFactor: screen.getDisplayMatching(window.getBounds()).scaleFactor
        };
      })
  };
}

function wrapForegroundController(app: ForegroundProductAppController): ProductJourneyAppController {
  return {
    kind: app.kind,
    close: () => app.close(),
    readForegroundDiagnostics: () => app.readForegroundDiagnostics(),
    readExecuteCommandCalls: async () => (await app.readExecuteCommandCalls()) as ExecuteCommandCall[],
    readRealtimePreviewHostCalls: async () => (await app.readRealtimePreviewHostCalls()) as RealtimePreviewHostCall[],
    readWindowMetrics: () => app.readWindowMetrics()
  };
}

async function expectFileExists(path: string): Promise<void> {
  await expect(access(path).then(
    () => true,
    () => false
  )).resolves.toBe(true);
}

function hashBuffer(buffer: Buffer): string {
  return createHash("sha256").update(buffer).digest("hex");
}

async function captureVisiblePreviewCenter(
  page: Page,
  app?: ProductJourneyAppController
): Promise<Buffer> {
  const host = page.getByLabel("实时预览宿主", { exact: true });
  await expect(host).toBeVisible();
  const box = await host.boundingBox();
  if (box === null) {
    throw new Error("Realtime preview host has no visible bounding box");
  }

  const clip = {
    x: Math.round(box.x + box.width * 0.28),
    y: Math.round(box.y + box.height * 0.22),
    width: Math.max(1, Math.round(box.width * 0.44)),
    height: Math.max(1, Math.round(box.height * 0.42))
  };

  if (process.platform === "darwin" && app !== undefined) {
    const metrics = await app.readWindowMetrics();
    if (metrics !== null) {
      return captureMacosScreenRegion(page, metrics, clip);
    }
  }

  return page.screenshot({ clip });
}

async function captureMacosScreenRegion(
  page: Page,
  metrics: ProductWindowMetrics,
  clip: { x: number; y: number; width: number; height: number }
): Promise<Buffer> {
  const viewport = await page.evaluate(() => ({
    width: window.innerWidth,
    height: window.innerHeight
  }));
  const scaleX = viewport.width > 0 ? metrics.contentBounds.width / viewport.width : 1;
  const scaleY = viewport.height > 0 ? metrics.contentBounds.height / viewport.height : 1;
  const screenClip = {
    x: Math.round((metrics.contentBounds.x + clip.x * scaleX) * metrics.displayScaleFactor),
    y: Math.round((metrics.contentBounds.y + clip.y * scaleY) * metrics.displayScaleFactor),
    width: Math.max(1, Math.round(clip.width * scaleX * metrics.displayScaleFactor)),
    height: Math.max(1, Math.round(clip.height * scaleY * metrics.displayScaleFactor))
  };
  const fullPath = join(
    tmpdir(),
    `video-editor-preview-full-${process.pid}-${Date.now()}-${Math.round(Math.random() * 1_000_000)}.png`
  );
  const cropPath = join(
    tmpdir(),
    `video-editor-preview-center-${process.pid}-${Date.now()}-${Math.round(Math.random() * 1_000_000)}.png`
  );
  try {
    await execFileAsync("screencapture", ["-x", fullPath]);
    await execFileAsync("sips", [
      "-c",
      String(screenClip.height),
      String(screenClip.width),
      "--cropOffset",
      String(screenClip.y),
      String(screenClip.x),
      fullPath,
      "--out",
      cropPath
    ]);
    return await readFile(cropPath);
  } finally {
    await unlink(fullPath).catch(() => undefined);
    await unlink(cropPath).catch(() => undefined);
  }
}

function parseTimecodeToMicroseconds(value: string): number {
  const match = value.trim().match(/^(\d{2}):(\d{2}):(\d{2})\.(\d{3})$/);
  if (match === null) {
    return 0;
  }
  const [, hours, minutes, seconds, millis] = match;
  return (
    Number(hours) * 3_600_000_000 +
    Number(minutes) * 60_000_000 +
    Number(seconds) * 1_000_000 +
    Number(millis) * 1_000
  );
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function formatExpectedTimecode(targetTimeUs: number): string {
  const milliseconds = Math.floor(targetTimeUs / 1000);
  const hours = Math.floor(milliseconds / 3_600_000);
  const minutes = Math.floor((milliseconds % 3_600_000) / 60_000);
  const seconds = Math.floor((milliseconds % 60_000) / 1000);
  const millis = milliseconds % 1000;
  return `${pad2(hours)}:${pad2(minutes)}:${pad2(seconds)}.${String(millis).padStart(3, "0")}`;
}

function pad2(value: number): string {
  return String(value).padStart(2, "0");
}

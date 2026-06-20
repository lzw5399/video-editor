import { _electron as electron, expect, type ElectronApplication, type Page } from "@playwright/test";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { access } from "node:fs/promises";
import { basename, join } from "node:path";
import { promisify } from "node:util";

import type { CommandName } from "../../src/generated/CommandEnvelope";
import { launchForegroundProductApp, type ForegroundProductAppController } from "./foregroundProductApp";

export const USER_JOURNEY_MEDIA_DIR = join(process.cwd(), "tests/fixtures/media");
export const USER_JOURNEY_MOVING_VIDEO = join(USER_JOURNEY_MEDIA_DIR, "p0-moving-testsrc.mp4");
export const USER_JOURNEY_TONE_AUDIO = join(USER_JOURNEY_MEDIA_DIR, "p0-tone.wav");
const execFileAsync = promisify(execFile);

type ExecuteCommandCall = {
  command: CommandName;
  kind: string;
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
};

export type ProductJourneyAppController = {
  readonly kind: "electron-launch" | "foreground-cdp";
  close: () => Promise<void>;
  readExecuteCommandCalls: () => Promise<ExecuteCommandCall[]>;
  readRealtimePreviewHostCalls: () => Promise<RealtimePreviewHostCall[]>;
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
  timecodeUs: number;
  placeholderText: string;
  imageSrc: string | null;
  hostState: RealtimePreviewHostState | null;
};

export async function waitForCompositedPreviewEvidence(
  page: Page,
  app?: ProductJourneyAppController,
  timeoutMs = 8_000
): Promise<PreviewEvidence> {
  const deadline = Date.now() + timeoutMs;
  let lastEvidence: PreviewEvidence | null = null;

  while (Date.now() < deadline) {
    lastEvidence = await capturePreviewEvidence(page);
    if (lastEvidence.hostState?.contentEvidence?.source === "renderGraphGpuComposited") {
      return lastEvidence;
    }
    await page.waitForTimeout(250);
  }

  const hostCalls = app === undefined ? [] : await readRealtimePreviewHostCalls(app);
  throw new Error(
    `Timed out waiting for composited preview evidence. Last host state: ${JSON.stringify(
      lastEvidence?.hostState ?? null
    )}. Host calls: ${JSON.stringify(hostCalls)}`
  );
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

  if (process.platform === "darwin") {
    const launch = await launchForegroundProductApp(openMaterialFiles, env);
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
      VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
      VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "0",
      VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify(openMaterialFiles),
      ...env
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

export async function clickPreviewPlay(page: Page): Promise<void> {
  const controls = page.getByRole("group", { name: "预览播放控制" });
  const playButton = controls.getByRole("button", { name: "播放预览" });
  await expect(playButton).toBeEnabled({ timeout: 20_000 });
  await playButton.click();
  await expect(controls.getByRole("button", { name: "暂停预览" })).toBeEnabled({ timeout: 10_000 });
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
  const placeholder = page.locator(".preview-placeholder");
  const image = page.getByRole("img", { name: "当前预览帧" });

  return {
    regionHash: hashBuffer(screenshot),
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
      })
  };
}

function wrapForegroundController(app: ForegroundProductAppController): ProductJourneyAppController {
  return {
    kind: app.kind,
    close: () => app.close(),
    readExecuteCommandCalls: async () => (await app.readExecuteCommandCalls()) as ExecuteCommandCall[],
    readRealtimePreviewHostCalls: async () => (await app.readRealtimePreviewHostCalls()) as RealtimePreviewHostCall[]
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

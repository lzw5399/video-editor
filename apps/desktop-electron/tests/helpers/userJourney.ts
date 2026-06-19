import { _electron as electron, expect, type ElectronApplication, type Page } from "@playwright/test";
import { createHash } from "node:crypto";
import { access } from "node:fs/promises";
import { basename, join } from "node:path";

import type { CommandName } from "../../src/generated/CommandEnvelope";

export const USER_JOURNEY_MEDIA_DIR = join(process.cwd(), "tests/fixtures/media");
export const USER_JOURNEY_MOVING_VIDEO = join(USER_JOURNEY_MEDIA_DIR, "p0-moving-testsrc.mp4");
export const USER_JOURNEY_TONE_AUDIO = join(USER_JOURNEY_MEDIA_DIR, "p0-tone.wav");

type ExecuteCommandCall = {
  command: CommandName;
  kind: string;
};

type RealtimePreviewHostCall = {
  kind: string;
  targetTimeMicroseconds?: number;
  playbackGeneration?: number;
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

  throw new Error(
    `Timed out waiting for composited preview evidence. Last host state: ${JSON.stringify(
      lastEvidence?.hostState ?? null
    )}`
  );
}

export async function launchProductJourneyApp(
  openMaterialFiles: string[],
  env: NodeJS.ProcessEnv = {}
): Promise<{ app: ElectronApplication; page: Page }> {
  await Promise.all(openMaterialFiles.map((filePath) => expectFileExists(filePath)));

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
  await expectProductWorkspace(page);
  return { app, page };
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
  app: ElectronApplication,
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
  app: ElectronApplication,
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

export async function readExecuteCommandCalls(app: ElectronApplication): Promise<ExecuteCommandCall[]> {
  return app.evaluate(() => {
    return (
      (globalThis as typeof globalThis & { __videoEditorTestExecuteCommandCalls?: ExecuteCommandCall[] })
        .__videoEditorTestExecuteCommandCalls ?? []
    );
  });
}

export async function readRealtimePreviewHostCalls(app: ElectronApplication): Promise<RealtimePreviewHostCall[]> {
  return app.evaluate(() => {
    return (
      (globalThis as typeof globalThis & { __videoEditorTestRealtimePreviewHostCalls?: RealtimePreviewHostCall[] })
        .__videoEditorTestRealtimePreviewHostCalls ?? []
    );
  });
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

async function waitForCommandCount(app: ElectronApplication, command: CommandName, expectedCount: number): Promise<void> {
  await expect.poll(async () => countCommand(app, command), { timeout: 30_000 }).toBeGreaterThanOrEqual(expectedCount);
}

async function countCommand(app: ElectronApplication, command: CommandName): Promise<number> {
  return (await readExecuteCommandCalls(app)).filter((call) => call.command === command).length;
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

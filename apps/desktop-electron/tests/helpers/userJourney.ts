import { _electron as electron, expect, type ElectronApplication, type Page } from "@playwright/test";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { access, readFile, unlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { promisify } from "node:util";

import {
  launchForegroundProductApp,
  type ForegroundProductAppController,
  type ForegroundProductAppDiagnostics,
  type ProductWindowMetrics
} from "./foregroundProductApp";

export const USER_JOURNEY_MEDIA_DIR = join(process.cwd(), "tests/fixtures/media");
export const USER_JOURNEY_MOVING_VIDEO = join(USER_JOURNEY_MEDIA_DIR, "p0-moving-testsrc.mp4");
export const USER_JOURNEY_AV_VIDEO = join(USER_JOURNEY_MEDIA_DIR, "p0-av-tone-testsrc.mp4");
export const USER_JOURNEY_LONG_MOVING_VIDEO = join(USER_JOURNEY_MEDIA_DIR, "p0-long-moving-testsrc.mp4");
export const USER_JOURNEY_LONG_AV_VIDEO = join(USER_JOURNEY_MEDIA_DIR, "p0-long-av-tone-testsrc.mp4");
export const USER_JOURNEY_OVERLAY_IMAGE = join(USER_JOURNEY_MEDIA_DIR, "p0-overlay-testsrc.png");
export const USER_JOURNEY_TONE_AUDIO = join(USER_JOURNEY_MEDIA_DIR, "p0-tone.wav");
export const USER_JOURNEY_LONG_TONE_AUDIO = join(USER_JOURNEY_MEDIA_DIR, "p0-long-tone.wav");
const TIMELINE_RULER_CLICK_TOLERANCE_US = 10_000;
const DEFAULT_INTENT_SEGMENT_DURATION_US = 3_000_000;
const execFileAsync = promisify(execFile);

type NativeCommandObservation = {
  command: string;
  kind: string;
  targetTime?: number | null;
  targetTimerange?: { start: number; duration: number } | null;
  duration?: number | null;
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
  srtContent?: string | null;
  outputPath?: string | null;
  preset?: string | null;
  sessionId?: string | null;
  projectSessionId?: string | null;
  expectedRevision?: number | null;
  hasDraftField?: boolean;
  deviceSelectionId?: string | null;
  maxPeakBins?: number | null;
};

type ProjectSessionCall = {
  command:
    | "createProjectSession"
    | "openProjectSession"
    | "executeProjectIntent"
    | "listProjectSessionMaterials"
    | "listProjectSessionMissingMaterials"
    | "startProjectSessionExport"
    | "closeProjectSession";
  sessionId: string | null;
  expectedRevision: number | null;
  intentKind: string | null;
  materialId: string | null;
  materialPath: string | null;
  outputPath?: string | null;
  preset?: string | null;
  targetTime?: number | null;
  targetTimerange?: { start: number; duration: number } | null;
  duration?: number | null;
  visual?: NativeCommandObservation["visual"] | null;
  textContent?: string | null;
  textSource?: string | null;
  textFontRef?: string | null;
  srtContent?: string | null;
  timelineSemanticKeys?: string[];
  hasDraftField: boolean;
};

type RealtimePreviewHostCall = {
  kind: string;
  nativeEventKind?: string;
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
  durationMs?: number;
  presentedFrameCount?: number;
  droppedFrameCount?: number;
  errorMessage?: string;
  presentationAvailable?: boolean;
  presentationBackend?: string;
  unsupportedReason?: string | null;
};

type RealtimePreviewHostState = {
  ok: boolean;
  productReady: boolean;
  hostAttached: boolean;
  fallbackActive: boolean;
  statusLabel: string;
  fallbackLabel: string | null;
  unsupportedReason: string | null;
  playbackGeneration: number | null;
  backend: "renderGraphGpu" | "none";
  diagnosticSource: "nativeVideoBridge" | "runtimeFrameRequest" | "none";
  telemetry: {
    firstFrameLatencyMs?: number | null;
    renderDurationMs?: number;
    presentedFrameCount: number;
    droppedFrameCount?: number;
    targetTimeMicroseconds: number;
    playbackGeneration: number;
    framePacing?: {
      sampleCount: number;
      intervalP50Ms: number | null;
      intervalP95Ms: number | null;
      intervalMaxMs: number | null;
      scheduleLatenessP95Ms: number | null;
      scheduleLatenessMaxMs: number | null;
      samples: Array<{
        targetTimeMicroseconds: number;
        intervalMs?: number | null;
        scheduleLatenessMs: number;
        renderDurationMs: number;
        droppedFrameCount: number;
      }>;
    };
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
    presentedFrames: number;
    submittedDraws: number;
    activeTextOverlays?: Array<{
      source: "text" | "subtitle";
      content: string;
    }>;
  } | null;
  surfacePlacement?: {
    surfaceBoundsCoordinateSpace: "browserWindowContentLogicalPixels";
    screenRectCoordinateSpace: "electronScreenLogicalPixels";
    hostScreenRect: { x: number; y: number; width: number; height: number };
    nativeScreenRect: { x: number; y: number; width: number; height: number };
    nativeAppKitScreenRect: { x: number; y: number; width: number; height: number };
    nativeDrawableLifecycleDiagnostic: string | null;
    deltaPx: { x: number; y: number; width: number; height: number };
    maxDeltaPx: number;
    aligned: boolean;
  } | null;
};

export type ProductJourneyAppController = {
  readonly kind: "electron-launch" | "foreground-cdp";
  close: () => Promise<void>;
  readNativeCommandObservations: () => Promise<NativeCommandObservation[]>;
  readProjectSessionCalls: () => Promise<ProjectSessionCall[]>;
  readRealtimePreviewHostCalls: () => Promise<RealtimePreviewHostCall[]>;
  readForegroundDiagnostics: () => Promise<ForegroundProductAppDiagnostics | null>;
  readWindowMetrics: () => Promise<ProductWindowMetrics | null>;
  maximizeMainWindow: () => Promise<ProductWindowMetrics | null>;
  resizeMainWindow: (width: number, height: number) => Promise<ProductWindowMetrics | null>;
};

export type PreviewEvidence = {
  regionHash: string;
  visibleCenterHash: string;
  timecodeUs: number;
  placeholderText: string;
  imageSrc: string | null;
  hostState: RealtimePreviewHostState | null;
};

const PREVIEW_COVERAGE_REGIONS = [
  { name: "topLeft", region: { x: 0.08, y: 0.08, width: 0.2, height: 0.2 } },
  { name: "topRight", region: { x: 0.72, y: 0.08, width: 0.2, height: 0.2 } },
  { name: "center", region: { x: 0.38, y: 0.36, width: 0.24, height: 0.24 } },
  { name: "bottomLeft", region: { x: 0.08, y: 0.72, width: 0.2, height: 0.2 } },
  { name: "bottomRight", region: { x: 0.72, y: 0.72, width: 0.2, height: 0.2 } }
] as const;

export type PreviewCoverageRegionName = (typeof PREVIEW_COVERAGE_REGIONS)[number]["name"];

export type PreviewCoverageEvidence = {
  visibleRegionHashes: Record<PreviewCoverageRegionName, string>;
  hostBox: { x: number; y: number; width: number; height: number } | null;
  canvasBox: { x: number; y: number; width: number; height: number } | null;
  hostState: RealtimePreviewHostState | null;
};

export type ProductPlaybackSuccessEvidence = {
  after: PreviewEvidence;
  visibleMotion: PreviewEvidence;
};

export type TimelineSegmentSnapshot = {
  label: string;
  targetLabel: string;
  targetStartUs: number;
  targetDurationUs: number;
  selected: boolean;
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

export async function waitForProductPlaybackSuccess(
  page: Page,
  app: ProductJourneyAppController,
  before: PreviewEvidence,
  visibleBefore: PreviewEvidence,
  frameRequestsBeforePlay: number,
  timeoutMs = 12_000
): Promise<ProductPlaybackSuccessEvidence> {
  const visibleMotion = await waitForVisiblePreviewCenterChange(page, app, visibleBefore.visibleCenterHash, Math.min(timeoutMs, 5_000));
  const after = await waitForCompositedPreviewEvidence(
    page,
    app,
    timeoutMs,
    before.hostState?.contentEvidence?.targetTimeMicroseconds ?? before.timecodeUs
  );
  expectProductPlaybackSuccessEvidence({
    before,
    visibleBefore,
    visibleMotion,
    after,
    frameRequestsBeforePlay,
    frameRequestsAfterPlay: requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app))
  });
  return { after, visibleMotion };
}

export function expectProductPlaybackSuccessEvidence({
  before,
  visibleBefore,
  visibleMotion,
  after,
  frameRequestsBeforePlay,
  frameRequestsAfterPlay
}: {
  before: PreviewEvidence;
  visibleBefore: PreviewEvidence;
  visibleMotion: PreviewEvidence;
  after: PreviewEvidence;
  frameRequestsBeforePlay: number;
  frameRequestsAfterPlay: number;
}): void {
  expect(after.hostState?.ok, "product playback requires an ok realtime host state").toBe(true);
  expect(after.hostState?.productReady, "product playback requires product-ready realtime preview").toBe(true);
  expect(after.hostState?.fallbackActive, "product playback must not be a fallback path").toBe(false);
  expect(after.hostState?.backend, "product playback success backend must be renderGraphGpu").toBe("renderGraphGpu");
  expect(after.hostState?.diagnosticSource, "product playback success must not come from diagnostic sources").toBe("none");
  expect(
    after.hostState?.contentEvidence?.source,
    "product playback success requires render-graph GPU composited evidence"
  ).toBe("renderGraphGpuComposited");
  expect(after.hostState?.contentEvidence?.digest).not.toBe(before.hostState?.contentEvidence?.digest ?? null);
  expect(after.hostState?.contentEvidence?.targetTimeMicroseconds ?? 0).toBeGreaterThan(
    before.hostState?.contentEvidence?.targetTimeMicroseconds ?? 0
  );
  expect(after.hostState?.telemetry?.presentedFrameCount ?? 0).toBeGreaterThan(
    before.hostState?.telemetry?.presentedFrameCount ?? 0
  );
  expect(after.timecodeUs, "product playback requires timeline time advancement").toBeGreaterThan(before.timecodeUs);
  expect(
    visibleMotion.visibleCenterHash,
    "visible video pixels in the preview center must change while playback is running"
  ).not.toBe(visibleBefore.visibleCenterHash);
  expect(
    frameRequestsAfterPlay,
    "product playback must not drive a requestProjectSessionPreviewFrame PNG/artifact loop"
  ).toBe(frameRequestsBeforePlay);
  expect(after.hostState?.frameDisplay).toBeNull();
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

export async function captureVisiblePreviewCoverageEvidence(
  page: Page,
  app: ProductJourneyAppController | undefined
): Promise<PreviewCoverageEvidence> {
  const host = page.getByLabel("实时预览画面", { exact: true });
  const canvas = page.getByLabel("预览画面", { exact: true });
  const visibleRegionHashes = Object.fromEntries(
    await Promise.all(
      PREVIEW_COVERAGE_REGIONS.map(async ({ name, region }) => [
        name,
        hashBuffer(await captureVisiblePreviewRegion(page, app, region))
      ])
    )
  ) as Record<PreviewCoverageRegionName, string>;

  return {
    visibleRegionHashes,
    hostBox: await host.boundingBox(),
    canvasBox: await canvas.boundingBox(),
    hostState: await readRealtimePreviewHostState(page)
  };
}

export async function captureVisiblePreviewHostImage(
  page: Page,
  app: ProductJourneyAppController | undefined
): Promise<Buffer> {
  return captureVisiblePreviewRegion(page, app, {
    x: 0,
    y: 0,
    width: 1,
    height: 1
  });
}

export function expectVisiblePreviewCoverageChanged(before: PreviewCoverageEvidence, after: PreviewCoverageEvidence): void {
  const unchangedRegions = PREVIEW_COVERAGE_REGIONS.map(({ name }) => name).filter(
    (name) => before.visibleRegionHashes[name] === after.visibleRegionHashes[name]
  );
  expect(
    unchangedRegions,
    `native preview pixels must change across the full host, not only a lower-left drawable subsection: ${JSON.stringify({
      before,
      after
    })}`
  ).toEqual([]);
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
    "childViewWindowAttached=",
    "childViewHasSuperview=",
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
    "childViewScreenFrame=",
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
  const projectBundlePath = env.VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE ?? createProductJourneyProjectPath();
  const productEnv = {
    VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
    VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "0",
    VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "0",
    VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE: projectBundlePath,
    ...env
  };

  if (process.platform === "darwin") {
    const launch = await launchForegroundProductApp(openMaterialFiles, productEnv);
    await createProjectFromProductEntry(wrapForegroundController(launch.app), launch.page);
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
  const controller = wrapElectronApp(app);
  await createProjectFromProductEntry(controller, page);
  await expectProductWorkspace(page);
  return { app: controller, page };
}

export async function expectProductEntry(page: Page): Promise<void> {
  await expect(page.getByRole("main", { name: "项目入口" })).toBeVisible();
  await expect(page.getByRole("button", { name: "新建项目" })).toBeVisible();
  await expect(page.getByRole("button", { name: "打开项目" })).toBeVisible();
  await expect(page.getByRole("button", { name: "导入素材" })).toHaveCount(0);
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toHaveCount(0);
}

export async function createProjectFromProductEntry(app: ProductJourneyAppController, page: Page): Promise<void> {
  await expectProductEntry(page);
  const nextCount = (await countProjectSessionCommand(app, "createProjectSession")) + 1;
  await page.getByRole("button", { name: "新建项目" }).click();
  await waitForProjectSessionCommandCount(app, "createProjectSession", nextCount);
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

function createProductJourneyProjectPath(): string {
  return join(tmpdir(), `video-editor-product-${Date.now()}-${Math.random().toString(16).slice(2)}.veproj`);
}

export async function importMaterialThroughProductPicker(
  app: ProductJourneyAppController,
  page: Page,
  materialPath: string
): Promise<void> {
  const materialName = basename(materialPath);
  const nextCount = (await countProjectSessionIntent(app, "importMaterial")) + 1;
  await page.getByRole("button", { name: "导入素材" }).click();
  await waitForProjectSessionIntentCount(app, "importMaterial", nextCount);
  await expect(page.getByRole("article", { name: `素材 ${materialName}` })).toBeVisible({
    timeout: 30_000
  });
}

export async function importMaterialsThroughProductPicker(
  app: ProductJourneyAppController,
  page: Page,
  materialPaths: string[]
): Promise<void> {
  const nextCount = (await countProjectSessionIntent(app, "importMaterial")) + materialPaths.length;
  await page.getByRole("button", { name: "导入素材" }).click();
  await waitForProjectSessionIntentCount(app, "importMaterial", nextCount);
  for (const materialPath of materialPaths) {
    const materialName = basename(materialPath);
    await expect(page.getByRole("article", { name: `素材 ${materialName}` })).toBeVisible({
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
  const nextCount = (await countProjectSessionIntent(app, "addTimelineSegmentIntent")) + 1;
  const materialRow = page.getByRole("article", { name: `素材 ${materialName}` });
  await expect(materialRow).toBeVisible({ timeout: 10_000 });
  const addButton = materialRow.getByRole("button", { name: `添加 ${materialName} 到时间线` });
  await expect(addButton).toBeEnabled({ timeout: 60_000 });
  await addButton.click();
  await waitForProjectSessionIntentCount(app, "addTimelineSegmentIntent", nextCount);
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(materialName)}`) })).toBeVisible();
  await expect(page.getByLabel("预览选中框")).toBeVisible();
}

export async function dragMaterialToTimeline(
  app: ProductJourneyAppController,
  page: Page,
  materialPath: string
): Promise<void> {
  const materialName = basename(materialPath);
  const nextCount = (await countProjectSessionIntent(app, "addTimelineSegmentIntent")) + 1;
  const materialRow = page.getByRole("article", { name: `素材 ${materialName}` });
  const timelineDropTarget = page.locator('[data-material-drop-target="true"]');

  await expect(materialRow).toBeVisible({ timeout: 10_000 });
  await expect(materialRow).toHaveAttribute("draggable", "true", { timeout: 60_000 });
  await expect(timelineDropTarget).toBeVisible();
  await materialRow.dragTo(timelineDropTarget);
  await waitForProjectSessionIntentCount(app, "addTimelineSegmentIntent", nextCount);
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(materialName)}`) })).toBeVisible();
  await expect(page.getByLabel("预览选中框")).toBeVisible();
}

export async function addVideoTrack(page: Page, app: ProductJourneyAppController): Promise<void> {
  const nextCount = (await countCommand(app, "addTrackIntent")) + 1;
  await page.getByRole("button", { name: "添加视频轨道" }).click();
  await waitForCommandCount(app, "addTrackIntent", nextCount);
  await expect(page.getByRole("button", { name: /选择轨道 视频轨道 2/ })).toBeVisible();
}

export async function addTextThroughProductPanel(
  page: Page,
  app: ProductJourneyAppController,
  content: string,
  expectedDurationUs = DEFAULT_INTENT_SEGMENT_DURATION_US
): Promise<void> {
  const nextCount = (await countCommand(app, "addTextSegmentIntent")) + 1;
  await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "文本" }).click();
  const textPanel = page.getByRole("region", { name: "素材面板" });
  await textPanel.getByLabel("默认文字").getByLabel("文字内容").fill(content);
  await textPanel.getByRole("button", { name: "添加文字", exact: true }).click();
  await waitForCommandCount(app, "addTextSegmentIntent", nextCount);
  await expectTimelineSegmentDuration(page, new RegExp(escapeRegex(content)), expectedDurationUs);
  await expect(page.getByRole("complementary", { name: "属性检查器" }).getByRole("textbox", { name: "文字内容" })).toHaveValue(
    content
  );
}

export async function importSubtitleSrtThroughProductPanel(
  page: Page,
  app: ProductJourneyAppController,
  srtContent: string
): Promise<void> {
  const nextCount = (await countCommand(app, "importSubtitleSrtIntent")) + 1;
  await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "字幕" }).click();
  const captionsPanel = page.getByRole("region", { name: "素材面板" });
  await expect(captionsPanel).not.toContainText("字幕暂未开放");
  await captionsPanel.getByLabel("SRT 内容").fill(srtContent);
  await captionsPanel.getByRole("button", { name: "导入字幕", exact: true }).click();
  await waitForCommandCount(app, "importSubtitleSrtIntent", nextCount);
  const lastImport = (await readNativeCommandObservations(app)).findLast((call) => call.command === "importSubtitleSrtIntent");
  expect(lastImport?.srtContent).toBe(srtContent);
  const firstCueText = firstSrtCueText(srtContent);
  if (firstCueText.length > 0) {
    await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(firstCueText.slice(0, 32))}`) })).toBeVisible({
      timeout: 10_000
    });
  }
}

export async function addAudioThroughProductPanel(
  page: Page,
  app: ProductJourneyAppController,
  audioPath: string,
  expectedDurationUs = DEFAULT_INTENT_SEGMENT_DURATION_US
): Promise<void> {
  const nextCount = (await countCommand(app, "addAudioSegmentIntent")) + 1;
  await page.getByRole("navigation", { name: "顶部功能区" }).getByRole("button", { name: "音频" }).click();
  const audioPanel = page.getByRole("region", { name: "素材面板" });
  await audioPanel.getByLabel("BGM素材").selectOption({ label: basename(audioPath) });
  await audioPanel.getByRole("button", { name: "添加音频", exact: true }).click();
  await waitForCommandCount(app, "addAudioSegmentIntent", nextCount);
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(basename(audioPath))}`) })).toBeVisible();
  await expectTimelineSegmentDuration(page, new RegExp(escapeRegex(basename(audioPath))), expectedDurationUs);
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
  const nextCount = (await countCommand(app, "updateSelectedSegmentVisual")) + 1;
  const visualTab = page.getByRole("tab", { name: "画面" });
  if ((await visualTab.count()) > 0) {
    await visualTab.click();
  }
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
  await waitForCommandCount(app, "updateSelectedSegmentVisual", nextCount);
}

export async function seekTimelinePlayhead(page: Page, app: ProductJourneyAppController, targetTimeUs: number): Promise<void> {
  const frameRequestsBefore = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
  await clickTimelineRulerAt(page, targetTimeUs);
  await expect
    .poll(async () => parseTimecodeToMicroseconds((await page.getByLabel("当前时间码").textContent()) ?? ""), {
      timeout: 10_000
    })
    .toBeGreaterThanOrEqual(targetTimeUs - TIMELINE_RULER_CLICK_TOLERANCE_US);
  await expect
    .poll(async () => parseTimecodeToMicroseconds((await page.getByLabel("当前时间码").textContent()) ?? ""), {
      timeout: 10_000
    })
    .toBeLessThanOrEqual(targetTimeUs + TIMELINE_RULER_CLICK_TOLERANCE_US);
  expect(
    requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app)),
    "product seek must not fall back to preview artifact frame requests"
  ).toBe(frameRequestsBefore);
}

export async function splitSelectedSegment(page: Page, app: ProductJourneyAppController, splitAtUs: number): Promise<void> {
  const nextCount = (await countCommand(app, "splitSelectedSegmentIntent")) + 1;
  await seekTimelinePlayhead(page, app, splitAtUs);
  await page.getByRole("button", { name: "分割所选片段" }).click();
  await waitForCommandCount(app, "splitSelectedSegmentIntent", nextCount);
}

export async function moveSelectedSegmentRight(page: Page, app: ProductJourneyAppController, deltaUs: number): Promise<void> {
  const nextCount = (await countCommand(app, "moveSelectedSegmentIntent")) + 1;
  await dragSelectedSegmentBy(page, deltaUs);
  await waitForCommandCount(app, "moveSelectedSegmentIntent", nextCount);
}

export async function trimSelectedSegmentLeftEdgeRight(
  page: Page,
  app: ProductJourneyAppController,
  deltaUs: number
): Promise<void> {
  const nextCount = (await countCommand(app, "trimSelectedSegmentIntent")) + 1;
  const handle = page.locator(".segment-block.selected .segment-trim-handle.left").first();
  const handleBox = await handle.boundingBox();
  const rulerBox = await page.locator(".ruler-track").boundingBox();
  if (handleBox === null || rulerBox === null) {
    throw new Error("Selected segment trim handle or timeline ruler is not visible for trim interaction");
  }

  const deltaPx = Math.max(6, (Math.abs(deltaUs) / 10_000_000) * rulerBox.width);
  const startX = handleBox.x + handleBox.width / 2;
  const startY = handleBox.y + handleBox.height / 2;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX + deltaPx, startY, { steps: 4 });
  await page.mouse.up();
  await waitForCommandCount(app, "trimSelectedSegmentIntent", nextCount);
}

export async function deleteSelectedSegment(page: Page, app: ProductJourneyAppController): Promise<void> {
  const nextCount = (await countCommand(app, "deleteSelectedSegment")) + 1;
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "删除所选片段" }).click();
  await waitForCommandCount(app, "deleteSelectedSegment", nextCount);
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

export async function zoomTimelineIn(page: Page): Promise<void> {
  const content = page.locator(".track-scroll-content");
  const widthBefore = await content.evaluate((element) => element.getBoundingClientRect().width);
  await page.getByRole("button", { name: "放大时间线" }).click();
  await expect(page.getByLabel("时间线缩放", { exact: true })).toContainText("125%");
  await expect
    .poll(async () => content.evaluate((element) => element.getBoundingClientRect().width))
    .toBeGreaterThan(widthBefore);
}

export async function expectTimelineSnappingStatusVisible(page: Page): Promise<void> {
  const snapping = page.locator(".snapping-status");
  await expect(snapping).toHaveAttribute("aria-label", /吸附/);
  await expect(snapping).toHaveAttribute("aria-pressed", /true|false/);
}

async function clickTimelineRulerAt(page: Page, targetTimeUs: number): Promise<void> {
  const ruler = page.locator(".ruler-track");
  const rulerBox = await ruler.boundingBox();
  if (rulerBox === null) {
    throw new Error("Timeline ruler is not visible for seek interaction");
  }
  const ratio = Math.max(0, Math.min(1, targetTimeUs / 10_000_000));
  await page.mouse.click(rulerBox.x + rulerBox.width * ratio, rulerBox.y + rulerBox.height / 2);
}

async function dragSelectedSegmentBy(page: Page, deltaUs: number): Promise<void> {
  const segment = page.locator(".segment-block.selected").first();
  const segmentBox = await segment.boundingBox();
  const rulerBox = await page.locator(".ruler-track").boundingBox();
  if (segmentBox === null || rulerBox === null) {
    throw new Error("Selected segment or timeline ruler is not visible for move interaction");
  }

  const deltaPx = (deltaUs / 10_000_000) * rulerBox.width;
  const startX = segmentBox.x + Math.max(12, Math.min(segmentBox.width - 12, segmentBox.width / 2));
  const startY = segmentBox.y + segmentBox.height / 2;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX + deltaPx, startY, { steps: 4 });
  await page.mouse.up();
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

export async function readNativeCommandObservations(app: ProductJourneyAppController): Promise<NativeCommandObservation[]> {
  const [directNativeObservations, sessionCalls] = await Promise.all([
    app.readNativeCommandObservations(),
    app.readProjectSessionCalls()
  ]);
  return [
    ...directNativeObservations,
    ...sessionCalls
      .filter(
        (call) =>
          (call.command === "executeProjectIntent" && call.intentKind !== null) ||
          call.command === "startProjectSessionExport"
      )
      .map(projectSessionCallToNativeObservation)
  ];
}

export async function readDirectNativeCommandObservations(app: ProductJourneyAppController): Promise<NativeCommandObservation[]> {
  return app.readNativeCommandObservations();
}

export async function readProjectSessionCalls(app: ProductJourneyAppController): Promise<ProjectSessionCall[]> {
  return app.readProjectSessionCalls();
}

export async function readRealtimePreviewHostCalls(app: ProductJourneyAppController): Promise<RealtimePreviewHostCall[]> {
  return app.readRealtimePreviewHostCalls();
}

export function requestProjectSessionPreviewFrameCount(calls: NativeCommandObservation[]): number {
  return calls.filter((call) => call.command === "requestProjectSessionPreviewFrame").length;
}

export async function readTimelineSegments(
  page: Page,
  labelFilter?: string | RegExp
): Promise<TimelineSegmentSnapshot[]> {
  const segments = await page.locator(".segment-block").evaluateAll((elements) =>
    elements.map((element) => {
      const block = element as HTMLElement;
      return {
        label: block.querySelector("strong")?.textContent?.trim() ?? "",
        targetLabel: block.querySelector(".segment-time-label")?.textContent?.trim() ?? "",
        selected: block.classList.contains("selected") || block.getAttribute("aria-pressed") === "true"
      };
    })
  );

  return segments
    .map((segment) => {
      const target = parseTimelineTargetLabel(segment.targetLabel);
      return {
        ...segment,
        targetStartUs: target?.startUs ?? 0,
        targetDurationUs: target?.durationUs ?? 0
      };
    })
    .filter((segment) => {
      if (labelFilter === undefined) {
        return true;
      }
      return typeof labelFilter === "string" ? segment.label.includes(labelFilter) : labelFilter.test(segment.label);
    });
}

async function expectTimelineSegmentDuration(page: Page, labelFilter: RegExp, expectedDurationUs: number): Promise<void> {
  await expect
    .poll(async () => {
      const segments = await readTimelineSegments(page, labelFilter);
      return segments.findLast((segment) => segment.selected)?.targetDurationUs ?? segments.at(-1)?.targetDurationUs ?? null;
    })
    .toBe(expectedDurationUs);
}

async function readRealtimePreviewHostState(page: Page): Promise<RealtimePreviewHostState | null> {
  await page.evaluate(() => {
    const target = window as typeof window & {
      __videoEditorRealtimePreviewHostState?: RealtimePreviewHostState | null;
      __videoEditorRealtimePreviewHostObserverInstalled?: boolean;
      videoEditorRealtimePreviewHost?: {
        subscribeTelemetry: (listener: (state: RealtimePreviewHostState) => void) => () => void;
      };
    };
    if (target.__videoEditorRealtimePreviewHostObserverInstalled) {
      return;
    }
    target.__videoEditorRealtimePreviewHostObserverInstalled = true;
    target.__videoEditorRealtimePreviewHostState = null;
    target.videoEditorRealtimePreviewHost?.subscribeTelemetry((state) => {
      target.__videoEditorRealtimePreviewHostState = state;
    });
  });
  return page.evaluate(() => {
    return (
      (window as typeof window & {
        __videoEditorRealtimePreviewHostState?: RealtimePreviewHostState | null;
      }).__videoEditorRealtimePreviewHostState ?? null
    );
  });
}

async function waitForCommandCount(app: ProductJourneyAppController, command: string, expectedCount: number): Promise<void> {
  await expect.poll(async () => countCommand(app, command), { timeout: 30_000 }).toBeGreaterThanOrEqual(expectedCount);
}

async function waitForProjectSessionCommandCount(
  app: ProductJourneyAppController,
  command: ProjectSessionCall["command"],
  expectedCount: number
): Promise<void> {
  await expect.poll(async () => countProjectSessionCommand(app, command), { timeout: 30_000 }).toBeGreaterThanOrEqual(expectedCount);
}

async function waitForProjectSessionIntentCount(
  app: ProductJourneyAppController,
  intentKind: string,
  expectedCount: number
): Promise<void> {
  await expect.poll(async () => countProjectSessionIntent(app, intentKind), { timeout: 30_000 }).toBeGreaterThanOrEqual(expectedCount);
}

async function countCommand(app: ProductJourneyAppController, command: string): Promise<number> {
  return (await readNativeCommandObservations(app)).filter((call) => call.command === command).length;
}

async function countProjectSessionCommand(app: ProductJourneyAppController, command: ProjectSessionCall["command"]): Promise<number> {
  return (await readProjectSessionCalls(app)).filter((call) => call.command === command).length;
}

async function countProjectSessionIntent(app: ProductJourneyAppController, intentKind: string): Promise<number> {
  return (await readProjectSessionCalls(app)).filter((call) => call.command === "executeProjectIntent" && call.intentKind === intentKind).length;
}

function projectSessionCallToNativeObservation(call: ProjectSessionCall): NativeCommandObservation {
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
    duration: call.duration ?? null,
    visual: call.visual ?? null,
    textContent: call.textContent ?? null,
    textSource: call.textSource ?? null,
    textFontRef: call.textFontRef ?? null,
    srtContent: call.srtContent ?? null,
    outputPath: call.outputPath ?? null,
    preset: call.preset ?? null,
    sessionId: call.sessionId,
    deviceSelectionId: null,
    maxPeakBins: null
  };
}

function wrapElectronApp(app: ElectronApplication): ProductJourneyAppController {
  return {
    kind: "electron-launch",
    close: () => app.close(),
    readForegroundDiagnostics: async () => null,
    readNativeCommandObservations: () =>
      app.evaluate(() => {
        return (
          (globalThis as typeof globalThis & { __videoEditorTestNativeCommandObservations?: NativeCommandObservation[] })
            .__videoEditorTestNativeCommandObservations ?? []
        );
      }),
    readProjectSessionCalls: () =>
      app.evaluate(() => {
        return (
          (globalThis as typeof globalThis & { __videoEditorTestProjectSessionCalls?: ProjectSessionCall[] })
            .__videoEditorTestProjectSessionCalls ?? []
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
      }),
    maximizeMainWindow: async () =>
      app.evaluate(({ BrowserWindow, screen }) => {
        const window = BrowserWindow.getAllWindows()[0];
        if (window === undefined) {
          return null;
        }
        window.maximize();
        return {
          bounds: window.getBounds(),
          contentBounds: window.getContentBounds(),
          displayScaleFactor: screen.getDisplayMatching(window.getBounds()).scaleFactor
        };
      }),
    resizeMainWindow: async (width, height) =>
      app.evaluate(({ BrowserWindow, screen }, size) => {
        const window = BrowserWindow.getAllWindows()[0];
        if (window === undefined) {
          return null;
        }
        window.unmaximize();
        window.setSize(size.width, size.height);
        return {
          bounds: window.getBounds(),
          contentBounds: window.getContentBounds(),
          displayScaleFactor: screen.getDisplayMatching(window.getBounds()).scaleFactor
        };
      }, { width, height })
  };
}

function wrapForegroundController(app: ForegroundProductAppController): ProductJourneyAppController {
  return {
    kind: app.kind,
    close: () => app.close(),
    readForegroundDiagnostics: () => app.readForegroundDiagnostics(),
    readNativeCommandObservations: async () => (await app.readNativeCommandObservations()) as NativeCommandObservation[],
    readProjectSessionCalls: async () => (await app.readProjectSessionCalls()) as ProjectSessionCall[],
    readRealtimePreviewHostCalls: async () => (await app.readRealtimePreviewHostCalls()) as RealtimePreviewHostCall[],
    readWindowMetrics: () => app.readWindowMetrics(),
    maximizeMainWindow: () => app.maximizeMainWindow(),
    resizeMainWindow: (width, height) => app.resizeMainWindow(width, height)
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
  return captureVisiblePreviewRegion(page, app, {
    x: 0.28,
    y: 0.22,
    width: 0.44,
    height: 0.42
  });
}

async function captureVisiblePreviewRegion(
  page: Page,
  app: ProductJourneyAppController | undefined,
  region: { x: number; y: number; width: number; height: number }
): Promise<Buffer> {
  const host = page.getByLabel("实时预览画面", { exact: true });
  await expect(host).toBeVisible();
  const box = await host.boundingBox();
  if (box === null) {
    throw new Error("Realtime preview host has no visible bounding box");
  }

  const clip = {
    x: Math.round(box.x + box.width * clampUnit(region.x)),
    y: Math.round(box.y + box.height * clampUnit(region.y)),
    width: Math.max(1, Math.round(box.width * clampUnit(region.width))),
    height: Math.max(1, Math.round(box.height * clampUnit(region.height)))
  };

  if (process.platform === "darwin" && app !== undefined) {
    const metrics = await app.readWindowMetrics();
    if (metrics !== null) {
      return captureMacosScreenRegion(page, metrics, clip);
    }
  }

  return page.screenshot({ clip });
}

function clampUnit(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(0, Math.min(1, value));
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

function parseTimelineTargetLabel(value: string): { startUs: number; durationUs: number } | null {
  const match = value.trim().match(/^目标\s+(\d{2}:\d{2}:\d{2}\.\d{3})\s+\/\s+(\d{2}:\d{2}:\d{2}\.\d{3})$/);
  if (match === null) {
    return null;
  }
  return {
    startUs: parseTimecodeToMicroseconds(match[1] ?? ""),
    durationUs: parseTimecodeToMicroseconds(match[2] ?? "")
  };
}

function firstSrtCueText(srtContent: string): string {
  return (
    srtContent
      .split(/\r?\n/)
      .map((line) => line.trim())
      .find((line) => line.length > 0 && !/^\d+$/.test(line) && !line.includes("-->")) ?? ""
  );
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

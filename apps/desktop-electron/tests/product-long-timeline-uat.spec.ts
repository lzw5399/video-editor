import { expect, test, type ElectronApplication, type Page } from "@playwright/test";

import { generatePhase20LongTimelineFixture, type Phase20LongTimelineFixtures } from "./helpers/longTimelineFixture";
import {
  collectPhase20FailureEvidence,
  expectPhase20PreviewProductionEvidence,
  writePhase20EvidenceSummary
} from "./helpers/longTimelineEvidence";
import { launchPackagedApp, type PackagedAppLaunch } from "./helpers/packagedApp";
import {
  activateProductJourneyApp,
  captureVisiblePreviewEvidence,
  clickPreviewPlay,
  expectProductWorkspace,
  openProjectFromProductEntry,
  readProjectSessionCalls,
  readNativeCommandObservations,
  readRealtimePreviewHostCalls,
  redoTimelineEdit,
  requestProjectSessionPreviewFrameCount,
  seekTimelinePlayhead,
  splitSelectedSegment,
  undoTimelineEdit,
  waitForCompositedPreviewEvidence,
  waitForProductPlaybackSuccess,
  type ProductJourneyAppController
} from "./helpers/userJourney";

test.describe.configure({ timeout: 180_000 });

const PHASE20_PACKAGED_ENV: NodeJS.ProcessEnv = {
  VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
  VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS: "0",
  VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS: "0",
  VIDEO_EDITOR_TEST_MOCK_ARTIFACT_COMMANDS: "0",
  VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "0",
  VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "0"
};

const RESPONSIVE_FEEDBACK_BUDGET_MS = 1_500;
const EDIT_OPERATION_BUDGET_MS = 2_000;
const INSPECTOR_EDIT_BUDGET_MS = 2_500;
const FIRST_VIDEO_SEGMENT_LABEL = "video material 000000";
const SPLIT_VIDEO_SEGMENT_LABEL = "video material 000002";
const FIRST_AUDIO_MATERIAL_LABEL = "audio material 000000";

type Phase20PackagedLaunch = {
  app: ProductJourneyAppController;
  page: Page;
  executablePath: string;
  rawApp: ElectronApplication;
};

test("Phase 20 packaged responsiveness UAT @phase20 @responsiveness", async () => {
  const fixtures = await generatePhase20LongTimelineFixture();
  const launched = await launchPhase20PackagedProject(fixtures);

  try {
    const metrics = await runResponsivenessWorkflow(launched.page, launched.app, fixtures);
    await writePhase20EvidenceSummary({
      evidenceDir: fixtures.evidenceDir,
      status: "passed",
      workflow: "phase20-packaged-long-session",
      stage: "responsiveness",
      productSummary: {
        message: "Packaged long timeline stayed responsive during selection, scroll/zoom, scrub/play, edit, undo, redo, and inspector visual changes.",
        budgets: phase20Budgets(),
        metrics
      },
      developerDetails: {
        executablePath: launched.executablePath,
        nativeCommandObservations: await readNativeCommandObservations(launched.app),
        realtimePreviewHostCalls: await readRealtimePreviewHostCalls(launched.app)
      }
    });
  } catch (error) {
    await collectPhase20FailureEvidence({
      fixtures,
      workflow: "phase20-packaged-long-session",
      stage: "responsiveness",
      error,
      page: launched.page,
      app: launched.app
    });
    throw error;
  } finally {
    await launched.app.close();
  }
});

async function runResponsivenessWorkflow(
  page: Page,
  app: ProductJourneyAppController,
  fixtures: Phase20LongTimelineFixtures
): Promise<Record<string, number | string>> {
  await expectLongProjectVisible(page);
  const metrics: Record<string, number | string> = {};

  metrics.zoomMs = await expectWithinBudget("zoom visible feedback", RESPONSIVE_FEEDBACK_BUDGET_MS, async () => {
    await zoomTimelineTo(page, 200);
  });

  metrics.selectionMs = await expectWithinBudget("selection", EDIT_OPERATION_BUDGET_MS, async () => {
    await selectLongVideoSegment(page, FIRST_VIDEO_SEGMENT_LABEL);
  });

  metrics.scrollMs = await expectWithinBudget("scroll visible feedback", RESPONSIVE_FEEDBACK_BUDGET_MS, async () => {
    await scrollTimelineHorizontally(page);
  });

  const scrubBefore = await captureVisiblePreviewEvidence(page, app);
  const frameRequestsBeforeScrub = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
  metrics.scrubMs = await expectWithinBudget("scrub visible feedback", RESPONSIVE_FEEDBACK_BUDGET_MS, async () => {
    await seekTimelinePlayhead(page, app, 30_000_000);
  });
  await waitForCompositedPreviewEvidence(page, app, 20_000, scrubBefore.hostState?.contentEvidence?.targetTimeMicroseconds ?? -1);
  const scrubAfter = await captureVisiblePreviewEvidence(page, app);
  expectPhase20PreviewProductionEvidence({
    before: scrubBefore,
    after: scrubAfter,
    frameRequestsBefore: frameRequestsBeforeScrub,
    frameRequestsAfter: requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app))
  });

  const playBefore = await captureVisiblePreviewEvidence(page, app);
  const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
  await activateProductJourneyApp(app, page);
  metrics.playClickMs = await expectWithinBudget("play visible feedback", RESPONSIVE_FEEDBACK_BUDGET_MS, async () => {
    await clickPreviewPlay(page);
  });
  const playbackEvidence = await waitForProductPlaybackSuccess(page, app, playBefore, playBefore, frameRequestsBeforePlay);
  expectPhase20PreviewProductionEvidence({
    before: playBefore,
    after: {
      ...playbackEvidence.after,
      visibleCenterHash: playbackEvidence.visibleMotion.visibleCenterHash
    },
    frameRequestsBefore: frameRequestsBeforePlay,
    frameRequestsAfter: requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app))
  });

  await selectLongVideoSegment(page, FIRST_VIDEO_SEGMENT_LABEL);
  metrics.trimMs = await expectWithinBudget("trim", EDIT_OPERATION_BUDGET_MS, async () => {
    await trimSelectedSegmentRightEdgeLeftForLongTimeline(page, app);
  });

  metrics.moveMs = await expectWithinBudget("move", EDIT_OPERATION_BUDGET_MS, async () => {
    await moveSelectedSegmentRightForLongTimeline(page, app);
  });

  await selectLongVideoSegment(page, SPLIT_VIDEO_SEGMENT_LABEL);
  metrics.splitMs = await expectWithinBudget("split", EDIT_OPERATION_BUDGET_MS, async () => {
    await splitSelectedSegment(page, app, 2_500_000);
  });

  metrics.undoMs = await expectWithinBudget("undo", EDIT_OPERATION_BUDGET_MS, async () => {
    await undoTimelineEdit(page, app);
  });

  metrics.redoMs = await expectWithinBudget("redo", EDIT_OPERATION_BUDGET_MS, async () => {
    await redoTimelineEdit(page, app);
  });

  await selectLongVideoSegment(page, SPLIT_VIDEO_SEGMENT_LABEL);
  const editPreviewBefore = await captureVisiblePreviewEvidence(page, app);
  const frameRequestsBeforeEdit = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
  metrics.inspectorEditMs = await expectWithinBudget("inspector visual edit", INSPECTOR_EDIT_BUDGET_MS, async () => {
    await editSelectedVisualPositionXThroughInspector(page, app, 92);
  });
  await waitForCompositedPreviewEvidence(page, app, 20_000, editPreviewBefore.hostState?.contentEvidence?.targetTimeMicroseconds ?? -1);
  const editPreviewAfter = await captureVisiblePreviewEvidence(page, app);
  expectPhase20PreviewProductionEvidence({
    before: editPreviewBefore,
    after: editPreviewAfter,
    frameRequestsBefore: frameRequestsBeforeEdit,
    frameRequestsAfter: requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app))
  });

  await expectNoFallbackEvidence(app);
  return metrics;
}

async function launchPhase20PackagedProject(
  fixtures: Phase20LongTimelineFixtures
): Promise<Phase20PackagedLaunch> {
  const launch = await launchPackagedApp({
    ...PHASE20_PACKAGED_ENV,
    VIDEO_EDITOR_TEST_PICK_OPEN_PROJECT_BUNDLE: fixtures.bundlePath,
    VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([])
  });
  const app = wrapPackagedApp(launch);
  await activateProductJourneyApp(app, launch.page);
  await openProjectFromProductEntry(app, launch.page);
  await expectProductWorkspace(launch.page);
  return {
    ...launch,
    app,
    rawApp: launch.app
  };
}

function wrapPackagedApp(launch: PackagedAppLaunch): ProductJourneyAppController {
  return {
    kind: "electron-launch",
    close: () => launch.app.close(),
    readForegroundDiagnostics: async () => null,
    readNativeCommandObservations: () => readObservationApi(launch.page, "getNativeCommandObservations"),
    readProjectSessionCalls: () => readObservationApi(launch.page, "getProjectSessionCalls"),
    readRealtimePreviewHostCalls: () => readObservationApi(launch.page, "getRealtimePreviewHostCalls"),
    readWindowMetrics: () => readObservationApi(launch.page, "getWindowMetrics"),
    maximizeMainWindow: () => readObservationApi(launch.page, "maximizeMainWindow"),
    moveMainWindow: (x, y) => readObservationApi(launch.page, "moveMainWindow", x, y),
    resizeMainWindow: (width, height) => readObservationApi(launch.page, "resizeMainWindow", width, height)
  };
}

async function readObservationApi<T>(page: Page, method: keyof VideoEditorTestObservations, ...args: unknown[]): Promise<T> {
  return page.evaluate(
    async ({ method: methodName, args: methodArgs }) => {
      const api = (window as typeof window & { videoEditorTestObservations?: VideoEditorTestObservations }).videoEditorTestObservations;
      if (api === undefined) {
        throw new Error("Packaged Phase 20 UAT requires videoEditorTestObservations from preload");
      }
      return api[methodName](...methodArgs);
    },
    { method, args }
  ) as Promise<T>;
}

type VideoEditorTestObservations = {
  getNativeCommandObservations: (...args: unknown[]) => Promise<unknown>;
  getProjectSessionCalls: (...args: unknown[]) => Promise<unknown>;
  getRealtimePreviewHostCalls: (...args: unknown[]) => Promise<unknown>;
  getWindowMetrics: (...args: unknown[]) => Promise<unknown>;
  maximizeMainWindow: (...args: unknown[]) => Promise<unknown>;
  moveMainWindow: (...args: unknown[]) => Promise<unknown>;
  resizeMainWindow: (...args: unknown[]) => Promise<unknown>;
};

async function expectLongProjectVisible(page: Page): Promise<void> {
  await expect(page.getByRole("article", { name: `素材 ${FIRST_VIDEO_SEGMENT_LABEL}` }).first()).toBeVisible({ timeout: 30_000 });
  await expect(page.getByRole("article", { name: `素材 ${FIRST_AUDIO_MATERIAL_LABEL}` }).first()).toBeVisible({ timeout: 30_000 });
  await expect(page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(FIRST_VIDEO_SEGMENT_LABEL)}`) }).first()).toBeVisible({
    timeout: 30_000
  });
}

async function selectLongVideoSegment(page: Page, label: string): Promise<void> {
  await page.locator(".track-list").evaluate((element) => {
    element.scrollLeft = 0;
  });
  const segment = page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(label)}`) }).first();
  await expect(segment).toBeVisible({ timeout: 10_000 });
  const box = await segment.boundingBox();
  if (box === null) {
    throw new Error("First long video segment is not measurable for selection");
  }
  await page.mouse.click(box.x + Math.max(1, Math.min(box.width - 1, box.width / 2)), box.y + box.height / 2);
  await expect(page.getByLabel("预览选中框")).toBeVisible({ timeout: 10_000 });
}

async function scrollTimelineHorizontally(page: Page): Promise<void> {
  const scroll = await page.locator(".track-list").evaluate((element) => {
    const before = element.scrollLeft;
    element.scrollLeft = Math.min(element.scrollWidth - element.clientWidth, before + 320);
    return {
      before,
      after: element.scrollLeft,
      max: element.scrollWidth - element.clientWidth
    };
  });
  expect(scroll.max, "long timeline must expose horizontal scroll range").toBeGreaterThan(0);
  expect(scroll.after, "timeline scroll visible feedback must update scrollLeft").toBeGreaterThan(scroll.before);
}

async function zoomTimelineTo(page: Page, targetPercent: number): Promise<void> {
  const content = page.locator(".track-scroll-content");
  const widthBefore = await content.evaluate((element) => element.getBoundingClientRect().width);
  const zoomShell = page.getByLabel("时间线缩放", { exact: true });
  const zoomIn = page.getByRole("button", { name: "放大时间线" });
  while (!((await zoomShell.textContent()) ?? "").includes(`${targetPercent}%`)) {
    await expect(zoomIn).toBeEnabled();
    await zoomIn.click();
  }
  await expect(zoomShell).toContainText(`${targetPercent}%`);
  await expect
    .poll(async () => content.evaluate((element) => element.getBoundingClientRect().width))
    .toBeGreaterThan(widthBefore);
}

async function trimSelectedSegmentRightEdgeLeftForLongTimeline(
  page: Page,
  app: ProductJourneyAppController
): Promise<void> {
  const nextCount = (await timelineMoveTrimCommitCount(app)) + 1;
  const handle = page.locator(".segment-block.selected .segment-trim-handle.right").first();
  const handleBox = await handle.boundingBox();
  if (handleBox === null) {
    throw new Error("Selected long timeline segment right trim handle is not visible");
  }
  const startX = handleBox.x + handleBox.width / 2;
  const startY = handleBox.y + handleBox.height / 2;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX - 18, startY, { steps: 4 });
  await page.mouse.up();
  await waitForTimelineMoveTrimCommitCount(app, nextCount);
}

async function moveSelectedSegmentRightForLongTimeline(
  page: Page,
  app: ProductJourneyAppController
): Promise<void> {
  const nextCount = (await timelineMoveTrimCommitCount(app)) + 1;
  const segment = page.locator(".segment-block.selected").first();
  const segmentBox = await segment.boundingBox();
  if (segmentBox === null) {
    throw new Error("Selected long timeline segment is not visible for move");
  }
  const startX = segmentBox.x + segmentBox.width / 2;
  const startY = segmentBox.y + segmentBox.height / 2;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX + 8, startY, { steps: 4 });
  await page.mouse.up();
  await waitForTimelineMoveTrimCommitCount(app, nextCount);
}

async function editSelectedVisualPositionXThroughInspector(
  page: Page,
  app: ProductJourneyAppController,
  positionX: number
): Promise<void> {
  const beforeCount = await visualEditObservationCount(app);
  const visualTab = page.getByRole("tab", { name: "画面" });
  if ((await visualTab.count()) > 0) {
    await visualTab.click();
  }
  const visualForm = page.getByLabel("画面基础表单");
  const positionInput = visualForm.getByLabel("位置 X", { exact: true });
  await expect(positionInput).toBeVisible();
  await positionInput.fill(String(positionX));
  await positionInput.blur();
  await expect.poll(async () => visualEditObservationCount(app), { timeout: 30_000 }).toBeGreaterThan(beforeCount);
}

async function waitForTimelineMoveTrimCommitCount(
  app: ProductJourneyAppController,
  expectedCount: number
): Promise<void> {
  await expect
    .poll(async () => timelineMoveTrimCommitCount(app), { timeout: 30_000 })
    .toBeGreaterThanOrEqual(expectedCount);
}

async function timelineMoveTrimCommitCount(app: ProductJourneyAppController): Promise<number> {
  return (await readProjectSessionCalls(app)).filter(
    (call) => call.command === "commitProjectInteraction" && call.interactionKind === "timelineMoveTrim" && call.resultOk === true
  ).length;
}

async function visualEditObservationCount(app: ProductJourneyAppController): Promise<number> {
  return (await readNativeCommandObservations(app)).filter(
    (call) =>
      call.command === "updateSelectedSegmentVisual" ||
      (call.command === "commitProjectInteraction" && call.interactionKind === "selectedSegmentVisual" && call.resultOk === true)
  ).length;
}

async function expectWithinBudget(label: string, budgetMs: number, action: () => Promise<void>): Promise<number> {
  const startedAt = Date.now();
  await action();
  const durationMs = Date.now() - startedAt;
  expect(durationMs, `${label} must complete within ${budgetMs}ms`).toBeLessThanOrEqual(budgetMs);
  return durationMs;
}

async function expectNoFallbackEvidence(app: ProductJourneyAppController): Promise<void> {
  const hostCalls = await readRealtimePreviewHostCalls(app);
  expect(hostCalls.map((call) => call.kind), "Phase 20 UAT must not accept missing-compositor fallback").not.toContain(
    "playRejectedMissingCompositor"
  );
}

function phase20Budgets(): Record<string, number> {
  return {
    responsiveFeedbackMs: RESPONSIVE_FEEDBACK_BUDGET_MS,
    editOperationMs: EDIT_OPERATION_BUDGET_MS,
    inspectorEditMs: INSPECTOR_EDIT_BUDGET_MS
  };
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

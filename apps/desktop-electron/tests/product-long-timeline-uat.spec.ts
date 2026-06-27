import { expect, test, type ElectronApplication, type Locator, type Page } from "@playwright/test";

import { generatePhase20LongTimelineFixture, type Phase20LongTimelineFixtures } from "./helpers/longTimelineFixture";
import {
  collectPhase20FailureEvidence,
  expectCanonicalDraftStable,
  expectNoDerivedArtifactPollution,
  expectPhase20ExportMedia,
  expectPhase20PreviewProductionEvidence,
  readCanonicalDraftSummary,
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
  readTaskRuntimeTelemetry,
  redoTimelineEdit,
  requestProjectSessionPreviewFrameCount,
  seekTimelinePlayhead,
  splitSelectedSegment,
  undoTimelineEdit,
  waitForCompositedPreviewEvidence,
  waitForProductPlaybackSuccess,
  waitForVisiblePreviewCenterChange,
  type ProductJourneyAppController
} from "./helpers/userJourney";

test.describe.configure({ timeout: 420_000 });

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

test("Phase 20 packaged canonical reopen and export UAT @phase20 @canonical @export", async () => {
  const fixtures = await generatePhase20LongTimelineFixture();
  let launched: Phase20PackagedLaunch | null = null;

  try {
    const result = await runCanonicalReopenExportWorkflow(fixtures);
    await writePhase20EvidenceSummary({
      evidenceDir: fixtures.evidenceDir,
      status: "passed",
      workflow: "phase20-packaged-long-session",
      stage: "canonical-export",
      productSummary: {
        message: "Packaged long timeline preserved canonical draft facts through two reopen cycles and two validated exports.",
        reopenCycles: 2,
        exportValidations: 2,
        canonicalFacts: result.canonicalFacts,
        exportPaths: fixtures.exportPaths
      },
      developerDetails: {
        firstExport: result.firstExport,
        secondExport: result.secondExport,
        nativeCommandObservationCounts: result.nativeCommandObservationCounts,
        projectSessionObservationCounts: result.projectSessionObservationCounts
      }
    });
  } catch (error) {
    if (launched !== null) {
      await collectPhase20FailureEvidence({
        fixtures,
        workflow: "phase20-packaged-long-session",
        stage: "canonical-export",
        error,
        page: launched.page,
        app: launched.app,
        exportPaths: [...fixtures.exportPaths]
      });
    } else {
      await collectPhase20FailureEvidence({
        fixtures,
        workflow: "phase20-packaged-long-session",
        stage: "canonical-export",
        error,
        exportPaths: [...fixtures.exportPaths]
      });
    }
    throw error;
  } finally {
    await launched?.app.close().catch(() => undefined);
  }

  async function launch(): Promise<Phase20PackagedLaunch> {
    await launched?.app.close().catch(() => undefined);
    launched = await launchPhase20PackagedProject(fixtures);
    return launched;
  }

  async function runCanonicalReopenExportWorkflow(
    workflowFixtures: Phase20LongTimelineFixtures
  ): Promise<{
    canonicalFacts: Record<string, number | string>;
    firstExport: Awaited<ReturnType<typeof expectPhase20ExportMedia>>;
    secondExport: Awaited<ReturnType<typeof expectPhase20ExportMedia>>;
    nativeCommandObservationCounts: Record<string, number>;
    projectSessionObservationCounts: Record<string, number>;
  }> {
    const initial = await launch();
    await expectLongProjectVisible(initial.page);
    await selectLongVideoSegment(initial.page, FIRST_VIDEO_SEGMENT_LABEL);
    await editSelectedVisualPositionXThroughInspector(initial.page, initial.app, 64);
    await expectNoDerivedArtifactPollution(workflowFixtures.bundlePath);
    const firstSaved = await readCanonicalDraftSummary(workflowFixtures.bundlePath);

    const firstReopen = await launch();
    await expectLongProjectVisible(firstReopen.page);
    await expectNoDerivedArtifactPollution(workflowFixtures.bundlePath);
    const firstReopened = await readCanonicalDraftSummary(workflowFixtures.bundlePath);
    expectCanonicalDraftStable(firstSaved, firstReopened, "Phase 20 first reopen must preserve canonical draft facts");
    const firstExport = await exportAndValidatePhase20Media(firstReopen.page, firstReopen.app, workflowFixtures.firstExportPath, workflowFixtures);

    await selectLongVideoSegment(firstReopen.page, SPLIT_VIDEO_SEGMENT_LABEL);
    await editSelectedVisualPositionXThroughInspector(firstReopen.page, firstReopen.app, 118);
    await expectNoDerivedArtifactPollution(workflowFixtures.bundlePath);
    const secondSaved = await readCanonicalDraftSummary(workflowFixtures.bundlePath);
    expect(secondSaved, "continued edit after first reopen must change canonical facts before second export").not.toEqual(firstSaved);

    const secondReopen = await launch();
    await expectLongProjectVisible(secondReopen.page);
    await expectNoDerivedArtifactPollution(workflowFixtures.bundlePath);
    const secondReopened = await readCanonicalDraftSummary(workflowFixtures.bundlePath);
    expectCanonicalDraftStable(secondSaved, secondReopened, "Phase 20 second reopen must preserve continued edit facts");
    const secondExport = await exportAndValidatePhase20Media(
      secondReopen.page,
      secondReopen.app,
      workflowFixtures.secondExportPath,
      workflowFixtures
    );

    const nativeCommandObservations = await readNativeCommandObservations(secondReopen.app);
    const projectSessionObservations = await readProjectSessionCalls(secondReopen.app);
    return {
      canonicalFacts: {
        materialCount: secondReopened.materialCount,
        trackCount: secondReopened.trackCount,
        segmentCount: secondReopened.segmentCount,
        firstRevision: firstReopened.revision ?? "none",
        secondRevision: secondReopened.revision ?? "none"
      },
      firstExport,
      secondExport,
      nativeCommandObservationCounts: countCommands(nativeCommandObservations.map((call) => call.command)),
      projectSessionObservationCounts: countCommands(projectSessionObservations.map((call) => call.command))
    };
  }
});

test("Phase 20 packaged scheduler pressure UAT @phase20 @pressure", async () => {
  const fixtures = await generatePhase20LongTimelineFixture();
  const launched = await launchPhase20PackagedProject(fixtures);

  try {
    const result = await runSchedulerPressureWorkflow(launched.page, launched.app, fixtures);
    await writePhase20EvidenceSummary({
      evidenceDir: fixtures.evidenceDir,
      status: "passed",
      workflow: "phase20-packaged-long-session",
      stage: "pressure",
      productSummary: {
        message: "Packaged long timeline stayed responsive under export, playback, preview, and interaction pressure.",
        budgets: phase20Budgets(),
        metrics: result.metrics,
        scheduler: result.scheduler
      },
      developerDetails: {
        nativeCommandObservations: result.nativeCommandObservations,
        projectSessionCalls: result.projectSessionCalls,
        realtimePreviewHostCalls: result.realtimePreviewHostCalls,
        previewEvidence: result.previewEvidence
      }
    });
  } catch (error) {
    await collectPhase20FailureEvidence({
      fixtures,
      workflow: "phase20-packaged-long-session",
      stage: "pressure",
      error,
      page: launched.page,
      app: launched.app,
      exportPaths: [fixtures.firstExportPath]
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

async function runSchedulerPressureWorkflow(
  page: Page,
  app: ProductJourneyAppController,
  fixtures: Phase20LongTimelineFixtures
): Promise<{
  metrics: Record<string, number | string | boolean>;
  scheduler: Awaited<ReturnType<typeof readTaskRuntimeTelemetry>>;
  nativeCommandObservations: Awaited<ReturnType<typeof readNativeCommandObservations>>;
  projectSessionCalls: Awaited<ReturnType<typeof readProjectSessionCalls>>;
  realtimePreviewHostCalls: Awaited<ReturnType<typeof readRealtimePreviewHostCalls>>;
  previewEvidence: Record<string, unknown>;
}> {
  await expectLongProjectVisible(page);
  await zoomTimelineTo(page, 200);
  await selectLongVideoSegment(page, FIRST_VIDEO_SEGMENT_LABEL);
  await seekTimelinePlayhead(page, app, 0);

  const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
  const previewBeforePlay = await waitForCompositedPreviewEvidence(page, app, 20_000, -1);
  const visibleBeforePlay = await captureVisiblePreviewEvidence(page, app);
  const telemetryBeforePressure = await readTaskRuntimeTelemetry(page);

  await activateProductJourneyApp(app, page);
  await clickPreviewPlay(page);
  const playbackEvidence = await waitForProductPlaybackSuccess(
    page,
    app,
    previewBeforePlay,
    visibleBeforePlay,
    frameRequestsBeforePlay,
    20_000
  );

  await startPhase20ExportPressureThroughProductUi(page, app, fixtures.firstExportPath);
  const telemetryAfterPressure = await waitForSchedulerTelemetryProgress(page, telemetryBeforePressure);

  const metrics: Record<string, number | string | boolean> = {};
  const scrubBefore = await captureVisiblePreviewEvidence(page, app);
  const frameRequestsBeforeScrub = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
  metrics.scrubMs = await expectWithinBudget("pressure scrub visible feedback", RESPONSIVE_FEEDBACK_BUDGET_MS, async () => {
    await seekTimelinePlayhead(page, app, 75_000_000);
  });
  await waitForVisiblePreviewCenterChange(page, app, scrubBefore.visibleCenterHash, 8_000);
  await waitForCompositedPreviewEvidence(page, app, 20_000, scrubBefore.hostState?.contentEvidence?.targetTimeMicroseconds ?? -1);
  const scrubAfter = await captureVisiblePreviewEvidence(page, app);
  expectPhase20PreviewProductionEvidence({
    before: scrubBefore,
    after: scrubAfter,
    frameRequestsBefore: frameRequestsBeforeScrub,
    frameRequestsAfter: requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app))
  });

  await selectLongVideoSegment(page, SPLIT_VIDEO_SEGMENT_LABEL);
  metrics.inspectorEditMs = await expectWithinBudget("pressure inspector visual edit", INSPECTOR_EDIT_BUDGET_MS, async () => {
    await editSelectedVisualPositionXThroughInspector(page, app, 156);
  });

  metrics.commitMs = await expectWithinBudget("pressure interaction commit", EDIT_OPERATION_BUDGET_MS, async () => {
    await moveSelectedSegmentRightForLongTimeline(page, app);
  });

  metrics.cancelMs = await expectWithinBudget("pressure interaction cancel", EDIT_OPERATION_BUDGET_MS, async () => {
    await cancelSelectedSegmentMoveForLongTimeline(page, app);
  });

  const telemetryAfterInteractions = await readTaskRuntimeTelemetry(page);
  const nativeCommandObservations = await readNativeCommandObservations(app);
  const projectSessionCalls = await readProjectSessionCalls(app);
  const realtimePreviewHostCalls = await readRealtimePreviewHostCalls(app);
  const hostKinds = realtimePreviewHostCalls.map((call) => call.kind);
  const timelineMoveTrimCommits = projectSessionCalls.filter(
    (call) => call.command === "commitProjectInteraction" && call.interactionKind === "timelineMoveTrim" && call.resultOk === true
  ).length;
  const timelineMoveTrimCancels = projectSessionCalls.filter(
    (call) => call.command === "cancelProjectInteraction" && call.interactionKind === "timelineMoveTrim" && call.resultOk === true
  ).length;

  expect(timelineMoveTrimCommits, "pressure workflow must record a visible UI commitProjectInteraction").toBeGreaterThan(0);
  expect(timelineMoveTrimCancels, "pressure workflow must record a visible UI cancelProjectInteraction").toBeGreaterThan(0);
  expect(nativeCommandObservations.some((call) => call.command === "startExport"), "pressure workflow must start export from product UI").toBe(
    true
  );
  expect(
    nativeCommandObservations.some((call) => call.command === "getTaskRuntimeTelemetry"),
    "pressure workflow must read scheduler telemetry through the product-safe API"
  ).toBe(true);
  expect(telemetryAfterInteractions.status, "scheduler telemetry must stay product-ready").toBe("ready");
  expect(telemetryAfterInteractions.submittedCount, "scheduler must record submitted pressure work").toBeGreaterThan(
    telemetryBeforePressure.submittedCount
  );
  expect(telemetryAfterInteractions.queueLatencyUs.sampleCount, "queueLatencyUs must include scheduler samples").toBeGreaterThan(0);
  expect(
    telemetryAfterInteractions.queueLatencyUs.p95 ?? 0,
    "queueLatencyUs.p95 must stay bounded under pressure"
  ).toBeLessThanOrEqual(2_000_000);
  expect(telemetryAfterInteractions.rejectedCount, "normal product work must not be rejected under pressure").toBe(0);
  expect(telemetryAfterInteractions.fallbackCount, "pressure success must not use scheduler fallback").toBe(0);
  expect(telemetryAfterInteractions.staleRejectedCount, "scheduler must expose stale-generation rejection telemetry").toBeGreaterThanOrEqual(
    0
  );
  expect(hostKinds, "host calls must not reject playback because the compositor is missing").not.toContain("playRejectedMissingCompositor");
  expectNoStaleGenerationPresentation(realtimePreviewHostCalls);

  metrics.renderGraphGpuComposited = scrubAfter.hostState?.contentEvidence?.source === "renderGraphGpuComposited";
  metrics.queueLatencyP95Us = telemetryAfterInteractions.queueLatencyUs.p95 ?? 0;
  metrics.rejectedCount = telemetryAfterInteractions.rejectedCount;
  metrics.fallbackCount = telemetryAfterInteractions.fallbackCount;
  metrics.staleRejectedCount = telemetryAfterInteractions.staleRejectedCount;
  metrics.submittedDelta = telemetryAfterInteractions.submittedCount - telemetryBeforePressure.submittedCount;
  metrics.pressureSubmittedDelta = telemetryAfterPressure.submittedCount - telemetryBeforePressure.submittedCount;

  return {
    metrics,
    scheduler: telemetryAfterInteractions,
    nativeCommandObservations,
    projectSessionCalls,
    realtimePreviewHostCalls,
    previewEvidence: {
      beforePlay: previewBeforePlay,
      playbackAfter: playbackEvidence.after,
      playbackVisibleMotion: playbackEvidence.visibleMotion,
      scrubBefore,
      scrubAfter
    }
  };
}

async function exportAndValidatePhase20Media(
  page: Page,
  app: ProductJourneyAppController,
  outputPath: string,
  fixtures: Phase20LongTimelineFixtures
): Promise<Awaited<ReturnType<typeof expectPhase20ExportMedia>>> {
  await exportPhase20MediaThroughProductUi(page, app, outputPath);
  return expectPhase20ExportMedia(page, {
    outputPath,
    expectedWidth: fixtures.expectedWidth,
    expectedHeight: fixtures.expectedHeight,
    expectedFrameRate: fixtures.expectedFrameRate,
    expectedDurationSeconds: fixtures.expectedDurationSeconds,
    expectedDurationToleranceSeconds: 1.0,
    sampleTimesSeconds: [0.5, fixtures.expectedDurationSeconds / 2, fixtures.expectedDurationSeconds - 0.5],
    editPointSeconds: [1, 2.5, 30],
    minDistinctSampleHashes: 2,
    evidenceDir: fixtures.evidenceDir
  });
}

async function exportPhase20MediaThroughProductUi(
  page: Page,
  app: ProductJourneyAppController,
  outputPath: string
): Promise<void> {
  const nextStartCount = countNativeCommand(await readNativeCommandObservations(app), "startExport") + 1;
  await page.getByLabel("产品操作").getByRole("button", { name: "导出", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "导出" });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("输出路径").fill(outputPath);
  await expect(dialog.getByRole("button", { name: "开始导出" })).toBeEnabled({ timeout: 20_000 });
  await dialog.getByRole("button", { name: "开始导出" }).click();
  await expect
    .poll(async () => countNativeCommand(await readNativeCommandObservations(app), "startExport"), { timeout: 30_000 })
    .toBeGreaterThanOrEqual(nextStartCount);
  await waitForPhase20ExportCompletion(page, app, dialog);
  await dialog.getByRole("button", { name: "关闭" }).click();
  await expect(dialog).toHaveCount(0);
}

async function startPhase20ExportPressureThroughProductUi(
  page: Page,
  app: ProductJourneyAppController,
  outputPath: string
): Promise<void> {
  const nextStartCount = countNativeCommand(await readNativeCommandObservations(app), "startExport") + 1;
  await page.getByLabel("产品操作").getByRole("button", { name: "导出", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "导出" });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("输出路径").fill(outputPath);
  await expect(dialog.getByRole("button", { name: "开始导出" })).toBeEnabled({ timeout: 20_000 });
  await dialog.getByRole("button", { name: "开始导出" }).click();
  await expect
    .poll(async () => countNativeCommand(await readNativeCommandObservations(app), "startExport"), { timeout: 30_000 })
    .toBeGreaterThanOrEqual(nextStartCount);
  await expect(dialog.getByLabel("导出进度")).toContainText(/排队中|导出中|校验中|已完成/, { timeout: 20_000 });
  await dialog.getByRole("button", { name: "关闭" }).click();
  await expect(dialog).toHaveCount(0);
}

async function waitForSchedulerTelemetryProgress(
  page: Page,
  before: Awaited<ReturnType<typeof readTaskRuntimeTelemetry>>
): Promise<Awaited<ReturnType<typeof readTaskRuntimeTelemetry>>> {
  let latest = before;
  await expect
    .poll(
      async () => {
        latest = await readTaskRuntimeTelemetry(page);
        return latest.submittedCount;
      },
      { timeout: 20_000 }
    )
    .toBeGreaterThan(before.submittedCount);
  return latest;
}

async function waitForPhase20ExportCompletion(
  page: Page,
  app: ProductJourneyAppController,
  dialog: Locator
): Promise<void> {
  const statusButton = dialog.getByRole("button", { name: "查询导出状态" });
  const deadline = Date.now() + 180_000;

  while (Date.now() < deadline) {
    const progressText = (await dialog.getByLabel("导出进度").textContent()) ?? "";
    const logText = (await dialog.getByLabel("导出状态", { exact: true }).textContent()) ?? "";
    const validationText = (await dialog.getByLabel("输出校验").textContent()) ?? "";
    if (progressText.includes("失败") || logText.includes("失败") || validationText.includes("失败")) {
      throw new Error(
        [
          `Phase 20 export failed: ${progressText}`,
          `Export log: ${logText}`,
          `Export validation: ${validationText}`,
          `Recorded commands: ${JSON.stringify(await readNativeCommandObservations(app))}`
        ].join("\n")
      );
    }
    if (progressText.includes("已完成")) {
      await expect(dialog.getByLabel("导出进度")).toContainText("已完成", { timeout: 5_000 });
      return;
    }

    if (await statusButton.isEnabled().catch(() => false)) {
      const nextStatusCount = countNativeCommand(await readNativeCommandObservations(app), "getExportJobStatus") + 1;
      await statusButton.click();
      await expect
        .poll(async () => countNativeCommand(await readNativeCommandObservations(app), "getExportJobStatus"), { timeout: 20_000 })
        .toBeGreaterThanOrEqual(nextStatusCount);
    }

    await page.waitForTimeout(750);
  }

  const progressText = (await dialog.getByLabel("导出进度").textContent()) ?? "";
  const logText = (await dialog.getByLabel("导出状态", { exact: true }).textContent()) ?? "";
  const validationText = (await dialog.getByLabel("输出校验").textContent()) ?? "";
  throw new Error(
    [
      `Phase 20 export did not complete before timeout: ${progressText}`,
      `Export log: ${logText}`,
      `Export validation: ${validationText}`,
      `Recorded commands: ${JSON.stringify(await readNativeCommandObservations(app))}`
    ].join("\n")
  );
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

async function cancelSelectedSegmentMoveForLongTimeline(
  page: Page,
  app: ProductJourneyAppController
): Promise<void> {
  const beforeCalls = await readProjectSessionCalls(app);
  const nextCancelCount = timelineMoveTrimCancelCount(beforeCalls) + 1;
  const nextBeginCount = timelineMoveTrimBeginCount(beforeCalls) + 1;
  const segment = page.locator(".segment-block.selected").first();
  const segmentBox = await segment.boundingBox();
  if (segmentBox === null) {
    throw new Error("Selected long timeline segment is not visible for cancel interaction");
  }
  const startX = segmentBox.x + segmentBox.width / 2;
  const startY = segmentBox.y + segmentBox.height / 2;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await expect.poll(async () => timelineMoveTrimBeginCount(await readProjectSessionCalls(app)), { timeout: 10_000 }).toBeGreaterThanOrEqual(
    nextBeginCount
  );
  await page.mouse.up();
  await expect
    .poll(async () => timelineMoveTrimCancelCount(await readProjectSessionCalls(app)), { timeout: 10_000 })
    .toBeGreaterThanOrEqual(nextCancelCount);
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

function timelineMoveTrimBeginCount(calls: Awaited<ReturnType<typeof readProjectSessionCalls>>): number {
  return calls.filter((call) => call.command === "beginProjectInteraction" && call.interactionKind === "timelineMoveTrim").length;
}

function timelineMoveTrimCancelCount(calls: Awaited<ReturnType<typeof readProjectSessionCalls>>): number {
  return calls.filter(
    (call) => call.command === "cancelProjectInteraction" && call.interactionKind === "timelineMoveTrim" && call.resultOk === true
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

function countNativeCommand(calls: Awaited<ReturnType<typeof readNativeCommandObservations>>, command: string): number {
  return calls.filter((call) => call.command === command).length;
}

function countCommands(commands: string[]): Record<string, number> {
  return commands.reduce<Record<string, number>>((counts, command) => {
    counts[command] = (counts[command] ?? 0) + 1;
    return counts;
  }, {});
}

function expectNoStaleGenerationPresentation(calls: Awaited<ReturnType<typeof readRealtimePreviewHostCalls>>): void {
  const presentedStates = calls.filter(
    (call) =>
      call.kind === "getPresentationState" &&
      call.presentationAvailable === true &&
      call.presentationBackend === "renderGraphGpu" &&
      typeof call.playbackGeneration === "number"
  );
  expect(presentedStates.length, "pressure workflow must observe renderGraphGpu presentation states").toBeGreaterThan(0);
  expect(
    calls.filter((call) => /stale.*present/i.test(call.kind)),
    "stale realtime preview generations must be rejected rather than presented"
  ).toEqual([]);
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

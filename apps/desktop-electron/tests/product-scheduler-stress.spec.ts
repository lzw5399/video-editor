import { expect, test, type Page } from "@playwright/test";
import { unlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

import {
  USER_JOURNEY_LONG_MOVING_VIDEO,
  USER_JOURNEY_LONG_TONE_AUDIO,
  activateProductJourneyApp,
  addMaterialToTimeline,
  captureVisiblePreviewEvidence,
  importMaterialsThroughProductPicker,
  launchProductJourneyApp,
  readNativeCommandObservations,
  readProjectSessionCalls,
  readRealtimePreviewHostCalls,
  readTaskRuntimeTelemetry,
  requestProjectSessionPreviewFrameCount,
  seekTimelinePlayhead,
  updateSelectedVisualThroughInspector,
  waitForCompositedPreviewEvidence,
  waitForProductPlaybackSuccess,
  waitForVisiblePreviewCenterChange,
  type ProductJourneyAppController,
  type TaskRuntimeTelemetryResponse
} from "./helpers/userJourney";

test.describe.configure({ timeout: 120_000 });

const REAL_PRODUCT_ENV = {
  VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "0"
} as const;
const PRODUCT_SAFE_TELEMETRY_API = "getTaskRuntimeTelemetry";

test("product scheduler keeps preview and inspector responsive during export and import pressure", async () => {
  const openMaterialFiles = [USER_JOURNEY_LONG_MOVING_VIDEO, USER_JOURNEY_LONG_TONE_AUDIO];
  const outputPath = join(tmpdir(), `video-editor-scheduler-stress-${Date.now()}.mp4`);
  const { app, page } = await launchProductJourneyApp(openMaterialFiles, REAL_PRODUCT_ENV);

  try {
    await importMaterialsThroughProductPicker(app, page, openMaterialFiles);
    await addMaterialToTimeline(app, page, USER_JOURNEY_LONG_MOVING_VIDEO);
    await selectTimelineSegment(page, USER_JOURNEY_LONG_MOVING_VIDEO);
    await seekTimelinePlayhead(page, app, 0);

    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    const previewBefore = await waitForCompositedPreviewEvidence(page, app, 15_000, -1);
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const telemetryBeforePressure = await readTaskRuntimeTelemetry(page);

    await activateProductJourneyApp(app, page);
    await clickPreviewPlay(page);
    const playbackEvidence = await waitForProductPlaybackSuccess(
      page,
      app,
      previewBefore,
      visibleBefore,
      frameRequestsBeforePlay
    );

    await startExportFromTopRightModal(page, app, outputPath);
    await ensurePlaybackRunning(page);
    const importPressureBefore = countImportMaterialIntents(await readProjectSessionCalls(app));
    await triggerProductImportPressure(page, app, importPressureBefore + openMaterialFiles.length);

    const telemetryAfterPressure = await waitForSchedulerTelemetryProgress(page, telemetryBeforePressure);
    const editStartedAt = Date.now();
    await selectTimelineSegment(page, USER_JOURNEY_LONG_MOVING_VIDEO);
    await updateSelectedVisualThroughInspector(page, app, {
      positionX: 96,
      positionY: -48,
      scaleX: 1180,
      scaleY: 1180,
      rotation: 6,
      opacity: 900,
      fitMode: "填充"
    });
    const inspectorEditMs = Date.now() - editStartedAt;

    await seekTimelinePlayhead(page, app, 1_000_000);
    const visibleWhilePressure = await waitForVisiblePreviewCenterChange(
      page,
      app,
      playbackEvidence.visibleMotion.visibleCenterHash,
      8_000
    );
    const previewAfterPressure = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsAfterPressure = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    const nativeCommands = await readNativeCommandObservations(app);
    const hostCalls = await readRealtimePreviewHostCalls(app);
    const renderGraphGpuComposited = previewAfterPressure.hostState?.contentEvidence?.source === "renderGraphGpuComposited";
    const fallbackActive = previewAfterPressure.hostState?.fallbackActive ?? true;
    const visibleCenterHash = visibleWhilePressure.visibleCenterHash;
    const queueLatencyUs = telemetryAfterPressure.queueLatencyUs;
    const resourceSaturationCount = telemetryAfterPressure.resourceSaturationCount;

    const metrics = {
      renderGraphGpuComposited,
      fallbackActive,
      visibleCenterHash,
      frameRequestsBeforePlay,
      frameRequestsAfterPressure,
      targetDeltaUs:
        (previewAfterPressure.hostState?.contentEvidence?.targetTimeMicroseconds ?? previewAfterPressure.timecodeUs) -
        (previewBefore.hostState?.contentEvidence?.targetTimeMicroseconds ?? previewBefore.timecodeUs),
      inspectorEditMs,
      scheduler: {
        api: PRODUCT_SAFE_TELEMETRY_API,
        status: telemetryAfterPressure.status,
        submittedDelta: telemetryAfterPressure.submittedCount - telemetryBeforePressure.submittedCount,
        completedDelta: telemetryAfterPressure.completedCount - telemetryBeforePressure.completedCount,
        queueLatencyUs,
        resourceSaturationCount
      },
      commands: nativeCommands.map((command) => command.command)
    };
    console.log(`product scheduler stress metrics ${JSON.stringify(metrics)}`);

    expect(renderGraphGpuComposited, "stress playback must use renderGraphGpuComposited product evidence").toBe(true);
    expect(fallbackActive, "stress playback must not report fallbackActive").toBe(false);
    expect(previewAfterPressure.hostState?.backend, "stress playback backend must remain renderGraphGpu").toBe("renderGraphGpu");
    expect(previewAfterPressure.hostState?.diagnosticSource, "stress success must not come from diagnostic sources").toBe("none");
    expect(frameRequestsAfterPressure, "scheduler stress must not use requestProjectSessionPreviewFrame artifact fallback").toBe(
      frameRequestsBeforePlay
    );
    expect(visibleWhilePressure.visibleCenterHash, "visibleCenterHash must change while scheduler pressure is active").not.toBe(
      playbackEvidence.visibleMotion.visibleCenterHash
    );
    expect(metrics.targetDeltaUs, "preview target time must advance during stress playback").toBeGreaterThan(500_000);
    expect(inspectorEditMs, "inspector edit command must stay responsive under scheduler pressure").toBeLessThanOrEqual(2_500);
    expect(nativeCommands.some((command) => command.command === "startExport"), "export must start from the product modal").toBe(true);
    expect(
      nativeCommands.some((command) => command.command === PRODUCT_SAFE_TELEMETRY_API),
      "stress test must read scheduler telemetry through getTaskRuntimeTelemetry"
    ).toBe(true);
    expect(
      nativeCommands.some((command) => command.command === "updateSelectedSegmentVisual"),
      "inspector edit must send the normal product visual command"
    ).toBe(true);
    expect(countImportMaterialIntents(await readProjectSessionCalls(app))).toBeGreaterThanOrEqual(
      importPressureBefore + openMaterialFiles.length
    );
    expect(telemetryAfterPressure.status, "scheduler telemetry must stay product-ready").toBe("ready");
    expect(telemetryAfterPressure.submittedCount, "scheduler must record submitted work under pressure").toBeGreaterThan(
      telemetryBeforePressure.submittedCount
    );
    expect(queueLatencyUs.sampleCount, "queueLatencyUs must include samples").toBeGreaterThan(0);
    expect(queueLatencyUs.p95 ?? 0, "queue latency p95 should remain bounded for product stress").toBeLessThanOrEqual(2_000_000);
    expect(resourceSaturationCount, "resourceSaturationCount must be reported as a scheduler telemetry counter").toBeGreaterThanOrEqual(
      0
    );
    expect(telemetryAfterPressure.rejectedCount, "stress workflow should not reject normal product work").toBe(0);
    expect(telemetryAfterPressure.fallbackCount, "stress workflow must not use fallback scheduler success").toBe(0);
    expect(hostCalls.map((call) => call.kind), "host calls must not reject missing compositor during stress").not.toContain(
      "playRejectedMissingCompositor"
    );
  } finally {
    await unlink(outputPath).catch(() => undefined);
    await app.close();
  }
});

async function startExportFromTopRightModal(
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
    .poll(async () => countNativeCommand(await readNativeCommandObservations(app), "startExport"), { timeout: 20_000 })
    .toBeGreaterThanOrEqual(nextStartCount);
  await expect(dialog.getByLabel("导出进度")).toContainText(/排队中|导出中|校验中|已完成/, { timeout: 20_000 });
  await dialog.getByRole("button", { name: "关闭" }).click();
  await expect(dialog).toHaveCount(0);
}

async function triggerProductImportPressure(page: Page, app: ProductJourneyAppController, expectedImportCount: number): Promise<void> {
  await page.getByLabel("顶部功能区").getByRole("button", { name: "素材" }).click({ timeout: 5_000 });
  await page.getByRole("button", { name: "导入素材" }).click();
  await expect
    .poll(async () => countImportMaterialIntents(await readProjectSessionCalls(app)), { timeout: 30_000 })
    .toBeGreaterThanOrEqual(expectedImportCount);
}

async function waitForSchedulerTelemetryProgress(
  page: Page,
  before: TaskRuntimeTelemetryResponse
): Promise<TaskRuntimeTelemetryResponse> {
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

async function clickPreviewPlay(page: Page): Promise<void> {
  const controls = page.getByRole("group", { name: "预览播放控制" });
  const playButton = controls.getByRole("button", { name: "播放预览" });
  await expect(playButton).toBeEnabled({ timeout: 20_000 });
  await playButton.click();
}

async function ensurePlaybackRunning(page: Page): Promise<void> {
  const controls = page.getByRole("group", { name: "预览播放控制" });
  const playButton = controls.getByRole("button", { name: "播放预览" });
  if (await playButton.isVisible({ timeout: 500 }).catch(() => false)) {
    await playButton.click();
  }
  await expect(controls.getByRole("button", { name: "暂停预览" })).toBeEnabled({ timeout: 10_000 });
}

async function selectTimelineSegment(page: Page, materialPath: string): Promise<void> {
  const materialName = basename(materialPath);
  await page.getByRole("button", { name: new RegExp(`片段 ${escapeRegex(materialName)}`) }).click();
  await expect(page.getByLabel("预览选中框")).toBeVisible();
}

function countNativeCommand(calls: Awaited<ReturnType<typeof readNativeCommandObservations>>, command: string): number {
  return calls.filter((call) => call.command === command).length;
}

function countImportMaterialIntents(calls: Awaited<ReturnType<typeof readProjectSessionCalls>>): number {
  return calls.filter((call) => call.command === "executeProjectIntent" && call.intentKind === "importMaterial").length;
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

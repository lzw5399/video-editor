import { expect, test } from "@playwright/test";

import {
  USER_JOURNEY_MOVING_VIDEO,
  USER_JOURNEY_TONE_AUDIO,
  activateProductJourneyApp,
  addAudioThroughProductPanel,
  addMaterialToTimeline,
  addTextThroughProductPanel,
  captureVisiblePreviewEvidence,
  importMaterialsThroughProductPicker,
  importMaterialThroughProductPicker,
  importSubtitleSrtThroughProductPanel,
  launchProductJourneyApp,
  type ProductJourneyAppController,
  readNativeCommandObservations,
  readRealtimePreviewHostCalls,
  requestProjectSessionPreviewFrameCount,
  seekTimelinePlayhead
} from "./helpers/userJourney";

type HostState = {
  ok: boolean;
  productReady: boolean;
  fallbackActive: boolean;
  unsupportedReason: string | null;
  backend: "renderGraphGpu" | "none";
  telemetry: {
    firstFrameLatencyMs: number | null;
    renderDurationMs: number;
    presentedFrameCount: number;
    droppedFrameCount: number;
    targetTimeMicroseconds: number;
    playbackGeneration: number;
    framePacing: {
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
  contentEvidence: {
    source: "nativeVideoBridge" | "renderGraphGpuComposited";
    digest: string;
    targetTimeMicroseconds: number;
  } | null;
};

test.describe.configure({ timeout: 90_000 });

test("product preview cadence presents sustained GPU frames without artifact fallback", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await seekTimelinePlayhead(page, app, 0);

    const controls = page.getByRole("group", { name: "预览播放控制" });
    const playButton = controls.getByRole("button", { name: "播放预览" });
    await expect(playButton).toBeEnabled({ timeout: 20_000 });

    const frameRequestsBefore = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    const before = await readHostState(page);
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const nativeHostCallCountBeforePlay = (await readRealtimePreviewHostCalls(app)).length;

    await activateProductJourneyApp(app, page);
    await playButton.click();
    await waitForPlaybackProgress(page, before);
    const hostCallCountBefore = (await readRealtimePreviewHostCalls(app)).length;
    await page.waitForTimeout(3_000);
    const after = await readHostState(page);
    const visibleAfter = await captureVisiblePreviewEvidence(page, app);
    const presentationDurations = (await readRealtimePreviewHostCalls(app))
      .slice(hostCallCountBefore)
      .filter((call) => call.kind === "getPresentationState" && typeof call.durationMs === "number")
      .map((call) => call.durationMs as number)
      .sort((first, second) => first - second);
    const nativeEvents = (await readRealtimePreviewHostCalls(app))
      .slice(nativeHostCallCountBeforePlay)
      .filter((call) => call.kind === "nativePreviewEvent");
    const durationPercentile = (percentile: number) =>
      presentationDurations[Math.min(presentationDurations.length - 1, Math.floor(presentationDurations.length * percentile))] ??
      null;

    const presentedBefore = before?.telemetry?.presentedFrameCount ?? 0;
    const presentedAfter = after?.telemetry?.presentedFrameCount ?? 0;
    const droppedBefore = before?.telemetry?.droppedFrameCount ?? 0;
    const droppedAfter = after?.telemetry?.droppedFrameCount ?? 0;
    const targetBefore = before?.contentEvidence?.targetTimeMicroseconds ?? 0;
    const targetAfter = after?.contentEvidence?.targetTimeMicroseconds ?? 0;
    const renderGraphActive =
      after?.ok === true &&
      after.productReady &&
      !after.fallbackActive &&
      after.backend === "renderGraphGpu" &&
      after.contentEvidence?.source === "renderGraphGpuComposited";
    const evidenceDigestChanged =
      typeof after?.contentEvidence?.digest === "string" &&
      after.contentEvidence.digest.length > 0 &&
      (typeof before?.contentEvidence?.digest !== "string" ||
        before.contentEvidence.digest !== after.contentEvidence.digest);
    const framePacing = after?.telemetry?.framePacing ?? null;
    const metrics = {
      renderGraphActive,
      unsupportedReason: after?.unsupportedReason ?? null,
      lastPresentationFailure: (await readRealtimePreviewHostCalls(app))
        .filter((call) => call.kind === "getPresentationState" && typeof call.unsupportedReason === "string")
        .at(-1)?.unsupportedReason,
      presentedDelta: presentedAfter - presentedBefore,
      droppedDelta: droppedAfter - droppedBefore,
      accountedFrameDelta: presentedAfter - presentedBefore + droppedAfter - droppedBefore,
      targetDeltaMicroseconds: targetAfter - targetBefore,
      evidenceDigestChanged,
      visibleChanged: visibleAfter.visibleCenterHash !== visibleBefore.visibleCenterHash,
      nativePreviewEvents: summarizeNativePreviewEvents(nativeEvents),
      presentationSnapshotReads: {
        count: presentationDurations.length,
        min: presentationDurations[0] ?? null,
        p50: durationPercentile(0.5),
        p95: durationPercentile(0.95),
        max: presentationDurations.at(-1) ?? null
      },
      framePacing: summarizeFramePacing(framePacing),
      renderDurationMs: after?.telemetry?.renderDurationMs ?? null,
      firstFrameLatencyMs: after?.telemetry?.firstFrameLatencyMs ?? null,
      frameRequestsBefore,
      frameRequestsAfter: requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app))
    };
    console.log(`product preview cadence metrics ${JSON.stringify(metrics)}`);

    const pauseButton = controls.getByRole("button", { name: "暂停预览" });
    if ((await pauseButton.count()) > 0) {
      await pauseButton.click({ timeout: 5_000 });
    }

    expect(metrics.frameRequestsAfter, "cadence playback must not use requestProjectSessionPreviewFrame artifact fallback").toBe(
      metrics.frameRequestsBefore
    );
    expect(metrics.renderGraphActive, "cadence playback must finish on the renderGraphGpu product path").toBe(true);
    expect(metrics.targetDeltaMicroseconds, "3s playback should advance near the full 3s media window").toBeGreaterThanOrEqual(
      2_900_000
    );
    expect(metrics.accountedFrameDelta, "3s playback should account for all 90 frames via present or dropped-frame policy").toBeGreaterThanOrEqual(
      90
    );
    if (metrics.droppedDelta === 0) {
      expect(metrics.presentedDelta, "3s playback without drops should present all 90 frames for 30fps media").toBeGreaterThanOrEqual(
        90
      );
    }
    expect(metrics.presentationSnapshotReads.count, "Electron snapshot reads must not become the playback cadence").toBeLessThanOrEqual(
      30
    );
    expect(
      metrics.nativePreviewEvents.framePresented,
      "Rust playback worker must drive telemetry fanout through framePresented native events"
    ).toBeGreaterThanOrEqual(1);
    expect(
      metrics.nativePreviewEvents.controlChanged,
      "Rust control changes should be visible through native preview events"
    ).toBeGreaterThanOrEqual(1);
    expect(metrics.presentationSnapshotReads.p50, "presentation state queries must be lightweight snapshots").not.toBeNull();
    expect(metrics.presentationSnapshotReads.p50, "presentation state p50 should not include decode/render/present work").toBeLessThanOrEqual(
      16
    );
    expect(metrics.presentationSnapshotReads.p95, "presentation state p95 should stay below frame budget tail latency").not.toBeNull();
    expect(metrics.presentationSnapshotReads.p95, "presentation state p95 should stay below frame budget tail latency").toBeLessThanOrEqual(
      50
    );
    expect(framePacing, "Rust worker must expose frame pacing telemetry for product preview").not.toBeNull();
    expect(
      framePacing?.sampleCount ?? 0,
      "3s playback should include one pacing sample per presented frame"
    ).toBeGreaterThanOrEqual(90);
    expect(framePacing?.samples?.length ?? 0, "recent pacing sample buffer should cover the 3s window").toBeGreaterThanOrEqual(90);
    expect(framePacing?.intervalP50Ms, "frame pacing p50 must be reported").not.toBeNull();
    expect(framePacing?.intervalP50Ms ?? Number.POSITIVE_INFINITY).toBeGreaterThanOrEqual(25);
    expect(framePacing?.intervalP50Ms ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(42);
    expect(framePacing?.intervalP95Ms, "frame pacing p95 must be reported").not.toBeNull();
    expect(framePacing?.intervalP95Ms ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(50);
    expect(framePacing?.intervalMaxMs, "frame pacing max interval must be reported").not.toBeNull();
    expect(framePacing?.intervalMaxMs ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(75);
    expect(framePacing?.scheduleLatenessP95Ms, "scheduler lateness p95 must be reported").not.toBeNull();
    expect(framePacing?.scheduleLatenessP95Ms ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(12);
    expect(metrics.evidenceDigestChanged, "rendered content evidence should change during playback").toBe(true);
    expect(metrics.visibleChanged, "visible preview pixels should change during playback").toBe(true);
  } finally {
    await app.close();
  }
});

test("product preview cadence stays sustained for video external audio text and two-cue SRT", async () => {
  const { app, page } = await launchProductJourneyApp([
    USER_JOURNEY_MOVING_VIDEO,
    USER_JOURNEY_TONE_AUDIO
  ]);
  const srtContent =
    "1\n00:00:00,000 --> 00:00:01,400\n第一条组合字幕\n\n2\n00:00:01,400 --> 00:00:03,000\n第二条组合字幕\n";

  try {
    await importMaterialsThroughProductPicker(app, page, [USER_JOURNEY_MOVING_VIDEO, USER_JOURNEY_TONE_AUDIO]);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addAudioThroughProductPanel(page, app, USER_JOURNEY_TONE_AUDIO, 3_000_000);
    await addTextThroughProductPanel(page, app, "产品级组合文字", 3_000_000);
    await importSubtitleSrtThroughProductPanel(page, app, srtContent);

    await expectCadencePlayback(page, app, "product preview combo cadence metrics");
  } finally {
    await app.close();
  }
});

async function expectCadencePlayback(
  page: import("@playwright/test").Page,
  app: ProductJourneyAppController,
  logLabel: string
): Promise<void> {
  const controls = page.getByRole("group", { name: "预览播放控制" });
  const playButton = controls.getByRole("button", { name: "播放预览" });
  await expect(playButton).toBeEnabled({ timeout: 20_000 });

  const frameRequestsBefore = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
  const before = await readHostState(page);
  const visibleBefore = await captureVisiblePreviewEvidence(page, app);
  const nativeHostCallCountBeforePlay = (await readRealtimePreviewHostCalls(app)).length;

  await activateProductJourneyApp(app, page);
  await playButton.click();
  await waitForPlaybackProgress(page, before);
  const hostCallCountBefore = (await readRealtimePreviewHostCalls(app)).length;
  await page.waitForTimeout(3_000);
  const after = await readHostState(page);
  const visibleAfter = await captureVisiblePreviewEvidence(page, app);
  const presentationDurations = (await readRealtimePreviewHostCalls(app))
    .slice(hostCallCountBefore)
    .filter((call) => call.kind === "getPresentationState" && typeof call.durationMs === "number")
    .map((call) => call.durationMs as number)
    .sort((first, second) => first - second);
  const nativeEvents = (await readRealtimePreviewHostCalls(app))
    .slice(nativeHostCallCountBeforePlay)
    .filter((call) => call.kind === "nativePreviewEvent");
  const durationPercentile = (percentile: number) =>
    presentationDurations[Math.min(presentationDurations.length - 1, Math.floor(presentationDurations.length * percentile))] ??
    null;

  const presentedBefore = before?.telemetry?.presentedFrameCount ?? 0;
  const presentedAfter = after?.telemetry?.presentedFrameCount ?? 0;
  const droppedBefore = before?.telemetry?.droppedFrameCount ?? 0;
  const droppedAfter = after?.telemetry?.droppedFrameCount ?? 0;
  const targetBefore = before?.contentEvidence?.targetTimeMicroseconds ?? 0;
  const targetAfter = after?.contentEvidence?.targetTimeMicroseconds ?? 0;
  const renderGraphActive =
    after?.ok === true &&
    after.productReady &&
    !after.fallbackActive &&
    after.backend === "renderGraphGpu" &&
    after.contentEvidence?.source === "renderGraphGpuComposited";
  const evidenceDigestChanged =
    typeof after?.contentEvidence?.digest === "string" &&
    after.contentEvidence.digest.length > 0 &&
    (typeof before?.contentEvidence?.digest !== "string" ||
      before.contentEvidence.digest !== after.contentEvidence.digest);
  const framePacing = after?.telemetry?.framePacing ?? null;
  const metrics = {
    renderGraphActive,
    unsupportedReason: after?.unsupportedReason ?? null,
    lastPresentationFailure: (await readRealtimePreviewHostCalls(app))
      .filter((call) => call.kind === "getPresentationState" && typeof call.unsupportedReason === "string")
      .at(-1)?.unsupportedReason,
    presentedDelta: presentedAfter - presentedBefore,
    droppedDelta: droppedAfter - droppedBefore,
    accountedFrameDelta: presentedAfter - presentedBefore + droppedAfter - droppedBefore,
    targetDeltaMicroseconds: targetAfter - targetBefore,
    evidenceDigestChanged,
    visibleChanged: visibleAfter.visibleCenterHash !== visibleBefore.visibleCenterHash,
    nativePreviewEvents: summarizeNativePreviewEvents(nativeEvents),
    presentationSnapshotReads: {
      count: presentationDurations.length,
      min: presentationDurations[0] ?? null,
      p50: durationPercentile(0.5),
      p95: durationPercentile(0.95),
      max: presentationDurations.at(-1) ?? null
    },
    framePacing: summarizeFramePacing(framePacing),
    renderDurationMs: after?.telemetry?.renderDurationMs ?? null,
    firstFrameLatencyMs: after?.telemetry?.firstFrameLatencyMs ?? null,
    frameRequestsBefore,
    frameRequestsAfter: requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app))
  };
  console.log(`${logLabel} ${JSON.stringify(metrics)}`);

  const pauseButton = controls.getByRole("button", { name: "暂停预览" });
  if ((await pauseButton.count()) > 0) {
    await pauseButton.click({ timeout: 5_000 });
  }

  expect(metrics.frameRequestsAfter, "cadence playback must not use requestProjectSessionPreviewFrame artifact fallback").toBe(
    metrics.frameRequestsBefore
  );
  expect(metrics.renderGraphActive, "cadence playback must finish on the renderGraphGpu product path").toBe(true);
  expect(metrics.targetDeltaMicroseconds, "3s playback should advance near the full 3s media window").toBeGreaterThanOrEqual(
    2_900_000
  );
  expect(metrics.accountedFrameDelta, "3s playback should account for all 90 frames via present or dropped-frame policy").toBeGreaterThanOrEqual(
    90
  );
  if (metrics.droppedDelta === 0) {
    expect(metrics.presentedDelta, "3s playback without drops should present all 90 frames for 30fps media").toBeGreaterThanOrEqual(
      90
    );
  }
  expect(metrics.presentationSnapshotReads.count, "Electron snapshot reads must not become the playback cadence").toBeLessThanOrEqual(
    30
  );
  expect(
    metrics.nativePreviewEvents.framePresented,
    "Rust playback worker must drive telemetry fanout through framePresented native events"
  ).toBeGreaterThanOrEqual(1);
  expect(metrics.nativePreviewEvents.controlChanged, "Rust control changes should be visible through native preview events").toBeGreaterThanOrEqual(
    1
  );
  expect(metrics.presentationSnapshotReads.p50, "presentation state queries must be lightweight snapshots").not.toBeNull();
  expect(metrics.presentationSnapshotReads.p50, "presentation state p50 should not include decode/render/present work").toBeLessThanOrEqual(
    16
  );
  expect(metrics.presentationSnapshotReads.p95, "presentation state p95 should stay below frame budget tail latency").not.toBeNull();
  expect(metrics.presentationSnapshotReads.p95, "presentation state p95 should stay below frame budget tail latency").toBeLessThanOrEqual(
    50
  );
  expect(framePacing, "Rust worker must expose frame pacing telemetry for product preview").not.toBeNull();
  expect(
    framePacing?.sampleCount ?? 0,
    "3s playback should include one pacing sample per presented frame"
  ).toBeGreaterThanOrEqual(90);
  expect(framePacing?.samples?.length ?? 0, "recent pacing sample buffer should cover the 3s window").toBeGreaterThanOrEqual(90);
  expect(framePacing?.intervalP50Ms, "frame pacing p50 must be reported").not.toBeNull();
  expect(framePacing?.intervalP50Ms ?? Number.POSITIVE_INFINITY).toBeGreaterThanOrEqual(25);
  expect(framePacing?.intervalP50Ms ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(42);
  expect(framePacing?.intervalP95Ms, "frame pacing p95 must be reported").not.toBeNull();
  expect(framePacing?.intervalP95Ms ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(50);
  expect(framePacing?.intervalMaxMs, "frame pacing max interval must be reported").not.toBeNull();
  expect(framePacing?.intervalMaxMs ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(75);
  expect(framePacing?.scheduleLatenessP95Ms, "scheduler lateness p95 must be reported").not.toBeNull();
  expect(framePacing?.scheduleLatenessP95Ms ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(12);
  expect(metrics.evidenceDigestChanged, "rendered content evidence should change during playback").toBe(true);
  expect(metrics.visibleChanged, "visible preview pixels should change during playback").toBe(true);
}

function summarizeNativePreviewEvents(events: Array<{ nativeEventKind?: string }>) {
  return {
    total: events.length,
    controlChanged: events.filter((event) => event.nativeEventKind === "controlChanged").length,
    framePresented: events.filter((event) => event.nativeEventKind === "framePresented").length,
    playbackEnded: events.filter((event) => event.nativeEventKind === "playbackEnded").length,
    playbackError: events.filter((event) => event.nativeEventKind === "playbackError").length
  };
}

function summarizeFramePacing(framePacing: NonNullable<HostState["telemetry"]>["framePacing"] | null) {
  if (framePacing === null) {
    return null;
  }
  return {
    sampleCount: framePacing.sampleCount,
    sampleBufferLength: framePacing.samples.length,
    intervalP50Ms: framePacing.intervalP50Ms,
    intervalP95Ms: framePacing.intervalP95Ms,
    intervalMaxMs: framePacing.intervalMaxMs,
    scheduleLatenessP95Ms: framePacing.scheduleLatenessP95Ms,
    scheduleLatenessMaxMs: framePacing.scheduleLatenessMaxMs
  };
}

async function waitForPlaybackProgress(
  page: import("@playwright/test").Page,
  baseline: HostState | null
): Promise<HostState> {
  const baselinePresented = baseline?.telemetry?.presentedFrameCount ?? 0;
  const baselineTarget = baseline?.contentEvidence?.targetTimeMicroseconds ?? -1;
  const deadline = Date.now() + 10_000;
  let lastState: HostState | null = null;

  while (Date.now() < deadline) {
    lastState = await readHostState(page);
    const presented = lastState?.telemetry?.presentedFrameCount ?? 0;
    const target = lastState?.contentEvidence?.targetTimeMicroseconds ?? -1;
    if (
      lastState?.ok === true &&
      lastState.productReady &&
      !lastState.fallbackActive &&
      lastState.backend === "renderGraphGpu" &&
      lastState.contentEvidence?.source === "renderGraphGpuComposited" &&
      presented > baselinePresented &&
      target > baselineTarget
    ) {
      return lastState;
    }
    await page.waitForTimeout(50);
  }

  throw new Error(`Timed out waiting for Rust playback progress. Last host state: ${JSON.stringify(lastState)}`);
}

async function readHostState(page: import("@playwright/test").Page): Promise<HostState | null> {
  await page.evaluate(() => {
    const target = window as typeof window & {
      __videoEditorRealtimePreviewHostState?: HostState | null;
      __videoEditorRealtimePreviewHostObserverInstalled?: boolean;
      videoEditorRealtimePreviewHost?: {
        subscribeTelemetry: (listener: (state: HostState) => void) => () => void;
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
        __videoEditorRealtimePreviewHostState?: HostState | null;
      }).__videoEditorRealtimePreviewHostState ?? null
    );
  });
}

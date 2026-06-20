import { expect, test } from "@playwright/test";

import {
  USER_JOURNEY_MOVING_VIDEO,
  activateProductJourneyApp,
  addMaterialToTimeline,
  captureVisiblePreviewEvidence,
  importMaterialThroughProductPicker,
  launchProductJourneyApp,
  readExecuteCommandCalls,
  requestPreviewFrameCount
} from "./helpers/userJourney";

type HostState = {
  ok: boolean;
  productReady: boolean;
  fallbackActive: boolean;
  backend: "renderGraphGpu" | "none";
  telemetry: {
    presentedFrameCount: number;
    targetTimeMicroseconds: number;
    playbackGeneration: number;
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

    const controls = page.getByRole("group", { name: "预览播放控制" });
    const playButton = controls.getByRole("button", { name: "播放预览" });
    await expect(playButton).toBeEnabled({ timeout: 20_000 });

    const frameRequestsBefore = requestPreviewFrameCount(await readExecuteCommandCalls(app));
    const before = await readHostState(page);
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);

    await activateProductJourneyApp(app, page);
    await playButton.click();
    await page.waitForTimeout(3_000);
    const after = await readHostState(page);
    const visibleAfter = await captureVisiblePreviewEvidence(page, app);

    const presentedBefore = before?.telemetry?.presentedFrameCount ?? 0;
    const presentedAfter = after?.telemetry?.presentedFrameCount ?? 0;
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
    const metrics = {
      renderGraphActive,
      presentedDelta: presentedAfter - presentedBefore,
      targetDeltaMicroseconds: targetAfter - targetBefore,
      evidenceDigestChanged,
      visibleChanged: visibleAfter.visibleCenterHash !== visibleBefore.visibleCenterHash,
      frameRequestsBefore,
      frameRequestsAfter: requestPreviewFrameCount(await readExecuteCommandCalls(app))
    };
    console.log(`product preview cadence metrics ${JSON.stringify(metrics)}`);

    const pauseButton = controls.getByRole("button", { name: "暂停预览" });
    if ((await pauseButton.count()) > 0) {
      await pauseButton.click({ timeout: 5_000 });
    }

    expect(metrics.frameRequestsAfter, "cadence playback must not use requestPreviewFrame artifact fallback").toBe(
      metrics.frameRequestsBefore
    );
    expect(metrics.renderGraphActive, "cadence playback must finish on the renderGraphGpu product path").toBe(true);
    expect(metrics.targetDeltaMicroseconds, "3s playback should advance near the media duration").toBeGreaterThanOrEqual(
      2_000_000
    );
    expect(metrics.presentedDelta, "3s playback should present sustained GPU frames").toBeGreaterThanOrEqual(20);
    expect(metrics.evidenceDigestChanged, "rendered content evidence should change during playback").toBe(true);
    expect(metrics.visibleChanged, "visible preview pixels should change during playback").toBe(true);
  } finally {
    await app.close();
  }
});

async function readHostState(page: import("@playwright/test").Page): Promise<HostState | null> {
  return page.evaluate(async () => {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return null;
    }
    return (await bridge.getTelemetry()) as HostState;
  });
}

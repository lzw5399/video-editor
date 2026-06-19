import { expect, test } from "@playwright/test";

import {
  USER_JOURNEY_MOVING_VIDEO,
  addMaterialToTimeline,
  capturePreviewEvidence,
  clickPreviewPlay,
  importMaterialThroughProductPicker,
  launchProductJourneyApp,
  readExecuteCommandCalls,
  readRealtimePreviewHostCalls,
  requestPreviewFrameCount
} from "./helpers/userJourney";

test.describe.configure({ timeout: 90_000 });

test("product user can import a repo video, add it to the timeline, and see playback frames advance", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);

    const before = await capturePreviewEvidence(page);
    const frameRequestsBeforePlay = requestPreviewFrameCount(await readExecuteCommandCalls(app));

    await clickPreviewPlay(page);
    await page.waitForTimeout(1_200);

    const after = await capturePreviewEvidence(page);
    const frameRequestsAfterPlay = requestPreviewFrameCount(await readExecuteCommandCalls(app));

    expect(after.timecodeUs, "the user-visible playhead must advance after clicking play").toBeGreaterThan(
      before.timecodeUs + 500_000
    );
    expect(
      frameRequestsAfterPlay,
      "normal product playback must not be implemented by repeatedly requesting preview PNG frames"
    ).toBe(frameRequestsBeforePlay);

    expect(
      after.hostState?.telemetry?.presentedFrameCount ?? 0,
      "the realtime preview host must present frames while the product playhead is running"
    ).toBeGreaterThan(before.hostState?.telemetry?.presentedFrameCount ?? 0);
    expect(
      after.hostState?.telemetry?.targetTimeMicroseconds ?? 0,
      "runtime-presented frame time must advance with the user-visible playhead"
    ).toBeGreaterThan(before.hostState?.telemetry?.targetTimeMicroseconds ?? 0);
    expect(
      after.hostState?.contentEvidence?.source ?? null,
      "normal product playback evidence must come from decoded/composited video content, not mock frame tokens"
    ).toMatch(/^(decoded|composited)$/);
    expect(
      after.hostState?.contentEvidence?.digest ?? null,
      "native decoded/composited content fingerprint must advance during playback"
    ).not.toBe(before.hostState?.contentEvidence?.digest ?? null);
    expect(
      after.hostState?.contentEvidence?.targetTimeMicroseconds ?? 0,
      "content fingerprint time must advance with the user-visible playhead"
    ).toBeGreaterThan(before.hostState?.contentEvidence?.targetTimeMicroseconds ?? 0);
    expect(
      after.hostState?.frameDisplay,
      "normal product playback must not expose mock frame display evidence"
    ).toBeNull();

    expect(after.placeholderText, "playback should not be left on the empty-preview placeholder").not.toContain("显示预览");
    await expect(page.getByLabel("实时预览帧")).toHaveCount(0);

    await expect
      .poll(async () => (await readRealtimePreviewHostCalls(app)).map((call) => call.kind), { timeout: 5_000 })
      .toEqual(expect.arrayContaining(["updateDraftSnapshot", "seek", "play"]));
  } finally {
    await app.close();
  }
});

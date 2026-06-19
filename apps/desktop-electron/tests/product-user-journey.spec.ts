import { expect, test } from "@playwright/test";

import {
  USER_JOURNEY_MOVING_VIDEO,
  addMaterialToTimeline,
  capturePreviewEvidence,
  expectNoRejectedSurfaceAcquire,
  importMaterialThroughProductPicker,
  launchProductJourneyApp,
  readExecuteCommandCalls,
  readRealtimePreviewHostCalls,
  requestPreviewFrameCount,
  waitForCompositedPreviewEvidence
} from "./helpers/userJourney";

test.describe.configure({ timeout: 90_000 });

test("product playback rejects missing render-graph GPU compositor evidence", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO], {
    VIDEO_EDITOR_TEST_DISABLE_RENDER_GRAPH_COMPOSITOR: "1"
  });

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);

    const before = await capturePreviewEvidence(page);
    const frameRequestsBeforePlay = requestPreviewFrameCount(await readExecuteCommandCalls(app));

    const controls = page.getByRole("group", { name: "预览播放控制" });
    const playButton = controls.getByRole("button", { name: "播放预览" });
    await expect(playButton).toBeEnabled({ timeout: 20_000 });
    await playButton.click();

    await page.waitForTimeout(800);
    const after = await capturePreviewEvidence(page);
    const frameRequestsAfterPlay = requestPreviewFrameCount(await readExecuteCommandCalls(app));

    expect(
      after.timecodeUs,
      "playhead must not advance when only native video bridge evidence is available"
    ).toBe(before.timecodeUs);
    await expect(playButton, "failed product playback must leave the play button available").toBeEnabled();
    await expect(controls.getByRole("button", { name: "暂停预览" })).toHaveCount(0);

    expect(after.hostState?.ok, "play command must fail closed without render-graph GPU compositor").toBe(false);
    expect(
      after.hostState?.productReady,
      "native video bridge must not mark product realtime preview as ready"
    ).toBe(false);
    expect(
      after.hostState?.fallbackActive,
      "native video bridge rejection must be visible as unavailable state"
    ).toBe(true);
    expect(after.hostState?.fallbackLabel ?? "").toContain("render graph GPU compositor scheduler");
    expect(
      after.hostState?.backend ?? null,
      "product host backend must expose only renderGraphGpu success or none"
    ).toBe("none");
    expect(
      after.hostState?.diagnosticSource ?? null,
      "missing compositor evidence must not route through native video diagnostics"
    ).toBe("none");
    expect(
      after.hostState?.contentEvidence?.source ?? null,
      "native bridge content evidence must not be exposed as product evidence"
    ).toBeNull();
    expect(
      after.hostState?.telemetry?.presentedFrameCount ?? 0,
      "native bridge evidence must not increment realtime compositor presented-frame telemetry"
    ).toBe(before.hostState?.telemetry?.presentedFrameCount ?? 0);
    expect(
      after.hostState?.telemetry?.targetTimeMicroseconds ?? 0,
      "runtime-presented frame time must not advance without the compositor"
    ).toBe(before.hostState?.telemetry?.targetTimeMicroseconds ?? 0);
    expect(
      frameRequestsAfterPlay,
      "product playback rejection must not fall back to repeated preview PNG frame requests"
    ).toBe(frameRequestsBeforePlay);
    expect(
      after.hostState?.frameDisplay,
      "product playback rejection must not expose mock frame display evidence"
    ).toBeNull();
    expect(
      after.hostState?.contentEvidence?.source ?? null,
      "render-graph compositor evidence is intentionally absent until the compositor path is connected"
    ).not.toBe("renderGraphGpuComposited");
    await expect(page.getByLabel("实时预览帧")).toHaveCount(0);

    await expect
      .poll(async () => (await readRealtimePreviewHostCalls(app)).map((call) => call.kind), { timeout: 5_000 })
      .toEqual(
        expect.arrayContaining([
          "updateDraftSnapshot",
          "seek",
          "playRejectedMissingCompositor"
        ])
      );
    expect(
      (await readRealtimePreviewHostCalls(app)).map((call) => call.kind),
      "native bridge must not receive a product play command"
    ).not.toContain("play");

    expect(
      after.placeholderText,
      "failed playback should not be left on an empty debug/mock preview placeholder"
    ).not.toContain("实时预览帧");
  } finally {
    await app.close();
  }
});

test("product user can import a repo video, add it to the timeline, and see render-graph GPU playback frames advance", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);

    const before = await capturePreviewEvidence(page);
    const frameRequestsBeforePlay = requestPreviewFrameCount(await readExecuteCommandCalls(app));
    const controls = page.getByRole("group", { name: "预览播放控制" });
    const playButton = controls.getByRole("button", { name: "播放预览" });
    await expect(playButton).toBeEnabled({ timeout: 20_000 });
    await playButton.click();

    const after = await waitForCompositedPreviewEvidence(page, 12_000);
    const frameRequestsAfterPlay = requestPreviewFrameCount(await readExecuteCommandCalls(app));
    const hostCallKinds = (await readRealtimePreviewHostCalls(app)).map((call) => call.kind);
    expectNoRejectedSurfaceAcquire(await readRealtimePreviewHostCalls(app));

    expect(after.hostState?.ok).toBe(true);
    expect(after.hostState?.productReady).toBe(true);
    expect(after.hostState?.fallbackActive).toBe(false);
    expect(after.hostState?.backend).toBe("renderGraphGpu");
    expect(after.hostState?.diagnosticSource).toBe("none");
    expect(after.hostState?.contentEvidence?.source).toBe("renderGraphGpuComposited");
    expect(after.hostState?.contentEvidence?.digest).not.toBe(before.hostState?.contentEvidence?.digest ?? null);
    expect(after.hostState?.contentEvidence?.targetTimeMicroseconds ?? 0).toBeGreaterThan(
      before.hostState?.contentEvidence?.targetTimeMicroseconds ?? 0
    );
    expect(after.hostState?.telemetry?.presentedFrameCount ?? 0).toBeGreaterThan(
      before.hostState?.telemetry?.presentedFrameCount ?? 0
    );
    expect(after.timecodeUs).toBeGreaterThan(before.timecodeUs);
    expect(
      after.regionHash,
      "visible preview content must advance, not only the playhead or telemetry"
    ).not.toBe(before.regionHash);
    expect(
      frameRequestsAfterPlay,
      "product playback must not drive a requestPreviewFrame PNG/artifact loop"
    ).toBe(frameRequestsBeforePlay);
    expect(after.hostState?.frameDisplay).toBeNull();
    await expect(page.getByLabel("实时预览帧")).toHaveCount(0);
    expect(hostCallKinds).toEqual(
      expect.arrayContaining([
        "updateDraftSnapshot",
        "seek",
        "schedulerDecodeCurrentFrame",
        "schedulerBuildRenderGraph",
        "schedulerPresentSurface",
        "schedulerCompositedEvidence",
        "play"
      ])
    );
    expect(hostCallKinds).not.toContain("playRejectedMissingCompositor");
    await expect(controls.getByRole("button", { name: "暂停预览" })).toBeEnabled({ timeout: 10_000 });
  } finally {
    await app.close();
  }
});

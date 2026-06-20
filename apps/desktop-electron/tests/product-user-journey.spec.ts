import { expect, test } from "@playwright/test";

import {
  USER_JOURNEY_OVERLAY_IMAGE,
  USER_JOURNEY_MOVING_VIDEO,
  USER_JOURNEY_TONE_AUDIO,
  addAudioThroughProductPanel,
  addTextThroughProductPanel,
  addMaterialToTimeline,
  addVideoTrack,
  activateProductJourneyApp,
  capturePreviewEvidence,
  captureVisiblePreviewEvidence,
  deleteSelectedSegment,
  expectOccludedSurfaceAcquireHasDrawableLifecycleDiagnostics,
  expectNoProductFallbackCalls,
  expectNoRejectedSurfaceAcquire,
  importMaterialsThroughProductPicker,
  importMaterialThroughProductPicker,
  launchProductJourneyApp,
  moveSelectedSegmentRight,
  readExecuteCommandCalls,
  readRealtimePreviewHostCalls,
  requestPreviewFrameCount,
  redoTimelineEdit,
  seekTimelinePlayhead,
  splitSelectedSegment,
  undoTimelineEdit,
  updateSelectedVisualThroughInspector,
  waitForCompositedPreviewEvidence,
  waitForVisiblePreviewCenterChange
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
    await activateProductJourneyApp(app, page);
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
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsBeforePlay = requestPreviewFrameCount(await readExecuteCommandCalls(app));
    const controls = page.getByRole("group", { name: "预览播放控制" });
    const playButton = controls.getByRole("button", { name: "播放预览" });
    await expect(playButton).toBeEnabled({ timeout: 20_000 });
    await activateProductJourneyApp(app, page);
    await playButton.click();

    let after;
    let visibleMotion;
    try {
      visibleMotion = await waitForVisiblePreviewCenterChange(page, app, visibleBefore.visibleCenterHash, 5_000);
      after = await waitForCompositedPreviewEvidence(
        page,
        app,
        12_000,
        before.hostState?.contentEvidence?.targetTimeMicroseconds ?? before.timecodeUs
      );
    } catch (error) {
      const hostCalls = await readRealtimePreviewHostCalls(app);
      if (hostCalls.some((call) => call.kind === "surfaceAcquireOccluded")) {
        expectOccludedSurfaceAcquireHasDrawableLifecycleDiagnostics(hostCalls);
      }
      throw error;
    }
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
      visibleMotion.visibleCenterHash,
      "visible video pixels in the preview center must change while playback is running"
    ).not.toBe(visibleBefore.visibleCenterHash);
    expect(visibleMotion.hostState?.contentEvidence?.targetTimeMicroseconds ?? 0).toBeGreaterThan(
      before.hostState?.contentEvidence?.targetTimeMicroseconds ?? 0
    );
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
  } finally {
    await app.close();
  }
});

test("product playback UAT keeps the native surface aligned with the preview monitor", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);

    const controls = page.getByRole("group", { name: "预览播放控制" });
    const playButton = controls.getByRole("button", { name: "播放预览" });
    await expect(playButton).toBeEnabled({ timeout: 20_000 });
    await activateProductJourneyApp(app, page);
    await playButton.click();

    const after = await waitForCompositedPreviewEvidence(page, app, 12_000);
    const placement = after.hostState?.surfacePlacement ?? null;
    expect(placement, "product playback must expose native surface placement evidence").not.toBeNull();
    expect(placement?.aligned, "native/WGPU surface must align with preview host").toBe(true);
    expect(placement?.maxDeltaPx ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(2);
  } finally {
    await app.close();
  }
});

test("product playback UAT uses native audio output instead of status-only or mock audio", async () => {
  const { app, page } = await launchProductJourneyApp([
    USER_JOURNEY_MOVING_VIDEO,
    USER_JOURNEY_TONE_AUDIO
  ]);

  try {
    await importMaterialsThroughProductPicker(app, page, [USER_JOURNEY_MOVING_VIDEO, USER_JOURNEY_TONE_AUDIO]);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addAudioThroughProductPanel(page, app, USER_JOURNEY_TONE_AUDIO, 3_000_000);

    const controls = page.getByRole("group", { name: "预览播放控制" });
    await activateProductJourneyApp(app, page);
    await controls.getByRole("button", { name: "播放预览" }).click();
    await waitForCompositedPreviewEvidence(page, app, 12_000);

    await expect
      .poll(async () => (await readExecuteCommandCalls(app)).map((call) => call.command), { timeout: 10_000 })
      .toContain("playAudioPreview");
    await expect(page.getByLabel("音频预览状态")).toContainText("正在播放", { timeout: 10_000 });
    await expect(
      page.getByLabel("输出设备状态"),
      "product playback must not report mock/status-only audio as audible output"
    ).not.toContainText(/Mock|mock|模拟|系统默认/);
  } finally {
    await app.close();
  }
});

test("product playback UAT keeps video presentation synchronized with timeline through sequence end", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);

    await activateProductJourneyApp(app, page);
    await page.getByRole("group", { name: "预览播放控制" }).getByRole("button", { name: "播放预览" }).click();
    await waitForCompositedPreviewEvidence(page, app, 12_000);

    await expect.poll(async () => (await capturePreviewEvidence(page)).timecodeUs, { timeout: 6_000 }).toBeGreaterThanOrEqual(3_000_000);
    const atEnd = await capturePreviewEvidence(page);
    const presentedTime = atEnd.hostState?.contentEvidence?.targetTimeMicroseconds ?? -1;
    expect(
      Math.abs(atEnd.timecodeUs - presentedTime),
      "timeline playhead and rendered video target time must stay synchronized at sequence end"
    ).toBeLessThanOrEqual(100_000);
    const frameCountAtEnd = atEnd.hostState?.telemetry?.presentedFrameCount ?? 0;

    await page.waitForTimeout(800);
    const afterStop = await capturePreviewEvidence(page);
    expect(afterStop.hostState?.telemetry?.presentedFrameCount ?? 0).toBe(
      frameCountAtEnd
    );
    await expect(page.getByRole("group", { name: "预览播放控制" }).getByRole("button", { name: "播放预览" })).toBeEnabled();
  } finally {
    await app.close();
  }
});

test("product user editing matrix uses real commands and still produces visible GPU playback", async () => {
  const { app, page } = await launchProductJourneyApp([
    USER_JOURNEY_MOVING_VIDEO,
    USER_JOURNEY_OVERLAY_IMAGE,
    USER_JOURNEY_TONE_AUDIO
  ]);

  try {
    await importMaterialsThroughProductPicker(app, page, [
      USER_JOURNEY_MOVING_VIDEO,
      USER_JOURNEY_OVERLAY_IMAGE,
      USER_JOURNEY_TONE_AUDIO
    ]);

    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addVideoTrack(page, app);
    await addMaterialToTimeline(app, page, USER_JOURNEY_OVERLAY_IMAGE);
    await page.getByRole("button", { name: /片段 p0-overlay-testsrc\.png/ }).click();
    await updateSelectedVisualThroughInspector(page, app, {
      positionX: -120,
      positionY: -70,
      scaleX: 350,
      scaleY: 350,
      rotation: 0,
      opacity: 760,
      fitMode: "适应"
    });
    await addTextThroughProductPanel(page, app, "产品级端到端字幕", 1_000_000);
    await addAudioThroughProductPanel(page, app, USER_JOURNEY_TONE_AUDIO, 2_000_000);

    await page.getByRole("button", { name: /片段 p0-moving-testsrc\.mp4/ }).click();
    await updateSelectedVisualThroughInspector(page, app);
    await seekTimelinePlayhead(page, app, 500_000);
    await splitSelectedSegment(page, app, 1_500_000);
    await moveSelectedSegmentRight(page, app, 250_000);
    await deleteSelectedSegment(page, app);
    await undoTimelineEdit(page, app);
    await redoTimelineEdit(page, app);
    await undoTimelineEdit(page, app);
    await page.getByRole("button", { name: /片段 p0-overlay-testsrc\.png/ }).click();
    await deleteSelectedSegment(page, app);
    await seekTimelinePlayhead(page, app, 2_100_000);

    const callsAfterEdits = await readExecuteCommandCalls(app);
    expect(callsAfterEdits.map((call) => call.command)).toEqual(
      expect.arrayContaining([
        "importMaterial",
        "addSegment",
        "addTrack",
        "addTextSegment",
        "addAudioSegment",
        "updateSegmentVisual",
        "splitSegment",
        "moveSegment",
        "deleteSegment",
        "undoTimelineEdit",
        "redoTimelineEdit"
      ])
    );
    expect(requestPreviewFrameCount(callsAfterEdits), "product editing matrix must not use artifact preview frames").toBe(0);
    expect(callsAfterEdits.find((call) => call.command === "addTextSegment")?.textContent).toBe("产品级端到端字幕");
    const visualCall = [...callsAfterEdits].reverse().find((call) => call.command === "updateSegmentVisual");
    expect(visualCall?.visual?.fitMode).toBe("fill");
    expect(visualCall?.visual?.transform.position.x).toBe(120);
    expect(visualCall?.visual?.transform.rotation.degrees).toBe(8);
    expect(visualCall?.visual?.transform.opacity.valueMillis).toBe(820);

    const before = await capturePreviewEvidence(page);
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    await activateProductJourneyApp(app, page);
    await page.getByRole("group", { name: "预览播放控制" }).getByRole("button", { name: "播放预览" }).click();
    const visibleMotion = await waitForVisiblePreviewCenterChange(page, app, visibleBefore.visibleCenterHash, 5_000);
    const after = await waitForCompositedPreviewEvidence(
      page,
      app,
      12_000,
      before.hostState?.contentEvidence?.targetTimeMicroseconds ?? before.timecodeUs
    );

    expect(after.hostState?.productReady).toBe(true);
    expect(after.hostState?.fallbackActive).toBe(false);
    expect(after.hostState?.contentEvidence?.source).toBe("renderGraphGpuComposited");
    expect(visibleMotion.visibleCenterHash).not.toBe(visibleBefore.visibleCenterHash);
    expectNoProductFallbackCalls(await readRealtimePreviewHostCalls(app));
  } finally {
    await app.close();
  }
});

test("product text and transform interaction UAT supports direct canvas drag", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addTextThroughProductPanel(page, app, "可拖拽文字", 2_000_000);

    const textOverlay = page.getByLabel("预览文字");
    await expect(textOverlay).toBeVisible({ timeout: 10_000 });
    const beforeBox = await textOverlay.boundingBox();
    expect(beforeBox, "text overlay must have a visible canvas box before drag").not.toBeNull();

    const commandsBefore = await readExecuteCommandCalls(app);
    const visualUpdatesBefore = commandsBefore.filter((call) => call.command === "updateSegmentVisual").length;
    await page.mouse.move(beforeBox!.x + beforeBox!.width / 2, beforeBox!.y + beforeBox!.height / 2);
    await page.mouse.down();
    await page.mouse.move(beforeBox!.x + beforeBox!.width / 2 + 80, beforeBox!.y + beforeBox!.height / 2 + 36, {
      steps: 8
    });
    await page.mouse.up();

    await expect
      .poll(
        async () => (await readExecuteCommandCalls(app)).filter((call) => call.command === "updateSegmentVisual").length,
        { timeout: 5_000 }
      )
      .toBeGreaterThan(visualUpdatesBefore);
    const afterBox = await textOverlay.boundingBox();
    expect(afterBox, "text overlay must remain visible after direct drag").not.toBeNull();
    expect(
      Math.abs((afterBox?.x ?? 0) - beforeBox!.x) + Math.abs((afterBox?.y ?? 0) - beforeBox!.y),
      "direct canvas drag must move the selected text overlay"
    ).toBeGreaterThan(20);
  } finally {
    await app.close();
  }
});

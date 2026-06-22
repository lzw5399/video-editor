import { expect, test, type Page } from "@playwright/test";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import {
  USER_JOURNEY_AV_VIDEO,
  USER_JOURNEY_LONG_AV_VIDEO,
  USER_JOURNEY_LONG_MOVING_VIDEO,
  USER_JOURNEY_LONG_TONE_AUDIO,
  USER_JOURNEY_OVERLAY_IMAGE,
  USER_JOURNEY_MOVING_VIDEO,
  USER_JOURNEY_TONE_AUDIO,
  addAudioThroughProductPanel,
  addTextThroughProductPanel,
  addMaterialToTimeline,
  dragMaterialToTimeline,
  addVideoTrack,
  activateProductJourneyApp,
  capturePreviewEvidence,
  captureVisiblePreviewCoverageEvidence,
  captureVisiblePreviewEvidence,
  captureVisiblePreviewHostImage,
  deleteSelectedSegment,
  expectOccludedSurfaceAcquireHasDrawableLifecycleDiagnostics,
  expectNoProductFallbackCalls,
  expectTimelineSnappingStatusVisible,
  expectNoRejectedSurfaceAcquire,
  expectProductPlaybackSuccessEvidence,
  expectVisiblePreviewCoverageChanged,
  importSubtitleSrtThroughProductPanel,
  importMaterialsThroughProductPicker,
  importMaterialThroughProductPicker,
  launchProductJourneyApp,
  moveSelectedSegmentRight,
  readNativeCommandObservations,
  readDirectNativeCommandObservations,
  readProjectSessionCalls,
  readRealtimePreviewHostCalls,
  readTimelineSegments,
  requestProjectSessionPreviewFrameCount,
  redoTimelineEdit,
  seekTimelinePlayhead,
  splitSelectedSegment,
  trimSelectedSegmentLeftEdgeRight,
  undoTimelineEdit,
  updateSelectedVisualThroughInspector,
  waitForCompositedPreviewEvidence,
  waitForProductPlaybackSuccess,
  zoomTimelineIn
} from "./helpers/userJourney";

test.describe.configure({ timeout: 90_000 });

const REPO_ROOT = join(process.cwd(), "../..");
const PHASE15_3_SCREENSHOT_DIR = join(REPO_ROOT, "test-results/phase15-3");
const USER_JOURNEY_SEQUENCE_DURATION_US = 3_000_000;
const THIRTY_FPS_FRAME_DURATION_US = 33_333;
const SEQUENCE_END_FRAME_ALIGNED_MIN_US =
  USER_JOURNEY_SEQUENCE_DURATION_US - THIRTY_FPS_FRAME_DURATION_US - 7_000;
const P0_USER_PORTRAIT_MATERIAL =
  process.env.VIDEO_EDITOR_P0_USER_MATERIAL ??
  join(process.env.HOME ?? "", "Downloads", "5300d8457cc6d4692ff5b922c089f823_raw.mp4");

test("product playback helper rejects playhead-only advancement without visible compositor motion", () => {
  const before = {
    regionHash: "region-before",
    visibleCenterHash: "same-visible-center",
    timecodeUs: 0,
    placeholderText: "",
    imageSrc: null,
    hostState: {
      ok: true,
      productReady: true,
      hostAttached: true,
      fallbackActive: false,
      statusLabel: "实时预览已接入",
      fallbackLabel: null,
      unsupportedReason: null,
      playbackGeneration: 1,
      backend: "renderGraphGpu" as const,
      diagnosticSource: "none" as const,
      fallbackReason: null,
      currentRequestCanceled: false,
      fallbackArtifactVisible: false,
      telemetry: {
        presentedFrameCount: 1,
        targetTimeMicroseconds: 0,
        playbackGeneration: 1
      },
      frameDisplay: null,
      contentEvidence: {
        source: "renderGraphGpuComposited" as const,
        digest: "digest-before",
        width: 320,
        height: 180,
        byteCount: 0,
        targetTimeMicroseconds: 0,
        presentedFrames: 1,
        submittedDraws: 1
      },
      surfacePlacement: null
    }
  };
  const playheadOnlyAfter = {
    ...before,
    regionHash: "region-after",
    timecodeUs: 1_000_000,
    hostState: {
      ...before.hostState,
      telemetry: {
        presentedFrameCount: 2,
        targetTimeMicroseconds: 1_000_000,
        playbackGeneration: 1
      },
      contentEvidence: {
        ...before.hostState.contentEvidence,
        digest: "digest-after",
        targetTimeMicroseconds: 1_000_000
      }
    }
  };

  expect(() =>
    expectProductPlaybackSuccessEvidence({
      before,
      visibleBefore: before,
      visibleMotion: playheadOnlyAfter,
      after: playheadOnlyAfter,
      frameRequestsBeforePlay: 0,
      frameRequestsAfterPlay: 0
    })
  ).toThrow(/visible video pixels/);
});

test("product playback rejects missing render-graph GPU compositor evidence", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO], {
    VIDEO_EDITOR_TEST_DISABLE_RENDER_GRAPH_COMPOSITOR: "1"
  });

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await dragMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    const sessionCalls = await readProjectSessionCalls(app);
    expect(sessionCalls).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          command: "createProjectSession",
          hasDraftField: false
        }),
        expect.objectContaining({
          command: "executeProjectIntent",
          expectedRevision: 0,
          intentKind: "importMaterial",
          hasDraftField: false
        }),
        expect.objectContaining({
          command: "executeProjectIntent",
          expectedRevision: 1,
          intentKind: "addTimelineSegmentIntent",
          hasDraftField: false,
          timelineSemanticKeys: []
        })
      ])
    );
    const directNativeCommands = (await readDirectNativeCommandObservations(app)).map((call) => call.command);
    expect(directNativeCommands, "product material reads must use Rust project session APIs").not.toContain("listMaterials");
    expect(directNativeCommands, "product missing-material reads must use Rust project session APIs").not.toContain("listMissingMaterials");
    expect(directNativeCommands, "product import must not use renderer-owned draft importMaterial").not.toContain("importMaterial");
    expect(directNativeCommands, "product add-to-timeline must not use renderer-owned draft addTimelineSegmentIntent").not.toContain(
      "addTimelineSegmentIntent"
    );

    const before = await capturePreviewEvidence(page);
    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));

    const controls = page.getByRole("group", { name: "预览播放控制" });
    const playButton = controls.getByRole("button", { name: "播放预览" });
    await expect(playButton).toBeEnabled({ timeout: 20_000 });
    await activateProductJourneyApp(app, page);
    await playButton.click();

    await page.waitForTimeout(800);
    const after = await capturePreviewEvidence(page);
    const frameRequestsAfterPlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));

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
          "updateProjectSessionSnapshot",
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
    await dragMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    const firstFrame = await waitForCompositedPreviewEvidence(page, app, 8_000, -1);
    expect(
      firstFrame.hostState?.contentEvidence?.targetTimeMicroseconds ?? Number.POSITIVE_INFINITY,
      "dragging material to the timeline must present a first preview frame before playback starts"
    ).toBeLessThanOrEqual(100_000);

    const before = firstFrame;
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    const controls = page.getByRole("group", { name: "预览播放控制" });
    const playButton = controls.getByRole("button", { name: "播放预览" });
    await expect(playButton).toBeEnabled({ timeout: 20_000 });
    await activateProductJourneyApp(app, page);
    await playButton.click();

    let after;
    let visibleMotion;
    try {
      ({ after, visibleMotion } = await waitForProductPlaybackSuccess(page, app, before, visibleBefore, frameRequestsBeforePlay));
    } catch (error) {
      const hostCalls = await readRealtimePreviewHostCalls(app);
      if (hostCalls.some((call) => call.kind === "surfaceAcquireOccluded")) {
        expectOccludedSurfaceAcquireHasDrawableLifecycleDiagnostics(hostCalls);
      }
      throw error;
    }
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
    expect(after.hostState?.frameDisplay).toBeNull();
    await expect(page.getByLabel("实时预览帧")).toHaveCount(0);
    expect(hostCallKinds).toEqual(
      expect.arrayContaining([
        "updateProjectSessionSnapshot",
        "seek",
        "schedulerPlaybackWorkerStart",
        "play"
      ])
    );
    expect(hostCallKinds).not.toContain("playRejectedMissingCompositor");
  } finally {
    await app.close();
  }
});

test("P0 user portrait material imports, drags to timeline, presents first frame, and plays on native surface", async () => {
  test.skip(
    !existsSync(P0_USER_PORTRAIT_MATERIAL),
    `P0 user material not present at ${P0_USER_PORTRAIT_MATERIAL}; set VIDEO_EDITOR_P0_USER_MATERIAL to run this local regression`
  );

  const { app, page } = await launchProductJourneyApp([P0_USER_PORTRAIT_MATERIAL]);

  try {
    await importMaterialThroughProductPicker(app, page, P0_USER_PORTRAIT_MATERIAL);
    await dragMaterialToTimeline(app, page, P0_USER_PORTRAIT_MATERIAL);

    const firstFrame = await waitForCompositedPreviewEvidence(page, app, 12_000, -1);
    expect(firstFrame.hostState?.contentEvidence?.source).toBe("renderGraphGpuComposited");
    expect(firstFrame.hostState?.fallbackActive).toBe(false);
    expect(
      firstFrame.hostState?.contentEvidence?.targetTimeMicroseconds ?? Number.POSITIVE_INFINITY,
      "dragging the P0 material must present a first preview frame before playback starts"
    ).toBeLessThanOrEqual(100_000);

    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    await activateProductJourneyApp(app, page);
    await page.getByRole("group", { name: "预览播放控制" }).getByRole("button", { name: "播放预览" }).click();
    const { after } = await waitForProductPlaybackSuccess(page, app, firstFrame, visibleBefore, frameRequestsBeforePlay, 15_000);

    expect(after.hostState?.surfacePlacement?.maxDeltaPx ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(2);
    expect(after.hostState?.contentEvidence?.width).toBeGreaterThan(0);
    expect(after.hostState?.contentEvidence?.height).toBeGreaterThan(0);
    mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "p0-user-portrait-native-preview.png"),
      await captureVisiblePreviewHostImage(page, app)
    );
    const hostCalls = await readRealtimePreviewHostCalls(app);
    expectNoProductFallbackCalls(hostCalls);
    expectNoRejectedSurfaceAcquire(hostCalls);
    expect(requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app))).toBe(frameRequestsBeforePlay);
  } finally {
    await app.close();
  }
});

test("product playback UAT keeps the native surface aligned with the preview monitor", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);

    const before = await capturePreviewEvidence(page);
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    const controls = page.getByRole("group", { name: "预览播放控制" });
    const playButton = controls.getByRole("button", { name: "播放预览" });
    await expect(playButton).toBeEnabled({ timeout: 20_000 });
    await activateProductJourneyApp(app, page);
    await playButton.click();

    const { after } = await waitForProductPlaybackSuccess(page, app, before, visibleBefore, frameRequestsBeforePlay);
    await expect(page.getByLabel("预览选中框")).toHaveCount(0, { timeout: 5_000 });
    const coverageBefore = await captureVisiblePreviewCoverageEvidence(page, app);
    const coverageStartTimeUs = after.hostState?.contentEvidence?.targetTimeMicroseconds ?? 0;
    await waitForCompositedPreviewEvidence(
      page,
      app,
      8_000,
      Math.min(coverageStartTimeUs + 500_000, SEQUENCE_END_FRAME_ALIGNED_MIN_US)
    );
    const coverageAfter = await captureVisiblePreviewCoverageEvidence(page, app);
    const placement = after.hostState?.surfacePlacement ?? null;
    expect(placement, "product playback must expose native surface placement evidence").not.toBeNull();
    const expectedScreenRect = await expectedPreviewHostScreenRect(page, app);
    await expectPreviewHostCoversCanvas(page);
    mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "native-surface-playing-coverage.png"),
      await captureVisiblePreviewHostImage(page, app)
    );
    await page.screenshot({
      path: join(PHASE15_3_SCREENSHOT_DIR, "native-surface-playing-workspace.png"),
      fullPage: true
    });
    expectVisiblePreviewCoverageChanged(coverageBefore, coverageAfter);
    expect(placement?.surfaceBoundsCoordinateSpace).toBe("browserWindowContentLogicalPixels");
    expect(placement?.screenRectCoordinateSpace).toBe("electronScreenLogicalPixels");
    expect(placement?.nativeAppKitScreenRect, "raw AppKit screen rect must be exposed for placement telemetry").toBeTruthy();
    expect(
      maxRectDelta(placement?.hostScreenRect ?? null, expectedScreenRect),
      `main-process host screen rect must use the BrowserWindow content-local logical-pixel contract: ${JSON.stringify({
        placement,
        expectedScreenRect
      })}`
    ).toBeLessThanOrEqual(2);
    expect(
      maxRectDelta(placement?.nativeScreenRect ?? null, expectedScreenRect),
      `native/WGPU child view must cover the DOM preview host during playback: ${JSON.stringify({
        placement,
        expectedScreenRect
      })}`
    ).toBeLessThanOrEqual(2);
    expect(placement?.maxDeltaPx ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(2);
    expect(Math.abs(placement?.deltaPx.x ?? Number.POSITIVE_INFINITY)).toBeLessThanOrEqual(2);
    expect(Math.abs(placement?.deltaPx.y ?? Number.POSITIVE_INFINITY)).toBeLessThanOrEqual(2);

    await page.getByLabel("产品操作").getByRole("button", { name: "导出", exact: true }).click();
    await expect(page.getByRole("dialog", { name: "导出" })).toBeVisible();
    await expect
      .poll(async () => (await readRealtimePreviewHostCalls(app)).some((call) => call.kind === "detachSurface"), { timeout: 5_000 })
      .toBe(true);
  } finally {
    await app.close();
  }
});

test("product playback UAT uses native audio output instead of status-only or mock audio", async () => {
  const { app, page } = await launchProductJourneyApp([
    USER_JOURNEY_LONG_MOVING_VIDEO,
    USER_JOURNEY_LONG_TONE_AUDIO
  ]);

  try {
    await importMaterialsThroughProductPicker(app, page, [USER_JOURNEY_LONG_MOVING_VIDEO, USER_JOURNEY_LONG_TONE_AUDIO]);
    await addMaterialToTimeline(app, page, USER_JOURNEY_LONG_MOVING_VIDEO);
    await addAudioThroughProductPanel(page, app, USER_JOURNEY_LONG_TONE_AUDIO, 8_000_000);

    const before = await capturePreviewEvidence(page);
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    const controls = page.getByRole("group", { name: "预览播放控制" });
    await activateProductJourneyApp(app, page);
    await controls.getByRole("button", { name: "播放预览" }).click();

    await expect
      .poll(async () => (await readNativeCommandObservations(app)).map((call) => call.command), { timeout: 10_000 })
      .toContain("playAudioPreview");
    await expect.poll(async () => (await readNativeCommandObservations(app)).some((call) => (
      call.command === "playAudioPreview" &&
      call.sessionId !== null &&
      typeof call.projectSessionId === "string" &&
      typeof call.expectedRevision === "number" &&
      call.hasDraftField === false
    )), { timeout: 10_000 }).toBe(true);
    await expectNativeAudioContinuity(page, app);
    await waitForProductPlaybackSuccess(page, app, before, visibleBefore, frameRequestsBeforePlay);
  } finally {
    await app.close();
  }
});

test("product playback UAT plays embedded video audio through native output", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_LONG_AV_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_LONG_AV_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_LONG_AV_VIDEO);

    const before = await capturePreviewEvidence(page);
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    const controls = page.getByRole("group", { name: "预览播放控制" });
    await activateProductJourneyApp(app, page);
    await controls.getByRole("button", { name: "播放预览" }).click();

    await expect
      .poll(async () => (await readNativeCommandObservations(app)).map((call) => call.command), { timeout: 10_000 })
      .toContain("playAudioPreview");
    await expect.poll(async () => (await readNativeCommandObservations(app)).some((call) => (
      call.command === "playAudioPreview" &&
      call.sessionId !== null &&
      typeof call.projectSessionId === "string" &&
      typeof call.expectedRevision === "number" &&
      call.hasDraftField === false
    )), { timeout: 10_000 }).toBe(true);
    await expectNativeAudioContinuity(page, app);
    await waitForProductPlaybackSuccess(page, app, before, visibleBefore, frameRequestsBeforePlay);
  } finally {
    await app.close();
  }
});

test("product playback UAT composites video external audio text and two-cue SRT on the native surface", async () => {
  const { app, page } = await launchProductJourneyApp([
    USER_JOURNEY_MOVING_VIDEO,
    USER_JOURNEY_TONE_AUDIO
  ]);
  const srtContent =
    "1\n00:00:00,000 --> 00:00:02,000\n第一条组合字幕\n\n2\n00:00:02,000 --> 00:00:03,000\n第二条组合字幕\n";

  try {
    await importMaterialsThroughProductPicker(app, page, [USER_JOURNEY_MOVING_VIDEO, USER_JOURNEY_TONE_AUDIO]);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addAudioThroughProductPanel(page, app, USER_JOURNEY_TONE_AUDIO);
    await addTextThroughProductPanel(page, app, "组合标题");
    await updateSelectedVisualThroughInspector(page, app, {
      positionX: 0,
      positionY: -240,
      scaleX: 1000,
      scaleY: 1000,
      rotation: 0,
      opacity: 1000,
      fitMode: "适应"
    });

    const commandCountBeforeSrt = await readNativeCommandObservations(app);
    await importSubtitleSrtThroughProductPanel(page, app, srtContent);
    const commandCountAfterSrt = await readNativeCommandObservations(app);
    expect(commandCountAfterSrt.filter((call) => call.command === "importSubtitleSrtIntent")).toHaveLength(1);
    expect(
      commandCountAfterSrt.filter((call) => call.command === "addTextSegment").length,
      "SRT import must not be faked by renderer-created text segment commands"
    ).toBe(commandCountBeforeSrt.filter((call) => call.command === "addTextSegment").length);

    await seekTimelinePlayhead(page, app, 0);
    const before = await capturePreviewEvidence(page);
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    await activateProductJourneyApp(app, page);
    await page.getByRole("group", { name: "预览播放控制" }).getByRole("button", { name: "播放预览" }).click();
    await expect
      .poll(async () => (await readNativeCommandObservations(app)).some((call) => (
        call.command === "playAudioPreview" &&
        call.sessionId !== null &&
        typeof call.projectSessionId === "string" &&
        typeof call.expectedRevision === "number" &&
        call.hasDraftField === false
      )), { timeout: 10_000 })
      .toBe(true);

    const firstSubtitleEvidence = await waitForActiveSubtitleEvidence(page, app, "第一条组合字幕", 0, 1_900_000);
    mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "combo-preview-first-subtitle.png"),
      firstSubtitleEvidence.hostImage
    );
    const { after } = await waitForProductPlaybackSuccess(page, app, before, visibleBefore, frameRequestsBeforePlay);
    const secondSubtitleEvidence = await waitForActiveSubtitleEvidence(page, app, "第二条组合字幕", 2_000_000);
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "combo-preview-second-subtitle.png"),
      secondSubtitleEvidence.hostImage
    );

    expect(firstSubtitleEvidence.activeTextOverlays).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ source: "text", content: "组合标题" }),
        expect.objectContaining({ source: "subtitle", content: "第一条组合字幕" })
      ])
    );
    expect(secondSubtitleEvidence.activeTextOverlays).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ source: "text", content: "组合标题" }),
        expect.objectContaining({ source: "subtitle", content: "第二条组合字幕" })
      ])
    );
    expect(firstSubtitleEvidence.activeTextOverlays).not.toEqual(secondSubtitleEvidence.activeTextOverlays);
    expect(after.hostState?.contentEvidence?.source).toBe("renderGraphGpuComposited");
    expect(after.hostState?.surfacePlacement?.maxDeltaPx ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(2);
    expect(after.hostState?.telemetry?.presentedFrameCount ?? 0).toBeGreaterThan(
      before.hostState?.telemetry?.presentedFrameCount ?? 0
    );
    await expect(page.locator(".preview-text-overlay"), "product realtime text evidence must not be a DOM overlay").toHaveCount(0);
    const hostCalls = await readRealtimePreviewHostCalls(app);
    expectNoProductFallbackCalls(hostCalls);
    expectNoRejectedSurfaceAcquire(hostCalls);
    expect(requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app))).toBe(frameRequestsBeforePlay);
  } finally {
    await app.close();
  }
});

async function waitForActiveSubtitleEvidence(
  page: Page,
  app: Awaited<ReturnType<typeof launchProductJourneyApp>>["app"],
  subtitle: string,
  minTargetTimeUs: number,
  maxTargetTimeUs = Number.POSITIVE_INFINITY
) {
  const deadline = Date.now() + 10_000;
  let lastEvidence: unknown = null;

  while (Date.now() < deadline) {
    const evidence = (await capturePreviewEvidence(page)).hostState?.contentEvidence;
    const activeTextOverlays = evidence?.activeTextOverlays ?? [];
    const activeSubtitle = activeTextOverlays.find((text) => text.source === "subtitle")?.content ?? null;
    lastEvidence = {
      activeSubtitle,
      activeTextOverlays,
      source: evidence?.source ?? null,
      targetTimeMicroseconds: evidence?.targetTimeMicroseconds ?? 0
    };
    if (
      evidence?.source === "renderGraphGpuComposited" &&
      (evidence.targetTimeMicroseconds ?? 0) >= minTargetTimeUs &&
      (evidence.targetTimeMicroseconds ?? 0) <= maxTargetTimeUs &&
      activeSubtitle === subtitle
    ) {
      return {
        activeTextOverlays,
        targetTimeMicroseconds: evidence.targetTimeMicroseconds,
        hostImage: await captureVisiblePreviewHostImage(page, app)
      };
    }
    await page.waitForTimeout(200);
  }

  throw new Error(`Timed out waiting for active subtitle ${subtitle}: ${JSON.stringify(lastEvidence)}`);
}

test("product playback UAT keeps video presentation synchronized with timeline through sequence end", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);

    const before = await capturePreviewEvidence(page);
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    await activateProductJourneyApp(app, page);
    await page.getByRole("group", { name: "预览播放控制" }).getByRole("button", { name: "播放预览" }).click();
    await waitForProductPlaybackSuccess(page, app, before, visibleBefore, frameRequestsBeforePlay);

    await expect
      .poll(async () => (await capturePreviewEvidence(page)).timecodeUs, { timeout: 6_000 })
      .toBeGreaterThanOrEqual(SEQUENCE_END_FRAME_ALIGNED_MIN_US);
    const atEnd = await capturePreviewEvidence(page);
    const presentedTime = atEnd.hostState?.contentEvidence?.targetTimeMicroseconds ?? -1;
    expect(
      presentedTime,
      "rendered video target time should reach the frame-aligned sequence end"
    ).toBeGreaterThanOrEqual(SEQUENCE_END_FRAME_ALIGNED_MIN_US);
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

async function expectedPreviewHostScreenRect(
  page: Page,
  app: { readWindowMetrics: () => Promise<{ contentBounds: { x: number; y: number; width: number; height: number } } | null> }
): Promise<{ x: number; y: number; width: number; height: number }> {
  const metrics = await app.readWindowMetrics();
  if (metrics === null) {
    throw new Error("Window metrics are required to validate native preview surface placement");
  }
  const hostRect = await page.getByLabel("实时预览画面", { exact: true }).evaluate((element) => {
    const box = element.getBoundingClientRect();
    return {
      x: Math.round(box.x),
      y: Math.round(box.y),
      width: Math.round(box.width),
      height: Math.round(box.height)
    };
  });
  return {
    x: metrics.contentBounds.x + hostRect.x,
    y: metrics.contentBounds.y + hostRect.y,
    width: hostRect.width,
    height: hostRect.height
  };
}

async function expectPreviewHostCoversCanvas(page: Page): Promise<void> {
  const rects = await page.evaluate(() => {
    const canvas = document.querySelector<HTMLElement>('[aria-label="预览画面"]');
    const host = document.querySelector<HTMLElement>('[aria-label="实时预览画面"]');
    if (canvas === null || host === null) {
      return null;
    }
    const canvasBox = canvas.getBoundingClientRect();
    const hostBox = host.getBoundingClientRect();
    return {
      canvas: {
        x: Math.round(canvasBox.x),
        y: Math.round(canvasBox.y),
        width: Math.round(canvasBox.width),
        height: Math.round(canvasBox.height)
      },
      host: {
        x: Math.round(hostBox.x),
        y: Math.round(hostBox.y),
        width: Math.round(hostBox.width),
        height: Math.round(hostBox.height)
      }
    };
  });
  expect(rects, "preview canvas and native host must both be present in the product workbench").not.toBeNull();
  expect(
    maxRectDelta(rects?.canvas ?? null, rects?.host ?? null),
    `native preview host must cover the preview canvas DOM region: ${JSON.stringify(rects)}`
  ).toBeLessThanOrEqual(2);
}

function maxRectDelta(
  first: { x: number; y: number; width: number; height: number } | null,
  second: { x: number; y: number; width: number; height: number } | null
): number {
  if (first === null || second === null) {
    return Number.POSITIVE_INFINITY;
  }
  return Math.max(
    Math.abs(first.x - second.x),
    Math.abs(first.y - second.y),
    Math.abs(first.width - second.width),
    Math.abs(first.height - second.height)
  );
}

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
    let movingSegments = await expectTimelineMaterialSegments(page, /p0-moving-testsrc\.mp4/, 1);
    expectTimelineSegmentRange(movingSegments[0], 0, 3_000_000);
    await addVideoTrack(page, app);
    await addMaterialToTimeline(app, page, USER_JOURNEY_OVERLAY_IMAGE);
    let overlaySegments = await expectTimelineMaterialSegments(page, /p0-overlay-testsrc\.png/, 1);
    expectTimelineSegmentRange(overlaySegments[0], 0, 3_000_000);
    await page.getByRole("button", { name: /片段 p0-overlay-testsrc\.png/ }).click();
    const inspector = page.getByRole("complementary", { name: "属性检查器" });
    await expect(page.getByLabel("画面基础表单")).toBeVisible();
    await expect(page.getByLabel("音频参数")).toHaveCount(0);
    await expect(inspector).not.toContainText(/segmentId|trackId|material-workspace|media\/|\/tmp|cache|artifact|diagnostic|诊断/i);
    await updateSelectedVisualThroughInspector(page, app, {
      positionX: -120,
      positionY: -70,
      scaleX: 350,
      scaleY: 350,
      rotation: 0,
      opacity: 760,
      fitMode: "适应"
    });
    await addTextThroughProductPanel(page, app, "产品级端到端字幕");
    await expect(inspector.getByRole("textbox", { name: "文字内容" })).toHaveValue("产品级端到端字幕");
    await expect(page.getByLabel("音频参数")).toHaveCount(0);
    await expect(inspector).not.toContainText(/segmentId|trackId|material-workspace|media\/|\/tmp|cache|artifact|diagnostic|诊断/i);
    await addAudioThroughProductPanel(page, app, USER_JOURNEY_TONE_AUDIO);
    await page.getByRole("button", { name: /片段 p0-tone\.wav/ }).click();
    await page.getByRole("tab", { name: "音频" }).click();
    await expect(page.getByLabel("音频参数")).toBeVisible();
    await expect(page.getByLabel("音频参数").getByRole("button", { name: "应用音频" })).toBeVisible();
    await expect(page.getByLabel("画面基础表单")).toHaveCount(0);
    await expect(inspector).not.toContainText(/segmentId|trackId|material-workspace|media\/|\/tmp|cache|artifact|diagnostic|诊断/i);

    await page.getByRole("button", { name: /片段 p0-moving-testsrc\.mp4/ }).click();
    await page.getByRole("tab", { name: "画面" }).click();
    await expect(page.getByLabel("画面基础表单")).toBeVisible();
    await updateSelectedVisualThroughInspector(page, app);
    await seekTimelinePlayhead(page, app, 500_000);
    await expectTimelineSnappingStatusVisible(page);
    await zoomTimelineIn(page);
    await splitSelectedSegment(page, app, 1_500_000);
    movingSegments = sortTimelineSegments(await expectTimelineMaterialSegments(page, /p0-moving-testsrc\.mp4/, 2));
    expectTimelineSegmentRange(movingSegments[0], 0, 1_500_000);
    expectTimelineSegmentRange(movingSegments[1], 1_500_000, 1_500_000);
    const nextOverlaySelectionCount =
      (await readNativeCommandObservations(app)).filter((call) => call.command === "selectTimelineItemIntent").length + 1;
    await page.getByRole("button", { name: /片段 p0-overlay-testsrc\.png/ }).click();
    await expect
      .poll(
        async () => (await readNativeCommandObservations(app)).filter((call) => call.command === "selectTimelineItemIntent").length,
        { timeout: 30_000 }
      )
      .toBeGreaterThanOrEqual(nextOverlaySelectionCount);
    await moveSelectedSegmentRight(page, app, 250_000);
    overlaySegments = await expectTimelineMaterialSegments(page, /p0-overlay-testsrc\.png/, 1);
    expectTimelineSegmentRange(overlaySegments[0], 250_000, 3_000_000);
    await trimSelectedSegmentLeftEdgeRight(page, app, 100_000);
    overlaySegments = await expectTimelineMaterialSegments(page, /p0-overlay-testsrc\.png/, 1);
    expectTimelineSegmentRange(overlaySegments[0], 350_000, 2_900_000);
    await deleteSelectedSegment(page, app);
    await expectTimelineMaterialSegments(page, /p0-overlay-testsrc\.png/, 0);
    await undoTimelineEdit(page, app);
    overlaySegments = await expectTimelineMaterialSegments(page, /p0-overlay-testsrc\.png/, 1);
    expectTimelineSegmentRange(overlaySegments[0], 350_000, 2_900_000);
    await redoTimelineEdit(page, app);
    await expectTimelineMaterialSegments(page, /p0-overlay-testsrc\.png/, 0);
    await undoTimelineEdit(page, app);
    overlaySegments = await expectTimelineMaterialSegments(page, /p0-overlay-testsrc\.png/, 1);
    expectTimelineSegmentRange(overlaySegments[0], 350_000, 2_900_000);
    await seekTimelinePlayhead(page, app, 2_100_000);

    const callsAfterEdits = await readNativeCommandObservations(app);
    expect(callsAfterEdits.map((call) => call.command)).toEqual(
      expect.arrayContaining([
        "importMaterial",
        "addTimelineSegmentIntent",
        "addTrackIntent",
        "addTextSegmentIntent",
        "addAudioSegmentIntent",
        "updateSelectedSegmentVisual",
        "splitSelectedSegmentIntent",
        "moveSelectedSegmentIntent",
        "trimSelectedSegmentIntent",
        "deleteSelectedSegment",
        "undoTimelineEdit",
        "redoTimelineEdit"
      ])
    );
    expectProductEditCommandsAreSessionOwned(
      await readProjectSessionCalls(app),
      await readDirectNativeCommandObservations(app),
      [
        "importMaterial",
        "addTimelineSegmentIntent",
        "addTrackIntent",
        "updateSelectedSegmentVisual",
        "addTextSegmentIntent",
        "addAudioSegmentIntent",
        "splitSelectedSegmentIntent",
        "moveSelectedSegmentIntent",
        "trimSelectedSegmentIntent",
        "deleteSelectedSegment",
        "undoTimelineEdit",
        "redoTimelineEdit"
      ]
    );
    expect(requestProjectSessionPreviewFrameCount(callsAfterEdits), "product editing matrix must not use artifact preview frames").toBe(0);
    const visualCall = [...callsAfterEdits].reverse().find((call) => call.command === "updateSelectedSegmentVisual");
    expect(visualCall?.visual?.fitMode).toBe("fill");
    expect(visualCall?.visual?.transform.position.x).toBe(120);
    expect(visualCall?.visual?.transform.rotation.degrees).toBe(8);
    expect(visualCall?.visual?.transform.opacity.valueMillis).toBe(820);

    const before = await capturePreviewEvidence(page);
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    await activateProductJourneyApp(app, page);
    await page.getByRole("group", { name: "预览播放控制" }).getByRole("button", { name: "播放预览" }).click();
    await waitForProductPlaybackSuccess(page, app, before, visibleBefore, frameRequestsBeforePlay);
    expectNoProductFallbackCalls(await readRealtimePreviewHostCalls(app));
  } finally {
    await app.close();
  }
});

async function expectTimelineMaterialSegments(page: Page, label: RegExp, count: number) {
  await expect
    .poll(async () => (await readTimelineSegments(page, label)).length, { timeout: 10_000 })
    .toBe(count);
  return readTimelineSegments(page, label);
}

function sortTimelineSegments<T extends { targetStartUs: number }>(segments: T[]): T[] {
  return [...segments].sort((first, second) => first.targetStartUs - second.targetStartUs);
}

function expectTimelineSegmentRange(
  segment: { targetStartUs: number; targetDurationUs: number } | undefined,
  startUs: number,
  durationUs: number,
  toleranceUs = 10_000
): void {
  expect(segment, "expected timeline segment to be visible").toBeDefined();
  expect(Math.abs((segment?.targetStartUs ?? -1) - startUs), "timeline segment target start changed").toBeLessThanOrEqual(
    toleranceUs
  );
  expect(
    Math.abs((segment?.targetDurationUs ?? -1) - durationUs),
    "timeline segment target duration changed"
  ).toBeLessThanOrEqual(toleranceUs);
}

function expectProductEditCommandsAreSessionOwned(
  sessionCalls: Awaited<ReturnType<typeof readProjectSessionCalls>>,
  directNativeObservations: Awaited<ReturnType<typeof readDirectNativeCommandObservations>>,
  intentKinds: readonly string[]
): void {
  const sessionIntentCalls = sessionCalls.filter((call) => call.command === "executeProjectIntent");
  expect(
    sessionIntentCalls,
    "product edits must use Rust-owned project session intents without renderer draft fields"
  ).toEqual(
    expect.arrayContaining(
      intentKinds.map((intentKind) =>
        expect.objectContaining({
          command: "executeProjectIntent",
          intentKind,
          hasDraftField: false
        })
      )
    )
  );

  for (const call of sessionIntentCalls) {
    expect(call.hasDraftField, `session intent ${call.intentKind ?? "<unknown>"} must not carry renderer draft`).toBe(false);
  }

  const semanticKeyGuardedIntentKinds = new Set([
    "addTimelineSegmentIntent",
    "moveSelectedSegmentIntent",
    "splitSelectedSegmentIntent",
    "trimSelectedSegmentIntent",
    "deleteSelectedSegment",
    "addTextSegmentIntent",
    "importSubtitleSrtIntent",
    "addAudioSegmentIntent",
    "addTrackIntent"
  ]);
  for (const call of sessionIntentCalls) {
    if (call.intentKind !== null && semanticKeyGuardedIntentKinds.has(call.intentKind)) {
      expect(
        call.timelineSemanticKeys ?? [],
        `session intent ${call.intentKind} must not carry renderer-owned segment/track/timerange semantic keys`
      ).toEqual([]);
    }
  }

  const forbiddenDirectNativeCommandSet = new Set([
    "addSegment",
    "moveSegment",
    "splitSegment",
    "trimSegment",
    "deleteSegment",
    "addTextSegment",
    "editTextSegment",
    "importSubtitleSrt",
    "addAudioSegment",
    "addTrack",
    "renameTrack",
    "setTrackLock",
    "setTrackVisibility",
    "setTrackMute",
    "updateSegmentVisual",
    "startExport",
    "listMaterials",
    "listMissingMaterials"
  ]);
  const forbiddenDirectNativeCommands = directNativeObservations
    .map((call) => call.command)
    .filter((command) => forbiddenDirectNativeCommandSet.has(command));
  expect(forbiddenDirectNativeCommands, "product edits must not fall back to renderer-owned generic command path").toEqual([]);
}

test("product text and transform interaction UAT supports direct canvas drag", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addTextThroughProductPanel(page, app, "可拖拽文字");

    const textOverlay = page.getByLabel("预览文字");
    await expect(textOverlay).toBeVisible({ timeout: 10_000 });
    const beforeBox = await textOverlay.boundingBox();
    expect(beforeBox, "text overlay must have a visible canvas box before drag").not.toBeNull();

    const commandsBefore = await readNativeCommandObservations(app);
    const visualUpdatesBefore = commandsBefore.filter((call) => call.command === "updateSelectedSegmentVisual").length;
    await page.mouse.move(beforeBox!.x + beforeBox!.width / 2, beforeBox!.y + beforeBox!.height / 2);
    await page.mouse.down();
    await page.mouse.move(beforeBox!.x + beforeBox!.width / 2 + 80, beforeBox!.y + beforeBox!.height / 2 + 36, {
      steps: 8
    });
    await page.mouse.up();

    await expect
      .poll(
        async () => (await readNativeCommandObservations(app)).filter((call) => call.command === "updateSelectedSegmentVisual").length,
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

async function expectNativeAudioContinuity(
  page: Page,
  app: ProductJourneyAppController
): Promise<void> {
  await page.waitForTimeout(5_500);
  const playCall = (await readNativeCommandObservations(app)).findLast((call) => call.command === "playAudioPreview");
  expect(playCall?.sessionId, "native audio continuity requires a real audio preview session").toEqual(expect.any(String));
  expect(playCall?.projectSessionId, "audio preview must be tied to the Rust project session").toEqual(expect.any(String));
  expect(playCall?.expectedRevision, "audio preview must use the Rust project revision").toEqual(expect.any(Number));

  const status = await page.evaluate(async (request) => {
    const core = (window as typeof window & {
      videoEditorCore: {
        getAudioPreviewStatus: (payload: typeof request) => Promise<{
          ok: boolean;
          data: null | {
            status: string;
            device: {
              status: string;
              diagnostics: string[];
            };
          };
        }>;
      };
    }).videoEditorCore;
    return core.getAudioPreviewStatus(request);
  }, {
    sessionId: playCall?.sessionId ?? null,
    projectSessionId: playCall?.projectSessionId ?? null,
    expectedRevision: playCall?.expectedRevision ?? null,
    targetTime: 0
  });
  expect(status.ok, `audio status must be readable: ${JSON.stringify(status)}`).toBe(true);
  expect(status.data?.status, `audio output must still be playing after the old 4s queue window: ${JSON.stringify(status)}`).toBe(
    "playing"
  );
  expect(status.data?.device.status, `native audio device must be ready: ${JSON.stringify(status)}`).toBe("ready");
  expect(status.data?.device.diagnostics ?? []).toEqual(
    expect.arrayContaining([expect.stringContaining("native CPAL output stream is active")])
  );
  const queueDiagnostic = (status.data?.device.diagnostics ?? []).find((diagnostic) =>
    diagnostic.startsWith("native queued samples:")
  );
  expect(queueDiagnostic, `audio status must expose native queue and underrun evidence: ${JSON.stringify(status)}`).toEqual(
    expect.any(String)
  );
  const match = /native queued samples: (\d+); underrun samples: (\d+)/.exec(queueDiagnostic ?? "");
  expect(match, `audio queue diagnostic must be parseable: ${queueDiagnostic}`).not.toBeNull();
  const queuedSamples = Number(match?.[1] ?? 0);
  const underrunSamples = Number(match?.[2] ?? 0);
  expect(queuedSamples, `audio refill must keep samples queued after sustained playback: ${queueDiagnostic}`).toBeGreaterThan(0);
  expect(underrunSamples, `audio output must not underrun during sustained product playback: ${queueDiagnostic}`).toBe(0);
}

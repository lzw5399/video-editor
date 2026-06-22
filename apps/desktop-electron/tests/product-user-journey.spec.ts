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
  zoomTimelineIn,
  type ProductJourneyAppController
} from "./helpers/userJourney";

test.describe.configure({ timeout: 90_000 });

const REPO_ROOT = join(process.cwd(), "../..");
const PHASE15_3_SCREENSHOT_DIR = join(REPO_ROOT, "test-results/phase15-3");
const USER_JOURNEY_SEQUENCE_DURATION_US = 3_000_000;
const THIRTY_FPS_FRAME_DURATION_US = 33_333;
const SEQUENCE_END_FRAME_ALIGNED_MIN_US =
  USER_JOURNEY_SEQUENCE_DURATION_US - THIRTY_FPS_FRAME_DURATION_US - 7_000;
const BUNDLED_SANS_FONT_REF = "font://bundled/noto-sans-cjk-sc-regular";
const BUNDLED_SERIF_FONT_REF = "font://bundled/noto-serif-cjk-sc-regular";
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
    await activateProductJourneyApp(app, page);
    const firstFrameHostImage = await captureVisiblePreviewHostImage(page, app);
    mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "p0-user-portrait-first-frame-before-play.png"),
      firstFrameHostImage
    );
    const firstFrameMetrics = await measurePngPreviewPlacement(page, firstFrameHostImage);
    expectP0NativePreviewPlacement(firstFrameMetrics, "first native preview frame");

    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    await page.getByRole("group", { name: "预览播放控制" }).getByRole("button", { name: "播放预览" }).click();
    const { after } = await waitForProductPlaybackSuccess(page, app, firstFrame, visibleBefore, frameRequestsBeforePlay, 15_000);

    expect(after.hostState?.surfacePlacement?.maxDeltaPx ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(2);
    expect(after.hostState?.contentEvidence?.width).toBeGreaterThan(0);
    expect(after.hostState?.contentEvidence?.height).toBeGreaterThan(0);
    mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
    const playingHostImage = await captureVisiblePreviewHostImage(page, app);
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "p0-user-portrait-native-preview.png"),
      playingHostImage
    );
    expectP0NativePreviewPlacement(await measurePngPreviewPlacement(page, playingHostImage), "playing native preview frame");
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
    await expect
      .poll(async () => (await readRealtimePreviewHostCalls(app)).findIndex((call) => call.kind === "detachSurface"), { timeout: 5_000 })
      .toBeGreaterThanOrEqual(0);
    await expect(page.getByRole("dialog", { name: "导出" })).toBeVisible();
  } finally {
    await app.close();
  }
});

test("product playback keeps native preview synced while resizing larger and smaller", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_LONG_AV_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_LONG_AV_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_LONG_AV_VIDEO);
    await app.resizeMainWindow(1120, 720);
    await expect
      .poll(async () => (await app.readWindowMetrics())?.bounds.width ?? Number.POSITIVE_INFINITY, { timeout: 5_000 })
      .toBeLessThanOrEqual(1120);

    const before = await capturePreviewEvidence(page);
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    const controls = page.getByRole("group", { name: "预览播放控制" });
    await activateProductJourneyApp(app, page);
    await controls.getByRole("button", { name: "播放预览" }).click();

    const { after: playing } = await waitForProductPlaybackSuccess(page, app, before, visibleBefore, frameRequestsBeforePlay);
    const generationBeforeResize = playing.hostState?.playbackGeneration;
    const presentedBeforeResize = playing.hostState?.telemetry?.presentedFrameCount ?? 0;
    expect(generationBeforeResize, "playback must expose a generation before resize").not.toBeNull();
    const hostCallCountBeforeResize = (await readRealtimePreviewHostCalls(app)).length;

    await app.resizeMainWindow(1500, 900);
    await expect
      .poll(async () => (await app.readWindowMetrics())?.bounds.width ?? 0, { timeout: 5_000 })
      .toBeGreaterThanOrEqual(1400);
    await waitForNativePreviewResizeSync(page, app, presentedBeforeResize);
    expectRealtimePreviewResizeDidNotRestartPlayback(
      (await readRealtimePreviewHostCalls(app)).slice(hostCallCountBeforeResize)
    );
    await expectPreviewHostCoversCanvas(page);
    mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
    await page.screenshot({
      path: join(PHASE15_3_SCREENSHOT_DIR, "native-surface-playing-expanded-workspace.png"),
      fullPage: true
    });
    const expandedHostImage = await captureVisiblePreviewHostImage(page, app);
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "native-surface-playing-expanded-host.png"),
      expandedHostImage
    );
    expectLandscapeNativePreviewPlacement(
      await measurePngPreviewPlacement(page, expandedHostImage),
      "expanded playback native preview"
    );

    const beforeNarrow = await capturePreviewEvidence(page);
    const presentedBeforeNarrow = beforeNarrow.hostState?.telemetry?.presentedFrameCount ?? 0;
    const hostCallCountBeforeNarrow = (await readRealtimePreviewHostCalls(app)).length;
    await app.resizeMainWindow(1120, 720);
    await expect
      .poll(async () => (await app.readWindowMetrics())?.bounds.width ?? Number.POSITIVE_INFINITY, { timeout: 5_000 })
      .toBeLessThanOrEqual(1120);
    await waitForNativePreviewResizeSync(page, app, presentedBeforeNarrow);
    expectRealtimePreviewResizeDidNotRestartPlayback(
      (await readRealtimePreviewHostCalls(app)).slice(hostCallCountBeforeNarrow)
    );
    await expectPreviewHostCoversCanvas(page);
    mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
    await page.screenshot({
      path: join(PHASE15_3_SCREENSHOT_DIR, "native-surface-playing-narrow-workspace.png"),
      fullPage: true
    });
    const narrowHostImage = await captureVisiblePreviewHostImage(page, app);
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "native-surface-playing-narrow-host.png"),
      narrowHostImage
    );
    expectLandscapeNativePreviewPlacement(
      await measurePngPreviewPlacement(page, narrowHostImage),
      "narrow playback native preview"
    );

    expect(requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app))).toBe(frameRequestsBeforePlay);
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

test("P0 user portrait material supports real text and subtitle native overlay editing", async () => {
  test.skip(
    !existsSync(P0_USER_PORTRAIT_MATERIAL),
    `P0 user material not present at ${P0_USER_PORTRAIT_MATERIAL}; set VIDEO_EDITOR_P0_USER_MATERIAL to run this local regression`
  );

  const { app, page } = await launchProductJourneyApp([P0_USER_PORTRAIT_MATERIAL]);
  const p0Srt = "1\n00:00:00,000 --> 00:00:02,000\n真实素材字幕\nPortrait 验证\n";

  try {
    await importMaterialThroughProductPicker(app, page, P0_USER_PORTRAIT_MATERIAL);
    await dragMaterialToTimeline(app, page, P0_USER_PORTRAIT_MATERIAL);

    await addTextThroughProductPanel(page, app, "真实素材标题 初稿");
    await editSelectedTextThroughInspector(page, app, {
      content: "真实素材标题\nSans 编辑",
      fontFamily: "Noto Sans CJK SC",
      fontSize: 44,
      color: "#3dff93",
      alignment: "center",
      textBoxWidthMillis: 780,
      textBoxHeightMillis: 180,
      layoutXMillis: 90,
      layoutYMillis: 120,
      layoutWidthMillis: 800,
      layoutHeightMillis: 220,
      lineHeightMillis: 1150,
      letterSpacingMillis: 50
    });
    const titleDragVisual = await dragSelectedPreviewTextOverlay(page, app, "真实素材标题\nSans 编辑", 42, 28);
    await updateSelectedVisualThroughInspector(page, app, {
      positionX: titleDragVisual.transform.position.x,
      positionY: titleDragVisual.transform.position.y,
      scaleX: 1040,
      scaleY: 1040,
      rotation: -7,
      opacity: 930,
      fitMode: "适应"
    });

    await importSubtitleSrtThroughProductPanel(page, app, p0Srt);
    await page.getByRole("button", { name: /片段 真实素材字幕/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "真实素材字幕\nPortrait 验证",
      fontFamily: "Noto Serif CJK SC",
      fontSize: 34,
      color: "#ffcf42",
      alignment: "center",
      textBoxWidthMillis: 780,
      textBoxHeightMillis: 160,
      layoutXMillis: 110,
      layoutYMillis: 710,
      layoutWidthMillis: 780,
      layoutHeightMillis: 190,
      lineHeightMillis: 1200,
      letterSpacingMillis: 70
    });

    await page.getByRole("button", { name: "选择轨道 视频轨道 1" }).click();
    await expect(page.locator(".preview-text-overlay"), "P0 text overlay evidence must come from native preview").toHaveCount(0);
    const evidence = await waitForActiveTextOverlaySetEvidence(
      page,
      app,
      ["真实素材标题\nSans 编辑", "真实素材字幕\nPortrait 验证"],
      0,
      {
        exactOverlayCount: 2,
        forbiddenContents: ["真实素材标题 初稿"]
      }
    );
    mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
    await page.screenshot({
      path: join(PHASE15_3_SCREENSHOT_DIR, "p0-user-portrait-text-subtitle-workspace.png"),
      fullPage: true
    });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "p0-user-portrait-text-subtitle-host.png"),
      evidence.hostImage
    );

    expect(evidence.previewEvidence.hostState?.productReady, "P0 text regression must use product-ready native preview").toBe(true);
    expect(evidence.previewEvidence.hostState?.fallbackActive, "P0 text regression must not use fallback preview").toBe(false);
    expect(evidence.previewEvidence.hostState?.backend, "P0 text regression backend").toBe("renderGraphGpu");
    expect(evidence.previewEvidence.hostState?.contentEvidence?.source, "P0 text regression content source").toBe("renderGraphGpuComposited");
    expect(evidence.previewEvidence.hostState?.surfacePlacement?.maxDeltaPx ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(2);
    await expectPreviewHostCoversCanvas(page);
    expectP0NativePreviewPlacement(await measurePngPreviewPlacement(page, evidence.hostImage), "P0 text/subtitle native preview");
    const contentEvidence = evidence.previewEvidence.hostState?.contentEvidence;
    const titleOverlay = overlayByContent(evidence.activeTextOverlays, "真实素材标题\nSans 编辑");
    const subtitleOverlay = overlayByContent(evidence.activeTextOverlays, "真实素材字幕\nPortrait 验证");
    expect(titleOverlay.fontRef).toBe(BUNDLED_SANS_FONT_REF);
    expect(titleOverlay.visualRotationDegrees).toBe(-7);
    expect(subtitleOverlay.source).toBe("subtitle");
    expect(subtitleOverlay.fontRef).toBe(BUNDLED_SERIF_FONT_REF);
    expect(subtitleOverlay.y).toBeGreaterThan(titleOverlay.y + titleOverlay.height);
    await expectTextOverlayPixelsInNativeHost(page, evidence.hostImage, contentEvidence, titleOverlay, "P0 portrait title");
    await expectTextOverlayPixelsInNativeHost(page, evidence.hostImage, contentEvidence, subtitleOverlay, "P0 portrait subtitle");

    const calls = await readNativeCommandObservations(app);
    expect(calls.filter((call) => call.command === "importSubtitleSrtIntent")).toHaveLength(1);
    expect(calls.filter((call) => call.command === "editSelectedText").length).toBeGreaterThanOrEqual(2);
    expect(calls.filter((call) => call.command === "updateSelectedSegmentVisual").length).toBeGreaterThanOrEqual(2);
    expect(requestProjectSessionPreviewFrameCount(calls), "P0 text regression must not request artifact preview frames").toBe(0);
    expectNoProductFallbackCalls(await readRealtimePreviewHostCalls(app));
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

    await page.getByRole("button", { name: "选择轨道 视频轨道 1" }).click();
    await expect(page.getByLabel("预览选中框"), "combo native host screenshots must not be satisfied by edit overlay chrome").toHaveCount(0);
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
    await page.screenshot({
      path: join(PHASE15_3_SCREENSHOT_DIR, "combo-preview-first-subtitle-workspace.png"),
      fullPage: true
    });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "combo-preview-first-subtitle.png"),
      firstSubtitleEvidence.hostImage
    );
    await expectComboSubtitleNativeEvidence(page, app, firstSubtitleEvidence, "first subtitle");
    const { after } = await waitForProductPlaybackSuccess(page, app, before, visibleBefore, frameRequestsBeforePlay);
    const secondSubtitleEvidence = await waitForActiveSubtitleEvidence(page, app, "第二条组合字幕", 2_000_000);
    await page.screenshot({
      path: join(PHASE15_3_SCREENSHOT_DIR, "combo-preview-second-subtitle-workspace.png"),
      fullPage: true
    });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "combo-preview-second-subtitle.png"),
      secondSubtitleEvidence.hostImage
    );
    await expectComboSubtitleNativeEvidence(page, app, secondSubtitleEvidence, "second subtitle");

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
    expect(
      firstSubtitleEvidence.hostImage.equals(secondSubtitleEvidence.hostImage),
      "native host pixels must change between the first and second subtitle cues"
    ).toBe(false);
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

test("product text and subtitle editing UAT covers multi-font multi-track native preview evidence", async () => {
  const { app, page } = await launchProductJourneyApp([
    USER_JOURNEY_LONG_AV_VIDEO,
    USER_JOURNEY_LONG_TONE_AUDIO
  ]);
  const firstSubtitleTrackSrt =
    "1\n00:00:00,000 --> 00:00:03,000\n并行字幕 A\n\n2\n00:00:03,000 --> 00:00:05,000\n后续字幕 A\n";
  const secondSubtitleTrackSrt =
    "1\n00:00:00,000 --> 00:00:03,000\n并行字幕 B\n\n2\n00:00:03,000 --> 00:00:05,000\n后续字幕 B\n";

  try {
    await importMaterialsThroughProductPicker(app, page, [USER_JOURNEY_LONG_AV_VIDEO, USER_JOURNEY_LONG_TONE_AUDIO]);
    await addMaterialToTimeline(app, page, USER_JOURNEY_LONG_AV_VIDEO);
    await addAudioThroughProductPanel(page, app, USER_JOURNEY_LONG_TONE_AUDIO, 8_000_000);

    await addTextThroughProductPanel(page, app, "多字体主标题");
    await editSelectedTextThroughInspector(page, app, {
      content: "多字体主标题 已编辑",
      fontFamily: "Noto Sans CJK SC",
      fontSize: 48,
      color: "#40ff80",
      alignment: "left",
      textBoxWidthMillis: 760,
      textBoxHeightMillis: 180,
      layoutXMillis: 80,
      layoutYMillis: 90,
      layoutWidthMillis: 820,
      layoutHeightMillis: 240,
      lineHeightMillis: 1150,
      letterSpacingMillis: 80
    });
    await updateSelectedVisualThroughInspector(page, app, {
      positionX: -80,
      positionY: 40,
      scaleX: 1000,
      scaleY: 1000,
      rotation: -8,
      opacity: 940,
      fitMode: "适应"
    });

    await importSubtitleSrtThroughProductPanel(page, app, firstSubtitleTrackSrt);
    await page.getByRole("button", { name: /片段 并行字幕 A/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "并行字幕 A 已校对",
      fontFamily: "Noto Sans CJK SC",
      fontSize: 36,
      color: "#ffd21f",
      alignment: "center",
      textBoxWidthMillis: 780,
      textBoxHeightMillis: 150,
      layoutXMillis: 110,
      layoutYMillis: 610,
      layoutWidthMillis: 780,
      layoutHeightMillis: 180,
      lineHeightMillis: 1200,
      letterSpacingMillis: 0
    });
    await page.getByRole("button", { name: /片段 后续字幕 A/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "后续字幕 A 已换字",
      fontFamily: "Noto Serif CJK SC",
      fontSize: 34,
      color: "#00e5ff",
      alignment: "center",
      textBoxWidthMillis: 760,
      textBoxHeightMillis: 150,
      layoutXMillis: 120,
      layoutYMillis: 620,
      layoutWidthMillis: 760,
      layoutHeightMillis: 180,
      lineHeightMillis: 1180,
      letterSpacingMillis: 40
    });

    await addRenamedSubtitleTrack(page, app, "字幕轨道 2");
    await importSubtitleSrtThroughProductPanel(page, app, secondSubtitleTrackSrt);
    await page.getByRole("button", { name: /片段 并行字幕 B/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "并行字幕 B 已旋转",
      fontFamily: "Noto Serif CJK SC",
      fontSize: 40,
      color: "#ff4fd8",
      alignment: "right",
      textBoxWidthMillis: 760,
      textBoxHeightMillis: 160,
      layoutXMillis: 90,
      layoutYMillis: 760,
      layoutWidthMillis: 820,
      layoutHeightMillis: 200,
      lineHeightMillis: 1250,
      letterSpacingMillis: 120
    });
    await updateSelectedVisualThroughInspector(page, app, {
      positionX: 120,
      positionY: -40,
      scaleX: 1100,
      scaleY: 1100,
      rotation: 14,
      opacity: 870,
      fitMode: "适应"
    });
    await page.getByRole("button", { name: /片段 后续字幕 B/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "后续字幕 B 已右移",
      fontFamily: "Noto Sans CJK SC",
      fontSize: 32,
      color: "#ffffff",
      alignment: "right",
      textBoxWidthMillis: 760,
      textBoxHeightMillis: 150,
      layoutXMillis: 150,
      layoutYMillis: 760,
      layoutWidthMillis: 760,
      layoutHeightMillis: 180,
      lineHeightMillis: 1220,
      letterSpacingMillis: 90
    });

    await page.getByRole("button", { name: "选择轨道 视频轨道 1" }).click();
    await expect(page.getByLabel("预览选中框"), "native text editing evidence must not be satisfied by edit chrome").toHaveCount(0);
    await expect(page.locator(".preview-text-overlay"), "native text editing evidence must not be satisfied by DOM text overlays").toHaveCount(0);
    await seekTimelinePlayhead(page, app, 500_000);
    const sameTimeEvidence = await waitForActiveTextOverlaySetEvidence(
      page,
      app,
      ["多字体主标题 已编辑", "并行字幕 A 已校对", "并行字幕 B 已旋转"],
      0,
      2_900_000
    );
    mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
    await page.screenshot({
      path: join(PHASE15_3_SCREENSHOT_DIR, "text-subtitle-editing-same-time-workspace.png"),
      fullPage: true
    });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "text-subtitle-editing-same-time-host.png"),
      sameTimeEvidence.hostImage
    );
    await expectTextEditingNativeEvidence(page, app, sameTimeEvidence, "same-time text/subtitle matrix");
    const titleOverlay = overlayByContent(sameTimeEvidence.activeTextOverlays, "多字体主标题 已编辑");
    const firstSubtitleOverlay = overlayByContent(sameTimeEvidence.activeTextOverlays, "并行字幕 A 已校对");
    const secondSubtitleOverlay = overlayByContent(sameTimeEvidence.activeTextOverlays, "并行字幕 B 已旋转");
    expect(titleOverlay.fontFamily, "title font family should come from inspector edit").toBe("Noto Sans CJK SC");
    expect(titleOverlay.fontRef).toBe(BUNDLED_SANS_FONT_REF);
    expect(titleOverlay.color).toBe("#40ff80");
    expect(titleOverlay.fontSize).toBe(48);
    expect(titleOverlay.alignment).toBe("left");
    expect(titleOverlay.letterSpacingMillis).toBe(80);
    expect(titleOverlay.visualRotationDegrees).toBe(-8);
    expect(titleOverlay.visualPositionX).toBe(-80);
    expect(firstSubtitleOverlay.fontFamily).toBe("Noto Sans CJK SC");
    expect(firstSubtitleOverlay.fontRef).toBe(BUNDLED_SANS_FONT_REF);
    expect(firstSubtitleOverlay.color).toBe("#ffd21f");
    expect(firstSubtitleOverlay.fontSize).toBe(36);
    expect(firstSubtitleOverlay.y, "first subtitle should render below title").toBeGreaterThan(titleOverlay.y + titleOverlay.height);
    expect(secondSubtitleOverlay.fontFamily).toBe("Noto Serif CJK SC");
    expect(secondSubtitleOverlay.fontRef).toBe(BUNDLED_SERIF_FONT_REF);
    expect(secondSubtitleOverlay.color).toBe("#ff4fd8");
    expect(secondSubtitleOverlay.fontSize).toBe(40);
    expect(secondSubtitleOverlay.alignment).toBe("right");
    expect(secondSubtitleOverlay.letterSpacingMillis).toBe(120);
    expect(secondSubtitleOverlay.visualRotationDegrees).toBe(14);
    expect(secondSubtitleOverlay.visualPositionX).toBe(120);
    expect(secondSubtitleOverlay.visualPositionY).toBe(-40);
    expect(secondSubtitleOverlay.visualScaleXMillis).toBe(1100);
    expect(secondSubtitleOverlay.visualOpacityMillis).toBe(870);
    expect(firstSubtitleOverlay.y, "same-time subtitle tracks must not share the same bbox").not.toBe(secondSubtitleOverlay.y);

    const before = sameTimeEvidence.previewEvidence;
    const visibleBefore = await captureVisiblePreviewEvidence(page, app);
    const frameRequestsBeforePlay = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));
    await activateProductJourneyApp(app, page);
    await page.getByRole("group", { name: "预览播放控制" }).getByRole("button", { name: "播放预览" }).click();
    await waitForProductPlaybackSuccess(page, app, before, visibleBefore, frameRequestsBeforePlay);
    const controls = page.getByRole("group", { name: "预览播放控制" });
    await controls.getByRole("button", { name: "暂停预览" }).click();
    await expect(controls.getByRole("button", { name: "播放预览" })).toBeEnabled();
    await seekTimelinePlayhead(page, app, 3_200_000);
    const laterEvidence = await waitForActiveTextOverlaySetEvidence(page, app, ["后续字幕 A 已换字", "后续字幕 B 已右移"], 3_000_000, 4_900_000);
    await page.screenshot({
      path: join(PHASE15_3_SCREENSHOT_DIR, "text-subtitle-editing-later-cues-workspace.png"),
      fullPage: true
    });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "text-subtitle-editing-later-cues-host.png"),
      laterEvidence.hostImage
    );
    await expectTextEditingNativeEvidence(page, app, laterEvidence, "later subtitle cues");
    expect(laterEvidence.activeTextOverlays.some((overlay) => textContentMatches(overlay.content, "并行字幕 A 已校对"))).toBe(false);
    expect(laterEvidence.activeTextOverlays.some((overlay) => textContentMatches(overlay.content, "并行字幕 B 已旋转"))).toBe(false);
    const laterSubtitleA = overlayByContent(laterEvidence.activeTextOverlays, "后续字幕 A 已换字");
    const laterSubtitleB = overlayByContent(laterEvidence.activeTextOverlays, "后续字幕 B 已右移");
    expect(laterSubtitleA.fontFamily).toBe("Noto Serif CJK SC");
    expect(laterSubtitleA.fontRef).toBe(BUNDLED_SERIF_FONT_REF);
    expect(laterSubtitleA.color).toBe("#00e5ff");
    expect(laterSubtitleA.letterSpacingMillis).toBe(40);
    expect(laterSubtitleB.fontFamily).toBe("Noto Sans CJK SC");
    expect(laterSubtitleB.fontRef).toBe(BUNDLED_SANS_FONT_REF);
    expect(laterSubtitleB.color).toBe("#ffffff");
    expect(laterSubtitleB.letterSpacingMillis).toBe(90);
    const calls = await readNativeCommandObservations(app);
    expect(calls.filter((call) => call.command === "importSubtitleSrtIntent")).toHaveLength(2);
    expect(calls.filter((call) => call.command === "editSelectedText").length).toBeGreaterThanOrEqual(5);
    expect(calls.filter((call) => call.command === "updateSelectedSegmentVisual").length).toBeGreaterThanOrEqual(2);
    expect(requestProjectSessionPreviewFrameCount(calls), "text editing matrix must not request artifact preview frames").toBe(
      frameRequestsBeforePlay
    );
    expectProductEditCommandsAreSessionOwned(
      await readProjectSessionCalls(app),
      await readDirectNativeCommandObservations(app),
      ["addTextSegmentIntent", "importSubtitleSrtIntent", "editSelectedText", "updateSelectedSegmentVisual", "addTrackIntent", "renameSelectedTrack"]
    );
    expectNoProductFallbackCalls(await readRealtimePreviewHostCalls(app));
  } finally {
    await app.close();
  }
});

test("product text editing UAT exercises preview drag, multi-font captions, and staggered subtitle tracks", async () => {
  const { app, page } = await launchProductJourneyApp([
    USER_JOURNEY_LONG_AV_VIDEO,
    USER_JOURNEY_LONG_TONE_AUDIO
  ]);
  const firstTrackSrt =
    "1\n00:00:00,000 --> 00:00:01,400\n同屏字幕甲\n\n2\n00:00:01,400 --> 00:00:02,800\n错峰字幕甲\n\n3\n00:00:03,200 --> 00:00:04,800\n尾部字幕甲\n";
  const secondTrackSrt =
    "1\n00:00:00,000 --> 00:00:01,400\n同屏字幕乙\n\n2\n00:00:01,400 --> 00:00:02,800\n错峰字幕乙\n";
  const thirdTrackSrt =
    "1\n00:00:00,000 --> 00:00:01,400\n同屏字幕丙\n\n2\n00:00:03,200 --> 00:00:04,800\n尾部字幕丙\n";

  try {
    await importMaterialsThroughProductPicker(app, page, [USER_JOURNEY_LONG_AV_VIDEO, USER_JOURNEY_LONG_TONE_AUDIO]);
    await addMaterialToTimeline(app, page, USER_JOURNEY_LONG_AV_VIDEO);
    await addAudioThroughProductPanel(page, app, USER_JOURNEY_LONG_TONE_AUDIO, 8_000_000);

    await addTextThroughProductPanel(page, app, "预览拖动标题");
    await editSelectedTextThroughInspector(page, app, {
      content: "预览拖动标题 Sans",
      fontFamily: "Noto Sans CJK SC",
      fontSize: 44,
      color: "#62ff9a",
      alignment: "left",
      textBoxWidthMillis: 720,
      textBoxHeightMillis: 170,
      layoutXMillis: 70,
      layoutYMillis: 80,
      layoutWidthMillis: 780,
      layoutHeightMillis: 210,
      lineHeightMillis: 1120,
      letterSpacingMillis: 60
    });
    const titleDragVisual = await dragSelectedPreviewTextOverlay(page, app, "预览拖动标题 Sans", 72, 34);
    await updateSelectedVisualThroughInspector(page, app, {
      positionX: titleDragVisual.transform.position.x,
      positionY: titleDragVisual.transform.position.y,
      scaleX: 1060,
      scaleY: 980,
      rotation: -12,
      opacity: 910,
      fitMode: "适应"
    });

    await importSubtitleSrtThroughProductPanel(page, app, firstTrackSrt);
    await page.getByRole("button", { name: /片段 同屏字幕甲/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "同屏字幕甲 Serif",
      fontFamily: "Noto Serif CJK SC",
      fontSize: 34,
      color: "#ffbf47",
      alignment: "center",
      textBoxWidthMillis: 780,
      textBoxHeightMillis: 140,
      layoutXMillis: 110,
      layoutYMillis: 545,
      layoutWidthMillis: 780,
      layoutHeightMillis: 170,
      lineHeightMillis: 1180,
      letterSpacingMillis: 20
    });
    await page.getByRole("button", { name: /片段 错峰字幕甲/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "错峰字幕甲 Sans",
      fontFamily: "Noto Sans CJK SC",
      fontSize: 35,
      color: "#37dcff",
      alignment: "center",
      textBoxWidthMillis: 760,
      textBoxHeightMillis: 140,
      layoutXMillis: 120,
      layoutYMillis: 620,
      layoutWidthMillis: 760,
      layoutHeightMillis: 170,
      lineHeightMillis: 1180,
      letterSpacingMillis: 70
    });
    await page.getByRole("button", { name: /片段 尾部字幕甲/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "尾部字幕甲 Serif",
      fontFamily: "Noto Serif CJK SC",
      fontSize: 36,
      color: "#ffe66d",
      alignment: "left",
      textBoxWidthMillis: 760,
      textBoxHeightMillis: 150,
      layoutXMillis: 90,
      layoutYMillis: 700,
      layoutWidthMillis: 760,
      layoutHeightMillis: 180,
      lineHeightMillis: 1210,
      letterSpacingMillis: 40
    });

    await addRenamedSubtitleTrack(page, app, "字幕轨道 2");
    await importSubtitleSrtThroughProductPanel(page, app, secondTrackSrt);
    await page.getByRole("button", { name: /片段 同屏字幕乙/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "同屏字幕乙 预览拖动",
      fontFamily: "Noto Sans CJK SC",
      fontSize: 33,
      color: "#ff5edb",
      alignment: "right",
      textBoxWidthMillis: 780,
      textBoxHeightMillis: 145,
      layoutXMillis: 105,
      layoutYMillis: 660,
      layoutWidthMillis: 800,
      layoutHeightMillis: 175,
      lineHeightMillis: 1200,
      letterSpacingMillis: 100
    });
    const subtitleDragVisual = await dragSelectedPreviewTextOverlay(page, app, "同屏字幕乙 预览拖动", -54, 42);
    await updateSelectedVisualThroughInspector(page, app, {
      positionX: subtitleDragVisual.transform.position.x,
      positionY: subtitleDragVisual.transform.position.y,
      scaleX: 1120,
      scaleY: 1080,
      rotation: 16,
      opacity: 850,
      fitMode: "适应"
    });
    await page.getByRole("button", { name: /片段 错峰字幕乙/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "错峰字幕乙 Serif",
      fontFamily: "Noto Serif CJK SC",
      fontSize: 36,
      color: "#d6ff43",
      alignment: "right",
      textBoxWidthMillis: 780,
      textBoxHeightMillis: 150,
      layoutXMillis: 110,
      layoutYMillis: 730,
      layoutWidthMillis: 780,
      layoutHeightMillis: 180,
      lineHeightMillis: 1240,
      letterSpacingMillis: 110
    });

    await addRenamedSubtitleTrack(page, app, "字幕轨道 3");
    await importSubtitleSrtThroughProductPanel(page, app, thirdTrackSrt);
    await page.getByRole("button", { name: /片段 同屏字幕丙/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "同屏字幕丙 右侧",
      fontFamily: "Noto Serif CJK SC",
      fontSize: 32,
      color: "#b8ff3d",
      alignment: "right",
      textBoxWidthMillis: 760,
      textBoxHeightMillis: 140,
      layoutXMillis: 125,
      layoutYMillis: 790,
      layoutWidthMillis: 760,
      layoutHeightMillis: 170,
      lineHeightMillis: 1200,
      letterSpacingMillis: 130
    });
    await page.getByRole("button", { name: /片段 尾部字幕丙/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "尾部字幕丙 Sans",
      fontFamily: "Noto Sans CJK SC",
      fontSize: 34,
      color: "#ffffff",
      alignment: "center",
      textBoxWidthMillis: 760,
      textBoxHeightMillis: 150,
      layoutXMillis: 115,
      layoutYMillis: 610,
      layoutWidthMillis: 780,
      layoutHeightMillis: 180,
      lineHeightMillis: 1200,
      letterSpacingMillis: 50
    });

    await page.getByRole("button", { name: "选择轨道 视频轨道 1" }).click();
    await expect(page.getByLabel("预览选中框"), "native text matrix must not be satisfied by edit selection chrome").toHaveCount(0);
    await expect(page.locator(".preview-text-overlay"), "native text matrix must not be satisfied by DOM text overlays").toHaveCount(0);

    await seekTimelinePlayhead(page, app, 500_000);
    const sameTimeEvidence = await waitForActiveTextOverlaySetEvidence(
      page,
      app,
      ["预览拖动标题 Sans", "同屏字幕甲 Serif", "同屏字幕乙 预览拖动", "同屏字幕丙 右侧"],
      0,
      {
        maxTargetTimeUs: 1_350_000,
        exactOverlayCount: 4,
        forbiddenContents: ["错峰字幕甲 Sans", "错峰字幕乙 Serif", "尾部字幕甲 Serif", "尾部字幕丙 Sans"]
      }
    );
    mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
    await page.screenshot({
      path: join(PHASE15_3_SCREENSHOT_DIR, "text-editing-preview-drag-same-time-workspace.png"),
      fullPage: true
    });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "text-editing-preview-drag-same-time-host.png"),
      sameTimeEvidence.hostImage
    );
    await expectTextEditingNativeEvidence(page, app, sameTimeEvidence, "preview drag same-time subtitle matrix");
    const draggedTitleOverlay = overlayByContent(sameTimeEvidence.activeTextOverlays, "预览拖动标题 Sans");
    const firstSameTimeOverlay = overlayByContent(sameTimeEvidence.activeTextOverlays, "同屏字幕甲 Serif");
    const draggedSubtitleOverlay = overlayByContent(sameTimeEvidence.activeTextOverlays, "同屏字幕乙 预览拖动");
    const thirdSameTimeOverlay = overlayByContent(sameTimeEvidence.activeTextOverlays, "同屏字幕丙 右侧");
    expect(draggedTitleOverlay.fontRef).toBe(BUNDLED_SANS_FONT_REF);
    expect(draggedTitleOverlay.visualPositionX).toBe(titleDragVisual.transform.position.x);
    expect(draggedTitleOverlay.visualPositionY).toBe(titleDragVisual.transform.position.y);
    expect(draggedTitleOverlay.visualRotationDegrees).toBe(-12);
    expect(draggedTitleOverlay.visualScaleXMillis).toBe(1060);
    expect(firstSameTimeOverlay.fontRef).toBe(BUNDLED_SERIF_FONT_REF);
    expect(firstSameTimeOverlay.color).toBe("#ffbf47");
    expect(draggedSubtitleOverlay.fontRef).toBe(BUNDLED_SANS_FONT_REF);
    expect(draggedSubtitleOverlay.visualPositionX).toBe(subtitleDragVisual.transform.position.x);
    expect(draggedSubtitleOverlay.visualPositionY).toBe(subtitleDragVisual.transform.position.y);
    expect(draggedSubtitleOverlay.visualRotationDegrees).toBe(16);
    expect(draggedSubtitleOverlay.visualScaleXMillis).toBe(1120);
    expect(thirdSameTimeOverlay.fontRef).toBe(BUNDLED_SERIF_FONT_REF);
    expect(new Set([
      firstSameTimeOverlay.y,
      draggedSubtitleOverlay.y,
      thirdSameTimeOverlay.y
    ]).size, "same-time subtitle tracks should occupy distinct render bboxes").toBe(3);

    await seekTimelinePlayhead(page, app, 1_650_000);
    const staggeredEvidence = await waitForActiveTextOverlaySetEvidence(
      page,
      app,
      ["预览拖动标题 Sans", "错峰字幕甲 Sans", "错峰字幕乙 Serif"],
      1_400_000,
      {
        maxTargetTimeUs: 2_750_000,
        exactOverlayCount: 3,
        forbiddenContents: ["同屏字幕甲 Serif", "同屏字幕乙 预览拖动", "同屏字幕丙 右侧", "尾部字幕甲 Serif"]
      }
    );
    await page.screenshot({
      path: join(PHASE15_3_SCREENSHOT_DIR, "text-editing-preview-drag-staggered-workspace.png"),
      fullPage: true
    });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "text-editing-preview-drag-staggered-host.png"),
      staggeredEvidence.hostImage
    );
    await expectTextEditingNativeEvidence(page, app, staggeredEvidence, "preview drag staggered subtitle matrix");
    expect(staggeredEvidence.activeTextOverlays.some((overlay) => textContentMatches(overlay.content, "同屏字幕甲 Serif"))).toBe(false);
    expect(staggeredEvidence.activeTextOverlays.some((overlay) => textContentMatches(overlay.content, "同屏字幕乙 预览拖动"))).toBe(false);
    expect(overlayByContent(staggeredEvidence.activeTextOverlays, "错峰字幕甲 Sans").fontRef).toBe(BUNDLED_SANS_FONT_REF);
    expect(overlayByContent(staggeredEvidence.activeTextOverlays, "错峰字幕乙 Serif").fontRef).toBe(BUNDLED_SERIF_FONT_REF);

    await seekTimelinePlayhead(page, app, 3_450_000);
    const tailEvidence = await waitForActiveTextOverlaySetEvidence(
      page,
      app,
      ["尾部字幕甲 Serif", "尾部字幕丙 Sans"],
      3_200_000,
      {
        maxTargetTimeUs: 4_750_000,
        exactOverlayCount: 2,
        forbiddenContents: ["预览拖动标题 Sans", "错峰字幕甲 Sans", "错峰字幕乙 Serif"]
      }
    );
    await expectTextEditingNativeEvidence(page, app, tailEvidence, "preview drag tail subtitle matrix");
    expect(tailEvidence.activeTextOverlays.some((overlay) => textContentMatches(overlay.content, "错峰字幕甲 Sans"))).toBe(false);
    expect(overlayByContent(tailEvidence.activeTextOverlays, "尾部字幕甲 Serif").fontRef).toBe(BUNDLED_SERIF_FONT_REF);
    expect(overlayByContent(tailEvidence.activeTextOverlays, "尾部字幕丙 Sans").fontRef).toBe(BUNDLED_SANS_FONT_REF);

    const calls = await readNativeCommandObservations(app);
    expect(calls.filter((call) => call.command === "importSubtitleSrtIntent")).toHaveLength(3);
    expect(calls.filter((call) => call.command === "editSelectedText").length).toBeGreaterThanOrEqual(8);
    expect(calls.filter((call) => call.command === "updateSelectedSegmentVisual").length).toBeGreaterThanOrEqual(4);
    expect(requestProjectSessionPreviewFrameCount(calls), "text editing matrix must not request artifact preview frames").toBe(0);
    expectProductEditCommandsAreSessionOwned(
      await readProjectSessionCalls(app),
      await readDirectNativeCommandObservations(app),
      ["addTextSegmentIntent", "importSubtitleSrtIntent", "editSelectedText", "updateSelectedSegmentVisual", "addTrackIntent", "renameSelectedTrack"]
    );
    expectNoProductFallbackCalls(await readRealtimePreviewHostCalls(app));
  } finally {
    await app.close();
  }
});

test("product text editing UAT covers repeated font switching, multiline copy, layered text, and timed subtitles", async () => {
  const { app, page } = await launchProductJourneyApp([
    USER_JOURNEY_LONG_AV_VIDEO,
    USER_JOURNEY_LONG_TONE_AUDIO
  ]);
  const firstTrackSrt =
    "1\n00:00:00,000 --> 00:00:01,800\n同屏字幕 A 第一行\n真实示例 A\n\n2\n00:00:01,800 --> 00:00:03,600\n错峰字幕 A 初稿\n";
  const secondTrackSrt =
    "1\n00:00:00,000 --> 00:00:01,800\n同屏字幕 B 第一行\nLaunch 2026\n\n2\n00:00:01,800 --> 00:00:03,600\n错峰字幕 B 初稿\n";

  try {
    await importMaterialsThroughProductPicker(app, page, [USER_JOURNEY_LONG_AV_VIDEO, USER_JOURNEY_LONG_TONE_AUDIO]);
    await addMaterialToTimeline(app, page, USER_JOURNEY_LONG_AV_VIDEO);
    await addAudioThroughProductPanel(page, app, USER_JOURNEY_LONG_TONE_AUDIO, 8_000_000);

    await addTextThroughProductPanel(page, app, "真实项目标题 初稿");
    await editSelectedTextThroughInspector(page, app, {
      content: "真实案例标题\nSans 初版",
      fontFamily: "Noto Sans CJK SC",
      fontSize: 46,
      color: "#52ff9f",
      alignment: "left",
      textBoxWidthMillis: 780,
      textBoxHeightMillis: 190,
      layoutXMillis: 70,
      layoutYMillis: 80,
      layoutWidthMillis: 820,
      layoutHeightMillis: 240,
      lineHeightMillis: 1120,
      letterSpacingMillis: 50
    });
    const initialTitleEvidence = await waitForActiveTextOverlaySetEvidence(page, app, ["真实案例标题\nSans 初版"], 0, {
      exactOverlayCount: 1,
      forbiddenContents: ["真实项目标题 初稿"]
    });
    const titleDragVisual = await dragSelectedPreviewTextOverlay(page, app, "真实案例标题\nSans 初版", 64, -30);

    await editSelectedTextThroughInspector(page, app, {
      content: "真实案例标题\nSerif 二次编辑",
      fontFamily: "Noto Serif CJK SC",
      fontSize: 45,
      color: "#c084ff",
      alignment: "center",
      textBoxWidthMillis: 800,
      textBoxHeightMillis: 190,
      layoutXMillis: 90,
      layoutYMillis: 90,
      layoutWidthMillis: 800,
      layoutHeightMillis: 230,
      lineHeightMillis: 1160,
      letterSpacingMillis: 95
    });
    const serifTitleEvidence = await waitForActiveTextOverlaySetEvidence(page, app, ["真实案例标题\nSerif 二次编辑"], 0, {
      exactOverlayCount: 1,
      forbiddenContents: ["真实案例标题\nSans 初版"]
    });
    expect(
      serifTitleEvidence.hostImage.equals(initialTitleEvidence.hostImage),
      "native host pixels must change after switching title content and font"
    ).toBe(false);
    const serifTitleOverlay = overlayByContent(serifTitleEvidence.activeTextOverlays, "真实案例标题\nSerif 二次编辑");
    expect(serifTitleOverlay.fontRef).toBe(BUNDLED_SERIF_FONT_REF);
    expect(serifTitleOverlay.visualPositionX).toBe(titleDragVisual.transform.position.x);
    expect(serifTitleOverlay.visualPositionY).toBe(titleDragVisual.transform.position.y);

    await editSelectedTextThroughInspector(page, app, {
      content: "真实案例标题\nSans 终版",
      fontFamily: "Noto Sans CJK SC",
      fontSize: 47,
      color: "#2cffb4",
      alignment: "right",
      textBoxWidthMillis: 800,
      textBoxHeightMillis: 190,
      layoutXMillis: 90,
      layoutYMillis: 90,
      layoutWidthMillis: 800,
      layoutHeightMillis: 230,
      lineHeightMillis: 1180,
      letterSpacingMillis: 30
    });
    const finalTitleEvidence = await waitForActiveTextOverlaySetEvidence(page, app, ["真实案例标题\nSans 终版"], 0, {
      exactOverlayCount: 1,
      forbiddenContents: ["真实案例标题\nSerif 二次编辑", "真实案例标题\nSans 初版"]
    });
    expect(
      finalTitleEvidence.hostImage.equals(serifTitleEvidence.hostImage),
      "native host pixels must change after switching the title font back to Sans"
    ).toBe(false);
    const finalTitleOnlyOverlay = overlayByContent(finalTitleEvidence.activeTextOverlays, "真实案例标题\nSans 终版");
    expect(finalTitleOnlyOverlay.fontRef).toBe(BUNDLED_SANS_FONT_REF);

    await addRenamedSubtitleTrack(page, app, "文字轨道 品牌条");
    await addTextThroughProductPanel(page, app, "品牌条 初稿");
    await editSelectedTextThroughInspector(page, app, {
      content: "品牌条｜多字体 Serif",
      fontFamily: "Noto Serif CJK SC",
      fontSize: 30,
      color: "#ff9e2c",
      alignment: "left",
      textBoxWidthMillis: 620,
      textBoxHeightMillis: 120,
      layoutXMillis: 70,
      layoutYMillis: 430,
      layoutWidthMillis: 650,
      layoutHeightMillis: 150,
      lineHeightMillis: 1100,
      letterSpacingMillis: 60
    });
    await updateSelectedVisualThroughInspector(page, app, {
      positionX: -130,
      positionY: -20,
      scaleX: 1040,
      scaleY: 1040,
      rotation: 6,
      opacity: 920,
      fitMode: "适应"
    });

    await addRenamedSubtitleTrack(page, app, "字幕轨道 A");
    await importSubtitleSrtThroughProductPanel(page, app, firstTrackSrt);
    await page.getByRole("button", { name: /片段 同屏字幕 A/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "同屏字幕 A\n真实示例",
      fontFamily: "Noto Sans CJK SC",
      fontSize: 35,
      color: "#f9f871",
      alignment: "center",
      textBoxWidthMillis: 780,
      textBoxHeightMillis: 160,
      layoutXMillis: 105,
      layoutYMillis: 600,
      layoutWidthMillis: 790,
      layoutHeightMillis: 190,
      lineHeightMillis: 1220,
      letterSpacingMillis: 20
    });
    await page.getByRole("button", { name: /片段 错峰字幕 A/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "错峰字幕 A\nSerif 后半段",
      fontFamily: "Noto Serif CJK SC",
      fontSize: 34,
      color: "#40c7ff",
      alignment: "center",
      textBoxWidthMillis: 770,
      textBoxHeightMillis: 160,
      layoutXMillis: 115,
      layoutYMillis: 650,
      layoutWidthMillis: 770,
      layoutHeightMillis: 190,
      lineHeightMillis: 1200,
      letterSpacingMillis: 70
    });

    await addRenamedSubtitleTrack(page, app, "字幕轨道 B");
    await importSubtitleSrtThroughProductPanel(page, app, secondTrackSrt);
    await page.getByRole("button", { name: /片段 同屏字幕 B/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "同屏字幕 B\nLaunch 2026",
      fontFamily: "Noto Serif CJK SC",
      fontSize: 33,
      color: "#ff63d8",
      alignment: "right",
      textBoxWidthMillis: 780,
      textBoxHeightMillis: 160,
      layoutXMillis: 105,
      layoutYMillis: 760,
      layoutWidthMillis: 800,
      layoutHeightMillis: 190,
      lineHeightMillis: 1240,
      letterSpacingMillis: 120
    });
    await updateSelectedVisualThroughInspector(page, app, {
      positionX: 95,
      positionY: -35,
      scaleX: 1090,
      scaleY: 1090,
      rotation: -10,
      opacity: 880,
      fitMode: "适应"
    });
    await page.getByRole("button", { name: /片段 错峰字幕 B/ }).click();
    await editSelectedTextThroughInspector(page, app, {
      content: "错峰字幕 B\nSans 后半段",
      fontFamily: "Noto Sans CJK SC",
      fontSize: 34,
      color: "#ffffff",
      alignment: "right",
      textBoxWidthMillis: 770,
      textBoxHeightMillis: 160,
      layoutXMillis: 130,
      layoutYMillis: 760,
      layoutWidthMillis: 770,
      layoutHeightMillis: 190,
      lineHeightMillis: 1200,
      letterSpacingMillis: 90
    });

    await page.getByRole("button", { name: "选择轨道 视频轨道 1" }).click();
    await expect(page.locator(".preview-text-overlay"), "native text regression must not be satisfied by DOM text overlays").toHaveCount(0);
    await seekTimelinePlayhead(page, app, 600_000);
    const sameTimeEvidence = await waitForActiveTextOverlaySetEvidence(
      page,
      app,
      ["真实案例标题\nSans 终版", "品牌条｜多字体 Serif", "同屏字幕 A\n真实示例", "同屏字幕 B\nLaunch 2026"],
      0,
      {
        maxTargetTimeUs: 1_750_000,
        exactOverlayCount: 4,
        forbiddenContents: ["真实案例标题\nSans 初版", "真实案例标题\nSerif 二次编辑", "错峰字幕 A\nSerif 后半段", "错峰字幕 B\nSans 后半段"]
      }
    );
    mkdirSync(PHASE15_3_SCREENSHOT_DIR, { recursive: true });
    await page.screenshot({
      path: join(PHASE15_3_SCREENSHOT_DIR, "text-editing-expanded-same-time-workspace.png"),
      fullPage: true
    });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "text-editing-expanded-same-time-host.png"),
      sameTimeEvidence.hostImage
    );
    await expectTextEditingNativeEvidence(page, app, sameTimeEvidence, "expanded same-time text/subtitle regression");
    const finalTitle = overlayByContent(sameTimeEvidence.activeTextOverlays, "真实案例标题\nSans 终版");
    const brandStrip = overlayByContent(sameTimeEvidence.activeTextOverlays, "品牌条｜多字体 Serif");
    const sameSubtitleA = overlayByContent(sameTimeEvidence.activeTextOverlays, "同屏字幕 A\n真实示例");
    const sameSubtitleB = overlayByContent(sameTimeEvidence.activeTextOverlays, "同屏字幕 B\nLaunch 2026");
    expect(finalTitle.source).toBe("text");
    expect(finalTitle.fontRef).toBe(BUNDLED_SANS_FONT_REF);
    expect(finalTitle.visualPositionX).toBe(titleDragVisual.transform.position.x);
    expect(finalTitle.visualPositionY).toBe(titleDragVisual.transform.position.y);
    expect(brandStrip.source).toBe("text");
    expect(brandStrip.fontRef).toBe(BUNDLED_SERIF_FONT_REF);
    expect(brandStrip.visualRotationDegrees).toBe(6);
    expect(sameSubtitleA.source).toBe("subtitle");
    expect(sameSubtitleA.fontRef).toBe(BUNDLED_SANS_FONT_REF);
    expect(sameSubtitleB.source).toBe("subtitle");
    expect(sameSubtitleB.fontRef).toBe(BUNDLED_SERIF_FONT_REF);
    expect(sameSubtitleB.visualRotationDegrees).toBe(-10);
    expect(new Set([finalTitle.y, brandStrip.y, sameSubtitleA.y, sameSubtitleB.y]).size).toBeGreaterThanOrEqual(3);

    await seekTimelinePlayhead(page, app, 3_200_000);
    const staggeredEvidence = await waitForActiveTextOverlaySetEvidence(
      page,
      app,
      ["错峰字幕 A\nSerif 后半段", "错峰字幕 B\nSans 后半段"],
      3_000_000,
      {
        maxTargetTimeUs: 3_580_000,
        exactOverlayCount: 2,
        forbiddenContents: ["真实案例标题\nSans 终版", "品牌条｜多字体 Serif", "同屏字幕 A\n真实示例", "同屏字幕 B\nLaunch 2026"]
      }
    );
    await page.screenshot({
      path: join(PHASE15_3_SCREENSHOT_DIR, "text-editing-expanded-staggered-workspace.png"),
      fullPage: true
    });
    writeFileSync(
      join(PHASE15_3_SCREENSHOT_DIR, "text-editing-expanded-staggered-host.png"),
      staggeredEvidence.hostImage
    );
    await expectTextEditingNativeEvidence(page, app, staggeredEvidence, "expanded staggered subtitle regression");
    const staggeredSubtitleA = overlayByContent(staggeredEvidence.activeTextOverlays, "错峰字幕 A\nSerif 后半段");
    const staggeredSubtitleB = overlayByContent(staggeredEvidence.activeTextOverlays, "错峰字幕 B\nSans 后半段");
    expect(staggeredSubtitleA.fontRef).toBe(BUNDLED_SERIF_FONT_REF);
    expect(staggeredSubtitleB.fontRef).toBe(BUNDLED_SANS_FONT_REF);
    expect(staggeredSubtitleA.y).not.toBe(staggeredSubtitleB.y);

    const calls = await readNativeCommandObservations(app);
    expect(calls.filter((call) => call.command === "importSubtitleSrtIntent")).toHaveLength(2);
    expect(calls.filter((call) => call.command === "editSelectedText").length).toBeGreaterThanOrEqual(8);
    expect(calls.filter((call) => call.command === "updateSelectedSegmentVisual").length).toBeGreaterThanOrEqual(3);
    expect(
      calls.filter((call) => call.command === "editSelectedText").map((call) => call.textFontRef),
      "font switching regression must exercise both bundled CJK font refs through session intents"
    ).toEqual(expect.arrayContaining([BUNDLED_SANS_FONT_REF, BUNDLED_SERIF_FONT_REF]));
    expect(requestProjectSessionPreviewFrameCount(calls), "expanded text regression must not request artifact preview frames").toBe(0);
    expectProductEditCommandsAreSessionOwned(
      await readProjectSessionCalls(app),
      await readDirectNativeCommandObservations(app),
      ["addTextSegmentIntent", "importSubtitleSrtIntent", "editSelectedText", "updateSelectedSegmentVisual", "addTrackIntent", "renameSelectedTrack"]
    );
    expectNoProductFallbackCalls(await readRealtimePreviewHostCalls(app));
  } finally {
    await app.close();
  }
});

type ActiveTextOverlayEvidence = {
  source: "text" | "subtitle";
  content: string;
  fontFamily: string;
  fontRef?: string | null;
  fontSize: number;
  color: string;
  alignment: "left" | "center" | "right";
  lineHeightMillis: number;
  letterSpacingMillis: number;
  x: number;
  y: number;
  width: number;
  height: number;
  visualPositionX: number;
  visualPositionY: number;
  visualScaleXMillis: number;
  visualScaleYMillis: number;
  visualRotationDegrees: number;
  visualOpacityMillis: number;
};

type TextInspectorEdit = {
  content: string;
  fontFamily: string;
  fontSize: number;
  color: string;
  alignment: "left" | "center" | "right";
  textBoxWidthMillis: number;
  textBoxHeightMillis: number;
  layoutXMillis: number;
  layoutYMillis: number;
  layoutWidthMillis: number;
  layoutHeightMillis: number;
  lineHeightMillis: number;
  letterSpacingMillis: number;
};

type TextOverlayWaitOptions = {
  maxTargetTimeUs?: number;
  exactOverlayCount?: number;
  forbiddenContents?: string[];
  timeoutMs?: number;
};

type VisualCommandEvidence = {
  visible: boolean;
  fitMode: string;
  transform: {
    position: { x: number; y: number };
    scale: { xMillis: number; yMillis: number };
    rotation: { degrees: number };
    opacity: { valueMillis: number };
  };
};

async function editSelectedTextThroughInspector(
  page: Page,
  app: ProductJourneyAppController,
  edit: TextInspectorEdit
): Promise<void> {
  const nextCount = (await commandCount(app, "editSelectedText")) + 1;
  const textTab = page.getByRole("tab", { name: "文本" });
  if ((await textTab.count()) > 0) {
    await textTab.click();
  }
  const textSection = page.locator('section[aria-label="文本"]');
  await expect(textSection).toBeVisible();
  await textSection.locator("textarea").fill(edit.content);
  await textSection.getByRole("combobox", { name: "字体" }).fill(edit.fontFamily);
  await textSection.getByRole("spinbutton", { name: "字号", exact: true }).fill(String(edit.fontSize));
  await textSection.getByRole("textbox", { name: "颜色", exact: true }).fill(edit.color);

  const styleSection = page.locator('section[aria-label="样式"]');
  const alignmentLabel = edit.alignment === "left" ? "左" : edit.alignment === "center" ? "中" : "右";
  await styleSection.getByRole("group", { name: "检查器文字对齐" }).getByRole("button", { name: alignmentLabel }).click();

  const textBoxSection = page.locator('section[aria-label="文本框"]');
  await textBoxSection.getByRole("spinbutton", { name: "宽度", exact: true }).fill(String(edit.textBoxWidthMillis));
  await textBoxSection.getByRole("spinbutton", { name: "高度", exact: true }).fill(String(edit.textBoxHeightMillis));
  await textBoxSection.getByRole("spinbutton", { name: "行高", exact: true }).fill(String(edit.lineHeightMillis));
  await textBoxSection.getByRole("spinbutton", { name: "字间距", exact: true }).fill(String(edit.letterSpacingMillis));

  const layoutSection = page.locator('section[aria-label="布局"]');
  await layoutSection.getByRole("spinbutton", { name: "X", exact: true }).fill(String(edit.layoutXMillis));
  await layoutSection.getByRole("spinbutton", { name: "Y", exact: true }).fill(String(edit.layoutYMillis));
  await layoutSection.getByRole("spinbutton", { name: "宽", exact: true }).fill(String(edit.layoutWidthMillis));
  await layoutSection.getByRole("spinbutton", { name: "高", exact: true }).fill(String(edit.layoutHeightMillis));
  await expect(layoutSection.getByRole("button", { name: "应用文字" })).toBeEnabled();
  await layoutSection.getByRole("button", { name: "应用文字" }).click();
  await waitForCommandCountAtLeast(app, "editSelectedText", nextCount);
  await expect(page.getByRole("complementary", { name: "属性检查器" }).getByRole("textbox", { name: "文字内容" })).toHaveValue(
    edit.content
  );
}

async function addRenamedSubtitleTrack(page: Page, app: ProductJourneyAppController, name: string): Promise<void> {
  const textTrackButtons = page.getByRole("button", { name: /选择轨道 文字轨道 \d+/ });
  const buttonIndex = await textTrackButtons.count();
  const nextAddTrackCount = (await commandCount(app, "addTrackIntent")) + 1;
  await page.getByRole("button", { name: "添加文字轨道" }).click();
  await waitForCommandCountAtLeast(app, "addTrackIntent", nextAddTrackCount);
  await expect(textTrackButtons).toHaveCount(buttonIndex + 1);
  const newTrackButton = textTrackButtons.nth(buttonIndex);
  const newTrackLabel = (await newTrackButton.getAttribute("aria-label")) ?? "";
  const newTrackName = newTrackLabel.replace(/^选择轨道\s+/, "");
  expect(newTrackName, "new text track name must be discoverable").not.toBe("");
  const nextRenameCount = (await commandCount(app, "renameSelectedTrack")) + 1;
  await newTrackButton.click();
  const nameInput = page.getByRole("textbox", { name: `${newTrackName} 名称` });
  await nameInput.fill(name);
  await nameInput.press("Enter");
  await waitForCommandCountAtLeast(app, "renameSelectedTrack", nextRenameCount);
  await expect(page.getByRole("button", { name: `选择轨道 ${name}` })).toBeVisible();
  await page.getByRole("button", { name: `选择轨道 ${name}` }).click();
}

async function dragSelectedPreviewTextOverlay(
  page: Page,
  app: ProductJourneyAppController,
  expectedContent: string,
  deltaX: number,
  deltaY: number
): Promise<VisualCommandEvidence> {
  const beforeEvidence = await waitForActiveTextOverlaySetEvidence(page, app, [expectedContent], 0, {
    timeoutMs: 8_000
  });
  const beforeOverlay = overlayByContent(beforeEvidence.activeTextOverlays, expectedContent);
  const nextVisualUpdateCount = (await commandCount(app, "updateSelectedSegmentVisual")) + 1;
  const selectionOutline = page.getByLabel("预览选中框");
  await expect(selectionOutline, "selected text must expose preview edit handles over the native surface").toBeVisible({
    timeout: 10_000
  });
  const box = await selectionOutline.boundingBox();
  expect(box, "selected preview text edit handle must have a measurable box").not.toBeNull();
  await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
  await page.mouse.down();
  await page.mouse.move(box!.x + box!.width / 2 + deltaX, box!.y + box!.height / 2 + deltaY, {
    steps: 8
  });
  await page.mouse.up();
  await waitForCommandCountAtLeast(app, "updateSelectedSegmentVisual", nextVisualUpdateCount);
  const visual = latestVisualCommandEvidence(await readNativeCommandObservations(app));
  const afterEvidence = await waitForTextOverlayVisualPositionEvidence(
    page,
    app,
    expectedContent,
    visual.transform.position.x,
    visual.transform.position.y
  );
  const afterOverlay = overlayByContent(afterEvidence.activeTextOverlays, expectedContent);
  expect(
    Math.abs(afterOverlay.visualPositionX - beforeOverlay.visualPositionX) +
      Math.abs(afterOverlay.visualPositionY - beforeOverlay.visualPositionY),
    `${expectedContent} native text evidence must reflect direct preview drag movement`
  ).toBeGreaterThan(10);
  expect(
    afterEvidence.hostImage.equals(beforeEvidence.hostImage),
    `${expectedContent} native host pixels must change after direct preview drag`
  ).toBe(false);
  return visual;
}

function latestVisualCommandEvidence(calls: Awaited<ReturnType<typeof readNativeCommandObservations>>): VisualCommandEvidence {
  const visual =
    calls.findLast(
      (call) => call.command === "updateSelectedSegmentVisual" && call.visual !== null && call.visual !== undefined
    )?.visual ?? null;
  expect(visual, "preview drag must send a session-owned visual update").not.toBeNull();
  return visual!;
}

async function waitForTextOverlayVisualPositionEvidence(
  page: Page,
  app: ProductJourneyAppController,
  expectedContent: string,
  visualPositionX: number,
  visualPositionY: number
) {
  const deadline = Date.now() + 8_000;
  let lastEvidence: unknown = null;

  while (Date.now() < deadline) {
    const previewEvidence = await capturePreviewEvidence(page);
    const evidence = previewEvidence.hostState?.contentEvidence;
    const activeTextOverlays = (evidence?.activeTextOverlays ?? []) as ActiveTextOverlayEvidence[];
    const overlay = activeTextOverlays.find((candidate) => textContentMatches(candidate.content, expectedContent));
    lastEvidence = {
      activeTextOverlays,
      source: evidence?.source ?? null,
      targetTimeMicroseconds: evidence?.targetTimeMicroseconds ?? 0,
      expectedContent,
      visualPositionX,
      visualPositionY
    };
    if (
      evidence?.source === "renderGraphGpuComposited" &&
      overlay !== undefined &&
      overlay.visualPositionX === visualPositionX &&
      overlay.visualPositionY === visualPositionY
    ) {
      return {
        previewEvidence,
        activeTextOverlays,
        targetTimeMicroseconds: evidence.targetTimeMicroseconds,
        hostImage: await captureVisiblePreviewHostImage(page, app)
      };
    }
    await page.waitForTimeout(200);
  }

  throw new Error(`Timed out waiting for text overlay visual position evidence: ${JSON.stringify(lastEvidence)}`);
}

async function waitForActiveTextOverlaySetEvidence(
  page: Page,
  app: ProductJourneyAppController,
  expectedContents: string[],
  minTargetTimeUs: number,
  maxTargetTimeUsOrOptions: number | TextOverlayWaitOptions = Number.POSITIVE_INFINITY
) {
  const options =
    typeof maxTargetTimeUsOrOptions === "number" ? { maxTargetTimeUs: maxTargetTimeUsOrOptions } : maxTargetTimeUsOrOptions;
  const maxTargetTimeUs = options.maxTargetTimeUs ?? Number.POSITIVE_INFINITY;
  const exactOverlayCount = options.exactOverlayCount ?? null;
  const forbiddenContents = options.forbiddenContents ?? [];
  const deadline = Date.now() + (options.timeoutMs ?? 12_000);
  let lastEvidence: unknown = null;

  while (Date.now() < deadline) {
    const previewEvidence = await capturePreviewEvidence(page);
    const evidence = previewEvidence.hostState?.contentEvidence;
    const activeTextOverlays = (evidence?.activeTextOverlays ?? []) as ActiveTextOverlayEvidence[];
    const activeContents = activeTextOverlays.map((overlay) => overlay.content);
    const expectedPresent = expectedContents.every((content) =>
      activeTextOverlays.some((overlay) => textContentMatches(overlay.content, content))
    );
    const forbiddenPresent = forbiddenContents.filter((content) =>
      activeTextOverlays.some((overlay) => textContentMatches(overlay.content, content))
    );
    lastEvidence = {
      activeContents,
      activeTextOverlays,
      source: evidence?.source ?? null,
      targetTimeMicroseconds: evidence?.targetTimeMicroseconds ?? 0,
      expectedContents,
      exactOverlayCount,
      forbiddenContents,
      forbiddenPresent
    };
    const targetTime = evidence?.targetTimeMicroseconds ?? 0;
    if (
      evidence?.source === "renderGraphGpuComposited" &&
      targetTime >= minTargetTimeUs &&
      targetTime <= maxTargetTimeUs &&
      expectedPresent &&
      forbiddenPresent.length === 0 &&
      (exactOverlayCount === null || activeTextOverlays.length === exactOverlayCount)
    ) {
      return {
        previewEvidence,
        activeTextOverlays,
        targetTimeMicroseconds: targetTime,
        hostImage: await captureVisiblePreviewHostImage(page, app)
      };
    }
    await page.waitForTimeout(200);
  }

  throw new Error(`Timed out waiting for active text overlays ${expectedContents.join(", ")}: ${JSON.stringify(lastEvidence)}`);
}

async function expectTextEditingNativeEvidence(
  page: Page,
  app: ProductJourneyAppController,
  evidence: Awaited<ReturnType<typeof waitForActiveTextOverlaySetEvidence>>,
  label: string
): Promise<void> {
  expect(evidence.previewEvidence.hostState?.productReady, `${label} must use product-ready native preview`).toBe(true);
  expect(evidence.previewEvidence.hostState?.fallbackActive, `${label} must not use fallback preview`).toBe(false);
  expect(evidence.previewEvidence.hostState?.backend, `${label} backend`).toBe("renderGraphGpu");
  expect(evidence.previewEvidence.hostState?.contentEvidence?.source, `${label} content source`).toBe("renderGraphGpuComposited");
  const expectedScreenRect = await expectedPreviewHostScreenRect(page, app);
  const placement = evidence.previewEvidence.hostState?.surfacePlacement ?? null;
  expect(maxRectDelta(placement?.hostScreenRect ?? null, expectedScreenRect), `${label} host rect`).toBeLessThanOrEqual(2);
  expect(maxRectDelta(placement?.nativeScreenRect ?? null, expectedScreenRect), `${label} native rect`).toBeLessThanOrEqual(2);
  await expectPreviewHostCoversCanvas(page);
  expectLandscapeNativePreviewPlacement(await measurePngPreviewPlacement(page, evidence.hostImage), `${label} native preview`);
  const contentEvidence = evidence.previewEvidence.hostState?.contentEvidence;
  for (const overlay of evidence.activeTextOverlays) {
    await expectTextOverlayPixelsInNativeHost(page, evidence.hostImage, contentEvidence, overlay, `${label} ${overlay.content}`);
  }
}

function overlayByContent(overlays: ActiveTextOverlayEvidence[], content: string): ActiveTextOverlayEvidence {
  const overlay = overlays.find((candidate) => textContentMatches(candidate.content, content));
  expect(overlay, `overlay ${content} must be active`).toBeDefined();
  return overlay!;
}

function textContentMatches(actual: string, expected: string): boolean {
  return normalizedTextContent(actual) === normalizedTextContent(expected);
}

function normalizedTextContent(content: string): string {
  return content.replace(/\s+/g, "");
}

async function commandCount(app: ProductJourneyAppController, command: string): Promise<number> {
  return (await readNativeCommandObservations(app)).filter((call) => call.command === command).length;
}

async function waitForCommandCountAtLeast(app: ProductJourneyAppController, command: string, count: number): Promise<void> {
  await expect.poll(async () => commandCount(app, command), { timeout: 10_000 }).toBeGreaterThanOrEqual(count);
}

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
    const previewEvidence = await capturePreviewEvidence(page);
    const evidence = previewEvidence.hostState?.contentEvidence;
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
        previewEvidence,
        activeTextOverlays,
        targetTimeMicroseconds: evidence.targetTimeMicroseconds,
        hostImage: await captureVisiblePreviewHostImage(page, app)
      };
    }
    await page.waitForTimeout(200);
  }

  throw new Error(`Timed out waiting for active subtitle ${subtitle}: ${JSON.stringify(lastEvidence)}`);
}

async function expectComboSubtitleNativeEvidence(
  page: Page,
  app: Awaited<ReturnType<typeof launchProductJourneyApp>>["app"],
  evidence: Awaited<ReturnType<typeof waitForActiveSubtitleEvidence>>,
  label: string
): Promise<void> {
  expect(evidence.previewEvidence.hostState?.productReady, `${label} must use product-ready native preview`).toBe(true);
  expect(evidence.previewEvidence.hostState?.fallbackActive, `${label} must not use fallback preview`).toBe(false);
  expect(evidence.previewEvidence.hostState?.backend, `${label} backend`).toBe("renderGraphGpu");
  expect(evidence.previewEvidence.hostState?.contentEvidence?.source, `${label} content source`).toBe("renderGraphGpuComposited");
  expect(evidence.previewEvidence.hostState?.surfacePlacement, `${label} must expose native surface placement`).not.toBeNull();
  const expectedScreenRect = await expectedPreviewHostScreenRect(page, app);
  const placement = evidence.previewEvidence.hostState?.surfacePlacement ?? null;
  expect(
    maxRectDelta(placement?.hostScreenRect ?? null, expectedScreenRect),
    `${label} host rect must match DOM preview host: ${JSON.stringify({ placement, expectedScreenRect })}`
  ).toBeLessThanOrEqual(2);
  expect(
    maxRectDelta(placement?.nativeScreenRect ?? null, expectedScreenRect),
    `${label} native rect must match DOM preview host: ${JSON.stringify({ placement, expectedScreenRect })}`
  ).toBeLessThanOrEqual(2);
  expect(placement?.maxDeltaPx ?? Number.POSITIVE_INFINITY, `${label} native placement delta`).toBeLessThanOrEqual(2);
  await expectPreviewHostCoversCanvas(page);
  const metrics = await measurePngPreviewPlacement(page, evidence.hostImage);
  expectLandscapeNativePreviewPlacement(metrics, `combo ${label} native preview`);
  const contentEvidence = evidence.previewEvidence.hostState?.contentEvidence;
  const title = evidence.activeTextOverlays.find((text) => text.source === "text" && text.content === "组合标题");
  const subtitle = evidence.activeTextOverlays.find((text) => text.source === "subtitle");
  expect(title, `${label} must include title bbox evidence`).toBeDefined();
  expect(subtitle, `${label} must include subtitle bbox evidence`).toBeDefined();
  expect(contentEvidence?.height ?? 0, `${label} must expose render target height`).toBeGreaterThan(0);
  if (title !== undefined && subtitle !== undefined) {
    expect(
      subtitle.y,
      `${label} subtitle must render below title text: ${JSON.stringify({ title, subtitle })}`
    ).toBeGreaterThan(title.y + title.height);
    expect(
      subtitle.y,
      `${label} subtitle must use the lower subtitle-safe region: ${JSON.stringify({ contentEvidence, subtitle })}`
    ).toBeGreaterThanOrEqual(Math.round((contentEvidence?.height ?? 0) * 0.55));
    await expectTextOverlayPixelsInNativeHost(page, evidence.hostImage, contentEvidence, title, `${label} title`);
    await expectTextOverlayPixelsInNativeHost(page, evidence.hostImage, contentEvidence, subtitle, `${label} subtitle`);
  }
}

async function expectTextOverlayPixelsInNativeHost(
  page: Page,
  image: Buffer,
  contentEvidence: { width: number; height: number } | null | undefined,
  overlay: ActiveTextOverlayEvidence,
  label: string
): Promise<void> {
  expect(contentEvidence?.width ?? 0, `${label} render target width`).toBeGreaterThan(0);
  expect(contentEvidence?.height ?? 0, `${label} render target height`).toBeGreaterThan(0);
  const transformedBox = transformedTextOverlayBox(contentEvidence, overlay);
  const textPixelCount = await countTextColorPixelsInOverlay(page, image, contentEvidence, transformedBox, overlay.color);
  expect(
    textPixelCount,
    `${label} bbox must contain real colored text pixels in the native host PNG: ${JSON.stringify({ overlay, transformedBox, contentEvidence })}`
  ).toBeGreaterThan(40);
}

function transformedTextOverlayBox(
  contentEvidence: { width: number; height: number } | null | undefined,
  overlay: ActiveTextOverlayEvidence
): { x: number; y: number; width: number; height: number } {
  const targetWidth = Math.max(1, contentEvidence?.width ?? 1);
  const targetHeight = Math.max(1, contentEvidence?.height ?? 1);
  const scaleX = Math.max(1, overlay.visualScaleXMillis) / 1000;
  const scaleY = Math.max(1, overlay.visualScaleYMillis) / 1000;
  const width = Math.max(1, overlay.width * scaleX);
  const height = Math.max(1, overlay.height * scaleY);
  const centerX = overlay.x + overlay.width / 2 + (targetWidth * overlay.visualPositionX) / 2000;
  const centerY = overlay.y + overlay.height / 2 - (targetHeight * overlay.visualPositionY) / 2000;
  const radians = (overlay.visualRotationDegrees * Math.PI) / 180;
  const rotatedWidth = Math.abs(width * Math.cos(radians)) + Math.abs(height * Math.sin(radians));
  const rotatedHeight = Math.abs(width * Math.sin(radians)) + Math.abs(height * Math.cos(radians));
  return {
    x: centerX - rotatedWidth / 2,
    y: centerY - rotatedHeight / 2,
    width: rotatedWidth,
    height: rotatedHeight
  };
}

async function countTextColorPixelsInOverlay(
  page: Page,
  image: Buffer,
  contentEvidence: { width: number; height: number } | null | undefined,
  overlay: { x: number; y: number; width: number; height: number },
  color: string
): Promise<number> {
  const base64 = image.toString("base64");
  const expectedColor = parseHexColor(color);
  return page.evaluate(
    async ({ pngBase64, evidenceWidth, evidenceHeight, box, expected }) => {
      const bytes = Uint8Array.from(atob(pngBase64), (character) => character.charCodeAt(0));
      const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
      const canvas = document.createElement("canvas");
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
      const context = canvas.getContext("2d");
      if (context === null) {
        throw new Error("Canvas 2D context unavailable for text pixel measurement");
      }
      context.drawImage(bitmap, 0, 0);
      bitmap.close();
      const scaleX = evidenceWidth > 0 ? canvas.width / evidenceWidth : 1;
      const scaleY = evidenceHeight > 0 ? canvas.height / evidenceHeight : 1;
      const left = Math.max(0, Math.floor(box.x * scaleX));
      const top = Math.max(0, Math.floor(box.y * scaleY));
      const right = Math.min(canvas.width, Math.ceil((box.x + box.width) * scaleX));
      const bottom = Math.min(canvas.height, Math.ceil((box.y + box.height) * scaleY));
      if (right <= left || bottom <= top) {
        return 0;
      }
      const data = context.getImageData(left, top, right - left, bottom - top).data;
      const colorDistanceThreshold = 125;
      let textPixels = 0;
      for (let index = 0; index < data.length; index += 4) {
        const red = data[index];
        const green = data[index + 1];
        const blue = data[index + 2];
        const distance = Math.hypot(red - expected.red, green - expected.green, blue - expected.blue);
        const expectedMax = Math.max(expected.red, expected.green, expected.blue);
        const expectedMin = Math.min(expected.red, expected.green, expected.blue);
        if (expectedMax - expectedMin <= 40) {
          const max = Math.max(red, green, blue);
          const min = Math.min(red, green, blue);
          if (min >= 170 && max - min <= 95) {
            textPixels += 1;
          }
        } else if (distance <= colorDistanceThreshold) {
          textPixels += 1;
        }
      }
      return textPixels;
    },
    {
      pngBase64: base64,
      evidenceWidth: contentEvidence?.width ?? 0,
      evidenceHeight: contentEvidence?.height ?? 0,
      box: overlay,
      expected: expectedColor
    }
  );
}

function parseHexColor(color: string): { red: number; green: number; blue: number } {
  const normalized = color.trim().replace(/^#/, "");
  expect(normalized, `text color must be #rrggbb: ${color}`).toMatch(/^[0-9a-fA-F]{6}$/);
  return {
    red: Number.parseInt(normalized.slice(0, 2), 16),
    green: Number.parseInt(normalized.slice(2, 4), 16),
    blue: Number.parseInt(normalized.slice(4, 6), 16)
  };
}

type PngPreviewPlacementMetrics = {
  width: number;
  height: number;
  mean: number;
  stddev: number;
  aspectRatio: number;
  foregroundCoverage: number;
  foregroundCenterOffsetX: number;
  foregroundCenterOffsetY: number;
  horizontalMarginDeltaRatio: number;
  verticalMarginDeltaRatio: number;
};

async function measurePngPreviewPlacement(page: Page, image: Buffer): Promise<PngPreviewPlacementMetrics> {
  const base64 = image.toString("base64");
  return page.evaluate(async (pngBase64) => {
    const bytes = Uint8Array.from(atob(pngBase64), (character) => character.charCodeAt(0));
    const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
    const canvas = document.createElement("canvas");
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    const context = canvas.getContext("2d");
    if (context === null) {
      throw new Error("Canvas 2D context unavailable for PNG luma measurement");
    }
    context.drawImage(bitmap, 0, 0);
    bitmap.close();
    const data = context.getImageData(0, 0, canvas.width, canvas.height).data;
    let count = 0;
    let sum = 0;
    let sumSquares = 0;
    let foregroundCount = 0;
    let minX = canvas.width;
    let minY = canvas.height;
    let maxX = -1;
    let maxY = -1;
    for (let index = 0; index < data.length; index += 4) {
      const luma = 0.2126 * data[index] + 0.7152 * data[index + 1] + 0.0722 * data[index + 2];
      const pixelIndex = index / 4;
      const x = pixelIndex % canvas.width;
      const y = Math.floor(pixelIndex / canvas.width);
      count += 1;
      sum += luma;
      sumSquares += luma * luma;
      if (luma > 12) {
        foregroundCount += 1;
        minX = Math.min(minX, x);
        minY = Math.min(minY, y);
        maxX = Math.max(maxX, x);
        maxY = Math.max(maxY, y);
      }
    }
    const mean = count === 0 ? 0 : sum / count;
    const variance = count === 0 ? 0 : Math.max(0, sumSquares / count - mean * mean);
    const hasForeground = foregroundCount > 0;
    const foregroundCenterX = hasForeground ? (minX + maxX + 1) / 2 : canvas.width / 2;
    const foregroundCenterY = hasForeground ? (minY + maxY + 1) / 2 : canvas.height / 2;
    const leftMargin = hasForeground ? minX : canvas.width;
    const rightMargin = hasForeground ? canvas.width - maxX - 1 : canvas.width;
    const topMargin = hasForeground ? minY : canvas.height;
    const bottomMargin = hasForeground ? canvas.height - maxY - 1 : canvas.height;
    return {
      width: canvas.width,
      height: canvas.height,
      mean,
      stddev: Math.sqrt(variance),
      aspectRatio: canvas.height === 0 ? 0 : canvas.width / canvas.height,
      foregroundCoverage: count === 0 ? 0 : foregroundCount / count,
      foregroundCenterOffsetX: canvas.width === 0 ? 0 : Math.abs(foregroundCenterX - canvas.width / 2) / canvas.width,
      foregroundCenterOffsetY: canvas.height === 0 ? 0 : Math.abs(foregroundCenterY - canvas.height / 2) / canvas.height,
      horizontalMarginDeltaRatio: canvas.width === 0 ? 0 : Math.abs(leftMargin - rightMargin) / canvas.width,
      verticalMarginDeltaRatio: canvas.height === 0 ? 0 : Math.abs(topMargin - bottomMargin) / canvas.height
    };
  }, base64);
}

function expectP0NativePreviewPlacement(metrics: PngPreviewPlacementMetrics, label: string): void {
  expect(metrics.width, `${label} width`).toBeGreaterThan(100);
  expect(metrics.height, `${label} height`).toBeGreaterThan(100);
  expect(metrics.aspectRatio, `${label} must keep the portrait material aspect ratio`).toBeGreaterThan(0.54);
  expect(metrics.aspectRatio, `${label} must keep the portrait material aspect ratio`).toBeLessThan(0.59);
  expect(metrics.mean, `${label} must not be an empty black surface`).toBeGreaterThan(5);
  expect(metrics.mean, `${label} must not be an empty white surface`).toBeLessThan(250);
  expect(metrics.stddev, `${label} must contain visible image detail`).toBeGreaterThan(3);
  expect(metrics.foregroundCoverage, `${label} must not be mostly black padding`).toBeGreaterThan(0.7);
  expect(metrics.foregroundCenterOffsetX, `${label} foreground must not be shifted toward the left or right edge`).toBeLessThanOrEqual(0.06);
  expect(metrics.foregroundCenterOffsetY, `${label} foreground must not be shifted toward the top or bottom edge`).toBeLessThanOrEqual(0.06);
  expect(metrics.horizontalMarginDeltaRatio, `${label} black side margins must be balanced`).toBeLessThanOrEqual(0.08);
  expect(metrics.verticalMarginDeltaRatio, `${label} black top/bottom margins must be balanced`).toBeLessThanOrEqual(0.08);
}

function expectLandscapeNativePreviewPlacement(metrics: PngPreviewPlacementMetrics, label: string): void {
  expect(metrics.width, `${label} width`).toBeGreaterThan(100);
  expect(metrics.height, `${label} height`).toBeGreaterThan(100);
  expect(metrics.aspectRatio, `${label} must keep a landscape preview shape`).toBeGreaterThan(1.6);
  expect(metrics.aspectRatio, `${label} must keep a landscape preview shape`).toBeLessThan(1.9);
  expect(metrics.mean, `${label} must not be an empty black surface`).toBeGreaterThan(5);
  expect(metrics.stddev, `${label} must contain visible image detail`).toBeGreaterThan(3);
  expect(metrics.foregroundCoverage, `${label} must not render into only a lower-left subsection`).toBeGreaterThan(0.7);
  expect(metrics.foregroundCenterOffsetX, `${label} foreground must not be shifted toward the left or right edge`).toBeLessThanOrEqual(0.06);
  expect(metrics.foregroundCenterOffsetY, `${label} foreground must not be shifted toward the top or bottom edge`).toBeLessThanOrEqual(0.06);
  expect(metrics.horizontalMarginDeltaRatio, `${label} side margins must be balanced`).toBeLessThanOrEqual(0.08);
  expect(metrics.verticalMarginDeltaRatio, `${label} top/bottom margins must be balanced`).toBeLessThanOrEqual(0.08);
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

async function waitForNativePreviewResizeSync(
  page: Page,
  app: Awaited<ReturnType<typeof launchProductJourneyApp>>["app"],
  presentedBeforeResize: number
): Promise<void> {
  const deadline = Date.now() + 5_000;
  let lastResizeEvidence: unknown = null;

  while (Date.now() < deadline) {
    const evidence = await capturePreviewEvidence(page);
    const expectedScreenRect = await expectedPreviewHostScreenRect(page, app);
    const placement = evidence.hostState?.surfacePlacement ?? null;
    const renderedTime = evidence.hostState?.contentEvidence?.targetTimeMicroseconds ?? -1;
    const telemetryTime = evidence.hostState?.telemetry?.targetTimeMicroseconds ?? -1;
    const mediaClockDelta = Math.abs(renderedTime - telemetryTime);
    const playheadDelta = Math.abs(renderedTime - evidence.timecodeUs);
    lastResizeEvidence = {
      generationAfterResize: evidence.hostState?.playbackGeneration ?? null,
      presentedBeforeResize,
      presentedAfterResize: evidence.hostState?.telemetry?.presentedFrameCount ?? 0,
      renderedTime,
      telemetryTime,
      timecodeUs: evidence.timecodeUs,
      mediaClockDelta,
      playheadDelta,
      placement,
      expectedScreenRect
    };
    if (
      (evidence.hostState?.telemetry?.presentedFrameCount ?? 0) > presentedBeforeResize &&
      placement !== null &&
      maxRectDelta(placement.hostScreenRect, expectedScreenRect) <= 2 &&
      maxRectDelta(placement.nativeScreenRect, expectedScreenRect) <= 2 &&
      (placement.maxDeltaPx ?? Number.POSITIVE_INFINITY) <= 2 &&
      mediaClockDelta <= 50_000 &&
      playheadDelta <= 300_000
    ) {
      return;
    }
    await page.waitForTimeout(200);
  }

  throw new Error(`native preview must stay attached and time-synced after maximize: ${JSON.stringify(lastResizeEvidence)}`);
}

function expectRealtimePreviewResizeDidNotRestartPlayback(hostCallsAfterResize: RealtimePreviewHostCall[]): void {
  const forbiddenRestartCommands = new Set([
    "attachSurface",
    "detachSurface",
    "updateProjectSessionSnapshot",
    "seek",
    "play",
    "pause",
    "stop",
    "schedulerPlaybackWorkerStart"
  ]);
  const restartCalls = hostCallsAfterResize.filter((call) => forbiddenRestartCommands.has(call.kind));

  expect(
    hostCallsAfterResize.some((call) => call.kind === "updateSurfaceBounds"),
    `resizing the product window must update native surface bounds: ${JSON.stringify(hostCallsAfterResize.slice(-20))}`
  ).toBe(true);
  expect(
    restartCalls,
    `surface resize must not restart playback or resync the project snapshot: ${JSON.stringify(hostCallsAfterResize.slice(-20))}`
  ).toEqual([]);
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

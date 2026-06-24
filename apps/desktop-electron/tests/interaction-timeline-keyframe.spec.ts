import { expect, test, type Page } from "@playwright/test";

import {
  USER_JOURNEY_MOVING_VIDEO,
  addVideoTrack,
  dragMaterialToTimeline,
  importMaterialThroughProductPicker,
  launchProductJourneyApp,
  readProjectSessionCalls,
  readRealtimePreviewHostCalls,
  readTimelineSegments,
  seekTimelinePlayhead,
  type ProductJourneyAppController
} from "./helpers/userJourney";

test.describe.configure({ timeout: 90_000 });

test("timeline move, cross-track move, and trim stream Rust provisional interactions", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await dragMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addVideoTrack(page, app);
    await page.getByRole("button", { name: /片段 p0-moving-testsrc\.mp4/ }).click();

    const trimHandles = page.locator(".segment-block.selected .segment-trim-handle");
    await expect(trimHandles.first()).toBeVisible();
    const handleWidths = await trimHandles.evaluateAll((handles) =>
      handles.map((handle) => Math.round((handle as HTMLElement).getBoundingClientRect().width))
    );
    expect(Math.min(...handleWidths), "trim handles need at least 16px effective hit target").toBeGreaterThanOrEqual(16);
    await expect(page.locator(".segment-transition-handle, .segment-fade-handle")).toHaveCount(0);

    const beforeCalls = await readProjectSessionCalls(app);
    const beforeIndex = beforeCalls.length;
    const baseRevision = latestResultRevision(beforeCalls);
    expect(baseRevision, "fixture setup must produce a canonical project revision").not.toBeNull();

    const segment = page.locator(".segment-block.selected").first();
    const targetTrack = page.locator(".track-row", { has: page.getByRole("button", { name: "选择轨道 视频轨道 2" }) }).first();
    const segmentBox = await requiredBox(segment, "selected segment");
    const targetTrackBox = await requiredBox(targetTrack, "target video track");
    const startX = segmentBox.x + Math.max(18, Math.min(segmentBox.width - 18, segmentBox.width / 2));
    const startY = segmentBox.y + segmentBox.height / 2;
    const targetX = startX + 60;
    const targetY = targetTrackBox.y + targetTrackBox.height / 2;

    await page.mouse.move(startX, startY);
    await page.mouse.down();
    await page.mouse.move(targetX, targetY, { steps: 8 });

    await expect
      .poll(async () => interactionCommandsSince(app, beforeIndex), { timeout: 10_000 })
      .toEqual(expect.arrayContaining(["beginProjectInteraction", "updateProjectInteraction"]));
    const liveMoveCalls = callsSince(await readProjectSessionCalls(app), beforeIndex);
    expect(commandCount(liveMoveCalls, "commitProjectInteraction"), "timeline drag must not commit before pointer-up").toBe(0);
    expect(
      liveMoveCalls.some((call) => call.command === "executeProjectIntent" && call.intentKind === "moveSelectedSegmentIntent"),
      "timeline drag must not route live samples through moveSelectedSegmentIntent"
    ).toBe(false);
    for (const update of liveMoveCalls.filter((call) => call.command === "updateProjectInteraction")) {
      expect(update.interactionKind).toBe("timelineMoveTrim");
      expect(update.interactionPayloadKind).toBe("timelineMoveTrim");
      expect(update.resultRevision).toBe(baseRevision);
      expect(update.revisionUnchanged).toBe(true);
      expect(update.resultDeltaCommand).toBe("moveSegment");
    }
    await expect(segment).toHaveAttribute("data-interaction-source", "rust-provisional");
    await expect(segment).toHaveAttribute("data-interaction-kind", "timelineMoveTrim");

    await page.mouse.up();
    await expect
      .poll(async () => commandCount(callsSince(await readProjectSessionCalls(app), beforeIndex), "commitProjectInteraction"), {
        timeout: 10_000
      })
      .toBe(1);
    const committedMoveCalls = callsSince(await readProjectSessionCalls(app), beforeIndex);
    const moveCommit = committedMoveCalls.findLast((call) => call.command === "commitProjectInteraction");
    expect(moveCommit?.interactionKind).toBe("timelineMoveTrim");
    expect(moveCommit?.resultRevision).toBe((baseRevision ?? 0) + 1);
    expect(
      committedMoveCalls.some((call) => call.command === "executeProjectIntent" && call.intentKind === "moveSelectedSegmentIntent"),
      "timeline drag commit must use commitProjectInteraction rather than executeProjectIntent"
    ).toBe(false);

    await expect
      .poll(async () => (await readTimelineSegments(page, /p0-moving-testsrc\.mp4/)).at(0)?.trackName ?? "", {
        timeout: 10_000
      })
      .toBe("视频轨道 2");

    const beforeTrimIndex = (await readProjectSessionCalls(app)).length;
    const trimBaseRevision = latestResultRevision(await readProjectSessionCalls(app));
    const leftHandle = page.locator(".segment-block.selected .segment-trim-handle.left").first();
    const trimBox = await requiredBox(leftHandle, "left trim handle");
    await page.mouse.move(trimBox.x + trimBox.width / 2, trimBox.y + trimBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(trimBox.x + trimBox.width / 2 + 36, trimBox.y + trimBox.height / 2, { steps: 6 });
    await expect
      .poll(async () => interactionCommandsSince(app, beforeTrimIndex), { timeout: 10_000 })
      .toEqual(expect.arrayContaining(["beginProjectInteraction", "updateProjectInteraction"]));
    const liveTrimCalls = callsSince(await readProjectSessionCalls(app), beforeTrimIndex);
    expect(commandCount(liveTrimCalls, "commitProjectInteraction"), "trim drag must not commit before pointer-up").toBe(0);
    expect(
      liveTrimCalls.some((call) => call.command === "executeProjectIntent" && call.intentKind === "trimSelectedSegmentIntent"),
      "trim drag must not route live samples through trimSelectedSegmentIntent"
    ).toBe(false);
    for (const update of liveTrimCalls.filter((call) => call.command === "updateProjectInteraction")) {
      expect(update.interactionKind).toBe("timelineMoveTrim");
      expect(update.resultRevision).toBe(trimBaseRevision);
      expect(update.revisionUnchanged).toBe(true);
      expect(update.resultDeltaCommand).toBe("trimSegment");
    }
    await page.mouse.up();
    await expect
      .poll(async () => commandCount(callsSince(await readProjectSessionCalls(app), beforeTrimIndex), "commitProjectInteraction"), {
        timeout: 10_000
      })
      .toBe(1);
  } finally {
    await app.close();
  }
});

test("playhead scrub uses a coalesced navigation interaction without draft revision changes", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await dragMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    const beforeCalls = await readProjectSessionCalls(app);
    const beforeIndex = beforeCalls.length;
    const baseRevision = latestResultRevision(beforeCalls);
    expect(baseRevision).not.toBeNull();

    const rulerBox = await requiredBox(page.locator(".ruler-track"), "timeline ruler");
    const startX = rulerBox.x + rulerBox.width * 0.1;
    const endX = rulerBox.x + rulerBox.width * 0.58;
    const y = rulerBox.y + rulerBox.height / 2;
    await page.mouse.move(startX, y);
    await page.mouse.down();
    await page.mouse.move(endX, y, { steps: 12 });

    await expect
      .poll(async () => interactionCommandsSince(app, beforeIndex), { timeout: 10_000 })
      .toEqual(expect.arrayContaining(["beginProjectInteraction", "updateProjectInteraction"]));
    const liveCalls = callsSince(await readProjectSessionCalls(app), beforeIndex);
    expect(commandCount(liveCalls, "commitProjectInteraction"), "scrub must not commit before pointer-up").toBe(0);
    expect(liveCalls.some((call) => call.command === "executeProjectIntent"), "scrub must not use canonical draft intents").toBe(false);
    for (const update of liveCalls.filter((call) => call.command === "updateProjectInteraction")) {
      expect(update.interactionKind).toBe("playheadScrub");
      expect(update.interactionPayloadKind).toBe("playheadScrub");
      expect(update.resultRevision).toBe(baseRevision);
      expect(update.revisionUnchanged).toBe(true);
      expect(update.resultDeltaCommand).toBe("seekAudioPreview");
    }
    const latestUpdate = liveCalls.findLast((call) => call.command === "updateProjectInteraction");
    expect(latestUpdate?.acceptedSequence, "scrub should accept multiple monotonic samples").toBeGreaterThan(1);
    expect(latestUpdate?.coalescedThrough, "scrub should expose coalescing through latest accepted target").toBe(
      latestUpdate?.acceptedSequence
    );

    await page.mouse.up();
    await expect
      .poll(async () => commandCount(callsSince(await readProjectSessionCalls(app), beforeIndex), "commitProjectInteraction"), {
        timeout: 10_000
      })
      .toBe(1);
    const committedCalls = callsSince(await readProjectSessionCalls(app), beforeIndex);
    const commit = committedCalls.findLast((call) => call.command === "commitProjectInteraction");
    expect(commit?.interactionKind).toBe("playheadScrub");
    expect(commit?.resultRevision).toBe(baseRevision);
    expect(commit?.revisionUnchanged).toBeNull();

    const hostSeeks = (await readRealtimePreviewHostCalls(app)).filter((call) => call.kind === "seek");
    expect(hostSeeks.at(-1)?.targetTimeMicroseconds, "scrub must seek realtime preview through the scheduler host").toBeGreaterThan(0);
  } finally {
    await app.close();
  }
});

test("keyframe markers and keyed value edits use segment-relative Rust keyframe interactions", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await dragMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await openVisualInspector(page);
    await page.getByRole("spinbutton", { name: "位置 X", exact: true }).fill("120");
    await page.getByRole("spinbutton", { name: "位置 X", exact: true }).blur();
    await seekTimelinePlayhead(page, app, 500_000);
    await page.getByRole("button", { name: "添加位置 X关键帧" }).first().click();
    await expect(page.getByLabel(/p0-moving-testsrc\.mp4 位置 X关键帧/)).toBeVisible({ timeout: 10_000 });

    const beforeMarkerIndex = (await readProjectSessionCalls(app)).length;
    const marker = page.getByLabel(/p0-moving-testsrc\.mp4 位置 X关键帧/).first();
    const markerBox = await requiredBox(marker, "timeline keyframe marker");
    await page.mouse.move(markerBox.x + markerBox.width / 2, markerBox.y + markerBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(markerBox.x + markerBox.width / 2 + 34, markerBox.y + markerBox.height / 2, { steps: 6 });
    await expect
      .poll(async () => interactionCommandsSince(app, beforeMarkerIndex), { timeout: 10_000 })
      .toEqual(expect.arrayContaining(["beginProjectInteraction", "updateProjectInteraction"]));
    const liveMarkerCalls = callsSince(await readProjectSessionCalls(app), beforeMarkerIndex);
    for (const update of liveMarkerCalls.filter((call) => call.command === "updateProjectInteraction")) {
      expect(update.interactionKind).toBe("keyframeEdit");
      expect(update.interactionPayloadKind).toBe("keyframeEdit");
      expect(Number.isInteger(update.keyframeAt), "keyframe marker updates must carry integer microseconds").toBe(true);
      expect(update.keyframeAt, "keyframe time must be segment-relative, not absolute timeline time").toBeGreaterThanOrEqual(0);
      expect(update.keyframeAt, "keyframe time must be segment-relative, not absolute timeline time").toBeLessThan(3_000_000);
      expect(update.resultRevision).toBe(latestResultRevision(await readProjectSessionCalls(app)));
      expect(update.revisionUnchanged).toBe(true);
      expect(update.resultDeltaCommand).toBe("setSegmentKeyframe");
    }
    await page.mouse.up();
    await expect
      .poll(async () => commandCount(callsSince(await readProjectSessionCalls(app), beforeMarkerIndex), "commitProjectInteraction"), {
        timeout: 10_000
      })
      .toBe(1);

    await openVisualInspector(page);
    await seekTimelinePlayhead(page, app, 720_000);
    const beforeValueIndex = (await readProjectSessionCalls(app)).length;
    const slider = page.getByLabel("位置 X滑杆");
    const sliderBox = await requiredBox(slider, "position X slider");
    await page.mouse.move(sliderBox.x + sliderBox.width * 0.62, sliderBox.y + sliderBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(sliderBox.x + sliderBox.width * 0.72, sliderBox.y + sliderBox.height / 2, { steps: 5 });
    await expect
      .poll(async () => interactionCommandsSince(app, beforeValueIndex), { timeout: 10_000 })
      .toEqual(expect.arrayContaining(["beginProjectInteraction", "updateProjectInteraction"]));
    const liveValueCalls = callsSince(await readProjectSessionCalls(app), beforeValueIndex);
    expect(
      liveValueCalls.some((call) => call.command === "updateProjectInteraction" && call.interactionKind === "selectedSegmentVisual"),
      "dragging a keyed property value must update the focused keyframe, not the segment visual directly"
    ).toBe(false);
    expect(
      liveValueCalls.some((call) => call.command === "updateProjectInteraction" && call.interactionKind === "keyframeEdit"),
      "keyed property value drag must use keyframeEdit interactions"
    ).toBe(true);
    await page.mouse.up();
  } finally {
    await app.close();
  }
});

test("focused keyframe deletion does not require exact playhead equality", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await dragMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await openVisualInspector(page);
    await page.getByRole("button", { name: "添加不透明度关键帧" }).first().click();
    await expect(page.getByLabel(/p0-moving-testsrc\.mp4 不透明度关键帧/)).toBeVisible({ timeout: 10_000 });

    await seekTimelinePlayhead(page, app, 120_000);
    await page.getByRole("tab", { name: "动画" }).click();
    await page.getByRole("button", { name: "不透明度关键帧" }).click();
    const beforeDeleteCount = keyframeIntentCount(await readProjectSessionCalls(app), "removeSelectedSegmentKeyframe");
    const focusedDelete = page.getByRole("button", { name: "删除不透明度关键帧" }).first();
    await expect(focusedDelete, "focused keyframe delete must stay enabled near the marker time").toBeEnabled();
    await focusedDelete.click();
    await expect
      .poll(async () => keyframeIntentCount(await readProjectSessionCalls(app), "removeSelectedSegmentKeyframe"), {
        timeout: 10_000
      })
      .toBe(beforeDeleteCount + 1);
    await expect(page.getByLabel(/p0-moving-testsrc\.mp4 不透明度关键帧/)).toHaveCount(0);
  } finally {
    await app.close();
  }
});

async function openVisualInspector(page: Page): Promise<void> {
  const visualTab = page.getByRole("tab", { name: "画面" });
  if ((await visualTab.count()) > 0) {
    await visualTab.click();
  }
  await expect(page.getByLabel("画面基础表单")).toBeVisible();
}

async function requiredBox(locator: ReturnType<Page["locator"]>, name: string): Promise<{ x: number; y: number; width: number; height: number }> {
  const box = await locator.boundingBox();
  expect(box, `${name} must be measurable`).not.toBeNull();
  return box!;
}

async function interactionCommandsSince(app: ProductJourneyAppController, index: number): Promise<string[]> {
  return callsSince(await readProjectSessionCalls(app), index)
    .map((call) => call.command)
    .filter((command) => command.includes("ProjectInteraction"));
}

function callsSince(calls: Awaited<ReturnType<typeof readProjectSessionCalls>>, index: number): Array<Record<string, any>> {
  return (calls as Array<Record<string, any>>).slice(index);
}

function commandCount(calls: Array<Record<string, any>>, command: string): number {
  return calls.filter((call) => call.command === command).length;
}

function latestResultRevision(calls: Awaited<ReturnType<typeof readProjectSessionCalls>>): number | null {
  const revisions = (calls as Array<Record<string, unknown>>)
    .map((call) => call.resultRevision)
    .filter((revision): revision is number => typeof revision === "number");
  return revisions.at(-1) ?? null;
}

function keyframeIntentCount(calls: Awaited<ReturnType<typeof readProjectSessionCalls>>, intentKind: string): number {
  return calls.filter((call) => call.command === "executeProjectIntent" && call.intentKind === intentKind && call.resultOk === true).length;
}

import { expect, test, type Page } from "@playwright/test";

import {
  USER_JOURNEY_MOVING_VIDEO,
  addMaterialToTimeline,
  addTextThroughProductPanel,
  capturePreviewEvidence,
  expectNoProductFallbackCalls,
  importMaterialThroughProductPicker,
  launchProductJourneyApp,
  readProjectSessionCalls,
  readRealtimePreviewHostCalls,
  readTaskRuntimeTelemetry,
  type ProductJourneyAppController
} from "./helpers/userJourney";

test.describe.configure({ timeout: 90_000 });

const INTERACTION_QUEUE_LATENCY_BUDGET_US = 2_000_000;

test("preview canvas drag streams Rust provisional updates and commits once", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    const beforeCalls = await readProjectSessionCalls(app);
    const beforeIndex = beforeCalls.length;
    const baseRevision = latestResultRevision(beforeCalls);
    expect(baseRevision, "fixture setup must produce a canonical project revision").not.toBeNull();

    const outline = page.getByLabel("预览选中框");
    await expect(outline).toBeVisible({ timeout: 10_000 });
    const box = await outline.boundingBox();
    expect(box, "selected preview outline must be measurable").not.toBeNull();

    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.mouse.down();
    await page.mouse.move(box!.x + box!.width / 2 + 72, box!.y + box!.height / 2 + 30, { steps: 8 });

    await expect
      .poll(async () => interactionCommandsSince(app, beforeIndex), { timeout: 10_000 })
      .toEqual(expect.arrayContaining(["beginProjectInteraction", "updateProjectInteraction"]));

    const liveCalls = callsSince(await readProjectSessionCalls(app), beforeIndex);
    expect(commandCount(liveCalls, "commitProjectInteraction"), "preview drag must not commit before pointer-up").toBe(0);
    expect(
      liveCalls.some((call) => call.command === "executeProjectIntent" && call.intentKind === "updateSelectedSegmentVisual"),
      "preview drag must not route live samples through canonical updateSelectedSegmentVisual"
    ).toBe(false);
    for (const update of liveCalls.filter((call) => call.command === "updateProjectInteraction")) {
      expect(update.interactionKind).toBe("selectedSegmentVisual");
      expect(update.resultRevision).toBe(baseRevision);
      expect(update.revisionUnchanged).toBe(true);
      expect(update.resultDeltaCommand).toBe("updateSegmentVisual");
    }
    expectCoalescedInteractionTelemetry(liveCalls, "selectedSegmentVisual", "preview drag");
    await expect(outline).toHaveAttribute("data-interaction-source", "rust-provisional");
    await expect(outline).toHaveAttribute("data-interaction-kind", "selectedSegmentVisual");

    await page.mouse.up();

    await expect
      .poll(async () => commandCount(callsSince(await readProjectSessionCalls(app), beforeIndex), "commitProjectInteraction"), {
        timeout: 10_000
      })
      .toBe(1);
    const committedCalls = callsSince(await readProjectSessionCalls(app), beforeIndex);
    const commit = committedCalls.findLast((call) => call.command === "commitProjectInteraction");
    expect(commit?.interactionKind).toBe("selectedSegmentVisual");
    expect(commit?.resultRevision).toBe((baseRevision ?? 0) + 1);
    expect(
      committedCalls.some((call) => call.command === "executeProjectIntent" && call.intentKind === "updateSelectedSegmentVisual"),
      "preview drag commit must use commitProjectInteraction rather than executeProjectIntent"
    ).toBe(false);
    await expectRealtimeHostInteractionRefresh(app, "preview drag");
    await expectInteractionQueueLatencyWithinBudget(page, "preview drag");

    await page.getByRole("button", { name: "撤销" }).click();
    await expect
      .poll(async () => {
        const undoCalls = callsSince(await readProjectSessionCalls(app), beforeIndex).filter(
          (call) => call.command === "executeProjectIntent" && call.intentKind === "undoTimelineEdit"
        );
        return undoCalls.at(-1)?.resultOk === true;
      }, { timeout: 10_000 })
      .toBe(true);
    await expect(outline, "one undo should revert the single committed drag interaction").not.toHaveAttribute(
      "data-interaction-source",
      "rust-provisional"
    );
  } finally {
    await app.close();
  }
});

test("preview text rotate handle changes native compositor rotation evidence", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);
  const textContent = "聚合旋转文字";

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addTextThroughProductPanel(page, app, textContent);

    const beforeOverlay = await waitForTextOverlayEvidence(page, textContent);
    const beforeCalls = await readProjectSessionCalls(app);
    const beforeIndex = beforeCalls.length;
    const baseRevision = latestResultRevision(beforeCalls);
    expect(baseRevision, "text rotate setup must produce a canonical project revision").not.toBeNull();

    const rotateHandle = page.getByRole("button", { name: "旋转文字", exact: true });
    await expect(rotateHandle, "selected text must expose the product rotate affordance").toBeVisible({ timeout: 10_000 });
    const box = await rotateHandle.boundingBox();
    expect(box, "preview rotate handle must be measurable").not.toBeNull();

    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.mouse.down();
    await page.mouse.move(box!.x + box!.width / 2 + 72, box!.y + box!.height / 2 + 96, { steps: 8 });

    await expect
      .poll(async () => interactionCommandsSince(app, beforeIndex), { timeout: 10_000 })
      .toEqual(expect.arrayContaining(["beginProjectInteraction", "updateProjectInteraction"]));

    const liveCalls = callsSince(await readProjectSessionCalls(app), beforeIndex);
    expect(commandCount(liveCalls, "commitProjectInteraction"), "rotate drag must not commit before pointer-up").toBe(0);
    expect(
      liveCalls.some((call) => call.command === "executeProjectIntent" && call.intentKind === "updateSelectedSegmentVisual"),
      "rotate drag must not route live samples through canonical updateSelectedSegmentVisual"
    ).toBe(false);
    for (const update of liveCalls.filter((call) => call.command === "updateProjectInteraction")) {
      expect(update.interactionKind).toBe("selectedSegmentVisual");
      expect(update.resultRevision).toBe(baseRevision);
      expect(update.revisionUnchanged).toBe(true);
      expect(update.visualPatch?.rotationDegrees, "rotate samples must send idempotent absolute rotation").toEqual(expect.any(Number));
      expect(update.visualPatch?.rotationDeltaDegrees, "rotate samples must not send cumulative rotation deltas").toBeUndefined();
    }
    expectCoalescedInteractionTelemetry(liveCalls, "selectedSegmentVisual", "preview text rotate");

    await page.mouse.up();

    await expect
      .poll(async () => commandCount(callsSince(await readProjectSessionCalls(app), beforeIndex), "commitProjectInteraction"), {
        timeout: 10_000
      })
      .toBe(1);
    const committedCalls = callsSince(await readProjectSessionCalls(app), beforeIndex);
    const commit = committedCalls.findLast((call) => call.command === "commitProjectInteraction");
    expect(commit?.interactionKind).toBe("selectedSegmentVisual");
    expect(commit?.resultRevision).toBe((baseRevision ?? 0) + 1);
    await expectRealtimeHostInteractionRefresh(app, "preview text rotate");
    const afterOverlay = await waitForTextOverlayRotationChangedEvidence(
      page,
      textContent,
      beforeOverlay.visualRotationDegrees
    );
    expect(
      Math.abs(afterOverlay.visualRotationDegrees - beforeOverlay.visualRotationDegrees),
      "native renderGraphGpuComposited text overlay rotation must change after handle drag"
    ).toBeGreaterThan(4);
    await expectInteractionQueueLatencyWithinBudget(page, "preview text rotate");
  } finally {
    await app.close();
  }
});

test("inspector visual slider uses Rust interaction sessions instead of debounce commands", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await openVisualInspector(page);
    const beforeCalls = await readProjectSessionCalls(app);
    const beforeIndex = beforeCalls.length;
    const baseRevision = latestResultRevision(beforeCalls);
    expect(baseRevision).not.toBeNull();

    const slider = page.getByLabel("不透明度滑杆");
    await expect(slider).toBeVisible();
    const box = await slider.boundingBox();
    expect(box, "opacity slider must be measurable").not.toBeNull();

    await page.mouse.move(box!.x + box!.width * 0.45, box!.y + box!.height / 2);
    await page.mouse.down();
    await page.mouse.move(box!.x + box!.width * 0.2, box!.y + box!.height / 2, { steps: 5 });

    await expect
      .poll(async () => interactionCommandsSince(app, beforeIndex), { timeout: 10_000 })
      .toEqual(expect.arrayContaining(["beginProjectInteraction", "updateProjectInteraction"]));
    const liveCalls = callsSince(await readProjectSessionCalls(app), beforeIndex);
    expect(commandCount(liveCalls, "commitProjectInteraction"), "slider drag must not commit before pointer-up").toBe(0);
    expect(
      liveCalls.some((call) => call.command === "executeProjectIntent" && call.intentKind === "updateSelectedSegmentVisual"),
      "slider drag must not use the old 160ms canonical command debounce path"
    ).toBe(false);
    for (const update of liveCalls.filter((call) => call.command === "updateProjectInteraction")) {
      expect(update.interactionKind).toBe("selectedSegmentVisual");
      expect(update.resultRevision).toBe(baseRevision);
      expect(update.revisionUnchanged).toBe(true);
    }
    expectCoalescedInteractionTelemetry(liveCalls, "selectedSegmentVisual", "inspector visual slider");

    await page.mouse.up();

    await expect
      .poll(async () => commandCount(callsSince(await readProjectSessionCalls(app), beforeIndex), "commitProjectInteraction"), {
        timeout: 10_000
      })
      .toBe(1);
    const committedCalls = callsSince(await readProjectSessionCalls(app), beforeIndex);
    expect(
      committedCalls.some((call) => call.command === "executeProjectIntent" && call.intentKind === "updateSelectedSegmentVisual"),
      "slider commit must use commitProjectInteraction rather than executeProjectIntent"
    ).toBe(false);
    await expectRealtimeHostInteractionRefresh(app, "inspector visual slider");
    await expectInteractionQueueLatencyWithinBudget(page, "inspector visual slider");
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

function expectCoalescedInteractionTelemetry(calls: Array<Record<string, any>>, kind: string, label: string): void {
  const updates = calls.filter((call) => call.command === "updateProjectInteraction" && call.interactionKind === kind);
  expect(updates.length, `${label} must record interaction update telemetry`).toBeGreaterThan(0);
  const latest = updates.at(-1);
  expect(latest?.acceptedSequence, `${label} must record accepted monotonic samples`).toBeGreaterThanOrEqual(1);
  expect(latest?.coalescedThrough, `${label} must expose coalesced-through sample telemetry`).toBeGreaterThanOrEqual(
    latest?.acceptedSequence ?? 0
  );
}

async function expectRealtimeHostInteractionRefresh(app: ProductJourneyAppController, label: string): Promise<void> {
  await expect
    .poll(
      async () =>
        (await readRealtimePreviewHostCalls(app)).some(
          (call) => call.kind === "updateProjectSessionSnapshot" && typeof call.interactionId === "string"
        ),
      { timeout: 5_000 }
    )
    .toBe(true);
  const hostCalls = await readRealtimePreviewHostCalls(app);
  expect(
    hostCalls.some((call) => call.kind === "updateProjectSessionSnapshot" && typeof call.interactionId === "string"),
    `${label} must refresh the realtime host with an interaction snapshot`
  ).toBe(true);
  expectNoProductFallbackCalls(hostCalls);
}

async function expectInteractionQueueLatencyWithinBudget(page: Page, label: string): Promise<void> {
  const telemetry = await readTaskRuntimeTelemetry(page);
  expect(telemetry.queueLatencyUs.p95 ?? 0, `${label} queue latency p95 must stay within the interaction budget`).toBeLessThanOrEqual(
    INTERACTION_QUEUE_LATENCY_BUDGET_US
  );
  expect(telemetry.coalescedCount, `${label} must expose coalescing telemetry`).toBeGreaterThanOrEqual(0);
  expect(telemetry.staleRejectedCount, `${label} must expose stale generation rejection telemetry`).toBeGreaterThanOrEqual(0);
}

type ActiveTextOverlayEvidence = NonNullable<
  NonNullable<
    NonNullable<Awaited<ReturnType<typeof capturePreviewEvidence>>["hostState"]>["contentEvidence"]
  >["activeTextOverlays"]
>[number];

async function waitForTextOverlayEvidence(page: Page, expectedContent: string): Promise<ActiveTextOverlayEvidence> {
  const deadline = Date.now() + 10_000;
  let lastEvidence: unknown = null;

  while (Date.now() < deadline) {
    const previewEvidence = await capturePreviewEvidence(page);
    const evidence = previewEvidence.hostState?.contentEvidence;
    const overlay = evidence?.activeTextOverlays?.find((candidate) => candidate.content === expectedContent);
    lastEvidence = {
      source: evidence?.source ?? null,
      targetTimeMicroseconds: evidence?.targetTimeMicroseconds ?? 0,
      activeTextOverlays: evidence?.activeTextOverlays ?? [],
      expectedContent
    };
    if (evidence?.source === "renderGraphGpuComposited" && overlay !== undefined) {
      return overlay;
    }
    await page.waitForTimeout(200);
  }

  throw new Error(`Timed out waiting for text overlay evidence: ${JSON.stringify(lastEvidence)}`);
}

async function waitForTextOverlayRotationChangedEvidence(
  page: Page,
  expectedContent: string,
  previousRotationDegrees: number
): Promise<ActiveTextOverlayEvidence> {
  const deadline = Date.now() + 10_000;
  let lastEvidence: unknown = null;

  while (Date.now() < deadline) {
    const previewEvidence = await capturePreviewEvidence(page);
    const evidence = previewEvidence.hostState?.contentEvidence;
    const overlay = evidence?.activeTextOverlays?.find((candidate) => candidate.content === expectedContent);
    lastEvidence = {
      source: evidence?.source ?? null,
      targetTimeMicroseconds: evidence?.targetTimeMicroseconds ?? 0,
      activeTextOverlays: evidence?.activeTextOverlays ?? [],
      expectedContent,
      previousRotationDegrees
    };
    if (
      evidence?.source === "renderGraphGpuComposited" &&
      overlay !== undefined &&
      overlay.visualRotationDegrees !== previousRotationDegrees
    ) {
      return overlay;
    }
    await page.waitForTimeout(200);
  }

  throw new Error(`Timed out waiting for text overlay rotation evidence: ${JSON.stringify(lastEvidence)}`);
}

function latestResultRevision(calls: Awaited<ReturnType<typeof readProjectSessionCalls>>): number | null {
  const revisions = (calls as Array<Record<string, unknown>>)
    .map((call) => call.resultRevision)
    .filter((revision): revision is number => typeof revision === "number");
  return revisions.at(-1) ?? null;
}

import { expect, test, type Page } from "@playwright/test";

import {
  USER_JOURNEY_MOVING_VIDEO,
  dragMaterialToTimeline,
  importMaterialThroughProductPicker,
  launchProductJourneyApp,
  readProjectSessionCalls,
  type ProductJourneyAppController
} from "./helpers/userJourney";

test.describe.configure({ timeout: 90_000 });

test("preview canvas drag streams Rust provisional updates and commits once", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await dragMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
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

test("inspector visual slider uses Rust interaction sessions instead of debounce commands", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await dragMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
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

function latestResultRevision(calls: Awaited<ReturnType<typeof readProjectSessionCalls>>): number | null {
  const revisions = (calls as Array<Record<string, unknown>>)
    .map((call) => call.resultRevision)
    .filter((revision): revision is number => typeof revision === "number");
  return revisions.at(-1) ?? null;
}

import { expect, test, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

import {
  addMaterialToTimeline,
  importMaterialThroughProductPicker,
  launchProductJourneyApp,
  productionEffectsPreviewExportParity,
  readNativeCommandObservations,
  readProjectSessionCalls,
  readRealtimePreviewHostCalls,
  requestProjectSessionPreviewFrameCount,
  USER_JOURNEY_MOVING_VIDEO,
  waitForCompositedPreviewEvidence,
  expectNoProductFallbackCalls
} from "./helpers/userJourney";

test.describe.configure({ timeout: 90_000 });

const REPO_ROOT = join(process.cwd(), "../..");

test("phase19 production-effects desktop workflow requires Rust capability contracts before visible controls", async () => {
  const generatedDraft = await readFile(join(REPO_ROOT, "apps/desktop-electron/src/generated/Draft.ts"), "utf8");
  const featurePanel = await readFile(join(REPO_ROOT, "apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx"), "utf8");
  const inspector = await readFile(join(REPO_ROOT, "apps/desktop-electron/src/renderer/workspace/Inspector.tsx"), "utf8");

  expect(
    generatedDraft,
    "desktop controls must not become functional until generated contracts expose registry-backed Phase 19 semantics"
  ).toContain("EffectCapabilityRegistry");
  expect(featurePanel, "effect/filter/transition categories must be wired to Rust capabilities, not static placeholder strings").toContain(
    "productionEffectCapabilities"
  );
  expect(inspector, "visible speed/effect/transition controls must emit project-session intents backed by Rust").toContain(
    "beginProductionEffectInteraction"
  );
});

test("phase19 production-effects desktop workflow requires no-fallback product evidence", async () => {
  const productJourney = await readFile(join(REPO_ROOT, "apps/desktop-electron/tests/helpers/userJourney.ts"), "utf8");

  expect(productJourney).toContain("renderGraphGpuComposited");
  expect(productJourney).toContain("fallbackActive");
  expect(
    productJourney,
    "Phase 19 desktop E2E must add production-effects preview/export parity assertions before controls are product-complete"
  ).toContain("productionEffectsPreviewExportParity");
});

test("phase19 production-effects desktop workflow includes template import coverage", async () => {
  const phase19ProductionEffectsTemplateImportCoverage = await readFile(
    join(REPO_ROOT, "apps/desktop-electron/tests/template-import.spec.ts"),
    "utf8"
  );

  expect(phase19ProductionEffectsTemplateImportCoverage).toContain("phase19ImportedProductionEffectReportEvidence");
  expect(phase19ProductionEffectsTemplateImportCoverage).toContain("expectPhase19ImportedProductionEffectsCanonical");
  expect(phase19ProductionEffectsTemplateImportCoverage).toContain("provider-private-skin-lut");
  expect(phase19ProductionEffectsTemplateImportCoverage).toContain("native-effect-beauty-retouch");
});

test("phase19 timeline and preview affordances use project interaction sessions", async () => {
  const timeline = await readFile(join(REPO_ROOT, "apps/desktop-electron/src/renderer/workspace/Timeline.tsx"), "utf8");
  const previewMonitor = await readFile(join(REPO_ROOT, "apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx"), "utf8");
  const projectInteraction = await readFile(join(REPO_ROOT, "apps/desktop-electron/src/renderer/workspace/projectInteraction.ts"), "utf8");

  expect(timeline).toContain("data-phase19-transition-grip");
  expect(timeline).toContain("selectedTransitionDuration");
  expect(timeline).toContain("data-phase19-retime-grip");
  expect(timeline).toContain("selectedSegmentRetime");
  expect(previewMonitor).toContain("data-phase19-mask-ghost");
  expect(previewMonitor).toContain("selectedSegmentMask");
  expect(previewMonitor).toContain("data-phase19-preview-proxy");
  expect(projectInteraction).toContain("PHASE19_PROJECT_INTERACTION_KINDS");
});

test("phase19 visible controls apply through Rust intents and coalesced interactions", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await app.resizeMainWindow(1280, 800);
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    await waitForCompositedPreviewEvidence(page, app, 15_000);
    const artifactRequestsBefore = requestProjectSessionPreviewFrameCount(await readNativeCommandObservations(app));

    await page.getByRole("tab", { name: "变速" }).click();
    await nudgeRange(page, page.getByRole("slider", { name: "变速倍率" }), "ArrowLeft", 10);
    await expectProjectInteractionSettled(app, "selectedSegmentRetime");

    await selectPhase19Category(page, "特效");
    await expect(page.getByRole("button", { name: /外部光效.*暂不支持/ })).toBeDisabled();
    await clickProjectIntentButton(app, page.getByRole("button", { name: /高斯模糊/ }), "applySelectedSegmentEffect");
    await page.getByRole("tab", { name: "效果" }).click();
    await dragRange(page, page.getByRole("slider", { name: "模糊" }), 0.72);
    await expectProjectInteractionSettled(app, "selectedSegmentEffect");

    await selectPhase19Category(page, "转场");
    await expect(page.getByRole("button", { name: /叠化/ })).toBeVisible();

    await page.getByRole("tab", { name: "蒙版" }).click();
    await clickProjectIntentButton(app, page.getByRole("button", { name: "矩形" }), "setSelectedSegmentMask");
    await expect(page.locator('[data-phase19-mask-ghost="true"][data-phase19-preview-proxy="mask"]')).toBeVisible();
    await dragElement(page, page.locator(".preview-mask-handle.bottom-right"), 18, 12);
    await expectProjectInteractionSettled(app, "selectedSegmentMask");

    await page.getByRole("tab", { name: "混合" }).click();
    await dragRange(page, page.getByRole("slider", { name: "混合透明度" }), 0.42);
    await expectProjectInteractionSettled(app, "selectedSegmentBlend");

    const previewEvidence = await waitForCompositedPreviewEvidence(page, app, 15_000);
    const projectSessionCalls = await readProjectSessionCalls(app);
    const nativeCalls = await readNativeCommandObservations(app);
    productionEffectsPreviewExportParity({
      previewEvidence,
      projectSessionCalls,
      nativeCommandObservations: nativeCalls
    });
    expect(requestProjectSessionPreviewFrameCount(nativeCalls)).toBe(artifactRequestsBefore);
    expectNoProductFallbackCalls(await readRealtimePreviewHostCalls(app));
  } finally {
    await app.close();
  }
});

test("phase19 controls fit desktop regression viewports", async () => {
  const { app, page } = await launchProductJourneyApp([USER_JOURNEY_MOVING_VIDEO]);

  try {
    await importMaterialThroughProductPicker(app, page, USER_JOURNEY_MOVING_VIDEO);
    await addMaterialToTimeline(app, page, USER_JOURNEY_MOVING_VIDEO);
    for (const viewport of [
      { width: 1280, height: 800 },
      { width: 1120, height: 720 }
    ]) {
      await app.resizeMainWindow(viewport.width, viewport.height);
      await page.waitForTimeout(250);
      await page.getByRole("tab", { name: "效果" }).click();
      await expectPhase19LayoutWithinViewport(page, viewport.width, viewport.height);
      await page.getByRole("tab", { name: "蒙版" }).click();
      await expectPhase19LayoutWithinViewport(page, viewport.width, viewport.height);
      await page.getByRole("tab", { name: "混合" }).click();
      await expectPhase19LayoutWithinViewport(page, viewport.width, viewport.height);
    }
  } finally {
    await app.close();
  }
});

type ProjectSessionCall = Awaited<ReturnType<typeof readProjectSessionCalls>>[number];

async function selectPhase19Category(page: Page, category: string): Promise<void> {
  const topFeatureNav = page.getByRole("navigation", { name: "顶部功能区" });
  const visibleButton = topFeatureNav.getByRole("button", { name: category });
  if ((await visibleButton.count()) > 0) {
    await visibleButton.click();
    return;
  }
  await page.getByRole("button", { name: "更多功能" }).click();
  await page.getByRole("menu", { name: "更多功能菜单" }).getByRole("menuitemradio", { name: category }).click();
}

async function clickProjectIntentButton(
  app: Awaited<ReturnType<typeof launchProductJourneyApp>>["app"],
  button: import("@playwright/test").Locator,
  intentKind: string
): Promise<void> {
  const before = await readProjectSessionCalls(app);
  await expect(button).toBeEnabled({ timeout: 15_000 });
  await button.click();
  await expect
    .poll(async () => latestSuccessfulIntent(await readProjectSessionCalls(app), intentKind, before.length), {
      timeout: 20_000
    })
    .toBe(true);
}

async function expectProjectInteractionSettled(
  app: Awaited<ReturnType<typeof launchProductJourneyApp>>["app"],
  interactionKind: string
): Promise<void> {
  await expect
    .poll(async () => {
      const allCalls = await readProjectSessionCalls(app);
      const calls = allCalls.filter((call) => call.interactionKind === interactionKind);
      const interactionIds = new Set(calls.map((call) => call.interactionId).filter((id): id is string => id !== null));
      const begins = calls.filter((call) => call.command === "beginProjectInteraction").length;
      const updates = calls.filter((call) => call.command === "updateProjectInteraction");
      const commits = allCalls.filter(
        (call) => call.command === "commitProjectInteraction" && call.interactionId !== null && interactionIds.has(call.interactionId)
      );
      if (begins === 0 || updates.length === 0 || commits.length === 0) {
        return `pending:${begins}:${updates.length}:${commits.length}`;
      }
      const mutatedUpdate = updates.find((call) => call.revisionUnchanged !== true);
      if (mutatedUpdate !== undefined) {
        return `update-mutated-revision:${JSON.stringify({
          resultOk: mutatedUpdate.resultOk,
          resultErrorKind: mutatedUpdate.resultErrorKind,
          resultErrorMessage: mutatedUpdate.resultErrorMessage,
          expectedRevision: mutatedUpdate.expectedRevision,
          resultRevision: mutatedUpdate.resultRevision,
          sequence: mutatedUpdate.interactionSequence
        })}`;
      }
      if (commits.length !== 1) {
        return `commit-count:${commits.length}`;
      }
      return "settled";
    }, { timeout: 20_000 })
    .toBe("settled");
}

async function dragRange(
  page: Page,
  slider: import("@playwright/test").Locator,
  endRatio: number,
  startRatio = 0.35
): Promise<void> {
  await expect(slider).toBeVisible({ timeout: 10_000 });
  const box = await slider.boundingBox();
  if (box === null) {
    throw new Error("Phase 19 slider is not visible");
  }
  const startX = box.x + box.width * Math.max(0.05, Math.min(0.95, startRatio));
  const endX = box.x + box.width * Math.max(0.05, Math.min(0.95, endRatio));
  const y = box.y + box.height / 2;
  await page.mouse.move(startX, y);
  await page.mouse.down();
  await page.mouse.move(endX, y, { steps: 8 });
  await page.mouse.up();
  await page.evaluate(() => window.dispatchEvent(new MouseEvent("mouseup", { bubbles: true })));
  await slider.evaluate((element) => (element as HTMLInputElement).blur());
}

async function nudgeRange(
  page: Page,
  slider: import("@playwright/test").Locator,
  key: "ArrowLeft" | "ArrowRight",
  steps: number
): Promise<void> {
  await expect(slider).toBeVisible({ timeout: 10_000 });
  await slider.focus();
  for (let index = 0; index < steps; index += 1) {
    await page.keyboard.press(key);
  }
  await page.evaluate(() => window.dispatchEvent(new MouseEvent("mouseup", { bubbles: true })));
  await slider.evaluate((element) => (element as HTMLInputElement).blur());
}

async function dragElement(
  page: Page,
  element: import("@playwright/test").Locator,
  deltaX: number,
  deltaY: number
): Promise<void> {
  await expect(element).toBeVisible({ timeout: 10_000 });
  const box = await element.boundingBox();
  if (box === null) {
    throw new Error("Phase 19 drag element is not visible");
  }
  const startX = box.x + box.width / 2;
  const startY = box.y + box.height / 2;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  await page.mouse.move(startX + deltaX, startY + deltaY, { steps: 6 });
  await page.mouse.up();
}

async function expectPhase19LayoutWithinViewport(
  page: Page,
  width: number,
  height: number
): Promise<void> {
  for (const locator of [
    page.locator('[aria-label="素材面板"]'),
    page.locator('[aria-label="预览窗口"]'),
    page.locator('[aria-label="属性检查器"]'),
    page.locator('[aria-label="时间线"]'),
    page.locator(".production-inspector-section").first()
  ]) {
    const box = await locator.boundingBox();
    expect(box, "Phase 19 layout target must be visible").not.toBeNull();
    expect(box!.x, "Phase 19 target left clipped").toBeGreaterThanOrEqual(0);
    expect(box!.y, "Phase 19 target top clipped").toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width, "Phase 19 target right clipped").toBeLessThanOrEqual(width + 1);
    expect(box!.y + box!.height, "Phase 19 target bottom clipped").toBeLessThanOrEqual(height + 1);
  }
  await expect(page.locator(".production-inspector-section").first()).not.toContainText(/\n\s*\n\s*\n/);
}

function latestSuccessfulIntent(calls: ProjectSessionCall[], intentKind: string, startIndex: number): boolean {
  const matching = calls
    .slice(startIndex)
    .filter((call) => call.command === "executeProjectIntent" && call.intentKind === intentKind);
  return matching.some((call) => call.resultOk === true);
}

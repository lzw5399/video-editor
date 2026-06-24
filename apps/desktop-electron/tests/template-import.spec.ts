import { expect, test, type Page } from "@playwright/test";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { copyFile, mkdir, readFile, unlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";

import {
  USER_JOURNEY_LONG_MOVING_VIDEO,
  USER_JOURNEY_OVERLAY_IMAGE,
  expectNoProductFallbackCalls,
  launchProductJourneyApp,
  readNativeCommandObservations,
  readProjectSessionCalls,
  readRealtimePreviewHostCalls,
  readTimelineSegments,
  waitForCompositedPreviewEvidence,
  type ProductJourneyAppController
} from "./helpers/userJourney";

test.describe.configure({ timeout: 120_000 });

const REPO_ROOT = join(process.cwd(), "../..");
const KAIPAI_FIXTURE_ROOT = join(REPO_ROOT, "fixtures/kaipai");
const BUNDLED_TEXT_FONT = join(REPO_ROOT, "assets/fonts/noto-sans-cjk-sc/NotoSansCJKsc-Regular.otf");
const FORBIDDEN_PROJECT_JSON_TOKENS = [
  "templateId",
  "recipeId",
  "formulaTaskId",
  "formulaRequestId",
  "rawFormula",
  "\"formula\"",
  "safeArea",
  "remoteRuntimeUrl",
  "remoteRenderUrl",
  "renderUrl",
  "http://",
  "https://",
  "kaipai",
  "provider"
] as const;
const FORBIDDEN_REPORT_COPY = [
  "formula.",
  "resources/",
  "externalId",
  "externalPath",
  "provenance",
  "token",
  "secret",
  "http://",
  "https://"
] as const;
const TEMPLATE_CATEGORY_LABEL = "模板导入";

type TemplatePickerSelection = {
  bundlePath: string;
  resourceRoot: string;
};

type TemplateResourceRef = {
  uri: string;
  kind: string;
};

test("product user imports offline Kaipai template, sees report copy, previews, saves cleanly, and exports", async () => {
  const projectBundlePath = join(
    tmpdir(),
    `video-editor-template-import-${Date.now()}-${Math.random().toString(16).slice(2)}.veproj`
  );
  const missingResource = await prepareTemplateFixture("missing-resource", "negative/missing-resource.json");
  const textSticker = await prepareTemplateFixture("text-sticker", "positive/text-sticker.json");
  const nativeEffect = await prepareTemplateFixture("native-effect", "negative/native-effect.json");
  const outputPath = join(dirname(projectBundlePath), "template-import-export.mp4");
  const { app, page } = await launchProductJourneyApp([], {
    VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE: projectBundlePath,
    VIDEO_EDITOR_TEST_OPEN_TEMPLATE_BUNDLE: JSON.stringify([missingResource, textSticker, nativeEffect])
  });

  try {
    await importTemplateThroughProductPanel(app, page);
    await expectTemplateReportSummary(page, {
      "已支持": 0,
      "近似还原": 0,
      "已舍弃": 1,
      "缺少资源": 1,
      "需本地效果": 0
    });
    await expect(page.getByText("资源缺失，相关片段已跳过")).toBeVisible();
    await expectTemplateReportRows(page, ["missingResource", "dropped"]);
    await expectTemplateStatusRowsUseDistinctBorders(page, ["missingResource", "dropped"]);
    await clickTemplateReportRow(page, "missingResource", /资源缺失/);
    await expectTemplateReportFocusState(page, /仅查看报告/);
    await expectNoSelectedTimelineSegment(page);

    await importTemplateThroughProductPanel(app, page);
    await expectTemplateReportSummary(page, {
      "已支持": 1,
      "近似还原": 1,
      "已舍弃": 1,
      "缺少资源": 0,
      "需本地效果": 0
    });
    await expect(page.getByText("文本已接入本地草稿")).toBeVisible();
    await expect(page.getByText("字体已使用本地替代")).toBeVisible();
    await expect(page.getByText("文字效果未写入草稿")).toBeVisible();
    await expectTemplateReportRows(page, ["dropped", "approximated", "supported"]);
    await clickTemplateReportRow(page, "supported", /文本已接入本地草稿/);
    await expectTemplateReportFocusState(page, /已定位/);
    await expectSelectedTimelineSegment(page, { targetStartUs: 600_000 });
    await expectLatestPreviewSeek(app, 600_000);

    await importTemplateThroughProductPanel(app, page);
    await expectTemplateReportSummary(page, {
      "已支持": 2,
      "近似还原": 0,
      "已舍弃": 1,
      "缺少资源": 0,
      "需本地效果": 1
    });
    await expect(page.getByText("本地效果能力待补齐")).toBeVisible();
    await expect(page.getByText("片段已跳过")).toBeVisible();
    await expectTemplateReportRows(page, ["needsNativeEffect", "dropped", "supported", "supported"]);
    await expectTemplateStatusRowsUseDistinctBorders(page, ["needsNativeEffect", "dropped", "supported"]);
    await clickTemplateReportRow(page, "needsNativeEffect", /本地效果能力待补齐/);
    await expectTemplateReportFocusState(page, /仅查看报告/);
    await expectSelectedTimelineSegment(page, { targetStartUs: 600_000 });
    const seekCountBeforeRapidNavigation = (await readRealtimePreviewHostCalls(app)).filter((call) => call.kind === "seek").length;
    await navigateTemplateReportRowsWithKeyboard(page, 3);
    await expectTemplateReportFocusState(page, /已定位/);
    await expectSelectedTimelineSegment(page, { targetStartUs: 0 });
    await expectLatestPreviewSeek(app, 0);
    const seekCountAfterRapidNavigation = (await readRealtimePreviewHostCalls(app)).filter((call) => call.kind === "seek").length;
    expect(
      seekCountAfterRapidNavigation - seekCountBeforeRapidNavigation,
      "rapid report row navigation should coalesce preview seeks instead of seeking every intermediate row"
    ).toBeLessThanOrEqual(2);

    const reportText = (await page.getByLabel("模板适配报告").textContent()) ?? "";
    for (const forbidden of FORBIDDEN_REPORT_COPY) {
      expect(reportText, `template report must not expose ${forbidden}`).not.toContain(forbidden);
    }

    await expect
      .poll(async () => (await readTimelineSegments(page)).length, { timeout: 30_000 })
      .toBeGreaterThan(0);
    const importCalls = await readTemplateImportCalls(app);
    expect(importCalls).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          command: "importKaipaiFormulaBundle",
          expectedRevision: expect.any(Number),
          hasDraftField: false,
          resultOk: true
        })
      ])
    );
    expect(importCalls).toHaveLength(3);
    const latestDeltaImportCall = [...importCalls]
      .reverse()
      .find((call) => call.resultOk === true && call.resultDeltaCommand === "importTemplate");
    expect(latestDeltaImportCall, "template import must record the Rust response delta facts").toBeDefined();
    expect(latestDeltaImportCall).toEqual(
      expect.objectContaining({
        resultEventKinds: expect.arrayContaining(["templateImported"]),
        resultDeltaCommand: "importTemplate",
        resultDeltaChangedDomains: expect.arrayContaining(["track", "timing", "visual", "material", "canvas"]),
        resultDeltaChangedRangeSources: expect.arrayContaining(["fullDraft"]),
        resultDeltaFullDraft: true,
        resultDeltaConsumerDomains: expect.arrayContaining(["preview", "exportPrep", "graphSnapshot", "previewCache"])
      })
    );

    const firstFrame = await waitForCompositedPreviewEvidence(page, app, 12_000, -1);
    expect(firstFrame.hostState?.contentEvidence?.source).toBe("renderGraphGpuComposited");
    expect(firstFrame.hostState?.fallbackActive).toBe(false);
    expectNoProductFallbackCalls(await readRealtimePreviewHostCalls(app));

    const projectJson = await readFile(join(projectBundlePath, "project.json"), "utf8");
    assertProjectJsonIsCanonical(projectJson);

    await exportProductProject(page, app, outputPath);
    expectNoProductFallbackCalls(await readRealtimePreviewHostCalls(app));
  } finally {
    await app.close();
    await unlink(outputPath).catch(() => undefined);
  }
});

async function prepareTemplateFixture(
  family: string,
  fixturePath: string,
  options: { missingUris?: string[] } = {}
): Promise<TemplatePickerSelection> {
  const sourceBundlePath = join(KAIPAI_FIXTURE_ROOT, fixturePath);
  const caseRoot = join(
    tmpdir(),
    `video-editor-template-import-${family}-${Date.now()}-${Math.random().toString(16).slice(2)}`
  );
  const bundlePath = join(caseRoot, basename(fixturePath));
  const resourceRoot = join(
    caseRoot,
    "resources"
  );
  const fixture = JSON.parse(await readFile(sourceBundlePath, "utf8")) as unknown;
  const missingUris = new Set(options.missingUris ?? []);
  const seededSha256ByUri = new Map<string, string>();

  for (const resource of collectTemplateResourceRefs(fixture)) {
    if (missingUris.has(resource.uri)) {
      continue;
    }
    const outputPath = await seedTemplateResource(resourceRoot, resource);
    seededSha256ByUri.set(resource.uri, await sha256File(outputPath));
  }

  applyTemplateResourceSha256(fixture, seededSha256ByUri);
  await mkdir(dirname(bundlePath), { recursive: true });
  await writeFile(bundlePath, `${JSON.stringify(fixture, null, 2)}\n`, "utf8");

  return { bundlePath, resourceRoot };
}

function collectTemplateResourceRefs(value: unknown): TemplateResourceRef[] {
  const refs = new Map<string, string>();
  if (isRecord(value)) {
    collectTemplateResourceRef(value.sourceMedia, refs);
    if (Array.isArray(value.directMaterials)) {
      for (const material of value.directMaterials) {
        collectTemplateResourceRef(material, refs);
      }
    }
    if (Array.isArray(value.resources)) {
      for (const resource of value.resources) {
        collectTemplateResourceRef(resource, refs);
      }
    }
  }

  return Array.from(refs, ([uri, kind]) => ({ uri, kind }));
}

function collectTemplateResourceRef(value: unknown, refs: Map<string, string>): void {
  if (!isRecord(value) || typeof value.uri !== "string" || typeof value.kind !== "string") {
    return;
  }
  refs.set(value.uri, value.kind);
}

async function seedTemplateResource(resourceRoot: string, resource: TemplateResourceRef): Promise<string> {
  const outputPath = join(resourceRoot, resource.uri);
  await mkdir(dirname(outputPath), { recursive: true });
  if (existsSync(outputPath)) {
    return outputPath;
  }

  if (resource.kind === "video") {
    await copyFile(USER_JOURNEY_LONG_MOVING_VIDEO, outputPath);
    return outputPath;
  }
  if (resource.kind === "image" || resource.kind === "sticker") {
    await copyFile(USER_JOURNEY_OVERLAY_IMAGE, outputPath);
    return outputPath;
  }
  if (resource.kind === "font") {
    await copyFile(BUNDLED_TEXT_FONT, outputPath);
    return outputPath;
  }

  throw new Error(`Unsupported template fixture resource ${resource.kind} at ${resource.uri}`);
}

async function sha256File(path: string): Promise<string> {
  const bytes = await readFile(path);
  return createHash("sha256").update(bytes).digest("hex");
}

function applyTemplateResourceSha256(value: unknown, sha256ByUri: Map<string, string>): void {
  if (Array.isArray(value)) {
    for (const item of value) {
      applyTemplateResourceSha256(item, sha256ByUri);
    }
    return;
  }

  if (!isRecord(value)) {
    return;
  }

  if (typeof value.uri === "string" && typeof value.sha256 === "string" && sha256ByUri.has(value.uri)) {
    value.sha256 = sha256ByUri.get(value.uri);
  }

  for (const child of Object.values(value)) {
    applyTemplateResourceSha256(child, sha256ByUri);
  }
}

async function importTemplateThroughProductPanel(app: ProductJourneyAppController, page: Page): Promise<void> {
  const nextCount = (await readTemplateImportCalls(app)).length + 1;
  await selectTemplateCategory(page);
  await expect(page.getByRole("heading", { name: TEMPLATE_CATEGORY_LABEL })).toBeVisible();
  await expect(page.getByText("智能包装")).toHaveCount(0);
  const importButton = page.getByRole("button", { name: "导入离线模板" });
  await expect(importButton, "template panel must expose the offline import command").toBeVisible({ timeout: 5_000 });
  await importButton.click();
  await expect.poll(async () => (await readTemplateImportCalls(app)).length, { timeout: 30_000 }).toBeGreaterThanOrEqual(nextCount);
  await expect
    .poll(async () => {
      const calls = await readTemplateImportCalls(app);
      const latest = calls[calls.length - 1];
      if (latest?.resultOk === true) {
        return "ok";
      }
      if (latest?.resultOk === false) {
        return `failed:${latest.resultErrorKind ?? "unknown"}:${latest.resultErrorMessage ?? ""}`;
      }
      return "pending";
    }, { timeout: 30_000 })
    .toBe("ok");
  await expect(page.getByLabel("模板适配报告")).toBeVisible();
}

async function selectTemplateCategory(page: Page): Promise<void> {
  const visibleButton = page.getByRole("button", { name: TEMPLATE_CATEGORY_LABEL });
  if ((await visibleButton.count()) > 0 && await visibleButton.first().isVisible()) {
    await visibleButton.first().click();
    return;
  }

  await page.getByRole("button", { name: "更多功能" }).click();
  await page.getByRole("menuitemradio", { name: TEMPLATE_CATEGORY_LABEL }).click();
}

async function expectTemplateReportSummary(page: Page, counts: Record<string, number>): Promise<void> {
  const panel = page.getByLabel("模板适配报告");
  await expect(panel).toBeVisible();
  for (const [label, count] of Object.entries(counts)) {
    await expect(panel.getByText(new RegExp(`${label}\\s+${count}`))).toBeVisible();
  }
}

async function expectTemplateReportRows(page: Page, expectedStatuses: string[]): Promise<void> {
  const panel = page.getByLabel("模板适配报告");
  const rows = panel.locator(".template-report-row");
  await expect(rows).toHaveCount(expectedStatuses.length);
  await expect(panel.getByText(`共 ${expectedStatuses.length} 条适配记录`)).toBeVisible();

  const actualStatuses = await rows.evaluateAll((elements) =>
    elements.map((element) => {
      const statusClass = Array.from(element.classList).find((className) => className.startsWith("status-"));
      return statusClass?.slice("status-".length) ?? "";
    })
  );
  expect(actualStatuses).toEqual(expectedStatuses);
}

async function clickTemplateReportRow(page: Page, status: string, label: RegExp): Promise<void> {
  const panel = page.getByLabel("模板适配报告");
  const row = panel.locator(`.template-report-row.status-${status}`).filter({ hasText: label }).first();
  await expect(row).toBeVisible();
  await row.click();
  await expect(row).toHaveAttribute("aria-current", "true");
}

async function navigateTemplateReportRowsWithKeyboard(page: Page, arrowDownCount: number): Promise<void> {
  const panel = page.getByLabel("模板适配报告");
  const firstRow = panel.locator(".template-report-row").first();
  await expect(firstRow).toBeVisible();
  await firstRow.focus();
  for (let index = 0; index < arrowDownCount; index += 1) {
    await page.keyboard.press("ArrowDown");
  }
  await expect(panel.locator(".template-report-row").nth(arrowDownCount)).toHaveAttribute("aria-current", "true");
}

async function expectTemplateReportFocusState(page: Page, label: RegExp): Promise<void> {
  await expect(page.getByLabel("模板报告定位状态").getByText(label)).toBeVisible({ timeout: 10_000 });
}

async function expectNoSelectedTimelineSegment(page: Page): Promise<void> {
  await expect
    .poll(async () => (await readTimelineSegments(page)).filter((segment) => segment.selected).length, { timeout: 10_000 })
    .toBe(0);
}

async function expectSelectedTimelineSegment(
  page: Page,
  expected: { targetStartUs: number }
): Promise<void> {
  await expect
    .poll(
      async () => {
        const selected = (await readTimelineSegments(page)).find((segment) => segment.selected);
        return selected?.targetStartUs ?? null;
      },
      { timeout: 10_000 }
    )
    .toBe(expected.targetStartUs);
}

async function expectLatestPreviewSeek(app: ProductJourneyAppController, targetTimeMicroseconds: number): Promise<void> {
  await expect
    .poll(
      async () =>
        (await readRealtimePreviewHostCalls(app))
          .filter((call) => call.kind === "seek")
          .at(-1)?.targetTimeMicroseconds ?? null,
      { timeout: 10_000 }
    )
    .toBe(targetTimeMicroseconds);
}

async function expectTemplateStatusRowsUseDistinctBorders(page: Page, statuses: string[]): Promise<void> {
  const panel = page.getByLabel("模板适配报告");
  const borderColors = await Promise.all(
    statuses.map((status) =>
      panel.locator(`.template-report-row.status-${status}`).first().evaluate((element) => getComputedStyle(element).borderTopColor)
    )
  );
  expect(new Set(borderColors).size).toBe(statuses.length);
}

async function readTemplateImportCalls(app: ProductJourneyAppController): Promise<Array<Record<string, unknown>>> {
  const calls = (await readProjectSessionCalls(app)) as unknown as Array<Record<string, unknown>>;
  return calls.filter((call) => call.command === "importKaipaiFormulaBundle");
}

async function exportProductProject(page: Page, app: ProductJourneyAppController, outputPath: string): Promise<void> {
  await unlink(outputPath).catch(() => undefined);
  const nextStartCount = commandCount(await readNativeCommandObservations(app), "startExport") + 1;
  await page.getByLabel("产品操作").getByRole("button", { name: "导出", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "导出" });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("输出路径").fill(outputPath);
  await expect(dialog.getByRole("button", { name: "开始导出" })).toBeEnabled({ timeout: 20_000 });
  await dialog.getByRole("button", { name: "开始导出" }).click();
  await expect
    .poll(async () => commandCount(await readNativeCommandObservations(app), "startExport"), { timeout: 30_000 })
    .toBeGreaterThanOrEqual(nextStartCount);

  const statusButton = dialog.getByRole("button", { name: "查询导出状态" });
  await expect(statusButton).toBeEnabled({ timeout: 20_000 });
  for (let attempt = 0; attempt < 80; attempt += 1) {
    const progressText = (await dialog.getByLabel("导出进度").textContent()) ?? "";
    if (progressText.includes("已完成")) {
      break;
    }
    if (await statusButton.isEnabled()) {
      await statusButton.click();
    }
    await page.waitForTimeout(500);
  }

  const finalProgressText = (await dialog.getByLabel("导出进度").textContent()) ?? "";
  const exportLogText = (await dialog.getByLabel("导出状态", { exact: true }).textContent()) ?? "";
  const validationText = (await dialog.getByLabel("输出校验").textContent()) ?? "";
  expect(
    finalProgressText,
    `template import export must complete: ${JSON.stringify({ finalProgressText, exportLogText, validationText })}`
  ).toContain("已完成");
  expect(existsSync(outputPath), `product export should create ${outputPath}`).toBe(true);
}

function commandCount(calls: Array<{ command: string }>, command: string): number {
  return calls.filter((call) => call.command === command).length;
}

function assertProjectJsonIsCanonical(projectJson: string): void {
  const parsed = JSON.parse(projectJson) as unknown;
  expect(isRecord(parsed) && Array.isArray(parsed.materials), "project JSON should contain canonical materials").toBe(true);
  const serialized = JSON.stringify(parsed);
  for (const forbidden of FORBIDDEN_PROJECT_JSON_TOKENS) {
    expect(serialized, `project.json must not leak ${forbidden}`).not.toContain(forbidden);
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

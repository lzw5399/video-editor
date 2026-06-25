import { expect, test } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

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

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
  ).toContain("ProductionEffectCapabilityRegistry");
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

import { expect, test } from "@playwright/test";
import { access, readFile } from "node:fs/promises";
import { join } from "node:path";

import { generatePhase20LongTimelineFixture } from "./helpers/longTimelineFixture";

test("phase 20 long fixture helper delegates draft materialization to Rust", async () => {
  test.setTimeout(180_000);

  const fixtures = await generatePhase20LongTimelineFixture();

  await expect(access(join(fixtures.bundlePath, "project.json")).then(() => true)).resolves.toBe(true);
  await expect(access(fixtures.exportsDir).then(() => true)).resolves.toBe(true);
  await expect(access(fixtures.evidenceDir).then(() => true)).resolves.toBe(true);
  expect(fixtures.exportPaths[0]).not.toBe(fixtures.exportPaths[1]);
  expect(fixtures.expectedScale).toEqual({
    segmentsPerTrack: 180,
    trackCount: 3,
    totalSegments: 540,
    segmentDurationUs: 1_000_000,
    durationUs: 180_000_000
  });
  expect(fixtures.materializerSummary.totalSegments).toBe(540);
});

test("phase 20 long fixture helper contains no TypeScript-authored segment semantics", async () => {
  const source = await readFile(join(process.cwd(), "tests/helpers/longTimelineFixture.ts"), "utf8");

  expect(source).toContain("cargo run -p testkit --bin phase20_long_fixture");
  expect(source).not.toMatch(/\bprojectJson\b|\bproject\.json\b.*writeFile|segments\s*:\s*\[/);
  expect(source).not.toMatch(/Array\.from\(\s*\{\s*length:\s*540\s*\}/);
});

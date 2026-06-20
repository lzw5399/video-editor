import { _electron as electron, test, type ElectronApplication, type Page } from "@playwright/test";
import { join } from "node:path";

import { generatePhase6MediaFixtures } from "./helpers/mediaFixtures";
import { launchPackagedApp } from "./helpers/packagedApp";
import { assertReopenedProjectState, runRealImportPreviewExportWorkflow } from "./helpers/realWorkflow";

const REAL_RUNTIME_TEST_ENV: NodeJS.ProcessEnv = {
  VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
  VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS: "0",
  VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS: "0",
  VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "0",
  VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "1"
};

test.describe.configure({ timeout: 120_000 });

test("dev no-mock import-preview-export workflow", async () => {
  const fixtures = await generatePhase6MediaFixtures();
  const { app, page } = await launchDevApp(fixtures);

  try {
    await runRealImportPreviewExportWorkflow(app, page, fixtures);
  } finally {
    await app.close();
  }

  const reopened = await launchDevApp(fixtures, {
    VIDEO_EDITOR_TEST_OPEN_PROJECT_BUNDLE: fixtures.bundlePath,
    VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([])
  });
  try {
    await assertReopenedProjectState(reopened.page, fixtures);
  } finally {
    await reopened.app.close();
  }
});

test("packaged no-mock import-preview-export workflow", async () => {
  const fixtures = await generatePhase6MediaFixtures();
  const { app, page } = await launchPackagedApp({
    ...REAL_RUNTIME_TEST_ENV,
    VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE: fixtures.bundlePath,
    VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([fixtures.videoPath, fixtures.imagePath, fixtures.audioPath])
  });

  try {
    await runRealImportPreviewExportWorkflow(app, page, fixtures);
  } finally {
    await app.close();
  }

  const reopened = await launchPackagedApp({
    ...REAL_RUNTIME_TEST_ENV,
    VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([]),
    VIDEO_EDITOR_TEST_OPEN_PROJECT_BUNDLE: fixtures.bundlePath
  });
  try {
    await assertReopenedProjectState(reopened.page, fixtures);
  } finally {
    await reopened.app.close();
  }
});

async function launchDevApp(
  fixtures: Awaited<ReturnType<typeof generatePhase6MediaFixtures>>,
  env: NodeJS.ProcessEnv = {}
): Promise<{ app: ElectronApplication; page: Page }> {
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")],
    env: {
      ...process.env,
      ...REAL_RUNTIME_TEST_ENV,
      VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE: fixtures.bundlePath,
      VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([fixtures.videoPath, fixtures.imagePath, fixtures.audioPath]),
      ...env
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  return { app, page };
}

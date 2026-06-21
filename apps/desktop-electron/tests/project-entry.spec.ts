import { _electron as electron, expect, test, type ElectronApplication, type Page } from "@playwright/test";
import { tmpdir } from "node:os";
import { join } from "node:path";

type ProjectSessionCall = {
  command: "createProjectSession" | "openProjectSession" | "executeProjectIntent" | "closeProjectSession";
  hasDraftField: boolean;
  timelineSemanticKeys?: string[];
};

test.describe.configure({ timeout: 60_000 });

test("default launch starts at project entry before import", async () => {
  const { app, page } = await launchProjectEntryApp();

  try {
    await expectProjectEntry(page);
    await expect(page.getByRole("button", { name: "导入素材" })).toHaveCount(0);
    await expect(page.locator('[aria-label="素材面板"]')).toHaveCount(0);
    await expect(page.locator('[aria-label="预览窗口"]')).toHaveCount(0);
    await expect(page.locator('[aria-label="属性检查器"]')).toHaveCount(0);
    await expect(page.locator('[aria-label="时间线"]')).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("new project creates a Rust-owned project session before showing import controls", async () => {
  const bundlePath = testProjectPath("new");
  const { app, page } = await launchProjectEntryApp({
    VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE: bundlePath
  });

  try {
    await expectProjectEntry(page);
    await page.getByRole("button", { name: "新建项目" }).click();
    await expectWorkspace(page);
    await expect.poll(async () => projectSessionCommandCount(app, "createProjectSession"), { timeout: 20_000 }).toBeGreaterThanOrEqual(1);
    await expect.poll(async () => projectSessionCalls(app), { timeout: 20_000 }).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          command: "createProjectSession",
          hasDraftField: false
        })
      ])
    );
    await expect(page.getByRole("button", { name: "导入素材" })).toBeVisible();
    await expect(page.getByText("草稿包路径")).toHaveCount(0);
  } finally {
    await app.close();
  }
});

test("open project uses a Rust-owned project session and invalid projects show product-safe copy", async () => {
  const bundlePath = testProjectPath("open");
  const created = await launchProjectEntryApp({
    VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE: bundlePath
  });
  try {
    await created.page.getByRole("button", { name: "新建项目" }).click();
    await expectWorkspace(created.page);
    await expect.poll(async () => projectSessionCommandCount(created.app, "createProjectSession"), { timeout: 20_000 }).toBeGreaterThanOrEqual(1);
  } finally {
    await created.app.close();
  }

  const opened = await launchProjectEntryApp({
    VIDEO_EDITOR_TEST_PICK_OPEN_PROJECT_BUNDLE: bundlePath
  });
  try {
    await expectProjectEntry(opened.page);
    await opened.page.getByRole("button", { name: "打开项目" }).click();
    await expectWorkspace(opened.page);
    await expect.poll(async () => projectSessionCommandCount(opened.app, "openProjectSession"), { timeout: 20_000 }).toBeGreaterThanOrEqual(1);
  } finally {
    await opened.app.close();
  }

  const invalidPath = testProjectPath("missing");
  const invalid = await launchProjectEntryApp({
    VIDEO_EDITOR_TEST_PICK_OPEN_PROJECT_BUNDLE: invalidPath
  });
  try {
    await invalid.page.getByRole("button", { name: "打开项目" }).click();
    const alert = invalid.page.getByRole("alert");
    await expect(alert).toContainText("项目打开失败，请确认草稿包完整后重试。");
    await expect(alert).not.toContainText(invalidPath);
    await expect(invalid.page.getByRole("button", { name: "导入素材" })).toHaveCount(0);
  } finally {
    await invalid.app.close();
  }
});

async function launchProjectEntryApp(env: NodeJS.ProcessEnv = {}): Promise<{ app: ElectronApplication; page: Page }> {
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")],
    env: {
      ...process.env,
      VIDEO_EDITOR_TEST_RECORD_COMMANDS: "1",
      VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES: "1",
      VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS: "0",
      VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES: JSON.stringify([]),
      ...env
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  return { app, page };
}

async function expectProjectEntry(page: Page): Promise<void> {
  await expect(page.getByRole("main", { name: "项目入口" })).toBeVisible();
  await expect(page.getByRole("button", { name: "新建项目" })).toBeVisible();
  await expect(page.getByRole("button", { name: "打开项目" })).toBeVisible();
}

async function expectWorkspace(page: Page): Promise<void> {
  await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
  await expect(page.locator('[aria-label="素材面板"]')).toBeVisible();
  await expect(page.locator('[aria-label="预览窗口"]')).toBeVisible();
  await expect(page.locator('[aria-label="属性检查器"]')).toBeVisible();
  await expect(page.locator('[aria-label="时间线"]')).toBeVisible();
}

async function projectSessionCommandCount(
  app: ElectronApplication,
  command: ProjectSessionCall["command"]
): Promise<number> {
  return (await projectSessionCalls(app)).filter((call) => call.command === command).length;
}

async function projectSessionCalls(app: ElectronApplication): Promise<ProjectSessionCall[]> {
  return app.evaluate(() => {
    return (
      (globalThis as typeof globalThis & { __videoEditorTestProjectSessionCalls?: ProjectSessionCall[] })
        .__videoEditorTestProjectSessionCalls ?? []
    );
  });
}

function testProjectPath(label: string): string {
  return join(tmpdir(), `video-editor-project-entry-${label}-${Date.now()}-${Math.random().toString(16).slice(2)}.veproj`);
}

import { _electron as electron, type ElectronApplication, type Page } from "@playwright/test";
import { constants } from "node:fs";
import { access, readdir, stat } from "node:fs/promises";
import { basename, join } from "node:path";

export type PackagedAppLaunch = {
  app: ElectronApplication;
  page: Page;
  executablePath: string;
};

export async function launchPackagedApp(env: NodeJS.ProcessEnv = {}): Promise<PackagedAppLaunch> {
  const executablePath = await findPackagedExecutable();
  const app = await electron.launch({
    executablePath,
    env: {
      ...process.env,
      ...env
    }
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  return { app, page, executablePath };
}

export async function findPackagedExecutable(outDir = join(process.cwd(), "out")): Promise<string> {
  if (process.platform === "darwin") {
    return findMacExecutable(outDir);
  }
  if (process.platform === "win32") {
    return findFirstExecutable(outDir, (path) => path.endsWith(".exe") && !basename(path).includes("Uninstall"));
  }
  return findFirstExecutable(outDir, (path) => !path.endsWith(".so") && !path.endsWith(".pak"));
}

async function findMacExecutable(outDir: string): Promise<string> {
  const appBundles = await findMatchingPaths(outDir, async (path) => path.endsWith(".app") && (await isDirectory(path)), 4);
  for (const appBundle of appBundles) {
    const macOsDir = join(appBundle, "Contents", "MacOS");
    const entries = await readdir(macOsDir).catch(() => []);
    for (const entry of entries) {
      const executablePath = join(macOsDir, entry);
      if (await isExecutableFile(executablePath)) {
        return executablePath;
      }
    }
  }
  throw new Error(`No packaged macOS executable found under ${outDir}`);
}

async function findFirstExecutable(outDir: string, predicate: (path: string) => boolean): Promise<string> {
  const matches = await findMatchingPaths(outDir, async (path) => predicate(path) && (await isExecutableFile(path)), 5);
  const match = matches[0];
  if (match === undefined) {
    throw new Error(`No packaged executable found under ${outDir}`);
  }
  return match;
}

async function findMatchingPaths(
  root: string,
  predicate: (path: string) => Promise<boolean>,
  maxDepth: number
): Promise<string[]> {
  const matches: string[] = [];

  async function visit(path: string, depth: number): Promise<void> {
    if (await predicate(path)) {
      matches.push(path);
      return;
    }
    if (depth >= maxDepth || !(await isDirectory(path))) {
      return;
    }

    const entries = await readdir(path);
    await Promise.all(entries.map((entry) => visit(join(path, entry), depth + 1)));
  }

  await visit(root, 0);
  return matches.sort();
}

async function isDirectory(path: string): Promise<boolean> {
  return stat(path)
    .then((value) => value.isDirectory())
    .catch(() => false);
}

async function isExecutableFile(path: string): Promise<boolean> {
  return access(path, constants.X_OK)
    .then(() => stat(path))
    .then((value) => value.isFile())
    .catch(() => false);
}

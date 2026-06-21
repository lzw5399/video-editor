import { _electron as electron, type ElectronApplication, type Page } from "@playwright/test";
import { constants } from "node:fs";
import { access, chmod, mkdtemp, readdir, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

export type PackagedAppLaunch = {
  app: ElectronApplication;
  page: Page;
  executablePath: string;
};

export async function launchPackagedApp(env: NodeJS.ProcessEnv = {}): Promise<PackagedAppLaunch> {
  const executablePath = await findPackagedExecutable();
  const poisonPath = await createPoisonRuntimePath();
  const app = await electron.launch({
    executablePath,
    env: sanitizedPackagedEnv(poisonPath, env)
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  return { app, page, executablePath };
}

function sanitizedPackagedEnv(poisonPath: string, overrides: NodeJS.ProcessEnv): NodeJS.ProcessEnv {
  return {
    HOME: process.env.HOME,
    TMPDIR: process.env.TMPDIR,
    TMP: process.env.TMP,
    TEMP: process.env.TEMP,
    USER: process.env.USER,
    LOGNAME: process.env.LOGNAME,
    LANG: process.env.LANG,
    LC_ALL: process.env.LC_ALL,
    PATH: poisonPath,
    ...overrides
  };
}

async function createPoisonRuntimePath(): Promise<string> {
  const directory = await mkdtemp(join(tmpdir(), "video-editor-poison-runtime-"));
  await Promise.all(["ffmpeg", "ffprobe"].map((name) => writePoisonBinary(directory, name)));
  return directory;
}

async function writePoisonBinary(directory: string, name: string): Promise<void> {
  const binaryName = process.platform === "win32" ? `${name}.cmd` : name;
  const path = join(directory, binaryName);
  const script =
    process.platform === "win32"
      ? `@echo off\r\necho VIDEO_EDITOR_POISON_PATH_${name} 1>&2\r\nexit /b 86\r\n`
      : `#!/bin/sh\necho VIDEO_EDITOR_POISON_PATH_${name} >&2\nexit 86\n`;
  await writeFile(path, script);
  if (process.platform !== "win32") {
    await chmod(path, 0o755);
  }
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

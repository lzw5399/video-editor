import { chromium, type Browser, type Page } from "@playwright/test";
import { execFile } from "node:child_process";
import { createServer } from "node:net";
import { promisify } from "node:util";

import { findPackagedExecutable } from "./packagedApp";

const execFileAsync = promisify(execFile);

export type ForegroundProductAppDiagnostics = {
  appBundlePath: string;
  remoteDebuggingPort: number;
  pid: number | null;
  processState: string | null;
};

export type ForegroundProductAppController = {
  readonly kind: "foreground-cdp";
  readonly diagnostics: ForegroundProductAppDiagnostics;
  close: () => Promise<void>;
  readExecuteCommandCalls: () => Promise<unknown[]>;
  readRealtimePreviewHostCalls: () => Promise<unknown[]>;
};

declare global {
  interface Window {
    videoEditorTestObservations?: {
      getExecuteCommandCalls: () => Promise<unknown[]>;
      getRealtimePreviewHostCalls: () => Promise<unknown[]>;
    };
  }
}

export async function launchForegroundProductApp(
  openMaterialFiles: string[],
  env: NodeJS.ProcessEnv = {}
): Promise<{ app: ForegroundProductAppController; page: Page }> {
  const executablePath = await findPackagedExecutable();
  const appBundlePath = macAppBundlePathForExecutable(executablePath);
  const remoteDebuggingPort = await allocatePort();
  const args = [
    "-n",
    "-F",
    appBundlePath,
    "--args",
    `--remote-debugging-port=${remoteDebuggingPort}`,
    "--video-editor-test-record-commands=1",
    "--video-editor-test-show-developer-diagnostics=0",
    `--video-editor-test-open-material-files=${encodeURIComponent(JSON.stringify(openMaterialFiles))}`,
    ...testSwitchesFromEnv(env)
  ];

  await execFileAsync("open", args);
  const browser = await connectOverCdp(remoteDebuggingPort);
  const page = await waitForFirstPage(browser);
  await page.waitForLoadState("domcontentloaded");
  const pid = await findProcessIdForRemoteDebuggingPort(remoteDebuggingPort);
  const diagnostics = {
    appBundlePath,
    remoteDebuggingPort,
    pid,
    processState: pid === null ? null : await readMacProcessState(pid)
  };

  const controller: ForegroundProductAppController = {
    kind: "foreground-cdp",
    diagnostics,
    close: async () => {
      await browser.close().catch(() => undefined);
      if (pid !== null) {
        process.kill(pid, "SIGTERM");
      }
    },
    readExecuteCommandCalls: async () => readTestObservation(page, "getExecuteCommandCalls", diagnostics),
    readRealtimePreviewHostCalls: async () => readTestObservation(page, "getRealtimePreviewHostCalls", diagnostics)
  };

  return { app: controller, page };
}

function macAppBundlePathForExecutable(executablePath: string): string {
  const marker = ".app/Contents/MacOS/";
  const markerIndex = executablePath.indexOf(marker);
  if (markerIndex === -1) {
    throw new Error(`Packaged executable is not inside a macOS app bundle: ${executablePath}`);
  }
  return executablePath.slice(0, markerIndex + ".app".length);
}

async function allocatePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (address !== null && typeof address === "object") {
          resolve(address.port);
          return;
        }
        reject(new Error("Failed to allocate a local CDP port"));
      });
    });
    server.on("error", reject);
  });
}

function testSwitchesFromEnv(env: NodeJS.ProcessEnv): string[] {
  const switches: string[] = [];
  if (env.VIDEO_EDITOR_TEST_DISABLE_RENDER_GRAPH_COMPOSITOR === "1") {
    switches.push("--video-editor-test-disable-render-graph-compositor=1");
  }
  return switches;
}

async function connectOverCdp(port: number): Promise<Browser> {
  const endpoint = `http://127.0.0.1:${port}`;
  const deadline = Date.now() + 15_000;
  let lastError: unknown = null;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`${endpoint}/json/version`);
      if (response.ok) {
        return chromium.connectOverCDP(endpoint);
      }
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Timed out waiting for packaged app CDP endpoint on ${endpoint}: ${String(lastError)}`);
}

async function waitForFirstPage(browser: Browser): Promise<Page> {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    for (const context of browser.contexts()) {
      const page = context.pages().find((candidate) => !candidate.isClosed() && candidate.url() !== "about:blank");
      if (page !== undefined) {
        return page;
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error("Timed out waiting for the packaged app renderer page after CDP attach");
}

async function findProcessIdForRemoteDebuggingPort(port: number): Promise<number | null> {
  const result = await execFileAsync("pgrep", ["-f", `--remote-debugging-port=${port}`]).catch(() => null);
  const pid = result?.stdout
    .trim()
    .split(/\s+/)
    .map((value) => Number.parseInt(value, 10))
    .filter((value) => Number.isInteger(value))
    .at(-1);
  return pid ?? null;
}

async function readMacProcessState(pid: number): Promise<string | null> {
  const script = [
    `tell application "System Events"`,
    `set targetProcess to first process whose unix id is ${pid}`,
    `return (name of targetProcess) & "|frontmost=" & (frontmost of targetProcess) & "|visible=" & (visible of targetProcess) & "|windows=" & (count of windows of targetProcess)`,
    `end tell`
  ].join("\n");
  const result = await execFileAsync("osascript", ["-e", script]).catch(() => null);
  return result?.stdout.trim() ?? null;
}

async function readTestObservation(
  page: Page,
  method: keyof NonNullable<Window["videoEditorTestObservations"]>,
  diagnostics: ForegroundProductAppDiagnostics
): Promise<unknown[]> {
  return page.evaluate(
    async ({ methodName, launchDiagnostics }) => {
      const bridge = window.videoEditorTestObservations;
      if (bridge === undefined) {
        throw new Error(`Packaged product CDP test observation bridge is unavailable: ${JSON.stringify(launchDiagnostics)}`);
      }
      return bridge[methodName]();
    },
    { methodName: method, launchDiagnostics: diagnostics }
  );
}

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
  frontmostApplications: string[];
  coreGraphicsWindows: CoreGraphicsWindowSummary[];
  probeErrors: string[];
};

export type ForegroundProductAppController = {
  readonly kind: "foreground-cdp";
  readonly diagnostics: ForegroundProductAppDiagnostics;
  close: () => Promise<void>;
  readExecuteCommandCalls: () => Promise<unknown[]>;
  readProjectSessionCalls: () => Promise<unknown[]>;
  readRealtimePreviewHostCalls: () => Promise<unknown[]>;
  readForegroundDiagnostics: () => Promise<ForegroundProductAppDiagnostics>;
  readWindowMetrics: () => Promise<ProductWindowMetrics>;
};

export type ProductWindowMetrics = {
  bounds: WindowBounds;
  contentBounds: WindowBounds;
  displayScaleFactor: number;
};

export type WindowBounds = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type CoreGraphicsWindowSummary = {
  windowNumber: number;
  layer: number;
  alpha: number;
  onscreen: boolean;
  name: string;
  bounds: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
};

declare global {
  interface Window {
    videoEditorTestObservations?: {
      getExecuteCommandCalls: () => Promise<unknown[]>;
      getProjectSessionCalls: () => Promise<unknown[]>;
      getRealtimePreviewHostCalls: () => Promise<unknown[]>;
      getWindowMetrics: () => Promise<ProductWindowMetrics>;
    };
  }
}

export async function launchForegroundProductApp(
  openMaterialFiles: string[],
  env: NodeJS.ProcessEnv = {}
): Promise<{ app: ForegroundProductAppController; page: Page }> {
  ensureLocalhostBypassesProxy();
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
  if (pid !== null) {
    await activateMacProductApp(pid);
  }
  const diagnostics = await readForegroundDiagnostics({
    appBundlePath,
    remoteDebuggingPort,
    pid
  });

  const controller: ForegroundProductAppController = {
    kind: "foreground-cdp",
    diagnostics,
    close: async () => {
      await browser.close().catch(() => undefined);
      await terminateMainProcess(remoteDebuggingPort, pid);
    },
    readExecuteCommandCalls: async () => readTestObservation(page, "getExecuteCommandCalls", diagnostics),
    readProjectSessionCalls: async () => readTestObservation(page, "getProjectSessionCalls", diagnostics),
    readRealtimePreviewHostCalls: async () => readTestObservation(page, "getRealtimePreviewHostCalls", diagnostics),
    readWindowMetrics: async () => readTestWindowMetrics(page, diagnostics),
    readForegroundDiagnostics: async () =>
      readForegroundDiagnostics({
        appBundlePath,
        remoteDebuggingPort,
        pid: (await findProcessIdForRemoteDebuggingPort(remoteDebuggingPort)) ?? pid
      })
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
  if (env.VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES !== undefined) {
    switches.push(`--video-editor-test-mock-runtime-capabilities=${env.VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES}`);
  }
  if (env.VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE !== undefined) {
    switches.push(`--video-editor-test-new-project-bundle=${encodeURIComponent(env.VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE)}`);
  }
  if (env.VIDEO_EDITOR_TEST_PICK_OPEN_PROJECT_BUNDLE !== undefined) {
    switches.push(
      `--video-editor-test-pick-open-project-bundle=${encodeURIComponent(env.VIDEO_EDITOR_TEST_PICK_OPEN_PROJECT_BUNDLE)}`
    );
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
  const result = await execFileAsync("/bin/ps", ["-axo", "pid=,command="]).catch(() => null);
  const mainProcessLine = result?.stdout
    .split("\n")
    .find((line) =>
      line.includes(`remote-debugging-port=${port}`) &&
      line.includes(".app/Contents/MacOS/Video Editor") &&
      !line.includes("Helper")
    );
  if (mainProcessLine === undefined) {
    return null;
  }
  const pid = Number.parseInt(mainProcessLine.trim().split(/\s+/)[0] ?? "", 10);
  return Number.isInteger(pid) ? pid : null;
}

async function terminateMainProcess(port: number, preferredPid: number | null): Promise<void> {
  const pids = new Set<number>();
  if (preferredPid !== null) {
    pids.add(preferredPid);
  }
  const livePid = await findProcessIdForRemoteDebuggingPort(port);
  if (livePid !== null) {
    pids.add(livePid);
  }

  for (const pid of pids) {
    try {
      process.kill(pid, "SIGTERM");
    } catch {
      // Process already exited.
    }
  }

  await new Promise((resolve) => setTimeout(resolve, 500));
  const remainingPid = await findProcessIdForRemoteDebuggingPort(port);
  if (remainingPid !== null) {
    try {
      process.kill(remainingPid, "SIGKILL");
    } catch {
      // Process already exited.
    }
  }
  await execFileAsync("/usr/bin/pkill", ["-f", `[r]emote-debugging-port=${port}`]).catch(() => undefined);
}

function ensureLocalhostBypassesProxy(): void {
  const bypass = "127.0.0.1,localhost,::1";
  process.env.NO_PROXY = appendNoProxyValue(process.env.NO_PROXY, bypass);
  process.env.no_proxy = appendNoProxyValue(process.env.no_proxy, bypass);
}

function appendNoProxyValue(current: string | undefined, value: string): string {
  if (current === undefined || current.trim().length === 0) {
    return value;
  }
  const entries = new Set(current.split(",").map((entry) => entry.trim()).filter((entry) => entry.length > 0));
  for (const entry of value.split(",")) {
    entries.add(entry);
  }
  return Array.from(entries).join(",");
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

async function activateMacProductApp(pid: number): Promise<void> {
  await execFileAsync("osascript", ["-e", `tell application id "org.videoeditor.desktop" to activate`]).catch(
    () => undefined
  );
  await execFileAsync("osascript", [
    "-e",
    `tell application "System Events" to set frontmost of (first process whose unix id is ${pid}) to true`
  ]).catch(() => undefined);
  await new Promise((resolve) => setTimeout(resolve, 750));
}

async function readForegroundDiagnostics(input: {
  appBundlePath: string;
  remoteDebuggingPort: number;
  pid: number | null;
}): Promise<ForegroundProductAppDiagnostics> {
  const probeErrors: string[] = [];
  let processState: string | null = null;
  let frontmostApplications: string[] = [];
  let coreGraphicsWindows: CoreGraphicsWindowSummary[] = [];

  if (input.pid !== null) {
    processState = await readMacProcessState(input.pid).catch((error) => {
      probeErrors.push(`system-events-process-state: ${errorMessage(error)}`);
      return null;
    });
    frontmostApplications = await readFrontmostApplications().catch((error) => {
      probeErrors.push(`system-events-frontmost: ${errorMessage(error)}`);
      return [];
    });
    coreGraphicsWindows = await readCoreGraphicsWindows(input.pid).catch((error) => {
      probeErrors.push(`core-graphics-windows: ${errorMessage(error)}`);
      return [];
    });
  }

  return {
    appBundlePath: input.appBundlePath,
    remoteDebuggingPort: input.remoteDebuggingPort,
    pid: input.pid,
    processState,
    frontmostApplications,
    coreGraphicsWindows,
    probeErrors
  };
}

async function readFrontmostApplications(): Promise<string[]> {
  const script = `tell application "System Events" to get (name of every process whose frontmost is true)`;
  const result = await execFileAsync("osascript", ["-e", script]);
  return result.stdout
    .trim()
    .split(",")
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0);
}

async function readCoreGraphicsWindows(pid: number): Promise<CoreGraphicsWindowSummary[]> {
  const swiftSource = `
import CoreGraphics
import Foundation

let targetPid = Int(CommandLine.arguments[1])!
let info = CGWindowListCopyWindowInfo(CGWindowListOption(arrayLiteral: .optionAll), kCGNullWindowID) as? [[String: Any]] ?? []
let windows: [[String: Any]] = info.compactMap { window in
  guard let ownerPid = window[kCGWindowOwnerPID as String] as? Int, ownerPid == targetPid else {
    return nil
  }
  let bounds = window[kCGWindowBounds as String] as? [String: Any] ?? [:]
  return [
    "windowNumber": window[kCGWindowNumber as String] as? Int ?? 0,
    "layer": window[kCGWindowLayer as String] as? Int ?? 0,
    "alpha": window[kCGWindowAlpha as String] as? Double ?? 0.0,
    "onscreen": window[kCGWindowIsOnscreen as String] as? Bool ?? false,
    "name": window[kCGWindowName as String] as? String ?? "",
    "bounds": [
      "x": bounds["X"] as? Double ?? 0.0,
      "y": bounds["Y"] as? Double ?? 0.0,
      "width": bounds["Width"] as? Double ?? 0.0,
      "height": bounds["Height"] as? Double ?? 0.0
    ]
  ]
}
let data = try JSONSerialization.data(withJSONObject: windows, options: [])
print(String(data: data, encoding: .utf8)!)
`;
  const result = await execFileAsync("/usr/bin/swift", ["-e", swiftSource, String(pid)], {
    maxBuffer: 1024 * 1024
  });
  const parsed = JSON.parse(result.stdout) as CoreGraphicsWindowSummary[];
  return parsed.filter((window) => window.windowNumber > 0);
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
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

async function readTestWindowMetrics(
  page: Page,
  diagnostics: ForegroundProductAppDiagnostics
): Promise<ProductWindowMetrics> {
  return page.evaluate(async (launchDiagnostics) => {
    const bridge = window.videoEditorTestObservations;
    if (bridge === undefined) {
      throw new Error(`Packaged product CDP test observation bridge is unavailable: ${JSON.stringify(launchDiagnostics)}`);
    }
    return bridge.getWindowMetrics();
  }, diagnostics);
}

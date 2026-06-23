import { app, BrowserWindow, dialog, ipcMain, screen, type IpcMainInvokeEvent, type Rectangle } from "electron";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type {
  AudioOutputDeviceSummary,
  AudioPreviewCommandResponse,
  AudioPreviewPlaybackStatus,
  AudioPreviewStatusResponse,
  ArtifactMaintenanceResult,
  ArtifactStatusSummary,
  ArtifactQuotaStatus,
  CommandResultEnvelope,
  ExportJobStatusResponse,
  RuntimeCapabilityReport,
  WaveformDisplayPeaksResponse,
  WaveformDisplayStatus
} from "../generated/CommandResultEnvelope";
import type { SegmentVisual } from "../generated/Draft";
import {
  cancelAudioPreview,
  cancelArtifactGeneration,
  closeProjectSession,
  configureBundledRuntimeDirectory,
  createAudioPreviewSession,
  createProjectSession,
  cancelExport,
  executeProjectIntent,
  getAudioPreviewStatus,
  getArtifactQuotaStatus,
  getArtifactStatus,
  getExportJobStatus,
  getWaveformDisplayPeaks,
  listProjectSessionMaterials,
  listProjectSessionMissingMaterials,
  listAudioOutputDevices,
  openProjectSession,
  pauseAudioPreview,
  ping,
  playAudioPreview,
  probeMediaRuntime,
  probeRuntimeCapabilities,
  refreshWaveformStatus,
  refreshArtifactStatus,
  resumeArtifactGeneration,
  retryArtifactGeneration,
  runArtifactGarbageCollection,
  seekAudioPreview,
  selectAudioOutputDevice,
  startProjectSessionExport,
  stopAudioPreview,
  version,
  type AudioPreviewRequest,
  type ArtifactGarbageCollectionRequest,
  type ArtifactGenerationActionRequest,
  type ArtifactQuotaRequest,
  type ArtifactStatusRequest,
  type CreateProjectSessionRequest,
  type ExportJobRequest,
  type ExecuteProjectIntentRequest,
  type OpenProjectSessionRequest,
  type ProjectSessionReadRequest,
  type ProjectSessionRequest,
  type SegmentVisualPatch,
  type StartProjectSessionExportRequest,
  type TextSegmentPatch
} from "./nativeBinding";
import { registerRealtimePreviewHost } from "./realtimePreviewHost";

type TestNativeCommandObservation = {
  command: CommandEnvelope["command"];
  kind: CommandEnvelope["payload"]["kind"];
  requestId: string | null;
  targetTime: number | null;
  targetTimerange: { start: number; duration: number } | null;
  duration: number | null;
  canvasConfig: {
    width: number;
    height: number;
    frameRate: { numerator: number; denominator: number };
  } | null;
  visual: SegmentVisual | null;
  keyframeProperty: string | null;
  keyframeAt: number | null;
  textContent: string | null;
  textSource: string | null;
  textFontRef: string | null;
  srtContent: string | null;
  outputPath: string | null;
  preset: string | null;
  jobId: string | null;
  sessionId: string | null;
  projectSessionId: string | null;
  expectedRevision: number | null;
  hasDraftField: boolean;
  deviceSelectionId: string | null;
  maxPeakBins: number | null;
};

type TestProjectSessionCall = {
  command:
    | "createProjectSession"
    | "openProjectSession"
    | "executeProjectIntent"
    | "listProjectSessionMaterials"
    | "listProjectSessionMissingMaterials"
    | "startProjectSessionExport"
    | "closeProjectSession";
  sessionId: string | null;
  expectedRevision: number | null;
  intentKind: string | null;
  itemHandle: string | null;
  materialId: string | null;
  materialPath: string | null;
  outputPath: string | null;
  preset: string | null;
  targetTime: number | null;
  targetTimerange: { start: number; duration: number } | null;
  duration: number | null;
  canvasConfig: TestNativeCommandObservation["canvasConfig"];
  visual: SegmentVisual | null;
  visualPatch: SegmentVisualPatch | null;
  keyframeProperty: string | null;
  keyframeAt: number | null;
  textPatch: TextSegmentPatch | null;
  textContent: string | null;
  textSource: string | null;
  textFontRef: string | null;
  targetTrackHandle: string | null;
  srtContent: string | null;
  timelineSemanticKeys: string[];
  hasDraftField: boolean;
};

type TestWindowMetrics = {
  bounds: Rectangle;
  contentBounds: Rectangle;
  displayScaleFactor: number;
};

function testWindowMetrics(window: BrowserWindow): TestWindowMetrics {
  return {
    bounds: window.getBounds(),
    contentBounds: window.getContentBounds(),
    displayScaleFactor: screen.getDisplayMatching(window.getBounds()).scaleFactor
  };
}

function sanitizeTestWindowDimension(value: unknown, fallback: number): number {
  return Math.max(320, Math.min(4096, Math.round(typeof value === "number" && Number.isFinite(value) ? value : fallback)));
}

type AudioPreviewCommandName =
  | "createAudioPreviewSession"
  | "playAudioPreview"
  | "pauseAudioPreview"
  | "stopAudioPreview"
  | "seekAudioPreview"
  | "cancelAudioPreview"
  | "getAudioPreviewStatus"
  | "listAudioOutputDevices"
  | "selectAudioOutputDevice"
  | "getWaveformDisplayPeaks"
  | "refreshWaveformStatus";
type ArtifactCommandName =
  | "getArtifactStatus"
  | "refreshArtifactStatus"
  | "retryArtifactGeneration"
  | "resumeArtifactGeneration"
  | "cancelArtifactGeneration"
  | "getArtifactQuotaStatus"
  | "runArtifactGarbageCollection";
type ProjectBundlePickerResponse = {
  canceled: boolean;
  bundlePath: string | null;
};

declare global {
  var __videoEditorTestNativeCommandObservations: TestNativeCommandObservation[] | undefined;
  var __videoEditorTestProjectSessionCalls: TestProjectSessionCall[] | undefined;
}

const devServerUrl = process.env.VITE_DEV_SERVER_URL;
const isDevelopment = !app.isPackaged && isLoopbackUrl(devServerUrl);
const packagedRendererFile = join(__dirname, "../renderer/index.html");
const packagedRendererUrl = pathToFileURL(packagedRendererFile).toString();
const allowedRendererUrl = isDevelopment && devServerUrl !== undefined ? devServerUrl : packagedRendererUrl;
const allowedRendererUrlArgument = `--video-editor-allowed-renderer-url=${allowedRendererUrl}`;
hydrateTestEnvironmentFromArguments();
configureBundledRuntimeEnvironment();
const showDeveloperDiagnostics =
  process.env.VIDEO_EDITOR_DEVELOPER_DIAGNOSTICS === "1" ||
  process.env.VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS === "1";
const testObservationEnabled = process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS === "1";
const rendererArguments = [
  allowedRendererUrlArgument,
  ...(showDeveloperDiagnostics ? ["--video-editor-developer-diagnostics=1"] : []),
  ...(testObservationEnabled ? ["--video-editor-test-observations=1"] : []),
  ...testRendererArgument("VIDEO_EDITOR_TEST_WORKSPACE_FIXTURE", "--video-editor-workspace-fixture="),
  ...testRendererArgument("VIDEO_EDITOR_TEST_OPEN_PROJECT_BUNDLE", "--video-editor-test-open-project-bundle=")
];

ipcMain.handle("core:ping", (event) => {
  assertAllowedIpcSender(event);
  return ping();
});
ipcMain.handle("core:version", (event) => {
  assertAllowedIpcSender(event);
  return version();
});
ipcMain.handle("core:probeMediaRuntime", (event) => {
  assertAllowedIpcSender(event);
  return probeMediaRuntime();
});
ipcMain.handle("core:probeRuntimeCapabilities", (event) => {
  assertAllowedIpcSender(event);
  const testRuntimeCapabilitiesResponse = maybeBuildTestRuntimeCapabilitiesResponse();
  if (testRuntimeCapabilitiesResponse !== null) {
    return testRuntimeCapabilitiesResponse;
  }
  return probeRuntimeCapabilities();
});
ipcMain.handle("core:createProjectSession", (event, request: CreateProjectSessionRequest) => {
  assertAllowedIpcSender(event);
  recordTestProjectSessionCall("createProjectSession", request);
  return createProjectSession(request);
});
ipcMain.handle("core:openProjectSession", (event, request: OpenProjectSessionRequest) => {
  assertAllowedIpcSender(event);
  recordTestProjectSessionCall("openProjectSession", request);
  return openProjectSession(request);
});
ipcMain.handle("core:executeProjectIntent", (event, request: ExecuteProjectIntentRequest) => {
  assertAllowedIpcSender(event);
  recordTestProjectSessionCall("executeProjectIntent", request);
  return executeProjectIntent(request);
});
ipcMain.handle("core:listProjectSessionMaterials", (event, request: ProjectSessionReadRequest) => {
  assertAllowedIpcSender(event);
  recordTestProjectSessionCall("listProjectSessionMaterials", request);
  return listProjectSessionMaterials(request);
});
ipcMain.handle("core:listProjectSessionMissingMaterials", (event, request: ProjectSessionReadRequest) => {
  assertAllowedIpcSender(event);
  recordTestProjectSessionCall("listProjectSessionMissingMaterials", request);
  return listProjectSessionMissingMaterials(request);
});
ipcMain.handle("core:startProjectSessionExport", (event, request: StartProjectSessionExportRequest) => {
  assertAllowedIpcSender(event);
  recordTestProjectSessionCall("startProjectSessionExport", request);
  const testExportResponse = maybeBuildTestProjectSessionExportResponse(request);
  if (testExportResponse !== null) {
    return testExportResponse;
  }
  return startProjectSessionExport(request);
});
ipcMain.handle("core:getExportJobStatus", (event, request: ExportJobRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitExportControlCall("getExportJobStatus", request);
  const testExportResponse = maybeBuildTestExplicitExportControlResponse("getExportJobStatus", request);
  if (testExportResponse !== null) {
    return testExportResponse;
  }
  return getExportJobStatus(request);
});
ipcMain.handle("core:cancelExport", (event, request: ExportJobRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitExportControlCall("cancelExport", request);
  const testExportResponse = maybeBuildTestExplicitExportControlResponse("cancelExport", request);
  if (testExportResponse !== null) {
    return testExportResponse;
  }
  return cancelExport(request);
});
ipcMain.handle("core:createAudioPreviewSession", (event, request: AudioPreviewRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitAudioPreviewCall("createAudioPreviewSession", request);
  const testAudioResponse = maybeBuildTestExplicitAudioResponse("createAudioPreviewSession", request);
  if (testAudioResponse !== null) {
    return testAudioResponse;
  }
  return createAudioPreviewSession(request);
});
ipcMain.handle("core:playAudioPreview", (event, request: AudioPreviewRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitAudioPreviewCall("playAudioPreview", request);
  const testAudioResponse = maybeBuildTestExplicitAudioResponse("playAudioPreview", request);
  if (testAudioResponse !== null) {
    return testAudioResponse;
  }
  return playAudioPreview(request);
});
ipcMain.handle("core:pauseAudioPreview", (event, request: AudioPreviewRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitAudioPreviewCall("pauseAudioPreview", request);
  const testAudioResponse = maybeBuildTestExplicitAudioResponse("pauseAudioPreview", request);
  if (testAudioResponse !== null) {
    return testAudioResponse;
  }
  return pauseAudioPreview(request);
});
ipcMain.handle("core:stopAudioPreview", (event, request: AudioPreviewRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitAudioPreviewCall("stopAudioPreview", request);
  const testAudioResponse = maybeBuildTestExplicitAudioResponse("stopAudioPreview", request);
  if (testAudioResponse !== null) {
    return testAudioResponse;
  }
  return stopAudioPreview(request);
});
ipcMain.handle("core:seekAudioPreview", (event, request: AudioPreviewRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitAudioPreviewCall("seekAudioPreview", request);
  const testAudioResponse = maybeBuildTestExplicitAudioResponse("seekAudioPreview", request);
  if (testAudioResponse !== null) {
    return testAudioResponse;
  }
  return seekAudioPreview(request);
});
ipcMain.handle("core:cancelAudioPreview", (event, request: AudioPreviewRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitAudioPreviewCall("cancelAudioPreview", request);
  const testAudioResponse = maybeBuildTestExplicitAudioResponse("cancelAudioPreview", request);
  if (testAudioResponse !== null) {
    return testAudioResponse;
  }
  return cancelAudioPreview(request);
});
ipcMain.handle("core:getAudioPreviewStatus", (event, request: AudioPreviewRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitAudioPreviewCall("getAudioPreviewStatus", request);
  const testAudioResponse = maybeBuildTestExplicitAudioResponse("getAudioPreviewStatus", request);
  if (testAudioResponse !== null) {
    return testAudioResponse;
  }
  return getAudioPreviewStatus(request);
});
ipcMain.handle("core:listAudioOutputDevices", (event, request: AudioPreviewRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitAudioPreviewCall("listAudioOutputDevices", request);
  const testAudioResponse = maybeBuildTestExplicitAudioResponse("listAudioOutputDevices", request);
  if (testAudioResponse !== null) {
    return testAudioResponse;
  }
  return listAudioOutputDevices(request);
});
ipcMain.handle("core:selectAudioOutputDevice", (event, request: AudioPreviewRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitAudioPreviewCall("selectAudioOutputDevice", request);
  const testAudioResponse = maybeBuildTestExplicitAudioResponse("selectAudioOutputDevice", request);
  if (testAudioResponse !== null) {
    return testAudioResponse;
  }
  return selectAudioOutputDevice(request);
});
ipcMain.handle("core:getWaveformDisplayPeaks", (event, request: AudioPreviewRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitAudioPreviewCall("getWaveformDisplayPeaks", request);
  const testAudioResponse = maybeBuildTestExplicitAudioResponse("getWaveformDisplayPeaks", request);
  if (testAudioResponse !== null) {
    return testAudioResponse;
  }
  return getWaveformDisplayPeaks(request);
});
ipcMain.handle("core:refreshWaveformStatus", (event, request: AudioPreviewRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitAudioPreviewCall("refreshWaveformStatus", request);
  const testAudioResponse = maybeBuildTestExplicitAudioResponse("refreshWaveformStatus", request);
  if (testAudioResponse !== null) {
    return testAudioResponse;
  }
  return refreshWaveformStatus(request);
});
ipcMain.handle("core:getArtifactStatus", (event, request: ArtifactStatusRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitArtifactCall("getArtifactStatus", request);
  const testArtifactResponse = maybeBuildTestExplicitArtifactResponse("getArtifactStatus", request);
  if (testArtifactResponse !== null) {
    return testArtifactResponse;
  }
  return getArtifactStatus(request);
});
ipcMain.handle("core:refreshArtifactStatus", (event, request: ArtifactStatusRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitArtifactCall("refreshArtifactStatus", request);
  const testArtifactResponse = maybeBuildTestExplicitArtifactResponse("refreshArtifactStatus", request);
  if (testArtifactResponse !== null) {
    return testArtifactResponse;
  }
  return refreshArtifactStatus(request);
});
ipcMain.handle("core:retryArtifactGeneration", (event, request: ArtifactGenerationActionRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitArtifactCall("retryArtifactGeneration", request);
  const testArtifactResponse = maybeBuildTestExplicitArtifactResponse("retryArtifactGeneration", request);
  if (testArtifactResponse !== null) {
    return testArtifactResponse;
  }
  return retryArtifactGeneration(request);
});
ipcMain.handle("core:resumeArtifactGeneration", (event, request: ArtifactGenerationActionRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitArtifactCall("resumeArtifactGeneration", request);
  const testArtifactResponse = maybeBuildTestExplicitArtifactResponse("resumeArtifactGeneration", request);
  if (testArtifactResponse !== null) {
    return testArtifactResponse;
  }
  return resumeArtifactGeneration(request);
});
ipcMain.handle("core:cancelArtifactGeneration", (event, request: ArtifactGenerationActionRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitArtifactCall("cancelArtifactGeneration", request);
  const testArtifactResponse = maybeBuildTestExplicitArtifactResponse("cancelArtifactGeneration", request);
  if (testArtifactResponse !== null) {
    return testArtifactResponse;
  }
  return cancelArtifactGeneration(request);
});
ipcMain.handle("core:getArtifactQuotaStatus", (event, request: ArtifactQuotaRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitArtifactCall("getArtifactQuotaStatus", request);
  const testArtifactResponse = maybeBuildTestExplicitArtifactResponse("getArtifactQuotaStatus", request);
  if (testArtifactResponse !== null) {
    return testArtifactResponse;
  }
  return getArtifactQuotaStatus(request);
});
ipcMain.handle("core:runArtifactGarbageCollection", (event, request: ArtifactGarbageCollectionRequest) => {
  assertAllowedIpcSender(event);
  recordTestExplicitArtifactCall("runArtifactGarbageCollection", request);
  const testArtifactResponse = maybeBuildTestExplicitArtifactResponse("runArtifactGarbageCollection", request);
  if (testArtifactResponse !== null) {
    return testArtifactResponse;
  }
  return runArtifactGarbageCollection(request);
});
ipcMain.handle("core:closeProjectSession", (event, request: ProjectSessionRequest) => {
  assertAllowedIpcSender(event);
  recordTestProjectSessionCall("closeProjectSession", request);
  return closeProjectSession(request);
});
ipcMain.handle("platform:openMaterialFiles", async (event) => {
  assertAllowedIpcSender(event);

  const testPaths = readTestOpenMaterialFiles();
  if (testPaths !== null) {
    return {
      canceled: testPaths.length === 0,
      filePaths: testPaths
    };
  }

  const result = await dialog.showOpenDialog({
    title: "导入素材",
    properties: ["openFile", "multiSelections"],
    filters: [
      { name: "媒体文件", extensions: ["mp4", "mov", "m4v", "webm", "mp3", "wav", "m4a", "aac", "png", "jpg", "jpeg", "webp"] },
      { name: "视频", extensions: ["mp4", "mov", "m4v", "webm"] },
      { name: "音频", extensions: ["mp3", "wav", "m4a", "aac"] },
      { name: "图片", extensions: ["png", "jpg", "jpeg", "webp"] }
    ]
  });

  return {
    canceled: result.canceled,
    filePaths: result.filePaths
  };
});
ipcMain.handle("platform:createProjectBundle", async (event): Promise<ProjectBundlePickerResponse> => {
  assertAllowedIpcSender(event);

  const testPath = readTestSinglePath("VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE");
  if (testPath !== null) {
    return {
      canceled: testPath.length === 0,
      bundlePath: testPath.length === 0 ? null : testPath
    };
  }

  const result = await dialog.showSaveDialog({
    title: "新建项目",
    defaultPath: "未命名项目.veproj",
    filters: [{ name: "视频剪辑项目", extensions: ["veproj"] }],
    properties: ["createDirectory"]
  });

  return {
    canceled: result.canceled,
    bundlePath: result.filePath ?? null
  };
});
ipcMain.handle("platform:openProjectBundle", async (event): Promise<ProjectBundlePickerResponse> => {
  assertAllowedIpcSender(event);

  const testPath = readTestSinglePath("VIDEO_EDITOR_TEST_PICK_OPEN_PROJECT_BUNDLE");
  if (testPath !== null) {
    return {
      canceled: testPath.length === 0,
      bundlePath: testPath.length === 0 ? null : testPath
    };
  }

  const result = await dialog.showOpenDialog({
    title: "打开项目",
    properties: ["openDirectory"],
    filters: [{ name: "视频剪辑项目", extensions: ["veproj"] }]
  });

  return {
    canceled: result.canceled,
    bundlePath: result.filePaths[0] ?? null
  };
});
if (testObservationEnabled) {
  ipcMain.handle("test:getNativeCommandObservations", (event) => {
    assertAllowedIpcSender(event);
    return globalThis.__videoEditorTestNativeCommandObservations ?? [];
  });
  ipcMain.handle("test:getProjectSessionCalls", (event) => {
    assertAllowedIpcSender(event);
    return globalThis.__videoEditorTestProjectSessionCalls ?? [];
  });
  ipcMain.handle("test:getRealtimePreviewHostCalls", (event) => {
    assertAllowedIpcSender(event);
    return globalThis.__videoEditorTestRealtimePreviewHostCalls ?? [];
  });
  ipcMain.handle("test:getWindowMetrics", (event): TestWindowMetrics => {
    assertAllowedIpcSender(event);
    const window = BrowserWindow.fromWebContents(event.sender);
    if (window === null) {
      throw new Error("No BrowserWindow is associated with the test observation sender");
    }
    return testWindowMetrics(window);
  });
  ipcMain.handle("test:maximizeMainWindow", (event): TestWindowMetrics => {
    assertAllowedIpcSender(event);
    const window = BrowserWindow.fromWebContents(event.sender);
    if (window === null) {
      throw new Error("No BrowserWindow is associated with the test observation sender");
    }
    window.maximize();
    return testWindowMetrics(window);
  });
  ipcMain.handle("test:moveMainWindow", (event, x: unknown, y: unknown): TestWindowMetrics => {
    assertAllowedIpcSender(event);
    const window = BrowserWindow.fromWebContents(event.sender);
    if (window === null) {
      throw new Error("No BrowserWindow is associated with the test observation sender");
    }
    window.setPosition(sanitizeTestWindowDimension(x, window.getBounds().x), sanitizeTestWindowDimension(y, window.getBounds().y));
    return testWindowMetrics(window);
  });
  ipcMain.handle("test:resizeMainWindow", (event, width: unknown, height: unknown): TestWindowMetrics => {
    assertAllowedIpcSender(event);
    const window = BrowserWindow.fromWebContents(event.sender);
    if (window === null) {
      throw new Error("No BrowserWindow is associated with the test observation sender");
    }
    window.unmaximize();
    window.setSize(sanitizeTestWindowDimension(width, 1120), sanitizeTestWindowDimension(height, 720));
    return testWindowMetrics(window);
  });
}

async function createWindow(): Promise<void> {
  const macosWindowChrome =
    process.platform === "darwin"
      ? ({
          titleBarStyle: "hiddenInset",
          trafficLightPosition: { x: 16, y: 10 }
        } as const)
      : {};
  const window = new BrowserWindow({
    width: 1280,
    height: 800,
    minWidth: 960,
    minHeight: 640,
    backgroundColor: "#171717",
    ...macosWindowChrome,
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      preload: join(__dirname, "../preload/index.cjs"),
      additionalArguments: rendererArguments
    }
  });
  registerRealtimePreviewHost(window, assertAllowedIpcSender);

  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  window.webContents.on("will-navigate", (event, targetUrl) => {
    if (!isAllowedRendererUrl(targetUrl)) {
      event.preventDefault();
    }
  });

  if (isDevelopment) {
    await window.loadURL(devServerUrl as string);
    return;
  }

  await window.loadFile(packagedRendererFile);
}

app.whenReady().then(async () => {
  prepareMacosForegroundApp();
  await createWindow();
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    void createWindow();
  }
});

function isLoopbackUrl(value: string | undefined): value is string {
  if (value === undefined) {
    return false;
  }

  try {
    const url = new URL(value);
    return (
      (url.protocol === "http:" || url.protocol === "https:") &&
      (url.hostname === "localhost" || url.hostname === "127.0.0.1" || url.hostname === "::1")
    );
  } catch {
    return false;
  }
}

function prepareMacosForegroundApp(): void {
  if (process.platform !== "darwin") {
    return;
  }

  app.setActivationPolicy("regular");
}

function assertAllowedIpcSender(event: IpcMainInvokeEvent): void {
  const senderUrl = event.senderFrame.url;
  if (!isAllowedRendererUrl(senderUrl)) {
    throw new Error(`Rejected IPC from untrusted renderer: ${senderUrl}`);
  }
}

function isAllowedRendererUrl(targetUrl: string): boolean {
  try {
    const target = new URL(targetUrl);
    const allowed = new URL(allowedRendererUrl);

    if (isDevelopment && devServerUrl !== undefined) {
      return target.origin === allowed.origin;
    }

    return target.protocol === "file:" && target.host === allowed.host && target.pathname === allowed.pathname;
  } catch {
    return false;
  }
}

function readTestOpenMaterialFiles(): string[] | null {
  const raw = decodeTestArgumentValue(process.env.VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES);
  if (raw === undefined) {
    return null;
  }

  try {
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed) && parsed.every((value) => typeof value === "string")) {
      return parsed;
    }
  } catch {
    return raw.length === 0 ? [] : raw.split(":").filter((value) => value.length > 0);
  }

  return [];
}

function readTestSinglePath(envName: string): string | null {
  const raw = decodeTestArgumentValue(process.env[envName]);
  if (raw === undefined) {
    return null;
  }
  return raw;
}

function hydrateTestEnvironmentFromArguments(): void {
  setEnvFromArgument("VIDEO_EDITOR_TEST_RECORD_COMMANDS", "--video-editor-test-record-commands=");
  setEnvFromArgument("VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS", "--video-editor-test-show-developer-diagnostics=");
  setEnvFromArgument("VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES", "--video-editor-test-open-material-files=");
  setEnvFromArgument("VIDEO_EDITOR_TEST_OPEN_PROJECT_BUNDLE", "--video-editor-test-open-project-bundle=");
  setEnvFromArgument("VIDEO_EDITOR_TEST_NEW_PROJECT_BUNDLE", "--video-editor-test-new-project-bundle=");
  setEnvFromArgument("VIDEO_EDITOR_TEST_PICK_OPEN_PROJECT_BUNDLE", "--video-editor-test-pick-open-project-bundle=");
  setEnvFromArgument("VIDEO_EDITOR_TEST_DISABLE_RENDER_GRAPH_COMPOSITOR", "--video-editor-test-disable-render-graph-compositor=");
  setEnvFromArgument("VIDEO_EDITOR_TEST_WORKSPACE_FIXTURE", "--video-editor-test-workspace-fixture=");
  setEnvFromArgument("VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES", "--video-editor-test-mock-runtime-capabilities=");
}

function configureBundledRuntimeEnvironment(): void {
  const root = app.isPackaged ? process.resourcesPath : join(__dirname, "../../runtime");
  const bundledRuntimeDir = join(root, "ffmpeg", platformArchSegment());
  configureBundledRuntimeDirectory(bundledRuntimeDir);
}

function platformArchSegment(): string {
  return `${process.platform}-${process.arch}`;
}

function testRendererArgument(envName: string, prefix: string): string[] {
  const value = process.env[envName];
  return value === undefined || value.trim().length === 0 ? [] : [`${prefix}${encodeURIComponent(value)}`];
}

function setEnvFromArgument(envName: string, prefix: string): void {
  const argument = process.argv.find((value) => value.startsWith(prefix));
  if (argument === undefined) {
    return;
  }
  process.env[envName] = decodeTestArgumentValue(argument.slice(prefix.length));
}

function decodeTestArgumentValue(value: string | undefined): string | undefined {
  if (value === undefined) {
    return undefined;
  }
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

function recordTestNativeCommandObservation(command: CommandEnvelope): void {
  if (!testObservationEnabled) {
    return;
  }

  const targetTime = null;
  const targetTimerange = null;
  const duration = null;
  const outputPath = command.payload.kind === "startExport" ? command.payload.outputPath : null;
  const preset = command.payload.kind === "startExport" ? command.payload.preset : null;
  const sessionId = isAudioPreviewCommandKind(command.payload.kind) ? command.payload.sessionId ?? null : null;
  const projectSessionId = isAudioPreviewCommandKind(command.payload.kind) ? command.payload.projectSessionId ?? null : null;
  const expectedRevision = isAudioPreviewCommandKind(command.payload.kind) ? command.payload.expectedRevision ?? null : null;
  const hasDraftField = isAudioPreviewCommandKind(command.payload.kind)
    ? Object.prototype.hasOwnProperty.call(command.payload, "draft")
    : false;
  const deviceSelectionId = isAudioPreviewCommandKind(command.payload.kind) ? command.payload.deviceSelectionId ?? null : null;
  const maxPeakBins = isAudioPreviewCommandKind(command.payload.kind) ? command.payload.maxPeakBins ?? null : null;
  const jobId =
    command.payload.kind === "getExportJobStatus" ||
    command.payload.kind === "cancelExport" ||
    command.payload.kind === "retryArtifactGeneration" ||
    command.payload.kind === "resumeArtifactGeneration" ||
    command.payload.kind === "cancelArtifactGeneration"
      ? command.payload.jobId
      : null;

  globalThis.__videoEditorTestNativeCommandObservations ??= [];
  globalThis.__videoEditorTestNativeCommandObservations.push({
    command: command.command,
    kind: command.payload.kind,
    requestId: command.requestId ?? null,
    targetTime,
    targetTimerange,
    duration,
    canvasConfig: null,
    visual: null,
    keyframeProperty: null,
    keyframeAt: null,
    textContent: null,
    textSource: null,
    textFontRef: null,
    srtContent: null,
    outputPath,
    preset,
    jobId,
    sessionId,
    projectSessionId,
    expectedRevision,
    hasDraftField,
    deviceSelectionId,
    maxPeakBins
  });
}

function recordTestExplicitExportControlCall(command: "getExportJobStatus" | "cancelExport", request: ExportJobRequest): void {
  recordTestNativeCommandObservation({
    command,
    payload: {
      kind: command,
      jobId: request.jobId
    },
    requestId: `explicit-${command}`
  });
}

function recordTestExplicitAudioPreviewCall(command: AudioPreviewCommandName, request: AudioPreviewRequest): void {
  recordTestNativeCommandObservation(buildExplicitAudioPreviewEnvelope(command, request));
}

function recordTestExplicitArtifactCall(
  command: ArtifactCommandName,
  request: ArtifactStatusRequest | ArtifactGenerationActionRequest | ArtifactQuotaRequest | ArtifactGarbageCollectionRequest
): void {
  recordTestNativeCommandObservation(buildExplicitArtifactEnvelope(command, request));
}

function recordTestProjectSessionCall(
  command: TestProjectSessionCall["command"],
  request:
    | CreateProjectSessionRequest
    | OpenProjectSessionRequest
    | ExecuteProjectIntentRequest
    | ProjectSessionRequest
    | ProjectSessionReadRequest
    | StartProjectSessionExportRequest
): void {
  if (!testObservationEnabled) {
    return;
  }

  const intent = "intent" in request ? request.intent : null;
  const intentRecord = intent as Record<string, unknown> | null;
  const textPatch =
    intentRecord?.kind === "editSelectedText" && typeof intentRecord.patch === "object" && intentRecord.patch !== null
      ? (intentRecord.patch as TextSegmentPatch)
      : null;
  const intentTargetTime =
    typeof intentRecord?.targetStart === "number"
      ? intentRecord.targetStart
      : typeof intentRecord?.timeOffset === "number"
        ? intentRecord.timeOffset
        : null;
  globalThis.__videoEditorTestProjectSessionCalls ??= [];
  globalThis.__videoEditorTestProjectSessionCalls.push({
    command,
    sessionId: "sessionId" in request ? request.sessionId : null,
    expectedRevision: "expectedRevision" in request ? request.expectedRevision : null,
    intentKind: intent?.kind ?? null,
    itemHandle: typeof intentRecord?.itemHandle === "string" ? intentRecord.itemHandle : null,
    materialId: intent !== null && "materialId" in intent ? intent.materialId ?? null : null,
    materialPath: intent !== null && "materialPath" in intent ? intent.materialPath ?? null : null,
    outputPath: "outputPath" in request ? request.outputPath : null,
    preset: "preset" in request ? request.preset : null,
    targetTime: "targetTime" in request ? request.targetTime : intentTargetTime,
    targetTimerange: "targetTimerange" in request ? request.targetTimerange : null,
    duration: typeof intentRecord?.duration === "number" ? intentRecord.duration : null,
    canvasConfig:
      intentRecord?.kind === "updateDraftCanvasConfig"
        ? (intentRecord.canvasConfig as TestNativeCommandObservation["canvasConfig"])
        : null,
    visual: intentRecord?.kind === "updateSelectedSegmentVisual" && "visual" in intentRecord ? (intentRecord.visual as SegmentVisual) : null,
    visualPatch:
      intentRecord?.kind === "updateSelectedSegmentVisual" && typeof intentRecord.patch === "object" && intentRecord.patch !== null
        ? (intentRecord.patch as SegmentVisualPatch)
        : null,
    keyframeProperty: typeof intentRecord?.property === "string" ? intentRecord.property : null,
    keyframeAt: typeof intentRecord?.at === "number" ? intentRecord.at : null,
    textPatch,
    textContent: typeof textPatch?.content === "string" ? textPatch.content : null,
    textSource: null,
    textFontRef: typeof textPatch?.fontRef === "string" ? textPatch.fontRef : null,
    targetTrackHandle: typeof intentRecord?.targetTrackHandle === "string" ? intentRecord.targetTrackHandle : null,
    srtContent: typeof intentRecord?.srtContent === "string" ? intentRecord.srtContent : null,
    timelineSemanticKeys: timelineSemanticKeys(intentRecord),
    hasDraftField:
      Object.prototype.hasOwnProperty.call(request, "draft") ||
      (intent !== null && Object.prototype.hasOwnProperty.call(intent, "draft"))
  });
}

function timelineSemanticKeys(intent: Record<string, unknown> | null): string[] {
  if (intent === null) {
    return [];
  }

  return ["segmentId", "rightSegmentId", "trackId", "targetTrackId", "sourceTimerange", "targetTimerange", "mainTrackMagnet"].filter(
    (key) => Object.prototype.hasOwnProperty.call(intent, key)
  );
}

function isAudioPreviewCommandKind(kind: CommandEnvelope["payload"]["kind"]): kind is
  | "createAudioPreviewSession"
  | "playAudioPreview"
  | "pauseAudioPreview"
  | "stopAudioPreview"
  | "seekAudioPreview"
  | "cancelAudioPreview"
  | "getAudioPreviewStatus"
  | "listAudioOutputDevices"
  | "selectAudioOutputDevice"
  | "getWaveformDisplayPeaks"
  | "refreshWaveformStatus" {
  return (
    kind === "createAudioPreviewSession" ||
    kind === "playAudioPreview" ||
    kind === "pauseAudioPreview" ||
    kind === "stopAudioPreview" ||
    kind === "seekAudioPreview" ||
    kind === "cancelAudioPreview" ||
    kind === "getAudioPreviewStatus" ||
    kind === "listAudioOutputDevices" ||
    kind === "selectAudioOutputDevice" ||
    kind === "getWaveformDisplayPeaks" ||
    kind === "refreshWaveformStatus"
  );
}

function maybeBuildTestRuntimeCapabilitiesResponse(): CommandResultEnvelope<RuntimeCapabilityReport> | null {
  if (process.env.VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES === "0") {
    return null;
  }

  if (process.env.VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES === "error") {
    return {
      ok: false,
      data: null,
      error: {
        kind: "runtimeDiscoveryFailed",
        message: "运行环境检测失败，请检查内置 FFmpeg/ffprobe runtime 后重试。",
        command: "probeRuntimeCapabilities"
      },
      events: []
    };
  }

  if (
    process.env.VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES !== "1" &&
    process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS !== "1"
  ) {
    return null;
  }

  return {
    ok: true,
    data: {
      status: "ready",
      executorName: "desktop-test-runtime",
      ffmpeg: {
        kind: "ffmpeg",
        path: "/tmp/video-editor-test-runtime/ffmpeg",
        source: "bundled",
        version: "ffmpeg version test",
        configureSummary: "configuration: test-runtime",
        status: "ready",
        diagnostic: null
      },
      ffprobe: {
        kind: "ffprobe",
        path: "/tmp/video-editor-test-runtime/ffprobe",
        source: "bundled",
        version: "ffprobe version test",
        configureSummary: "configuration: test-runtime",
        status: "ready",
        diagnostic: null
      },
      h264Encoder: {
        name: "H.264",
        available: true,
        status: "ready",
        diagnostic: null
      },
      aacEncoder: {
        name: "AAC",
        available: true,
        status: "ready",
        diagnostic: null
      },
      assFilter: {
        name: "ASS",
        available: true,
        status: "ready",
        diagnostic: null
      },
      subtitlesFilter: {
        name: "subtitles",
        available: true,
        status: "ready",
        diagnostic: null
      },
      fontReadiness: {
        envTextFontPath: null,
        availableFontPaths: [
          "assets/fonts/noto-sans-cjk-sc/NotoSansCJKsc-Regular.otf",
          "/System/Library/Fonts/PingFang.ttc"
        ],
        bundledFontRef: "font://bundled/noto-sans-cjk-sc-regular",
        bundledFontFamily: "Noto Sans CJK SC",
        bundledFontPath: "assets/fonts/noto-sans-cjk-sc/NotoSansCJKsc-Regular.otf",
        bundledFontLicense: "OFL-1.1",
        status: "ready",
        diagnostic: null
      },
      licensePosture: {
        externalRuntime: false,
        redistributableBuild: false,
        source: "bundledRuntime",
        message: "当前使用打包内置 FFmpeg/ffprobe；公开再发行仍需完成法律审查。"
      },
      diagnostics: []
    },
    error: null,
    events: []
  };
}

function maybeBuildTestExportResponse(command: CommandEnvelope): CommandResultEnvelope<ExportJobStatusResponse> | null {
  if (process.env.VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS !== "1") {
    return null;
  }

  if (command.payload.kind === "startExport") {
    return {
      ok: true,
      data: {
        jobId: "test-export-job",
        phase: "running",
        outputPath: command.payload.outputPath,
        preset: command.payload.preset,
        progressPerMille: 120,
        outTime: 960_000,
        logSummary: "导出任务已启动",
        validation: null,
        diagnostic: null
      },
      error: null,
      events: []
    };
  }

  if (command.payload.kind === "getExportJobStatus") {
    return {
      ok: true,
      data: {
        jobId: command.payload.jobId,
        phase: "completed",
        outputPath: "/tmp/video-editor-export.mp4",
        preset: "h264AacBalanced",
        progressPerMille: 1000,
        outTime: 8_000_000,
        logSummary: "导出完成，输出校验通过",
        validation: {
          path: "/tmp/video-editor-export.mp4",
          fileSizeBytes: 123456,
          duration: 8_000_000,
          frameRate: { numerator: 30, denominator: 1 },
          width: 1920,
          height: 1080,
          hasAudio: true
        },
        diagnostic: null
      },
      error: null,
      events: []
    };
  }

  if (command.payload.kind === "cancelExport") {
    return {
      ok: true,
      data: {
        jobId: command.payload.jobId,
        phase: "cancelled",
        outputPath: "/tmp/video-editor-export.mp4",
        preset: "h264AacBalanced",
        progressPerMille: 120,
        outTime: 960_000,
        logSummary: "导出任务已取消",
        validation: null,
        diagnostic: {
          kind: "cancelled",
          message: "导出任务已取消",
          stdoutSummary: null,
          stderrSummary: null
        }
      },
      error: null,
      events: []
    };
  }

  return null;
}

function maybeBuildTestExplicitExportControlResponse(
  command: "getExportJobStatus" | "cancelExport",
  request: ExportJobRequest
): CommandResultEnvelope<ExportJobStatusResponse> | null {
  return maybeBuildTestExportResponse({
    command,
    payload: {
      kind: command,
      jobId: request.jobId
    },
    requestId: `explicit-${command}`
  });
}

function maybeBuildTestExplicitAudioResponse(
  command: AudioPreviewCommandName,
  request: AudioPreviewRequest
):
  | CommandResultEnvelope<AudioPreviewCommandResponse>
  | CommandResultEnvelope<AudioPreviewStatusResponse>
  | CommandResultEnvelope<AudioOutputDeviceSummary[]>
  | CommandResultEnvelope<WaveformDisplayPeaksResponse>
  | null {
  return maybeBuildTestAudioResponse(buildExplicitAudioPreviewEnvelope(command, request));
}

function maybeBuildTestExplicitArtifactResponse(
  command: ArtifactCommandName,
  request: ArtifactStatusRequest | ArtifactGenerationActionRequest | ArtifactQuotaRequest | ArtifactGarbageCollectionRequest
):
  | CommandResultEnvelope<ArtifactStatusSummary>
  | CommandResultEnvelope<ArtifactQuotaStatus>
  | CommandResultEnvelope<ArtifactMaintenanceResult>
  | null {
  return maybeBuildTestArtifactResponse(buildExplicitArtifactEnvelope(command, request));
}

function buildExplicitAudioPreviewEnvelope(command: AudioPreviewCommandName, request: AudioPreviewRequest): CommandEnvelope {
  return {
    command,
    payload: {
      ...request,
      kind: command
    } as CommandEnvelope["payload"],
    requestId: `explicit-${command}`
  };
}

function buildExplicitArtifactEnvelope(
  command: ArtifactCommandName,
  request: ArtifactStatusRequest | ArtifactGenerationActionRequest | ArtifactQuotaRequest | ArtifactGarbageCollectionRequest
): CommandEnvelope {
  return {
    command,
    payload: {
      ...request,
      kind: command
    } as CommandEnvelope["payload"],
    requestId: `explicit-${command}`
  };
}

function maybeBuildTestProjectSessionExportResponse(
  request: StartProjectSessionExportRequest
): CommandResultEnvelope<ExportJobStatusResponse> | null {
  if (process.env.VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS !== "1") {
    return null;
  }

  return {
    ok: true,
    data: {
      jobId: "test-export-job",
      phase: "running",
      outputPath: request.outputPath,
      preset: request.preset,
      progressPerMille: 120,
      outTime: 960_000,
      logSummary: "导出任务已启动",
      validation: null,
      diagnostic: null
    },
    error: null,
    events: []
  };
}

function maybeBuildTestAudioResponse(
  command: CommandEnvelope
):
  | CommandResultEnvelope<AudioPreviewCommandResponse>
  | CommandResultEnvelope<AudioPreviewStatusResponse>
  | CommandResultEnvelope<AudioOutputDeviceSummary[]>
  | CommandResultEnvelope<WaveformDisplayPeaksResponse>
  | null {
  if (process.env.VIDEO_EDITOR_TEST_MOCK_AUDIO_COMMANDS !== "1" || !isAudioPreviewCommandKind(command.payload.kind)) {
    return null;
  }

  const rejectedCommand = process.env.VIDEO_EDITOR_TEST_AUDIO_REJECT_COMMAND;
  if (rejectedCommand === command.payload.kind) {
    return {
      ok: false,
      data: null,
      error: {
        kind: "previewServiceFailed",
        message: "音频命令被测试场景拒绝",
        command: command.payload.kind
      },
      events: []
    };
  }

  if (command.payload.kind === "listAudioOutputDevices") {
    return {
      ok: true,
      data: buildTestAudioDevices(),
      error: null,
      events: []
    };
  }

  if (command.payload.kind === "getAudioPreviewStatus") {
    return {
      ok: true,
      data: buildTestAudioStatusResponse(command.payload.sessionId ?? "desktop-audio-preview-session", "ready", command.payload.targetTime ?? 0),
      error: null,
      events: []
    };
  }

  if (command.payload.kind === "getWaveformDisplayPeaks" || command.payload.kind === "refreshWaveformStatus") {
    return {
      ok: true,
      data: buildTestWaveformResponse(command.payload.materialId ?? null, command.payload.maxPeakBins ?? 16),
      error: null,
      events: []
    };
  }

  const status = audioStatusForCommand(command.payload.kind);
  return {
    ok: true,
    data: {
      sessionId: command.payload.sessionId ?? "desktop-audio-preview-session",
      generation: command.payload.playbackGeneration ?? 1,
      accepted: true,
      status,
      statusLabel: audioStatusLabel(status),
      targetTime: command.payload.targetTime ?? 0,
      diagnostics: []
    },
    error: null,
    events: []
  };
}

function audioStatusForCommand(kind: CommandEnvelope["payload"]["kind"]): AudioPreviewPlaybackStatus {
  if (kind === "playAudioPreview") {
    return "playing";
  }
  if (kind === "pauseAudioPreview") {
    return "paused";
  }
  if (kind === "stopAudioPreview") {
    return "stopped";
  }
  if (kind === "seekAudioPreview") {
    return "seeking";
  }
  if (kind === "cancelAudioPreview") {
    return "canceled";
  }
  return "ready";
}

function audioStatusLabel(status: AudioPreviewPlaybackStatus): string {
  const labels: Record<AudioPreviewPlaybackStatus, string> = {
    ready: "音频就绪",
    playing: "正在播放",
    paused: "已暂停",
    stopped: "已暂停",
    buffering: "音频缓冲中",
    seeking: "正在定位声音",
    canceled: "音频请求已取消",
    staleRejected: "声音已同步到最新播放头",
    unavailable: "音频暂不可用",
    failed: "音频预览失败：请检查素材是否可用，或重新连接输出设备后重试。"
  };

  return labels[status];
}

function buildTestAudioStatusResponse(
  sessionId: string,
  status: AudioPreviewPlaybackStatus,
  targetTime: number
): AudioPreviewStatusResponse {
  return {
    sessionId,
    generation: 1,
    status,
    statusLabel: audioStatusLabel(status),
    targetTime,
    bufferedUntil: targetTime + 2_000_000,
    device: buildTestAudioDevices()[0],
    diagnostics: []
  };
}

function buildTestAudioDevices(): AudioOutputDeviceSummary[] {
  return [
    {
      selectionId: "system-default",
      displayName: "系统默认",
      status: "ready",
      statusLabel: "输出设备就绪",
      isDefault: true,
      sampleRateHz: 48_000,
      channelCount: 2,
      diagnostics: []
    },
    {
      selectionId: "desktop-output-secondary",
      displayName: "外接监听",
      status: "ready",
      statusLabel: "输出设备就绪",
      isDefault: false,
      sampleRateHz: 48_000,
      channelCount: 2,
      diagnostics: []
    }
  ];
}

function buildTestWaveformResponse(materialId: string | null, maxPeakBins: number): WaveformDisplayPeaksResponse {
  const status = waveformStatusFromEnv();
  const safeBins = Math.max(1, Math.min(64, Math.round(maxPeakBins)));
  const peaks =
    status === "ready"
      ? Array.from({ length: safeBins }, (_, index) => {
          const height = 180 + ((index * 137) % 720);
          return {
            minMillis: -height,
            maxMillis: height
          };
        })
      : [];

  return {
    materialId,
    status,
    statusLabel: waveformStatusLabel(status),
    targetTimerange: { start: 0, duration: 8_000_000 },
    requestedPeakBins: safeBins,
    returnedPeakBins: peaks.length,
    peaks,
    diagnostics: []
  };
}

function waveformStatusFromEnv(): WaveformDisplayStatus {
  const status = process.env.VIDEO_EDITOR_TEST_AUDIO_WAVEFORM_STATUS;
  if (status === "pending" || status === "missing" || status === "failed") {
    return status;
  }
  return "ready";
}

function waveformStatusLabel(status: WaveformDisplayStatus): string {
  const labels: Record<WaveformDisplayStatus, string> = {
    ready: "波形就绪",
    pending: "波形生成中",
    missing: "暂无波形",
    failed: "波形生成失败"
  };

  return labels[status];
}

function maybeBuildTestArtifactResponse(
  command: CommandEnvelope
):
  | CommandResultEnvelope<ArtifactStatusSummary>
  | CommandResultEnvelope<ArtifactQuotaStatus>
  | CommandResultEnvelope<ArtifactMaintenanceResult>
  | null {
  if (process.env.VIDEO_EDITOR_TEST_MOCK_ARTIFACT_COMMANDS !== "1") {
    return null;
  }

  if (command.payload.kind === "getArtifactStatus" || command.payload.kind === "refreshArtifactStatus") {
    return {
      ok: true,
      data: buildTestArtifactStatusSummary(command.payload.sessionId, "生成中"),
      error: null,
      events: []
    };
  }

  if (command.payload.kind === "cancelArtifactGeneration") {
    return {
      ok: true,
      data: buildTestArtifactStatusSummary(command.payload.sessionId, "资源任务已更新", command.payload.jobId),
      error: null,
      events: []
    };
  }

  if (command.payload.kind === "retryArtifactGeneration" || command.payload.kind === "resumeArtifactGeneration") {
    return {
      ok: true,
      data: buildTestArtifactStatusSummary(command.payload.sessionId, "资源任务已恢复"),
      error: null,
      events: []
    };
  }

  if (command.payload.kind === "getArtifactQuotaStatus") {
    return {
      ok: true,
      data: buildTestArtifactQuotaStatus(),
      error: null,
      events: []
    };
  }

  if (command.payload.kind === "runArtifactGarbageCollection") {
    return {
      ok: true,
      data: {
        sessionId: command.payload.sessionId,
        statusLabel: command.payload.dryRun ? "缓存空间偏高" : "缓存清理完成",
        mode: command.payload.dryRun ? "dryRun" : "apply",
        affectedCount: command.payload.dryRun ? 3 : 2,
        reclaimableLabel: "860 MB",
        releasedLabel: command.payload.dryRun ? "0 MB" : "640 MB",
        completed: !command.payload.dryRun
      },
      error: null,
      events: []
    };
  }

  return null;
}

function buildTestArtifactStatusSummary(
  sessionId: string,
  statusLabel: string,
  cancelledJobId: string | null = null
): ArtifactStatusSummary {
  const includeOverflowTask = process.env.VIDEO_EDITOR_TEST_ARTIFACT_TASK_COUNT === "4";
  const tasks: ArtifactStatusSummary["tasks"] = [
    {
      jobId: "artifact-job-waveform",
      artifactKind: "waveform",
      displayLabel: "城市街景.mp4",
      status: cancelledJobId === null || cancelledJobId === "artifact-job-waveform" ? "cancelRequested" : "running",
      statusLabel: cancelledJobId === null || cancelledJobId === "artifact-job-waveform" ? "正在取消" : "生成中",
      progressPerMille: 420,
      canRetry: false,
      canResume: false,
      canCancel: true,
      errorCategory: null
    },
    {
      jobId: "artifact-job-thumbnail",
      artifactKind: "thumbnail",
      displayLabel: "封面图.png",
      status: "failed",
      statusLabel: "生成失败",
      progressPerMille: null,
      canRetry: true,
      canResume: false,
      canCancel: false,
      errorCategory: "missingSource"
    },
    {
      jobId: "artifact-job-proxy",
      artifactKind: "proxy",
      displayLabel: "背景音乐.wav",
      status: "resumable",
      statusLabel: "可继续",
      progressPerMille: 510,
      canRetry: false,
      canResume: true,
      canCancel: false,
      errorCategory: null
    }
  ];

  if (includeOverflowTask) {
    tasks.push({
      jobId: "artifact-job-preview",
      artifactKind: "preview",
      displayLabel: "预览资源",
      status: "waiting",
      statusLabel: "等待生成",
      progressPerMille: null,
      canRetry: false,
      canResume: false,
      canCancel: false,
      errorCategory: null
    });
  }

  return {
    sessionId,
    statusLabel,
    materials: [
      {
        materialId: "material-workspace-video",
        materialLabel: "城市街景.mp4",
        artifactKind: "thumbnail",
        status: "ready",
        statusLabel: "资源就绪",
        progressPerMille: 1000,
        canRefresh: true,
        canRetry: false,
        canResume: false,
        canCancel: false,
        displayRef: null,
        errorCategory: null
      },
      {
        materialId: "material-workspace-video",
        materialLabel: "城市街景.mp4",
        artifactKind: "waveform",
        status: "running",
        statusLabel: "生成中",
        progressPerMille: 420,
        canRefresh: true,
        canRetry: false,
        canResume: false,
        canCancel: true,
        displayRef: null,
        errorCategory: null
      },
      {
        materialId: "material-workspace-audio",
        materialLabel: "背景音乐.wav",
        artifactKind: "proxy",
        status: "resumable",
        statusLabel: "可继续",
        progressPerMille: 510,
        canRefresh: true,
        canRetry: false,
        canResume: true,
        canCancel: false,
        displayRef: null,
        errorCategory: null
      },
      {
        materialId: "material-workspace-missing",
        materialLabel: "封面图.png",
        artifactKind: "thumbnail",
        status: "failed",
        statusLabel: "生成失败",
        progressPerMille: null,
        canRefresh: true,
        canRetry: true,
        canResume: false,
        canCancel: false,
        displayRef: null,
        errorCategory: "missingSource"
      }
    ],
    tasks,
    quota: buildTestArtifactQuotaStatus(),
    refreshAvailable: true
  };
}

function buildTestArtifactQuotaStatus(): ArtifactQuotaStatus {
  return {
    statusLabel: "缓存空间偏高",
    severity: "warning",
    usedLabel: "2.4 GB",
    reclaimableLabel: "860 MB",
    releasedLabel: "0 MB",
    cleanupAvailable: true
  };
}

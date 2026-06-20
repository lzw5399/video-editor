import { app, BrowserWindow, dialog, ipcMain, screen, type IpcMainInvokeEvent, type Rectangle } from "electron";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import type { CommandEnvelope, CommandState, TimelineSelection } from "../generated/CommandEnvelope";
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
  PreviewArtifactResponse,
  RuntimeCapabilityReport,
  TimelineCommandResponse,
  WaveformDisplayPeaksResponse,
  WaveformDisplayStatus
} from "../generated/CommandResultEnvelope";
import type { Draft, Keyframe, Material, Segment, SegmentVisual, TextSegment, Track } from "../generated/Draft";
import { executeCommand, ping, version } from "./nativeBinding";
import { registerRealtimePreviewHost } from "./realtimePreviewHost";

type TestExecuteCommandCall = {
  command: CommandEnvelope["command"];
  kind: CommandEnvelope["payload"]["kind"];
  requestId: string | null;
  targetTime: number | null;
  targetTimerange: { start: number; duration: number } | null;
  canvasConfig: {
    width: number;
    height: number;
    frameRate: { numerator: number; denominator: number };
  } | null;
  visual: SegmentVisual | null;
  keyframe: Keyframe | null;
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
  deviceSelectionId: string | null;
  maxPeakBins: number | null;
};

type TestWindowMetrics = {
  bounds: Rectangle;
  contentBounds: Rectangle;
  displayScaleFactor: number;
};
type ProjectBundlePickerResponse = {
  canceled: boolean;
  bundlePath: string | null;
};

declare global {
  var __videoEditorTestExecuteCommandCalls: TestExecuteCommandCall[] | undefined;
}

const devServerUrl = process.env.VITE_DEV_SERVER_URL;
const isDevelopment = !app.isPackaged && isLoopbackUrl(devServerUrl);
const packagedRendererFile = join(__dirname, "../renderer/index.html");
const packagedRendererUrl = pathToFileURL(packagedRendererFile).toString();
const allowedRendererUrl = isDevelopment && devServerUrl !== undefined ? devServerUrl : packagedRendererUrl;
const allowedRendererUrlArgument = `--video-editor-allowed-renderer-url=${allowedRendererUrl}`;
hydrateTestEnvironmentFromArguments();
const showDeveloperDiagnostics =
  process.env.VIDEO_EDITOR_DEVELOPER_DIAGNOSTICS === "1" ||
  process.env.VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS === "1";
const testObservationEnabled = process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS === "1";
const testCommandMocksEnabled =
  process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS === "1" &&
  process.env.VIDEO_EDITOR_TEST_COMMAND_MOCKS !== "0";
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
ipcMain.handle("core:executeCommand", (event, command: CommandEnvelope) => {
  assertAllowedIpcSender(event);
  recordTestExecuteCommand(command);
  const testRuntimeCapabilitiesResponse = maybeBuildTestRuntimeCapabilitiesResponse(command);
  if (testRuntimeCapabilitiesResponse !== null) {
    return testRuntimeCapabilitiesResponse;
  }
  const testCanvasResponse = maybeBuildTestCanvasCommandResponse(command);
  if (testCanvasResponse !== null) {
    return testCanvasResponse;
  }
  const testVisualResponse = maybeBuildTestVisualCommandResponse(command);
  if (testVisualResponse !== null) {
    return testVisualResponse;
  }
  const testTextResponse = maybeBuildTestTextCommandResponse(command);
  if (testTextResponse !== null) {
    return testTextResponse;
  }
  const testTimelineAudioResponse = maybeBuildTestTimelineAudioCommandResponse(command);
  if (testTimelineAudioResponse !== null) {
    return testTimelineAudioResponse;
  }
  const testKeyframeResponse = maybeBuildTestKeyframeCommandResponse(command);
  if (testKeyframeResponse !== null) {
    return testKeyframeResponse;
  }
  const testPreviewResponse = maybeBuildTestPreviewResponse(command);
  if (testPreviewResponse !== null) {
    return testPreviewResponse;
  }
  const testExportResponse = maybeBuildTestExportResponse(command);
  if (testExportResponse !== null) {
    return testExportResponse;
  }
  const testArtifactResponse = maybeBuildTestArtifactResponse(command);
  if (testArtifactResponse !== null) {
    return testArtifactResponse;
  }
  const testAudioResponse = maybeBuildTestAudioResponse(command);
  if (testAudioResponse !== null) {
    return testAudioResponse;
  }
  return executeCommand(command);
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
ipcMain.handle("platform:pathToFileUrl", (event, filePath: string) => {
  assertAllowedIpcSender(event);
  return pathToFileURL(filePath).toString();
});
if (testObservationEnabled) {
  ipcMain.handle("test:getExecuteCommandCalls", (event) => {
    assertAllowedIpcSender(event);
    return globalThis.__videoEditorTestExecuteCommandCalls ?? [];
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
    return {
      bounds: window.getBounds(),
      contentBounds: window.getContentBounds(),
      displayScaleFactor: screen.getDisplayMatching(window.getBounds()).scaleFactor
    };
  });
}

async function createWindow(): Promise<void> {
  const window = new BrowserWindow({
    width: 1280,
    height: 800,
    minWidth: 960,
    minHeight: 640,
    backgroundColor: "#171717",
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
  setEnvFromArgument("VIDEO_EDITOR_TEST_COMMAND_MOCKS", "--video-editor-test-command-mocks=");
  setEnvFromArgument("VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES", "--video-editor-test-mock-runtime-capabilities=");
}

function testRendererArgument(envName: string, prefix: string): string[] {
  const value = process.env[envName];
  return value === undefined || value.trim().length === 0 ? [] : [`${prefix}${encodeURIComponent(value)}`];
}

function setEnvFromArgument(envName: string, prefix: string): void {
  if (process.env[envName] !== undefined) {
    return;
  }
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

function recordTestExecuteCommand(command: CommandEnvelope): void {
  if (!testObservationEnabled) {
    return;
  }

  const targetTime = command.payload.kind === "requestPreviewFrame" ? command.payload.targetTime : null;
  const targetTimerange =
    command.payload.kind === "requestPreviewSegment" ||
    command.payload.kind === "addTextSegment" ||
    command.payload.kind === "addAudioSegment"
      ? command.payload.targetTimerange
      : null;
  const canvasConfig = command.payload.kind === "updateDraftCanvasConfig" ? command.payload.canvasConfig : null;
  const visual = command.payload.kind === "updateSegmentVisual" ? command.payload.visual : null;
  const keyframe = command.payload.kind === "setSegmentKeyframe" ? command.payload.keyframe : null;
  const keyframeProperty =
    command.payload.kind === "setSegmentKeyframe"
      ? command.payload.keyframe.property
      : command.payload.kind === "removeSegmentKeyframe"
        ? command.payload.property
        : null;
  const keyframeAt =
    command.payload.kind === "setSegmentKeyframe"
      ? command.payload.keyframe.at
      : command.payload.kind === "removeSegmentKeyframe"
        ? command.payload.at
        : null;
  const text =
    command.payload.kind === "addTextSegment" || command.payload.kind === "editTextSegment" ? command.payload.text : null;
  const srtContent = command.payload.kind === "importSubtitleSrt" ? command.payload.srtContent : null;
  const outputPath = command.payload.kind === "startExport" ? command.payload.outputPath : null;
  const preset = command.payload.kind === "startExport" ? command.payload.preset : null;
  const sessionId = isAudioPreviewCommandKind(command.payload.kind) ? command.payload.sessionId ?? null : null;
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

  globalThis.__videoEditorTestExecuteCommandCalls ??= [];
  globalThis.__videoEditorTestExecuteCommandCalls.push({
    command: command.command,
    kind: command.payload.kind,
    requestId: command.requestId ?? null,
    targetTime,
    targetTimerange,
    canvasConfig,
    visual,
    keyframe,
    keyframeProperty,
    keyframeAt,
    textContent: text?.content ?? null,
    textSource: text?.source ?? null,
    textFontRef: text?.style.font.fontRef ?? null,
    srtContent,
    outputPath,
    preset,
    jobId,
    sessionId,
    deviceSelectionId,
    maxPeakBins
  });
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

function maybeBuildTestVisualCommandResponse(command: CommandEnvelope): CommandResultEnvelope<TimelineCommandResponse> | null {
  if (command.payload.kind !== "updateSegmentVisual") {
    return null;
  }

  if (!testCommandMocksEnabled) {
    return null;
  }

  const draft = {
    ...command.payload.draft,
    tracks: command.payload.draft.tracks.map((track) => ({
      ...track,
      segments: track.segments.map((segment) =>
        segment.segmentId === command.payload.segmentId ? { ...segment, visual: command.payload.visual } : segment
      )
    }))
  };

  return {
    ok: true,
    data: {
      draft,
      commandState: {
        ...command.payload.commandState,
        undoStack: [
          ...command.payload.commandState.undoStack,
          {
            draft: command.payload.draft,
            selection: command.payload.selection,
            label: "updateSegmentVisual"
          }
        ],
        redoStack: []
      },
      selection: command.payload.selection,
      events: [
        {
          kind: "segmentVisualUpdated",
          message: null
        }
      ]
    },
    error: null,
    events: [
      {
        kind: "segmentVisualUpdated",
        message: null
      }
    ]
  };
}

function maybeBuildTestTextCommandResponse(command: CommandEnvelope): CommandResultEnvelope<TimelineCommandResponse> | null {
  if (
    command.payload.kind !== "addTextSegment" &&
    command.payload.kind !== "editTextSegment" &&
    command.payload.kind !== "importSubtitleSrt"
  ) {
    return null;
  }

  if (!testCommandMocksEnabled) {
    return null;
  }

  if (command.payload.kind === "addTextSegment") {
    const segment: Segment = {
      segmentId: command.payload.segmentId,
      materialId: command.payload.materialId,
      sourceTimerange: command.payload.sourceTimerange,
      targetTimerange: command.payload.targetTimerange,
      mainTrackMagnet: { enabled: false },
      keyframes: [],
      filters: [],
      transition: null,
          text: command.payload.text,
          volume: { levelMillis: 1000 },
          audio: defaultTestSegmentAudio(),
          visual: defaultTestSegmentVisual(command.payload.draft)
        };
    const draft = {
      ...ensureTestTextMaterial(command.payload.draft, command.payload.materialId, "默认文字"),
      tracks: command.payload.draft.tracks.map((track) =>
        track.trackId === command.payload.trackId ? { ...track, segments: [...track.segments, segment] } : track
      )
    };

    return buildTestTimelineCommandResponse(command, draft, command.payload.trackId, [command.payload.segmentId], "textSegmentAdded");
  }

  if (command.payload.kind === "editTextSegment") {
    let trackId = command.payload.selection.trackIds[0] ?? "";
    const draft = {
      ...command.payload.draft,
      tracks: command.payload.draft.tracks.map((track) => ({
        ...track,
        segments: track.segments.map((segment) => {
          if (segment.segmentId !== command.payload.segmentId) {
            return segment;
          }

          trackId = track.trackId;
          return { ...segment, text: command.payload.text };
        })
      }))
    };

    return buildTestTimelineCommandResponse(command, draft, trackId, [command.payload.segmentId], "textSegmentEdited");
  }

  const subtitleText: TextSegment = {
    content: "测试字幕",
    source: "subtitle",
    style: command.payload.style,
    textBox: command.payload.textBox,
    layoutRegion: command.payload.layoutRegion,
    wrapping: command.payload.wrapping,
    bubble: null,
    effect: null
  };
  const materialId = `${command.payload.materialIdPrefix}-1`;
  const segmentId = `${command.payload.segmentIdPrefix}-1`;
  const segment: Segment = {
    segmentId,
    materialId,
    sourceTimerange: { start: 0, duration: 2_000_000 },
    targetTimerange: { start: command.payload.timeOffset, duration: 2_000_000 },
    mainTrackMagnet: { enabled: false },
    keyframes: [],
    filters: [],
    transition: null,
    text: subtitleText,
    volume: { levelMillis: 1000 },
    audio: defaultTestSegmentAudio(),
    visual: defaultTestSegmentVisual(command.payload.draft)
  };
  const draftWithMaterial = ensureTestTextMaterial(command.payload.draft, materialId, "导入字幕");
  const tracks = draftWithMaterial.tracks.some((track) => track.trackId === command.payload.trackId)
    ? draftWithMaterial.tracks.map((track) =>
        track.trackId === command.payload.trackId ? { ...track, segments: [...track.segments, segment] } : track
      )
    : [
        ...draftWithMaterial.tracks,
        {
          trackId: command.payload.trackId,
          kind: "text",
          name: command.payload.trackName,
          muted: false,
          locked: false,
          visible: true,
          segments: [segment]
        } satisfies Track
      ];
  const draft = {
    ...draftWithMaterial,
    tracks
  };

  return buildTestTimelineCommandResponse(command, draft, command.payload.trackId, [segmentId], "subtitleSrtImported");
}

function maybeBuildTestTimelineAudioCommandResponse(command: CommandEnvelope): CommandResultEnvelope<TimelineCommandResponse> | null {
  if (
    command.payload.kind !== "addAudioSegment" &&
    command.payload.kind !== "setSegmentVolume" &&
    command.payload.kind !== "setTrackMute" &&
    command.payload.kind !== "updateSegmentAudio"
  ) {
    return null;
  }

  if (!testCommandMocksEnabled) {
    return null;
  }

  if (command.payload.kind === "addAudioSegment") {
    const segment: Segment = {
      segmentId: command.payload.segmentId,
      materialId: command.payload.materialId,
      sourceTimerange: command.payload.sourceTimerange,
      targetTimerange: command.payload.targetTimerange,
      mainTrackMagnet: { enabled: false },
      keyframes: [],
      filters: [],
      transition: null,
      text: null,
      volume: { levelMillis: 1000 },
      audio: defaultTestSegmentAudio(),
      visual: defaultTestSegmentVisual(command.payload.draft)
    };
    const draft: Draft = {
      ...command.payload.draft,
      tracks: command.payload.draft.tracks.map((track) =>
        track.trackId === command.payload.trackId ? { ...track, segments: [...track.segments, segment] } : track
      )
    };

    return buildTestTimelineCommandResponse(command, draft, command.payload.trackId, [command.payload.segmentId], "audioSegmentAdded");
  }

  if (command.payload.kind === "setTrackMute") {
    const draft: Draft = {
      ...command.payload.draft,
      tracks: command.payload.draft.tracks.map((track) =>
        track.trackId === command.payload.trackId ? { ...track, muted: command.payload.muted } : track
      )
    };

    return buildTestTimelineCommandResponse(
      command,
      draft,
      command.payload.trackId,
      command.payload.selection.segmentIds,
      "trackMuteSet"
    );
  }

  const segmentId = command.payload.segmentId;
  let trackId = command.payload.selection.trackIds[0] ?? "";
  const draft: Draft = {
    ...command.payload.draft,
    tracks: command.payload.draft.tracks.map((track) => ({
      ...track,
      segments: track.segments.map((segment) => {
        if (segment.segmentId !== segmentId) {
          return segment;
        }

        trackId = track.trackId;
        if (command.payload.kind === "setSegmentVolume") {
          return {
            ...segment,
            volume: command.payload.volume,
            audio: {
              ...(segment.audio ?? defaultTestSegmentAudio()),
              gainMillis: command.payload.volume.levelMillis
            }
          };
        }

        return {
          ...segment,
          audio: {
            ...(segment.audio ?? defaultTestSegmentAudio()),
            gainMillis: command.payload.gainMillis ?? segment.audio?.gainMillis ?? segment.volume.levelMillis,
            panBalanceMillis: command.payload.panBalanceMillis ?? segment.audio?.panBalanceMillis ?? 0,
            fadeInDuration: command.payload.fadeInDuration ?? segment.audio?.fadeInDuration ?? { duration: 0 },
            fadeOutDuration: command.payload.fadeOutDuration ?? segment.audio?.fadeOutDuration ?? { duration: 0 },
            effectSlots: command.payload.effectSlots ?? segment.audio?.effectSlots ?? []
          },
          volume:
            command.payload.gainMillis === null || command.payload.gainMillis === undefined
              ? segment.volume
              : { levelMillis: command.payload.gainMillis }
        };
      })
    }))
  };

  return buildTestTimelineCommandResponse(command, draft, trackId, [segmentId], "segmentAudioUpdated");
}

function maybeBuildTestKeyframeCommandResponse(command: CommandEnvelope): CommandResultEnvelope<TimelineCommandResponse> | null {
  if (command.payload.kind !== "setSegmentKeyframe" && command.payload.kind !== "removeSegmentKeyframe") {
    return null;
  }

  if (!testCommandMocksEnabled) {
    return null;
  }

  let trackId = command.payload.selection.trackIds[0] ?? "";
  const draft: Draft = {
    ...command.payload.draft,
    tracks: command.payload.draft.tracks.map((track) => ({
      ...track,
      segments: track.segments.map((segment) => {
        if (segment.segmentId !== command.payload.segmentId) {
          return segment;
        }

        trackId = track.trackId;

        if (command.payload.kind === "setSegmentKeyframe") {
          const keyframes = [
            ...segment.keyframes.filter(
              (keyframe) =>
                keyframe.property !== command.payload.keyframe.property || keyframe.at !== command.payload.keyframe.at
            ),
            command.payload.keyframe
          ].sort(compareKeyframes);

          return {
            ...segment,
            keyframes
          };
        }

        return {
          ...segment,
          keyframes: segment.keyframes
            .filter((keyframe) => keyframe.property !== command.payload.property || keyframe.at !== command.payload.at)
            .sort(compareKeyframes)
        };
      })
    }))
  };

  return buildTestTimelineCommandResponse(
    command,
    draft,
    trackId,
    command.payload.selection.segmentIds,
    command.payload.kind === "setSegmentKeyframe" ? "segmentKeyframeSet" : "segmentKeyframeRemoved"
  );
}

function compareKeyframes(left: Keyframe, right: Keyframe): number {
  const propertyOrder = left.property.localeCompare(right.property);
  return propertyOrder === 0 ? left.at - right.at : propertyOrder;
}

function buildTestTimelineCommandResponse(
  command: {
    payload: {
      kind: string;
      draft: Draft;
      commandState: CommandState;
      selection: TimelineSelection;
    };
  },
  draft: Draft,
  trackId: string,
  segmentIds: string[],
  eventKind: string
): CommandResultEnvelope<TimelineCommandResponse> {
  return {
    ok: true,
    data: {
      draft,
      commandState: {
        ...command.payload.commandState,
        undoStack: [
          ...command.payload.commandState.undoStack,
          {
            draft: command.payload.draft,
            selection: command.payload.selection,
            label: command.payload.kind
          }
        ],
        redoStack: []
      },
      selection: {
        segmentIds,
        trackIds: [trackId]
      },
      events: [
        {
          kind: eventKind,
          message: null
        }
      ]
    },
    error: null,
    events: [
      {
        kind: eventKind,
        message: null
      }
    ]
  };
}

function ensureTestTextMaterial(draft: Draft, materialId: string, displayName: string): Draft {
  if (draft.materials.some((material) => material.materialId === materialId)) {
    return draft;
  }

  const material: Material = {
    materialId,
    kind: "text",
    uri: `text://${materialId}`,
    displayName,
    metadata: {
      hasVideo: false,
      hasAudio: false
    },
    status: "available"
  };

  return {
    ...draft,
    materials: [...draft.materials, material]
  };
}

function defaultTestSegmentVisual(draft: Draft): SegmentVisual {
  return (
    draft.tracks.flatMap((track) => track.segments).find((segment) => segment.visual !== undefined)?.visual ?? {
      visible: true,
      transform: {
        position: { x: 0, y: 0 },
        scale: { xMillis: 1000, yMillis: 1000 },
        rotation: { degrees: 0 },
        opacity: { valueMillis: 1000 },
        crop: { leftMillis: 0, rightMillis: 0, topMillis: 0, bottomMillis: 0 },
        anchor: { xMillis: 500, yMillis: 500 }
      },
      fitMode: "fit",
      backgroundFilling: { kind: "none" },
      blendMode: { kind: "normal" },
      mask: { kind: "none" }
    }
  );
}

function defaultTestSegmentAudio() {
  return {
    gainMillis: 1000,
    panBalanceMillis: 0,
    fadeInDuration: { duration: 0 },
    fadeOutDuration: { duration: 0 },
    effectSlots: []
  };
}

function maybeBuildTestCanvasCommandResponse(command: CommandEnvelope): CommandResultEnvelope<TimelineCommandResponse> | null {
  if (command.payload.kind !== "updateDraftCanvasConfig") {
    return null;
  }

  if (!testCommandMocksEnabled) {
    return null;
  }

  const draft = {
    ...command.payload.draft,
    canvasConfig: command.payload.canvasConfig
  };

  return {
    ok: true,
    data: {
      draft,
      commandState: {
        ...command.payload.commandState,
        undoStack: [
          ...command.payload.commandState.undoStack,
          {
            draft: command.payload.draft,
            selection: command.payload.selection,
            label: "updateDraftCanvasConfig"
          }
        ],
        redoStack: []
      },
      selection: command.payload.selection,
      events: [
        {
          kind: "draftCanvasConfigUpdated",
          message: null
        }
      ]
    },
    error: null,
    events: [
      {
        kind: "draftCanvasConfigUpdated",
        message: null
      }
    ]
  };
}

function maybeBuildTestRuntimeCapabilitiesResponse(
  command: CommandEnvelope
): CommandResultEnvelope<RuntimeCapabilityReport> | null {
  if (command.payload.kind !== "probeRuntimeCapabilities") {
    return null;
  }

  if (process.env.VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES === "0") {
    return null;
  }

  if (process.env.VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES === "error") {
    return {
      ok: false,
      data: null,
      error: {
        kind: "runtimeDiscoveryFailed",
        message: "运行环境检测失败，请检查 FFmpeg/ffprobe 路径后重试。",
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
        source: "PATH",
        version: "ffmpeg version test",
        configureSummary: "configuration: test-runtime",
        status: "ready",
        diagnostic: null
      },
      ffprobe: {
        kind: "ffprobe",
        path: "/tmp/video-editor-test-runtime/ffprobe",
        source: "PATH",
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
        externalRuntime: true,
        redistributableBuild: false,
        source: "externalRuntime",
        message: "当前使用本机 FFmpeg，仅用于本地测试，不代表可再发行构建。"
      },
      diagnostics: []
    },
    error: null,
    events: []
  };
}

function maybeBuildTestPreviewResponse(command: CommandEnvelope): CommandResultEnvelope<PreviewArtifactResponse> | null {
  if (process.env.VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS !== "1") {
    return null;
  }

  if (command.payload.kind === "requestPreviewFrame") {
    return {
      ok: true,
      data: {
        profile: "framePng",
        path: `/tmp/video-editor-preview-cache/test-frame-${command.payload.targetTime}.png`,
        mimeType: "image/png",
        status: "generated",
        targetTimerange: {
          start: command.payload.targetTime,
          duration: 33_333
        },
        diagnostic: null
      },
      error: null,
      events: []
    };
  }

  if (command.payload.kind === "requestPreviewSegment") {
    return {
      ok: true,
      data: {
        profile: "segmentMp4",
        path: `/tmp/video-editor-preview-cache/test-segment-${command.payload.targetTimerange.start}.mp4`,
        mimeType: "video/mp4",
        status: "cached",
        targetTimerange: command.payload.targetTimerange,
        diagnostic: null
      },
      error: null,
      events: []
    };
  }

  return null;
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

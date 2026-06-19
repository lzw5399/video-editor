import { app, BrowserWindow, dialog, ipcMain, type IpcMainInvokeEvent } from "electron";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import type { CommandEnvelope, CommandState, TimelineSelection } from "../generated/CommandEnvelope";
import type {
  ArtifactMaintenanceResult,
  ArtifactStatusSummary,
  ArtifactQuotaStatus,
  CommandResultEnvelope,
  ExportJobStatusResponse,
  PreviewArtifactResponse,
  RuntimeCapabilityReport,
  TimelineCommandResponse
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
  srtContent: string | null;
  outputPath: string | null;
  preset: string | null;
  jobId: string | null;
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
const showDeveloperDiagnostics =
  process.env.VIDEO_EDITOR_DEVELOPER_DIAGNOSTICS === "1" ||
  process.env.VIDEO_EDITOR_TEST_SHOW_DEVELOPER_DIAGNOSTICS === "1";
const rendererArguments = [
  allowedRendererUrlArgument,
  ...(showDeveloperDiagnostics ? ["--video-editor-developer-diagnostics=1"] : []),
  ...(process.env.VIDEO_EDITOR_TEST_WORKSPACE_FIXTURE === "demo" ? ["--video-editor-workspace-fixture=demo"] : [])
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
ipcMain.handle("platform:pathToFileUrl", (event, filePath: string) => {
  assertAllowedIpcSender(event);
  return pathToFileURL(filePath).toString();
});

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

app.whenReady().then(createWindow);

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
  const raw = process.env.VIDEO_EDITOR_TEST_OPEN_MATERIAL_FILES;
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

function recordTestExecuteCommand(command: CommandEnvelope): void {
  if (process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS !== "1") {
    return;
  }

  const targetTime = command.payload.kind === "requestPreviewFrame" ? command.payload.targetTime : null;
  const targetTimerange = command.payload.kind === "requestPreviewSegment" ? command.payload.targetTimerange : null;
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
    srtContent,
    outputPath,
    preset,
    jobId
  });
}

function maybeBuildTestVisualCommandResponse(command: CommandEnvelope): CommandResultEnvelope<TimelineCommandResponse> | null {
  if (command.payload.kind !== "updateSegmentVisual") {
    return null;
  }

  if (process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS !== "1") {
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

  if (process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS !== "1") {
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
          segments: [segment]
        } satisfies Track
      ];
  const draft = {
    ...draftWithMaterial,
    tracks
  };

  return buildTestTimelineCommandResponse(command, draft, command.payload.trackId, [segmentId], "subtitleSrtImported");
}

function maybeBuildTestKeyframeCommandResponse(command: CommandEnvelope): CommandResultEnvelope<TimelineCommandResponse> | null {
  if (command.payload.kind !== "setSegmentKeyframe" && command.payload.kind !== "removeSegmentKeyframe") {
    return null;
  }

  if (process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS !== "1") {
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
      fitMode: "stretch",
      backgroundFilling: { kind: "none" },
      blendMode: { kind: "normal" },
      mask: { kind: "none" }
    }
  );
}

function maybeBuildTestCanvasCommandResponse(command: CommandEnvelope): CommandResultEnvelope<TimelineCommandResponse> | null {
  if (command.payload.kind !== "updateDraftCanvasConfig") {
    return null;
  }

  if (process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS !== "1") {
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
        availableFontPaths: ["/System/Library/Fonts/PingFang.ttc"],
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

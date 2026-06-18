import { app, BrowserWindow, ipcMain, type IpcMainInvokeEvent } from "electron";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type {
  CommandResultEnvelope,
  ExportJobStatusResponse,
  PreviewArtifactResponse,
  RuntimeCapabilityReport,
  TimelineCommandResponse
} from "../generated/CommandResultEnvelope";
import type { SegmentVisual } from "../generated/Draft";
import { executeCommand, ping, version } from "./nativeBinding";

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
  const testPreviewResponse = maybeBuildTestPreviewResponse(command);
  if (testPreviewResponse !== null) {
    return testPreviewResponse;
  }
  const testExportResponse = maybeBuildTestExportResponse(command);
  if (testExportResponse !== null) {
    return testExportResponse;
  }
  return executeCommand(command);
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
      additionalArguments: [allowedRendererUrlArgument]
    }
  });

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

function recordTestExecuteCommand(command: CommandEnvelope): void {
  if (process.env.VIDEO_EDITOR_TEST_RECORD_COMMANDS !== "1") {
    return;
  }

  const targetTime = command.payload.kind === "requestPreviewFrame" ? command.payload.targetTime : null;
  const targetTimerange = command.payload.kind === "requestPreviewSegment" ? command.payload.targetTimerange : null;
  const canvasConfig = command.payload.kind === "updateDraftCanvasConfig" ? command.payload.canvasConfig : null;
  const visual = command.payload.kind === "updateSegmentVisual" ? command.payload.visual : null;
  const outputPath = command.payload.kind === "startExport" ? command.payload.outputPath : null;
  const preset = command.payload.kind === "startExport" ? command.payload.preset : null;
  const jobId =
    command.payload.kind === "getExportJobStatus" || command.payload.kind === "cancelExport"
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

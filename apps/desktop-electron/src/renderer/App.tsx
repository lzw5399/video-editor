import { useEffect, useRef, useState } from "react";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type {
  AudioOutputDeviceSummary,
  AudioPreviewCommandResponse,
  AudioPreviewStatusResponse,
  ArtifactMaintenanceResult,
  ArtifactQuotaStatus,
  ArtifactStatusSummary,
  CommandResultEnvelope,
  ExportJobStatusResponse,
  ImportMaterialResponse,
  ListMaterialsResponse,
  ListMissingMaterialsResponse,
  PreviewArtifactResponse,
  RuntimeCapabilityReport,
  TimelineCommandResponse,
  WaveformDisplayPeaksResponse
} from "../generated/CommandResultEnvelope";
import type { ExportPreset } from "../generated/CommandEnvelope";
import type {
  Draft,
  DraftCanvasConfig,
  Material,
  MaterialKind,
  Keyframe,
  KeyframeEasing,
  KeyframeInterpolation,
  KeyframeProperty,
  KeyframeValue,
  SegmentVisual,
  SegmentVolume,
  TextSegment,
  TrackKind
} from "../generated/Draft";
import {
  applyTimelineCommandResult,
  buildAddSegmentCommand,
  buildAddAudioSegmentCommand,
  buildAddTextSegmentCommand,
  buildCancelAudioPreviewCommand,
  buildCancelArtifactGenerationCommand,
  buildDeleteSegmentCommand,
  buildEditTextSegmentCommand,
  buildCancelExportCommand,
  buildCreateAudioPreviewSessionCommand,
  buildGetAudioPreviewStatusCommand,
  buildGetArtifactQuotaStatusCommand,
  buildGetArtifactStatusCommand,
  buildGetExportJobStatusCommand,
  buildGetWaveformDisplayPeaksCommand,
  buildImportMaterialCommand,
  buildImportSubtitleSrtCommand,
  buildListMaterialsCommand,
  buildListMissingMaterialsCommand,
  buildMoveSegmentCommand,
  buildProbeRuntimeCapabilitiesCommand,
  buildPlayAudioPreviewCommand,
  buildPauseAudioPreviewCommand,
  buildRequestPreviewFrameCommand,
  buildRequestPreviewSegmentCommand,
  buildRefreshWaveformStatusCommand,
  buildRefreshArtifactStatusCommand,
  buildRedoTimelineEditCommand,
  buildRemoveSegmentKeyframeCommand,
  buildResumeArtifactGenerationCommand,
  buildRetryArtifactGenerationCommand,
  buildRunArtifactGarbageCollectionCommand,
  buildSelectTimelineSegmentsCommand,
  buildSeekAudioPreviewCommand,
  buildSelectAudioOutputDeviceCommand,
  buildListAudioOutputDevicesCommand,
  buildSetSegmentKeyframeCommand,
  buildSetSegmentVolumeCommand,
  buildSetTrackMuteCommand,
  buildSplitSegmentCommand,
  buildStopAudioPreviewCommand,
  buildStartExportCommand,
  buildTrimSegmentCommand,
  buildUndoTimelineEditCommand,
  buildUpdateDraftCanvasConfigCommand,
  buildUpdateSegmentAudioCommand,
  buildUpdateSegmentVisualCommand,
  commandErrorMessage,
  runtimeDiagnosticsFromError,
  runtimeDiagnosticsFromReport
} from "./commandHelpers";
import {
  createCheckingRuntimeDiagnosticsState,
  createInitialWorkspaceState,
  audioDevicesFromSummaries,
  audioPreviewFromCommandResponse,
  audioPreviewFromStatusResponse,
  artifactPreviewStatusLabel,
  findFirstMaterialByKind,
  findTrackByKind,
  formatExportDiagnostic,
  formatCommandError,
  formatMicroseconds,
  formatPreviewStatus,
  getSelectedSegmentView,
  getSelectedTrackView,
  nextTrackStart,
  resourcePanelFromArtifactStatus,
  resourcePanelWithError,
  resourcePanelWithMaintenanceResult,
  resourcePanelWithQuota,
  waveformDisplayFromResponse,
  resolveWorkspaceStartupDraft,
  type ExportDisplayState,
  type PreviewDisplayState,
  type WorkspaceStartupFixture,
  type WorkspaceCategory,
  type WorkspaceState
} from "./viewModel";
import { WorkspaceShell } from "./workspace/WorkspaceShell";

type PingResponse = { pong: boolean };
type VersionResponse = { coreVersion: string; contractVersion: string };

const PREVIEW_SEGMENT_DURATION_US = 2_000_000;

type VideoEditorCoreApi = {
  ping: () => Promise<CommandResultEnvelope<PingResponse>>;
  version: () => Promise<CommandResultEnvelope<VersionResponse>>;
  executeCommand: <T = unknown>(command: CommandEnvelope) => Promise<CommandResultEnvelope<T>>;
};
type OpenMaterialFilesResponse = {
  canceled: boolean;
  filePaths: string[];
};
type VideoEditorPlatformApi = {
  openMaterialFiles: () => Promise<OpenMaterialFilesResponse>;
  pathToFileUrl: (path: string) => Promise<string>;
};
type RealtimePreviewHostState = {
  ok: boolean;
  statusLabel: string;
  fallbackLabel: string | null;
  playbackGeneration: number | null;
};

type DraftCommandBuilder = (current: WorkspaceState) => CommandEnvelope;
type DraftCommandResultApplier<T> = (
  current: WorkspaceState,
  result: CommandResultEnvelope<T>,
  command: CommandEnvelope
) => WorkspaceState;
type ExecutedDraftCommand<T> = {
  command: CommandEnvelope;
  result: CommandResultEnvelope<T>;
};
type ExportCommandResultApplier = (
  current: WorkspaceState,
  result: CommandResultEnvelope<ExportJobStatusResponse>
) => WorkspaceState;
type ArtifactCommandResultApplier<T> = (
  current: WorkspaceState,
  result: CommandResultEnvelope<T>
) => WorkspaceState;
type DerivedStateInvalidationCopy = {
  frameStatusLabel: string;
  frameMetadataLabel: string;
  segmentStatusLabel: string;
  segmentMetadataLabel: string;
  exportLogSummary: string;
};

const CANVAS_DERIVED_STATE_COPY: DerivedStateInvalidationCopy = {
  frameStatusLabel: "画布已更新，请重新请求预览帧",
  frameMetadataLabel: "预览帧需要重新生成",
  segmentStatusLabel: "画布已更新，请重新生成预览片段",
  segmentMetadataLabel: "预览片段需要重新生成",
  exportLogSummary: "草稿已更新，请重新开始导出"
};

const VISUAL_DERIVED_STATE_COPY: DerivedStateInvalidationCopy = {
  frameStatusLabel: "画面变换已更新，请重新请求预览帧",
  frameMetadataLabel: "预览帧需要重新生成",
  segmentStatusLabel: "画面变换已更新，请重新生成预览片段",
  segmentMetadataLabel: "预览片段需要重新生成",
  exportLogSummary: "画面变换已更新，请重新开始导出"
};

const TEXT_DERIVED_STATE_COPY: DerivedStateInvalidationCopy = {
  frameStatusLabel: "文字已更新，请重新请求预览帧",
  frameMetadataLabel: "预览帧需要重新生成",
  segmentStatusLabel: "文字已更新，请重新生成预览片段",
  segmentMetadataLabel: "预览片段需要重新生成",
  exportLogSummary: "文字已更新，请重新开始导出"
};

const KEYFRAME_DERIVED_STATE_COPY: DerivedStateInvalidationCopy = {
  frameStatusLabel: "关键帧已更新，请重新请求预览帧",
  frameMetadataLabel: "预览帧需要重新生成",
  segmentStatusLabel: "关键帧已更新，请重新生成预览片段",
  segmentMetadataLabel: "预览片段需要重新生成",
  exportLogSummary: "关键帧已更新，请重新开始导出"
};

const AUDIO_DERIVED_STATE_COPY: DerivedStateInvalidationCopy = {
  frameStatusLabel: "音频已更新，请重新请求预览帧",
  frameMetadataLabel: "预览帧需要重新生成",
  segmentStatusLabel: "音频已更新，请重新生成预览片段",
  segmentMetadataLabel: "预览片段需要重新生成",
  exportLogSummary: "音频已更新，请重新开始导出"
};

declare global {
  interface Window {
    videoEditorAppConfig?: {
      workspaceFixture?: WorkspaceStartupFixture;
      showDeveloperDiagnostics?: boolean;
    };
    videoEditorCore: VideoEditorCoreApi;
    videoEditorPlatform?: VideoEditorPlatformApi;
  }
}

export function App(): React.ReactElement {
  const startupFixture = readWorkspaceStartupFixture();
  const showDeveloperDiagnostics = window.videoEditorAppConfig?.showDeveloperDiagnostics === true;
  const [workspace, setWorkspace] = useState<WorkspaceState>(() =>
    createInitialWorkspaceState(resolveWorkspaceStartupDraft(startupFixture))
  );
  const [activeCategory, setActiveCategory] = useState<WorkspaceCategory>("媒体");
  const [bundlePath, setBundlePath] = useState(startupFixture === "demo" ? "/tmp/phase-04-demo.veproj" : "/tmp/video-editor-workspace.veproj");
  const [materialPath, setMaterialPath] = useState(startupFixture === "demo" ? "/tmp/demo-material.mp4" : "");
  const [playheadUs, setPlayheadUs] = useState(0);
  const [playbackRunning, setPlaybackRunning] = useState(false);
  const workspaceRef = useRef(workspace);
  const playheadRef = useRef(playheadUs);
  const commandInFlightRef = useRef(false);
  const audioCommandInFlightRef = useRef(false);
  const runtimeProbeInFlightRef = useRef(false);
  const pendingAutoPreviewTimeRef = useRef<number | null>(null);
  const autoPreviewRetryTimerRef = useRef<number | null>(null);
  const autoPreviewRetryCountRef = useRef(0);
  const playbackClockRef = useRef<{ lastTimestampMs: number | null; accumulatedUs: number }>({
    lastTimestampMs: null,
    accumulatedUs: 0
  });

  useEffect(() => {
    workspaceRef.current = workspace;
  }, [workspace]);

  useEffect(() => {
    playheadRef.current = playheadUs;
  }, [playheadUs]);

  useEffect(() => {
    flushPendingAutoPreviewFrame();
  }, [workspace.pendingCommand, workspace.runtimeDiagnostics.canPreview]);

  useEffect(() => {
    if (!playbackRunning) {
      playbackClockRef.current = { lastTimestampMs: null, accumulatedUs: 0 };
      return;
    }

    let animationFrame: number | null = null;

    const tick = (timestampMs: number) => {
      const clock = playbackClockRef.current;
      if (clock.lastTimestampMs === null) {
        clock.lastTimestampMs = timestampMs;
      }

      const elapsedUs = Math.max(0, Math.round((timestampMs - clock.lastTimestampMs) * 1000));
      clock.lastTimestampMs = timestampMs;
      clock.accumulatedUs += elapsedUs;

      const frameStepUs = frameDurationUs(workspaceRef.current.draft.canvasConfig);
      const sequenceDurationUs = getSequenceDurationUs(workspaceRef.current);
      if (sequenceDurationUs <= 0) {
        setPlaybackRunning(false);
        return;
      }

      if (clock.accumulatedUs >= frameStepUs) {
        const elapsedFrames = Math.max(1, Math.floor(clock.accumulatedUs / frameStepUs));
        clock.accumulatedUs %= frameStepUs;
        const targetTime = Math.min(sequenceDurationUs, playheadRef.current + elapsedFrames * frameStepUs);
        setPlayheadUs(targetTime);

        if (targetTime >= sequenceDurationUs) {
          void stopRealtimePreviewHost();
          setPlaybackRunning(false);
          return;
        }
      }

      animationFrame = window.requestAnimationFrame(tick);
    };

    animationFrame = window.requestAnimationFrame(tick);
    return () => {
      if (animationFrame !== null) {
        window.cancelAnimationFrame(animationFrame);
      }
    };
  }, [playbackRunning]);

  useEffect(() => {
    return () => {
      if (autoPreviewRetryTimerRef.current !== null) {
        window.clearTimeout(autoPreviewRetryTimerRef.current);
      }
    };
  }, []);

  useEffect(() => {
    const artifactPath = workspace.preview.frameArtifactPath;
    const platform = window.videoEditorPlatform;
    if (artifactPath === null || workspace.preview.frameDisplayUrl !== null || platform === undefined) {
      return;
    }

    let cancelled = false;
    void platform.pathToFileUrl(artifactPath).then(
      (displayUrl) => {
        if (cancelled) {
          return;
        }
        setWorkspace((current) => {
          if (current.preview.frameArtifactPath !== artifactPath) {
            return current;
          }
          const next = {
            ...current,
            preview: {
              ...current.preview,
              frameDisplayUrl: displayUrl
            }
          };
          workspaceRef.current = next;
          return next;
        });
      },
      () => {
        if (cancelled) {
          return;
        }
        setWorkspace((current) => {
          if (current.preview.frameArtifactPath !== artifactPath) {
            return current;
          }
          const next = {
            ...current,
            preview: {
              ...current.preview,
              frameDisplayUrl: null
            }
          };
          workspaceRef.current = next;
          return next;
        });
      }
    );

    return () => {
      cancelled = true;
    };
  }, [workspace.preview.frameArtifactPath, workspace.preview.frameDisplayUrl]);

  useEffect(() => {
    let cancelled = false;

    async function bootstrapWorkspace(): Promise<void> {
      const [ping, version, materialList] = await Promise.all([
        window.videoEditorCore.ping(),
        window.videoEditorCore.version(),
        window.videoEditorCore.executeCommand<ListMaterialsResponse>(buildListMaterialsCommand(workspaceRef.current.draft))
      ]);

      if (cancelled) {
        return;
      }

      if (!ping.ok || !version.ok || !materialList.ok) {
        const message =
          ping.error?.message ??
          version.error?.message ??
          materialList.error?.message ??
          "剪辑核心连接失败";
        setWorkspace((current) => ({
          ...current,
          bindingStatus: {
            kind: "error",
            label: formatCommandError(message)
          },
          commandError: formatCommandError(message)
        }));
        return;
      }

      setWorkspace((current) => ({
        ...current,
        materials: materialList.data?.materials ?? current.materials,
        bindingStatus: {
          kind: "ready",
          label: `剪辑核心已连接 ${version.data?.coreVersion ?? "0.0.0"} / 合约 ${
            version.data?.contractVersion ?? "0.0.0"
          }`
        },
        commandError: null
      }));

      void handleGetArtifactStatus();
      void handleProbeRuntimeCapabilities();
      window.setTimeout(() => {
        void refreshWaveformDisplay();
      }, 250);
    }

    void bootstrapWorkspace().catch((error: unknown) => {
      if (!cancelled) {
        const message = error instanceof Error ? error.message : String(error);
        setWorkspace((current) => ({
          ...current,
          bindingStatus: {
            kind: "error",
            label: formatCommandError(message)
          },
          commandError: formatCommandError(message)
        }));
      }
    });

    return () => {
      cancelled = true;
    };
  }, []);

  function readWorkspaceStartupFixture(): WorkspaceStartupFixture {
    return window.videoEditorAppConfig?.workspaceFixture === "demo" ? "demo" : "blank";
  }

  async function executeDraftCommand<T>(
    buildCommand: DraftCommandBuilder,
    pendingCommand: string,
    applyResult: DraftCommandResultApplier<T>
  ): Promise<ExecutedDraftCommand<T> | null> {
    if (commandInFlightRef.current) {
      setWorkspace((current) => {
        const next = {
          ...current,
          commandError: commandErrorMessage("上一个操作仍在执行，请等待剪辑核心返回")
        };
        workspaceRef.current = next;
        return next;
      });
      return null;
    }

    commandInFlightRef.current = true;
    setWorkspace((current) => {
      const next = {
        ...current,
        pendingCommand,
        commandError: null
      };
      workspaceRef.current = next;
      return next;
    });

    try {
      const command = buildCommand(workspaceRef.current);
      const result = await window.videoEditorCore.executeCommand<T>(command);
      setWorkspace((current) => {
        const next = applyResult(current, result, command);
        workspaceRef.current = next;
        return next;
      });
      return { command, result };
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspace((current) => {
        const next = {
          ...current,
          pendingCommand: null,
          commandError: commandErrorMessage(message)
        };
        workspaceRef.current = next;
        return next;
      });
      return null;
    } finally {
      commandInFlightRef.current = false;
    }
  }

  async function executeTimelineCommand(
    buildCommand: DraftCommandBuilder,
    pendingCommand: string
  ): Promise<ExecutedDraftCommand<TimelineCommandResponse> | null> {
    const executed = await executeDraftCommand<TimelineCommandResponse>(buildCommand, pendingCommand, (current, result, command) => {
      const applied = applyTimelineCommandResult(
        {
          draft: current.draft,
          commandState: current.commandState,
          selection: current.selection
        },
        result
      );

      const next = {
        ...current,
        draft: applied.state.draft,
        commandState: applied.state.commandState,
        selection: applied.state.selection,
        materials: applied.state.draft.materials,
        pendingCommand: null,
        commandError: applied.errorMessage
      };

      if (result.ok && result.data !== null && command.payload.kind === "updateSegmentVisual") {
        return {
          ...next,
          preview: clearDerivedPreviewState(current.preview, VISUAL_DERIVED_STATE_COPY),
          export: clearDerivedExportState(current.export, VISUAL_DERIVED_STATE_COPY.exportLogSummary)
        };
      }

      if (
        result.ok &&
        result.data !== null &&
        (command.payload.kind === "addTextSegment" ||
          command.payload.kind === "editTextSegment" ||
          command.payload.kind === "importSubtitleSrt")
      ) {
        return {
          ...next,
          preview: clearDerivedPreviewState(current.preview, TEXT_DERIVED_STATE_COPY),
          export: clearDerivedExportState(current.export, TEXT_DERIVED_STATE_COPY.exportLogSummary)
        };
      }

      if (
        result.ok &&
        result.data !== null &&
        (command.payload.kind === "setSegmentKeyframe" || command.payload.kind === "removeSegmentKeyframe")
      ) {
        return {
          ...next,
          preview: clearDerivedPreviewState(current.preview, KEYFRAME_DERIVED_STATE_COPY),
          export: clearDerivedExportState(current.export, KEYFRAME_DERIVED_STATE_COPY.exportLogSummary)
        };
      }

      if (
        result.ok &&
        result.data !== null &&
        (command.payload.kind === "setSegmentVolume" ||
          command.payload.kind === "updateSegmentAudio" ||
          command.payload.kind === "setTrackMute" ||
          command.payload.kind === "addAudioSegment")
      ) {
        return {
          ...next,
          preview: clearDerivedPreviewState(current.preview, AUDIO_DERIVED_STATE_COPY),
          export: clearDerivedExportState(current.export, AUDIO_DERIVED_STATE_COPY.exportLogSummary)
        };
      }

      return next;
    });

    if (
      executed !== null &&
      executed.result.ok &&
      executed.result.data !== null &&
      executed.command.payload.kind === "addSegment"
    ) {
      queueAutoPreviewFrame(executed.command.payload.targetTimerange.start);
    }

    return executed;
  }

  async function executePreviewCommand(
    buildCommand: DraftCommandBuilder,
    pendingCommand: string,
    applyResult: DraftCommandResultApplier<PreviewArtifactResponse>
  ): Promise<void> {
    if (commandInFlightRef.current) {
      setWorkspace((current) => {
        const message = commandErrorMessage("上一个操作仍在执行，请等待剪辑核心返回");
        const next = {
          ...current,
          commandError: message,
          preview: {
            ...current.preview,
            error: message
          }
        };
        workspaceRef.current = next;
        return next;
      });
      return;
    }

    commandInFlightRef.current = true;
    setWorkspace((current) => {
      const next = {
        ...current,
        pendingCommand,
        commandError: null,
        preview: {
          ...current.preview,
          error: null,
          frameArtifactPath: pendingCommand === "请求预览帧" ? null : current.preview.frameArtifactPath,
          frameDisplayUrl: pendingCommand === "请求预览帧" ? null : current.preview.frameDisplayUrl,
          frameStatusLabel: pendingCommand === "请求预览帧" ? "正在请求预览帧" : current.preview.frameStatusLabel,
          segmentStatusLabel: pendingCommand === "生成预览片段" ? "正在生成预览片段" : current.preview.segmentStatusLabel
        }
      };
      workspaceRef.current = next;
      return next;
    });

    try {
      const command = buildCommand(workspaceRef.current);
      const result = await window.videoEditorCore.executeCommand<PreviewArtifactResponse>(command);
      setWorkspace((current) => {
        const next = applyResult(current, result, command);
        workspaceRef.current = next;
        return next;
      });
    } catch (error: unknown) {
      const message = previewCommandErrorMessage(error instanceof Error ? error.message : String(error), pendingCommand);
      setWorkspace((current) => {
        const next = {
          ...current,
          pendingCommand: null,
          commandError: message,
          preview: {
            ...current.preview,
            error: message,
            frameArtifactPath: pendingCommand === "请求预览帧" ? null : current.preview.frameArtifactPath,
            frameDisplayUrl: pendingCommand === "请求预览帧" ? null : current.preview.frameDisplayUrl,
            frameStatusLabel: pendingCommand === "请求预览帧" ? "预览帧失败" : current.preview.frameStatusLabel,
            segmentStatusLabel: pendingCommand === "生成预览片段" ? "预览片段失败" : current.preview.segmentStatusLabel
          }
        };
        workspaceRef.current = next;
        return next;
      });
    } finally {
      commandInFlightRef.current = false;
    }
  }

  async function executeExportCommand(
    buildCommand: DraftCommandBuilder,
    pendingCommand: string,
    applyResult: ExportCommandResultApplier
  ): Promise<void> {
    if (commandInFlightRef.current) {
      setWorkspace((current) => {
        const message = commandErrorMessage("上一个操作仍在执行，请等待剪辑核心返回");
        const next = {
          ...current,
          commandError: message,
          export: {
            ...current.export,
            error: message
          }
        };
        workspaceRef.current = next;
        return next;
      });
      return;
    }

    commandInFlightRef.current = true;
    setWorkspace((current) => {
      const next = {
        ...current,
        pendingCommand,
        commandError: null,
        export: {
          ...current.export,
          error: null,
          logSummary:
            pendingCommand === "开始导出"
              ? "正在开始导出"
              : pendingCommand === "查询导出状态"
                ? "正在查询导出状态"
                : "正在取消导出"
        }
      };
      workspaceRef.current = next;
      return next;
    });

    try {
      const command = buildCommand(workspaceRef.current);
      const result = await window.videoEditorCore.executeCommand<ExportJobStatusResponse>(command);
      setWorkspace((current) => {
        const next = applyResult(current, result);
        workspaceRef.current = next;
        return next;
      });
    } catch (error: unknown) {
      const message = exportCommandErrorMessage(error instanceof Error ? error.message : String(error), pendingCommand);
      setWorkspace((current) => {
        const next = {
          ...current,
          pendingCommand: null,
          commandError: message,
          export: {
            ...current.export,
            error: message,
            logSummary: message
          }
        };
        workspaceRef.current = next;
        return next;
      });
    } finally {
      commandInFlightRef.current = false;
    }
  }

  async function executeArtifactCommand<T>(
    buildCommand: (current: WorkspaceState) => CommandEnvelope,
    pendingCommand: string,
    applyResult: ArtifactCommandResultApplier<T>
  ): Promise<void> {
    if (commandInFlightRef.current) {
      setWorkspace((current) => {
        const message = commandErrorMessage("上一个操作仍在执行，请等待剪辑核心返回");
        const next = {
          ...current,
          commandError: message,
          resourcePanel: resourcePanelWithError(current.resourcePanel, message)
        };
        workspaceRef.current = next;
        return next;
      });
      return;
    }

    commandInFlightRef.current = true;
    setWorkspace((current) => {
      const next = {
        ...current,
        pendingCommand,
        commandError: null,
        resourcePanel: {
          ...current.resourcePanel,
          cleanupRunning: pendingCommand === "清理缓存",
          pendingJobId: pendingCommand.startsWith("资源任务") ? current.resourcePanel.pendingJobId : current.resourcePanel.pendingJobId
        }
      };
      workspaceRef.current = next;
      return next;
    });

    try {
      const command = buildCommand(workspaceRef.current);
      const result = await window.videoEditorCore.executeCommand<T>(command);
      setWorkspace((current) => {
        const next = applyResult(current, result);
        workspaceRef.current = next;
        return next;
      });
    } catch (error: unknown) {
      const message = commandErrorMessage(error instanceof Error ? error.message : String(error));
      setWorkspace((current) => {
        const next = {
          ...current,
          pendingCommand: null,
          commandError: message,
          resourcePanel: resourcePanelWithError(current.resourcePanel, message)
        };
        workspaceRef.current = next;
        return next;
      });
    } finally {
      commandInFlightRef.current = false;
    }
  }

  function handleGetArtifactStatus(): void {
    void executeArtifactCommand<ArtifactStatusSummary>(
      (current) =>
        buildGetArtifactStatusCommand({
          sessionId: current.resourcePanel.sessionId,
          bundlePath
        }),
      "读取资源状态",
      applyArtifactStatusResult
    );
  }

  function handleRefreshArtifactStatus(): void {
    void (async () => {
      await executeArtifactCommand<ArtifactStatusSummary>(
        (current) =>
          buildGetArtifactStatusCommand({
            sessionId: current.resourcePanel.sessionId,
            bundlePath
          }),
        "读取资源状态",
        applyArtifactStatusResult
      );
      await executeArtifactCommand<ArtifactStatusSummary>(
        (current) =>
          buildRefreshArtifactStatusCommand({
            sessionId: current.resourcePanel.sessionId,
            bundlePath
          }),
        "刷新状态",
        applyArtifactStatusResult
      );
    })();
  }

  function handleArtifactTaskAction(action: "cancel" | "retry" | "resume", jobId: string): void {
    setWorkspace((current) => {
      const next = {
        ...current,
        resourcePanel: {
          ...current.resourcePanel,
          pendingJobId: jobId,
          tasks:
            action === "cancel"
              ? current.resourcePanel.tasks.map((task) =>
                  task.jobId === jobId ? { ...task, statusLabel: "正在取消", tone: "active" as const } : task
                )
              : current.resourcePanel.tasks
        }
      };
      workspaceRef.current = next;
      return next;
    });

    const buildCommand =
      action === "cancel"
        ? buildCancelArtifactGenerationCommand
        : action === "retry"
          ? buildRetryArtifactGenerationCommand
          : buildResumeArtifactGenerationCommand;
    void executeArtifactCommand<ArtifactStatusSummary>(
      (current) =>
        buildCommand({
          sessionId: current.resourcePanel.sessionId,
          bundlePath,
          jobId
        }),
      `资源任务${action}`,
      applyArtifactStatusResult
    );
  }

  function handlePrepareArtifactCleanup(): void {
    void executeArtifactCommand<ArtifactQuotaStatus>(
      (current) => buildGetArtifactQuotaStatusCommand(current.resourcePanel.sessionId, bundlePath),
      "检查缓存空间",
      (current, result) => ({
        ...current,
        pendingCommand: null,
        commandError: result.ok ? null : commandErrorMessage(result),
        resourcePanel:
          result.ok && result.data !== null
            ? {
                ...resourcePanelWithQuota(current.resourcePanel, result.data),
                cleanupConfirming: true
              }
            : resourcePanelWithError(current.resourcePanel, commandErrorMessage(result))
      })
    );
  }

  function handleConfirmArtifactCleanup(): void {
    void executeArtifactCommand<ArtifactMaintenanceResult>(
      (current) => buildRunArtifactGarbageCollectionCommand(current.resourcePanel.sessionId, bundlePath, false),
      "清理缓存",
      (current, result) => ({
        ...current,
        pendingCommand: null,
        commandError: result.ok ? null : commandErrorMessage(result),
        resourcePanel:
          result.ok && result.data !== null
            ? resourcePanelWithMaintenanceResult(current.resourcePanel, result.data)
            : resourcePanelWithError(current.resourcePanel, commandErrorMessage(result))
      })
    );
  }

  function handleDismissResourceNotice(): void {
    setWorkspace((current) => {
      const next = {
        ...current,
        resourcePanel: {
          ...current.resourcePanel,
          cleanupConfirming: false,
          notice: null,
          maintenance: {
            ...current.resourcePanel.maintenance,
            resultLabel: null,
            errorLabel: null
          }
        }
      };
      workspaceRef.current = next;
      return next;
    });
  }

  async function handleProbeRuntimeCapabilities(): Promise<void> {
    if (runtimeProbeInFlightRef.current) {
      return;
    }

    runtimeProbeInFlightRef.current = true;
    setWorkspace((current) => {
      const next = {
        ...current,
        runtimeDiagnostics: createCheckingRuntimeDiagnosticsState(),
        commandError: null
      };
      workspaceRef.current = next;
      return next;
    });

    try {
      const result = await window.videoEditorCore.executeCommand<RuntimeCapabilityReport>(
        buildProbeRuntimeCapabilitiesCommand()
      );
      setWorkspace((current) => {
        const runtimeDiagnostics =
          result.ok && result.data !== null
            ? runtimeDiagnosticsFromReport(result.data)
            : runtimeDiagnosticsFromError(result.error?.message ?? "运行环境检测失败");
        const next = {
          ...current,
          runtimeDiagnostics,
          commandError: result.ok ? current.commandError : runtimeDiagnostics.statusLabel
        };
        workspaceRef.current = next;
        return next;
      });
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspace((current) => {
        const runtimeDiagnostics = runtimeDiagnosticsFromError(message);
        const next = {
          ...current,
          runtimeDiagnostics,
          commandError: runtimeDiagnostics.statusLabel
        };
        workspaceRef.current = next;
        return next;
      });
    } finally {
      runtimeProbeInFlightRef.current = false;
    }
  }

  async function executeAudioCommand<T>(
    buildCommand: (current: WorkspaceState) => CommandEnvelope,
    pendingAudioCommand: string,
    applyResult: (current: WorkspaceState, result: CommandResultEnvelope<T>) => WorkspaceState
  ): Promise<CommandResultEnvelope<T> | null> {
    if (audioCommandInFlightRef.current) {
      setWorkspace((current) => {
        const next = {
          ...current,
          commandError: commandErrorMessage("上一个音频操作仍在执行，请稍后重试")
        };
        workspaceRef.current = next;
        return next;
      });
      return null;
    }

    audioCommandInFlightRef.current = true;
    setWorkspace((current) => {
      const next = {
        ...current,
        pendingAudioCommand,
        commandError: null
      };
      workspaceRef.current = next;
      return next;
    });

    try {
      const command = buildCommand(workspaceRef.current);
      const result = await window.videoEditorCore.executeCommand<T>(command);
      setWorkspace((current) => {
        const next = applyResult(current, result);
        workspaceRef.current = next;
        return next;
      });
      return result;
    } catch (error: unknown) {
      const message = commandErrorMessage(error instanceof Error ? error.message : String(error));
      setWorkspace((current) => {
        const next = {
          ...current,
          pendingAudioCommand: null,
          commandError: message
        };
        workspaceRef.current = next;
        return next;
      });
      return null;
    } finally {
      audioCommandInFlightRef.current = false;
    }
  }

  async function ensureAudioPreviewSession(): Promise<string | null> {
    const existingSessionId = workspaceRef.current.audioPreview.sessionId;
    if (existingSessionId !== null) {
      return existingSessionId;
    }

    const result = await executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        buildCreateAudioPreviewSessionCommand({
          draft: current.draft,
          targetTime: playheadRef.current
        }),
      "创建音频预览",
      applyAudioPreviewCommandResult
    );

    return result?.ok === true && result.data !== null ? result.data.sessionId : null;
  }

  async function refreshAudioDevices(): Promise<void> {
    await executeAudioCommand<AudioOutputDeviceSummary[]>(
      () => buildListAudioOutputDevicesCommand(),
      "读取输出设备",
      (current, result) => ({
        ...current,
        pendingAudioCommand: null,
        commandError: result.ok ? null : commandErrorMessage(result),
        audioDevices:
          result.ok && result.data !== null
            ? audioDevicesFromSummaries(result.data, current.audioDevices.selectedDeviceId)
            : current.audioDevices
      })
    );
  }

  async function refreshAudioPreviewStatus(): Promise<void> {
    const sessionId = await ensureAudioPreviewSession();
    if (sessionId === null) {
      return;
    }

    await executeAudioCommand<AudioPreviewStatusResponse>(
      () =>
        buildGetAudioPreviewStatusCommand({
          sessionId,
          targetTime: playheadRef.current
        }),
      "读取音频状态",
      (current, result) => ({
        ...current,
        pendingAudioCommand: null,
        commandError: result.ok ? null : commandErrorMessage(result),
        audioPreview: result.ok && result.data !== null ? audioPreviewFromStatusResponse(result.data) : current.audioPreview
      })
    );
  }

  async function refreshWaveformDisplay(): Promise<void> {
    const materialId = firstAudioMaterialId(workspaceRef.current);
    if (materialId === null) {
      return;
    }

    await executeAudioCommand<WaveformDisplayPeaksResponse>(
      (current) =>
        buildGetWaveformDisplayPeaksCommand({
          draft: current.draft,
          materialId,
          maxPeakBins: 16
        }),
      "读取波形",
      applyWaveformResult
    );
    await executeAudioCommand<WaveformDisplayPeaksResponse>(
      (current) =>
        buildRefreshWaveformStatusCommand({
          draft: current.draft,
          materialId,
          maxPeakBins: 16
        }),
      "刷新波形",
      applyWaveformResult
    );
  }

  async function importMaterialPath(path: string): Promise<void> {
    await executeDraftCommand<ImportMaterialResponse>(
      (current) =>
        buildImportMaterialCommand({
          draft: current.draft,
          bundlePath,
          materialPath: path
        }),
      "导入素材",
      (current, result) => {
        if (!result.ok || result.data === null) {
          return {
            ...current,
            pendingCommand: null,
            commandError: commandErrorMessage(result)
          };
        }

        return {
          ...current,
          draft: result.data.draft,
          materials: result.data.draft.materials,
          materialDiagnostics: result.data.diagnostic === null || result.data.diagnostic === undefined ? [] : [result.data.diagnostic],
          pendingCommand: null,
          commandError: null
        };
      }
    );
  }

  async function handleImportMaterial(): Promise<void> {
    if (commandInFlightRef.current) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("上一个操作仍在执行，请等待剪辑核心返回")
      }));
      return;
    }

    const platform = window.videoEditorPlatform;
    if (platform === undefined) {
      const fallbackPath = materialPath.trim();
      if (fallbackPath.length === 0) {
        setWorkspace((current) => ({
          ...current,
          commandError: commandErrorMessage("当前环境没有系统文件选择器，请填写素材路径后重试")
        }));
        return;
      }
      await importMaterialPath(fallbackPath);
      return;
    }

    try {
      const result = await platform.openMaterialFiles();
      if (result.canceled || result.filePaths.length === 0) {
        return;
      }

      setMaterialPath(result.filePaths[0] ?? "");
      for (const selectedPath of result.filePaths) {
        await importMaterialPath(selectedPath);
      }
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspace((current) => ({
        ...current,
        pendingCommand: null,
        commandError: commandErrorMessage(message)
      }));
    }
  }

  async function handleImportMaterialFromPath(): Promise<void> {
    const fallbackPath = materialPath.trim();
    if (fallbackPath.length === 0) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("请先填写素材路径")
      }));
      return;
    }
    await importMaterialPath(fallbackPath);
  }

  async function handleRefreshMaterials(): Promise<void> {
    setWorkspace((current) => ({
      ...current,
      pendingCommand: "刷新素材",
      commandError: null
    }));

    try {
      const result = await window.videoEditorCore.executeCommand<ListMaterialsResponse>(buildListMaterialsCommand(workspace.draft));
      setWorkspace((current) => ({
        ...current,
        materials: result.ok && result.data !== null ? result.data.materials : current.materials,
        pendingCommand: null,
        commandError: result.ok ? null : commandErrorMessage(result)
      }));
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspace((current) => ({
        ...current,
        pendingCommand: null,
        commandError: commandErrorMessage(message)
      }));
    }
  }

  async function handleListMissingMaterials(): Promise<void> {
    setWorkspace((current) => ({
      ...current,
      pendingCommand: "检查丢失素材",
      commandError: null
    }));

    try {
      const result = await window.videoEditorCore.executeCommand<ListMissingMaterialsResponse>(
        buildListMissingMaterialsCommand(workspace.draft, bundlePath)
      );
      setWorkspace((current) => ({
        ...current,
        materialDiagnostics: result.ok && result.data !== null ? result.data.diagnostics : current.materialDiagnostics,
        pendingCommand: null,
        commandError: result.ok ? null : commandErrorMessage(result)
      }));
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspace((current) => ({
        ...current,
        pendingCommand: null,
        commandError: commandErrorMessage(message)
      }));
    }
  }

  function handleAddTextSegment(text: TextSegment, durationUs: number): void {
    const safeDurationUs = toPositiveMicroseconds(durationUs);
    const segmentId = `text-segment-${Date.now().toString(36)}`;
    const materialId = `text-material-${Date.now().toString(36)}`;

    void executeTimelineCommand(
      (current) => {
        const textTrack = findTrackByKind(current.draft, "text");
        if (textTrack === null) {
          throw new Error("当前草稿没有文字轨道");
        }

        return buildAddTextSegmentCommand({
          context: current,
          trackId: textTrack.trackId,
          segmentId,
          materialId,
          sourceTimerange: { start: 0, duration: safeDurationUs },
          targetTimerange: { start: nextTrackStart(textTrack), duration: safeDurationUs },
          text
        });
      },
      "添加文字"
    );
  }

  function handleImportSubtitleSrt(srtContent: string, timeOffsetUs: number, textTemplate: TextSegment): void {
    const batchId = Date.now().toString(36);

    void executeTimelineCommand(
      (current) =>
        buildImportSubtitleSrtCommand({
          context: current,
          trackId: "track-subtitle",
          trackName: "字幕",
          srtContent,
          timeOffset: Math.max(0, Math.round(timeOffsetUs)),
          segmentIdPrefix: `subtitle-segment-${batchId}`,
          materialIdPrefix: `subtitle-material-${batchId}`,
          style: textTemplate.style,
          textBox: textTemplate.textBox,
          layoutRegion: textTemplate.layoutRegion,
          wrapping: textTemplate.wrapping
        }),
      "导入字幕"
    );
  }

  function handleAddAudioSegment(materialId: string, durationUs: number): void {
    const safeDurationUs = toPositiveMicroseconds(durationUs);
    const segmentId = `audio-segment-${Date.now().toString(36)}`;
    void executeTimelineCommand(
      (current) => {
        const audioTrack = findTrackByKind(current.draft, "audio");
        const audioMaterial = materialId.length > 0 ? { materialId } : findFirstMaterialByKind(current.draft, "audio");

        if (audioTrack === null || audioMaterial === null) {
          throw new Error("当前草稿没有可用音频轨道或音频素材");
        }

        return buildAddAudioSegmentCommand({
          context: current,
          trackId: audioTrack.trackId,
          segmentId,
          materialId: audioMaterial.materialId,
          sourceTimerange: { start: 0, duration: safeDurationUs },
          targetTimerange: { start: nextTrackStart(audioTrack), duration: safeDurationUs }
        });
      },
      "添加音频"
    );
  }

  function handleSetSelectedSegmentVolume(levelMillis: number): void {
    const volume: SegmentVolume = {
      levelMillis: Math.max(0, Math.min(4000, Math.round(levelMillis)))
    };

    void executeTimelineCommand(
      (current) => {
        const selectedSegment = getSelectedSegmentView(current.draft, current.selection);
        if (selectedSegment === null) {
          throw new Error("请先选择一个片段");
        }
        return buildSetSegmentVolumeCommand(current, selectedSegment.segment.segmentId, volume);
      },
      "调整音量"
    );
  }

  function handleEditSelectedText(text: TextSegment): void {
    void executeTimelineCommand(
      (current) => {
        const selectedSegment = getSelectedSegmentView(current.draft, current.selection);
        if (selectedSegment === null || selectedSegment.segment.text === null || selectedSegment.segment.text === undefined) {
          throw new Error("请先选择一个文字片段");
        }
        return buildEditTextSegmentCommand(current, selectedSegment.segment.segmentId, text);
      },
      "应用文字"
    );
  }

  function handleSetSelectedTrackMute(trackId: string, muted: boolean): void {
    void executeTimelineCommand((current) => {
      const selectedTrack = getSelectedTrackView(current.draft, current.selection);
      const resolvedTrackId = trackId || selectedTrack?.trackId;

      if (resolvedTrackId === undefined) {
        throw new Error("请先选择一个轨道");
      }

      return buildSetTrackMuteCommand(current, resolvedTrackId, muted);
    }, "切换轨道静音");
  }

  function handleUpdateSelectedSegmentAudio(options: {
    gainMillis: number;
    panBalanceMillis: number;
    fadeInDuration: number;
    fadeOutDuration: number;
  }): void {
    void executeTimelineCommand(
      (current) => {
        const selectedSegment = getSelectedSegmentView(current.draft, current.selection);
        if (selectedSegment === null) {
          throw new Error("请先选择一个音频片段");
        }

        return buildUpdateSegmentAudioCommand({
          context: current,
          segmentId: selectedSegment.segment.segmentId,
          gainMillis: Math.max(0, Math.min(4000, Math.round(options.gainMillis))),
          panBalanceMillis: Math.max(-1000, Math.min(1000, Math.round(options.panBalanceMillis))),
          fadeInDuration: { duration: Math.max(0, Math.round(options.fadeInDuration)) },
          fadeOutDuration: { duration: Math.max(0, Math.round(options.fadeOutDuration)) },
          effectSlots: []
        });
      },
      "应用音频"
    );
  }

  function handleSelectTimelineSegment(segmentId: string): void {
    void executeTimelineCommand(
      (current) => {
        const selected = getSelectedSegmentView(current.draft, {
          segmentIds: [segmentId],
          trackIds: []
        });

        if (selected === null) {
          throw new Error("找不到要选择的片段");
        }

        return buildSelectTimelineSegmentsCommand(current, [segmentId], [selected.track.trackId]);
      },
      "选择片段"
    );
  }

  function handleAddTimelineSegment(materialId: string): void {
    const segmentId = `segment-${Date.now().toString(36)}`;
    void executeTimelineCommand(
      (current) => {
        const material = resolveTimelineMaterial(current.draft, materialId);
        const track = material === null ? null : findTrackByKind(current.draft, compatibleTrackKind(material.kind));

        if (material === null || track === null) {
          throw new Error("没有可添加到时间线的兼容素材或轨道");
        }

        const duration = toPositiveMicroseconds(material.metadata.duration ?? 3_000_000);
        const insertedTargetStart = nextTrackStart(track);
        return buildAddSegmentCommand({
          context: current,
          trackId: track.trackId,
          segmentId,
          materialId: material.materialId,
          sourceTimerange: {
            start: 0,
            duration
          },
          targetTimerange: {
            start: insertedTargetStart,
            duration
          }
        });
      },
      "添加片段"
    );
  }

  function handleMoveSelectedSegment(deltaUs: number): void {
    void executeTimelineCommand(
      (current) => {
        const selected = getSelectedSegmentView(current.draft, current.selection);
        if (selected === null) {
          throw new Error("请先选择一个片段");
        }

        return buildMoveSegmentCommand(
          current,
          selected.segment.segmentId,
          selected.track.trackId,
          Math.max(0, selected.segment.targetTimerange.start + Math.round(deltaUs))
        );
      },
      "移动片段"
    );
  }

  function handleSplitSelectedSegment(splitAt: number): void {
    const rightSegmentId = `segment-right-${Date.now().toString(36)}`;
    void executeTimelineCommand(
      (current) => {
        const selected = getSelectedSegmentView(current.draft, current.selection);
        if (selected === null) {
          throw new Error("请先选择一个片段");
        }

        return buildSplitSegmentCommand(current, selected.segment.segmentId, rightSegmentId, Math.max(0, Math.round(splitAt)));
      },
      "分割片段"
    );
  }

  function handleTrimSelectedSegment(direction: "left" | "right", deltaUs: number): void {
    const safeDelta = Math.max(1, Math.round(deltaUs));

    void executeTimelineCommand(
      (current) => {
        const selected = getSelectedSegmentView(current.draft, current.selection);
        if (selected === null) {
          throw new Error("请先选择一个片段");
        }

        const currentRange = selected.segment.targetTimerange;
        const targetTimerange =
          direction === "left"
            ? {
                start: currentRange.start + safeDelta,
                duration: Math.max(1, currentRange.duration - safeDelta)
              }
            : {
                start: currentRange.start,
                duration: Math.max(1, currentRange.duration - safeDelta)
              };

        return buildTrimSegmentCommand(current, selected.segment.segmentId, direction, targetTimerange);
      },
      direction === "left" ? "左侧裁剪" : "右侧裁剪"
    );
  }

  function handleDeleteSelectedSegment(): void {
    const selected = getSelectedSegmentView(workspaceRef.current.draft, workspaceRef.current.selection);

    if (selected === null) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("请先选择一个片段")
      }));
      return;
    }

    if (!window.confirm("删除片段：确定删除所选片段？此操作可通过撤销恢复。")) {
      return;
    }

    void executeTimelineCommand(
      (current) => {
        const currentSelected = getSelectedSegmentView(current.draft, current.selection);
        if (currentSelected === null) {
          throw new Error("请先选择一个片段");
        }
        return buildDeleteSegmentCommand(current, currentSelected.segment.segmentId);
      },
      "删除片段"
    );
  }

  function handleUndoTimelineEdit(): void {
    void executeTimelineCommand((current) => buildUndoTimelineEditCommand(current), "撤销");
  }

  function handleRedoTimelineEdit(): void {
    void executeTimelineCommand((current) => buildRedoTimelineEditCommand(current), "重做");
  }

  function handleUpdateDraftCanvasConfig(canvasConfig: DraftCanvasConfig): void {
    void executeDraftCommand<TimelineCommandResponse>(
      (current) => buildUpdateDraftCanvasConfigCommand(current, canvasConfig),
      "应用草稿参数",
      (current, result) => {
        if (!result.ok || result.data === null) {
          return {
            ...current,
            pendingCommand: null,
            commandError: canvasCommandErrorMessage(result)
          };
        }

        return {
          ...current,
          draft: result.data.draft,
          commandState: result.data.commandState,
          selection: result.data.selection,
          materials: result.data.draft.materials,
          preview: clearDerivedPreviewState(current.preview),
          export: clearDerivedExportState(current.export),
          pendingCommand: null,
          commandError: null
        };
      }
    );
  }

  function handleUpdateSelectedSegmentVisual(visual: SegmentVisual): void {
    void (async () => {
      await executeTimelineCommand(
        (current) => {
          const selectedSegment = getSelectedSegmentView(current.draft, current.selection);
          if (selectedSegment === null) {
            throw new Error("请先选择一个片段");
          }

          return buildUpdateSegmentVisualCommand(current, selectedSegment.segment.segmentId, visual);
        },
        "应用画面"
      );

      if (workspaceRef.current.commandError === null) {
        requestPreviewFrameAt(normalizePlayheadTime(playheadUs));
      }
    })();
  }

  function handleSetSelectedSegmentKeyframe(
    property: KeyframeProperty,
    interpolation: KeyframeInterpolation = "linear",
    easing: KeyframeEasing = "none"
  ): void {
    void executeTimelineCommand(
      (current) => {
        const selectedSegment = getSelectedSegmentView(current.draft, current.selection);
        if (selectedSegment === null) {
          throw new Error("请先选择一个片段");
        }

        const keyframe: Keyframe = {
          at: resolveSegmentRelativePlayhead(selectedSegment.segment.targetTimerange.start, selectedSegment.segment.targetTimerange.duration, playheadUs),
          property,
          value: keyframeValueForSegmentProperty(selectedSegment.segment, property),
          interpolation,
          easing
        };

        return buildSetSegmentKeyframeCommand(current, selectedSegment.segment.segmentId, keyframe);
      },
      "设置关键帧"
    );
  }

  function handleRemoveSelectedSegmentKeyframe(property: KeyframeProperty, at: number): void {
    void executeTimelineCommand(
      (current) => {
        const selectedSegment = getSelectedSegmentView(current.draft, current.selection);
        if (selectedSegment === null) {
          throw new Error("请先选择一个片段");
        }

        return buildRemoveSegmentKeyframeCommand(current, selectedSegment.segment.segmentId, property, Math.max(0, Math.round(at)));
      },
      "删除关键帧"
    );
  }

  function handleSeekPlayhead(value: number): void {
    const targetTime = normalizePlayheadTime(value);
    setPlaybackRunning(false);
    setPlayheadUs(targetTime);
    void seekRealtimePreviewHost(targetTime);
    void handleSeekAudioPreview(targetTime);
    requestPreviewFrameAt(targetTime);
  }

  function handleTogglePlayback(): void {
    void (async () => {
      if (playbackRunning) {
        await pauseRealtimePreviewHost();
        await handlePauseAudioPreview();
        setPlaybackRunning(false);
        return;
      }

      const sequenceDurationUs = getSequenceDurationUs(workspaceRef.current);
      if (sequenceDurationUs <= 0) {
        return;
      }

      if (playheadRef.current >= sequenceDurationUs) {
        setPlayheadUs(0);
        playheadRef.current = 0;
      }

      const snapshotReady = await updateRealtimePreviewDraftSnapshot();
      if (!snapshotReady) {
        return;
      }
      const seekReady = await seekRealtimePreviewHost(playheadRef.current);
      if (!seekReady) {
        return;
      }
      const playbackReady = await playRealtimePreviewHost();
      if (!playbackReady) {
        return;
      }

      playbackClockRef.current = { lastTimestampMs: null, accumulatedUs: 0 };
      void handlePlayAudioPreview();
      setPlaybackRunning(true);
    })();
  }

  function handleStopPlayback(): void {
    void handleStopAudioPreview();
    void stopRealtimePreviewHost();
    setPlaybackRunning(false);
    setPlayheadUs(0);
    playheadRef.current = 0;
    requestPreviewFrameAt(0);
  }

  async function updateRealtimePreviewDraftSnapshot(): Promise<boolean> {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return applyRealtimePreviewHostError("实时预览宿主不可用");
    }

    try {
      return applyRealtimePreviewHostState(await bridge.updateDraftSnapshot(workspaceRef.current.draft, bundlePath));
    } catch (error: unknown) {
      return applyRealtimePreviewHostError(error instanceof Error ? error.message : String(error));
    }
  }

  async function seekRealtimePreviewHost(targetTime: number): Promise<boolean> {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return applyRealtimePreviewHostError("实时预览宿主不可用");
    }

    try {
      return applyRealtimePreviewHostState(await bridge.seek(Math.max(0, Math.round(targetTime))));
    } catch (error: unknown) {
      return applyRealtimePreviewHostError(error instanceof Error ? error.message : String(error));
    }
  }

  async function playRealtimePreviewHost(): Promise<boolean> {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return applyRealtimePreviewHostError("实时预览宿主不可用");
    }

    try {
      return applyRealtimePreviewHostState(await bridge.play());
    } catch (error: unknown) {
      return applyRealtimePreviewHostError(error instanceof Error ? error.message : String(error));
    }
  }

  async function pauseRealtimePreviewHost(): Promise<boolean> {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return applyRealtimePreviewHostError("实时预览宿主不可用");
    }

    try {
      return applyRealtimePreviewHostState(await bridge.pause());
    } catch (error: unknown) {
      return applyRealtimePreviewHostError(error instanceof Error ? error.message : String(error));
    }
  }

  async function stopRealtimePreviewHost(): Promise<boolean> {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return applyRealtimePreviewHostError("实时预览宿主不可用");
    }

    try {
      return applyRealtimePreviewHostState(await bridge.stop());
    } catch (error: unknown) {
      return applyRealtimePreviewHostError(error instanceof Error ? error.message : String(error));
    }
  }

  function applyRealtimePreviewHostState(hostState: RealtimePreviewHostState): boolean {
    if (hostState.ok) {
      return true;
    }
    return applyRealtimePreviewHostError(hostState.fallbackLabel ?? hostState.statusLabel);
  }

  function applyRealtimePreviewHostError(message: string): false {
    const errorMessage = commandErrorMessage(message);
    setWorkspace((current) => {
      const next = {
        ...current,
        commandError: errorMessage,
        preview: {
          ...current.preview,
          error: errorMessage
        }
      };
      workspaceRef.current = next;
      return next;
    });
    return false;
  }

  async function handlePlayAudioPreview(): Promise<void> {
    const sessionId = await ensureAudioPreviewSession();
    if (sessionId === null) {
      return;
    }

    await refreshAudioDevices();
    await executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        buildPlayAudioPreviewCommand({
          draft: current.draft,
          sessionId,
          targetTime: playheadRef.current,
          playbackGeneration: current.audioPreview.generation
        }),
      "播放音频",
      applyAudioPreviewCommandResult
    );
  }

  async function handlePauseAudioPreview(): Promise<void> {
    const sessionId = workspaceRef.current.audioPreview.sessionId;
    if (sessionId === null) {
      return;
    }

    await executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        buildPauseAudioPreviewCommand({
          sessionId,
          targetTime: playheadRef.current,
          playbackGeneration: current.audioPreview.generation
        }),
      "暂停音频",
      applyAudioPreviewCommandResult
    );
  }

  async function handleStopAudioPreview(): Promise<void> {
    const sessionId = workspaceRef.current.audioPreview.sessionId;
    if (sessionId === null) {
      return;
    }

    await executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        buildStopAudioPreviewCommand({
          sessionId,
          targetTime: 0,
          playbackGeneration: current.audioPreview.generation
        }),
      "停止音频",
      applyAudioPreviewCommandResult
    );
  }

  async function handleSeekAudioPreview(targetTime: number): Promise<void> {
    const sessionId = workspaceRef.current.audioPreview.sessionId;
    if (sessionId === null) {
      return;
    }

    await executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        buildSeekAudioPreviewCommand({
          sessionId,
          targetTime,
          playbackGeneration: current.audioPreview.generation
        }),
      "定位音频",
      applyAudioPreviewCommandResult
    );
  }

  async function handleRetryAudioPreview(): Promise<void> {
    const sessionId = await ensureAudioPreviewSession();
    if (sessionId === null) {
      return;
    }

    await executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        buildCancelAudioPreviewCommand({
          sessionId,
          targetTime: playheadRef.current,
          playbackGeneration: current.audioPreview.generation
        }),
      "取消音频请求",
      applyAudioPreviewCommandResult
    );
    await refreshAudioPreviewStatus();
  }

  function handleSelectAudioOutputDevice(deviceSelectionId: string): void {
    void executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        buildSelectAudioOutputDeviceCommand({
          sessionId: current.audioPreview.sessionId,
          deviceSelectionId,
          playbackGeneration: current.audioPreview.generation
        }),
      "选择输出设备",
      (current, result) => ({
        ...current,
        pendingAudioCommand: null,
        commandError: result.ok ? null : commandErrorMessage(result),
        audioDevices:
          result.ok && result.data !== null
            ? {
                ...current.audioDevices,
                selectedDeviceId: deviceSelectionId,
                statusLabel: current.audioDevices.devices.find((device) => device.selectionId === deviceSelectionId)?.statusLabel ?? current.audioDevices.statusLabel
              }
            : current.audioDevices,
        audioPreview:
          result.ok && result.data !== null ? audioPreviewFromCommandResponse(current.audioPreview, result.data) : current.audioPreview
      })
    );
  }

  function queueAutoPreviewFrame(targetTime: number): void {
    setPlayheadUs(targetTime);
    pendingAutoPreviewTimeRef.current = targetTime;
    autoPreviewRetryCountRef.current = 0;
    schedulePendingAutoPreviewFlush();
  }

  function flushPendingAutoPreviewFrame(): void {
    const targetTime = pendingAutoPreviewTimeRef.current;
    if (targetTime === null || commandInFlightRef.current || !workspaceRef.current.runtimeDiagnostics.canPreview) {
      return;
    }

    pendingAutoPreviewTimeRef.current = null;
    requestPreviewFrameAt(targetTime);
  }

  function schedulePendingAutoPreviewFlush(): void {
    if (autoPreviewRetryTimerRef.current !== null) {
      return;
    }

    autoPreviewRetryTimerRef.current = window.setTimeout(() => {
      autoPreviewRetryTimerRef.current = null;
      flushPendingAutoPreviewFrame();
      if (pendingAutoPreviewTimeRef.current !== null && autoPreviewRetryCountRef.current < 80) {
        autoPreviewRetryCountRef.current += 1;
        schedulePendingAutoPreviewFlush();
      }
    }, 50);
  }

  function handleRequestPreviewFrame(): void {
    requestPreviewFrameAt(normalizePlayheadTime(playheadUs));
  }

  function requestPreviewFrameAt(targetTime: number): void {
    if (!workspaceRef.current.runtimeDiagnostics.canPreview) {
      const message = runtimeUnavailableMessage(workspaceRef.current, "预览暂不可用");
      setWorkspace((current) => {
        const next = {
          ...current,
          commandError: message,
          preview: {
            ...current.preview,
            frameArtifactPath: null,
            frameDisplayUrl: null,
            frameStatusLabel: "预览暂不可用",
            error: message
          }
        };
        workspaceRef.current = next;
        return next;
      });
      return;
    }

    void executePreviewCommand(
      (current) =>
        buildRequestPreviewFrameCommand({
          draft: current.draft,
          bundlePath,
          targetTime
        }),
      "请求预览帧",
      (current, result) => {
        if (!result.ok || result.data === null) {
          const message = previewCommandErrorMessage(result, "请求预览帧");
          return {
            ...current,
            pendingCommand: null,
            commandError: message,
            preview: {
              ...current.preview,
              frameArtifactPath: null,
              frameDisplayUrl: null,
              frameStatusLabel: "预览帧失败",
              error: message,
              lastRequestedPlayhead: targetTime
            }
          };
        }

        return {
          ...current,
          pendingCommand: null,
          commandError: null,
          preview: {
            ...current.preview,
            frameArtifactPath: result.data.path,
            frameDisplayUrl: null,
            frameStatusLabel: `预览帧${formatPreviewStatus(result.data.status)}`,
            frameMetadataLabel: `${result.data.mimeType} · ${formatMicroseconds(result.data.targetTimerange.start)}`,
            error: null,
            lastRequestedPlayhead: targetTime
          }
        };
      }
    );
  }

  function handleRequestPreviewSegment(): void {
    if (!workspaceRef.current.runtimeDiagnostics.canPreview) {
      const message = runtimeUnavailableMessage(workspaceRef.current, "预览暂不可用");
      setWorkspace((current) => {
        const next = {
          ...current,
          commandError: message,
          preview: {
            ...current.preview,
            segmentStatusLabel: "预览暂不可用",
            error: message
          }
        };
        workspaceRef.current = next;
        return next;
      });
      return;
    }

    const targetTimerange = {
      start: Math.max(0, Math.round(playheadUs)),
      duration: PREVIEW_SEGMENT_DURATION_US
    };

    void executePreviewCommand(
      (current) =>
        buildRequestPreviewSegmentCommand({
          draft: current.draft,
          bundlePath,
          targetTimerange
        }),
      "生成预览片段",
      (current, result) => {
        const rangeLabel = `${formatMicroseconds(targetTimerange.start)} - ${formatMicroseconds(
          targetTimerange.start + targetTimerange.duration
        )}`;

        if (!result.ok || result.data === null) {
          const message = previewCommandErrorMessage(result, "生成预览片段");
          return {
            ...current,
            pendingCommand: null,
            commandError: message,
            preview: {
              ...current.preview,
              segmentStatusLabel: "预览片段失败",
              error: message,
              lastRequestedPlayhead: targetTimerange.start,
              lastRequestedRangeLabel: rangeLabel
            }
          };
        }

        return {
          ...current,
          pendingCommand: null,
          commandError: null,
          preview: {
            ...current.preview,
            segmentArtifactPath: result.data.path,
            segmentStatusLabel: `预览片段${formatPreviewStatus(result.data.status)}`,
            segmentMetadataLabel: `${result.data.mimeType} · ${rangeLabel}`,
            error: null,
            lastRequestedPlayhead: targetTimerange.start,
            lastRequestedRangeLabel: rangeLabel
          }
        };
      }
    );
  }

  function handleExportOutputPathChange(value: string): void {
    setWorkspace((current) => ({
      ...current,
      export: {
        ...current.export,
        outputPath: value
      }
    }));
  }

  function handleExportPresetChange(value: ExportPreset): void {
    setWorkspace((current) => ({
      ...current,
      export: {
        ...current.export,
        preset: value
      }
    }));
  }

  function handleStartExport(): void {
    if (!workspaceRef.current.runtimeDiagnostics.canExport) {
      const message = runtimeUnavailableMessage(workspaceRef.current, "导出暂不可用");
      setWorkspace((current) => {
        const next = {
          ...current,
          commandError: message,
          export: {
            ...current.export,
            error: message,
            logSummary: message
          }
        };
        workspaceRef.current = next;
        return next;
      });
      return;
    }

    void executeExportCommand(
      (current) =>
        buildStartExportCommand({
          draft: current.draft,
          outputPath: current.export.outputPath,
          preset: current.export.preset
        }),
      "开始导出",
      (current, result) => applyExportCommandResult(current, result, "开始导出")
    );
  }

  function handleRefreshExportStatus(): void {
    void executeExportCommand(
      (current) => {
        if (current.export.jobId === null) {
          throw new Error("请先开始导出");
        }
        return buildGetExportJobStatusCommand(current.export.jobId);
      },
      "查询导出状态",
      (current, result) => applyExportCommandResult(current, result, "查询导出状态")
    );
  }

  function handleCancelExport(): void {
    void executeExportCommand(
      (current) => {
        if (current.export.jobId === null) {
          throw new Error("请先开始导出");
        }
        return buildCancelExportCommand(current.export.jobId);
      },
      "取消导出",
      (current, result) => applyExportCommandResult(current, result, "取消导出")
    );
  }

  return (
        <WorkspaceShell
          workspace={workspace}
          activeCategory={activeCategory}
      showDeveloperDiagnostics={showDeveloperDiagnostics}
      bundlePath={bundlePath}
      materialPath={materialPath}
      playheadUs={playheadUs}
      playbackRunning={playbackRunning}
      onCategoryChange={setActiveCategory}
      onBundlePathChange={setBundlePath}
      onMaterialPathChange={setMaterialPath}
      onPlayheadChange={handleSeekPlayhead}
      onTogglePlayback={handleTogglePlayback}
      onStopPlayback={handleStopPlayback}
      onRequestPreviewFrame={handleRequestPreviewFrame}
      onRequestPreviewSegment={handleRequestPreviewSegment}
      onProbeRuntimeCapabilities={handleProbeRuntimeCapabilities}
      onExportOutputPathChange={handleExportOutputPathChange}
      onExportPresetChange={handleExportPresetChange}
      onStartExport={handleStartExport}
      onRefreshExportStatus={handleRefreshExportStatus}
      onCancelExport={handleCancelExport}
      onRetryAudioPreview={handleRetryAudioPreview}
      onSelectAudioOutputDevice={handleSelectAudioOutputDevice}
      onImportMaterial={handleImportMaterial}
          onImportMaterialFromPath={handleImportMaterialFromPath}
          onRefreshMaterials={handleRefreshMaterials}
          onListMissingMaterials={handleListMissingMaterials}
          onRefreshArtifactStatus={handleRefreshArtifactStatus}
          onCancelArtifactGeneration={(jobId) => handleArtifactTaskAction("cancel", jobId)}
          onRetryArtifactGeneration={(jobId) => handleArtifactTaskAction("retry", jobId)}
          onResumeArtifactGeneration={(jobId) => handleArtifactTaskAction("resume", jobId)}
          onPrepareArtifactCleanup={handlePrepareArtifactCleanup}
          onConfirmArtifactCleanup={handleConfirmArtifactCleanup}
          onDismissResourceNotice={handleDismissResourceNotice}
          onAddTextSegment={handleAddTextSegment}
      onImportSubtitleSrt={handleImportSubtitleSrt}
      onAddAudioSegment={handleAddAudioSegment}
      onEditSelectedText={handleEditSelectedText}
      onUpdateDraftCanvasConfig={handleUpdateDraftCanvasConfig}
      onUpdateSelectedSegmentVisual={handleUpdateSelectedSegmentVisual}
      onSetSelectedSegmentKeyframe={handleSetSelectedSegmentKeyframe}
      onRemoveSelectedSegmentKeyframe={handleRemoveSelectedSegmentKeyframe}
      onSetSelectedSegmentVolume={handleSetSelectedSegmentVolume}
      onUpdateSelectedSegmentAudio={handleUpdateSelectedSegmentAudio}
      onSetSelectedTrackMute={handleSetSelectedTrackMute}
      onSelectTimelineSegment={handleSelectTimelineSegment}
      onAddTimelineSegment={handleAddTimelineSegment}
      onMoveSelectedSegment={handleMoveSelectedSegment}
      onSplitSelectedSegment={handleSplitSelectedSegment}
      onTrimSelectedSegment={handleTrimSelectedSegment}
      onDeleteSelectedSegment={handleDeleteSelectedSegment}
      onSetTimelineTrackMute={handleSetSelectedTrackMute}
      onUndoTimelineEdit={handleUndoTimelineEdit}
      onRedoTimelineEdit={handleRedoTimelineEdit}
    />
  );
}

function applyArtifactStatusResult(
  current: WorkspaceState,
  result: CommandResultEnvelope<ArtifactStatusSummary>
): WorkspaceState {
  if (!result.ok || result.data === null) {
    const message = commandErrorMessage(result);
    return {
      ...current,
      pendingCommand: null,
      commandError: message,
      resourcePanel: resourcePanelWithError(current.resourcePanel, message)
    };
  }

  return {
    ...current,
    pendingCommand: null,
    commandError: null,
    resourcePanel: resourcePanelFromArtifactStatus(result.data)
  };
}

function applyAudioPreviewCommandResult(
  current: WorkspaceState,
  result: CommandResultEnvelope<AudioPreviewCommandResponse>
): WorkspaceState {
  if (!result.ok || result.data === null) {
    return {
      ...current,
      pendingAudioCommand: null,
      commandError: commandErrorMessage(result)
    };
  }

  return {
    ...current,
    pendingAudioCommand: null,
    commandError: null,
    audioPreview: audioPreviewFromCommandResponse(current.audioPreview, result.data)
  };
}

function applyWaveformResult(
  current: WorkspaceState,
  result: CommandResultEnvelope<WaveformDisplayPeaksResponse>
): WorkspaceState {
  if (!result.ok || result.data === null) {
    return {
      ...current,
      pendingAudioCommand: null,
      commandError: commandErrorMessage(result)
    };
  }

  return {
    ...current,
    pendingAudioCommand: null,
    commandError: null,
    waveform: waveformDisplayFromResponse(result.data)
  };
}

function applyExportCommandResult(
  current: WorkspaceState,
  result: CommandResultEnvelope<ExportJobStatusResponse>,
  actionLabel: string
): WorkspaceState {
  const response = result.data;
  const message = result.ok ? null : exportCommandErrorMessage(result, actionLabel);
  const diagnosticLabel =
    response?.diagnostic === null || response?.diagnostic === undefined
      ? null
      : `${formatExportDiagnostic(response.diagnostic.kind) ?? response.diagnostic.kind}：${response.diagnostic.message}`;

  return {
    ...current,
    pendingCommand: null,
    commandError: message,
    export: {
      ...current.export,
      outputPath: response?.outputPath && response.outputPath.length > 0 ? response.outputPath : current.export.outputPath,
      preset: response?.preset ?? current.export.preset,
      jobId: response?.jobId && response.jobId !== "unavailable" ? response.jobId : current.export.jobId,
      phase: response?.phase ?? current.export.phase,
      progressPerMille: response?.progressPerMille ?? current.export.progressPerMille,
      outTime: response?.outTime ?? current.export.outTime,
      logSummary: response?.logSummary ?? message ?? current.export.logSummary,
      validation: response?.validation ?? current.export.validation,
      diagnosticLabel,
      error: message
    }
  };
}

function clearDerivedPreviewState(
  preview: PreviewDisplayState,
  copy: DerivedStateInvalidationCopy = CANVAS_DERIVED_STATE_COPY
): PreviewDisplayState {
  return {
    ...preview,
    frameArtifactPath: null,
    frameDisplayUrl: null,
    frameStatusLabel: copy.frameStatusLabel,
    frameMetadataLabel: copy.frameMetadataLabel,
    segmentArtifactPath: null,
    segmentStatusLabel: copy.segmentStatusLabel,
    segmentMetadataLabel: copy.segmentMetadataLabel,
    error: null,
    lastRequestedPlayhead: null,
    lastRequestedRangeLabel: null
  };
}

function clearDerivedExportState(
  exportState: ExportDisplayState,
  logSummary = CANVAS_DERIVED_STATE_COPY.exportLogSummary
): ExportDisplayState {
  return {
    ...exportState,
    jobId: null,
    phase: null,
    progressPerMille: null,
    outTime: null,
    logSummary,
    validation: null,
    diagnosticLabel: null,
    error: null
  };
}

function previewCommandErrorMessage(resultOrMessage: CommandResultEnvelope<unknown> | string, actionLabel: string): string {
  const kindLabels: Record<string, string> = {
    previewServiceFailed: "预览服务失败",
    runtimeDiscoveryFailed: "运行时发现失败",
    invalidPayload: "命令参数无效",
    internal: "内部错误"
  };
  const commandError =
    typeof resultOrMessage === "string" ? null : resultOrMessage.error;
  const message =
    typeof resultOrMessage === "string"
      ? resultOrMessage
      : resultOrMessage.error?.message ?? "剪辑核心返回未知预览错误";
  const kindLabel = commandError === null ? "预览命令失败" : kindLabels[commandError.kind] ?? commandError.kind;

  return `${actionLabel}失败（${kindLabel}）：${message}`;
}

function exportCommandErrorMessage(resultOrMessage: CommandResultEnvelope<unknown> | string, actionLabel: string): string {
  const kindLabels: Record<string, string> = {
    exportServiceFailed: "导出服务失败",
    runtimeDiscoveryFailed: "运行时发现失败",
    invalidPayload: "命令参数无效",
    internal: "内部错误"
  };
  const commandError =
    typeof resultOrMessage === "string" ? null : resultOrMessage.error;
  const message =
    typeof resultOrMessage === "string"
      ? resultOrMessage
      : resultOrMessage.error?.message ?? "剪辑核心返回未知导出错误";
  const kindLabel = commandError === null ? "导出命令失败" : kindLabels[commandError.kind] ?? commandError.kind;

  return `${actionLabel}失败（${kindLabel}）：${message}`;
}

function canvasCommandErrorMessage(result: CommandResultEnvelope<unknown>): string {
  const message = result.error?.message ?? "剪辑核心返回未知画布错误";
  return `画布参数更新失败：${message}。请检查画布尺寸、帧率或背景设置后重试。`;
}

function runtimeUnavailableMessage(workspace: WorkspaceState, actionLabel: string): string {
  const detail =
    workspace.runtimeDiagnostics.status === "checking"
      ? workspace.runtimeDiagnostics.statusLabel
      : workspace.runtimeDiagnostics.statusDetail || workspace.runtimeDiagnostics.statusLabel;

  return `${actionLabel}：${detail}`;
}

function normalizePlayheadTime(value: number): number {
  return Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0;
}

function frameDurationUs(canvasConfig: DraftCanvasConfig): number {
  const numerator = Math.max(1, Math.round(canvasConfig.frameRate.numerator));
  const denominator = Math.max(1, Math.round(canvasConfig.frameRate.denominator));
  return Math.max(1, Math.round((denominator * 1_000_000) / numerator));
}

function getSequenceDurationUs(workspace: WorkspaceState): number {
  return workspace.draft.tracks.reduce((duration, track) => {
    const trackEnd = track.segments.reduce((end, segment) => {
      return Math.max(end, segment.targetTimerange.start + segment.targetTimerange.duration);
    }, 0);
    return Math.max(duration, trackEnd);
  }, 0);
}

function firstAudioMaterialId(workspace: WorkspaceState): string | null {
  return workspace.materials.find((material) => material.kind === "audio" && material.status === "available")?.materialId ?? null;
}

function resolveTimelineMaterial(draft: Draft, materialId: string): Material | null {
  if (materialId.length > 0) {
    return draft.materials.find((material) => material.materialId === materialId && material.status === "available") ?? null;
  }

  return (
    draft.materials.find(
      (material) =>
        material.status === "available" &&
        (material.kind === "video" || material.kind === "image" || material.kind === "audio")
    ) ?? null
  );
}

function compatibleTrackKind(materialKind: MaterialKind): TrackKind {
  if (materialKind === "audio") {
    return "audio";
  }

  if (materialKind === "text") {
    return "text";
  }

  if (materialKind === "sticker") {
    return "sticker";
  }

  return "video";
}

function resolveSegmentRelativePlayhead(segmentStart: number, segmentDuration: number, playhead: number): number {
  const relative = Math.round(playhead) - segmentStart;
  return Math.max(0, Math.min(Math.max(0, segmentDuration), relative));
}

function keyframeValueForSegmentProperty(
  segment: Draft["tracks"][number]["segments"][number],
  property: KeyframeProperty
): KeyframeValue {
  switch (property) {
    case "visualPositionX":
      return { kind: "int", value: segment.visual.transform.position.x };
    case "visualPositionY":
      return { kind: "int", value: segment.visual.transform.position.y };
    case "visualScaleX":
      return { kind: "uint", value: segment.visual.transform.scale.xMillis };
    case "visualScaleY":
      return { kind: "uint", value: segment.visual.transform.scale.yMillis };
    case "visualRotation":
      return { kind: "int", value: segment.visual.transform.rotation.degrees };
    case "visualOpacity":
      return { kind: "uint", value: segment.visual.transform.opacity.valueMillis };
    case "volume":
      return { kind: "uint", value: segment.volume.levelMillis };
    case "textFontSize":
      assertSegmentHasText(segment, property);
      return { kind: "uint", value: segment.text.style.fontSize };
    case "textColor":
      assertSegmentHasText(segment, property);
      return { kind: "color", value: segment.text.style.color };
    case "textLineHeight":
      assertSegmentHasText(segment, property);
      return { kind: "uint", value: segment.text.style.lineHeightMillis };
    case "textLetterSpacing":
      assertSegmentHasText(segment, property);
      return { kind: "uint", value: segment.text.style.letterSpacingMillis };
    case "textLayoutX":
      assertSegmentHasText(segment, property);
      return { kind: "uint", value: segment.text.layoutRegion.xMillis };
    case "textLayoutY":
      assertSegmentHasText(segment, property);
      return { kind: "uint", value: segment.text.layoutRegion.yMillis };
    case "textLayoutWidth":
      assertSegmentHasText(segment, property);
      return { kind: "uint", value: segment.text.layoutRegion.widthMillis };
    case "textLayoutHeight":
      assertSegmentHasText(segment, property);
      return { kind: "uint", value: segment.text.layoutRegion.heightMillis };
    case "stickerPositionX":
    case "stickerPositionY":
    case "stickerScaleX":
    case "stickerScaleY":
    case "filterParameterUnsupported":
      throw new Error("当前阶段暂不支持该参数动画");
  }
}

function assertSegmentHasText(
  segment: Draft["tracks"][number]["segments"][number],
  property: KeyframeProperty
): asserts segment is Draft["tracks"][number]["segments"][number] & { text: TextSegment } {
  if (segment.text === null || segment.text === undefined) {
    throw new Error(`当前片段没有可用于 ${property} 的文字参数`);
  }
}

function toPositiveMicroseconds(value: number): number {
  return Math.max(1, Math.round(Number.isFinite(value) ? value : 1));
}

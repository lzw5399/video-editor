import { useCallback, useEffect, useRef, useState } from "react";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type {
  AudioPreviewRequest,
  CreateProjectSessionRequest,
  ExportJobRequest,
  ExecuteProjectIntentRequest,
  OpenProjectSessionRequest,
  ProjectSessionImportMaterialResponse,
  ProjectSessionIntentResponse,
  ProjectSessionMaterialsResponse,
  ProjectSessionMissingMaterialsResponse,
  ProjectSessionOpenResponse,
  ProjectSessionReadRequest,
  ProjectSessionRequest,
  ProjectSessionTimelineIntentResponse,
  ProjectSessionClosedResponse,
  RequestProjectSessionPreviewFrameRequest,
  RequestProjectSessionPreviewSegmentRequest,
  StartProjectSessionExportRequest
} from "../main/nativeBinding";
import type {
  AudioOutputDeviceSummary,
  AudioPreviewCommandResponse,
  AudioPreviewStatusResponse,
  ArtifactMaintenanceResult,
  ArtifactQuotaStatus,
  ArtifactStatusSummary,
  CommandResultEnvelope,
  ExportJobStatusResponse,
  PreviewArtifactResponse,
  RuntimeCapabilityReport,
  WaveformDisplayPeaksResponse
} from "../generated/CommandResultEnvelope";
import type { ExportPreset } from "../generated/CommandEnvelope";
import type {
  DraftCanvasConfig,
  KeyframeEasing,
  KeyframeInterpolation,
  KeyframeProperty,
  SegmentVisual,
  SegmentVolume,
  TextSegment,
  TrackKind
} from "../generated/Draft";
import {
  buildCancelArtifactGenerationCommand,
  buildGetArtifactQuotaStatusCommand,
  buildGetArtifactStatusCommand,
  buildProbeRuntimeCapabilitiesCommand,
  buildRefreshArtifactStatusCommand,
  buildResumeArtifactGenerationCommand,
  buildRetryArtifactGenerationCommand,
  buildRunArtifactGarbageCollectionCommand,
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
  formatExportDiagnostic,
  formatCommandError,
  formatMicroseconds,
  formatPreviewStatus,
  resourcePanelFromArtifactStatus,
  resourcePanelWithError,
  resourcePanelWithMaintenanceResult,
  resourcePanelWithQuota,
  waveformDisplayFromResponse,
  type ExportDisplayState,
  type ProjectEntryState,
  type PreviewDisplayState,
  type WorkspaceStartupFixture,
  type WorkspaceCategory,
  type WorkspaceState
} from "./viewModel";
import { WorkspaceShell } from "./workspace/WorkspaceShell";
import type { RealtimePreviewHostState } from "./workspace/PreviewMonitor";

type PingResponse = { pong: boolean };
type VersionResponse = { coreVersion: string; contractVersion: string };

const PREVIEW_SEGMENT_DURATION_US = 2_000_000;
const SEQUENCE_END_EPSILON_US = 7_000;

type VideoEditorCoreApi = {
  ping: () => Promise<CommandResultEnvelope<PingResponse>>;
  version: () => Promise<CommandResultEnvelope<VersionResponse>>;
  executeCommand: <T = unknown>(command: CommandEnvelope) => Promise<CommandResultEnvelope<T>>;
  createProjectSession: (request: CreateProjectSessionRequest) => Promise<CommandResultEnvelope<ProjectSessionOpenResponse>>;
  openProjectSession: (request: OpenProjectSessionRequest) => Promise<CommandResultEnvelope<ProjectSessionOpenResponse>>;
  executeProjectIntent: <T = ProjectSessionIntentResponse>(
    request: ExecuteProjectIntentRequest
  ) => Promise<CommandResultEnvelope<T>>;
  listProjectSessionMaterials: (request: ProjectSessionReadRequest) => Promise<CommandResultEnvelope<ProjectSessionMaterialsResponse>>;
  listProjectSessionMissingMaterials: (
    request: ProjectSessionReadRequest
  ) => Promise<CommandResultEnvelope<ProjectSessionMissingMaterialsResponse>>;
  startProjectSessionExport: (
    request: StartProjectSessionExportRequest
  ) => Promise<CommandResultEnvelope<ExportJobStatusResponse>>;
  getExportJobStatus: (request: ExportJobRequest) => Promise<CommandResultEnvelope<ExportJobStatusResponse>>;
  cancelExport: (request: ExportJobRequest) => Promise<CommandResultEnvelope<ExportJobStatusResponse>>;
  createAudioPreviewSession: (request: AudioPreviewRequest) => Promise<CommandResultEnvelope<AudioPreviewCommandResponse>>;
  playAudioPreview: (request: AudioPreviewRequest) => Promise<CommandResultEnvelope<AudioPreviewCommandResponse>>;
  pauseAudioPreview: (request: AudioPreviewRequest) => Promise<CommandResultEnvelope<AudioPreviewCommandResponse>>;
  stopAudioPreview: (request: AudioPreviewRequest) => Promise<CommandResultEnvelope<AudioPreviewCommandResponse>>;
  seekAudioPreview: (request: AudioPreviewRequest) => Promise<CommandResultEnvelope<AudioPreviewCommandResponse>>;
  cancelAudioPreview: (request: AudioPreviewRequest) => Promise<CommandResultEnvelope<AudioPreviewCommandResponse>>;
  getAudioPreviewStatus: (request: AudioPreviewRequest) => Promise<CommandResultEnvelope<AudioPreviewStatusResponse>>;
  listAudioOutputDevices: (request: AudioPreviewRequest) => Promise<CommandResultEnvelope<AudioOutputDeviceSummary[]>>;
  selectAudioOutputDevice: (request: AudioPreviewRequest) => Promise<CommandResultEnvelope<AudioPreviewCommandResponse>>;
  getWaveformDisplayPeaks: (request: AudioPreviewRequest) => Promise<CommandResultEnvelope<WaveformDisplayPeaksResponse>>;
  refreshWaveformStatus: (request: AudioPreviewRequest) => Promise<CommandResultEnvelope<WaveformDisplayPeaksResponse>>;
  requestProjectSessionPreviewFrame: (
    request: RequestProjectSessionPreviewFrameRequest
  ) => Promise<CommandResultEnvelope<PreviewArtifactResponse>>;
  requestProjectSessionPreviewSegment: (
    request: RequestProjectSessionPreviewSegmentRequest
  ) => Promise<CommandResultEnvelope<PreviewArtifactResponse>>;
  closeProjectSession: (request: ProjectSessionRequest) => Promise<CommandResultEnvelope<ProjectSessionClosedResponse>>;
};
type OpenMaterialFilesResponse = {
  canceled: boolean;
  filePaths: string[];
};
type ProjectBundlePickerResponse = {
  canceled: boolean;
  bundlePath: string | null;
};
type VideoEditorPlatformApi = {
  createProjectBundle: () => Promise<ProjectBundlePickerResponse>;
  openProjectBundle: () => Promise<ProjectBundlePickerResponse>;
  openMaterialFiles: () => Promise<OpenMaterialFilesResponse>;
  pathToFileUrl: (path: string) => Promise<string>;
};
type CoreCommandBuilder = (current: WorkspaceState) => CommandEnvelope;
type PreviewCommandRunner = (session: ProjectSessionClientState) => Promise<CommandResultEnvelope<PreviewArtifactResponse>>;
type PreviewCommandResultApplier = (
  current: WorkspaceState,
  result: CommandResultEnvelope<PreviewArtifactResponse>
) => WorkspaceState;
type ProjectSessionClientState = {
  sessionId: string;
  revision: number;
};
type RealtimePreviewProjectSessionSnapshotKey = {
  projectSessionId: string;
  revision: number;
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
      openProjectBundlePath?: string;
      showDeveloperDiagnostics?: boolean;
    };
    videoEditorCore: VideoEditorCoreApi;
    videoEditorPlatform?: VideoEditorPlatformApi;
  }
}

export function App(): React.ReactElement {
  const startupFixture = readWorkspaceStartupFixture();
  const showDeveloperDiagnostics = window.videoEditorAppConfig?.showDeveloperDiagnostics === true;
  const startupOpenProjectBundlePath = window.videoEditorAppConfig?.openProjectBundlePath;
  const startupProjectState = createStartupProjectState(startupFixture, startupOpenProjectBundlePath);
  const [workspace, setWorkspace] = useState<WorkspaceState>(() =>
    createInitialWorkspaceState(startupProjectState)
  );
  const [activeCategory, setActiveCategory] = useState<WorkspaceCategory>("媒体");
  const [bundlePath, setBundlePath] = useState(
    startupOpenProjectBundlePath ?? (startupFixture === "demo" ? "/tmp/phase-04-demo.veproj" : "/tmp/video-editor-workspace.veproj")
  );
  const [materialPath, setMaterialPath] = useState(startupFixture === "demo" ? "/tmp/demo-material.mp4" : "");
  const [playheadUs, setPlayheadUs] = useState(0);
  const [playbackRunning, setPlaybackRunning] = useState(false);
  const workspaceRef = useRef(workspace);
  const playheadRef = useRef(playheadUs);
  const projectSessionRef = useRef<ProjectSessionClientState | null>(null);
  const commandInFlightRef = useRef(false);
  const audioCommandInFlightRef = useRef(false);
  const runtimeProbeInFlightRef = useRef(false);
  const pendingAutoPreviewTimeRef = useRef<number | null>(null);
  const autoPreviewRetryTimerRef = useRef<number | null>(null);
  const autoPreviewRetryCountRef = useRef(0);
  const realtimePreviewSnapshotRef = useRef<RealtimePreviewProjectSessionSnapshotKey | null>(null);
  const realtimePreviewLastSeekTargetRef = useRef<number | null>(null);

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
      const [ping, version] = await Promise.all([
        window.videoEditorCore.ping(),
        window.videoEditorCore.version()
      ]);

      if (cancelled) {
        return;
      }

      if (!ping.ok || !version.ok) {
        const message =
          ping.error?.message ??
          version.error?.message ??
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

      const readyBindingStatus = {
        kind: "ready" as const,
        label: `剪辑核心已连接 ${version.data?.coreVersion ?? "0.0.0"} / 合约 ${
          version.data?.contractVersion ?? "0.0.0"
        }`
      };
      const openBundlePath = window.videoEditorAppConfig?.openProjectBundlePath?.trim();
      if ((openBundlePath === undefined || openBundlePath.length === 0) && startupFixture === undefined) {
        setWorkspace((current) => {
          const next = {
            ...current,
            bindingStatus: readyBindingStatus,
            commandError: null,
            projectState:
              current.projectState.kind === "open"
                ? current.projectState
                : {
                    kind: "entry" as const,
                    statusLabel: "先新建或打开项目，再导入素材开始剪辑。",
                    error: null
                  }
          };
          workspaceRef.current = next;
          return next;
        });
        return;
      }

      const openedProject =
        openBundlePath !== undefined && openBundlePath.length > 0
          ? await openProjectSessionForBundle(openBundlePath)
          : startupFixture !== undefined
            ? await createProjectSessionForBundle(bundlePath, startupFixture)
            : null;
      if (cancelled) {
        return;
      }

      if (openedProject !== null && (!openedProject.ok || openedProject.data === null)) {
        const message = projectSafeErrorMessage("open");
        setWorkspace((current) => ({
          ...current,
          bindingStatus: {
            kind: "error",
            label: message
          },
          projectState: {
            kind: "entry",
            statusLabel: "先新建或打开项目，再导入素材开始剪辑。",
            error: message
          },
          commandError: message
        }));
        return;
      }

      const materials =
        openedProject?.data === undefined
          ? []
          : await listProjectSessionMaterials(openedProject.data.sessionId, openedProject.data.revision);
      if (cancelled) {
        return;
      }

      setWorkspace((current) => {
        const activeBundlePath = openedProject?.data?.bundlePath ?? bundlePath;
        const base = createInitialWorkspaceState({
          kind: "open",
          bundlePath: activeBundlePath,
          statusLabel: "项目已打开",
          error: null
        });
        const next = {
          ...base,
          viewModel: openedProject?.data?.viewModel ?? base.viewModel,
          materials,
          bindingStatus: readyBindingStatus,
          commandError: openedProject?.data?.warnings.length ? commandErrorMessage(openedProject.data.warnings.join("；")) : null
        };
        workspaceRef.current = next;
        return next;
      });
      if (openedProject?.data !== undefined) {
        setBundlePath(openedProject.data.bundlePath);
      }

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

  function readWorkspaceStartupFixture(): WorkspaceStartupFixture | undefined {
    const fixture = window.videoEditorAppConfig?.workspaceFixture;
    return fixture === "demo" || fixture === "blank" ? fixture : undefined;
  }

  async function executeProjectSessionIntent<T extends ProjectSessionIntentResponse>(
    intent: ExecuteProjectIntentRequest["intent"],
    pendingCommand: string,
    applyResult: (current: WorkspaceState, result: CommandResultEnvelope<T>) => WorkspaceState
  ): Promise<CommandResultEnvelope<T> | null> {
    const session = projectSessionRef.current;
    if (session === null) {
      const message = commandErrorMessage("项目会话未就绪，请先新建或打开项目");
      setWorkspace((current) => {
        const next = {
          ...current,
          pendingCommand: null,
          commandError: message
        };
        workspaceRef.current = next;
        return next;
      });
      return null;
    }
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
      const result = await window.videoEditorCore.executeProjectIntent<T>({
        sessionId: session.sessionId,
        expectedRevision: session.revision,
        intent
      });
      if (result.ok && result.data !== null) {
        projectSessionRef.current = {
          sessionId: result.data.sessionId,
          revision: result.data.revision
        };
        setBundlePath(result.data.bundlePath);
      }
      const next = applyResult(workspaceRef.current, result);
      workspaceRef.current = next;
      setWorkspace(next);
      return result;
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      const next = {
        ...workspaceRef.current,
        pendingCommand: null,
        commandError: commandErrorMessage(message)
      };
      workspaceRef.current = next;
      setWorkspace(next);
      return null;
    } finally {
      commandInFlightRef.current = false;
    }
  }

  async function syncProjectSessionPlayhead(
    playhead: number,
    action: string,
    options: { reportBusy?: boolean } = {}
  ): Promise<boolean> {
    const reportBusy = options.reportBusy ?? true;
    const session = projectSessionRef.current;
    if (session === null) {
      if (reportBusy) {
        const next = {
          ...workspaceRef.current,
          pendingCommand: null,
          commandError: commandErrorMessage(`项目会话未就绪，无法${action}`)
        };
        workspaceRef.current = next;
        setWorkspace(next);
      }
      return false;
    }
    if (commandInFlightRef.current) {
      if (reportBusy) {
        const next = {
          ...workspaceRef.current,
          commandError: commandErrorMessage("上一个操作仍在执行，请等待剪辑核心返回")
        };
        workspaceRef.current = next;
        setWorkspace(next);
      }
      return false;
    }

    commandInFlightRef.current = true;
    try {
      const result = await window.videoEditorCore.executeProjectIntent<ProjectSessionTimelineIntentResponse>({
        sessionId: session.sessionId,
        expectedRevision: session.revision,
        intent: {
          kind: "setSessionPlayhead",
          playhead: normalizePlayheadTime(playhead)
        }
      });
      if (result.ok && result.data !== null) {
        projectSessionRef.current = {
          sessionId: result.data.sessionId,
          revision: result.data.revision
        };
        setBundlePath(result.data.bundlePath);
        return true;
      }
      if (reportBusy) {
        const next = {
          ...workspaceRef.current,
          pendingCommand: null,
          commandError: commandErrorMessage(result)
        };
        workspaceRef.current = next;
        setWorkspace(next);
      }
      return false;
    } catch (error: unknown) {
      if (reportBusy) {
        const message = error instanceof Error ? error.message : String(error);
        const next = {
          ...workspaceRef.current,
          pendingCommand: null,
          commandError: commandErrorMessage(message)
        };
        workspaceRef.current = next;
        setWorkspace(next);
      }
      return false;
    } finally {
      commandInFlightRef.current = false;
    }
  }

  async function createProjectSessionForBundle(
    bundlePath: string,
    fixture?: WorkspaceStartupFixture
  ): Promise<CommandResultEnvelope<ProjectSessionOpenResponse>> {
    await closeCurrentProjectSession();
    const result = await window.videoEditorCore.createProjectSession({
      bundlePath,
      draftName: projectNameFromBundlePath(bundlePath),
      ...(fixture === "demo" ? { fixture: "demo" as const } : {})
    });
    if (result.ok && result.data !== null) {
      projectSessionRef.current = {
        sessionId: result.data.sessionId,
        revision: result.data.revision
      };
      setBundlePath(result.data.bundlePath);
    } else {
      projectSessionRef.current = null;
    }
    return result;
  }

  async function openProjectSessionForBundle(bundlePath: string): Promise<CommandResultEnvelope<ProjectSessionOpenResponse>> {
    await closeCurrentProjectSession();
    const result = await window.videoEditorCore.openProjectSession({ bundlePath });
    if (result.ok && result.data !== null) {
      projectSessionRef.current = {
        sessionId: result.data.sessionId,
        revision: result.data.revision
      };
      setBundlePath(result.data.bundlePath);
    } else {
      projectSessionRef.current = null;
    }
    return result;
  }

  async function closeCurrentProjectSession(): Promise<void> {
    const session = projectSessionRef.current;
    if (session === null) {
      return;
    }
    projectSessionRef.current = null;
    await window.videoEditorCore.closeProjectSession({ sessionId: session.sessionId }).catch(() => undefined);
  }

  function currentProjectSessionReadRequest(action: string): { sessionId: string; expectedRevision: number } | null {
    const session = projectSessionRef.current;
    if (session !== null) {
      return {
        sessionId: session.sessionId,
        expectedRevision: session.revision
      };
    }

    setWorkspace((current) => ({
      ...current,
      pendingCommand: null,
      commandError: commandErrorMessage(`项目会话未就绪，无法${action}`)
    }));
    return null;
  }

  function currentProjectSessionAudioRequest(action: string): { projectSessionId: string; expectedRevision: number } | null {
    const request = currentProjectSessionReadRequest(action);
    return request === null
      ? null
      : {
          projectSessionId: request.sessionId,
          expectedRevision: request.expectedRevision
        };
  }

  async function listProjectSessionMaterials(sessionId: string, expectedRevision: number) {
    const result = await window.videoEditorCore.listProjectSessionMaterials({
      sessionId,
      expectedRevision
    });
    if (!result.ok || result.data === null) {
      throw new Error(result.error?.message ?? "素材列表读取失败");
    }
    return result.data.materials;
  }

  async function listProjectSessionMissingMaterials(sessionId: string, expectedRevision: number) {
    const result = await window.videoEditorCore.listProjectSessionMissingMaterials({
      sessionId,
      expectedRevision
    });
    if (!result.ok || result.data === null) {
      throw new Error(result.error?.message ?? "丢失素材检查失败");
    }
    return result.data.diagnostics;
  }

  async function executeProjectTimelineIntent(
    intent: ExecuteProjectIntentRequest["intent"],
    pendingCommand: string
  ): Promise<CommandResultEnvelope<ProjectSessionTimelineIntentResponse> | null> {
    const result = await executeProjectSessionIntent<ProjectSessionTimelineIntentResponse>(
      intent,
      pendingCommand,
      (current, result) => applyProjectSessionTimelineResult(current, result, intent.kind)
    );

    if (
      result !== null &&
      result.ok &&
      result.data !== null &&
      (intent.kind === "addTimelineSegmentIntent" ||
        intent.kind === "addTextSegmentIntent" ||
        intent.kind === "addAudioSegmentIntent")
    ) {
      const previewTarget = selectedSegmentStart(result.data);
      if (previewTarget !== null) {
        queueAutoPreviewFrame(previewTarget);
      }
    }

    return result;
  }

  function applyProjectSessionTimelineResult(
    current: WorkspaceState,
    result: CommandResultEnvelope<ProjectSessionTimelineIntentResponse>,
    intentKind: ExecuteProjectIntentRequest["intent"]["kind"]
  ): WorkspaceState {
    const errorMessage = result.ok && result.data !== null ? null : commandErrorMessage(result);

    const next = {
      ...current,
      viewModel: result.ok && result.data !== null ? result.data.viewModel : current.viewModel,
      pendingCommand: null,
      commandError: errorMessage
    };

    if (
      result.ok &&
      result.data !== null &&
      (intentKind === "updateSelectedSegmentVisual" || intentKind === "setSelectedTrackVisibility")
    ) {
      return {
        ...next,
        preview: clearDerivedPreviewState(current.preview, VISUAL_DERIVED_STATE_COPY),
        export: clearDerivedExportState(current.export, VISUAL_DERIVED_STATE_COPY.exportLogSummary)
      };
    }

    if (
      result.ok &&
      result.data !== null &&
      (intentKind === "addTextSegmentIntent" || intentKind === "editSelectedText" || intentKind === "importSubtitleSrtIntent")
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
      (intentKind === "setSelectedSegmentKeyframe" || intentKind === "removeSelectedSegmentKeyframe")
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
      (intentKind === "setSelectedSegmentVolume" ||
        intentKind === "updateSelectedSegmentAudio" ||
        intentKind === "setSelectedTrackMute" ||
        intentKind === "addAudioSegmentIntent")
    ) {
      return {
        ...next,
        preview: clearDerivedPreviewState(current.preview, AUDIO_DERIVED_STATE_COPY),
        export: clearDerivedExportState(current.export, AUDIO_DERIVED_STATE_COPY.exportLogSummary)
      };
    }

    if (result.ok && result.data !== null && intentKind === "updateDraftCanvasConfig") {
      return {
        ...next,
        preview: clearDerivedPreviewState(current.preview),
        export: clearDerivedExportState(current.export)
      };
    }

    return next;
  }

  async function executePreviewCommand(
    runCommand: PreviewCommandRunner,
    pendingCommand: string,
    applyResult: PreviewCommandResultApplier
  ): Promise<void> {
    const session = projectSessionRef.current;
    if (session === null) {
      setWorkspace((current) => {
        const message = commandErrorMessage("项目会话未就绪，请先新建或打开项目");
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
      const result = await runCommand(session);
      setWorkspace((current) => {
        const next = applyResult(current, result);
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

  async function executeExportJobControl(
    runCommand: (jobId: string) => Promise<CommandResultEnvelope<ExportJobStatusResponse>>,
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
      const jobId = workspaceRef.current.export.jobId;
      if (jobId === null) {
        throw new Error("请先开始导出");
      }
      const result = await runCommand(jobId);
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

  async function executeProjectSessionStartExport(applyResult: ExportCommandResultApplier): Promise<void> {
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

    const projectSession = projectSessionRef.current;
    if (projectSession === null) {
      setWorkspace((current) => {
        const message = commandErrorMessage("项目会话尚未建立，无法开始导出");
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

    commandInFlightRef.current = true;
    setWorkspace((current) => {
      const next = {
        ...current,
        pendingCommand: "开始导出",
        commandError: null,
        export: {
          ...current.export,
          error: null,
          logSummary: "正在开始导出"
        }
      };
      workspaceRef.current = next;
      return next;
    });

    try {
      const current = workspaceRef.current;
      const result = await window.videoEditorCore.startProjectSessionExport({
        sessionId: projectSession.sessionId,
        expectedRevision: projectSession.revision,
        outputPath: current.export.outputPath,
        preset: current.export.preset
      });
      setWorkspace((current) => {
        const next = applyResult(current, result);
        workspaceRef.current = next;
        return next;
      });
    } catch (error: unknown) {
      const message = exportCommandErrorMessage(error instanceof Error ? error.message : String(error), "开始导出");
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
          commandError: result.ok ? current.commandError : null
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
          commandError: null
        };
        workspaceRef.current = next;
        return next;
      });
    } finally {
      runtimeProbeInFlightRef.current = false;
    }
  }

  async function executeAudioCommand<T>(
    runCommand: (current: WorkspaceState) => Promise<CommandResultEnvelope<T>>,
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
      const result = await runCommand(workspaceRef.current);
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

    const projectSession = currentProjectSessionAudioRequest("创建音频预览");
    if (projectSession === null) {
      return null;
    }

    const result = await executeAudioCommand<AudioPreviewCommandResponse>(
      () =>
        window.videoEditorCore.createAudioPreviewSession({
          projectSessionId: projectSession.projectSessionId,
          expectedRevision: projectSession.expectedRevision,
          targetTime: playheadRef.current
        }),
      "创建音频预览",
      applyAudioPreviewCommandResult
    );

    return result?.ok === true && result.data !== null ? result.data.sessionId : null;
  }

  async function refreshAudioDevices(): Promise<void> {
    const projectSession = currentProjectSessionAudioRequest("读取输出设备");
    if (projectSession === null) {
      return;
    }
    await executeAudioCommand<AudioOutputDeviceSummary[]>(
      () =>
        window.videoEditorCore.listAudioOutputDevices({
          projectSessionId: projectSession.projectSessionId,
          expectedRevision: projectSession.expectedRevision
        }),
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
    const projectSession = currentProjectSessionAudioRequest("读取音频状态");
    if (projectSession === null) {
      return;
    }

    await executeAudioCommand<AudioPreviewStatusResponse>(
      () =>
        window.videoEditorCore.getAudioPreviewStatus({
          projectSessionId: projectSession.projectSessionId,
          expectedRevision: projectSession.expectedRevision,
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
    const projectSession = currentProjectSessionAudioRequest("读取波形");
    if (projectSession === null) {
      return;
    }

    await executeAudioCommand<WaveformDisplayPeaksResponse>(
      () =>
        window.videoEditorCore.getWaveformDisplayPeaks({
          projectSessionId: projectSession.projectSessionId,
          expectedRevision: projectSession.expectedRevision,
          materialId,
          maxPeakBins: 16
        }),
      "读取波形",
      applyWaveformResult
    );
    await executeAudioCommand<WaveformDisplayPeaksResponse>(
      () =>
        window.videoEditorCore.refreshWaveformStatus({
          projectSessionId: projectSession.projectSessionId,
          expectedRevision: projectSession.expectedRevision,
          materialId,
          maxPeakBins: 16
        }),
      "刷新波形",
      applyWaveformResult
    );
  }

  async function importMaterialPath(path: string): Promise<void> {
    await executeProjectSessionIntent<ProjectSessionImportMaterialResponse>(
      {
        kind: "importMaterial",
        materialPath: path
      },
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
          viewModel: result.data.viewModel,
          materials: result.data.materials,
          materialDiagnostics: result.data.diagnostic === null || result.data.diagnostic === undefined ? [] : [result.data.diagnostic],
          pendingCommand: null,
          commandError: null
        };
      }
    );
  }

  async function handleCreateProject(): Promise<void> {
    if (commandInFlightRef.current) {
      setProjectEntryError("create", "上一个操作仍在执行，请等待剪辑核心返回");
      return;
    }

    const platform = window.videoEditorPlatform;
    if (platform === undefined) {
      setProjectEntryError("create", "当前环境无法打开系统项目窗口，请稍后重试。");
      return;
    }

    try {
      const picked = await platform.createProjectBundle();
      const nextBundlePath = picked.bundlePath?.trim() ?? "";
      if (picked.canceled || nextBundlePath.length === 0) {
        return;
      }

      commandInFlightRef.current = true;
      setWorkspace((current) => {
        const next = {
          ...current,
          pendingCommand: "新建项目",
          commandError: null,
          projectState: {
            kind: "opening" as const,
            action: "create" as const,
            statusLabel: "正在新建项目",
            error: null
          }
        };
        workspaceRef.current = next;
        return next;
      });

      const result = await createProjectSessionForBundle(nextBundlePath);
      if (!result.ok || result.data === null) {
        setProjectEntryError("create", projectSafeErrorMessage("create"));
        return;
      }

      const materials = await listProjectSessionMaterials(result.data.sessionId, result.data.revision);
      openWorkspaceFromSession(result.data.viewModel, materials, result.data.bundlePath, "项目已新建");
      void handleGetArtifactStatus();
      void handleProbeRuntimeCapabilities();
    } catch {
      setProjectEntryError("create", projectSafeErrorMessage("create"));
    } finally {
      commandInFlightRef.current = false;
    }
  }

  async function handleOpenProject(): Promise<void> {
    if (commandInFlightRef.current) {
      setProjectEntryError("open", "上一个操作仍在执行，请等待剪辑核心返回");
      return;
    }

    const platform = window.videoEditorPlatform;
    if (platform === undefined) {
      setProjectEntryError("open", "当前环境无法打开系统项目窗口，请稍后重试。");
      return;
    }

    try {
      const picked = await platform.openProjectBundle();
      const nextBundlePath = picked.bundlePath?.trim() ?? "";
      if (picked.canceled || nextBundlePath.length === 0) {
        return;
      }

      commandInFlightRef.current = true;
      setWorkspace((current) => {
        const next = {
          ...current,
          pendingCommand: "打开项目",
          commandError: null,
          projectState: {
            kind: "opening" as const,
            action: "open" as const,
            statusLabel: "正在打开项目",
            error: null
          }
        };
        workspaceRef.current = next;
        return next;
      });

      const result = await openProjectSessionForBundle(nextBundlePath);
      if (!result.ok || result.data === null) {
        setProjectEntryError("open", projectSafeErrorMessage("open"));
        return;
      }

      const materials = await listProjectSessionMaterials(result.data.sessionId, result.data.revision);
      openWorkspaceFromSession(result.data.viewModel, materials, result.data.bundlePath, "项目已打开");
      setWorkspace((current) => ({
        ...current,
        commandError: result.data?.warnings.length ? commandErrorMessage(result.data.warnings.join("；")) : null
      }));
      void handleGetArtifactStatus();
      void handleProbeRuntimeCapabilities();
      window.setTimeout(() => {
        void refreshWaveformDisplay();
      }, 250);
    } catch {
      setProjectEntryError("open", projectSafeErrorMessage("open"));
    } finally {
      commandInFlightRef.current = false;
    }
  }

  function openWorkspaceFromSession(
    viewModel: ProjectSessionOpenResponse["viewModel"],
    materials: ProjectSessionMaterialsResponse["materials"],
    nextBundlePath: string,
    statusLabel: string
  ): void {
    const next = {
      ...createInitialWorkspaceState({
        kind: "open" as const,
        bundlePath: nextBundlePath,
        statusLabel,
        error: null
      }),
      viewModel,
      materials,
      bindingStatus: workspaceRef.current.bindingStatus,
      pendingCommand: null,
      commandError: null
    };
    workspaceRef.current = next;
    setWorkspace(next);
    setBundlePath(nextBundlePath);
  }

  function setProjectEntryError(_action: "create" | "open", message: string): void {
    setWorkspace((current) => {
      const next = {
        ...current,
        pendingCommand: null,
        commandError: message,
        projectState: {
          kind: "entry" as const,
          statusLabel: "先新建或打开项目，再导入素材开始剪辑。",
          error: message
        }
      };
      workspaceRef.current = next;
      return next;
    });
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
      const request = currentProjectSessionReadRequest("刷新素材");
      if (request === null) {
        return;
      }
      const materials = await listProjectSessionMaterials(request.sessionId, request.expectedRevision);
      setWorkspace((current) => ({
        ...current,
        materials,
        pendingCommand: null,
        commandError: null
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
      const request = currentProjectSessionReadRequest("检查丢失素材");
      if (request === null) {
        return;
      }
      const diagnostics = await listProjectSessionMissingMaterials(request.sessionId, request.expectedRevision);
      setWorkspace((current) => ({
        ...current,
        materialDiagnostics: diagnostics,
        pendingCommand: null,
        commandError: null
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

  function handleAddTextSegment(content: string): void {
    void (async () => {
      const playhead = normalizePlayheadTime(playheadUs);
      const synced = await syncProjectSessionPlayhead(playhead, "定位文字播放头");
      if (!synced) {
        return;
      }
      await executeProjectTimelineIntent(
        {
          kind: "addTextSegmentIntent",
          content
        },
        "添加文字"
      );
    })();
  }

  function handleImportSubtitleSrt(srtContent: string): void {
    void (async () => {
      const playhead = normalizePlayheadTime(playheadUs);
      const synced = await syncProjectSessionPlayhead(playhead, "定位字幕播放头");
      if (!synced) {
        return;
      }
      await executeProjectTimelineIntent(
        {
          kind: "importSubtitleSrtIntent",
          srtContent
        },
        "导入字幕"
      );
    })();
  }

  function handleAddAudioSegment(materialId: string): void {
    void (async () => {
      const playhead = normalizePlayheadTime(playheadUs);
      const synced = await syncProjectSessionPlayhead(playhead, "定位音频播放头");
      if (!synced) {
        return;
      }
      await executeProjectTimelineIntent(
        {
          kind: "addAudioSegmentIntent",
          materialId: materialId.length > 0 ? materialId : null
        },
        "添加音频"
      );
    })();
  }

  function handleSetSelectedSegmentVolume(levelMillis: number): void {
    const volume: SegmentVolume = {
      levelMillis: Math.max(0, Math.min(4000, Math.round(levelMillis)))
    };

    void executeProjectTimelineIntent(
      {
        kind: "setSelectedSegmentVolume",
        volume
      },
      "调整音量"
    );
  }

  function handleEditSelectedText(text: TextSegment): void {
    void executeProjectTimelineIntent(
      {
        kind: "editSelectedText",
        text
      },
      "应用文字"
    );
  }

  function handleSelectTimelineTrack(itemHandle: string): void {
    void executeProjectTimelineIntent(
      {
        kind: "selectTimelineItemIntent",
        itemHandle
      },
      "选择轨道"
    );
  }

  async function selectTrackThenExecuteSelectedTrackIntent(
    itemHandle: string,
    intent: Extract<
      ExecuteProjectIntentRequest["intent"],
      { kind: "renameSelectedTrack" | "setSelectedTrackLock" | "setSelectedTrackVisibility" | "setSelectedTrackMute" }
    >,
    pendingCommand: string
  ): Promise<void> {
    if (itemHandle.length === 0) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("请先选择一个轨道")
      }));
      return;
    }

    const selected = await executeProjectTimelineIntent(
      {
        kind: "selectTimelineItemIntent",
        itemHandle
      },
      "选择轨道"
    );
    if (selected === null || !selected.ok) {
      return;
    }

    await executeProjectTimelineIntent(intent, pendingCommand);
  }

  function handleSetSelectedTrackMute(itemHandle: string, muted: boolean): void {
    void selectTrackThenExecuteSelectedTrackIntent(
      itemHandle,
      {
        kind: "setSelectedTrackMute",
        muted
      },
      "切换轨道静音"
    );
  }

  function handleAddTimelineTrack(trackKind: TrackKind): void {
    void executeProjectTimelineIntent(
      {
        kind: "addTrackIntent",
        trackKind
      },
      "添加轨道"
    );
  }

  function handleRenameTimelineTrack(itemHandle: string, name: string): void {
    const trimmedName = name.trim();
    if (trimmedName.length === 0) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("轨道名称不能为空")
      }));
      return;
    }
    void selectTrackThenExecuteSelectedTrackIntent(
      itemHandle,
      {
        kind: "renameSelectedTrack",
        name: trimmedName
      },
      "重命名轨道"
    );
  }

  function handleSetTimelineTrackLock(itemHandle: string, locked: boolean): void {
    void selectTrackThenExecuteSelectedTrackIntent(
      itemHandle,
      {
        kind: "setSelectedTrackLock",
        locked
      },
      "切换轨道锁定"
    );
  }

  function handleSetTimelineTrackVisibility(itemHandle: string, visible: boolean): void {
    void selectTrackThenExecuteSelectedTrackIntent(
      itemHandle,
      {
        kind: "setSelectedTrackVisibility",
        visible
      },
      "切换轨道显示"
    );
  }

  function handleUpdateSelectedSegmentAudio(options: {
    gainMillis: number;
    panBalanceMillis: number;
    fadeInDuration: number;
    fadeOutDuration: number;
  }): void {
    void executeProjectTimelineIntent(
      {
        kind: "updateSelectedSegmentAudio",
        gainMillis: Math.max(0, Math.min(4000, Math.round(options.gainMillis))),
        panBalanceMillis: Math.max(-1000, Math.min(1000, Math.round(options.panBalanceMillis))),
        fadeInDuration: { duration: Math.max(0, Math.round(options.fadeInDuration)) },
        fadeOutDuration: { duration: Math.max(0, Math.round(options.fadeOutDuration)) },
        effectSlots: []
      },
      "应用音频"
    );
  }

  function handleSelectTimelineSegment(itemHandle: string): void {
    void executeProjectTimelineIntent(
      {
        kind: "selectTimelineItemIntent",
        itemHandle
      },
      "选择片段"
    );
  }

  function handleAddTimelineSegment(materialId: string): void {
    void (async () => {
      const playhead = normalizePlayheadTime(playheadUs);
      const synced = await syncProjectSessionPlayhead(playhead, "定位添加播放头");
      if (!synced) {
        return;
      }
      await executeProjectTimelineIntent(
        {
          kind: "addTimelineSegmentIntent",
          materialId
        },
        "添加片段"
      );
    })();
  }

  function handleMoveSelectedSegment(startAt: number): void {
    void executeProjectTimelineIntent(
      {
        kind: "moveSelectedSegmentIntent",
        startAt: normalizePlayheadTime(startAt)
      },
      "移动片段"
    );
  }

  function handleSplitSelectedSegment(): void {
    void (async () => {
      const playhead = normalizePlayheadTime(playheadUs);
      const synced = await syncProjectSessionPlayhead(playhead, "定位分割播放头");
      if (!synced) {
        return;
      }
      await executeProjectTimelineIntent(
        {
          kind: "splitSelectedSegmentIntent"
        },
        "分割片段"
      );
    })();
  }

  function handleTrimSelectedSegment(direction: "left" | "right", trimAt: number): void {
    void executeProjectTimelineIntent(
      {
        kind: "trimSelectedSegmentIntent",
        direction,
        trimAt: normalizePlayheadTime(trimAt)
      },
      direction === "left" ? "左侧裁剪" : "右侧裁剪"
    );
  }

  function handleDeleteSelectedSegment(): void {
    const selected = workspaceRef.current.viewModel.selectedSegment;

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

    void executeProjectTimelineIntent(
      {
        kind: "deleteSelectedSegment"
      },
      "删除片段"
    );
  }

  function handleUndoTimelineEdit(): void {
    void executeProjectTimelineIntent({ kind: "undoTimelineEdit" }, "撤销");
  }

  function handleRedoTimelineEdit(): void {
    void executeProjectTimelineIntent({ kind: "redoTimelineEdit" }, "重做");
  }

  function handleUpdateDraftCanvasConfig(canvasConfig: DraftCanvasConfig): void {
    void (async () => {
      await executeProjectTimelineIntent(
        {
          kind: "updateDraftCanvasConfig",
          canvasConfig
        },
        "应用草稿参数"
      );
    })();
  }

  function handleUpdateSelectedSegmentVisual(visual: SegmentVisual): void {
    void (async () => {
      await executeProjectTimelineIntent(
        {
          kind: "updateSelectedSegmentVisual",
          visual
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
    void (async () => {
      const playhead = normalizePlayheadTime(playheadUs);
      const synced = await syncProjectSessionPlayhead(playhead, "定位关键帧播放头");
      if (!synced) {
        return;
      }
      await executeProjectTimelineIntent(
        {
          kind: "setSelectedSegmentKeyframe",
          property,
          interpolation,
          easing
        },
        "设置关键帧"
      );
    })();
  }

  function handleRemoveSelectedSegmentKeyframe(property: KeyframeProperty): void {
    void (async () => {
      const playhead = normalizePlayheadTime(playheadUs);
      const synced = await syncProjectSessionPlayhead(playhead, "定位关键帧播放头");
      if (!synced) {
        return;
      }
      await executeProjectTimelineIntent(
        {
          kind: "removeSelectedSegmentKeyframe",
          property
        },
        "删除关键帧"
      );
    })();
  }

  function handleSeekPlayhead(value: number): void {
    const targetTime = normalizePlayheadTime(value);
    setPlaybackRunning(false);
    setPlayheadUs(targetTime);
    void syncProjectSessionPlayhead(targetTime, "定位播放头", { reportBusy: false });
    void seekRealtimePreviewHost(targetTime);
    void handleSeekAudioPreview(targetTime);
  }

  function handleTogglePlayback(): void {
    void (async () => {
      if (playbackRunning) {
        await pauseRealtimePreviewHost();
        await handlePauseAudioPreview();
        setPlaybackRunning(false);
        return;
      }

      const sequenceDurationUs = workspaceRef.current.viewModel.project.sequenceDuration;
      if (sequenceDurationUs <= 0) {
        return;
      }

      if (playheadRef.current >= sequenceDurationUs) {
        setPlayheadUs(0);
        playheadRef.current = 0;
      }

      const snapshotReady = await updateRealtimePreviewProjectSessionSnapshot();
      if (!snapshotReady) {
        return;
      }
      const seekReady = await seekRealtimePreviewHost(playheadRef.current);
      if (!seekReady) {
        return;
      }

      const playbackReady = await playRealtimePreviewHost();
      if (!playbackReady) {
        setPlaybackRunning(false);
        return;
      }
      setPlaybackRunning(true);
      void handlePlayAudioPreview();
    })();
  }

  function handleStopPlayback(): void {
    void handleStopAudioPreview();
    void stopRealtimePreviewHost();
    setPlaybackRunning(false);
    setPlayheadUs(0);
    playheadRef.current = 0;
  }

  const handleRealtimePreviewHostStateChange = useCallback((hostState: RealtimePreviewHostState): void => {
    if (!hostState.productReady || hostState.telemetry === null) {
      return;
    }

    const sequenceDurationUs = workspaceRef.current.viewModel.project.sequenceDuration;
    if (sequenceDurationUs <= 0) {
      return;
    }

    const rawPresentedTime = Math.min(
      sequenceDurationUs,
      Math.max(
        hostState.telemetry.targetTimeMicroseconds,
        hostState.contentEvidence?.targetTimeMicroseconds ?? 0
      )
    );
    const frameAlignedAtEnd = isFrameAlignedSequenceEnd(
      workspaceRef.current,
      rawPresentedTime,
      sequenceDurationUs
    );
    const nextPlayhead = frameAlignedAtEnd ? sequenceDurationUs : normalizePlayheadTime(rawPresentedTime);
    setPlayheadUs(nextPlayhead);
    playheadRef.current = nextPlayhead;

    if (frameAlignedAtEnd && playbackRunning) {
      void handlePauseAudioPreview();
      void pauseRealtimePreviewHost();
      setPlaybackRunning(false);
    }
  }, [playbackRunning]);

  async function updateRealtimePreviewProjectSessionSnapshot(): Promise<boolean> {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return applyRealtimePreviewHostError("预览画面暂不可用");
    }
    const projectSession = projectSessionRef.current;
    if (projectSession === null) {
      return applyRealtimePreviewHostError("项目会话尚未就绪");
    }

    const currentSnapshot = realtimePreviewSnapshotRef.current;
    if (
      currentSnapshot?.projectSessionId === projectSession.sessionId &&
      currentSnapshot.revision === projectSession.revision
    ) {
      return true;
    }

    try {
      const ok = applyRealtimePreviewHostState(
        await bridge.updateProjectSessionSnapshot(projectSession.sessionId, projectSession.revision)
      );
      if (ok) {
        realtimePreviewSnapshotRef.current = {
          projectSessionId: projectSession.sessionId,
          revision: projectSession.revision
        };
        realtimePreviewLastSeekTargetRef.current = null;
      }
      return ok;
    } catch (error: unknown) {
      return applyRealtimePreviewHostError(
        showDeveloperDiagnostics ? (error instanceof Error ? error.message : String(error)) : "预览画面暂不可用"
      );
    }
  }

  async function seekRealtimePreviewHost(targetTime: number): Promise<boolean> {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return applyRealtimePreviewHostError("预览画面暂不可用");
    }

    const sanitizedTargetTime = Math.max(0, Math.round(targetTime));
    const currentSnapshot = realtimePreviewSnapshotRef.current;
    const currentProjectSession = projectSessionRef.current;
    if (
      currentSnapshot !== null &&
      currentProjectSession !== null &&
      currentSnapshot.projectSessionId === currentProjectSession.sessionId &&
      currentSnapshot.revision === currentProjectSession.revision &&
      realtimePreviewLastSeekTargetRef.current === sanitizedTargetTime
    ) {
      return true;
    }

    try {
      const ok = applyRealtimePreviewHostState(await bridge.seek(sanitizedTargetTime));
      if (ok) {
        realtimePreviewLastSeekTargetRef.current = sanitizedTargetTime;
      }
      return ok;
    } catch (error: unknown) {
      return applyRealtimePreviewHostError(
        showDeveloperDiagnostics ? (error instanceof Error ? error.message : String(error)) : "预览画面暂不可用"
      );
    }
  }

  async function playRealtimePreviewHost(): Promise<boolean> {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return applyRealtimePreviewHostError("预览画面暂不可用");
    }

    try {
      return applyRealtimePreviewHostState(await bridge.play());
    } catch (error: unknown) {
      return applyRealtimePreviewHostError(
        showDeveloperDiagnostics ? (error instanceof Error ? error.message : String(error)) : "预览画面暂不可用"
      );
    }
  }

  async function pauseRealtimePreviewHost(): Promise<boolean> {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return applyRealtimePreviewHostError("预览画面暂不可用");
    }

    try {
      return applyRealtimePreviewHostState(await bridge.pause());
    } catch (error: unknown) {
      return applyRealtimePreviewHostError(
        showDeveloperDiagnostics ? (error instanceof Error ? error.message : String(error)) : "预览画面暂不可用"
      );
    }
  }

  async function stopRealtimePreviewHost(): Promise<boolean> {
    const bridge = window.videoEditorRealtimePreviewHost;
    if (bridge === undefined) {
      return applyRealtimePreviewHostError("预览画面暂不可用");
    }

    try {
      const ok = applyRealtimePreviewHostState(await bridge.stop());
      if (ok) {
        realtimePreviewLastSeekTargetRef.current = 0;
      }
      return ok;
    } catch (error: unknown) {
      return applyRealtimePreviewHostError(
        showDeveloperDiagnostics ? (error instanceof Error ? error.message : String(error)) : "预览画面暂不可用"
      );
    }
  }

  function applyRealtimePreviewHostState(hostState: RealtimePreviewHostState): boolean {
    if (hostState.ok) {
      return true;
    }
    return applyRealtimePreviewHostError(
      showDeveloperDiagnostics ? hostState.fallbackLabel ?? hostState.statusLabel : "预览画面暂不可用"
    );
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
    const projectSession = currentProjectSessionAudioRequest("播放音频");
    if (projectSession === null) {
      return;
    }
    await executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        window.videoEditorCore.playAudioPreview({
          projectSessionId: projectSession.projectSessionId,
          expectedRevision: projectSession.expectedRevision,
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
    const projectSession = currentProjectSessionAudioRequest("暂停音频");
    if (projectSession === null) {
      return;
    }

    await executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        window.videoEditorCore.pauseAudioPreview({
          projectSessionId: projectSession.projectSessionId,
          expectedRevision: projectSession.expectedRevision,
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
    const projectSession = currentProjectSessionAudioRequest("停止音频");
    if (projectSession === null) {
      return;
    }

    await executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        window.videoEditorCore.stopAudioPreview({
          projectSessionId: projectSession.projectSessionId,
          expectedRevision: projectSession.expectedRevision,
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
    const projectSession = currentProjectSessionAudioRequest("定位音频");
    if (projectSession === null) {
      return;
    }

    await executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        window.videoEditorCore.seekAudioPreview({
          projectSessionId: projectSession.projectSessionId,
          expectedRevision: projectSession.expectedRevision,
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
    const projectSession = currentProjectSessionAudioRequest("取消音频请求");
    if (projectSession === null) {
      return;
    }

    await executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        window.videoEditorCore.cancelAudioPreview({
          projectSessionId: projectSession.projectSessionId,
          expectedRevision: projectSession.expectedRevision,
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
    const projectSession = currentProjectSessionAudioRequest("选择输出设备");
    const sessionId = workspaceRef.current.audioPreview.sessionId;
    if (projectSession === null || sessionId === null) {
      return;
    }
    void executeAudioCommand<AudioPreviewCommandResponse>(
      (current) =>
        window.videoEditorCore.selectAudioOutputDevice({
          projectSessionId: projectSession.projectSessionId,
          expectedRevision: projectSession.expectedRevision,
          sessionId,
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
    if (!showDeveloperDiagnostics) {
      void (async () => {
        const snapshotReady = await updateRealtimePreviewProjectSessionSnapshot();
        if (snapshotReady) {
          await seekRealtimePreviewHost(targetTime);
        }
      })();
      return;
    }

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
      (session) =>
        window.videoEditorCore.requestProjectSessionPreviewFrame({
          sessionId: session.sessionId,
          expectedRevision: session.revision,
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

    const previewRange = {
      start: Math.max(0, Math.round(playheadUs)),
      duration: PREVIEW_SEGMENT_DURATION_US
    };

    void executePreviewCommand(
      (session) =>
        window.videoEditorCore.requestProjectSessionPreviewSegment({
          sessionId: session.sessionId,
          expectedRevision: session.revision,
          targetTimerange: previewRange
        }),
      "生成预览片段",
      (current, result) => {
        const rangeLabel = `${formatMicroseconds(previewRange.start)} - ${formatMicroseconds(
          previewRange.start + previewRange.duration
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
              lastRequestedPlayhead: previewRange.start,
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
            lastRequestedPlayhead: previewRange.start,
            lastRequestedRangeLabel: rangeLabel
          }
        };
      }
    );
  }

  function handleExportOutputPathChange(value: string): void {
    setWorkspace((current) => {
      const next = {
        ...current,
        export: {
          ...current.export,
          outputPath: value
        }
      };
      workspaceRef.current = next;
      return next;
    });
  }

  function handleExportPresetChange(value: ExportPreset): void {
    setWorkspace((current) => {
      const next = {
        ...current,
        export: {
          ...current.export,
          preset: value
        }
      };
      workspaceRef.current = next;
      return next;
    });
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

    void executeProjectSessionStartExport((current, result) => applyExportCommandResult(current, result, "开始导出"));
  }

  function handleRefreshExportStatus(): void {
    void executeExportJobControl(
      (jobId) => window.videoEditorCore.getExportJobStatus({ jobId }),
      "查询导出状态",
      (current, result) => applyExportCommandResult(current, result, "查询导出状态")
    );
  }

  function handleCancelExport(): void {
    void executeExportJobControl(
      (jobId) => window.videoEditorCore.cancelExport({ jobId }),
      "取消导出",
      (current, result) => applyExportCommandResult(current, result, "取消导出")
    );
  }

  if (workspace.projectState.kind !== "open") {
    return (
      <ProjectEntry
        state={workspace.projectState}
        bindingStatusLabel={workspace.bindingStatus.label}
        pending={workspace.pendingCommand !== null}
        onCreateProject={handleCreateProject}
        onOpenProject={handleOpenProject}
      />
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
      onRealtimePreviewHostStateChange={handleRealtimePreviewHostStateChange}
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
      onSelectTimelineTrack={handleSelectTimelineTrack}
      onAddTimelineSegment={handleAddTimelineSegment}
      onAddTimelineTrack={handleAddTimelineTrack}
      onRenameTimelineTrack={handleRenameTimelineTrack}
      onSetTimelineTrackLock={handleSetTimelineTrackLock}
      onSetTimelineTrackVisibility={handleSetTimelineTrackVisibility}
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

type ProjectEntryProps = {
  state: ProjectEntryState;
  bindingStatusLabel: string;
  pending: boolean;
  onCreateProject: () => void;
  onOpenProject: () => void;
};

function ProjectEntry({
  state,
  bindingStatusLabel,
  pending,
  onCreateProject,
  onOpenProject
}: ProjectEntryProps): React.ReactElement {
  const busy = state.kind === "opening" || pending;

  return (
    <main className="project-entry" aria-label="项目入口">
      <section className="project-entry-panel" aria-label="项目操作">
        <div className="project-entry-mark">
          <h1>视频剪辑</h1>
          <p>{state.statusLabel}</p>
        </div>
        <div className="project-entry-actions">
          <button type="button" className="primary-action project-entry-action" onClick={onCreateProject} disabled={busy}>
            新建项目
          </button>
          <button type="button" className="secondary-action project-entry-action" onClick={onOpenProject} disabled={busy}>
            打开项目
          </button>
        </div>
        <p className="project-entry-empty">先新建或打开项目，再导入素材开始剪辑。</p>
        <p className="project-entry-status" aria-label="项目入口状态">
          {busy ? state.statusLabel : bindingStatusLabel}
        </p>
        {state.error !== null ? (
          <p className="project-entry-error" role="alert">
            {state.error}
          </p>
        ) : null}
      </section>
    </main>
  );
}

function createStartupProjectState(
  fixture: WorkspaceStartupFixture | undefined,
  openProjectBundlePath: string | undefined
): ProjectEntryState {
  if (fixture !== undefined) {
    return {
      kind: "open",
      bundlePath: fixture === "demo" ? "/tmp/phase-04-demo.veproj" : "/tmp/video-editor-workspace.veproj",
      statusLabel: "项目已打开",
      error: null
    };
  }

  if (openProjectBundlePath !== undefined && openProjectBundlePath.trim().length > 0) {
    return {
      kind: "opening",
      action: "open",
      statusLabel: "正在打开项目",
      error: null
    };
  }

  return {
    kind: "entry",
    statusLabel: "先新建或打开项目，再导入素材开始剪辑。",
    error: null
  };
}

function projectSafeErrorMessage(action: "create" | "open"): string {
  return action === "create"
    ? "项目新建失败，请确认保存位置可用后重试。"
    : "项目打开失败，请确认草稿包完整后重试。";
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

function selectedSegmentStart(response: ProjectSessionTimelineIntentResponse): number | null {
  return response.viewModel.selectedSegment?.targetTimerange.start ?? null;
}

function isFrameAlignedSequenceEnd(
  workspace: WorkspaceState,
  presentedTimeUs: number,
  sequenceDurationUs: number
): boolean {
  if (sequenceDurationUs <= 0) {
    return false;
  }
  const endToleranceUs = workspace.viewModel.project.frameDuration + SEQUENCE_END_EPSILON_US;
  return normalizePlayheadTime(presentedTimeUs) >= Math.max(0, sequenceDurationUs - endToleranceUs);
}

function firstAudioMaterialId(workspace: WorkspaceState): string | null {
  return workspace.materials.find((material) => material.kind === "audio" && material.status === "available")?.materialId ?? null;
}

function projectNameFromBundlePath(bundlePath: string): string {
  const normalized = bundlePath.replace(/\\/g, "/");
  const parts = normalized.split("/").filter((part) => part.length > 0);
  const last = parts.length > 0 ? parts[parts.length - 1] ?? "未命名项目" : "未命名项目";
  return last.endsWith(".veproj") ? last.slice(0, -".veproj".length) || "未命名项目" : last;
}

function toPositiveMicroseconds(value: number): number {
  return Math.max(1, Math.round(Number.isFinite(value) ? value : 1));
}

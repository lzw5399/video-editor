import { useEffect, useRef, useState } from "react";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type {
  CommandResultEnvelope,
  ExportJobStatusResponse,
  ImportMaterialResponse,
  ListMaterialsResponse,
  ListMissingMaterialsResponse,
  PreviewArtifactResponse,
  RuntimeCapabilityReport,
  TimelineCommandResponse
} from "../generated/CommandResultEnvelope";
import type { ExportPreset } from "../generated/CommandEnvelope";
import type { Draft, Material, MaterialKind, SegmentVolume, TextSegment, TrackKind } from "../generated/Draft";
import {
  applyTimelineCommandResult,
  buildAddSegmentCommand,
  buildAddAudioSegmentCommand,
  buildAddTextSegmentCommand,
  buildDeleteSegmentCommand,
  buildEditTextSegmentCommand,
  buildCancelExportCommand,
  buildGetExportJobStatusCommand,
  buildImportMaterialCommand,
  buildListMaterialsCommand,
  buildListMissingMaterialsCommand,
  buildMoveSegmentCommand,
  buildProbeRuntimeCapabilitiesCommand,
  buildRequestPreviewFrameCommand,
  buildRequestPreviewSegmentCommand,
  buildRedoTimelineEditCommand,
  buildSelectTimelineSegmentsCommand,
  buildSetSegmentVolumeCommand,
  buildSetTrackMuteCommand,
  buildSplitSegmentCommand,
  buildStartExportCommand,
  buildTrimSegmentCommand,
  buildUndoTimelineEditCommand,
  commandErrorMessage,
  runtimeDiagnosticsFromError,
  runtimeDiagnosticsFromReport
} from "./commandHelpers";
import {
  createCheckingRuntimeDiagnosticsState,
  createInitialWorkspaceState,
  findFirstMaterialByKind,
  findTrackByKind,
  formatExportDiagnostic,
  formatCommandError,
  formatMicroseconds,
  formatPreviewStatus,
  getSelectedSegmentView,
  getSelectedTrackView,
  initialWorkspaceDraft,
  nextTrackStart,
  type WorkspaceCategory,
  type WorkspaceState
} from "./viewModel";
import { WorkspaceShell } from "./workspace/WorkspaceShell";

type PingResponse = { pong: boolean };
type VersionResponse = { coreVersion: string; contractVersion: string };

const PREVIEW_CACHE_ROOT = "/tmp/video-editor-preview-cache";
const PREVIEW_SEGMENT_DURATION_US = 2_000_000;

type VideoEditorCoreApi = {
  ping: () => Promise<CommandResultEnvelope<PingResponse>>;
  version: () => Promise<CommandResultEnvelope<VersionResponse>>;
  executeCommand: <T = unknown>(command: CommandEnvelope) => Promise<CommandResultEnvelope<T>>;
};

type DraftCommandBuilder = (current: WorkspaceState) => CommandEnvelope;
type DraftCommandResultApplier<T> = (current: WorkspaceState, result: CommandResultEnvelope<T>) => WorkspaceState;
type ExportCommandResultApplier = (
  current: WorkspaceState,
  result: CommandResultEnvelope<ExportJobStatusResponse>
) => WorkspaceState;

declare global {
  interface Window {
    videoEditorCore: VideoEditorCoreApi;
  }
}

export function App(): React.ReactElement {
  const [workspace, setWorkspace] = useState<WorkspaceState>(() => createInitialWorkspaceState());
  const [activeCategory, setActiveCategory] = useState<WorkspaceCategory>("媒体");
  const [bundlePath, setBundlePath] = useState("/tmp/phase-04-demo.veproj");
  const [materialPath, setMaterialPath] = useState("/tmp/demo-material.mp4");
  const [playheadUs, setPlayheadUs] = useState(0);
  const workspaceRef = useRef(workspace);
  const commandInFlightRef = useRef(false);
  const runtimeProbeInFlightRef = useRef(false);

  useEffect(() => {
    workspaceRef.current = workspace;
  }, [workspace]);

  useEffect(() => {
    let cancelled = false;

    async function bootstrapWorkspace(): Promise<void> {
      const [ping, version, materialList] = await Promise.all([
        window.videoEditorCore.ping(),
        window.videoEditorCore.version(),
        window.videoEditorCore.executeCommand<ListMaterialsResponse>(buildListMaterialsCommand(initialWorkspaceDraft))
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

      void handleProbeRuntimeCapabilities();
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

  async function executeDraftCommand<T>(
    buildCommand: DraftCommandBuilder,
    pendingCommand: string,
    applyResult: DraftCommandResultApplier<T>
  ): Promise<void> {
    if (commandInFlightRef.current) {
      setWorkspace((current) => {
        const next = {
          ...current,
          commandError: commandErrorMessage("上一个操作仍在执行，请等待剪辑核心返回")
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
    } finally {
      commandInFlightRef.current = false;
    }
  }

  async function executeTimelineCommand(buildCommand: DraftCommandBuilder, pendingCommand: string): Promise<void> {
    await executeDraftCommand<TimelineCommandResponse>(buildCommand, pendingCommand, (current, result) => {
      const applied = applyTimelineCommandResult(
        {
          draft: current.draft,
          commandState: current.commandState,
          selection: current.selection
        },
        result
      );

      return {
        ...current,
        draft: applied.state.draft,
        commandState: applied.state.commandState,
        selection: applied.state.selection,
        materials: applied.state.draft.materials,
        pendingCommand: null,
        commandError: applied.errorMessage
      };
    });
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

  async function handleImportMaterial(): Promise<void> {
    await executeDraftCommand<ImportMaterialResponse>(
      (current) =>
        buildImportMaterialCommand({
          draft: current.draft,
          bundlePath,
          materialPath
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
            start: nextTrackStart(track),
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

  function handleRequestPreviewFrame(): void {
    if (!workspaceRef.current.runtimeDiagnostics.canPreview) {
      const message = runtimeUnavailableMessage(workspaceRef.current, "预览暂不可用");
      setWorkspace((current) => {
        const next = {
          ...current,
          commandError: message,
          preview: {
            ...current.preview,
            frameStatusLabel: "预览暂不可用",
            error: message
          }
        };
        workspaceRef.current = next;
        return next;
      });
      return;
    }

    const targetTime = Math.max(0, Math.round(playheadUs));

    void executePreviewCommand(
      (current) =>
        buildRequestPreviewFrameCommand({
          draft: current.draft,
          cacheRoot: PREVIEW_CACHE_ROOT,
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
          cacheRoot: PREVIEW_CACHE_ROOT,
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
      bundlePath={bundlePath}
      materialPath={materialPath}
      playheadUs={playheadUs}
      onCategoryChange={setActiveCategory}
      onBundlePathChange={setBundlePath}
      onMaterialPathChange={setMaterialPath}
      onPlayheadChange={setPlayheadUs}
      onRequestPreviewFrame={handleRequestPreviewFrame}
      onRequestPreviewSegment={handleRequestPreviewSegment}
      onProbeRuntimeCapabilities={handleProbeRuntimeCapabilities}
      onExportOutputPathChange={handleExportOutputPathChange}
      onExportPresetChange={handleExportPresetChange}
      onStartExport={handleStartExport}
      onRefreshExportStatus={handleRefreshExportStatus}
      onCancelExport={handleCancelExport}
      onImportMaterial={handleImportMaterial}
      onRefreshMaterials={handleRefreshMaterials}
      onListMissingMaterials={handleListMissingMaterials}
      onAddTextSegment={handleAddTextSegment}
      onAddAudioSegment={handleAddAudioSegment}
      onEditSelectedText={handleEditSelectedText}
      onSetSelectedSegmentVolume={handleSetSelectedSegmentVolume}
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

function toPositiveMicroseconds(value: number): number {
  return Math.max(1, Math.round(Number.isFinite(value) ? value : 1));
}

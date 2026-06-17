import { useEffect, useState } from "react";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type {
  CommandResultEnvelope,
  ImportMaterialResponse,
  ListMaterialsResponse,
  ListMissingMaterialsResponse,
  TimelineCommandResponse
} from "../generated/CommandResultEnvelope";
import type { Draft, Material, MaterialKind, SegmentVolume, TextSegment, TrackKind } from "../generated/Draft";
import {
  applyTimelineCommandResult,
  buildAddSegmentCommand,
  buildAddAudioSegmentCommand,
  buildAddTextSegmentCommand,
  buildDeleteSegmentCommand,
  buildEditTextSegmentCommand,
  buildImportMaterialCommand,
  buildListMaterialsCommand,
  buildListMissingMaterialsCommand,
  buildMoveSegmentCommand,
  buildRedoTimelineEditCommand,
  buildSelectTimelineSegmentsCommand,
  buildSetSegmentVolumeCommand,
  buildSetTrackMuteCommand,
  buildSplitSegmentCommand,
  buildTrimSegmentCommand,
  buildUndoTimelineEditCommand,
  commandErrorMessage
} from "./commandHelpers";
import {
  createInitialWorkspaceState,
  findFirstMaterialByKind,
  findTrackByKind,
  formatCommandError,
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

type VideoEditorCoreApi = {
  ping: () => Promise<CommandResultEnvelope<PingResponse>>;
  version: () => Promise<CommandResultEnvelope<VersionResponse>>;
  executeCommand: <T = unknown>(command: CommandEnvelope) => Promise<CommandResultEnvelope<T>>;
};

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

  async function executeTimelineCommand(command: CommandEnvelope, pendingCommand: string): Promise<void> {
    setWorkspace((current) => ({
      ...current,
      pendingCommand,
      commandError: null
    }));

    try {
      const result = await window.videoEditorCore.executeCommand<TimelineCommandResponse>(command);
      setWorkspace((current) => {
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
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspace((current) => ({
        ...current,
        pendingCommand: null,
        commandError: commandErrorMessage(message)
      }));
    }
  }

  async function handleImportMaterial(): Promise<void> {
    setWorkspace((current) => ({
      ...current,
      pendingCommand: "导入素材",
      commandError: null
    }));

    const command = buildImportMaterialCommand({
      draft: workspace.draft,
      bundlePath,
      materialPath
    });

    try {
      const result = await window.videoEditorCore.executeCommand<ImportMaterialResponse>(command);
      setWorkspace((current) => {
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
      });
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspace((current) => ({
        ...current,
        pendingCommand: null,
        commandError: commandErrorMessage(message)
      }));
    }
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
    const textTrack = findTrackByKind(workspace.draft, "text");

    if (textTrack === null) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("当前草稿没有文字轨道")
      }));
      return;
    }

    const segmentId = `text-segment-${Date.now().toString(36)}`;
    const materialId = `text-material-${Date.now().toString(36)}`;
    const targetStart = nextTrackStart(textTrack);

    void executeTimelineCommand(
      buildAddTextSegmentCommand({
        context: workspace,
        trackId: textTrack.trackId,
        segmentId,
        materialId,
        sourceTimerange: { start: 0, duration: durationUs },
        targetTimerange: { start: targetStart, duration: durationUs },
        text
      }),
      "添加文字"
    );
  }

  function handleAddAudioSegment(materialId: string, durationUs: number): void {
    const audioTrack = findTrackByKind(workspace.draft, "audio");
    const audioMaterial = materialId.length > 0 ? { materialId } : findFirstMaterialByKind(workspace.draft, "audio");

    if (audioTrack === null || audioMaterial === null) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("当前草稿没有可用音频轨道或音频素材")
      }));
      return;
    }

    const targetStart = nextTrackStart(audioTrack);
    void executeTimelineCommand(
      buildAddAudioSegmentCommand({
        context: workspace,
        trackId: audioTrack.trackId,
        segmentId: `audio-segment-${Date.now().toString(36)}`,
        materialId: audioMaterial.materialId,
        sourceTimerange: { start: 0, duration: durationUs },
        targetTimerange: { start: targetStart, duration: durationUs }
      }),
      "添加音频"
    );
  }

  function handleSetSelectedSegmentVolume(levelMillis: number): void {
    const selectedSegment = getSelectedSegmentView(workspace.draft, workspace.selection);

    if (selectedSegment === null) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("请先选择一个片段")
      }));
      return;
    }

    const volume: SegmentVolume = {
      levelMillis: Math.max(0, Math.min(4000, Math.round(levelMillis)))
    };

    void executeTimelineCommand(
      buildSetSegmentVolumeCommand(workspace, selectedSegment.segment.segmentId, volume),
      "调整音量"
    );
  }

  function handleEditSelectedText(text: TextSegment): void {
    const selectedSegment = getSelectedSegmentView(workspace.draft, workspace.selection);

    if (selectedSegment === null || selectedSegment.segment.text === null || selectedSegment.segment.text === undefined) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("请先选择一个文字片段")
      }));
      return;
    }

    void executeTimelineCommand(
      buildEditTextSegmentCommand(workspace, selectedSegment.segment.segmentId, text),
      "应用文字"
    );
  }

  function handleSetSelectedTrackMute(trackId: string, muted: boolean): void {
    const selectedTrack = getSelectedTrackView(workspace.draft, workspace.selection);
    const resolvedTrackId = trackId || selectedTrack?.trackId;

    if (resolvedTrackId === undefined) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("请先选择一个轨道")
      }));
      return;
    }

    void executeTimelineCommand(buildSetTrackMuteCommand(workspace, resolvedTrackId, muted), "切换轨道静音");
  }

  function handleSelectTimelineSegment(segmentId: string): void {
    const selected = getSelectedSegmentView(
      workspace.draft,
      {
        segmentIds: [segmentId],
        trackIds: []
      }
    );

    void executeTimelineCommand(
      buildSelectTimelineSegmentsCommand(workspace, [segmentId], selected === null ? [] : [selected.track.trackId]),
      "选择片段"
    );
  }

  function handleAddTimelineSegment(materialId: string): void {
    const material = resolveTimelineMaterial(workspace.draft, materialId);
    const track = material === null ? null : findTrackByKind(workspace.draft, compatibleTrackKind(material.kind));

    if (material === null || track === null) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("没有可添加到时间线的兼容素材或轨道")
      }));
      return;
    }

    const duration = Math.max(1_000_000, material.metadata.duration ?? 3_000_000);
    void executeTimelineCommand(
      buildAddSegmentCommand({
        context: workspace,
        trackId: track.trackId,
        segmentId: `segment-${Date.now().toString(36)}`,
        materialId: material.materialId,
        sourceTimerange: {
          start: 0,
          duration
        },
        targetTimerange: {
          start: nextTrackStart(track),
          duration
        }
      }),
      "添加片段"
    );
  }

  function handleMoveSelectedSegment(deltaUs: number): void {
    const selected = getSelectedSegmentView(workspace.draft, workspace.selection);

    if (selected === null) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("请先选择一个片段")
      }));
      return;
    }

    void executeTimelineCommand(
      buildMoveSegmentCommand(
        workspace,
        selected.segment.segmentId,
        selected.track.trackId,
        Math.max(0, selected.segment.targetTimerange.start + Math.round(deltaUs))
      ),
      "移动片段"
    );
  }

  function handleSplitSelectedSegment(splitAt: number): void {
    const selected = getSelectedSegmentView(workspace.draft, workspace.selection);

    if (selected === null) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("请先选择一个片段")
      }));
      return;
    }

    void executeTimelineCommand(
      buildSplitSegmentCommand(
        workspace,
        selected.segment.segmentId,
        `segment-right-${Date.now().toString(36)}`,
        Math.max(0, Math.round(splitAt))
      ),
      "分割片段"
    );
  }

  function handleTrimSelectedSegment(direction: "left" | "right", deltaUs: number): void {
    const selected = getSelectedSegmentView(workspace.draft, workspace.selection);

    if (selected === null) {
      setWorkspace((current) => ({
        ...current,
        commandError: commandErrorMessage("请先选择一个片段")
      }));
      return;
    }

    const safeDelta = Math.max(1, Math.round(deltaUs));
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

    void executeTimelineCommand(
      buildTrimSegmentCommand(workspace, selected.segment.segmentId, direction, targetTimerange),
      direction === "left" ? "左侧裁剪" : "右侧裁剪"
    );
  }

  function handleDeleteSelectedSegment(): void {
    const selected = getSelectedSegmentView(workspace.draft, workspace.selection);

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

    void executeTimelineCommand(buildDeleteSegmentCommand(workspace, selected.segment.segmentId), "删除片段");
  }

  function handleUndoTimelineEdit(): void {
    void executeTimelineCommand(buildUndoTimelineEditCommand(workspace), "撤销");
  }

  function handleRedoTimelineEdit(): void {
    void executeTimelineCommand(buildRedoTimelineEditCommand(workspace), "重做");
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
      onUndoTimelineEdit={handleUndoTimelineEdit}
      onRedoTimelineEdit={handleRedoTimelineEdit}
    />
  );
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

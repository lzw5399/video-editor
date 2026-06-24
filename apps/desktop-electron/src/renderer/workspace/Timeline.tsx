import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, DragEvent as ReactDragEvent, PointerEvent as ReactPointerEvent } from "react";

import type { TrackKind } from "../../generated/Draft";
import type { ProjectInteractionPayload } from "../../main/nativeBinding";
import { appIconUrls, type AppIconName } from "../assets/icons";
import {
  formatTimelineTime,
  segmentBlockStyle,
  type TimelineSegmentView,
  type TimelineSegmentVisualKind,
  type TimelineTrackRow as TimelineTrackRowView,
  type WaveformDisplayModel,
  type WorkspaceState
} from "../viewModel";
import { MATERIAL_DRAG_DATA_TYPE, TEXT_SEGMENT_DRAG_DATA_TYPE } from "./dragTypes";
import type { ProjectInteractionController, ProjectInteractionEvidence } from "./projectInteraction";

import "./timeline.css";

const TIMELINE_HEADER_WIDTH_PX = 128;
const SEGMENT_FILMSTRIP_CELL_COUNT = 12;
const SEGMENT_TEXT_CHIP_COUNT = 6;
const AUDIO_WAVEFORM_PLACEHOLDER_PATTERN: readonly ("short" | "medium" | "tall")[] = [
  "short",
  "medium",
  "tall",
  "medium",
  "short",
  "tall",
  "medium",
  "short",
  "medium",
  "tall",
  "short",
  "medium"
];

type TimelineDropPlacement = {
  targetStart: number;
  targetTrackHandle: string | null;
};

type TimelineDragPreviewState = {
  phase: "active" | "committing";
  mode: "move" | "trim-left" | "trim-right";
  deltaPx: number;
  deltaY: number;
  laneWidth: number;
  baseStart: number;
  baseDuration: number;
  baseTrackSelectionHandle: string;
};

type TimelineMoveTrimPayload = Extract<ProjectInteractionPayload, { kind: "timelineMoveTrim" }>;

type TimelineTrackDropTarget = {
  selectionHandle: string;
  kind: TrackKind;
  left: number;
  right: number;
  top: number;
  bottom: number;
};

type TimelineProps = {
  workspace: WorkspaceState;
  showDeveloperDiagnostics: boolean;
  playheadUs: number;
  playbackRunning: boolean;
  projectInteractions: ProjectInteractionController;
  onPlayheadChange: (value: number) => void;
  onTogglePlayback: () => void;
  onStopPlayback: () => void;
  onSelectSegment?: (itemHandle: string) => void;
  onSelectTrack?: (itemHandle: string) => void;
  onAddSegment?: (materialId: string, placement?: TimelineDropPlacement) => void;
  onAddTextSegment?: (content: string, placement?: TimelineDropPlacement) => void;
  onAddTrack?: (trackKind: TrackKind) => void;
  onRenameTrack?: (itemHandle: string, name: string) => void;
  onSetTrackLock?: (itemHandle: string, locked: boolean) => void;
  onSetTrackVisibility?: (itemHandle: string, visible: boolean) => void;
  onMoveSelectedSegment?: (startAt: number, targetTrackHandle?: string | null) => void;
  onSplitSelectedSegment?: () => void;
  onTrimSelectedSegment?: (direction: "left" | "right", trimAt: number) => void;
  onDeleteSelectedSegment?: () => void;
  onSetTrackMute?: (itemHandle: string, muted: boolean) => void;
  onUndo?: () => void;
  onRedo?: () => void;
};

export function Timeline({
  workspace,
  showDeveloperDiagnostics,
  playheadUs,
  playbackRunning,
  projectInteractions,
  onPlayheadChange,
  onTogglePlayback,
  onSelectSegment,
  onSelectTrack,
  onAddSegment,
  onAddTextSegment,
  onAddTrack,
  onRenameTrack,
  onSetTrackLock,
  onSetTrackVisibility,
  onSplitSelectedSegment,
  onDeleteSelectedSegment,
  onSetTrackMute,
  onUndo,
  onRedo
}: TimelineProps): React.ReactElement {
  const timeline = workspace.viewModel.timeline;
  const trackListRef = useRef<HTMLDivElement>(null);
  const trackContentRef = useRef<HTMLDivElement>(null);
  const [zoomPercent, setZoomPercent] = useState(100);
  const [materialDropActive, setMaterialDropActive] = useState(false);
  const [timelineInteractionEvidence, setTimelineInteractionEvidence] = useState<ProjectInteractionEvidence | null>(null);
  const playheadRatio = Math.max(0, Math.min(1, Math.max(0, playheadUs) / Math.max(1, timeline.duration)));
  const playheadStyle = {
    left: `calc(${TIMELINE_HEADER_WIDTH_PX}px + ${playheadRatio * 100}% - ${TIMELINE_HEADER_WIDTH_PX * playheadRatio}px)`
  };
  const timelineSurfaceStyle = {
    "--timeline-header-width": `${TIMELINE_HEADER_WIDTH_PX}px`
  } as CSSProperties;
  const zoomContentStyle = {
    width: `${zoomPercent}%`,
    minWidth: "100%"
  };
  const seekFromTrackClientX = useCallback(
    (clientX: number) => {
      const trackContent = trackContentRef.current;
      if (trackContent === null) {
        return;
      }
      const box = trackContent.getBoundingClientRect();
      const laneLeft = box.left + TIMELINE_HEADER_WIDTH_PX;
      const laneWidth = Math.max(1, box.width - TIMELINE_HEADER_WIDTH_PX);
      onPlayheadChange(pointerTimeFromLane(clientX, laneLeft, laneWidth, timeline.duration));
    },
    [onPlayheadChange, timeline.duration]
  );
  const handleRulerPointerDown = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (event.button !== 0) {
        return;
      }
      const box = event.currentTarget.getBoundingClientRect();
      onPlayheadChange(pointerTimeFromLane(event.clientX, box.left, box.width, timeline.duration));
    },
    [onPlayheadChange, timeline.duration]
  );
  const handlePlayheadPointerDown = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (event.button !== 0) {
        return;
      }
      event.preventDefault();
      event.currentTarget.setPointerCapture(event.pointerId);
      seekFromTrackClientX(event.clientX);
    },
    [seekFromTrackClientX]
  );
  const handlePlayheadPointerMove = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (!event.currentTarget.hasPointerCapture(event.pointerId)) {
        return;
      }
      seekFromTrackClientX(event.clientX);
    },
    [seekFromTrackClientX]
  );
  const handlePlayheadPointerUp = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (!event.currentTarget.hasPointerCapture(event.pointerId)) {
        return;
      }
      seekFromTrackClientX(event.clientX);
      event.currentTarget.releasePointerCapture(event.pointerId);
    },
    [seekFromTrackClientX]
  );
  const canAcceptTimelineDrop = useCallback(
    (dataTransfer: DataTransfer) => {
      if (workspace.pendingCommand !== null) {
        return false;
      }
      const transferTypes = Array.from(dataTransfer.types);
      return (
        (onAddSegment !== undefined && transferTypes.includes(MATERIAL_DRAG_DATA_TYPE)) ||
        (onAddTextSegment !== undefined && transferTypes.includes(TEXT_SEGMENT_DRAG_DATA_TYPE))
      );
    },
    [onAddSegment, onAddTextSegment, workspace.pendingCommand]
  );
  const handleTimelineDragOver = useCallback(
    (event: ReactDragEvent<HTMLDivElement>) => {
      if (!canAcceptTimelineDrop(event.dataTransfer)) {
        return;
      }
      event.preventDefault();
      event.dataTransfer.dropEffect = "copy";
      setMaterialDropActive(true);
    },
    [canAcceptTimelineDrop]
  );
  const handleTimelineDragLeave = useCallback((event: ReactDragEvent<HTMLDivElement>) => {
    const relatedTarget = event.relatedTarget;
    if (relatedTarget instanceof Node && event.currentTarget.contains(relatedTarget)) {
      return;
    }
    setMaterialDropActive(false);
  }, []);
  const handleTimelineDrop = useCallback(
    (event: ReactDragEvent<HTMLDivElement>) => {
      if (!canAcceptTimelineDrop(event.dataTransfer)) {
        setMaterialDropActive(false);
        return;
      }
      event.preventDefault();
      setMaterialDropActive(false);
      const trackContent = trackContentRef.current;
      const placement =
        trackContent === null
          ? undefined
          : {
              targetStart: pointerTimeFromTrackContent(event.clientX, trackContent, timeline.duration),
              targetTrackHandle: timelineTrackHandleAtPoint(event.clientX, event.clientY)
            };

      const materialId = event.dataTransfer.getData(MATERIAL_DRAG_DATA_TYPE).trim();
      if (materialId.length > 0) {
        onAddSegment?.(materialId, placement);
        return;
      }

      const textContent = event.dataTransfer.getData(TEXT_SEGMENT_DRAG_DATA_TYPE).trim();
      if (textContent.length > 0) {
        onAddTextSegment?.(textContent, placement);
      }
    },
    [canAcceptTimelineDrop, onAddSegment, onAddTextSegment, timeline.duration]
  );

  return (
    <div
      className={materialDropActive ? "timeline-surface material-drop-active" : "timeline-surface"}
      style={timelineSurfaceStyle}
    >
      <TransportStrip
        workspace={workspace}
        showDeveloperDiagnostics={showDeveloperDiagnostics}
        playheadUs={playheadUs}
        playbackRunning={playbackRunning}
        onTogglePlayback={onTogglePlayback}
        onAddSegment={onAddSegment}
        onAddTrack={onAddTrack}
        onSplitSelectedSegment={onSplitSelectedSegment}
        onDeleteSelectedSegment={onDeleteSelectedSegment}
        onUndo={onUndo}
        onRedo={onRedo}
        zoomPercent={zoomPercent}
        onZoomPercentChange={setZoomPercent}
      />

      <div className="timeline-ruler-shell">
        <div className="timeline-ruler" aria-label="时间线标尺" style={zoomContentStyle}>
          <div className="timeline-header-spacer" />
          <div className="ruler-track" onPointerDown={handleRulerPointerDown}>
            {timeline.rulerTicks.map((tick) => (
              <span className="ruler-tick" key={tick} style={{ left: `${(tick / timeline.duration) * 100}%` }}>
                {formatTimelineTime(tick)}
              </span>
            ))}
          </div>
        </div>
      </div>

      <div
        className="track-list"
        aria-label="轨道列表"
        data-material-drop-target="true"
        ref={trackListRef}
        onDragEnter={handleTimelineDragOver}
        onDragOver={handleTimelineDragOver}
        onDragLeave={handleTimelineDragLeave}
        onDrop={handleTimelineDrop}
      >
        <div className="track-scroll-content" ref={trackContentRef} style={zoomContentStyle}>
          <div
            className="playhead"
            aria-hidden="true"
            title="播放头拖动"
            style={playheadStyle}
            onPointerDown={handlePlayheadPointerDown}
            onPointerMove={handlePlayheadPointerMove}
            onPointerUp={handlePlayheadPointerUp}
            onPointerCancel={handlePlayheadPointerUp}
          />
          {timeline.rows.map((row) => (
            <TimelineTrackRow
              key={row.rowKey}
              row={row}
              waveform={workspace.waveform}
              timelineDuration={timeline.duration}
              onSelectSegment={onSelectSegment}
              onSelectTrack={onSelectTrack}
              onRenameTrack={onRenameTrack}
              onSetTrackLock={onSetTrackLock}
              onSetTrackVisibility={onSetTrackVisibility}
              onSetTrackMute={onSetTrackMute}
              projectInteractions={projectInteractions}
              interactionEvidence={timelineInteractionEvidence}
              onInteractionEvidenceChange={setTimelineInteractionEvidence}
              pending={workspace.pendingCommand !== null}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function pointerTimeFromLane(clientX: number, laneLeft: number, laneWidth: number, timelineDuration: number): number {
  const ratio = Math.max(0, Math.min(1, (clientX - laneLeft) / Math.max(1, laneWidth)));
  return Math.max(0, Math.round(ratio * Math.max(1, timelineDuration)));
}

function pointerTimeFromTrackContent(clientX: number, trackContent: HTMLElement, timelineDuration: number): number {
  const box = trackContent.getBoundingClientRect();
  const laneLeft = box.left + TIMELINE_HEADER_WIDTH_PX;
  const laneWidth = Math.max(1, box.width - TIMELINE_HEADER_WIDTH_PX);
  return pointerTimeFromLane(clientX, laneLeft, laneWidth, timelineDuration);
}

function timelineTrackHandleAtPoint(clientX: number, clientY: number, excludeElement?: HTMLElement | null): string | null {
  const targets = document.elementsFromPoint(clientX, clientY);
  for (const target of targets) {
    if (!(target instanceof HTMLElement)) {
      continue;
    }
    const row = target.closest(".track-row");
    if (!(row instanceof HTMLElement)) {
      continue;
    }
    if (excludeElement !== null && excludeElement !== undefined && row.contains(excludeElement)) {
      continue;
    }
    const handle = row.dataset.trackSelectionHandle?.trim();
    return handle === undefined || handle.length === 0 ? null : handle;
  }
  return null;
}

function collectTimelineTrackDropTargets(root: Element | null): TimelineTrackDropTarget[] {
  if (root === null) {
    return [];
  }
  return Array.from(root.querySelectorAll(".track-row"))
    .filter((row): row is HTMLElement => row instanceof HTMLElement)
    .flatMap((row) => {
      const selectionHandle = row.dataset.trackSelectionHandle?.trim() ?? "";
      const kind = row.dataset.trackKind;
      if (selectionHandle.length === 0) {
        return [];
      }
      if (!isTrackKind(kind)) {
        return [];
      }
      const rect = row.getBoundingClientRect();
      return [
        {
          selectionHandle,
          kind,
          left: rect.left,
          right: rect.right,
          top: rect.top,
          bottom: rect.bottom
        }
      ];
    });
}

function timelineTrackHandleFromTargetsAtPoint(
  targets: readonly TimelineTrackDropTarget[],
  clientX: number,
  clientY: number,
  segmentVisualKind?: TimelineSegmentVisualKind
): string | null {
  const target = targets.find(
    (candidate) =>
      clientX >= candidate.left &&
      clientX <= candidate.right &&
      clientY >= candidate.top &&
      clientY <= candidate.bottom &&
      (segmentVisualKind === undefined || trackKindAcceptsSegment(candidate.kind, segmentVisualKind))
  );
  return target?.selectionHandle ?? null;
}

function isTrackKind(value: string | undefined): value is TrackKind {
  return value === "video" || value === "audio" || value === "text" || value === "sticker" || value === "filter";
}

function trackKindAcceptsSegment(trackKind: TrackKind, visualKind: TimelineSegmentVisualKind): boolean {
  switch (visualKind) {
    case "video":
    case "image":
      return trackKind === "video";
    case "audio":
      return trackKind === "audio";
    case "text":
      return trackKind === "text";
    case "sticker":
      return trackKind === "sticker";
    case "filter":
      return trackKind === "filter";
  }
}

function TransportStrip({
  workspace,
  showDeveloperDiagnostics,
  playheadUs,
  playbackRunning,
  onTogglePlayback,
  onAddSegment,
  onAddTrack,
  onSplitSelectedSegment,
  onDeleteSelectedSegment,
  onUndo,
  onRedo,
  zoomPercent,
  onZoomPercentChange
}: {
  workspace: WorkspaceState;
  showDeveloperDiagnostics: boolean;
  playheadUs: number;
  playbackRunning: boolean;
  onTogglePlayback: () => void;
  onAddSegment?: (materialId: string) => void;
  onAddTrack?: (trackKind: TrackKind) => void;
  onSplitSelectedSegment?: () => void;
  onDeleteSelectedSegment?: () => void;
  onUndo?: () => void;
  onRedo?: () => void;
  zoomPercent: number;
  onZoomPercentChange: (value: number | ((current: number) => number)) => void;
}): React.ReactElement {
  const timelineMaterials = useMemo(
    () =>
      workspace.materials.filter(
        (material) =>
          material.status === "available" &&
          (material.kind === "video" || material.kind === "image" || material.kind === "audio")
      ),
    [workspace.materials]
  );
  const [materialId, setMaterialId] = useState(timelineMaterials[0]?.materialId ?? "");
  const selectedMaterialId = materialId || (timelineMaterials[0]?.materialId ?? "");
  const editControls = workspace.viewModel.editControls;
  const pending = workspace.pendingCommand !== null;
  const snappingLabel = editControls.snappingLabel;
  const isPlaybackRunning = playbackRunning;
  const togglePlayback = onTogglePlayback;
  const showMaterialQuickAdd = showDeveloperDiagnostics && onAddSegment !== undefined;

  return (
    <div className="transport-strip" aria-label="时间线控制">
      <div className="timeline-edit-cluster timeline-edit-cluster-left">
        <div className="timeline-tool-group transport-buttons" role="group" aria-label="历史">
          <TimelineIconButton
            label="撤销"
            icon="undo"
            onClick={onUndo}
            disabled={pending || !editControls.canUndo}
          />
          <TimelineIconButton
            label="重做"
            icon="redo"
            onClick={onRedo}
            disabled={pending || !editControls.canRedo}
          />
        </div>
        <span className="timeline-tool-divider" aria-hidden="true" />
        <div className="timeline-tool-group" role="group" aria-label="剪辑">
          <TimelineIconButton
            label="分割所选片段"
            icon="split"
            onClick={() => onSplitSelectedSegment?.()}
            disabled={pending || !editControls.hasSelectedSegment}
          />
          <TimelineIconButton
            label="删除所选片段"
            icon="delete"
            className="danger"
            onClick={onDeleteSelectedSegment}
            disabled={pending || !editControls.hasSelectedSegment}
          />
        </div>
        <span className="timeline-tool-divider" aria-hidden="true" />
        <div className="timeline-tool-group" role="group" aria-label="添加轨道">
          <TimelineIconButton label="添加视频轨道" icon="categoryMedia" onClick={() => onAddTrack?.("video")} disabled={pending} />
          <TimelineIconButton label="添加音频轨道" icon="categoryAudio" onClick={() => onAddTrack?.("audio")} disabled={pending} />
          <TimelineIconButton label="添加文字轨道" icon="categoryText" onClick={() => onAddTrack?.("text")} disabled={pending} />
        </div>
        {showMaterialQuickAdd ? (
          <>
            <span className="timeline-tool-divider" aria-hidden="true" />
            <label className="timeline-control compact-select">
              <span>素材</span>
              <select value={selectedMaterialId} onChange={(event) => setMaterialId(event.currentTarget.value)}>
                {timelineMaterials.map((material) => (
                  <option key={material.materialId} value={material.materialId}>
                    {material.displayName}
                  </option>
                ))}
              </select>
            </label>
            <button
              type="button"
              className="transport-button icon-only accent add-action"
              aria-label="添加片段"
              title="添加片段"
              onClick={() => onAddSegment?.(selectedMaterialId)}
              disabled={pending || selectedMaterialId.length === 0}
            >
              <IconGlyph icon="timelineAdd" />
            </button>
          </>
        ) : null}
      </div>
      <div className="timeline-edit-cluster timeline-edit-cluster-center">
        <TimelineIconButton
          label={isPlaybackRunning ? "暂停" : "播放"}
          icon={isPlaybackRunning ? "pause" : "play"}
          onClick={togglePlayback}
          disabled={pending && !isPlaybackRunning}
        />
      </div>
      <div className="timeline-edit-cluster timeline-edit-cluster-right">
        <button
          type="button"
          className="snapping-status"
          aria-label={snappingLabel}
          aria-pressed={editControls.snappingEnabled}
          disabled
        >
          <IconGlyph icon={editControls.snappingEnabled ? "timelineSnapOn" : "timelineSnapOff"} />
        </button>
        <div className="timeline-zoom-shell" aria-label="时间线缩放">
          <TimelineIconButton
            label="缩小时间线"
            icon="zoomOut"
            onClick={() => onZoomPercentChange((current) => Math.max(50, current - 25))}
            disabled={zoomPercent <= 50}
          />
          <input
            aria-label="时间线缩放比例"
            type="range"
            min="50"
            max="200"
            step="25"
            value={zoomPercent}
            onChange={(event) => onZoomPercentChange(event.currentTarget.valueAsNumber)}
          />
          <TimelineIconButton
            label="放大时间线"
            icon="zoomIn"
            onClick={() => onZoomPercentChange((current) => Math.min(200, current + 25))}
            disabled={zoomPercent >= 200}
          />
          <span>{zoomPercent}%</span>
        </div>
        <span className="playhead-time">{formatTimelineTime(playheadUs)}</span>
        {showDeveloperDiagnostics || workspace.pendingCommand !== null ? (
          <span className="timeline-status">{workspace.pendingCommand ?? "等待剪辑命令"}</span>
        ) : null}
      </div>
    </div>
  );
}

function TimelineIconButton({
  label,
  icon,
  symbol,
  className = "",
  disabled = false,
  onClick
}: {
  label: string;
  icon?: AppIconName;
  symbol?: string;
  className?: string;
  disabled?: boolean;
  onClick?: () => void;
}): React.ReactElement {
  return (
    <button
      type="button"
      className={`transport-button icon-only ${className}`.trim()}
      aria-label={label}
      title={label}
      onClick={onClick}
      disabled={disabled}
    >
      <IconGlyph icon={icon} symbol={symbol} />
    </button>
  );
}

function IconGlyph({ icon, symbol }: { icon?: AppIconName; symbol?: string }): React.ReactElement {
  if (icon !== undefined) {
    return <span className="app-icon-mask" style={iconMaskStyle(icon)} aria-hidden="true" />;
  }

  return <span aria-hidden="true">{symbol}</span>;
}

function iconMaskStyle(icon: AppIconName): CSSProperties {
  return { "--app-icon-url": `url("${appIconUrls[icon]}")` } as CSSProperties;
}

function trackKindIcon(kindLabel: string): AppIconName {
  if (kindLabel === "音频") {
    return "categoryAudio";
  }
  if (kindLabel === "文字") {
    return "categoryText";
  }
  return "categoryMedia";
}

function TimelineTrackRow({
  row,
  waveform,
  timelineDuration,
  onSelectSegment,
  onSelectTrack,
  onRenameTrack,
  onSetTrackLock,
  onSetTrackVisibility,
  onSetTrackMute,
  projectInteractions,
  interactionEvidence,
  onInteractionEvidenceChange,
  pending
}: {
  row: TimelineTrackRowView;
  waveform: WaveformDisplayModel;
  timelineDuration: number;
  onSelectSegment?: (itemHandle: string) => void;
  onSelectTrack?: (itemHandle: string) => void;
  onRenameTrack?: (itemHandle: string, name: string) => void;
  onSetTrackLock?: (itemHandle: string, locked: boolean) => void;
  onSetTrackVisibility?: (itemHandle: string, visible: boolean) => void;
  onSetTrackMute?: (itemHandle: string, muted: boolean) => void;
  projectInteractions: ProjectInteractionController;
  interactionEvidence: ProjectInteractionEvidence | null;
  onInteractionEvidenceChange: (evidence: ProjectInteractionEvidence | null) => void;
  pending: boolean;
}): React.ReactElement {
  const [draftName, setDraftName] = useState(row.name);
  const selected = row.selected;
  const canToggleVisibility = row.canToggleVisibility && onSetTrackVisibility !== undefined;

  useEffect(() => {
    setDraftName(row.name);
  }, [row.name]);

  const commitName = useCallback((value: string) => {
    const trimmed = value.trim();
    if (trimmed.length === 0) {
      setDraftName(row.name);
      return;
    }
    if (trimmed !== row.name) {
      onRenameTrack?.(row.selectionHandle, trimmed);
    }
  }, [onRenameTrack, row.selectionHandle, row.name]);

  return (
    <div className={row.rowClassName} data-track-selection-handle={row.selectionHandle} data-track-kind={row.kind}>
      <div className="track-header">
        <div className="track-header-main">
          <button
            type="button"
            className="track-target-button"
            aria-label={`选择轨道 ${row.name}`}
            aria-pressed={selected}
            title={`选择轨道 ${row.name}`}
            onClick={() => onSelectTrack?.(row.selectionHandle)}
            disabled={pending || onSelectTrack === undefined}
          >
            <span className="track-kind-symbol app-icon-mask" style={iconMaskStyle(trackKindIcon(row.kindLabel))} aria-hidden="true" />
          </button>
          <input
            className="track-name-input"
            aria-label={`${row.name} 名称`}
            value={draftName}
            disabled={pending || onRenameTrack === undefined}
            onChange={(event) => setDraftName(event.currentTarget.value)}
            onBlur={(event) => commitName(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                commitName(event.currentTarget.value);
              }
              if (event.key === "Escape") {
                setDraftName(row.name);
                event.currentTarget.blur();
              }
            }}
          />
        </div>
        <div className="track-header-controls" aria-label={`${row.name} 状态`}>
          <TrackStateButton
            label={`${row.name} 锁定状态：${row.lockLabel}`}
            icon={row.lockActive ? "trackLockOn" : "trackLockOff"}
            active={row.lockActive}
            disabled={pending || onSetTrackLock === undefined}
            onClick={() => onSetTrackLock?.(row.selectionHandle, row.nextLocked)}
          />
          <TrackStateButton
            label={`${row.name} 可见状态：${row.visibilityLabel}`}
            icon={row.visibilityActive ? "trackHideOff" : "trackHideOn"}
            active={row.visibilityActive}
            disabled={pending || !canToggleVisibility}
            onClick={() => onSetTrackVisibility?.(row.selectionHandle, row.nextVisible)}
          />
          <TrackStateButton
            label={`${row.name} 静音状态：${row.muteLabel}`}
            icon={row.muteActive ? "trackMuteOn" : "trackMuteOff"}
            active={row.muteActive}
            disabled={pending || !row.canToggleMute || onSetTrackMute === undefined}
            onClick={() => onSetTrackMute?.(row.selectionHandle, row.nextMuted)}
          />
        </div>
      </div>
      <div className="segment-lane">
        {row.segments.map((segment) => (
          <TimelineSegmentBlock
            key={segment.segmentKey}
            segment={segment}
            waveform={waveform}
            timelineDuration={timelineDuration}
            onSelectSegment={onSelectSegment}
            projectInteractions={projectInteractions}
            interactionEvidence={interactionEvidence}
            onInteractionEvidenceChange={onInteractionEvidenceChange}
            trackSelectionHandle={row.selectionHandle}
            pending={pending}
          />
        ))}
      </div>
    </div>
  );
}

function TimelineSegmentBlock({
  segment,
  waveform,
  timelineDuration,
  onSelectSegment,
  projectInteractions,
  interactionEvidence,
  onInteractionEvidenceChange,
  trackSelectionHandle,
  pending
}: {
  segment: TimelineSegmentView;
  waveform: WaveformDisplayModel;
  timelineDuration: number;
  onSelectSegment?: (itemHandle: string) => void;
  projectInteractions: ProjectInteractionController;
  interactionEvidence: ProjectInteractionEvidence | null;
  onInteractionEvidenceChange: (evidence: ProjectInteractionEvidence | null) => void;
  trackSelectionHandle: string;
  pending: boolean;
}): React.ReactElement {
  const showKeyframeStrip = segment.selected || segment.duration >= 700_000;
  const showAudioWaveform = segment.visualKind === "audio";
  const [dragPreview, setDragPreview] = useState<TimelineDragPreviewState | null>(null);
  const dragRef = useRef<{
    mode: "move" | "trim-left" | "trim-right";
    pointerId: number;
    startClientX: number;
    startClientY: number;
    laneWidth: number;
    trackTargets: TimelineTrackDropTarget[];
    moved: boolean;
    baseStart: number;
    baseDuration: number;
    baseTrackSelectionHandle: string;
    interactionId: string | null;
    sequence: number;
    beginPromise: Promise<void>;
    updateInFlight: boolean;
    rafId: number | null;
    pendingPayload: TimelineMoveTrimPayload | null;
  } | null>(null);
  const suppressClickRef = useRef(false);
  const segmentStyle = buildTimelineSegmentBlockStyle(segment, timelineDuration, dragPreview);

  useEffect(() => {
    setDragPreview((current) => {
      if (current === null || current.phase !== "committing") {
        return current;
      }
      if (
        current.baseStart !== segment.start ||
        current.baseDuration !== segment.duration ||
        current.baseTrackSelectionHandle !== trackSelectionHandle
      ) {
        return null;
      }
      return current;
    });
  }, [segment.duration, segment.start, trackSelectionHandle]);

  useEffect(() => {
    if (dragPreview?.phase !== "committing") {
      return undefined;
    }
    const timeout = window.setTimeout(() => setDragPreview(null), 1500);
    return () => window.clearTimeout(timeout);
  }, [dragPreview]);

  const beginPointerIntent = useCallback(
    (event: ReactPointerEvent<HTMLElement>, mode: "move" | "trim-left" | "trim-right") => {
      if (event.button !== 0 || pending) {
        return;
      }
      const lane = event.currentTarget.closest(".segment-lane");
      if (!(lane instanceof HTMLElement)) {
        return;
      }
      const laneWidth = Math.max(1, lane.getBoundingClientRect().width);
      const trackTargets = collectTimelineTrackDropTargets(lane.closest(".track-scroll-content"));
      event.preventDefault();
      event.stopPropagation();
      event.currentTarget.setPointerCapture(event.pointerId);
      if (!segment.selected) {
        onSelectSegment?.(segment.selectionHandle);
      }
      const interaction = {
        mode,
        pointerId: event.pointerId,
        startClientX: event.clientX,
        startClientY: event.clientY,
        laneWidth,
        trackTargets,
        moved: false,
        baseStart: segment.start,
        baseDuration: segment.duration,
        baseTrackSelectionHandle: trackSelectionHandle,
        interactionId: null,
        sequence: 0,
        beginPromise: Promise.resolve(),
        updateInFlight: false,
        rafId: null,
        pendingPayload: null
      };
      dragRef.current = {
        ...interaction,
        beginPromise: projectInteractions.begin("timelineMoveTrim").then((begin) => {
          if (dragRef.current === null || dragRef.current.pointerId !== interaction.pointerId || begin === null) {
            return;
          }
          dragRef.current.interactionId = begin.interactionId;
          onInteractionEvidenceChange({
            kind: "timelineMoveTrim",
            generation: begin.generation
          });
          flushTimelineMoveTrimUpdate(dragRef.current);
        })
      };
      setDragPreview({
        phase: "active",
        mode,
        deltaPx: 0,
        deltaY: 0,
        laneWidth,
        baseStart: segment.start,
        baseDuration: segment.duration,
        baseTrackSelectionHandle: trackSelectionHandle
      });
    },
    [
      onInteractionEvidenceChange,
      onSelectSegment,
      pending,
      projectInteractions,
      segment.duration,
      segment.selected,
      segment.selectionHandle,
      segment.start,
      trackSelectionHandle
    ]
  );

  const updatePointerIntent = useCallback(
    (event: ReactPointerEvent<HTMLElement>) => {
      const drag = dragRef.current;
      if (drag === null || drag.pointerId !== event.pointerId) {
        return;
      }
      if (Math.abs(event.clientX - drag.startClientX) + Math.abs(event.clientY - drag.startClientY) >= 3) {
        drag.moved = true;
      }
      const deltaPx = event.clientX - drag.startClientX;
      const deltaY = drag.mode === "move" ? event.clientY - drag.startClientY : 0;
      setDragPreview({
        phase: "active",
        mode: drag.mode,
        deltaPx,
        deltaY,
        laneWidth: drag.laneWidth,
        baseStart: drag.baseStart,
        baseDuration: drag.baseDuration,
        baseTrackSelectionHandle: drag.baseTrackSelectionHandle
      });
      queueTimelineMoveTrimUpdate(drag, timelineMoveTrimPayloadFromPointer(drag, event.clientX, event.clientY, timelineDuration));
    },
    [timelineDuration]
  );

  const completePointerIntent = useCallback(
    (event: ReactPointerEvent<HTMLElement>) => {
      const drag = dragRef.current;
      if (drag === null || drag.pointerId !== event.pointerId) {
        return;
      }
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
      event.preventDefault();
      event.stopPropagation();

      const deltaUs = Math.round(((event.clientX - drag.startClientX) / drag.laneWidth) * Math.max(1, timelineDuration));
      const targetTrackHandle =
        drag.mode === "move" ? timelineTrackHandleFromTargetsAtPoint(drag.trackTargets, event.clientX, event.clientY, segment.visualKind) : null;
      const trackChanged = targetTrackHandle !== null && targetTrackHandle !== trackSelectionHandle;
      if (!drag.moved || (deltaUs === 0 && !trackChanged)) {
        setDragPreview(null);
        void finishTimelineMoveTrimInteraction(drag, "cancel");
        return;
      }
      suppressClickRef.current = true;
      const committingPreview: TimelineDragPreviewState = {
        phase: "committing",
        mode: drag.mode,
        deltaPx: event.clientX - drag.startClientX,
        deltaY: drag.mode === "move" ? event.clientY - drag.startClientY : 0,
        laneWidth: drag.laneWidth,
        baseStart: drag.baseStart,
        baseDuration: drag.baseDuration,
        baseTrackSelectionHandle: drag.baseTrackSelectionHandle
      };
      setDragPreview(committingPreview);
      queueTimelineMoveTrimUpdate(drag, timelineMoveTrimPayloadFromPointer(drag, event.clientX, event.clientY, timelineDuration));
      void finishTimelineMoveTrimInteraction(drag, "commit");
    },
    [timelineDuration, trackSelectionHandle]
  );

  function timelineMoveTrimPayloadFromPointer(
    interaction: NonNullable<typeof dragRef.current>,
    clientX: number,
    clientY: number,
    duration: number
  ): TimelineMoveTrimPayload {
    const deltaUs = Math.round(((clientX - interaction.startClientX) / interaction.laneWidth) * Math.max(1, duration));
    if (interaction.mode === "move") {
      const targetTrackHandle = timelineTrackHandleFromTargetsAtPoint(
        interaction.trackTargets,
        clientX,
        clientY,
        segment.visualKind
      );
      return {
        kind: "timelineMoveTrim",
        mode: "move",
        startAt: Math.max(0, interaction.baseStart + deltaUs),
        targetTrackHandle
      };
    }
    if (interaction.mode === "trim-left") {
      return {
        kind: "timelineMoveTrim",
        mode: "trimLeft",
        trimAt: Math.max(0, interaction.baseStart + deltaUs)
      };
    }
    return {
      kind: "timelineMoveTrim",
      mode: "trimRight",
      trimAt: Math.max(interaction.baseStart + 1, interaction.baseStart + interaction.baseDuration + deltaUs)
    };
  }

  function queueTimelineMoveTrimUpdate(
    interaction: NonNullable<typeof dragRef.current>,
    payload: TimelineMoveTrimPayload
  ): void {
    interaction.pendingPayload = payload;
    if (interaction.rafId !== null) {
      return;
    }
    interaction.rafId = window.requestAnimationFrame(() => {
      interaction.rafId = null;
      flushTimelineMoveTrimUpdate(interaction);
    });
  }

  function flushTimelineMoveTrimUpdate(interaction: NonNullable<typeof dragRef.current>): void {
    if (interaction.updateInFlight || interaction.interactionId === null || interaction.pendingPayload === null) {
      return;
    }
    const payload = interaction.pendingPayload;
    interaction.pendingPayload = null;
    interaction.updateInFlight = true;
    interaction.sequence += 1;
    void projectInteractions.update(interaction.interactionId, interaction.sequence, payload).then((update) => {
      interaction.updateInFlight = false;
      if (update !== null) {
        onInteractionEvidenceChange({
          kind: "timelineMoveTrim",
          generation: update.generation
        });
        setDragPreview(null);
      }
      if (dragRef.current !== interaction) {
        return;
      }
      flushTimelineMoveTrimUpdate(interaction);
    });
  }

  async function finishTimelineMoveTrimInteraction(
    interaction: NonNullable<typeof dragRef.current>,
    action: "commit" | "cancel"
  ): Promise<void> {
    if (interaction.rafId !== null) {
      window.cancelAnimationFrame(interaction.rafId);
      interaction.rafId = null;
    }
    await interaction.beginPromise;
    while (interaction.updateInFlight) {
      await new Promise((resolve) => window.setTimeout(resolve, 0));
    }
    if (interaction.pendingPayload !== null) {
      flushTimelineMoveTrimUpdate(interaction);
      while (interaction.updateInFlight || interaction.pendingPayload !== null) {
        await new Promise((resolve) => window.setTimeout(resolve, 0));
      }
    }
    if (interaction.interactionId === null) {
      setDragPreview(null);
      onInteractionEvidenceChange(null);
      if (dragRef.current === interaction) {
        dragRef.current = null;
      }
      return;
    }
    if (action === "commit") {
      await projectInteractions.commit(interaction.interactionId);
    } else {
      await projectInteractions.cancel(interaction.interactionId);
    }
    setDragPreview(null);
    onInteractionEvidenceChange(null);
    if (dragRef.current === interaction) {
      dragRef.current = null;
    }
  }

  const rustProvisionalActive =
    segment.selected && interactionEvidence?.kind === "timelineMoveTrim" ? interactionEvidence : null;

  return (
    <button
      type="button"
      className={`segment-block segment-kind-${segment.visualKind}${segment.selected ? " selected" : ""}${
        dragPreview !== null ? " dragging" : ""
      }`}
      style={segmentStyle}
      data-interaction-source={rustProvisionalActive !== null ? "rust-provisional" : undefined}
      data-interaction-kind={rustProvisionalActive?.kind}
      onPointerDown={(event) => beginPointerIntent(event, "move")}
      onPointerMove={updatePointerIntent}
      onPointerUp={completePointerIntent}
      onPointerCancel={completePointerIntent}
      onClick={() => {
        if (suppressClickRef.current) {
          suppressClickRef.current = false;
          return;
        }
        if (dragRef.current === null) {
          onSelectSegment?.(segment.selectionHandle);
        }
      }}
      aria-pressed={segment.selected}
      title={`${segment.label}，${segment.targetLabel}`}
      aria-label={`片段 ${segment.label}`}
    >
      <span
        className="segment-trim-handle left"
        aria-label={`${segment.label} 左侧裁剪手柄`}
        title="左侧裁剪"
        onPointerDown={(event) => beginPointerIntent(event, "trim-left")}
        onPointerMove={updatePointerIntent}
        onPointerUp={completePointerIntent}
        onPointerCancel={completePointerIntent}
      />
      <SegmentVisualBed visualKind={segment.visualKind} />
      <strong>{segment.label}</strong>
      <span className="segment-time-label">{segment.targetLabel}</span>
      {showAudioWaveform && segment.waveformMaterialId !== null ? (
        <AudioWaveform waveform={waveform} materialId={segment.waveformMaterialId} />
      ) : null}
      {segment.keyframeMarkers.length > 0 && showKeyframeStrip ? (
        <span className="segment-keyframe-strip" aria-label="关键帧标记">
          {segment.keyframeMarkers.map((marker) => (
            <span
              key={marker.markerKey}
              className="segment-keyframe-marker"
              style={{
                left: `${marker.positionPerMille / 10}%`
              }}
              title={marker.title}
              aria-label={marker.ariaLabel}
            />
          ))}
        </span>
      ) : null}
      <span
        className="segment-trim-handle right"
        aria-label={`${segment.label} 右侧裁剪手柄`}
        title="右侧裁剪"
        onPointerDown={(event) => beginPointerIntent(event, "trim-right")}
        onPointerMove={updatePointerIntent}
        onPointerUp={completePointerIntent}
        onPointerCancel={completePointerIntent}
      />
    </button>
  );
}

function buildTimelineSegmentBlockStyle(
  segment: TimelineSegmentView,
  timelineDuration: number,
  dragPreview: TimelineDragPreviewState | null
): CSSProperties {
  const baseStyle = segmentBlockStyle(segment, timelineDuration);
  if (dragPreview === null) {
    return baseStyle;
  }

  const safeDuration = Math.max(1, timelineDuration);
  const baseLeftPercent = (Math.max(0, segment.start) / safeDuration) * 100;
  const baseWidthPercent = (Math.max(1, segment.duration) / safeDuration) * 100;
  const baseWidthPx = (Math.max(1, segment.duration) / safeDuration) * Math.max(1, dragPreview.laneWidth);
  const maxShrinkPx = Math.max(0, baseWidthPx - 8);

  if (dragPreview.mode === "move") {
    return {
      ...baseStyle,
      transform: `translate(${Math.round(dragPreview.deltaPx)}px, ${Math.round(dragPreview.deltaY)}px)`,
      zIndex: 8
    };
  }

  if (dragPreview.mode === "trim-left") {
    const deltaPx = Math.max(0, Math.min(maxShrinkPx, dragPreview.deltaPx));
    return {
      ...baseStyle,
      left: `calc(${baseLeftPercent}% + ${Math.round(deltaPx)}px)`,
      width: `calc(${baseWidthPercent}% - ${Math.round(deltaPx)}px)`,
      zIndex: 8
    };
  }

  const deltaPx = Math.min(0, Math.max(-maxShrinkPx, dragPreview.deltaPx));
  return {
    ...baseStyle,
    width: `calc(${baseWidthPercent}% + ${Math.round(deltaPx)}px)`,
    zIndex: 8
  };
}

function SegmentVisualBed({ visualKind }: { visualKind: TimelineSegmentView["visualKind"] }): React.ReactElement | null {
  if (visualKind === "video" || visualKind === "image" || visualKind === "sticker") {
    return (
      <span className={`segment-visual-bed segment-filmstrip segment-filmstrip-${visualKind}`} aria-hidden="true">
        {Array.from({ length: SEGMENT_FILMSTRIP_CELL_COUNT }, (_, index) => (
          <span key={index} className="segment-filmstrip-cell" />
        ))}
      </span>
    );
  }

  if (visualKind === "text") {
    return (
      <span className="segment-visual-bed segment-text-bed" aria-hidden="true">
        {Array.from({ length: SEGMENT_TEXT_CHIP_COUNT }, (_, index) => (
          <span key={index} className="segment-text-chip" />
        ))}
      </span>
    );
  }

  if (visualKind === "filter") {
    return <span className="segment-visual-bed segment-effect-bed" aria-hidden="true" />;
  }

  return null;
}

function AudioWaveform({
  waveform,
  materialId
}: {
  waveform: WaveformDisplayModel;
  materialId: string;
}): React.ReactElement {
  if (waveform.status === "ready" && waveform.materialId === materialId && waveform.peaks.length > 0) {
    return (
      <span className="segment-wave-bed audio-waveform-placeholder audio-waveform-ready" aria-label="音频波形" title={waveform.statusLabel}>
        {waveform.peaks.map((peak, index) => {
          const heightMillis = Math.max(Math.abs(peak.minMillis), Math.abs(peak.maxMillis));
          return (
            <span
              key={`${peak.minMillis}-${peak.maxMillis}-${index}`}
              className="audio-waveform-bar"
              style={{ height: `${Math.max(3, Math.min(14, Math.round((heightMillis / 1000) * 14)))}px` }}
              aria-hidden="true"
            />
          );
        })}
      </span>
    );
  }

  return (
    <span
      className={`segment-wave-bed audio-waveform-placeholder audio-waveform-${waveform.status}`}
      aria-label="音频波形占位"
      title={waveform.statusLabel}
    >
      {AUDIO_WAVEFORM_PLACEHOLDER_PATTERN.map((height, index) => (
        <span key={`${height}-${index}`} className="audio-waveform-bar" data-height={height} aria-hidden="true" />
      ))}
      {waveform.status === "pending" || waveform.status === "failed" ? (
        <span className="audio-waveform-state">{waveform.statusLabel}</span>
      ) : null}
    </span>
  );
}

function TrackStateButton({
  label,
  icon,
  active = false,
  disabled = false,
  onClick
}: {
  label: string;
  icon: AppIconName;
  active?: boolean;
  disabled?: boolean;
  onClick?: () => void;
}): React.ReactElement {
  return (
    <button
      type="button"
      className={active ? "track-state-button active" : "track-state-button"}
      aria-label={label}
      title={label}
      onClick={onClick}
      disabled={disabled}
    >
      <IconGlyph icon={icon} />
    </button>
  );
}

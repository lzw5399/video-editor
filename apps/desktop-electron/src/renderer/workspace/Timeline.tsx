import { useCallback, useMemo, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";

import type { SegmentId } from "../../generated/Draft";
import {
  deriveTimelineRows,
  formatKeyframeEasing,
  formatKeyframeProperty,
  formatTimelineTime,
  segmentBlockStyle,
  type WorkspaceState
} from "../viewModel";

import "./timeline.css";

const TIMELINE_HEADER_WIDTH_PX = 160;
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

type TimelineProps = {
  workspace: WorkspaceState;
  playheadUs: number;
  playbackRunning: boolean;
  onPlayheadChange: (value: number) => void;
  onTogglePlayback: () => void;
  onStopPlayback: () => void;
  onSelectSegment?: (segmentId: SegmentId) => void;
  onAddSegment?: (materialId: string) => void;
  onMoveSelectedSegment?: (deltaUs: number) => void;
  onSplitSelectedSegment?: (splitAt: number) => void;
  onTrimSelectedSegment?: (direction: "left" | "right", deltaUs: number) => void;
  onDeleteSelectedSegment?: () => void;
  onSetTrackMute?: (trackId: string, muted: boolean) => void;
  onUndo?: () => void;
  onRedo?: () => void;
};

export function Timeline({
  workspace,
  playheadUs,
  playbackRunning,
  onPlayheadChange,
  onTogglePlayback,
  onStopPlayback,
  onSelectSegment,
  onAddSegment,
  onMoveSelectedSegment,
  onSplitSelectedSegment,
  onTrimSelectedSegment,
  onDeleteSelectedSegment,
  onSetTrackMute,
  onUndo,
  onRedo
}: TimelineProps): React.ReactElement {
  const timeline = deriveTimelineRows(workspace.draft, workspace.selection);
  const trackListRef = useRef<HTMLDivElement>(null);
  const playheadRatio = Math.max(0, Math.min(1, Math.max(0, playheadUs) / Math.max(1, timeline.duration)));
  const playheadStyle = {
    left: `calc(${TIMELINE_HEADER_WIDTH_PX}px + ${playheadRatio * 100}% - ${TIMELINE_HEADER_WIDTH_PX * playheadRatio}px)`
  };
  const seekFromTrackClientX = useCallback(
    (clientX: number) => {
      const trackList = trackListRef.current;
      if (trackList === null) {
        return;
      }
      const box = trackList.getBoundingClientRect();
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

  return (
    <div className="timeline-surface">
      <TransportStrip
        workspace={workspace}
        playheadUs={playheadUs}
        playbackRunning={playbackRunning}
        onPlayheadChange={onPlayheadChange}
        onTogglePlayback={onTogglePlayback}
        onStopPlayback={onStopPlayback}
        onAddSegment={onAddSegment}
        onMoveSelectedSegment={onMoveSelectedSegment}
        onSplitSelectedSegment={onSplitSelectedSegment}
        onTrimSelectedSegment={onTrimSelectedSegment}
        onDeleteSelectedSegment={onDeleteSelectedSegment}
        onUndo={onUndo}
        onRedo={onRedo}
      />

      <div className="timeline-ruler" aria-label="时间线标尺">
        <div className="timeline-header-spacer" />
        <div className="ruler-track" onPointerDown={handleRulerPointerDown}>
          {timeline.rulerTicks.map((tick) => (
            <span className="ruler-tick" key={tick} style={{ left: `${(tick / timeline.duration) * 100}%` }}>
              {formatTimelineTime(tick)}
            </span>
          ))}
        </div>
      </div>

      <div className="track-list" aria-label="轨道列表" ref={trackListRef}>
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
            key={row.track.trackId}
            row={row}
            timelineDuration={timeline.duration}
            onSelectSegment={onSelectSegment}
            onSetTrackMute={onSetTrackMute}
            pending={workspace.pendingCommand !== null}
          />
        ))}
      </div>
    </div>
  );
}

function pointerTimeFromLane(clientX: number, laneLeft: number, laneWidth: number, timelineDuration: number): number {
  const ratio = Math.max(0, Math.min(1, (clientX - laneLeft) / Math.max(1, laneWidth)));
  return Math.max(0, Math.round(ratio * Math.max(1, timelineDuration)));
}

function TransportStrip({
  workspace,
  playheadUs,
  playbackRunning,
  onPlayheadChange,
  onTogglePlayback,
  onStopPlayback,
  onAddSegment,
  onMoveSelectedSegment,
  onSplitSelectedSegment,
  onTrimSelectedSegment,
  onDeleteSelectedSegment,
  onUndo,
  onRedo
}: {
  workspace: WorkspaceState;
  playheadUs: number;
  playbackRunning: boolean;
  onPlayheadChange: (value: number) => void;
  onTogglePlayback: () => void;
  onStopPlayback: () => void;
  onAddSegment?: (materialId: string) => void;
  onMoveSelectedSegment?: (deltaUs: number) => void;
  onSplitSelectedSegment?: (splitAt: number) => void;
  onTrimSelectedSegment?: (direction: "left" | "right", deltaUs: number) => void;
  onDeleteSelectedSegment?: () => void;
  onUndo?: () => void;
  onRedo?: () => void;
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
  const [moveStepUs, setMoveStepUs] = useState(500_000);
  const [splitAtUs, setSplitAtUs] = useState(playheadUs);
  const [trimStepUs, setTrimStepUs] = useState(500_000);
  const [zoomPercent, setZoomPercent] = useState(100);
  const selectedMaterialId = materialId || (timelineMaterials[0]?.materialId ?? "");
  const hasSelection = workspace.selection.segmentIds.length > 0;
  const pending = workspace.pendingCommand !== null;
  const snappingLabel = workspace.commandState.snapping.enabled ? "吸附 开" : "吸附 关";
  const isPlaybackRunning = playbackRunning;
  const togglePlayback = onTogglePlayback;
  const stopPlayback = onStopPlayback;

  return (
    <div className="transport-strip" aria-label="时间线控制">
      <div className="timeline-tool-group transport-buttons" role="group" aria-label="播放与历史">
        <TimelineIconButton
          label="撤销"
          symbol="↶"
          onClick={onUndo}
          disabled={pending || workspace.commandState.undoStack.length === 0}
        />
        <TimelineIconButton
          label="重做"
          symbol="↷"
          onClick={onRedo}
          disabled={pending || workspace.commandState.redoStack.length === 0}
        />
        <TimelineIconButton
          label={isPlaybackRunning ? "暂停" : "播放"}
          symbol={isPlaybackRunning ? "⏸" : "▶"}
          onClick={togglePlayback}
          disabled={(pending && !isPlaybackRunning) || !workspace.runtimeDiagnostics.canPreview}
        />
        <TimelineIconButton label="停止" symbol="■" onClick={stopPlayback} disabled={pending && !isPlaybackRunning} />
      </div>
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
        className="transport-button symbol-action accent add-action"
        aria-label="添加片段"
        title="添加片段"
        onClick={() => onAddSegment?.(selectedMaterialId)}
        disabled={pending || selectedMaterialId.length === 0}
      >
        添加片段
      </button>
      <label className="playhead-control">
        <span>播放头</span>
        <input
          type="number"
          min="0"
          step="100000"
          value={playheadUs}
          onChange={(event) => onPlayheadChange(Math.max(0, event.currentTarget.valueAsNumber || 0))}
        />
      </label>
      <label className="timeline-control">
        <span>移动</span>
        <input
          type="number"
          min="1"
          step="100000"
          value={moveStepUs}
          onChange={(event) => setMoveStepUs(Math.max(1, event.currentTarget.valueAsNumber || 1))}
        />
      </label>
      <div className="timeline-tool-group" role="group" aria-label="移动片段">
        <TimelineIconButton
          label="左移所选片段"
          symbol="←"
          onClick={() => onMoveSelectedSegment?.(-moveStepUs)}
          disabled={pending || !hasSelection}
        />
        <TimelineIconButton
          label="右移所选片段"
          symbol="→"
          onClick={() => onMoveSelectedSegment?.(moveStepUs)}
          disabled={pending || !hasSelection}
        />
      </div>
      <label className="timeline-control">
        <span>分割</span>
        <input
          type="number"
          min="0"
          step="100000"
          value={splitAtUs}
          onChange={(event) => setSplitAtUs(Math.max(0, event.currentTarget.valueAsNumber || 0))}
        />
      </label>
      <TimelineIconButton
        label="分割所选片段"
        symbol="⧉"
        onClick={() => onSplitSelectedSegment?.(splitAtUs)}
        disabled={pending || !hasSelection}
      />
      <label className="timeline-control">
        <span>裁剪</span>
        <input
          type="number"
          min="1"
          step="100000"
          value={trimStepUs}
          onChange={(event) => setTrimStepUs(Math.max(1, event.currentTarget.valueAsNumber || 1))}
        />
      </label>
      <div className="timeline-tool-group" role="group" aria-label="裁剪片段">
        <TimelineIconButton
          label="左侧裁剪"
          symbol="["
          onClick={() => onTrimSelectedSegment?.("left", trimStepUs)}
          disabled={pending || !hasSelection}
        />
        <TimelineIconButton
          label="右侧裁剪"
          symbol="]"
          onClick={() => onTrimSelectedSegment?.("right", trimStepUs)}
          disabled={pending || !hasSelection}
        />
      </div>
      <TimelineIconButton
        label="删除所选片段"
        symbol="⌫"
        className="danger"
        onClick={onDeleteSelectedSegment}
        disabled={pending || !hasSelection}
      />
      <div className="timeline-zoom-shell" aria-label="时间线缩放">
        <TimelineIconButton
          label="缩小时间线"
          symbol="-"
          onClick={() => setZoomPercent((current) => Math.max(50, current - 25))}
          disabled={zoomPercent <= 50}
        />
        <input
          aria-label="时间线缩放比例"
          type="range"
          min="50"
          max="200"
          step="25"
          value={zoomPercent}
          onChange={(event) => setZoomPercent(event.currentTarget.valueAsNumber)}
        />
        <TimelineIconButton
          label="放大时间线"
          symbol="+"
          onClick={() => setZoomPercent((current) => Math.min(200, current + 25))}
          disabled={zoomPercent >= 200}
        />
        <span>{zoomPercent}%</span>
      </div>
      <span className="snapping-status" aria-label={snappingLabel}>
        {snappingLabel}
      </span>
      <span className="playhead-time">{formatTimelineTime(playheadUs)}</span>
      <span className="timeline-status">{workspace.pendingCommand ?? "等待剪辑命令"}</span>
    </div>
  );
}

function TimelineIconButton({
  label,
  symbol,
  className = "",
  disabled = false,
  onClick
}: {
  label: string;
  symbol: string;
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
      <span aria-hidden="true">{symbol}</span>
    </button>
  );
}

function TimelineTrackRow({
  row,
  timelineDuration,
  onSelectSegment,
  onSetTrackMute,
  pending
}: {
  row: ReturnType<typeof deriveTimelineRows>["rows"][number];
  timelineDuration: number;
  onSelectSegment?: (segmentId: SegmentId) => void;
  onSetTrackMute?: (trackId: string, muted: boolean) => void;
  pending: boolean;
}): React.ReactElement {
  return (
    <div className={row.rowClassName}>
      <div className="track-header">
        <div className="track-header-main">
          <span className="track-kind-symbol" aria-hidden="true">
            {row.symbol}
          </span>
          <strong>{row.track.name}</strong>
        </div>
        <div className="track-header-controls" aria-label={`${row.track.name} 状态`}>
          <TrackStateButton label={`${row.track.name} 锁定状态：${row.lockLabel}`} symbol="锁" active={row.track.locked} disabled />
          <TrackStateButton
            label={`${row.track.name} 可见状态：${row.visibilityLabel}`}
            symbol={row.track.kind === "audio" ? "听" : "眼"}
            disabled
          />
          <TrackStateButton
            label={`${row.track.name} 静音状态：${row.muteLabel}`}
            symbol="静"
            active={row.track.muted}
            disabled={pending || onSetTrackMute === undefined}
            onClick={() => onSetTrackMute?.(row.track.trackId, !row.track.muted)}
          />
        </div>
        <span className="track-status-line">
          {row.statusLabel} · {row.lockLabel} · {row.muteLabel}
        </span>
      </div>
      <div className="segment-lane">
        {row.segments.map((segment) => (
          <TimelineSegmentBlock
            key={segment.segment.segmentId}
            segment={segment}
            timelineDuration={timelineDuration}
            onSelectSegment={onSelectSegment}
          />
        ))}
      </div>
    </div>
  );
}

function TimelineSegmentBlock({
  segment,
  timelineDuration,
  onSelectSegment
}: {
  segment: ReturnType<typeof deriveTimelineRows>["rows"][number]["segments"][number];
  timelineDuration: number;
  onSelectSegment?: (segmentId: SegmentId) => void;
}): React.ReactElement {
  const showKeyframeStrip = segment.selected || segment.duration >= 700_000;
  const showAudioWaveform = segment.visualKind === "audio";

  return (
    <button
      type="button"
      className={`segment-block segment-kind-${segment.visualKind}${segment.selected ? " selected" : ""}`}
      style={segmentBlockStyle(segment, timelineDuration)}
      onClick={() => onSelectSegment?.(segment.segment.segmentId)}
      aria-pressed={segment.selected}
      title={`${segment.label}，${segment.targetLabel}`}
      aria-label={`片段 ${segment.label}`}
    >
      <strong>{segment.label}</strong>
      <span className="segment-time-label">{segment.targetLabel}</span>
      {showAudioWaveform ? <AudioWaveformPlaceholder /> : null}
      {segment.segment.keyframes.length > 0 && showKeyframeStrip ? (
        <span className="segment-keyframe-strip" aria-label="关键帧标记">
          {segment.segment.keyframes.map((keyframe) => (
            <span
              key={`${keyframe.property}-${keyframe.at}`}
              className="segment-keyframe-marker"
              style={{
                left: `${(Math.max(0, Math.min(segment.duration, keyframe.at)) / Math.max(1, segment.duration)) * 100}%`
              }}
              title={`${formatKeyframeProperty(keyframe.property)}关键帧 ${formatTimelineTime(keyframe.at)} · ${formatKeyframeEasing(
                keyframe.easing
              )}`}
              aria-label={`${segment.label} ${formatKeyframeProperty(keyframe.property)}关键帧 ${formatTimelineTime(keyframe.at)}`}
            />
          ))}
        </span>
      ) : null}
    </button>
  );
}

function AudioWaveformPlaceholder(): React.ReactElement {
  return (
    <span className="audio-waveform-placeholder" aria-label="音频波形占位">
      {AUDIO_WAVEFORM_PLACEHOLDER_PATTERN.map((height, index) => (
        <span key={`${height}-${index}`} className="audio-waveform-bar" data-height={height} aria-hidden="true" />
      ))}
    </span>
  );
}

function TrackStateButton({
  label,
  symbol,
  active = false,
  disabled = false,
  onClick
}: {
  label: string;
  symbol: string;
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
      <span aria-hidden="true">{symbol}</span>
    </button>
  );
}

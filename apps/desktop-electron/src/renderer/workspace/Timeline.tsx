import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, PointerEvent as ReactPointerEvent } from "react";

import type { TrackKind } from "../../generated/Draft";
import { appIconUrls, type AppIconName } from "../assets/icons";
import {
  formatKeyframeEasing,
  formatKeyframeProperty,
  formatTimelineTime,
  segmentBlockStyle,
  type TimelineSegmentView,
  type TimelineTrackRow as TimelineTrackRowView,
  type WaveformDisplayModel,
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
  onSelectSegment?: (itemHandle: string) => void;
  onSelectTrack?: (itemHandle: string) => void;
  onAddSegment?: (materialId: string) => void;
  onAddTrack?: (trackKind: TrackKind) => void;
  onRenameTrack?: (itemHandle: string, name: string) => void;
  onSetTrackLock?: (itemHandle: string, locked: boolean) => void;
  onSetTrackVisibility?: (itemHandle: string, visible: boolean) => void;
  onMoveSelectedSegment?: (deltaUs: number) => void;
  onSplitSelectedSegment?: (splitAt: number) => void;
  onTrimSelectedSegment?: (direction: "left" | "right", deltaUs: number) => void;
  onDeleteSelectedSegment?: () => void;
  onSetTrackMute?: (itemHandle: string, muted: boolean) => void;
  onUndo?: () => void;
  onRedo?: () => void;
};

export function Timeline({
  workspace,
  playheadUs,
  playbackRunning,
  onPlayheadChange,
  onTogglePlayback,
  onSelectSegment,
  onSelectTrack,
  onAddSegment,
  onAddTrack,
  onRenameTrack,
  onSetTrackLock,
  onSetTrackVisibility,
  onMoveSelectedSegment,
  onSplitSelectedSegment,
  onTrimSelectedSegment,
  onDeleteSelectedSegment,
  onSetTrackMute,
  onUndo,
  onRedo
}: TimelineProps): React.ReactElement {
  const timeline = workspace.viewModel.timeline;
  const trackListRef = useRef<HTMLDivElement>(null);
  const trackContentRef = useRef<HTMLDivElement>(null);
  const [zoomPercent, setZoomPercent] = useState(100);
  const playheadRatio = Math.max(0, Math.min(1, Math.max(0, playheadUs) / Math.max(1, timeline.duration)));
  const playheadStyle = {
    left: `calc(${TIMELINE_HEADER_WIDTH_PX}px + ${playheadRatio * 100}% - ${TIMELINE_HEADER_WIDTH_PX * playheadRatio}px)`
  };
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

  return (
    <div className="timeline-surface">
      <TransportStrip
        workspace={workspace}
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

      <div className="track-list" aria-label="轨道列表" ref={trackListRef}>
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
              key={row.track.trackId}
              row={row}
              waveform={workspace.waveform}
              timelineDuration={timeline.duration}
              onSelectSegment={onSelectSegment}
              onSelectTrack={onSelectTrack}
              onRenameTrack={onRenameTrack}
              onSetTrackLock={onSetTrackLock}
              onSetTrackVisibility={onSetTrackVisibility}
              onSetTrackMute={onSetTrackMute}
              onMoveSelectedSegment={onMoveSelectedSegment}
              onTrimSelectedSegment={onTrimSelectedSegment}
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

function TransportStrip({
  workspace,
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
  playheadUs: number;
  playbackRunning: boolean;
  onTogglePlayback: () => void;
  onAddSegment?: (materialId: string) => void;
  onAddTrack?: (trackKind: TrackKind) => void;
  onSplitSelectedSegment?: (splitAt: number) => void;
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

  return (
    <div className="transport-strip" aria-label="时间线控制">
      <div className="timeline-tool-group transport-buttons" role="group" aria-label="播放与历史">
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
        <TimelineIconButton
          label={isPlaybackRunning ? "暂停" : "播放"}
          icon={isPlaybackRunning ? "pause" : "play"}
          onClick={togglePlayback}
          disabled={pending && !isPlaybackRunning}
        />
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
      <div className="timeline-tool-group" role="group" aria-label="添加轨道">
        <TimelineIconButton label="添加视频轨道" symbol="V+" onClick={() => onAddTrack?.("video")} disabled={pending} />
        <TimelineIconButton label="添加音频轨道" symbol="A+" onClick={() => onAddTrack?.("audio")} disabled={pending} />
        <TimelineIconButton label="添加文字轨道" symbol="T+" onClick={() => onAddTrack?.("text")} disabled={pending} />
      </div>
      <TimelineIconButton
        label="分割所选片段"
        icon="split"
        onClick={() => onSplitSelectedSegment?.(playheadUs)}
        disabled={pending || !editControls.hasSelectedSegment}
      />
      <TimelineIconButton
        label="删除所选片段"
        icon="delete"
        className="danger"
        onClick={onDeleteSelectedSegment}
        disabled={pending || !editControls.hasSelectedSegment}
      />
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
      <button
        type="button"
        className="snapping-status"
        aria-label={snappingLabel}
        aria-pressed={editControls.snappingEnabled}
        disabled
      >
        {snappingLabel}
      </button>
      <span className="playhead-time">{formatTimelineTime(playheadUs)}</span>
      <span className="timeline-status">{workspace.pendingCommand ?? "等待剪辑命令"}</span>
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
  onMoveSelectedSegment,
  onTrimSelectedSegment,
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
  onMoveSelectedSegment?: (deltaUs: number) => void;
  onTrimSelectedSegment?: (direction: "left" | "right", deltaUs: number) => void;
  pending: boolean;
}): React.ReactElement {
  const [draftName, setDraftName] = useState(row.track.name);
  const selected = row.rowClassName.includes("selected-track");
  const canToggleVisibility = row.track.kind !== "audio" && onSetTrackVisibility !== undefined;

  useEffect(() => {
    setDraftName(row.track.name);
  }, [row.track.name]);

  const commitName = useCallback((value: string) => {
    const trimmed = value.trim();
    if (trimmed.length === 0) {
      setDraftName(row.track.name);
      return;
    }
    if (trimmed !== row.track.name) {
      onRenameTrack?.(row.selectionHandle, trimmed);
    }
  }, [onRenameTrack, row.selectionHandle, row.track.name]);

  return (
    <div className={row.rowClassName}>
      <div className="track-header">
        <div className="track-header-main">
          <button
            type="button"
            className="track-target-button"
            aria-label={`选择轨道 ${row.track.name}`}
            aria-pressed={selected}
            title={`选择轨道 ${row.track.name}`}
            onClick={() => onSelectTrack?.(row.selectionHandle)}
            disabled={pending || onSelectTrack === undefined}
          >
            <span className="track-kind-symbol" aria-hidden="true">
              {row.symbol}
            </span>
          </button>
          <input
            className="track-name-input"
            aria-label={`${row.track.name} 名称`}
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
                setDraftName(row.track.name);
                event.currentTarget.blur();
              }
            }}
          />
        </div>
        <div className="track-header-controls" aria-label={`${row.track.name} 状态`}>
          <TrackStateButton
            label={`${row.track.name} 锁定状态：${row.lockLabel}`}
            symbol="锁"
            active={row.track.locked}
            disabled={pending || onSetTrackLock === undefined}
            onClick={() => onSetTrackLock?.(row.selectionHandle, !row.track.locked)}
          />
          <TrackStateButton
            label={`${row.track.name} 可见状态：${row.visibilityLabel}`}
            symbol={row.track.kind === "audio" ? "听" : "眼"}
            active={row.track.kind === "audio" ? !row.track.muted : row.track.visible}
            disabled={pending || !canToggleVisibility}
            onClick={() => onSetTrackVisibility?.(row.selectionHandle, !row.track.visible)}
          />
          <TrackStateButton
            label={`${row.track.name} 静音状态：${row.muteLabel}`}
            symbol="静"
            active={row.track.muted}
            disabled={pending || row.track.kind !== "audio" || onSetTrackMute === undefined}
            onClick={() => onSetTrackMute?.(row.selectionHandle, !row.track.muted)}
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
            waveform={waveform}
            timelineDuration={timelineDuration}
            onSelectSegment={onSelectSegment}
            onMoveSelectedSegment={onMoveSelectedSegment}
            onTrimSelectedSegment={onTrimSelectedSegment}
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
  onMoveSelectedSegment,
  onTrimSelectedSegment,
  pending
}: {
  segment: TimelineSegmentView;
  waveform: WaveformDisplayModel;
  timelineDuration: number;
  onSelectSegment?: (itemHandle: string) => void;
  onMoveSelectedSegment?: (deltaUs: number) => void;
  onTrimSelectedSegment?: (direction: "left" | "right", deltaUs: number) => void;
  pending: boolean;
}): React.ReactElement {
  const showKeyframeStrip = segment.selected || segment.duration >= 700_000;
  const showAudioWaveform = segment.visualKind === "audio";
  const dragRef = useRef<{
    mode: "move" | "trim-left" | "trim-right";
    pointerId: number;
    startClientX: number;
    laneWidth: number;
    moved: boolean;
  } | null>(null);
  const suppressClickRef = useRef(false);

  const beginPointerIntent = useCallback(
    (event: ReactPointerEvent<HTMLElement>, mode: "move" | "trim-left" | "trim-right") => {
      if (event.button !== 0 || pending) {
        return;
      }
      const lane = event.currentTarget.closest(".segment-lane");
      if (!(lane instanceof HTMLElement)) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      onSelectSegment?.(segment.selectionHandle);
      event.currentTarget.setPointerCapture(event.pointerId);
      dragRef.current = {
        mode,
        pointerId: event.pointerId,
        startClientX: event.clientX,
        laneWidth: Math.max(1, lane.getBoundingClientRect().width),
        moved: false
      };
    },
    [onSelectSegment, pending, segment.selectionHandle]
  );

  const updatePointerIntent = useCallback((event: ReactPointerEvent<HTMLElement>) => {
    const drag = dragRef.current;
    if (drag === null || drag.pointerId !== event.pointerId) {
      return;
    }
    if (Math.abs(event.clientX - drag.startClientX) >= 3) {
      drag.moved = true;
    }
  }, []);

  const completePointerIntent = useCallback(
    (event: ReactPointerEvent<HTMLElement>) => {
      const drag = dragRef.current;
      if (drag === null || drag.pointerId !== event.pointerId) {
        return;
      }
      dragRef.current = null;
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
      event.preventDefault();
      event.stopPropagation();

      const deltaUs = Math.round(((event.clientX - drag.startClientX) / drag.laneWidth) * Math.max(1, timelineDuration));
      if (!drag.moved || deltaUs === 0) {
        return;
      }
      suppressClickRef.current = true;

      if (drag.mode === "move") {
        onMoveSelectedSegment?.(deltaUs);
        return;
      }

      if (drag.mode === "trim-left" && deltaUs > 0) {
        onTrimSelectedSegment?.("left", deltaUs);
      }
      if (drag.mode === "trim-right" && deltaUs < 0) {
        onTrimSelectedSegment?.("right", Math.abs(deltaUs));
      }
    },
    [onMoveSelectedSegment, onTrimSelectedSegment, timelineDuration]
  );

  return (
    <button
      type="button"
      className={`segment-block segment-kind-${segment.visualKind}${segment.selected ? " selected" : ""}`}
      style={segmentBlockStyle(segment, timelineDuration)}
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
      <strong>{segment.label}</strong>
      <span className="segment-time-label">{segment.targetLabel}</span>
      {showAudioWaveform ? <AudioWaveform waveform={waveform} materialId={segment.segment.materialId} /> : null}
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

function AudioWaveform({
  waveform,
  materialId
}: {
  waveform: WaveformDisplayModel;
  materialId: string;
}): React.ReactElement {
  if (waveform.status === "ready" && waveform.materialId === materialId && waveform.peaks.length > 0) {
    return (
      <span className="audio-waveform-placeholder audio-waveform-ready" aria-label="音频波形" title={waveform.statusLabel}>
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
    <span className={`audio-waveform-placeholder audio-waveform-${waveform.status}`} aria-label="音频波形占位" title={waveform.statusLabel}>
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

import { useMemo, useState } from "react";

import type { SegmentId } from "../../generated/Draft";
import {
  deriveTimelineRows,
  formatTimelineTime,
  segmentBlockStyle,
  type WorkspaceState
} from "../viewModel";

type TimelineProps = {
  workspace: WorkspaceState;
  playheadUs: number;
  onPlayheadChange: (value: number) => void;
  onSelectSegment?: (segmentId: SegmentId) => void;
  onAddSegment?: (materialId: string) => void;
  onMoveSelectedSegment?: (deltaUs: number) => void;
  onSplitSelectedSegment?: (splitAt: number) => void;
  onTrimSelectedSegment?: (direction: "left" | "right", deltaUs: number) => void;
  onDeleteSelectedSegment?: () => void;
  onUndo?: () => void;
  onRedo?: () => void;
};

export function Timeline({
  workspace,
  playheadUs,
  onPlayheadChange,
  onSelectSegment,
  onAddSegment,
  onMoveSelectedSegment,
  onSplitSelectedSegment,
  onTrimSelectedSegment,
  onDeleteSelectedSegment,
  onUndo,
  onRedo
}: TimelineProps): React.ReactElement {
  const timeline = deriveTimelineRows(workspace.draft, workspace.selection);
  const playheadStyle = {
    left: `${(Math.max(0, playheadUs) / Math.max(1, timeline.duration)) * 100}%`
  };

  return (
    <div className="timeline-surface">
      <TransportStrip
        workspace={workspace}
        playheadUs={playheadUs}
        onPlayheadChange={onPlayheadChange}
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
        <div className="ruler-track">
          {timeline.rulerTicks.map((tick) => (
            <span className="ruler-tick" key={tick} style={{ left: `${(tick / timeline.duration) * 100}%` }}>
              {formatTimelineTime(tick)}
            </span>
          ))}
        </div>
      </div>

      <div className="track-list" aria-label="轨道列表">
        <div className="playhead" aria-hidden="true" style={playheadStyle} />
        {timeline.rows.map((row) => (
          <TimelineTrackRow
            key={row.track.trackId}
            row={row}
            timelineDuration={timeline.duration}
            onSelectSegment={onSelectSegment}
          />
        ))}
      </div>
    </div>
  );
}

function TransportStrip({
  workspace,
  playheadUs,
  onPlayheadChange,
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
  onPlayheadChange: (value: number) => void;
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
  const selectedMaterialId = materialId || (timelineMaterials[0]?.materialId ?? "");
  const hasSelection = workspace.selection.segmentIds.length > 0;
  const pending = workspace.pendingCommand !== null;

  return (
    <div className="transport-strip" aria-label="时间线控制">
      <div className="transport-buttons">
        <button
          type="button"
          className="transport-button"
          onClick={onUndo}
          disabled={pending || workspace.commandState.undoStack.length === 0}
        >
          撤销
        </button>
        <button
          type="button"
          className="transport-button"
          onClick={onRedo}
          disabled={pending || workspace.commandState.redoStack.length === 0}
        >
          重做
        </button>
        <button type="button" className="transport-button" disabled>
          播放
        </button>
        <button type="button" className="transport-button" onClick={() => onPlayheadChange(0)}>
          停止
        </button>
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
        className="transport-button wide"
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
      <button
        type="button"
        className="transport-button"
        onClick={() => onMoveSelectedSegment?.(-moveStepUs)}
        disabled={pending || !hasSelection}
      >
        左移
      </button>
      <button
        type="button"
        className="transport-button"
        onClick={() => onMoveSelectedSegment?.(moveStepUs)}
        disabled={pending || !hasSelection}
      >
        右移
      </button>
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
      <button
        type="button"
        className="transport-button"
        onClick={() => onSplitSelectedSegment?.(splitAtUs)}
        disabled={pending || !hasSelection}
      >
        分割
      </button>
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
      <button
        type="button"
        className="transport-button"
        onClick={() => onTrimSelectedSegment?.("left", trimStepUs)}
        disabled={pending || !hasSelection}
      >
        裁左
      </button>
      <button
        type="button"
        className="transport-button"
        onClick={() => onTrimSelectedSegment?.("right", trimStepUs)}
        disabled={pending || !hasSelection}
      >
        裁右
      </button>
      <button
        type="button"
        className="transport-button danger"
        onClick={onDeleteSelectedSegment}
        disabled={pending || !hasSelection}
      >
        删除
      </button>
      <span className="playhead-time">{formatTimelineTime(playheadUs)}</span>
      <span className="timeline-status">{workspace.pendingCommand ?? "等待剪辑命令"}</span>
    </div>
  );
}

function TimelineTrackRow({
  row,
  timelineDuration,
  onSelectSegment
}: {
  row: ReturnType<typeof deriveTimelineRows>["rows"][number];
  timelineDuration: number;
  onSelectSegment?: (segmentId: SegmentId) => void;
}): React.ReactElement {
  return (
    <div className={row.rowClassName}>
      <div className="track-header">
        <strong>{row.track.name}</strong>
        <span>
          {row.kindLabel}轨道 · {row.track.muted ? "已静音" : "未静音"}
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
  return (
    <button
      type="button"
      className={segment.selected ? "segment-block selected" : "segment-block"}
      style={segmentBlockStyle(segment, timelineDuration)}
      onClick={() => onSelectSegment?.(segment.segment.segmentId)}
      aria-pressed={segment.selected}
      aria-label={`片段 ${segment.label}`}
    >
      <strong>{segment.label}</strong>
      <span>{segment.targetLabel}</span>
    </button>
  );
}

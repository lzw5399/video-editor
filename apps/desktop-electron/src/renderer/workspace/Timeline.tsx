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
};

export function Timeline({
  workspace,
  playheadUs,
  onPlayheadChange,
  onSelectSegment
}: TimelineProps): React.ReactElement {
  const timeline = deriveTimelineRows(workspace.draft, workspace.selection);
  const playheadStyle = {
    left: `${(Math.max(0, playheadUs) / Math.max(1, timeline.duration)) * 100}%`
  };

  return (
    <div className="timeline-surface">
      <TransportStrip workspace={workspace} playheadUs={playheadUs} onPlayheadChange={onPlayheadChange} />

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
  onPlayheadChange
}: {
  workspace: WorkspaceState;
  playheadUs: number;
  onPlayheadChange: (value: number) => void;
}): React.ReactElement {
  return (
    <div className="transport-strip" aria-label="时间线控制">
      <div className="transport-buttons">
        <button type="button" className="transport-button" disabled>
          撤销
        </button>
        <button type="button" className="transport-button" disabled>
          重做
        </button>
        <button type="button" className="transport-button" disabled>
          播放
        </button>
        <button type="button" className="transport-button" onClick={() => onPlayheadChange(0)}>
          停止
        </button>
      </div>
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

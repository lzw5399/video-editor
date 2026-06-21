import { useMemo, useState } from "react";

import type { Material, TextSegment } from "../../generated/Draft";
import {
  formatMaterialDetail,
  formatMaterialDiagnostic,
  formatMaterialKind,
  formatMaterialStatus,
  formatMicroseconds,
  materialStatusMessage,
  type MaterialResourceStatusView,
  type ResourcePanelState,
  type ResourceStatusTone,
  type WorkspaceCategory,
  type WorkspaceState
} from "../viewModel";

type FeaturePanelProps = {
  category: WorkspaceCategory;
  workspace: WorkspaceState;
  showDeveloperDiagnostics: boolean;
  bundlePath: string;
  materialPath: string;
  onBundlePathChange: (value: string) => void;
  onMaterialPathChange: (value: string) => void;
  onImportMaterial: () => void;
  onImportMaterialFromPath: () => void;
  onRefreshMaterials: () => void;
  onListMissingMaterials: () => void;
  onRefreshArtifactStatus: () => void;
  onCancelArtifactGeneration: (jobId: string) => void;
  onRetryArtifactGeneration: (jobId: string) => void;
  onResumeArtifactGeneration: (jobId: string) => void;
  onPrepareArtifactCleanup: () => void;
  onConfirmArtifactCleanup: () => void;
  onDismissResourceNotice: () => void;
  onSelectAudioOutputDevice: (deviceSelectionId: string) => void;
  onAddTimelineSegment: (materialId: string) => void;
  onAddTextSegment: (text: TextSegment, durationUs: number) => void;
  onImportSubtitleSrt: (srtContent: string, timeOffsetUs: number, textTemplate: TextSegment) => void;
  onAddAudioSegment: (materialId: string, durationUs: number) => void;
  onSetSelectedSegmentVolume: (levelMillis: number) => void;
  onUpdateSelectedSegmentAudio: (options: {
    gainMillis: number;
    panBalanceMillis: number;
    fadeInDuration: number;
    fadeOutDuration: number;
  }) => void;
  onSetSelectedTrackMute: (itemHandle: string, muted: boolean) => void;
};

type MaterialFilter = "全部" | "视频" | "图片" | "音频" | "丢失";

const MATERIAL_FILTERS: readonly MaterialFilter[] = ["全部", "视频", "图片", "音频", "丢失"];

export function FeaturePanel(props: FeaturePanelProps): React.ReactElement {
  let content: React.ReactElement;

  if (props.category === "媒体") {
    content = <MaterialPanel {...props} />;
  } else if (props.category === "文字") {
    content = <TextPanel {...props} />;
  } else if (props.category === "字幕") {
    content = <CaptionsPanel {...props} />;
  } else if (props.category === "音频") {
    content = <AudioPanel {...props} />;
  } else {
    content = <DeferredCategoryPanel category={props.category} />;
  }

  return (
    <div className="resource-panel-shell">
      <div className="resource-content-panel">{content}</div>
    </div>
  );
}

function MaterialPanel({
  workspace,
  showDeveloperDiagnostics,
  bundlePath,
  materialPath,
  onBundlePathChange,
  onMaterialPathChange,
  onImportMaterial,
  onImportMaterialFromPath,
  onRefreshMaterials,
  onListMissingMaterials,
  onRefreshArtifactStatus,
  onCancelArtifactGeneration,
  onRetryArtifactGeneration,
  onResumeArtifactGeneration,
  onPrepareArtifactCleanup,
  onConfirmArtifactCleanup,
  onDismissResourceNotice,
  onAddTimelineSegment
}: FeaturePanelProps): React.ReactElement {
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<MaterialFilter>("全部");
  const filteredMaterials = useMemo(
    () =>
      workspace.materials.filter((material) => {
        const matchesSearch =
          search.trim().length === 0 ||
          material.displayName.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase()) ||
          material.uri.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase());
        const matchesFilter =
          filter === "全部" ||
          (filter === "视频" && material.kind === "video") ||
          (filter === "图片" && material.kind === "image") ||
          (filter === "音频" && material.kind === "audio") ||
          (filter === "丢失" && material.status !== "available");

        return matchesSearch && matchesFilter;
      }),
    [filter, search, workspace.materials]
  );

  return (
    <div className="feature-panel-content">
      <div className="panel-header">
        <h2>媒体</h2>
        <button type="button" className="primary-action" onClick={onImportMaterial} disabled={workspace.pendingCommand !== null}>
          导入素材
        </button>
      </div>

      {showDeveloperDiagnostics ? (
        <div className="field-stack advanced-import-fields">
          <label className="field-row">
            <span>草稿包路径</span>
            <input value={bundlePath} onChange={(event) => onBundlePathChange(event.currentTarget.value)} />
          </label>
          <label className="field-row">
            <span>素材路径</span>
            <input value={materialPath} onChange={(event) => onMaterialPathChange(event.currentTarget.value)} />
          </label>
          <div className="button-row">
            <button
              type="button"
              className="secondary-action"
              onClick={onImportMaterialFromPath}
              disabled={workspace.pendingCommand !== null || materialPath.trim().length === 0}
            >
              导入路径
            </button>
            <button type="button" className="secondary-action" onClick={onRefreshMaterials}>
              刷新
            </button>
            <button type="button" className="secondary-action" onClick={onListMissingMaterials}>
              检查丢失
            </button>
          </div>
        </div>
      ) : null}

      <div className="media-tool-row">
        <input
          aria-label="搜索素材"
          placeholder="搜索素材"
          value={search}
          onChange={(event) => setSearch(event.currentTarget.value)}
        />
      </div>

      <div className="material-filter-bar" role="group" aria-label="素材筛选">
        {MATERIAL_FILTERS.map((value) => (
          <button
            key={value}
            type="button"
            className={filter === value ? "active" : ""}
            aria-pressed={filter === value}
            onClick={() => setFilter(value)}
          >
            {value}
          </button>
        ))}
      </div>

      {showDeveloperDiagnostics && workspace.materialDiagnostics.length > 0 ? (
        <div className="diagnostic-list" aria-label="素材诊断">
          {workspace.materialDiagnostics.map((diagnostic) => (
            <p key={`${diagnostic.materialId}-${diagnostic.kind}`}>{formatMaterialDiagnostic(diagnostic)}</p>
          ))}
        </div>
      ) : null}

      {showDeveloperDiagnostics ? (
        <ResourceTaskStrip
          resourcePanel={workspace.resourcePanel}
          pending={workspace.pendingCommand !== null}
          onRefresh={onRefreshArtifactStatus}
          onCancel={onCancelArtifactGeneration}
          onRetry={onRetryArtifactGeneration}
          onResume={onResumeArtifactGeneration}
        />
      ) : null}

      {showDeveloperDiagnostics ? (
        <ResourceMaintenance
          resourcePanel={workspace.resourcePanel}
          pending={workspace.pendingCommand !== null}
          onPrepareCleanup={onPrepareArtifactCleanup}
          onConfirmCleanup={onConfirmArtifactCleanup}
          onDismiss={onDismissResourceNotice}
        />
      ) : null}

      <MaterialList
        materials={filteredMaterials}
        resourceStatuses={workspace.resourcePanel.materials}
        pending={workspace.pendingCommand !== null}
        onAddTimelineSegment={onAddTimelineSegment}
      />
    </div>
  );
}

function TextPanel({ workspace, onAddTextSegment }: FeaturePanelProps): React.ReactElement {
  const [content, setContent] = useState("输入文字");
  const [textDurationInputSeconds, setTextDurationInputSeconds] = useState(3);
  const hasTextTrack = workspace.viewModel.timeline.capabilities.hasTextTrack;

  const text: TextSegment = useMemo(() => createDefaultTextSegment(content, "text"), [content]);

  return (
    <div className="feature-panel-content">
      <div className="panel-header">
        <h2>文字</h2>
      </div>

      <section className="function-card field-stack text-feature-card" aria-label="默认文字">
        <div className="text-card-header">
          <h3>默认文字</h3>
          <span>文字片段</span>
        </div>
        <label className="field-row">
          <span>文字内容</span>
          <textarea value={content} onChange={(event) => setContent(event.currentTarget.value)} />
        </label>
        <label className="field-row">
          <span>时长</span>
          <input
            aria-label="文字时长（秒）"
            type="number"
            min="0.1"
            step="0.1"
            value={textDurationInputSeconds}
            onChange={(event) =>
              setTextDurationInputSeconds(toPositiveSeconds(event.currentTarget.valueAsNumber, textDurationInputSeconds))
            }
          />
        </label>
        <button
          type="button"
          className="primary-action wide-action"
          onClick={() => onAddTextSegment(text, secondsToMicroseconds(textDurationInputSeconds))}
          disabled={workspace.pendingCommand !== null || !hasTextTrack}
        >
          添加文字
        </button>
      </section>

      <DeferredTextCapabilityCard title="花字" detail="暂未接入，导入后将以不支持能力报告显示。" />
      <DeferredTextCapabilityCard title="气泡" detail="暂未接入，导入后将以不支持能力报告显示。" />
    </div>
  );
}

function CaptionsPanel({ workspace, onImportSubtitleSrt }: FeaturePanelProps): React.ReactElement {
  const [srtContent, setSrtContent] = useState("1\n00:00:00,000 --> 00:00:02,000\n第一句字幕\n");
  const [subtitleOffsetSeconds, setSubtitleOffsetSeconds] = useState(0);
  const textTemplate = useMemo(() => createDefaultTextSegment("字幕", "subtitle"), []);

  return (
    <div className="feature-panel-content">
      <div className="panel-header">
        <h2>字幕</h2>
      </div>

      <section className="function-card field-stack text-feature-card" aria-label="字幕 导入字幕">
        <div className="text-card-header">
          <h3>导入字幕</h3>
          <span>SRT 字幕</span>
        </div>
        <label className="field-row">
          <span>SRT 内容</span>
          <textarea
            aria-label="SRT 内容"
            value={srtContent}
            onChange={(event) => setSrtContent(event.currentTarget.value)}
          />
        </label>
        <label className="field-row">
          <span>时间偏移</span>
          <input
            aria-label="字幕时间偏移"
            type="number"
            min="0"
            step="0.1"
            value={subtitleOffsetSeconds}
            onChange={(event) =>
              setSubtitleOffsetSeconds(toBoundedSeconds(event.currentTarget.valueAsNumber, subtitleOffsetSeconds, 0, 3_600))
            }
          />
        </label>
        <button
          type="button"
          className="primary-action wide-action"
          onClick={() => onImportSubtitleSrt(srtContent, secondsToNonNegativeMicroseconds(subtitleOffsetSeconds), textTemplate)}
          disabled={workspace.pendingCommand !== null || srtContent.trim().length === 0}
        >
          导入字幕
        </button>
      </section>
    </div>
  );
}

function createDefaultTextSegment(content: string, source: TextSegment["source"]): TextSegment {
  return {
    content,
    source,
    style: {
      font: {
        family: "Noto Sans CJK SC",
        fontRef: "font://bundled/noto-sans-cjk-sc-regular"
      },
      fontSize: 36,
      color: "#ffffff",
      alignment: "center",
      lineHeightMillis: 1200,
      letterSpacingMillis: 0,
      stroke: { color: "#000000", width: 2 },
      shadow: { color: "#222222", offsetX: 2, offsetY: 2, blur: 4 },
      background: null
    },
    textBox: {
      widthMillis: 800,
      heightMillis: 200
    },
    layoutRegion: {
      xMillis: 100,
      yMillis: 100,
      widthMillis: 800,
      heightMillis: 800
    },
    wrapping: "auto",
    bubble: null,
    effect: null
  };
}

function DeferredTextCapabilityCard({ title, detail }: { title: string; detail: string }): React.ReactElement {
  return (
    <section className="function-card text-feature-card deferred-text-card" aria-label={title}>
      <div className="text-card-header">
        <h3>{title}</h3>
        <span>暂未接入</span>
      </div>
      <p>{detail}</p>
    </section>
  );
}

function AudioPanel({
  workspace,
  onAddAudioSegment,
  onSelectAudioOutputDevice,
  onSetSelectedSegmentVolume,
  onUpdateSelectedSegmentAudio,
  onSetSelectedTrackMute
}: FeaturePanelProps): React.ReactElement {
  const audioMaterials = workspace.materials.filter((material) => material.kind === "audio" && material.status === "available");
  const [materialId, setMaterialId] = useState(audioMaterials[0]?.materialId ?? "");
  const [audioDurationInputSeconds, setAudioDurationInputSeconds] = useState(4);
  const [volumePercent, setVolumePercent] = useState(100);
  const [panPercent, setPanPercent] = useState(0);
  const [fadeInSeconds, setFadeInSeconds] = useState(0);
  const [fadeOutSeconds, setFadeOutSeconds] = useState(0);
  const selectedSegment = workspace.viewModel.selectedSegment;
  const selectedTrack = workspace.viewModel.selectedTrack;
  const hasAudioTrack = workspace.viewModel.timeline.capabilities.hasAudioTrack;
  const selectedMaterialId = materialId || (audioMaterials[0]?.materialId ?? "");

  return (
    <div className="feature-panel-content">
      <div className="panel-header">
        <h2>音频</h2>
        <button
          type="button"
          className="primary-action"
          onClick={() => onAddAudioSegment(selectedMaterialId, secondsToMicroseconds(audioDurationInputSeconds))}
          disabled={workspace.pendingCommand !== null || !hasAudioTrack || selectedMaterialId.length === 0}
        >
          添加音频
        </button>
      </div>

      <div className="function-chip-row" aria-label="音频功能">
        <span>BGM</span>
        <span>音效</span>
        <span>淡入淡出</span>
      </div>

      <div className="function-card field-stack">
        <label className="field-row">
          <span>BGM素材</span>
          <select value={selectedMaterialId} onChange={(event) => setMaterialId(event.currentTarget.value)}>
            {audioMaterials.map((material) => (
              <option key={material.materialId} value={material.materialId}>
                {material.displayName}
              </option>
            ))}
          </select>
        </label>
        <label className="field-row">
          <span>时长</span>
          <input
            aria-label="音频时长（秒）"
            type="number"
            min="0.1"
            step="0.1"
            value={audioDurationInputSeconds}
            onChange={(event) =>
              setAudioDurationInputSeconds(toPositiveSeconds(event.currentTarget.valueAsNumber, audioDurationInputSeconds))
            }
          />
        </label>
      </div>

      <div className="function-card field-stack">
        <h3>输出设备</h3>
        <label className="field-row">
          <span>输出设备</span>
          <select
            aria-label="输出设备"
            value={workspace.audioDevices.selectedDeviceId}
            onChange={(event) => onSelectAudioOutputDevice(event.currentTarget.value)}
            disabled={workspace.pendingAudioCommand !== null}
          >
            {workspace.audioDevices.devices.map((device) => (
              <option key={device.selectionId} value={device.selectionId}>
                {device.displayName}
              </option>
            ))}
          </select>
        </label>
        <p className="audio-safe-status">{workspace.audioDevices.statusLabel}</p>
      </div>

      <div className="function-card field-stack">
        <h3>音频</h3>
        <label className="field-row">
          <span>音量</span>
          <input
            type="number"
            min="0"
            max="400"
            step="5"
            value={volumePercent}
            onChange={(event) => setVolumePercent(toBoundedNumber(event.currentTarget.valueAsNumber, volumePercent, 0, 400))}
          />
        </label>
        <label className="field-row">
          <span>声像</span>
          <input
            type="range"
            min="-100"
            max="100"
            step="5"
            value={panPercent}
            onChange={(event) => setPanPercent(toBoundedNumber(event.currentTarget.valueAsNumber, panPercent, -100, 100))}
          />
        </label>
        <label className="field-row">
          <span>淡入</span>
          <input
            aria-label="淡入秒数"
            type="number"
            min="0"
            step="0.1"
            value={fadeInSeconds}
            onChange={(event) => setFadeInSeconds(toBoundedSeconds(event.currentTarget.valueAsNumber, fadeInSeconds, 0, 60))}
          />
        </label>
        <label className="field-row">
          <span>淡出</span>
          <input
            aria-label="淡出秒数"
            type="number"
            min="0"
            step="0.1"
            value={fadeOutSeconds}
            onChange={(event) => setFadeOutSeconds(toBoundedSeconds(event.currentTarget.valueAsNumber, fadeOutSeconds, 0, 60))}
          />
        </label>
        <div className="button-row">
          <button
            type="button"
            className="secondary-action"
            onClick={() =>
              onUpdateSelectedSegmentAudio({
                gainMillis: volumePercent * 10,
                panBalanceMillis: panPercent * 10,
                fadeInDuration: secondsToNonNegativeMicroseconds(fadeInSeconds),
                fadeOutDuration: secondsToNonNegativeMicroseconds(fadeOutSeconds)
              })
            }
            disabled={workspace.pendingCommand !== null || selectedSegment === null}
          >
            应用音频
          </button>
          <button
            type="button"
            className="secondary-action"
            onClick={() => selectedTrack && onSetSelectedTrackMute(selectedTrack.selectionHandle, !selectedTrack.muted)}
            disabled={workspace.pendingCommand !== null || selectedTrack === null}
          >
            {selectedTrack?.muted ? "取消轨道静音" : "轨道静音"}
          </button>
        </div>
      </div>
    </div>
  );
}

function DeferredCategoryPanel({ category }: { category: WorkspaceCategory }): React.ReactElement {
  const title = category === "数字人" ? "能力暂未开放" : `${category}暂未开放`;

  return (
    <div className="feature-panel-content">
      <div className="panel-header">
        <h2>{category}</h2>
      </div>
      <div className="empty-state deferred-category-state" aria-label={`${category}暂不可用`}>
        <strong>{title}</strong>
        <span>当前版本暂不提供该类编辑，切换分类不会修改草稿内容。</span>
      </div>
    </div>
  );
}

function MaterialList({
  materials,
  resourceStatuses,
  pending,
  onAddTimelineSegment
}: {
  materials: Material[];
  resourceStatuses: MaterialResourceStatusView[];
  pending: boolean;
  onAddTimelineSegment: (materialId: string) => void;
}): React.ReactElement {
  if (materials.length === 0) {
    return (
      <div className="empty-state">
        <strong>还没有素材</strong>
        <span>导入视频、图片或音频后，可添加到时间线开始剪辑。</span>
      </div>
    );
  }

  return (
    <div className="material-list">
      {materials.map((material) => {
        const statusMessage = materialStatusMessage(material);
        const resourceStatus = resourceStatuses.find((status) => status.materialId === material.materialId);

        return (
          <article className="material-row" aria-label={`素材 ${material.displayName}`} key={material.materialId}>
            <div className="material-thumb" aria-hidden="true">{formatMaterialKind(material.kind)}</div>
            <div className="material-copy">
              <div className="material-title">
                <strong>{material.displayName}</strong>
                <span className={`material-status ${material.status}`}>{formatMaterialStatus(material.status)}</span>
              </div>
              <div className="material-metadata">
                <span>{formatMicroseconds(material.metadata.duration)}</span>
                <span>{formatMaterialDetail(material)}</span>
              </div>
              <MaterialResourceStatusLine status={resourceStatus} />
              {statusMessage === null ? null : <p className="material-warning">{statusMessage}</p>}
            </div>
            <button
              type="button"
              className="secondary-action compact-action material-row-action"
              aria-label={`添加 ${material.displayName} 到时间线`}
              onClick={() => onAddTimelineSegment(material.materialId)}
              disabled={pending || material.status !== "available"}
            >
              添加到时间线
            </button>
          </article>
        );
      })}
    </div>
  );
}

function ResourceTaskStrip({
  resourcePanel,
  pending,
  onRefresh,
  onCancel,
  onRetry,
  onResume
}: {
  resourcePanel: ResourcePanelState;
  pending: boolean;
  onRefresh: () => void;
  onCancel: (jobId: string) => void;
  onRetry: (jobId: string) => void;
  onResume: (jobId: string) => void;
}): React.ReactElement {
  const visibleTasks = resourcePanel.tasks.slice(0, 3);
  const overflowCount = Math.max(0, resourcePanel.tasks.length - visibleTasks.length);

  return (
    <section className="resource-task-strip" aria-label="资源任务">
      <div className="resource-section-header">
        <h3>资源任务</h3>
        <span>{resourcePanel.statusLabel}</span>
        <button type="button" className="compact-action" onClick={onRefresh} disabled={pending || !resourcePanel.refreshAvailable}>
          更新状态
        </button>
      </div>
      {visibleTasks.length === 0 ? (
        <p className="resource-empty">
          <strong>暂无资源任务</strong>
          <span>导入素材或请求预览后，会显示缩略图、波形、代理和预览资源状态。</span>
        </p>
      ) : (
        <div className="resource-task-list">
          {visibleTasks.map((task) => (
            <div className="resource-task-row" key={task.jobId}>
              <div className="resource-task-copy">
                <strong title={task.label}>{task.label}</strong>
                <span className={`resource-tone-${task.tone}`}>{task.statusLabel}</span>
              </div>
              <ResourceProgress value={task.progressPerMille} />
              <div className="resource-task-actions">
                {task.canCancel ? (
                  <button type="button" aria-label="取消生成" title="取消生成" onClick={() => onCancel(task.jobId)} disabled={pending}>
                    取消
                  </button>
                ) : null}
                {task.canRetry ? (
                  <button type="button" aria-label="重新生成" title="重新生成" onClick={() => onRetry(task.jobId)} disabled={pending}>
                    重试
                  </button>
                ) : null}
                {task.canResume ? (
                  <button type="button" aria-label="继续生成" title="继续生成" onClick={() => onResume(task.jobId)} disabled={pending}>
                    继续
                  </button>
                ) : null}
              </div>
            </div>
          ))}
          {overflowCount > 0 ? <span className="resource-overflow">另有 {overflowCount} 个资源任务</span> : null}
        </div>
      )}
    </section>
  );
}

function MaterialResourceStatusLine({ status }: { status: MaterialResourceStatusView | undefined }): React.ReactElement {
  const chips =
    status?.chips.length === 0 || status === undefined
      ? [
          { key: "thumbnail", label: "缩略图", statusLabel: "等待生成", tone: "warning" as const, progressPerMille: null },
          { key: "waveform", label: "波形", statusLabel: "等待生成", tone: "warning" as const, progressPerMille: null },
          { key: "proxy", label: "代理", statusLabel: "等待生成", tone: "warning" as const, progressPerMille: null }
        ]
      : status.chips;

  return (
    <div className="material-resource-status" aria-label="素材资源状态">
      {chips.map((chip) => (
        <span className={`resource-chip resource-tone-${chip.tone}`} key={chip.key}>
          <strong>{chip.label}</strong>
          <em>{chip.statusLabel}</em>
          {chip.progressPerMille === null ? null : <ResourceProgress value={chip.progressPerMille} compact />}
        </span>
      ))}
    </div>
  );
}

function ResourceMaintenance({
  resourcePanel,
  pending,
  onPrepareCleanup,
  onConfirmCleanup,
  onDismiss
}: {
  resourcePanel: ResourcePanelState;
  pending: boolean;
  onPrepareCleanup: () => void;
  onConfirmCleanup: () => void;
  onDismiss: () => void;
}): React.ReactElement {
  const maintenance = resourcePanel.maintenance;
  const summary = `${maintenance.statusLabel} · ${maintenance.usedLabel}`;

  return (
    <section className="resource-maintenance" aria-label="资源维护">
      <div className="resource-section-header">
        <h3>资源维护</h3>
        <button
          type="button"
          className="compact-action primary"
          aria-label="清理缓存"
          onClick={onPrepareCleanup}
          disabled={pending || !maintenance.cleanupAvailable}
        >
          清理缓存
        </button>
      </div>
      <p className={`resource-quota resource-tone-${maintenance.severity}`} aria-label="缓存空间状态">
        {summary}
      </p>
      <p className="resource-safe-copy">可清理 {maintenance.reclaimableLabel} · 不会删除原始素材</p>
      {resourcePanel.cleanupConfirming ? (
        <div className="resource-cleanup-confirm" aria-label="确认清理缓存">
          <span>将清理未被草稿使用的缓存，不会删除原始素材。继续清理？</span>
          <button type="button" className="compact-action primary" onClick={onConfirmCleanup} disabled={pending}>
            确认清理缓存
          </button>
        </div>
      ) : null}
      {resourcePanel.cleanupRunning ? <p className="resource-safe-copy">正在清理缓存</p> : null}
      {maintenance.resultLabel !== null ? (
        <div className="resource-maintenance-result">
          <span>{maintenance.resultLabel}</span>
          <button type="button" className="compact-action" onClick={onDismiss}>
            知道了
          </button>
        </div>
      ) : null}
      {maintenance.errorLabel !== null ? <p className="resource-maintenance-error">{maintenance.errorLabel}</p> : null}
    </section>
  );
}

function ResourceProgress({ value, compact = false }: { value: number | null; compact?: boolean }): React.ReactElement | null {
  if (value === null) {
    return null;
  }

  return <progress className={compact ? "resource-progress compact" : "resource-progress"} max={1000} value={value} />;
}

function toPositiveInteger(value: number, fallback: number): number {
  return Math.max(1, Math.round(Number.isFinite(value) ? value : fallback));
}

function toPositiveSeconds(value: number, fallback: number): number {
  return Math.max(0.1, Number.isFinite(value) ? value : fallback);
}

function secondsToMicroseconds(value: number): number {
  return toPositiveInteger(value * 1_000_000, 1_000_000);
}

function secondsToNonNegativeMicroseconds(value: number): number {
  return Math.max(0, Math.round((Number.isFinite(value) ? value : 0) * 1_000_000));
}

function toBoundedNumber(value: number, fallback: number, min: number, max: number): number {
  const rounded = Math.round(Number.isFinite(value) ? value : fallback);
  return Math.max(min, Math.min(max, rounded));
}

function toBoundedSeconds(value: number, fallback: number, min: number, max: number): number {
  const safeValue = Number.isFinite(value) ? value : fallback;
  return Math.max(min, Math.min(max, safeValue));
}

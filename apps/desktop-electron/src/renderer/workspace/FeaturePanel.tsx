import { useEffect, useMemo, useRef, useState, type CSSProperties, type DragEvent as ReactDragEvent } from "react";

import type { Material } from "../../generated/Draft";
import type {
  AdaptationCategory,
  AdaptationReport,
  AdaptationReportItem,
  AdaptationStatus,
  AdaptationTargetKind
} from "../../generated/TemplateImport";
import { appIconUrls, type AppIconName } from "../assets/icons";
import {
  WORKSPACE_CATEGORY_META,
  formatMaterialDetail,
  formatMaterialDiagnostic,
  formatMaterialKind,
  formatMaterialStatus,
  formatMicroseconds,
  materialReadyForTimeline,
  materialStatusMessage,
  type MaterialResourceStatusView,
  type ResourcePanelState,
  type ResourceStatusTone,
  type WorkspaceCategory,
  type WorkspaceState
} from "../viewModel";
import { MATERIAL_DRAG_DATA_TYPE, TEXT_SEGMENT_DRAG_DATA_TYPE } from "./dragTypes";

type FeaturePanelProps = {
  category: WorkspaceCategory;
  workspace: WorkspaceState;
  templateImportReport: AdaptationReport | null;
  showDeveloperDiagnostics: boolean;
  bundlePath: string;
  materialPath: string;
  onBundlePathChange: (value: string) => void;
  onMaterialPathChange: (value: string) => void;
  onImportMaterial: () => void;
  onImportTemplateBundle: () => void;
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
  onAddTextSegment: (content: string) => void;
  onImportSubtitleSrt: (srtContent: string) => void;
  onAddAudioSegment: (materialId: string) => void;
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
type MediaSourceSection = {
  label: string;
  active: boolean;
  disabled: boolean;
};
type ShowcaseCategory = Exclude<WorkspaceCategory, "媒体" | "音频" | "文字" | "字幕" | "模板">;
type ShowcasePanelSpec = {
  rail: readonly string[];
  cards: readonly string[];
};
type AudioPanelOptions = {
  gainMillis: number;
  panBalanceMillis: number;
  fadeInDuration: number;
  fadeOutDuration: number;
};

const MATERIAL_FILTERS: readonly MaterialFilter[] = ["全部", "视频", "图片", "音频", "丢失"];
const MEDIA_SOURCE_SECTIONS: readonly MediaSourceSection[] = [
  { label: "导入", active: true, disabled: false },
  { label: "我的", active: false, disabled: true },
  { label: "AI生成", active: false, disabled: true },
  { label: "云素材", active: false, disabled: true },
  { label: "官方素材", active: false, disabled: true },
  { label: "即梦AI", active: false, disabled: true }
];
const SHOWCASE_PANEL_SPECS: Record<ShowcaseCategory, ShowcasePanelSpec> = {
  贴纸: {
    rail: ["热门", "表情", "装饰", "收藏"],
    cards: ["基础贴纸", "情绪符号", "指示箭头", "动态装饰"]
  },
  特效: {
    rail: ["热门", "画面", "氛围", "收藏"],
    cards: ["基础光效", "速度感", "复古颗粒", "镜头闪白"]
  },
  转场: {
    rail: ["基础", "运镜", "遮罩", "收藏"],
    cards: ["叠化", "推拉", "闪白", "模糊转场"]
  },
  滤镜: {
    rail: ["人像", "风景", "电影", "收藏"],
    cards: ["自然", "清透", "胶片", "冷调"]
  },
  调节: {
    rail: ["基础", "HSL", "曲线", "LUT"],
    cards: ["自定义调节", "亮度", "对比度", "饱和度"]
  },
  数字人: {
    rail: ["形象", "声音", "口型", "收藏"],
    cards: ["数字人形象", "声音配置", "口型驱动", "场景模板"]
  }
};

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
  } else if (props.category === "模板") {
    content = <TemplatePanel {...props} />;
  } else {
    content = <ShowcaseCategoryPanel category={props.category as ShowcaseCategory} />;
  }

  return (
    <div className="resource-panel-shell">
      <div className="resource-content-panel">{content}</div>
    </div>
  );
}

const TEMPLATE_REPORT_STATUSES: readonly AdaptationStatus[] = [
  "supported",
  "approximated",
  "dropped",
  "missingResource",
  "needsNativeEffect"
];
const TEMPLATE_REPORT_STATUS_LABELS: Record<AdaptationStatus, string> = {
  supported: "已支持",
  approximated: "近似还原",
  dropped: "已舍弃",
  missingResource: "缺少资源",
  needsNativeEffect: "需本地效果"
};
const TEMPLATE_REPORT_CATEGORY_LABELS: Record<AdaptationCategory, string> = {
  sourceMedia: "源素材",
  canvas: "画布",
  material: "素材",
  track: "轨道",
  segment: "片段",
  text: "文字",
  sticker: "贴纸",
  audio: "音频",
  animation: "动画",
  transition: "转场",
  resource: "资源",
  font: "字体",
  nativeEffect: "本地效果"
};
const TEMPLATE_REPORT_TARGET_LABELS: Record<AdaptationTargetKind, string> = {
  draft: "草稿",
  canvas: "画布",
  material: "素材",
  track: "轨道",
  segment: "片段",
  text: "文本",
  sticker: "贴纸",
  audio: "音频",
  keyframe: "关键帧",
  filter: "滤镜",
  transition: "转场",
  resource: "资源",
  font: "字体",
  effect: "效果"
};

function TemplatePanel({
  workspace,
  templateImportReport,
  onImportTemplateBundle
}: FeaturePanelProps): React.ReactElement {
  const pending = workspace.pendingCommand !== null;

  return (
    <div className="feature-panel-content template-feature-panel">
      <div className="panel-header template-panel-header">
        <h2>智能包装</h2>
        <button
          type="button"
          className="primary-action template-import-action"
          aria-label="导入离线模板"
          onClick={onImportTemplateBundle}
          disabled={pending}
        >
          <span className="app-icon-mask" style={iconMaskStyle("categoryTemplate")} aria-hidden="true" />
          <span>导入模板</span>
        </button>
      </div>

      <section className="template-report-panel" aria-label="模板适配报告">
        <div className="template-report-header">
          <h3>模板适配报告</h3>
          <span>{templateImportReport === null ? "等待导入" : "本地导入"}</span>
        </div>
        <div className="template-report-summary" aria-label="适配状态统计">
          {TEMPLATE_REPORT_STATUSES.map((status) => (
            <span className={`template-report-chip status-${status}`} key={status}>
              {TEMPLATE_REPORT_STATUS_LABELS[status]} {templateImportReport?.summary[status] ?? 0}
            </span>
          ))}
        </div>
        {templateImportReport === null ? (
          <p className="template-report-empty">选择离线模板后显示适配结果。</p>
        ) : (
          <div className="template-report-list" aria-label="适配条目">
            {templateImportReport.items.slice(0, 8).map((item, index) => (
              <TemplateReportRow item={item} key={`${item.status}-${item.category}-${index}`} />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

function TemplateReportRow({ item }: { item: AdaptationReportItem }): React.ReactElement {
  return (
    <article className={`template-report-row status-${item.status}`} aria-label={`适配条目 ${TEMPLATE_REPORT_STATUS_LABELS[item.status]}`}>
      <span className="template-report-row-status">{TEMPLATE_REPORT_STATUS_LABELS[item.status]}</span>
      <div className="template-report-row-copy">
        <strong>{templateReportItemCopy(item)}</strong>
        <span>
          {TEMPLATE_REPORT_CATEGORY_LABELS[item.category]}
          {item.target?.kind === undefined ? "" : ` · ${TEMPLATE_REPORT_TARGET_LABELS[item.target.kind]}`}
        </span>
      </div>
    </article>
  );
}

function templateReportItemCopy(item: AdaptationReportItem): string {
  if (item.status === "missingResource") {
    return "资源缺失，相关片段已跳过";
  }
  if (item.status === "needsNativeEffect") {
    return "本地效果能力待补齐";
  }
  if (item.status === "supported" && item.category === "text") {
    return "文本已接入本地草稿";
  }
  if (item.status === "approximated" && item.category === "font") {
    return "字体已使用本地替代";
  }
  if (item.status === "dropped" && item.category === "text") {
    return "文字效果未写入草稿";
  }
  if (item.status === "dropped" && item.category === "segment") {
    return "片段已跳过";
  }
  if (item.status === "supported") {
    return `${TEMPLATE_REPORT_CATEGORY_LABELS[item.category]}已接入本地草稿`;
  }
  if (item.status === "approximated") {
    return `${TEMPLATE_REPORT_CATEGORY_LABELS[item.category]}已用本地能力近似还原`;
  }
  if (item.status === "dropped") {
    return `${TEMPLATE_REPORT_CATEGORY_LABELS[item.category]}未写入草稿`;
  }
  return "此项已记录";
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
    <div className="feature-panel-content media-feature-panel">
      <div className="media-panel-layout">
        <nav className="media-source-rail" aria-label="媒体来源">
          {MEDIA_SOURCE_SECTIONS.map((section) => (
            <button
              key={section.label}
              type="button"
              className={section.active ? "active" : ""}
              aria-current={section.active ? "page" : undefined}
              disabled={section.disabled}
            >
              <span className="media-source-label">{section.label}</span>
              {section.label === "即梦AI" ? null : <span className="media-source-chevron" aria-hidden="true" />}
            </button>
          ))}
        </nav>

        <div className="media-library-pane">
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

          <div className="media-toolbar" role="group" aria-label="媒体工具">
            <button
              type="button"
              className="media-import-button"
              aria-label="导入素材"
              onClick={onImportMaterial}
              disabled={workspace.pendingCommand !== null}
            >
              <span className="app-icon-mask" style={iconMaskStyle("mediaImport")} aria-hidden="true" />
              <span>导入</span>
            </button>
            <div className="media-search-row">
              <input
                aria-label="搜索素材"
                placeholder="搜索文件名"
                value={search}
                onChange={(event) => setSearch(event.currentTarget.value)}
              />
            </div>
            <button type="button" className="media-tool-icon-button active" aria-label="列表视图" aria-pressed="true">
              <span className="app-icon-mask" style={iconMaskStyle("mediaList")} aria-hidden="true" />
            </button>
            <button type="button" className="media-tool-icon-button" aria-label="高级筛选" disabled>
              <span className="app-icon-mask" style={iconMaskStyle("mediaFilter")} aria-hidden="true" />
            </button>
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
            bundlePath={bundlePath}
            materials={filteredMaterials}
            resourceStatuses={workspace.resourcePanel.materials}
            pending={workspace.pendingCommand !== null}
            showResourceDiagnostics={showDeveloperDiagnostics}
            onAddTimelineSegment={onAddTimelineSegment}
          />
        </div>
      </div>
    </div>
  );
}

function TextPanel({ workspace, onAddTextSegment }: FeaturePanelProps): React.ReactElement {
  const [content, setContent] = useState("输入文字");
  const hasTextTrack = workspace.viewModel.timeline.capabilities.hasTextTrack;
  const canAddText = workspace.pendingCommand === null && hasTextTrack && content.trim().length > 0;

  function handleTextTemplateDragStart(event: ReactDragEvent<HTMLElement>): void {
    if (!canAddText) {
      event.preventDefault();
      return;
    }

    event.dataTransfer.effectAllowed = "copy";
    event.dataTransfer.setData(TEXT_SEGMENT_DRAG_DATA_TYPE, content);
    event.dataTransfer.setData("text/plain", content);
  }

  return (
    <div className="feature-panel-content">
      <div className="panel-header">
        <h2>{WORKSPACE_CATEGORY_META["文字"].label}</h2>
      </div>

      <section className="function-card field-stack text-feature-card" aria-label="默认文字">
        <div
          className="text-card-header text-template-drag-source"
          aria-label="文字模板 默认文字"
          draggable={canAddText}
          onDragStart={handleTextTemplateDragStart}
        >
          <h3>默认文字</h3>
          <span>文字片段</span>
        </div>
        <label className="field-row">
          <span>文字内容</span>
          <textarea value={content} onChange={(event) => setContent(event.currentTarget.value)} />
        </label>
        <button
          type="button"
          className="primary-action wide-action"
          onClick={() => onAddTextSegment(content)}
          disabled={!canAddText}
        >
          添加文字
        </button>
      </section>

      <DeferredTextCapabilityCard title="花字" detail="标题、强调词和口播包装常用样式。" />
      <DeferredTextCapabilityCard title="气泡" detail="对白、标注和画面说明常用样式。" />
    </div>
  );
}

function CaptionsPanel({ workspace, onImportSubtitleSrt }: FeaturePanelProps): React.ReactElement {
  const [srtContent, setSrtContent] = useState("1\n00:00:00,000 --> 00:00:02,000\n第一句字幕\n");

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
        <button
          type="button"
          className="primary-action wide-action"
          onClick={() => onImportSubtitleSrt(srtContent)}
          disabled={workspace.pendingCommand !== null || srtContent.trim().length === 0}
        >
          导入字幕
        </button>
      </section>
    </div>
  );
}

function DeferredTextCapabilityCard({ title, detail }: { title: string; detail: string }): React.ReactElement {
  return (
    <section className="function-card text-feature-card deferred-text-card" aria-label={title}>
      <div className="text-card-header">
        <h3>{title}</h3>
        <span>模板</span>
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
  const [volumePercent, setVolumePercent] = useState(100);
  const [panPercent, setPanPercent] = useState(0);
  const [fadeInSeconds, setFadeInSeconds] = useState(0);
  const [fadeOutSeconds, setFadeOutSeconds] = useState(0);
  const selectedSegment = workspace.viewModel.selectedSegment;
  const selectedTrack = workspace.viewModel.selectedTrack;
  const hasAudioTrack = workspace.viewModel.timeline.capabilities.hasAudioTrack;
  const selectedMaterialId = materialId || (audioMaterials[0]?.materialId ?? "");
  const audioCommitKeyRef = useRef<string | null>(null);
  const audioHydrationSelectionRef = useRef<string | null>(null);
  const audioOptions = useMemo(
    () => audioPanelOptionsFromState(volumePercent, panPercent, fadeInSeconds, fadeOutSeconds),
    [fadeInSeconds, fadeOutSeconds, panPercent, volumePercent]
  );

  useEffect(() => {
    if (selectedSegment === null) {
      audioCommitKeyRef.current = null;
      audioHydrationSelectionRef.current = null;
      return;
    }

    const selectedOptions = audioPanelOptionsFromSelected(selectedSegment);
    setVolumePercent(Math.round(selectedOptions.gainMillis / 10));
    setPanPercent(Math.round(selectedOptions.panBalanceMillis / 10));
    setFadeInSeconds(microsecondsToSeconds(selectedOptions.fadeInDuration));
    setFadeOutSeconds(microsecondsToSeconds(selectedOptions.fadeOutDuration));
    audioCommitKeyRef.current = audioPanelOptionsKey(selectedOptions);
    audioHydrationSelectionRef.current = selectedSegment.selectionHandle;
  }, [
    selectedSegment?.selectionHandle,
    selectedSegment?.volume.levelMillis,
    selectedSegment?.audio?.gainMillis,
    selectedSegment?.audio?.panBalanceMillis,
    selectedSegment?.audio?.fadeInDuration.duration,
    selectedSegment?.audio?.fadeOutDuration.duration
  ]);

  useEffect(() => {
    if (selectedSegment === null) {
      return undefined;
    }
    if (audioHydrationSelectionRef.current === selectedSegment.selectionHandle) {
      audioHydrationSelectionRef.current = null;
      return undefined;
    }
    if (workspace.pendingCommand !== null) {
      return undefined;
    }
    const nextKey = audioPanelOptionsKey(audioOptions);
    if (nextKey === audioCommitKeyRef.current) {
      return undefined;
    }
    const timeout = window.setTimeout(() => {
      audioCommitKeyRef.current = nextKey;
      onUpdateSelectedSegmentAudio(audioOptions);
    }, 160);
    return () => window.clearTimeout(timeout);
  }, [audioOptions, onUpdateSelectedSegmentAudio, selectedSegment, workspace.pendingCommand]);

  return (
    <div className="feature-panel-content">
      <div className="panel-header">
        <h2>音频</h2>
        <button
          type="button"
          className="primary-action"
          onClick={() => onAddAudioSegment(selectedMaterialId)}
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

function ShowcaseCategoryPanel({ category }: { category: ShowcaseCategory }): React.ReactElement {
  const label = WORKSPACE_CATEGORY_META[category].label;
  const spec = SHOWCASE_PANEL_SPECS[category];

  return (
    <div className="feature-panel-content showcase-feature-panel">
      <div className="panel-header showcase-panel-header">
        <h2>{label}</h2>
      </div>
      <div className="showcase-panel-layout">
        <nav className="showcase-rail" aria-label={`${label}分类`}>
          {spec.rail.map((item, index) => (
            <button key={item} type="button" className={index === 0 ? "active" : ""} aria-current={index === 0 ? "page" : undefined}>
              {item}
            </button>
          ))}
        </nav>
        <div className="showcase-card-grid" aria-label={`${label}资源`}>
          {spec.cards.map((item) => (
            <article key={item} className="showcase-card" aria-label={`${label} ${item}`}>
              <span className="showcase-card-preview" aria-hidden="true" />
              <strong>{item}</strong>
            </article>
          ))}
        </div>
      </div>
    </div>
  );
}

function MaterialList({
  bundlePath,
  materials,
  resourceStatuses,
  pending,
  showResourceDiagnostics,
  onAddTimelineSegment
}: {
  bundlePath: string;
  materials: Material[];
  resourceStatuses: MaterialResourceStatusView[];
  pending: boolean;
  showResourceDiagnostics: boolean;
  onAddTimelineSegment: (materialId: string) => void;
}): React.ReactElement {
  function handleMaterialDragStart(event: ReactDragEvent<HTMLElement>, material: Material): void {
    if (!materialReadyForTimeline(material)) {
      event.preventDefault();
      return;
    }

    event.dataTransfer.effectAllowed = "copy";
    event.dataTransfer.setData(MATERIAL_DRAG_DATA_TYPE, material.materialId);
    event.dataTransfer.setData("text/plain", material.materialId);
  }

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
        const showStatusLabel = material.status !== "available";
        const resourceStatus = resourceStatuses.find((status) => status.materialId === material.materialId);
        const thumbnailUrl = materialThumbnailUrl(bundlePath, resourceStatus);

        return (
          <article
            className="material-row"
            aria-label={`素材 ${material.displayName}`}
            draggable={materialReadyForTimeline(material)}
            key={material.materialId}
            onDragStart={(event) => handleMaterialDragStart(event, material)}
          >
            <MaterialThumbnail
              material={material}
              thumbnailUrl={thumbnailUrl}
              pending={pending}
              onAddTimelineSegment={onAddTimelineSegment}
            />
            <div className="material-copy">
              <div className="material-title">
                <strong>{material.displayName}</strong>
                {showStatusLabel ? (
                  <span className={`material-status ${material.status}`}>{formatMaterialStatus(material.status)}</span>
                ) : null}
              </div>
              <div className="material-metadata">
                <span>{formatMicroseconds(material.metadata.duration)}</span>
                <span>{formatMaterialDetail(material)}</span>
              </div>
              {showResourceDiagnostics ? <MaterialResourceStatusLine status={resourceStatus} /> : null}
              {statusMessage === null ? null : <p className="material-warning">{statusMessage}</p>}
            </div>
          </article>
        );
      })}
    </div>
  );
}

function MaterialThumbnail({
  material,
  thumbnailUrl,
  pending,
  onAddTimelineSegment
}: {
  material: Material;
  thumbnailUrl: string | null;
  pending: boolean;
  onAddTimelineSegment: (materialId: string) => void;
}): React.ReactElement {
  return (
    <div className={`material-thumb material-thumb-${material.kind}`}>
      {thumbnailUrl !== null ? (
        <img src={thumbnailUrl} alt="" draggable={false} />
      ) : (
        <span aria-hidden="true">{formatMaterialKind(material.kind)}</span>
      )}
      <button
        type="button"
        className="material-add-icon-button"
        aria-label={`添加 ${material.displayName} 到时间线`}
        title={`添加 ${material.displayName} 到时间线`}
        onClick={(event) => {
          event.stopPropagation();
          onAddTimelineSegment(material.materialId);
        }}
        disabled={pending || !materialReadyForTimeline(material)}
      >
        <span className="app-icon-mask" style={iconMaskStyle("timelineAdd")} aria-hidden="true" />
      </button>
    </div>
  );
}

function materialThumbnailUrl(bundlePath: string, resourceStatus: MaterialResourceStatusView | undefined): string | null {
  const ref = resourceStatus?.thumbnailRef ?? null;
  if (ref === null || ref.artifactKind !== "thumbnail") {
    return null;
  }

  return projectRelativeFileUrl(bundlePath, ref.projectRelativeRef);
}

function projectRelativeFileUrl(bundlePath: string, projectRelativeRef: string): string | null {
  const root = bundlePath.trim();
  const ref = projectRelativeRef.trim();
  const refSegments = ref.split("/");
  if (
    root.length === 0 ||
    ref.length === 0 ||
    !root.startsWith("/") ||
    ref.startsWith("/") ||
    ref.includes("\\") ||
    refSegments.some((segment) => segment.length === 0 || segment === "." || segment === "..")
  ) {
    return null;
  }

  const rootUrl = absolutePathFileUrl(root.endsWith("/") ? root : `${root}/`);
  return new URL(refSegments.map(encodeURIComponent).join("/"), rootUrl).toString();
}

function absolutePathFileUrl(path: string): string {
  return `file://${path.split("/").map(encodeURIComponent).join("/")}`;
}

function iconMaskStyle(icon: AppIconName): CSSProperties {
  return { "--app-icon-url": `url("${appIconUrls[icon]}")` } as CSSProperties;
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

function audioPanelOptionsFromSelected(
  selectedSegment: NonNullable<WorkspaceState["viewModel"]["selectedSegment"]>
): AudioPanelOptions {
  return {
    gainMillis: selectedSegment.audio?.gainMillis ?? selectedSegment.volume.levelMillis,
    panBalanceMillis: selectedSegment.audio?.panBalanceMillis ?? 0,
    fadeInDuration: selectedSegment.audio?.fadeInDuration.duration ?? 0,
    fadeOutDuration: selectedSegment.audio?.fadeOutDuration.duration ?? 0
  };
}

function audioPanelOptionsFromState(
  volumePercent: number,
  panPercent: number,
  fadeInSeconds: number,
  fadeOutSeconds: number
): AudioPanelOptions {
  return {
    gainMillis: volumePercent * 10,
    panBalanceMillis: panPercent * 10,
    fadeInDuration: secondsToNonNegativeMicroseconds(fadeInSeconds),
    fadeOutDuration: secondsToNonNegativeMicroseconds(fadeOutSeconds)
  };
}

function audioPanelOptionsKey(options: AudioPanelOptions): string {
  return JSON.stringify(options);
}

function microsecondsToSeconds(value: number): number {
  return Math.max(0, value / 1_000_000);
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

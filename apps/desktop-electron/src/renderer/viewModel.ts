import type { CommandState, TimelineSelection } from "../generated/CommandEnvelope";
import type {
  Draft,
  Material,
  MaterialKind,
  MaterialStatus,
  Microseconds,
  TrackKind
} from "../generated/Draft";

export type WorkspaceCategory = "媒体" | "音频" | "文字" | "贴纸" | "特效" | "转场" | "滤镜" | "调节";

export const WORKSPACE_CATEGORIES: readonly WorkspaceCategory[] = [
  "媒体",
  "音频",
  "文字",
  "贴纸",
  "特效",
  "转场",
  "滤镜",
  "调节"
];

export type BindingStatus =
  | { kind: "checking"; label: string }
  | { kind: "ready"; label: string }
  | { kind: "error"; label: string };

export type WorkspaceState = {
  draft: Draft;
  commandState: CommandState;
  selection: TimelineSelection;
  materials: Material[];
  bindingStatus: BindingStatus;
  commandError: string | null;
};

export const initialWorkspaceDraft: Draft = {
  schemaVersion: 1,
  draftId: "draft-phase-04-workspace",
  metadata: {
    name: "未命名草稿",
    description: "阶段四桌面工作区展示草稿"
  },
  materials: [
    {
      materialId: "material-workspace-video",
      kind: "video",
      uri: "media/workspace-video.mp4",
      displayName: "城市街景.mp4",
      metadata: {
        duration: 12_000_000,
        width: 1920,
        height: 1080,
        frameRate: {
          numerator: 30,
          denominator: 1
        },
        hasVideo: true,
        hasAudio: true,
        audioSampleRate: 48_000,
        audioChannels: 2
      },
      status: "available"
    },
    {
      materialId: "material-workspace-audio",
      kind: "audio",
      uri: "media/bgm.wav",
      displayName: "背景音乐.wav",
      metadata: {
        duration: 18_000_000,
        hasVideo: false,
        hasAudio: true,
        audioSampleRate: 44_100,
        audioChannels: 2
      },
      status: "available"
    },
    {
      materialId: "material-workspace-missing",
      kind: "image",
      uri: "media/missing-cover.png",
      displayName: "封面图.png",
      metadata: {
        duration: 3_000_000,
        width: 1280,
        height: 720,
        hasVideo: true,
        hasAudio: false
      },
      status: "missing"
    }
  ],
  tracks: [
    {
      trackId: "track-main-video",
      kind: "video",
      name: "视频轨道 1",
      muted: false,
      locked: false,
      segments: [
        {
          segmentId: "segment-main-video",
          materialId: "material-workspace-video",
          sourceTimerange: {
            start: 0,
            duration: 8_000_000
          },
          targetTimerange: {
            start: 0,
            duration: 8_000_000
          },
          mainTrackMagnet: {
            enabled: true
          },
          keyframes: [],
          filters: [],
          transition: null,
          volume: {
            levelMillis: 1000
          }
        }
      ]
    },
    {
      trackId: "track-bgm",
      kind: "audio",
      name: "音频轨道 1",
      muted: false,
      locked: false,
      segments: [
        {
          segmentId: "segment-bgm",
          materialId: "material-workspace-audio",
          sourceTimerange: {
            start: 0,
            duration: 8_000_000
          },
          targetTimerange: {
            start: 0,
            duration: 8_000_000
          },
          mainTrackMagnet: {
            enabled: false
          },
          keyframes: [],
          filters: [],
          transition: null,
          volume: {
            levelMillis: 800
          }
        }
      ]
    }
  ]
};

export const initialCommandState: CommandState = {
  undoStack: [],
  redoStack: [],
  maxHistoryEntries: 50,
  snapping: {
    enabled: true,
    threshold: 120_000
  }
};

export const initialTimelineSelection: TimelineSelection = {
  segmentIds: [],
  trackIds: []
};

export function createInitialWorkspaceState(): WorkspaceState {
  return {
    draft: initialWorkspaceDraft,
    commandState: initialCommandState,
    selection: initialTimelineSelection,
    materials: initialWorkspaceDraft.materials,
    bindingStatus: {
      kind: "checking",
      label: "正在连接剪辑核心"
    },
    commandError: null
  };
}

export function formatMicroseconds(duration: Microseconds | null | undefined): string {
  if (duration === null || duration === undefined) {
    return "时长未知";
  }

  const totalMilliseconds = Math.max(0, Math.floor(duration / 1000));
  const milliseconds = totalMilliseconds % 1000;
  const totalSeconds = Math.floor(totalMilliseconds / 1000);
  const seconds = totalSeconds % 60;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const minutes = totalMinutes % 60;
  const hours = Math.floor(totalMinutes / 60);

  return `${padTime(hours)}:${padTime(minutes)}:${padTime(seconds)}.${milliseconds.toString().padStart(3, "0")}`;
}

export function formatMaterialKind(kind: MaterialKind): string {
  const labels: Record<MaterialKind, string> = {
    video: "视频",
    image: "图片",
    audio: "音频",
    text: "文字",
    sticker: "贴纸"
  };

  return labels[kind];
}

export function formatMaterialStatus(status: MaterialStatus): string {
  const labels: Record<MaterialStatus, string> = {
    available: "可用",
    missing: "素材丢失",
    probeFailed: "解析失败"
  };

  return labels[status];
}

export function formatTrackKind(kind: TrackKind): string {
  const labels: Record<TrackKind, string> = {
    video: "视频",
    audio: "音频",
    text: "文字",
    sticker: "贴纸",
    filter: "滤镜"
  };

  return labels[kind];
}

export function formatMaterialDetail(material: Material): string {
  const { metadata } = material;

  if (metadata.width !== null && metadata.width !== undefined && metadata.height !== null && metadata.height !== undefined) {
    return `${metadata.width} x ${metadata.height}`;
  }

  if (
    metadata.audioSampleRate !== null &&
    metadata.audioSampleRate !== undefined &&
    metadata.audioChannels !== null &&
    metadata.audioChannels !== undefined
  ) {
    return `${metadata.audioSampleRate} Hz / ${metadata.audioChannels} 声道`;
  }

  return metadata.probeError ?? "素材信息待解析";
}

export function formatCommandError(message: string): string {
  return `操作失败：${message}。请检查素材或撤销上一步后重试。`;
}

function padTime(value: number): string {
  return value.toString().padStart(2, "0");
}

import type {
  AddTimelineSegmentIntentCommandPayload,
  AddAudioSegmentIntentCommandPayload,
  AddSegmentCommandPayload,
  AddAudioSegmentCommandPayload,
  AddTextSegmentCommandPayload,
  AddTextSegmentIntentCommandPayload,
  AddTrackCommandPayload,
  AddTrackIntentCommandPayload,
  AudioPreviewCommandPayload,
  CancelExportCommandPayload,
  CommandEnvelope,
  CommandState,
  DeleteSegmentCommandPayload,
  DirtyRange,
  EditTextSegmentCommandPayload,
  ExportPreset,
  ArtifactGenerationActionCommandPayload,
  GetArtifactQuotaStatusCommandPayload,
  GetArtifactStatusCommandPayload,
  GetExportJobStatusCommandPayload,
  ImportSubtitleSrtCommandPayload,
  ImportSubtitleSrtIntentCommandPayload,
  ImportMaterialCommandPayload,
  InvalidatePreviewCacheCommandPayload,
  ListMissingMaterialsCommandPayload,
  MoveSegmentCommandPayload,
  MoveSelectedSegmentIntentCommandPayload,
  OpenProjectBundleCommandPayload,
  ProbeRuntimeCapabilitiesCommandPayload,
  PreviewCacheEntryRef,
  RedoTimelineEditCommandPayload,
  RenameTrackCommandPayload,
  RemoveSegmentKeyframeCommandPayload,
  RequestPreviewFrameCommandPayload,
  RequestPreviewSegmentCommandPayload,
  RefreshArtifactStatusCommandPayload,
  SelectTimelineSegmentsCommandPayload,
  SaveProjectBundleCommandPayload,
  SetSegmentKeyframeCommandPayload,
  SetTrackLockCommandPayload,
  SetTrackVisibilityCommandPayload,
  SplitSegmentCommandPayload,
  SplitSelectedSegmentIntentCommandPayload,
  StartExportCommandPayload,
  RunArtifactGarbageCollectionCommandPayload,
  TimelineSelection,
  TrimSegmentCommandPayload,
  TrimSelectedSegmentIntentCommandPayload,
  UndoTimelineEditCommandPayload,
  UpdateDraftCanvasConfigCommandPayload,
  UpdateSegmentAudioCommandPayload,
  UpdateSegmentVisualCommandPayload
} from "../generated/CommandEnvelope";
import type {
  CommandResultEnvelope,
  RuntimeBinaryCapability,
  RuntimeCapabilityReport,
  RuntimeFeatureCapability,
  RuntimeFontCapability,
  TimelineCommandResponse
} from "../generated/CommandResultEnvelope";
import type {
  AudioFade,
  AudioPanBalance,
  Draft,
  DraftCanvasConfig,
  Keyframe,
  KeyframeProperty,
  MaterialId,
  MaterialKind,
  Microseconds,
  SegmentId,
  SegmentVisual,
  SegmentVolume,
  SourceTimerange,
  TargetTimerange,
  TextBox,
  TextLayoutRegion,
  TextSegment,
  TextStyle,
  TextWrapping,
  TrackId,
  TrackKind
} from "../generated/Draft";
import type {
  RuntimeDiagnosticsDisplayState,
  RuntimeDiagnosticsRow,
  RuntimeDiagnosticsTone
} from "./viewModel";

export type CommandContext = {
  draft: Draft;
  commandState: CommandState;
  selection: TimelineSelection;
};

type ImportMaterialOptions = {
  draft: Draft;
  bundlePath: string;
  materialPath: string;
  materialId?: MaterialId | null;
  displayName?: string | null;
  materialKindHint?: MaterialKind | null;
};

export function buildOpenProjectBundleCommand(bundlePath: string): CommandEnvelope {
  const payload = {
    kind: "openProjectBundle",
    bundlePath
  } satisfies OpenProjectBundleCommandPayload & { kind: "openProjectBundle" };

  return envelope("openProjectBundle", payload);
}

export function buildSaveProjectBundleCommand(draft: Draft, bundlePath: string): CommandEnvelope {
  const payload = {
    kind: "saveProjectBundle",
    draft,
    bundlePath
  } satisfies SaveProjectBundleCommandPayload & { kind: "saveProjectBundle" };

  return envelope("saveProjectBundle", payload);
}

export function buildImportMaterialCommand(options: ImportMaterialOptions): CommandEnvelope {
  const payload = {
    kind: "importMaterial",
    draft: options.draft,
    bundlePath: options.bundlePath,
    materialPath: options.materialPath,
    materialId: options.materialId ?? null,
    displayName: options.displayName ?? null,
    materialKindHint: options.materialKindHint ?? null
  } satisfies ImportMaterialCommandPayload & { kind: "importMaterial" };

  return envelope("importMaterial", payload);
}

export function buildListMaterialsCommand(draft: Draft): CommandEnvelope {
  return envelope("listMaterials", {
    kind: "listMaterials",
    draft
  });
}

export function buildListMissingMaterialsCommand(draft: Draft, bundlePath: string): CommandEnvelope {
  const payload = {
    kind: "listMissingMaterials",
    draft,
    bundlePath
  } satisfies ListMissingMaterialsCommandPayload & { kind: "listMissingMaterials" };

  return envelope("listMissingMaterials", payload);
}

export function buildProbeRuntimeCapabilitiesCommand(): CommandEnvelope {
  const payload = {
    kind: "probeRuntimeCapabilities"
  } satisfies ProbeRuntimeCapabilitiesCommandPayload & { kind: "probeRuntimeCapabilities" };

  return envelope("probeRuntimeCapabilities", payload);
}

type AddSegmentOptions = {
  context: CommandContext;
  trackId: TrackId;
  segmentId: SegmentId;
  materialId: MaterialId;
  sourceTimerange: SourceTimerange;
  targetTimerange: TargetTimerange;
};

export function buildAddSegmentCommand(options: AddSegmentOptions): CommandEnvelope {
  const payload = {
    kind: "addSegment",
    draft: options.context.draft,
    commandState: options.context.commandState,
    selection: options.context.selection,
    trackId: options.trackId,
    segmentId: options.segmentId,
    materialId: options.materialId,
    sourceTimerange: options.sourceTimerange,
    targetTimerange: options.targetTimerange
  } satisfies AddSegmentCommandPayload & { kind: "addSegment" };

  return envelope("addSegment", payload);
}

export function buildAddTimelineSegmentIntentCommand(context: CommandContext, materialId: MaterialId): CommandEnvelope {
  const payload = {
    kind: "addTimelineSegmentIntent",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    materialId
  } satisfies AddTimelineSegmentIntentCommandPayload & { kind: "addTimelineSegmentIntent" };

  return envelope("addTimelineSegmentIntent", payload);
}

export function buildSelectTimelineSegmentsCommand(
  context: CommandContext,
  segmentIds: SegmentId[],
  trackIds: TrackId[]
): CommandEnvelope {
  const payload = {
    kind: "selectTimelineSegments",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentIds,
    trackIds
  } satisfies SelectTimelineSegmentsCommandPayload & { kind: "selectTimelineSegments" };

  return envelope("selectTimelineSegments", payload);
}

export function buildMoveSegmentCommand(
  context: CommandContext,
  segmentId: SegmentId,
  targetTrackId: TrackId,
  targetStart: Microseconds
): CommandEnvelope {
  const payload = {
    kind: "moveSegment",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    targetTrackId,
    targetStart
  } satisfies MoveSegmentCommandPayload & { kind: "moveSegment" };

  return envelope("moveSegment", payload);
}

export function buildMoveSelectedSegmentIntentCommand(context: CommandContext, delta: Microseconds): CommandEnvelope {
  const payload = {
    kind: "moveSelectedSegmentIntent",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    delta
  } satisfies MoveSelectedSegmentIntentCommandPayload & { kind: "moveSelectedSegmentIntent" };

  return envelope("moveSelectedSegmentIntent", payload);
}

export function buildSplitSegmentCommand(
  context: CommandContext,
  segmentId: SegmentId,
  rightSegmentId: SegmentId,
  splitAt: Microseconds
): CommandEnvelope {
  const payload = {
    kind: "splitSegment",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    rightSegmentId,
    splitAt
  } satisfies SplitSegmentCommandPayload & { kind: "splitSegment" };

  return envelope("splitSegment", payload);
}

export function buildSplitSelectedSegmentIntentCommand(context: CommandContext, splitAt: Microseconds): CommandEnvelope {
  const payload = {
    kind: "splitSelectedSegmentIntent",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    splitAt
  } satisfies SplitSelectedSegmentIntentCommandPayload & { kind: "splitSelectedSegmentIntent" };

  return envelope("splitSelectedSegmentIntent", payload);
}

export function buildTrimSegmentCommand(
  context: CommandContext,
  segmentId: SegmentId,
  direction: "left" | "right",
  targetTimerange: TargetTimerange
): CommandEnvelope {
  const payload = {
    kind: "trimSegment",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    direction,
    targetTimerange
  } satisfies TrimSegmentCommandPayload & { kind: "trimSegment" };

  return envelope("trimSegment", payload);
}

export function buildTrimSelectedSegmentIntentCommand(
  context: CommandContext,
  direction: "left" | "right",
  delta: Microseconds
): CommandEnvelope {
  const payload = {
    kind: "trimSelectedSegmentIntent",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    direction,
    delta
  } satisfies TrimSelectedSegmentIntentCommandPayload & { kind: "trimSelectedSegmentIntent" };

  return envelope("trimSelectedSegmentIntent", payload);
}

export function buildDeleteSegmentCommand(context: CommandContext, segmentId: SegmentId): CommandEnvelope {
  const payload = {
    kind: "deleteSegment",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId
  } satisfies DeleteSegmentCommandPayload & { kind: "deleteSegment" };

  return envelope("deleteSegment", payload);
}

export function buildUndoTimelineEditCommand(context: CommandContext): CommandEnvelope {
  const payload = {
    kind: "undoTimelineEdit",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection
  } satisfies UndoTimelineEditCommandPayload & { kind: "undoTimelineEdit" };

  return envelope("undoTimelineEdit", payload);
}

export function buildRedoTimelineEditCommand(context: CommandContext): CommandEnvelope {
  const payload = {
    kind: "redoTimelineEdit",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection
  } satisfies RedoTimelineEditCommandPayload & { kind: "redoTimelineEdit" };

  return envelope("redoTimelineEdit", payload);
}

type TextCommandOptions = {
  context: CommandContext;
  trackId: TrackId;
  segmentId: SegmentId;
  materialId: MaterialId;
  sourceTimerange: SourceTimerange;
  targetTimerange: TargetTimerange;
  text: TextSegment;
};

export function buildAddTextSegmentCommand(options: TextCommandOptions): CommandEnvelope {
  const payload = {
    kind: "addTextSegment",
    draft: options.context.draft,
    commandState: options.context.commandState,
    selection: options.context.selection,
    trackId: options.trackId,
    segmentId: options.segmentId,
    materialId: options.materialId,
    sourceTimerange: options.sourceTimerange,
    targetTimerange: options.targetTimerange,
    text: options.text
  } satisfies AddTextSegmentCommandPayload & { kind: "addTextSegment" };

  return envelope("addTextSegment", payload);
}

export function buildAddTextSegmentIntentCommand(
  context: CommandContext,
  text: TextSegment,
  duration: Microseconds
): CommandEnvelope {
  const payload = {
    kind: "addTextSegmentIntent",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    text,
    duration
  } satisfies AddTextSegmentIntentCommandPayload & { kind: "addTextSegmentIntent" };

  return envelope("addTextSegmentIntent", payload);
}

export function buildEditTextSegmentCommand(
  context: CommandContext,
  segmentId: SegmentId,
  text: TextSegment
): CommandEnvelope {
  const payload = {
    kind: "editTextSegment",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    text
  } satisfies EditTextSegmentCommandPayload & { kind: "editTextSegment" };

  return envelope("editTextSegment", payload);
}

type ImportSubtitleSrtOptions = {
  context: CommandContext;
  trackId: TrackId;
  trackName: string;
  srtContent: string;
  timeOffset: Microseconds;
  segmentIdPrefix: string;
  materialIdPrefix: string;
  style: TextStyle;
  textBox: TextBox;
  layoutRegion: TextLayoutRegion;
  wrapping: TextWrapping;
};

export function buildImportSubtitleSrtCommand(options: ImportSubtitleSrtOptions): CommandEnvelope {
  const payload = {
    kind: "importSubtitleSrt",
    draft: options.context.draft,
    commandState: options.context.commandState,
    selection: options.context.selection,
    trackId: options.trackId,
    trackName: options.trackName,
    srtContent: options.srtContent,
    timeOffset: options.timeOffset,
    segmentIdPrefix: options.segmentIdPrefix,
    materialIdPrefix: options.materialIdPrefix,
    style: options.style,
    textBox: options.textBox,
    layoutRegion: options.layoutRegion,
    wrapping: options.wrapping
  } satisfies ImportSubtitleSrtCommandPayload & { kind: "importSubtitleSrt" };

  return envelope("importSubtitleSrt", payload);
}

export function buildImportSubtitleSrtIntentCommand(
  context: CommandContext,
  srtContent: string,
  timeOffset: Microseconds,
  textTemplate: TextSegment
): CommandEnvelope {
  const payload = {
    kind: "importSubtitleSrtIntent",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    srtContent,
    timeOffset,
    style: textTemplate.style,
    textBox: textTemplate.textBox,
    layoutRegion: textTemplate.layoutRegion,
    wrapping: textTemplate.wrapping
  } satisfies ImportSubtitleSrtIntentCommandPayload & { kind: "importSubtitleSrtIntent" };

  return envelope("importSubtitleSrtIntent", payload);
}

type AudioCommandOptions = {
  context: CommandContext;
  trackId: TrackId;
  segmentId: SegmentId;
  materialId: MaterialId;
  sourceTimerange: SourceTimerange;
  targetTimerange: TargetTimerange;
};

export function buildAddAudioSegmentCommand(options: AudioCommandOptions): CommandEnvelope {
  const payload = {
    kind: "addAudioSegment",
    draft: options.context.draft,
    commandState: options.context.commandState,
    selection: options.context.selection,
    trackId: options.trackId,
    segmentId: options.segmentId,
    materialId: options.materialId,
    sourceTimerange: options.sourceTimerange,
    targetTimerange: options.targetTimerange
  } satisfies AddAudioSegmentCommandPayload & { kind: "addAudioSegment" };

  return envelope("addAudioSegment", payload);
}

export function buildAddAudioSegmentIntentCommand(
  context: CommandContext,
  materialId: MaterialId | null,
  duration: Microseconds | null
): CommandEnvelope {
  const payload = {
    kind: "addAudioSegmentIntent",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    materialId,
    duration
  } satisfies AddAudioSegmentIntentCommandPayload & { kind: "addAudioSegmentIntent" };

  return envelope("addAudioSegmentIntent", payload);
}

export function buildSetSegmentVolumeCommand(
  context: CommandContext,
  segmentId: SegmentId,
  volume: SegmentVolume
): CommandEnvelope {
  return envelope("setSegmentVolume", {
    kind: "setSegmentVolume",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    volume
  });
}

export function buildSetTrackMuteCommand(context: CommandContext, trackId: TrackId, muted: boolean): CommandEnvelope {
  return envelope("setTrackMute", {
    kind: "setTrackMute",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    trackId,
    muted
  });
}

export function buildAddTrackCommand(
  context: CommandContext,
  trackId: TrackId,
  trackKind: TrackKind,
  name: string
): CommandEnvelope {
  const payload = {
    kind: "addTrack",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    trackId,
    trackKind,
    name
  } satisfies AddTrackCommandPayload & { kind: "addTrack" };

  return envelope("addTrack", payload);
}

export function buildAddTrackIntentCommand(context: CommandContext, trackKind: TrackKind): CommandEnvelope {
  const payload = {
    kind: "addTrackIntent",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    trackKind
  } satisfies AddTrackIntentCommandPayload & { kind: "addTrackIntent" };

  return envelope("addTrackIntent", payload);
}

export function buildRenameTrackCommand(context: CommandContext, trackId: TrackId, name: string): CommandEnvelope {
  const payload = {
    kind: "renameTrack",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    trackId,
    name
  } satisfies RenameTrackCommandPayload & { kind: "renameTrack" };

  return envelope("renameTrack", payload);
}

export function buildSetTrackLockCommand(context: CommandContext, trackId: TrackId, locked: boolean): CommandEnvelope {
  const payload = {
    kind: "setTrackLock",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    trackId,
    locked
  } satisfies SetTrackLockCommandPayload & { kind: "setTrackLock" };

  return envelope("setTrackLock", payload);
}

export function buildSetTrackVisibilityCommand(
  context: CommandContext,
  trackId: TrackId,
  visible: boolean
): CommandEnvelope {
  const payload = {
    kind: "setTrackVisibility",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    trackId,
    visible
  } satisfies SetTrackVisibilityCommandPayload & { kind: "setTrackVisibility" };

  return envelope("setTrackVisibility", payload);
}

type UpdateSegmentAudioOptions = {
  context: CommandContext;
  segmentId: SegmentId;
  gainMillis?: number | null;
  panBalanceMillis?: AudioPanBalance | null;
  fadeInDuration?: AudioFade | null;
  fadeOutDuration?: AudioFade | null;
  effectSlots?: UpdateSegmentAudioCommandPayload["effectSlots"];
};

export function buildUpdateSegmentAudioCommand(options: UpdateSegmentAudioOptions): CommandEnvelope {
  const payload = {
    kind: "updateSegmentAudio",
    draft: options.context.draft,
    commandState: options.context.commandState,
    selection: options.context.selection,
    segmentId: options.segmentId,
    gainMillis: options.gainMillis ?? null,
    panBalanceMillis: options.panBalanceMillis ?? null,
    fadeInDuration: options.fadeInDuration ?? null,
    fadeOutDuration: options.fadeOutDuration ?? null,
    effectSlots: options.effectSlots ?? null
  } satisfies UpdateSegmentAudioCommandPayload & { kind: "updateSegmentAudio" };

  return envelope("updateSegmentAudio", payload);
}

export function buildUpdateDraftCanvasConfigCommand(
  context: CommandContext,
  canvasConfig: DraftCanvasConfig
): CommandEnvelope {
  const payload = {
    kind: "updateDraftCanvasConfig",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    canvasConfig
  } satisfies UpdateDraftCanvasConfigCommandPayload & { kind: "updateDraftCanvasConfig" };

  return envelope("updateDraftCanvasConfig", payload);
}

export function buildUpdateSegmentVisualCommand(
  context: CommandContext,
  segmentId: SegmentId,
  visual: SegmentVisual
): CommandEnvelope {
  const payload = {
    kind: "updateSegmentVisual",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    visual
  } satisfies UpdateSegmentVisualCommandPayload & { kind: "updateSegmentVisual" };

  return envelope("updateSegmentVisual", payload);
}

export function buildSetSegmentKeyframeCommand(
  context: CommandContext,
  segmentId: SegmentId,
  keyframe: Keyframe
): CommandEnvelope {
  const payload = {
    kind: "setSegmentKeyframe",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    keyframe
  } satisfies SetSegmentKeyframeCommandPayload & { kind: "setSegmentKeyframe" };

  return envelope("setSegmentKeyframe", payload);
}

export function buildRemoveSegmentKeyframeCommand(
  context: CommandContext,
  segmentId: SegmentId,
  property: KeyframeProperty,
  at: Microseconds
): CommandEnvelope {
  const payload = {
    kind: "removeSegmentKeyframe",
    draft: context.draft,
    commandState: context.commandState,
    selection: context.selection,
    segmentId,
    property,
    at
  } satisfies RemoveSegmentKeyframeCommandPayload & { kind: "removeSegmentKeyframe" };

  return envelope("removeSegmentKeyframe", payload);
}

type RequestPreviewFrameOptions = {
  draft: Draft;
  cacheRoot?: string;
  bundlePath?: string;
  targetTime: Microseconds;
};

export function buildRequestPreviewFrameCommand(options: RequestPreviewFrameOptions): CommandEnvelope {
  const payload = {
    kind: "requestPreviewFrame",
    draft: options.draft,
    ...(options.cacheRoot === undefined ? {} : { cacheRoot: options.cacheRoot }),
    ...(options.bundlePath === undefined ? {} : { bundlePath: options.bundlePath }),
    targetTime: options.targetTime
  } satisfies RequestPreviewFrameCommandPayload & { kind: "requestPreviewFrame" };

  return envelope("requestPreviewFrame", payload);
}

type RequestPreviewSegmentOptions = {
  draft: Draft;
  cacheRoot?: string;
  bundlePath?: string;
  targetTimerange: TargetTimerange;
};

export function buildRequestPreviewSegmentCommand(options: RequestPreviewSegmentOptions): CommandEnvelope {
  const payload = {
    kind: "requestPreviewSegment",
    draft: options.draft,
    ...(options.cacheRoot === undefined ? {} : { cacheRoot: options.cacheRoot }),
    ...(options.bundlePath === undefined ? {} : { bundlePath: options.bundlePath }),
    targetTimerange: options.targetTimerange
  } satisfies RequestPreviewSegmentCommandPayload & { kind: "requestPreviewSegment" };

  return envelope("requestPreviewSegment", payload);
}

type InvalidatePreviewCacheOptions = {
  entries: PreviewCacheEntryRef[];
  changedRanges: DirtyRange[];
  changedMaterialIds: MaterialId[];
  changedGraphNodeIds?: string[];
  changedDomains?: InvalidatePreviewCacheCommandPayload["changedDomains"];
  runtimeCapabilityFingerprint?: string | null;
  outputProfileFingerprint?: string | null;
  fullDraft?: boolean;
  reason: string;
  artifactSchemaVersion?: number;
  generatorVersion?: string;
};

export function buildInvalidatePreviewCacheCommand(options: InvalidatePreviewCacheOptions): CommandEnvelope {
  const payload = {
    kind: "invalidatePreviewCache",
    entries: options.entries,
    changedRanges: options.changedRanges,
    changedMaterialIds: options.changedMaterialIds,
    changedGraphNodeIds: options.changedGraphNodeIds,
    changedDomains: options.changedDomains,
    runtimeCapabilityFingerprint: options.runtimeCapabilityFingerprint,
    outputProfileFingerprint: options.outputProfileFingerprint,
    fullDraft: options.fullDraft,
    reason: options.reason,
    artifactSchemaVersion: options.artifactSchemaVersion,
    generatorVersion: options.generatorVersion
  } satisfies InvalidatePreviewCacheCommandPayload & { kind: "invalidatePreviewCache" };

  return envelope("invalidatePreviewCache", payload);
}

type AudioPreviewCommandKind =
  | "createAudioPreviewSession"
  | "playAudioPreview"
  | "pauseAudioPreview"
  | "stopAudioPreview"
  | "seekAudioPreview"
  | "cancelAudioPreview"
  | "getAudioPreviewStatus"
  | "listAudioOutputDevices"
  | "selectAudioOutputDevice"
  | "getWaveformDisplayPeaks"
  | "refreshWaveformStatus";

type AudioPreviewCommandOptions = {
  draft?: Draft | null;
  sessionId?: string | null;
  materialId?: MaterialId | null;
  targetTime?: Microseconds | null;
  targetTimerange?: TargetTimerange | null;
  playbackGeneration?: number | null;
  deviceSelectionId?: string | null;
  maxPeakBins?: number | null;
};

export function buildCreateAudioPreviewSessionCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("createAudioPreviewSession", options);
}

export function buildPlayAudioPreviewCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("playAudioPreview", options);
}

export function buildPauseAudioPreviewCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("pauseAudioPreview", options);
}

export function buildStopAudioPreviewCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("stopAudioPreview", options);
}

export function buildSeekAudioPreviewCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("seekAudioPreview", options);
}

export function buildCancelAudioPreviewCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("cancelAudioPreview", options);
}

export function buildGetAudioPreviewStatusCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("getAudioPreviewStatus", options);
}

export function buildListAudioOutputDevicesCommand(options: AudioPreviewCommandOptions = {}): CommandEnvelope {
  return buildAudioPreviewCommand("listAudioOutputDevices", options);
}

export function buildSelectAudioOutputDeviceCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("selectAudioOutputDevice", options);
}

export function buildGetWaveformDisplayPeaksCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("getWaveformDisplayPeaks", options);
}

export function buildRefreshWaveformStatusCommand(options: AudioPreviewCommandOptions): CommandEnvelope {
  return buildAudioPreviewCommand("refreshWaveformStatus", options);
}

function buildAudioPreviewCommand(kind: AudioPreviewCommandKind, options: AudioPreviewCommandOptions): CommandEnvelope {
  const payload = {
    kind,
    draft: options.draft ?? null,
    sessionId: options.sessionId ?? null,
    materialId: options.materialId ?? null,
    targetTime: options.targetTime ?? null,
    targetTimerange: options.targetTimerange ?? null,
    playbackGeneration: options.playbackGeneration ?? null,
    deviceSelectionId: options.deviceSelectionId ?? null,
    maxPeakBins: options.maxPeakBins ?? null
  } satisfies AudioPreviewCommandPayload & { kind: AudioPreviewCommandKind };

  return envelope(kind, payload);
}

type ArtifactStatusCommandOptions = {
  sessionId: string;
  bundlePath: string;
  materialId?: MaterialId | null;
};

export function buildGetArtifactStatusCommand(options: ArtifactStatusCommandOptions): CommandEnvelope {
  const payload = {
    kind: "getArtifactStatus",
    sessionId: options.sessionId,
    bundlePath: options.bundlePath,
    materialId: options.materialId ?? null
  } satisfies GetArtifactStatusCommandPayload & { kind: "getArtifactStatus" };

  return envelope("getArtifactStatus", payload);
}

export function buildRefreshArtifactStatusCommand(options: ArtifactStatusCommandOptions): CommandEnvelope {
  const payload = {
    kind: "refreshArtifactStatus",
    sessionId: options.sessionId,
    bundlePath: options.bundlePath,
    materialId: options.materialId ?? null
  } satisfies RefreshArtifactStatusCommandPayload & { kind: "refreshArtifactStatus" };

  return envelope("refreshArtifactStatus", payload);
}

type ArtifactGenerationActionOptions = {
  sessionId: string;
  bundlePath: string;
  jobId: string;
};

export function buildRetryArtifactGenerationCommand(options: ArtifactGenerationActionOptions): CommandEnvelope {
  return buildArtifactGenerationActionCommand("retryArtifactGeneration", options);
}

export function buildResumeArtifactGenerationCommand(options: ArtifactGenerationActionOptions): CommandEnvelope {
  return buildArtifactGenerationActionCommand("resumeArtifactGeneration", options);
}

export function buildCancelArtifactGenerationCommand(options: ArtifactGenerationActionOptions): CommandEnvelope {
  return buildArtifactGenerationActionCommand("cancelArtifactGeneration", options);
}

function buildArtifactGenerationActionCommand(
  kind: "retryArtifactGeneration" | "resumeArtifactGeneration" | "cancelArtifactGeneration",
  options: ArtifactGenerationActionOptions
): CommandEnvelope {
  const payload = {
    kind,
    sessionId: options.sessionId,
    bundlePath: options.bundlePath,
    jobId: options.jobId
  } satisfies ArtifactGenerationActionCommandPayload & { kind: typeof kind };

  return envelope(kind, payload);
}

export function buildGetArtifactQuotaStatusCommand(sessionId: string, bundlePath: string): CommandEnvelope {
  const payload = {
    kind: "getArtifactQuotaStatus",
    sessionId,
    bundlePath
  } satisfies GetArtifactQuotaStatusCommandPayload & { kind: "getArtifactQuotaStatus" };

  return envelope("getArtifactQuotaStatus", payload);
}

export function buildRunArtifactGarbageCollectionCommand(
  sessionId: string,
  bundlePath: string,
  dryRun: boolean
): CommandEnvelope {
  const payload = {
    kind: "runArtifactGarbageCollection",
    sessionId,
    bundlePath,
    dryRun
  } satisfies RunArtifactGarbageCollectionCommandPayload & { kind: "runArtifactGarbageCollection" };

  return envelope("runArtifactGarbageCollection", payload);
}

type StartExportOptions = {
  draft: Draft;
  outputPath: string;
  preset: ExportPreset;
  dirtyFacts?: StartExportCommandPayload["dirtyFacts"];
};

export function buildStartExportCommand(options: StartExportOptions): CommandEnvelope {
  const payload = {
    kind: "startExport",
    draft: options.draft,
    outputPath: options.outputPath,
    preset: options.preset,
    dirtyFacts: options.dirtyFacts
  } satisfies StartExportCommandPayload & { kind: "startExport" };

  return envelope("startExport", payload);
}

export function buildGetExportJobStatusCommand(jobId: string): CommandEnvelope {
  const payload = {
    kind: "getExportJobStatus",
    jobId
  } satisfies GetExportJobStatusCommandPayload & { kind: "getExportJobStatus" };

  return envelope("getExportJobStatus", payload);
}

export function buildCancelExportCommand(jobId: string): CommandEnvelope {
  const payload = {
    kind: "cancelExport",
    jobId
  } satisfies CancelExportCommandPayload & { kind: "cancelExport" };

  return envelope("cancelExport", payload);
}

export function applyTimelineCommandResult(
  current: CommandContext,
  result: CommandResultEnvelope<TimelineCommandResponse>
): { state: CommandContext; errorMessage: string | null } {
  if (!result.ok || result.data === null) {
    return {
      state: current,
      errorMessage: commandErrorMessage(result)
    };
  }

  return {
    state: {
      draft: result.data.draft,
      commandState: result.data.commandState,
      selection: result.data.selection
    },
    errorMessage: null
  };
}

export function commandErrorMessage(resultOrMessage: CommandResultEnvelope<unknown> | string): string {
  const message =
    typeof resultOrMessage === "string"
      ? resultOrMessage
      : resultOrMessage.error?.message ?? "剪辑核心返回未知错误";

  return `操作失败：${message}。请检查素材或撤销上一步后重试。`;
}

export function runtimeDiagnosticsFromReport(report: RuntimeCapabilityReport): RuntimeDiagnosticsDisplayState {
  const encoderReady = report.h264Encoder.available && report.aacEncoder.available;
  const subtitleReady = report.assFilter.available && report.subtitlesFilter.available;
  const hasBlockingRuntime = report.status === "unavailable";
  const canPreview = !hasBlockingRuntime && report.ffmpeg.status !== "unavailable" && report.ffprobe.status !== "unavailable";
  const canExport = canPreview && encoderReady;
  const status = report.status === "ready" ? "ready" : report.status === "unavailable" ? "error" : "warning";
  const diagnostics = [
    ...report.diagnostics,
    report.licensePosture.message
  ].filter((message, index, all) => message.length > 0 && all.indexOf(message) === index);

  return {
    status,
    statusLabel:
      status === "ready"
        ? "运行环境就绪"
        : status === "error"
          ? "运行环境检测失败，请检查媒体运行环境后重试。"
          : "部分能力不可用，可继续编辑，但预览或导出可能受限。",
    statusDetail:
      status === "ready"
        ? "预览和导出能力已通过剪辑核心检测。"
        : status === "error"
          ? "运行环境检测失败，请检查媒体运行环境后重试。"
        : "部分能力不可用，可继续编辑，但预览或导出可能受限。",
    packageStatusLabel: report.licensePosture.externalRuntime ? "外部运行环境" : "打包应用已就绪",
    rows: [
      binaryRow("媒体运行环境", report.ffmpeg),
      binaryRow("媒体探测环境", report.ffprobe),
      featurePairRow("编码能力", report.h264Encoder, report.aacEncoder),
      featurePairRow("字幕能力", report.assFilter, report.subtitlesFilter),
      fontRow("字体环境", report.fontReadiness),
      {
        label: "打包状态",
        value: report.licensePosture.redistributableBuild ? "可再发行构建" : "本机外部运行环境",
        detail: report.licensePosture.message,
        tone: report.licensePosture.redistributableBuild ? "ready" : "warning"
      }
    ],
    diagnostics,
    canPreview,
    canExport,
    checkedAtLabel: "刚刚检测"
  };
}

export function runtimeDiagnosticsFromError(message: string): RuntimeDiagnosticsDisplayState {
  return {
    status: "error",
    statusLabel: "运行环境检测失败，请检查媒体运行环境后重试。",
    statusDetail: message,
    packageStatusLabel: "运行环境不可用",
    rows: [
      {
        label: "媒体运行环境",
        value: message.includes("媒体运行环境") ? "未找到" : "待检测",
        detail: message,
        tone: "error"
      },
      {
        label: "媒体探测环境",
        value: message.includes("媒体探测环境") ? "未找到" : "待检测",
        detail: message,
        tone: "error"
      }
    ],
    diagnostics: [message],
    canPreview: false,
    canExport: false,
    checkedAtLabel: "检测失败"
  };
}

function binaryRow(label: string, capability: RuntimeBinaryCapability): RuntimeDiagnosticsRow {
  return {
    label,
    value: statusValue(capability.status),
    detail: [capability.path, capability.version, capability.source, capability.configureSummary, capability.diagnostic]
      .filter((value): value is string => value !== null && value !== undefined && value.length > 0)
      .join(" · "),
    tone: statusTone(capability.status)
  };
}

function featurePairRow(
  label: string,
  first: RuntimeFeatureCapability,
  second: RuntimeFeatureCapability
): RuntimeDiagnosticsRow {
  const ready = first.available && second.available;
  const detail = [featureDetail(first), featureDetail(second)].join(" · ");

  return {
    label,
    value: ready ? "可用" : "能力受限",
    detail,
    tone: ready ? "ready" : "warning"
  };
}

function fontRow(label: string, capability: RuntimeFontCapability): RuntimeDiagnosticsRow {
  return {
    label,
    value: statusValue(capability.status),
    detail:
      capability.availableFontPaths.length > 0
        ? capability.availableFontPaths.join(" · ")
        : capability.diagnostic ?? "字体环境未完全就绪，文字渲染可能与导出结果不一致。",
    tone: statusTone(capability.status)
  };
}

function featureDetail(feature: RuntimeFeatureCapability): string {
  return `${feature.name} ${feature.available ? "可用" : "不可用"}`;
}

function statusValue(status: RuntimeBinaryCapability["status"]): string {
  const labels: Record<RuntimeBinaryCapability["status"], string> = {
    ready: "可用",
    warning: "不可用",
    unavailable: "未找到"
  };

  return labels[status];
}

function statusTone(status: RuntimeBinaryCapability["status"]): RuntimeDiagnosticsTone {
  const tones: Record<RuntimeBinaryCapability["status"], RuntimeDiagnosticsTone> = {
    ready: "ready",
    warning: "warning",
    unavailable: "error"
  };

  return tones[status];
}

function envelope(command: CommandEnvelope["command"], payload: CommandEnvelope["payload"]): CommandEnvelope {
  return {
    command,
    payload,
    requestId: `${command}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`
  };
}

import type {
  AddSegmentCommandPayload,
  AddAudioSegmentCommandPayload,
  AddTextSegmentCommandPayload,
  CommandEnvelope,
  CommandState,
  DeleteSegmentCommandPayload,
  EditTextSegmentCommandPayload,
  ImportMaterialCommandPayload,
  InvalidatePreviewCacheCommandPayload,
  ListMissingMaterialsCommandPayload,
  MoveSegmentCommandPayload,
  PreviewCacheEntryRef,
  RedoTimelineEditCommandPayload,
  RequestPreviewFrameCommandPayload,
  RequestPreviewSegmentCommandPayload,
  SelectTimelineSegmentsCommandPayload,
  SplitSegmentCommandPayload,
  TimelineSelection,
  TrimSegmentCommandPayload,
  TrackId,
  UndoTimelineEditCommandPayload
} from "../generated/CommandEnvelope";
import type { CommandResultEnvelope, TimelineCommandResponse } from "../generated/CommandResultEnvelope";
import type {
  Draft,
  MaterialId,
  MaterialKind,
  Microseconds,
  SegmentId,
  SegmentVolume,
  SourceTimerange,
  TargetTimerange,
  TextSegment
} from "../generated/Draft";

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

type RequestPreviewFrameOptions = {
  draft: Draft;
  cacheRoot: string;
  targetTime: Microseconds;
};

export function buildRequestPreviewFrameCommand(options: RequestPreviewFrameOptions): CommandEnvelope {
  const payload = {
    kind: "requestPreviewFrame",
    draft: options.draft,
    cacheRoot: options.cacheRoot,
    targetTime: options.targetTime
  } satisfies RequestPreviewFrameCommandPayload & { kind: "requestPreviewFrame" };

  return envelope("requestPreviewFrame", payload);
}

type RequestPreviewSegmentOptions = {
  draft: Draft;
  cacheRoot: string;
  targetTimerange: TargetTimerange;
};

export function buildRequestPreviewSegmentCommand(options: RequestPreviewSegmentOptions): CommandEnvelope {
  const payload = {
    kind: "requestPreviewSegment",
    draft: options.draft,
    cacheRoot: options.cacheRoot,
    targetTimerange: options.targetTimerange
  } satisfies RequestPreviewSegmentCommandPayload & { kind: "requestPreviewSegment" };

  return envelope("requestPreviewSegment", payload);
}

type InvalidatePreviewCacheOptions = {
  entries: PreviewCacheEntryRef[];
  changedRanges: TargetTimerange[];
  changedMaterialIds: MaterialId[];
  reason: string;
};

export function buildInvalidatePreviewCacheCommand(options: InvalidatePreviewCacheOptions): CommandEnvelope {
  const payload = {
    kind: "invalidatePreviewCache",
    entries: options.entries,
    changedRanges: options.changedRanges,
    changedMaterialIds: options.changedMaterialIds,
    reason: options.reason
  } satisfies InvalidatePreviewCacheCommandPayload & { kind: "invalidatePreviewCache" };

  return envelope("invalidatePreviewCache", payload);
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

function envelope(command: CommandEnvelope["command"], payload: CommandEnvelope["payload"]): CommandEnvelope {
  return {
    command,
    payload,
    requestId: `${command}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`
  };
}

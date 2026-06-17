import type {
  AddAudioSegmentCommandPayload,
  AddTextSegmentCommandPayload,
  CommandEnvelope,
  CommandState,
  EditTextSegmentCommandPayload,
  ImportMaterialCommandPayload,
  ListMissingMaterialsCommandPayload,
  TimelineSelection,
  TrackId
} from "../generated/CommandEnvelope";
import type { CommandResultEnvelope, TimelineCommandResponse } from "../generated/CommandResultEnvelope";
import type {
  Draft,
  MaterialId,
  MaterialKind,
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

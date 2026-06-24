import type {
  ProjectInteractionBeginResponse,
  ProjectInteractionCancelResponse,
  ProjectInteractionCommitResponse,
  ProjectInteractionKind,
  ProjectInteractionPayload,
  ProjectInteractionUpdateResponse
} from "../../main/nativeBinding";

export type ProjectInteractionController = {
  begin: (kind: ProjectInteractionKind) => Promise<ProjectInteractionBeginResponse | null>;
  update: (
    interactionId: string,
    sequence: number,
    payload: ProjectInteractionPayload
  ) => Promise<ProjectInteractionUpdateResponse | null>;
  commit: (interactionId: string) => Promise<ProjectInteractionCommitResponse | null>;
  cancel: (interactionId: string) => Promise<ProjectInteractionCancelResponse | null>;
};

export type ProjectInteractionEvidence = {
  kind: ProjectInteractionKind;
  generation: number;
};

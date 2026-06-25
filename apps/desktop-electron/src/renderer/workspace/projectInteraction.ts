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

export const PHASE19_PROJECT_INTERACTION_KINDS = [
  "selectedSegmentRetime",
  "selectedSegmentEffect",
  "selectedSegmentMask",
  "selectedSegmentBlend",
  "selectedTransitionDuration"
] as const satisfies readonly ProjectInteractionKind[];

export type Phase19ProjectInteractionKind = (typeof PHASE19_PROJECT_INTERACTION_KINDS)[number];

export function isPhase19ProjectInteractionKind(
  kind: ProjectInteractionKind
): kind is Phase19ProjectInteractionKind {
  return PHASE19_PROJECT_INTERACTION_KINDS.includes(kind as Phase19ProjectInteractionKind);
}

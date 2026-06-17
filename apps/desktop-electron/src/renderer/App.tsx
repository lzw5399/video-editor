import { useEffect, useMemo, useState } from "react";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type { CommandResultEnvelope, ListMaterialsResponse } from "../generated/CommandResultEnvelope";
import {
  createInitialWorkspaceState,
  formatCommandError,
  initialWorkspaceDraft,
  type WorkspaceCategory,
  type WorkspaceState
} from "./viewModel";
import { WorkspaceShell } from "./workspace/WorkspaceShell";

type PingResponse = { pong: boolean };
type VersionResponse = { coreVersion: string; contractVersion: string };

type VideoEditorCoreApi = {
  ping: () => Promise<CommandResultEnvelope<PingResponse>>;
  version: () => Promise<CommandResultEnvelope<VersionResponse>>;
  executeCommand: <T = unknown>(command: CommandEnvelope) => Promise<CommandResultEnvelope<T>>;
};

declare global {
  interface Window {
    videoEditorCore: VideoEditorCoreApi;
  }
}

export function App(): React.ReactElement {
  const [workspace, setWorkspace] = useState<WorkspaceState>(() => createInitialWorkspaceState());
  const [activeCategory, setActiveCategory] = useState<WorkspaceCategory>("媒体");

  const materialListCommand = useMemo<CommandEnvelope>(
    () => ({
      command: "listMaterials",
      payload: {
        kind: "listMaterials",
        draft: initialWorkspaceDraft
      },
      requestId: "renderer-workspace-list-materials"
    }),
    []
  );

  useEffect(() => {
    let cancelled = false;

    async function bootstrapWorkspace(): Promise<void> {
      const [ping, version, materialList] = await Promise.all([
        window.videoEditorCore.ping(),
        window.videoEditorCore.version(),
        window.videoEditorCore.executeCommand<ListMaterialsResponse>(materialListCommand)
      ]);

      if (cancelled) {
        return;
      }

      if (!ping.ok || !version.ok || !materialList.ok) {
        const message =
          ping.error?.message ??
          version.error?.message ??
          materialList.error?.message ??
          "剪辑核心连接失败";
        setWorkspace((current) => ({
          ...current,
          bindingStatus: {
            kind: "error",
            label: formatCommandError(message)
          },
          commandError: formatCommandError(message)
        }));
        return;
      }

      setWorkspace((current) => ({
        ...current,
        materials: materialList.data?.materials ?? current.materials,
        bindingStatus: {
          kind: "ready",
          label: `剪辑核心已连接 ${version.data?.coreVersion ?? "0.0.0"} / 合约 ${
            version.data?.contractVersion ?? "0.0.0"
          }`
        },
        commandError: null
      }));
    }

    void bootstrapWorkspace().catch((error: unknown) => {
      if (!cancelled) {
        const message = error instanceof Error ? error.message : String(error);
        setWorkspace((current) => ({
          ...current,
          bindingStatus: {
            kind: "error",
            label: formatCommandError(message)
          },
          commandError: formatCommandError(message)
        }));
      }
    });

    return () => {
      cancelled = true;
    };
  }, [materialListCommand]);

  return <WorkspaceShell workspace={workspace} activeCategory={activeCategory} onCategoryChange={setActiveCategory} />;
}

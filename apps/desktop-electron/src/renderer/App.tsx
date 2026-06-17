import { useEffect, useMemo, useState } from "react";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type { CommandResultEnvelope, ListMaterialsResponse } from "../generated/CommandResultEnvelope";
import type { Draft, Material, Microseconds } from "../generated/Draft";

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

type SmokeState =
  | { status: "checking"; detail: string }
  | { status: "ready"; detail: string }
  | { status: "error"; detail: string };

const smokeDraft: Draft = {
  schemaVersion: 1,
  draftId: "draft-electron-smoke",
  metadata: {
    name: "Electron smoke"
  },
  materials: [
    {
      materialId: "material-electron-smoke-video",
      kind: "video",
      uri: "media/smoke-video.mp4",
      displayName: "smoke-video.mp4",
      metadata: {
        duration: 1_000_000,
        width: 320,
        height: 180,
        frameRate: {
          numerator: 30,
          denominator: 1
        },
        hasVideo: true,
        hasAudio: true,
        audioSampleRate: 44_100,
        audioChannels: 2
      },
      status: "available"
    }
  ],
  tracks: []
};

export function App(): React.ReactElement {
  const [smokeState, setSmokeState] = useState<SmokeState>({
    status: "checking",
    detail: "Binding"
  });
  const [materials, setMaterials] = useState<Material[]>([]);

  const smokeCommand = useMemo<CommandEnvelope>(
    () => ({
      command: "ping",
      payload: { kind: "ping" },
      requestId: "renderer-smoke-ping"
    }),
    []
  );
  const materialListCommand = useMemo<CommandEnvelope>(
    () => ({
      command: "listMaterials",
      payload: {
        kind: "listMaterials",
        draft: smokeDraft
      },
      requestId: "renderer-smoke-list-materials"
    }),
    []
  );

  useEffect(() => {
    let cancelled = false;

    async function runSmoke(): Promise<void> {
      const [ping, version, command, materialList] = await Promise.all([
        window.videoEditorCore.ping(),
        window.videoEditorCore.version(),
        window.videoEditorCore.executeCommand(smokeCommand),
        window.videoEditorCore.executeCommand<ListMaterialsResponse>(materialListCommand)
      ]);

      if (cancelled) {
        return;
      }

      if (!ping.ok || !version.ok || !command.ok || !materialList.ok) {
        const message =
          ping.error?.message ??
          version.error?.message ??
          command.error?.message ??
          materialList.error?.message ??
          "Binding error";
        setSmokeState({ status: "error", detail: message });
        return;
      }

      setMaterials(materialList.data?.materials ?? []);
      setSmokeState({
        status: "ready",
        detail: `Core ${version.data?.coreVersion ?? "0.0.0"} / Contract ${
          version.data?.contractVersion ?? "0.0.0"
        }`
      });
    }

    void runSmoke().catch((error: unknown) => {
      if (!cancelled) {
        setSmokeState({
          status: "error",
          detail: error instanceof Error ? error.message : String(error)
        });
      }
    });

    return () => {
      cancelled = true;
    };
  }, [materialListCommand, smokeCommand]);

  return (
    <main className="workbench" aria-label="Video editor smoke workbench">
      <header className="topbar">
        <span className="brand">Video Editor</span>
        <nav aria-label="Feature categories">
          <button type="button" className="category active">
            Media
          </button>
          <button type="button" className="category">
            Text
          </button>
          <button type="button" className="category">
            Audio
          </button>
          <button type="button" className="category">
            Effects
          </button>
        </nav>
      </header>

      <section className="media-bin" aria-label="Material bin">
        <h2>Materials</h2>
        {materials.map((material) => (
          <MaterialRow key={material.materialId} material={material} />
        ))}
      </section>

      <section className="preview-monitor" aria-label="Preview monitor">
        <div className="monitor-frame">
          <span className={`status-dot ${smokeState.status}`} />
          <strong>{smokeState.status === "ready" ? "Binding ready" : "Binding check"}</strong>
          <span>{smokeState.detail}</span>
        </div>
      </section>

      <aside className="inspector" aria-label="Inspector">
        <h2>Inspector</h2>
        <dl>
          <div>
            <dt>Draft</dt>
            <dd>Untitled</dd>
          </div>
          <div>
            <dt>Selection</dt>
            <dd>None</dd>
          </div>
        </dl>
      </aside>

      <section className="timeline" aria-label="Timeline">
        <div className="timeline-ruler" />
        <div className="track">
          <span>Video 1</span>
        </div>
        <div className="track">
          <span>Audio 1</span>
        </div>
      </section>
    </main>
  );
}

function MaterialRow({ material }: { material: Material }): React.ReactElement {
  return (
    <article className="material-row" aria-label={`Material ${material.displayName}`}>
      <div className="material-title">
        <strong>{material.displayName}</strong>
        <span>{material.kind}</span>
      </div>
      <div className="material-metadata">
        <span>{formatDuration(material.metadata.duration)}</span>
        <span>{formatStreamDetail(material)}</span>
        <span>{formatStatus(material)}</span>
      </div>
    </article>
  );
}

function formatDuration(duration: Microseconds | null | undefined): string {
  if (duration === null || duration === undefined) {
    return "duration unknown";
  }

  return `${duration.toString()} us`;
}

function formatStreamDetail(material: Material): string {
  const { metadata } = material;
  if (metadata.width !== null && metadata.width !== undefined && metadata.height !== null && metadata.height !== undefined) {
    return `${metadata.width}x${metadata.height}`;
  }

  if (
    metadata.audioSampleRate !== null &&
    metadata.audioSampleRate !== undefined &&
    metadata.audioChannels !== null &&
    metadata.audioChannels !== undefined
  ) {
    return `${metadata.audioSampleRate} Hz / ${metadata.audioChannels} ch`;
  }

  return material.metadata.probeError ?? "stream details unavailable";
}

function formatStatus(material: Material): string {
  if (material.status === "probeFailed") {
    return "probe failed";
  }

  return material.status;
}

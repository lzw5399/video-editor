import { useEffect, useMemo, useState } from "react";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type { CommandResultEnvelope } from "../generated/CommandResultEnvelope";

type PingResponse = { pong: boolean };
type VersionResponse = { coreVersion: string; contractVersion: string };

type VideoEditorCoreApi = {
  ping: () => Promise<CommandResultEnvelope<PingResponse>>;
  version: () => Promise<CommandResultEnvelope<VersionResponse>>;
  executeCommand: (command: CommandEnvelope) => Promise<CommandResultEnvelope<unknown>>;
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

export function App(): React.ReactElement {
  const [smokeState, setSmokeState] = useState<SmokeState>({
    status: "checking",
    detail: "Binding"
  });

  const smokeCommand = useMemo<CommandEnvelope>(
    () => ({
      command: "ping",
      payload: { kind: "ping" },
      requestId: "renderer-smoke-ping"
    }),
    []
  );

  useEffect(() => {
    let cancelled = false;

    async function runSmoke(): Promise<void> {
      const [ping, version, command] = await Promise.all([
        window.videoEditorCore.ping(),
        window.videoEditorCore.version(),
        window.videoEditorCore.executeCommand(smokeCommand)
      ]);

      if (cancelled) {
        return;
      }

      if (!ping.ok || !version.ok || !command.ok) {
        const message =
          ping.error?.message ?? version.error?.message ?? command.error?.message ?? "Binding error";
        setSmokeState({ status: "error", detail: message });
        return;
      }

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
  }, [smokeCommand]);

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
        <div className="material-row">Draft media</div>
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

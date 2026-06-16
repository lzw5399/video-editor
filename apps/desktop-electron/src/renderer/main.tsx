import React from "react";
import { createRoot } from "react-dom/client";

import "./styles.css";

function SmokeWorkbench(): React.ReactElement {
  return (
    <main className="workbench" aria-label="Video editor smoke workbench">
      <section className="media-bin" aria-label="Material bin">
        <h2>Media</h2>
      </section>
      <section className="preview-monitor" aria-label="Preview monitor">
        <h1>Video Editor</h1>
      </section>
      <aside className="inspector" aria-label="Inspector">
        <h2>Inspector</h2>
      </aside>
      <section className="timeline" aria-label="Timeline">
        <h2>Timeline</h2>
      </section>
    </main>
  );
}

createRoot(document.getElementById("root") as HTMLElement).render(<SmokeWorkbench />);

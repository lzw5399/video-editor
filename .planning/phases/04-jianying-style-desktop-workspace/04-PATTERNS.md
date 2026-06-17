# Phase 04: Jianying-Style Desktop Workspace - Pattern Map

**Mapped:** 2026-06-17  
**Files analyzed:** 11 new/modified files  
**Analogs found:** 11 / 11

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `apps/desktop-electron/src/renderer/App.tsx` | component | request-response | `apps/desktop-electron/src/renderer/App.tsx` | exact replace |
| `apps/desktop-electron/src/renderer/styles.css` | component | transform | `apps/desktop-electron/src/renderer/styles.css` | exact replace |
| `apps/desktop-electron/src/renderer/commandHelpers.ts` | utility | request-response | `apps/desktop-electron/src/renderer/App.tsx` + generated contracts | role-match |
| `apps/desktop-electron/src/renderer/viewModel.ts` | utility | transform | `apps/desktop-electron/src/renderer/App.tsx` formatters + `Draft.ts` | role-match |
| `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx` | component | event-driven | `apps/desktop-electron/src/renderer/App.tsx` | role-match |
| `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx` | component | request-response | `apps/desktop-electron/src/renderer/App.tsx` material list section | role-match |
| `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` | component | request-response | `apps/desktop-electron/src/renderer/App.tsx` preview monitor | role-match |
| `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` | component | request-response | `apps/desktop-electron/src/renderer/App.tsx` inspector section | role-match |
| `apps/desktop-electron/src/renderer/workspace/Timeline.tsx` | component | request-response | `CommandEnvelope.ts` + `CommandResultEnvelope.ts` | role-match |
| `apps/desktop-electron/tests/electron-smoke.spec.ts` or `workspace.spec.ts` | test | request-response | `apps/desktop-electron/tests/electron-smoke.spec.ts` | exact extend |
| `package.json`, `apps/desktop-electron/package.json`, `justfile` | config | batch | existing phase scripts in same files | exact extend |

## Pattern Assignments

### `apps/desktop-electron/src/renderer/App.tsx` (component, request-response)

**Analog:** `apps/desktop-electron/src/renderer/App.tsx`

**Generated import and bridge pattern** (lines 1-19):
```typescript
import { useEffect, useMemo, useState } from "react";

import type { CommandEnvelope } from "../generated/CommandEnvelope";
import type { CommandResultEnvelope, ListMaterialsResponse } from "../generated/CommandResultEnvelope";
import type { Draft, Material, Microseconds } from "../generated/Draft";

type VideoEditorCoreApi = {
  ping: () => Promise<CommandResultEnvelope<PingResponse>>;
  version: () => Promise<CommandResultEnvelope<VersionResponse>>;
  executeCommand: <T = unknown>(command: CommandEnvelope) => Promise<CommandResultEnvelope<T>>;
};
```

**Command construction pattern** (lines 65-83):
```typescript
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
```

**Async command/error handling pattern** (lines 85-132):
```typescript
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
  }
```

**Copy with changes:** Keep the generated type imports, global `window.videoEditorCore` declaration, cancellation guard, and `CommandResultEnvelope` checks. Replace English smoke labels and local smoke-only state with Chinese workspace state. Accepted timeline state must be assigned only from `TimelineCommandResponse`.

---

### `apps/desktop-electron/src/renderer/styles.css` (component, transform)

**Analog:** `apps/desktop-electron/src/renderer/styles.css`

**Desktop grid shell pattern** (lines 19-26):
```css
.workbench {
  display: grid;
  grid-template-columns: 280px minmax(360px, 1fr) 280px;
  grid-template-rows: 52px minmax(0, 1fr) 220px;
  height: 100vh;
  gap: 1px;
  background: #2b2a27;
}
```

**Stable topbar/category pattern** (lines 28-62):
```css
.topbar {
  grid-column: 1 / 4;
  display: flex;
  align-items: center;
  gap: 28px;
  padding: 0 18px;
  background: #20201e;
}

.category {
  height: 30px;
  padding: 0 12px;
  border: 0;
  border-radius: 6px;
  color: #cfcac0;
  background: transparent;
  font: inherit;
  font-size: 13px;
}
```

**Panel/timeline containment pattern** (lines 64-91):
```css
.media-bin,
.preview-monitor,
.inspector,
.timeline {
  min-width: 0;
  min-height: 0;
  background: #1f1f1d;
}

.timeline {
  grid-column: 1 / 4;
  display: grid;
  grid-template-rows: 26px repeat(2, 1fr);
  gap: 8px;
  padding: 14px 20px;
  background: #181818;
}
```

**Copy with changes:** Update dimensions to UI-SPEC: `300px minmax(420px, 1fr) 300px`, `52px minmax(0, 1fr) 260px`, `1280x800` and `1120x720` verified constraints, Chinese font stack, fixed row heights, and `#20c7d9` accent. Preserve full-viewport grid, `1px` dividers, `min-width: 0`, `min-height: 0`, and internal scrolling.

---

### `apps/desktop-electron/src/renderer/commandHelpers.ts` (utility, request-response)

**Analogs:** `CommandEnvelope.ts`, `CommandResultEnvelope.ts`, `App.tsx`

**Available command payloads** (`CommandEnvelope.ts` lines 5-24):
```typescript
export type CommandName = "ping" | "version" | "probeMediaRuntime" | "importMaterial" | "listMaterials" | "listMissingMaterials" | "addSegment" | "selectTimelineSegments" | "moveSegment" | "splitSegment" | "trimSegment" | "deleteSegment" | "undoTimelineEdit" | "redoTimelineEdit" | "addTextSegment" | "editTextSegment" | "addAudioSegment" | "setSegmentVolume" | "setTrackMute";
export type AddSegmentCommandPayload = { draft: Draft, commandState: CommandState, selection: TimelineSelection, trackId: TrackId, segmentId: SegmentId, materialId: MaterialId, sourceTimerange: SourceTimerange, targetTimerange: TargetTimerange, };
export type SelectTimelineSegmentsCommandPayload = { draft: Draft, commandState: CommandState, selection: TimelineSelection, segmentIds: Array<SegmentId>, trackIds: Array<TrackId>, };
export type SetSegmentVolumeCommandPayload = { draft: Draft, commandState: CommandState, selection: TimelineSelection, segmentId: SegmentId, volume: SegmentVolume, };
export type SetTrackMuteCommandPayload = { draft: Draft, commandState: CommandState, selection: TimelineSelection, trackId: TrackId, muted: boolean, };
```

**Timeline result replacement contract** (`CommandResultEnvelope.ts` lines 6-15):
```typescript
export type CommandError = { kind: CommandErrorKind, message: string, command: string | null, };
export type CommandResultEnvelope<T> = { ok: boolean, data: T | null, error: CommandError | null, events: Array<CommandEvent>, };
export type TimelineCommandResponse = { draft: Draft, commandState: CommandState, selection: TimelineSelection, events: Array<CommandEvent>, };
```

**Execute command usage** (`App.tsx` lines 89-94):
```typescript
const [ping, version, command, materialList] = await Promise.all([
  window.videoEditorCore.ping(),
  window.videoEditorCore.version(),
  window.videoEditorCore.executeCommand(smokeCommand),
  window.videoEditorCore.executeCommand<ListMaterialsResponse>(materialListCommand)
]);
```

**Copy with changes:** Centralize envelope builders and a result applier. On `ok=false` or `data=null`, preserve previous `{ draft, commandState, selection }` and return Chinese error copy: `操作失败：${message}。请检查素材或撤销上一步后重试。`

---

### `apps/desktop-electron/src/renderer/viewModel.ts` (utility, transform)

**Analogs:** `Draft.ts`, `App.tsx` material formatters

**Draft semantic shape** (`Draft.ts` lines 11-31):
```typescript
export type MaterialKind = "video" | "image" | "audio" | "text" | "sticker";
export type MaterialStatus = "available" | "missing" | "probeFailed";
export type TrackKind = "video" | "audio" | "text" | "sticker" | "filter";
export type Segment = { segmentId: SegmentId, materialId: MaterialId, sourceTimerange: SourceTimerange, targetTimerange: TargetTimerange, mainTrackMagnet: MainTrackMagnet, keyframes: Array<Keyframe>, filters: Array<Filter>, transition?: Transition | null, text?: TextSegment | null, volume: SegmentVolume, };
export type Track = { trackId: TrackId, kind: TrackKind, name: string, muted: boolean, locked: boolean, segments: Array<Segment>, };
export type Draft = { schemaVersion: DraftSchemaVersion, draftId: DraftId, metadata: DraftMetadata, materials: Array<Material>, tracks: Array<Track>, };
```

**Existing formatter pattern** (`App.tsx` lines 212-244):
```typescript
function formatDuration(duration: Microseconds | null | undefined): string {
  if (duration === null || duration === undefined) {
    return "duration unknown";
  }

  return `${duration.toString()} us`;
}

function formatStatus(material: Material): string {
  if (material.status === "probeFailed") {
    return "probe failed";
  }

  return material.status;
}
```

**Copy with changes:** Convert formatters to Chinese display values: `video -> 视频`, `audio -> 音频`, `available -> 可用`, `missing -> 素材丢失`, `probeFailed -> 解析失败`. View model functions may read `draft.tracks` and `track.segments` but must not assign, push, splice, sort, or repair semantic state.

---

### `apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx` (component, event-driven)

**Analog:** `apps/desktop-electron/src/renderer/App.tsx`

**Region layout pattern** (lines 134-192):
```tsx
<main className="workbench" aria-label="Video editor smoke workbench">
  <header className="topbar">
    <span className="brand">Video Editor</span>
    <nav aria-label="Feature categories">
      <button type="button" className="category active">
        Media
      </button>
    </nav>
  </header>

  <section className="media-bin" aria-label="Material bin">...</section>
  <section className="preview-monitor" aria-label="Preview monitor">...</section>
  <aside className="inspector" aria-label="Inspector">...</aside>
  <section className="timeline" aria-label="Timeline">...</section>
</main>
```

**Copy with changes:** Preserve the five-region structure but replace ARIA and text with Phase 4 labels: `顶部功能区`, `素材面板`, `预览窗口`, `属性检查器`, `时间线`. Categories must be exactly visible: `媒体`, `音频`, `文字`, `贴纸`, `特效`, `转场`, `滤镜`, `调节`.

---

### `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx` (component, request-response)

**Analog:** `apps/desktop-electron/src/renderer/App.tsx`

**Material list rendering pattern** (lines 154-159, 196-209):
```tsx
<section className="media-bin" aria-label="Material bin">
  <h2>Materials</h2>
  {materials.map((material) => (
    <MaterialRow key={material.materialId} material={material} />
  ))}
</section>

function MaterialRow({ material }: { material: Material }): React.ReactElement {
  return (
    <article className="material-row" aria-label={`Material ${material.displayName}`}>
      <div className="material-title">
        <strong>{material.displayName}</strong>
        <span>{material.kind}</span>
      </div>
    </article>
  );
}
```

**Copy with changes:** Use Chinese headings and article names. Add panel empty state from UI-SPEC: `还没有素材` and `导入视频、图片或音频后，可添加到时间线开始剪辑。` Material import/list actions should use `importMaterial`, `listMaterials`, and `listMissingMaterials` envelopes, not local-only material insertion.

---

### `apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx` (component, request-response)

**Analog:** `apps/desktop-electron/src/renderer/App.tsx`

**Binding status shell pattern** (lines 161-166):
```tsx
<section className="preview-monitor" aria-label="Preview monitor">
  <div className="monitor-frame">
    <span className={`status-dot ${smokeState.status}`} />
    <strong>{smokeState.status === "ready" ? "Binding ready" : "Binding check"}</strong>
    <span>{smokeState.detail}</span>
  </div>
</section>
```

**Copy with changes:** Keep monitor shell and status line, but use a 16:9 frame and Chinese placeholder `预览将在下一阶段接入`. Do not create preview caches, waveform paths, FFmpeg commands, render graphs, or playback semantics in Phase 4.

---

### `apps/desktop-electron/src/renderer/workspace/Inspector.tsx` (component, request-response)

**Analog:** `apps/desktop-electron/src/renderer/App.tsx`

**Inspector region pattern** (lines 169-181):
```tsx
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
```

**Copy with changes:** For no selection show UI-SPEC copy: `未选择片段` and `在时间线中选择一个片段后，可在这里调整文字、音量和轨道状态。` For segment selection, fields must call generated commands on explicit commit: `editTextSegment`, `setSegmentVolume`, `setTrackMute`, `trimSegment`, or `moveSegment`. Do not mutate selected segment objects directly.

---

### `apps/desktop-electron/src/renderer/workspace/Timeline.tsx` (component, request-response)

**Analogs:** `CommandEnvelope.ts`, `CommandResultEnvelope.ts`, `styles.css`

**Timeline command payloads** (`CommandEnvelope.ts` lines 12-24):
```typescript
export type AddSegmentCommandPayload = { draft: Draft, commandState: CommandState, selection: TimelineSelection, trackId: TrackId, segmentId: SegmentId, materialId: MaterialId, sourceTimerange: SourceTimerange, targetTimerange: TargetTimerange, };
export type MoveSegmentCommandPayload = { draft: Draft, commandState: CommandState, selection: TimelineSelection, segmentId: SegmentId, targetTrackId: TrackId, targetStart: Microseconds, };
export type SplitSegmentCommandPayload = { draft: Draft, commandState: CommandState, selection: TimelineSelection, segmentId: SegmentId, rightSegmentId: SegmentId, splitAt: Microseconds, };
export type TrimSegmentCommandPayload = { draft: Draft, commandState: CommandState, selection: TimelineSelection, segmentId: SegmentId, direction: TrimSegmentDirection, targetTimerange: TargetTimerange, };
export type DeleteSegmentCommandPayload = { draft: Draft, commandState: CommandState, selection: TimelineSelection, segmentId: SegmentId, };
```

**Timeline CSS base** (`styles.css` lines 84-91, 200-211):
```css
.timeline {
  grid-column: 1 / 4;
  display: grid;
  grid-template-rows: 26px repeat(2, 1fr);
  gap: 8px;
  padding: 14px 20px;
  background: #181818;
}

.timeline-ruler {
  border-bottom: 1px solid #3a3935;
}
```

**Copy with changes:** Render fixed-height rows from `draft.tracks`, segments from `track.segments`, and selection from `TimelineSelection`. Button/click actions must call command helpers and consume `TimelineCommandResponse`. The source guard should allow read-only iteration but reject `.tracks =`, `.segments =`, `push`, `splice`, `sort`, timerange assignment, and local snapping/main-track magnet computation.

---

### `apps/desktop-electron/tests/electron-smoke.spec.ts` or `workspace.spec.ts` (test, request-response)

**Analog:** `apps/desktop-electron/tests/electron-smoke.spec.ts`

**Electron launch helper** (lines 22-29):
```typescript
async function launchSmokeApp(): Promise<{ app: ElectronApplication; page: Page }> {
  const app = await electron.launch({
    args: [join(process.cwd(), "dist/main/index.cjs")]
  });
  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  return { app, page };
}
```

**Preload bridge assertions** (lines 83-100):
```typescript
await expect(page.getByRole("main", { name: "Video editor smoke workbench" })).toBeVisible();

const exposedKeys = await page.evaluate(() => Object.keys(window));
expect(exposedKeys).toContain("videoEditorCore");
expect(exposedKeys).not.toContain("ipcRenderer");

const apiShape = await page.evaluate(() => ({
  ping: typeof window.videoEditorCore?.ping,
  version: typeof window.videoEditorCore?.version,
  executeCommand: typeof window.videoEditorCore?.executeCommand,
  keys: Object.keys(window.videoEditorCore ?? {})
}));
```

**Renderer command assertion** (lines 117-130):
```typescript
const command: CommandEnvelope = {
  command: "ping",
  payload: { kind: "ping" },
  requestId: "electron-smoke-ping"
};
const result = await page.evaluate((commandEnvelope) => {
  return window.videoEditorCore?.executeCommand(commandEnvelope);
}, command);
expect(result).toEqual({
  ok: true,
  data: { pong: true },
  error: null,
  events: []
});
```

**Source guard pattern** (lines 144-148):
```typescript
test("renderer source does not construct FFmpeg or ffprobe commands", async () => {
  const source = await readFile(join(process.cwd(), "src/renderer/App.tsx"), "utf8");

  expect(source).not.toMatch(/ffmpeg|ffprobe/i);
});
```

**Copy with changes:** Keep `_electron.launch`, `firstWindow`, preload bridge assertions, and trusted/untrusted navigation checks. Update visible region assertions to Chinese names and add layout bounding-box checks at `1280x800` and `1120x720`. Add tests for material rows (`可用`, `素材丢失`, `解析失败` where fixtures permit) and at least one timeline edit whose UI update follows a `TimelineCommandResponse`.

---

### `package.json`, `apps/desktop-electron/package.json`, `justfile` (config, batch)

**Analogs:** existing root scripts and just recipes

**Root phase guard script pattern** (`package.json` lines 26-30):
```json
"test:phase3-source-guards": "bash -lc 'set -euo pipefail; ! rg -n \"media_runtime|media_runtime_desktop|project_store|preview_service|render_graph|ffmpeg_compiler|bindings_node|ffmpeg|ffprobe|electron|napi|node|std::fs|fs::|std::process\" crates/draft_commands/src crates/draft_commands/Cargo.toml; ! rg -n \"sourceTimerange|targetTimerange|mainTrackMagnet|segmentId|trackId|\\.tracks[[:space:]]*(=|\\[)|tracks\\.(push|splice|sort)|\\.segments[[:space:]]*(=|\\[)|segments\\.(push|splice|sort)\" apps/desktop-electron/src/renderer apps/desktop-electron/src/main apps/desktop-electron/src/preload; ...; git diff --exit-code schemas apps/desktop-electron/src/generated'",
"test:contracts": "git diff --exit-code schemas apps/desktop-electron/src/generated",
"test": "pnpm run test:rust && ... && pnpm run test:phase3-source-guards && pnpm run test:contracts"
```

**Desktop test script pattern** (`apps/desktop-electron/package.json` lines 8-12):
```json
"build:electron": "vite build --mode main && vite build --mode preload && vite build",
"build": "pnpm run build:native && pnpm run build:electron",
"build:native": "napi build --manifest-path ../../crates/bindings_node/Cargo.toml --platform --release --output-dir native --js index.cjs --dts index.d.ts",
"test": "pnpm run build && playwright test"
```

**Just gate pattern** (`justfile` lines 12-32):
```just
build:
  pnpm install --frozen-lockfile
  pnpm run build:rust
  pnpm --filter @video-editor/desktop build

test:
  pnpm install --frozen-lockfile
  pnpm run test:rust
  pnpm run test:desktop
  pnpm run test:phase3-source-guards
  pnpm run test:contracts
```

**Copy with changes:** Add Phase 4 scripts after existing phase scripts, not instead of them. Source guards should cover direct renderer mutation, direct Electron/Node imports in renderer, renderer FFmpeg/ffprobe construction, English-only Phase 4 labels, and generated contract drift.

## Shared Patterns

### Safe Renderer-to-Core Boundary

**Source:** `apps/desktop-electron/src/preload/index.ts` lines 7-12 and `apps/desktop-electron/src/main/index.ts` lines 15-26

```typescript
contextBridge.exposeInMainWorld("videoEditorCore", {
  ping: () => ipcRenderer.invoke("core:ping"),
  version: () => ipcRenderer.invoke("core:version"),
  executeCommand: (command: CommandEnvelope) => ipcRenderer.invoke("core:executeCommand", command)
});

ipcMain.handle("core:executeCommand", (event, command: CommandEnvelope) => {
  assertAllowedIpcSender(event);
  return executeCommand(command);
});
```

Apply to all renderer components: call `window.videoEditorCore.executeCommand`; never import `electron`, `node:*`, filesystem, or native binding modules in renderer code.

### Trusted Renderer Guard

**Source:** `apps/desktop-electron/src/main/index.ts` lines 28-57 and 89-109

```typescript
const window = new BrowserWindow({
  width: 1280,
  height: 800,
  minWidth: 960,
  minHeight: 640,
  backgroundColor: "#171717",
  webPreferences: {
    contextIsolation: true,
    nodeIntegration: false,
    sandbox: true,
    preload: join(__dirname, "../preload/index.cjs"),
    additionalArguments: [allowedRendererUrlArgument]
  }
});

function assertAllowedIpcSender(event: IpcMainInvokeEvent): void {
  const senderUrl = event.senderFrame.url;
  if (!isAllowedRendererUrl(senderUrl)) {
    throw new Error(`Rejected IPC from untrusted renderer: ${senderUrl}`);
  }
}
```

Phase 4 may raise `minWidth`/`minHeight` to match UI-SPEC if planned, but must preserve context isolation, no Node integration, sandbox, preload, and sender allowlist.

### Native Binding Error Envelope

**Source:** `apps/desktop-electron/src/main/nativeBinding.ts` lines 39-45 and 90-100

```typescript
export function executeCommand(command: CommandEnvelope): CommandResultEnvelope<unknown> {
  const binding = loadNativeBinding();
  if (binding === null) {
    return bindingLoadError(command.command);
  }
  return binding.executeCommand(command);
}

function bindingLoadError(command: string): CommandResultEnvelope<never> {
  return {
    ok: false,
    data: null,
    error: {
      kind: "internal",
      command,
      message: `Native binding failed to load: ${cachedLoadError ?? "unknown load failure"}`
    },
    events: []
  };
}
```

Renderer error handling should treat any `ok=false` envelope the same way: preserve accepted draft state and show Chinese error text.

### Generated Contracts Are Read-Only

**Source:** generated file headers and `package.json` line 29

```typescript
// This file was generated by Rust ts-rs declarations. Do not edit this file manually.
```

```json
"test:contracts": "git diff --exit-code schemas apps/desktop-electron/src/generated"
```

Planner should never assign work that edits generated TypeScript directly. If contract changes are required, they belong in Rust/schema generation work.

### No Reference Code Copying

**Source:** `AGENTS.md` constraints

Use Kdenlive/MLT/pyJianYingDraft only as conceptual vocabulary and product references. Do not copy GPL code, assets, XML definitions, presets, UI implementation, or proprietary draft IDs into Phase 4 source.

## No Analog Found

No Phase 4 planned file lacks an analog. For richer drag editing, waveform rendering, preview frames, export, stickers/effects parity, or render graph features, planner should defer rather than invent patterns in Phase 4.

## Metadata

**Analog search scope:** `apps/desktop-electron/src`, `apps/desktop-electron/tests`, `crates/bindings_node/src`, `crates/draft_commands/src`, root scripts  
**Files scanned:** 18 source/config/test files plus Phase 04 context/research/UI spec  
**Pattern extraction date:** 2026-06-17

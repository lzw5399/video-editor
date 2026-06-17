---
phase: 04-jianying-style-desktop-workspace
reviewed: 2026-06-17T11:18:54Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - apps/desktop-electron/package.json
  - apps/desktop-electron/src/renderer/App.tsx
  - apps/desktop-electron/src/renderer/commandHelpers.ts
  - apps/desktop-electron/src/renderer/styles.css
  - apps/desktop-electron/src/renderer/viewModel.ts
  - apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx
  - apps/desktop-electron/src/renderer/workspace/Inspector.tsx
  - apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx
  - apps/desktop-electron/src/renderer/workspace/Timeline.tsx
  - apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx
  - apps/desktop-electron/tests/electron-smoke.spec.ts
  - apps/desktop-electron/tests/workspace.spec.ts
  - scripts/phase4-source-guards.sh
  - package.json
  - justfile
findings:
  critical: 2
  warning: 2
  info: 0
  total: 4
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-06-17T11:18:54Z
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

Reviewed the Phase 4 Electron renderer workspace, command helpers, CSS, Playwright coverage, source guards, and package gates against the Jianying-style desktop workspace intent. The implementation keeps the obvious Electron/Node/FFmpeg boundary out of renderer source, but command execution is still vulnerable to stale draft payloads and the UI can emit non-integer microsecond values into Rust command contracts. Both issues violate the Rust-owned timeline/time-model boundary and can produce incorrect edits or rejected commands.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: Concurrent UI actions can apply stale Rust command responses over newer draft state

**Classification:** BLOCKER
**File:** `apps/desktop-electron/src/renderer/App.tsx:131`
**Issue:** `executeTimelineCommand` receives a prebuilt `CommandEnvelope`, sends it asynchronously, and then unconditionally replaces `current.draft`, `current.commandState`, and `current.selection` with the command response at lines 139-158. The command payload itself is built from the render-time `workspace` captured by handlers such as `handleAddTimelineSegment` at lines 400-416, `handleSelectTimelineSegment` at lines 382-384, and undo/redo at lines 512-517. Because `pendingCommand` is only set through async React state, a rapid double click or two near-simultaneous handlers can send multiple commands against the same old draft. Whichever response resolves last overwrites the current state with a response derived from stale input, so accepted edits can disappear, undo history can be wrong, and selection can revert.
**Fix:** Serialize core commands before they are sent and build each command from the latest accepted workspace state, not from a stale render closure. A minimal fix is to keep a synchronous in-flight ref/latest-state ref, reject or queue commands while one is running, and construct the envelope inside that guarded path:

```tsx
const workspaceRef = useRef(workspace);
const commandInFlightRef = useRef(false);

useEffect(() => {
  workspaceRef.current = workspace;
}, [workspace]);

async function executeTimelineCommand(
  buildCommand: (current: WorkspaceState) => CommandEnvelope,
  pendingCommand: string
): Promise<void> {
  if (commandInFlightRef.current) {
    return;
  }

  commandInFlightRef.current = true;
  setWorkspace((current) => ({ ...current, pendingCommand, commandError: null }));

  try {
    const result = await window.videoEditorCore.executeCommand<TimelineCommandResponse>(
      buildCommand(workspaceRef.current)
    );
    setWorkspace((current) => {
      const applied = applyTimelineCommandResult(current, result);
      return {
        ...current,
        draft: applied.state.draft,
        commandState: applied.state.commandState,
        selection: applied.state.selection,
        materials: applied.state.draft.materials,
        pendingCommand: null,
        commandError: applied.errorMessage
      };
    });
  } finally {
    commandInFlightRef.current = false;
  }
}
```

Then update handlers to pass command builders, for example `executeTimelineCommand((current) => buildUndoTimelineEditCommand(current), "撤销")`. Add a Playwright test that triggers two timeline actions without waiting and asserts that no accepted edit is lost.

### CR-02: Duration controls can send fractional microseconds to Rust command contracts

**Classification:** BLOCKER
**File:** `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx:151`
**Issue:** Text and audio duration inputs store seconds as unrestricted JavaScript numbers and convert them with `durationSeconds * 1_000_000` at lines 151 and 261. `<input type="number" min="1">` still permits decimal values such as `1.3333333`, which produces a fractional microsecond value. These values are sent as `sourceTimerange.duration` and `targetTimerange.duration` in generated command payloads, violating the project time model requirement that editing semantics use integer microseconds, frame indices, or rational frame rates. Depending on Rust deserialization/validation, this can either reject valid-looking UI input or leak floating-point time into persisted draft semantics.
**Fix:** Store and edit microseconds as integers, or round/clamp at the UI boundary before building any command payload. The UI spec explicitly allows numeric inputs for microsecond-backed values.

```tsx
const [durationUs, setDurationUs] = useState(3_000_000);

<input
  type="number"
  min="1"
  step="1"
  value={durationUs}
  onChange={(event) => setDurationUs(Math.max(1, Math.round(event.currentTarget.valueAsNumber || 1)))}
/>

onClick={() => onAddTextSegment(text, durationUs)}
```

Apply the same integer conversion to audio duration and add source-guard or Playwright coverage that decimal seconds cannot enter command payload timeranges.

## Warnings

### WR-01: Inspector form state does not refresh when the selected segment changes without changing IDs

**Classification:** WARNING
**File:** `apps/desktop-electron/src/renderer/workspace/Inspector.tsx:51`
**Issue:** The inspector copies selected segment text and volume into local state in an effect, but the dependency is only `[selected?.segment.segmentId]` at line 79. Commands such as volume changes, text edits, undo, redo, and track mute can return a new draft for the same selected segment ID. In those cases the inspector keeps stale local form values even though `workspace.draft` has changed, so subsequent "应用文字" or "应用音量" can send outdated values back to Rust.
**Fix:** Depend on the selected semantic fields that are copied into local state, or derive the form state from the current segment when no unsaved edit is active. At minimum include the selected segment object fields used by the effect:

```tsx
useEffect(() => {
  // existing synchronization body
}, [
  selected?.segment.segmentId,
  selected?.segment.volume.levelMillis,
  selected?.segment.text?.content,
  selected?.segment.text?.style
]);
```

For a more robust editor interaction, track dirty form state and refresh from Rust responses only when the user has no uncommitted local edit.

### WR-02: Workspace command spy depends on Electron private internals

**Classification:** WARNING
**File:** `apps/desktop-electron/tests/workspace.spec.ts:56`
**Issue:** `spyExecuteCommandCalls` reaches into `ipcMain._invokeHandlers` and replaces the `core:executeCommand` handler at lines 56-75. `_invokeHandlers` is an Electron private implementation detail, not a public testing API. An Electron update can remove or rename it while the production command boundary still works, making the Phase 4 workspace test fail for harness reasons instead of product regressions. This weakens the required Playwright/source-guard coverage.
**Fix:** Instrument through a stable seam. Prefer a test-only preload/main hook exposed under an explicit environment flag, or assert command effects through public UI plus source guards. If command call recording is required, add a supported main-process test hook such as `VIDEO_EDITOR_TEST_RECORD_COMMANDS=1` that records commands inside the real registered handler without patching Electron internals, then read that test-only record through `app.evaluate`.

---

_Reviewed: 2026-06-17T11:18:54Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_

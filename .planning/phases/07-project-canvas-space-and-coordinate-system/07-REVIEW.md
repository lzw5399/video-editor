---
phase: 07-project-canvas-space-and-coordinate-system
reviewed: 2026-06-18T01:12:27Z
depth: standard
files_reviewed: 49
files_reviewed_list:
  - apps/desktop-electron/src/generated/CommandEnvelope.ts
  - apps/desktop-electron/src/generated/Draft.ts
  - apps/desktop-electron/src/main/index.ts
  - apps/desktop-electron/src/renderer/App.tsx
  - apps/desktop-electron/src/renderer/commandHelpers.ts
  - apps/desktop-electron/src/renderer/viewModel.ts
  - apps/desktop-electron/src/renderer/workspace/Inspector.tsx
  - apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx
  - apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx
  - apps/desktop-electron/src/renderer/workspace/preview-inspector.css
  - apps/desktop-electron/tests/workspace.spec.ts
  - crates/bindings_node/src/lib.rs
  - crates/bindings_node/src/preview_export_service.rs
  - crates/bindings_node/tests/binding_smoke.rs
  - crates/bindings_node/tests/canvas_commands.rs
  - crates/bindings_node/tests/export_commands.rs
  - crates/draft_commands/src/canvas.rs
  - crates/draft_commands/src/lib.rs
  - crates/draft_commands/src/timeline.rs
  - crates/draft_commands/tests/canvas_commands.rs
  - crates/draft_model/src/canvas.rs
  - crates/draft_model/src/draft.rs
  - crates/draft_model/src/lib.rs
  - crates/draft_model/src/validation.rs
  - crates/draft_model/tests/canvas_config.rs
  - crates/draft_model/tests/draft_fixtures.rs
  - crates/draft_model/tests/schema_exports.rs
  - crates/engine_core/src/normalize.rs
  - crates/engine_core/tests/canvas_profile.rs
  - crates/ffmpeg_compiler/src/job.rs
  - crates/ffmpeg_compiler/tests/canvas_profile_snapshots.rs
  - crates/preview_service/src/service.rs
  - crates/preview_service/tests/canvas_profile.rs
  - crates/render_graph/src/graph.rs
  - crates/render_graph/src/lib.rs
  - crates/render_graph/tests/canvas_background.rs
  - crates/testkit/tests/preview_export_parity.rs
  - docs/canvas-coordinate-system.md
  - fixtures/draft/invalid-timeline-command.json
  - fixtures/draft/minimal-timeline-command.json
  - fixtures/draft/negative/invalid-canvas-background-reference/project.json
  - fixtures/draft/negative/invalid-unknown-field/project.json
  - fixtures/draft/negative/missing-canvas-config/project.json
  - fixtures/draft/positive/materials-round-trip/project.json
  - fixtures/draft/positive/minimal-draft/project.json
  - fixtures/draft/positive/missing-material/project.json
  - schemas/command.schema.json
  - schemas/draft.schema.json
  - scripts/phase5-source-guards.sh
  - scripts/phase7-source-guards.sh
findings:
  critical: 2
  warning: 1
  info: 0
  total: 3
status: issues_found
---

# Phase 07: Code Review Report

**Reviewed:** 2026-06-18T01:12:27Z
**Depth:** standard
**Files Reviewed:** 49
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Reviewed the Phase 07 canvas model, command boundary, Electron UI command routing, preview/export derivation path, schemas, fixtures, and guards. The Rust semantic model and command envelope routing are mostly aligned with the project constraints, but supported canvas settings can still produce incorrect derived artifacts or unintended semantic rewrites.

## Critical Issues

### CR-01: Supported solid canvas backgrounds compile to black output

**Classification:** BLOCKER
**File:** `crates/ffmpeg_compiler/src/job.rs:260`
**Issue:** `compile_ffmpeg_job` forwards the render graph to `generate_filter_script`, but that path never applies `plan.graph.canvas.background`; the called filter generator hardcodes `color=c=black` for the base video. Render graph marks `solidColor` as `supported` and UI marks it ready, so a draft with `background: { kind: "solidColor", color: "#112233" }` exports/previews black whenever the base is visible.
**Fix:**
```rust
// In the filter generation path, derive the base from plan.graph.canvas.background.
let background_color = match &plan.graph.canvas.background.mode {
    RenderCanvasBackgroundMode::SolidColor => plan
        .graph
        .canvas
        .background
        .color
        .as_deref()
        .unwrap_or("#000000"),
    _ => "black",
};
lines.push(format!(
    "color=c={}:s={width}x{height}:r={rate}:d={duration}[vbase0]",
    ffmpeg_color_arg(background_color),
    width = dimensions.width,
    height = dimensions.height,
    rate = frame_rate_arg(plan),
    duration = format_seconds(output_duration(plan))
));
```
Add compiler and preview/export parity tests that assert a solid-color draft emits the selected color in the filter script and rendered frame.

### CR-02: Canvas inspector silently rewrites non-listed rational frame rates

**Classification:** BLOCKER
**File:** `apps/desktop-electron/src/renderer/workspace/Inspector.tsx:796`
**Issue:** `canvasFormFromConfig` collapses `frameRate` through `frameRateControlValue`; unsupported rates default to `"30"` and rational rates are rounded. Applying an unrelated canvas change then sends `{ numerator: 30, denominator: 1 }`, silently changing canonical draft semantics such as `30000/1001` or `48/1`.
**Fix:** Preserve the exact `RationalFrameRate` unless the user explicitly changes it. For example, keep numerator/denominator in form state and support custom display values:
```ts
type CanvasFormState = {
  // ...
  frameRateNumerator: string;
  frameRateDenominator: string;
};

function canvasFormFromConfig(config: DraftCanvasConfig): CanvasFormState {
  return {
    // ...
    frameRateNumerator: String(config.frameRate.numerator),
    frameRateDenominator: String(config.frameRate.denominator)
  };
}

// Build { numerator, denominator } from those exact fields instead of rounding/defaulting.
```
Keep the preset select as a convenience, but do not coerce unknown rational frame rates during unrelated canvas edits.

## Warnings

### WR-01: Canvas semantic changes leave stale derived preview/export state visible

**Classification:** WARNING
**File:** `apps/desktop-electron/src/renderer/App.tsx:768`
**Issue:** After `updateDraftCanvasConfig` succeeds, the app updates `draft` but preserves `preview.frameArtifactPath`, `preview.segmentArtifactPath`, export `jobId`, validation, and progress from the previous draft. Preview/export artifacts are derived from `.veproj/project.json`; after canvas dimensions, frame rate, or background changes, the visible artifact paths and validation metadata no longer describe the canonical draft.
**Fix:** Clear derived preview/export state on every successful draft semantic mutation that changes canvas/timeline/materials, or at least on canvas updates:
```ts
return {
  ...current,
  draft: result.data.draft,
  commandState: result.data.commandState,
  selection: result.data.selection,
  materials: result.data.draft.materials,
  preview: {
    frameArtifactPath: null,
    frameStatusLabel: "画布已更新，请重新请求预览帧",
    frameMetadataLabel: "预览帧需要重新生成",
    segmentArtifactPath: null,
    segmentStatusLabel: "画布已更新，请重新生成预览片段",
    segmentMetadataLabel: "预览片段需要重新生成",
    error: null,
    lastRequestedPlayhead: null,
    lastRequestedRangeLabel: null
  },
  export: {
    ...current.export,
    jobId: null,
    phase: null,
    progressPerMille: null,
    outTime: null,
    validation: null,
    diagnosticLabel: null,
    error: null,
    logSummary: "草稿已更新，请重新开始导出"
  },
  pendingCommand: null,
  commandError: null
};
```

---

_Reviewed: 2026-06-18T01:12:27Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_

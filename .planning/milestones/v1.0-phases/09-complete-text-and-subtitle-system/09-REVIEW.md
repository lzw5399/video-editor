---
phase: 09-complete-text-and-subtitle-system
reviewed: 2026-06-18T05:09:37Z
depth: standard
files_reviewed: 38
files_reviewed_list:
  - apps/desktop-electron/src/generated/CommandEnvelope.ts
  - apps/desktop-electron/src/generated/Draft.ts
  - apps/desktop-electron/src/main/index.ts
  - apps/desktop-electron/src/renderer/App.tsx
  - apps/desktop-electron/src/renderer/commandHelpers.ts
  - apps/desktop-electron/src/renderer/styles.css
  - apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx
  - apps/desktop-electron/src/renderer/workspace/Inspector.tsx
  - apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx
  - apps/desktop-electron/src/renderer/workspace/preview-inspector.css
  - apps/desktop-electron/tests/workspace.spec.ts
  - crates/bindings_node/src/lib.rs
  - crates/bindings_node/tests/preview_commands.rs
  - crates/bindings_node/tests/text_commands.rs
  - crates/draft_commands/src/text.rs
  - crates/draft_commands/src/timeline.rs
  - crates/draft_commands/tests/subtitle_commands.rs
  - crates/draft_commands/tests/text_audio_commands.rs
  - crates/draft_model/src/lib.rs
  - crates/draft_model/src/timeline.rs
  - crates/draft_model/src/validation.rs
  - crates/draft_model/tests/draft_schema.rs
  - crates/draft_model/tests/schema_exports.rs
  - crates/engine_core/src/text_layout.rs
  - crates/engine_core/tests/canvas_profile.rs
  - crates/engine_core/tests/frame_state_snapshots.rs
  - crates/engine_core/tests/normalization.rs
  - crates/ffmpeg_compiler/src/ass.rs
  - crates/ffmpeg_compiler/src/job.rs
  - crates/ffmpeg_compiler/tests/ass_snapshots.rs
  - crates/ffmpeg_compiler/tests/capability_snapshots.rs
  - crates/ffmpeg_compiler/tests/common/mod.rs
  - crates/preview_service/tests/preview_generation.rs
  - crates/render_graph/tests/render_graph_snapshots.rs
  - crates/testkit/tests/preview_export_parity.rs
  - schemas/command.schema.json
  - schemas/draft.schema.json
  - scripts/phase9-source-guards.sh
findings:
  critical: 0
  warning: 4
  info: 0
  total: 4
status: issues_found
---

# Phase 09: Code Review Report

**Reviewed:** 2026-06-18T05:09:37Z
**Depth:** standard
**Files Reviewed:** 38
**Status:** issues_found

## Summary

Reviewed the Phase 09 text and subtitle implementation across Rust model/commands/engine/compiler, generated schemas, Electron renderer, UI tests, and source guards. I did not find command-boundary violations in the renderer: SRT content is passed raw to Rust, and the UI does not construct FFmpeg/render graph/cache artifacts. `importSubtitleSrt` also performs mutations on a cloned draft and returns a single undo snapshot after validation.

The issues below are correctness and contract gaps around SRT parsing, text wrapping behavior, UI validation, and generated schema constraints.

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: [WARNING] SRT import does not split cues separated by whitespace-only blank lines

**File:** `crates/draft_commands/src/text.rs:226`

**Issue:** `parse_srt` splits the input with `normalized.split("\n\n")` before trimming lines. Real SRT files commonly contain blank separator lines with spaces or tabs. In that case the separator is not `"\n\n"`, so multiple cues collapse into one block; the second cue's numeric index and timing line are imported as text for the first cue instead of creating a separate subtitle segment.

**Fix:** Parse cue blocks line-by-line and treat any line where `trim().is_empty()` as a separator, or normalize blank separator lines before splitting. Add a regression test using a separator line containing spaces.

```rust
let mut blocks = Vec::new();
let mut current = Vec::new();
for line in normalized.lines().map(str::trim_end) {
    if line.trim().is_empty() {
        if !current.is_empty() {
            blocks.push(std::mem::take(&mut current));
        }
    } else {
        current.push(line);
    }
}
if !current.is_empty() {
    blocks.push(current);
}
```

### WR-02: [WARNING] `TextWrapping::Auto` is persisted but never resolved into wrapped lines

**File:** `crates/engine_core/src/text_layout.rs:231`

**Issue:** `resolve_text_overlay` derives `line_count` only from explicit newline characters in `text.content`, then carries `text.wrapping` through as metadata. It never uses the resolved text box or layout width to compute deterministic line breaks. The ASS compiler then emits `WrapStyle: 2` and passes `overlay.overlay.content` directly, so automatic wrapping is disabled in the export sidecar as well. A long subtitle with `wrapping: auto` can preview/export as a single overflowing line even though the draft says wrapping is enabled.

**Fix:** Resolve wrapping in the Rust engine using deterministic width/font/spacing inputs, expose the resolved lines in `ResolvedTextOverlay`, and have ASS generation write those explicit line breaks. If automatic wrapping is intentionally unsupported for this phase, validation or compiler diagnostics should reject it instead of silently accepting metadata that is not honored.

### WR-03: [WARNING] Inspector accepts text box values that Rust validation rejects

**File:** `apps/desktop-electron/src/renderer/workspace/Inspector.tsx:488`

**Issue:** The text box width and height fields allow values up to `2000`, and `validateTextForm` accepts `1..=2000` at `Inspector.tsx:1035`. Rust validation rejects text box `widthMillis` and `heightMillis` above `1000` through `validate_text_box` and `validate_positive_text_millis` (`crates/draft_model/src/validation.rs:461`). This lets users enter a value such as `1500`, see the form as valid, and then receive a Rust command failure when applying the edit.

**Fix:** Align the renderer controls and Chinese validation copy with the Rust contract by changing both max values and the validation range to `1000`, unless the intended product range is actually `2000`; in that case update the Rust constants, validation, schema, and tests together.

### WR-04: [WARNING] Generated schemas advertise weaker text constraints than Rust enforces

**File:** `schemas/draft.schema.json:932`

**Issue:** The generated draft schema declares `TextBox.widthMillis` and `heightMillis` as `minimum: 0` with no maximum, while Rust requires both to be greater than zero and `<= 1000`. The same mismatch exists in the command schema at `schemas/command.schema.json:3089`. `TextStyle` fields have similar contract drift: schema color fields are plain strings, and `fontSize`, `lineHeightMillis`, and `letterSpacingMillis` lack the Rust validation bounds shown in `crates/draft_model/src/validation.rs:417`. Schema consumers can therefore generate or accept draft/command JSON that passes schema validation but fails the Rust model.

**Fix:** Encode the Rust text constraints into the exported schemas, either via schemars attributes/newtypes or a schema post-processing step covered by `schema_exports` tests. At minimum, add `minimum: 1` and `maximum: 1000` for text box dimensions, `pattern: "^#[0-9A-Fa-f]{6}$"` for text colors, and the Rust min/max bounds for font size, line height, and letter spacing.

---

_Reviewed: 2026-06-18T05:09:37Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_

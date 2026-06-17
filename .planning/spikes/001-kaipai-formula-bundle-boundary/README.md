---
spike: 001
name: kaipai-formula-bundle-boundary
type: standard
validates: "Given an already-produced Kaipai formula bundle, when Video Editor imports it, then adapter/service/core responsibilities can be separated without calling Kaipai API or depending on Android worker runtime"
verdict: VALIDATED
related: []
tags: [kaipai, compatibility, draft, resources, safe-area]
---

# Spike 001: Kaipai Formula Bundle Boundary

## What This Validates

Given an already-produced Kaipai smart-edit formula, direct material list, recognizer `word_list`, and `safe_area` evidence, when Video Editor imports the data, then the project can route it through an adapter/resource-localization boundary into `.veproj/project.json` without making Kaipai API calls and without depending on the Android worker for runtime rendering.

This spike validates the architecture boundary. It does not validate pixel-level Kaipai rendering parity and does not implement the mapper.

## Research

### Project Constraints Checked

- `.planning/PROJECT.md` says external drafts go through adapters and compatibility reports; proprietary IDs are external references, not internal render semantics.
- `.planning/REQUIREMENTS.md` keeps Jianying/CapCut/Kaipai compatibility out of v1 MVP and lists compatibility report work under v2.
- `crates/draft_model/src/draft.rs` currently stores only `schema_version`, `draft_id`, `metadata`, `materials`, and `tracks`.
- `crates/draft_model/src/material.rs` supports material kinds `video`, `image`, `audio`, `text`, and `sticker`, but not font resources.
- `crates/draft_model/src/timeline.rs` supports target/source time ranges, tracks, segments, keyframes, filters, transitions, MVP text, and volume. It does not yet model canvas, transform, sticker payloads, z-order/layer fields beyond track ordering, font resources, or compatibility reports.

### dcoin Evidence Checked

- `/Users/zhiwen/code/dcoin/src/workers/kp-android-reverse-worker/src/worker-task-executor.js` has `prepareKaipaiSmartEditFormula()`, which obtains a selected template, fetches direct material list, submits formula data, polls formula result, and returns `formula` before Android export.
- `/Users/zhiwen/code/dcoin/src/workers/kp-android-reverse-worker/src/kaipai-app-template.js` requires explicit `safeArea`, `safeAreaStatus`, and `safeAreaSource` when building formula submit data.
- `/Users/zhiwen/code/dcoin/src/workers/kp-android-reverse-worker/android-entry/src/com/meitu/action/dcoin/reverse/KpReverseRecognizerStore.java` produces `safe_area` through App face detection over early video frames. This is provider/preprocess evidence, not a timeline property.
- `/Users/zhiwen/code/dcoin/src/editor-render-engine/datasets/cases/subtitle-font-stroke-shadow/input/resources.json` shows localized font resources can be represented by local path and sha256.
- `/Users/zhiwen/code/dcoin/src/editor-render-engine/datasets/cases/sticker-image-overlay/input/resources.json` shows localized video/PIP resources can be represented by local path and sha256.
- dcoin editor-render-engine README treats Android output as oracle evidence and does not permit Android/native tools as pure Rust runtime dependencies.

### Approach Comparison

| Approach | Pros | Cons | Status |
|---|---|---|---|
| Direct Kaipai API integration first | End-to-end provider path eventually needed | Adds auth/rate-limit/provider complexity before proving import semantics | Deferred |
| Keep Android worker as runtime renderer | Fastest route to existing MP4 output | No local preview/edit loop; repeats the current product problem | Rejected |
| Store Kaipai raw formula directly in `.veproj` | Minimal initial mapping work | Pollutes canonical draft schema and render path with proprietary external JSON | Rejected |
| Offline formula bundle -> resource localizer -> adapter -> `.veproj` + compatibility report | Proves editor-owned preview/edit/export path; isolates external data | Requires Draft v2/template semantics before full mapper | Chosen |

## Boundary Decision

The compatibility path should be:

```text
Kaipai formula bundle
  -> adapter/resource localizer
  -> Kaipai draft mapper
  -> .veproj/project.json + resources + compatibility report
  -> Video Editor preview/edit/export path
```

The layer split should be:

| Layer | Owns | Must Not Own |
|---|---|---|
| `adapter_kaipai` | formula bundle parsing, external provenance, supported/degraded/unsupported mapping, compatibility report | preview/render execution |
| resource localizer service | downloading/copying fonts, videos, images, stickers, sha256, relative paths | draft editing behavior |
| `draft_model` | canonical Jianying-style draft/material/track/segment/canvas/sticker/text semantics | raw Kaipai formula JSON, API calls, App-only evidence generation |
| `engine_core` | normalized draft/frame state from canonical semantics | Kaipai provider branches |
| `render_graph` / `ffmpeg_compiler` | platform-independent render intent and FFmpeg job generation | direct Kaipai formula interpretation |
| `media_runtime` | FFmpeg execution/progress/cancel/errors | deciding template semantics |
| Android worker | oracle/calibration fixture source | production runtime dependency |

## Formula Bundle Shape

The first adapter should accept an offline input package shaped like this. Names are transport/evidence names, while mapped draft fields should continue using Jianying concepts.

```json
{
  "schemaVersion": 1,
  "kind": "kaipaiSmartEditFormulaBundle",
  "provenance": {
    "templateId": "SEC0054",
    "recipeId": "1721819614877617",
    "formulaTaskId": "formula-task-1",
    "formulaRequestId": "formula-req-1",
    "capturedAt": "2026-06-17T00:00:00.000Z"
  },
  "sourceMedia": {
    "uri": "file:///local/source.mp4",
    "width": 1080,
    "height": 1920,
    "durationMs": 3000
  },
  "recognizerResult": {
    "word_list": []
  },
  "safeArea": {
    "value": "100,200,300,400",
    "status": "detected",
    "source": "app_face_detector_frame_0ms"
  },
  "directMaterials": [],
  "formula": {},
  "resources": []
}
```

`safeArea` should stay in this bundle/provenance unless the adapter maps a concrete visual result into draft semantics. If Video Editor later needs to generate it, that belongs to a `SafeAreaProvider`/preprocess service with replaceable backends, not to `draft_model`.

## Schema Gaps Before Mapper Implementation

The current v1 draft can accept simple video/audio/text timelines, but the adapter needs a template-semantics foundation first:

- `Draft.canvas`: output width, height, fps, background, coordinate policy.
- `MaterialKind::Font`: font file material with local resource path and metadata.
- Resource manifest: remote origin, local relative path, sha256, kind, download status.
- Segment canvas adjustment/transform: position, anchor, scale, rotate, opacity, crop/fit, flip, layer/z-order, blend mode.
- Sticker/text sticker segment payloads: image/video sticker, text sticker, font path, stroke, shadow, bubble/effect fallback.
- Compatibility report: supported, degraded, unsupported, missing resource, needs native effect.
- External provenance: template id, recipe id, formula task id, raw formula digest, source system.

These are not Kaipai-only features; they also support Jianying draft import, manual stickers, mobile/server rendering, and preview/export parity.

## How to Verify

This spike is self-verifiable by source inspection, not by running product code:

```bash
rg -n "prepareKaipaiSmartEditFormula|safeArea|fetchDirectMaterialList|submitSmartEditRecipeFormula|pollSmartEditRecipeFormula" /Users/zhiwen/code/dcoin/src/workers/kp-android-reverse-worker
rg -n "textEditInfoList|fontPath|pipList|stickerList|videoCanvasConfig|videoClipList" /Users/zhiwen/code/dcoin/src/editor-render-engine/datasets/cases
rg -n "MaterialKind|struct Draft|struct Segment|TextSegment|TrackKind" crates/draft_model/src
```

## Investigation Trail

1. Checked Video Editor planning constraints and confirmed compatibility is explicitly adapter/report-driven, not core-format-driven.
2. Inspected current `draft_model` and confirmed v1 schema lacks canvas, transform, font resources, resource bundles, sticker payloads, and compatibility reporting.
3. Inspected dcoin worker and confirmed formulas are available before Android export through `prepareKaipaiSmartEditFormula()`.
4. Inspected dcoin safe-area path and confirmed `safe_area` is formula-generation evidence produced by App face detection, not a render-layer concept.
5. Inspected dcoin editor-render-engine fixtures and confirmed local resources can be represented with path/sha256 manifests and Android oracle outputs.
6. Compared implementation approaches and rejected both Android-runtime dependency and raw-formula-in-draft storage.

## Results

Verdict: VALIDATED for architecture and sequencing.

Key findings:

- We can start with offline Kaipai formula bundles and avoid live Kaipai API integration for the first adapter work.
- The adapter should emit `.veproj/project.json`, local project resources, and a compatibility report.
- `safe_area` belongs to formula provider/preprocess evidence and adapter provenance. It should not be implemented inside Rust draft core.
- Current v1 draft schema should not be stretched with raw strings to fake template support. Add template semantics deliberately before mapper implementation.
- The next independently shippable pieces are fixture corpus, compatibility report schema, resource bundle/localizer, and Draft v2 template semantics.

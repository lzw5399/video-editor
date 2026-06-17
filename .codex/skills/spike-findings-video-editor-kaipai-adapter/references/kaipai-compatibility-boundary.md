# Kaipai Compatibility Boundary

## Requirements

- Accept offline Kaipai formula bundles before building live Kaipai API integration.
- Keep Kaipai raw JSON out of `draft_model`, `engine_core`, `render_graph`, and `ffmpeg_compiler`.
- Localize template resources into stable project resources before preview/export depends on them.
- Treat `safe_area` as provider/preprocess evidence and adapter provenance, not as Rust draft core semantics.
- Use Android worker output only as oracle/calibration evidence.
- Keep Jianying-style vocabulary across schema, commands, docs, and tests.

## How to Build It

1. Start with a dedicated compatibility phase instead of extending the current desktop UI phase.
   The first phase should be named around compatibility foundation, not full Kaipai adapter parity.

2. Define the offline input contract first:

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

3. Build the first implementation in this order:

   - Fixture corpus: sanitized formula bundle JSON, direct material list, recognizer `word_list`, `safeArea` evidence, and resource manifest samples.
   - `CompatibilityReport`: supported, degraded, unsupported, missing resource, and needs native effect entries.
   - Resource bundle/localizer: local relative paths, sha256 validation, missing resource diagnostics, no renderer-side remote URLs.
   - Draft/template semantics foundation: `Draft.canvas`, font material, resource references, canvas adjustment/transform, sticker/text sticker payloads, external provenance.
   - Offline mapper POC: source video, canvas, PIP/video overlay, basic text sticker, image sticker, font references, and report output.

4. Keep layer ownership strict:

   | Layer | Owns | Must Not Own |
   |---|---|---|
   | `adapter_kaipai` | formula bundle parsing, external provenance, supported/degraded/unsupported mapping, compatibility report | preview/render execution |
   | Resource localizer service | downloading/copying fonts, videos, images, stickers, sha256, relative paths | draft editing behavior |
   | `draft_model` | canonical Jianying-style draft/material/track/segment/canvas/sticker/text semantics | raw Kaipai formula JSON, API calls, App-only evidence generation |
   | `engine_core` | normalized draft/frame state from canonical semantics | Kaipai provider branches |
   | `render_graph` / `ffmpeg_compiler` | platform-independent render intent and FFmpeg job generation | direct Kaipai formula interpretation |
   | `media_runtime` | FFmpeg execution/progress/cancel/errors | deciding template semantics |
   | Android worker | oracle/calibration fixture source | production runtime dependency |

5. Add regression gates before calling a plan complete:

   - `just test`
   - schema/TypeScript drift checks for any draft or report contract changes
   - golden fixture validation for formula bundle and compatibility report snapshots
   - source guards proving core/render crates do not import Kaipai provider code or Android worker code
   - resource-localizer tests proving `.veproj/project.json` does not rely on remote template URLs for renderable resources

## What to Avoid

- Do not integrate Kaipai live APIs before the offline formula bundle path is proven.
- Do not keep Android worker as the runtime renderer; it recreates the old no-preview/no-edit product problem.
- Do not store raw Kaipai formula JSON directly as canonical `.veproj` render semantics.
- Do not hide unsupported native effects inside generic `Filter.parameters` strings just to make the mapper appear complete.
- Do not put `safe_area` detection inside `draft_model`, `engine_core`, `render_graph`, or `ffmpeg_compiler`.

## Constraints

- Current v1 draft schema lacks canvas, transform, font resources, sticker payloads, resource manifest, and compatibility report.
- `safe_area` is an input/evidence requirement for formula generation in the current dcoin chain; it can be stored as provenance for imported formula bundles.
- Pixel-level Kaipai parity is not validated by the first spike. Unsupported and degraded features must be explicit in the report.
- dcoin editor-render-engine fixtures and Android oracle outputs are useful evidence, but Android/native tools are not allowed as the pure Rust runtime.

## Origin

Synthesized from spikes: 001
Source files available in: `sources/001-kaipai-formula-bundle-boundary/`

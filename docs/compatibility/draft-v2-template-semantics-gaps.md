# Draft v2 Template Semantics Gap Inventory

This inventory is the blocking schema-preparation contract before any offline Kaipai mapper proof-of-concept can claim preview/export support. It keeps the next steps Jianying-style and general-purpose so Jianying/CapCut draft import, manual stickers, mobile/server rendering, and preview/export parity can reuse the same draft semantics.

The current Phase 03.1 adapter accepts offline formula evidence, emits compatibility reports, and localizes resources. It does not change `draft_model`, does not implement a mapper, and does not claim native effect parity.

## Draft.canvas

- **Current source file gap:** `crates/draft_model/src/draft.rs` has `Draft` metadata, materials, and tracks, but no canvas/profile field for width, height, background, aspect ratio, or display transform.
- **Required canonical concept:** Add `Draft.canvas` as canonical draft semantics, not provider evidence. It should describe the editable canvas for tracks, segments, filters, transitions, and later render graph normalization.
- **Import/report behavior until resolved:** Template import must report canvas semantics as degraded or unsupported when the offline formula requires a canvas that cannot be represented in `.veproj/project.json`.
- **Verification gate:** Schema and fixture tests must prove `Draft.canvas` round-trips through Rust, JSON Schema, TypeScript contracts, and positive `.veproj` fixtures before mapper preview/export support is claimed.

## Font material/resource references

- **Current source file gap:** `crates/draft_model/src/material.rs` has `MaterialKind::Text` and `MaterialKind::Sticker`, but no font material kind, font family resource reference, fallback font pinning, or local resource URI relationship.
- **Required canonical concept:** Font resources should be represented through material/resource semantics that text segments can reference without storing provider IDs as render behavior.
- **Import/report behavior until resolved:** Formula text that depends on a specific external font must produce a compatibility report item such as degraded or missingResource, while preserving text content and basic text material/segment data when possible.
- **Verification gate:** Positive draft fixtures must prove text segments can reference local font material/resource entries, and negative fixtures must reject remote font render URLs or raw provider font IDs in canonical draft semantics.

## Resource manifest

- **Current source file gap:** `crates/draft_model/src/material.rs` stores material URI and metadata, and `crates/adapter_kaipai/src/resource_localizer.rs` emits `LocalizedResourceManifest`, but canonical draft v1 has no general resource manifest for fonts, stickers, overlays, or reusable template assets.
- **Required canonical concept:** Add a draft-owned resource manifest or resource material layer that maps local bundle-relative resources to material, track, segment, sticker, text, filter, and transition consumers.
- **Import/report behavior until resolved:** The adapter may localize resources under `.veproj/resources`, but mapper work must not rely on those resources as preview/export semantics until canonical draft references exist.
- **Verification gate:** Resource-localizer tests plus draft fixture tests must prove renderable resources are local relative paths and that `.veproj/project.json` contains semantic references only, not remote render URLs or derived artifacts.

## Canvas adjustment/transform

- **Current source file gap:** `crates/draft_model/src/timeline.rs` has segment source/target timerange and placeholder string keyframes, but no typed position, scale, rotation, opacity, crop, anchor, or canvas adjustment model.
- **Required canonical concept:** Segment transform and canvas adjustment should be typed Jianying-style track/segment semantics, using integer or rational values where persisted precision matters.
- **Import/report behavior until resolved:** Any formula `safeArea`, placement, crop, or canvas adjustment evidence must remain adapter provenance or produce degraded/unsupported report items; raw formula values must not become render logic.
- **Verification gate:** Draft model tests must prove typed transform fields validate, serialize, and round-trip; command tests must prove edits go through Rust-owned commands before preview/export uses them.

## Sticker and text sticker payloads

- **Current source file gap:** `crates/draft_model/src/timeline.rs` has `TrackKind::Sticker`, `TrackKind::Text`, and `Segment.text`, but no image sticker payload, animated sticker payload, PIP/video overlay payload, text sticker bubble, or text effect payload.
- **Required canonical concept:** Stickers and text stickers should be represented as material-backed track/segment payloads with explicit sticker/text/filter/transition relationships.
- **Import/report behavior until resolved:** Formula sticker, PIP, and text sticker inputs must classify as supported only for the subset that maps to current text/material semantics; all richer payloads must be degraded, unsupported, missingResource, or needsNativeEffect.
- **Verification gate:** Adapter and draft fixtures must cover image sticker, local video overlay, text sticker, and missing sticker resource cases before mapper preview/export support is claimed.

## External provenance

- **Current source file gap:** `crates/draft_model/src/draft.rs` has no general external provenance model, while `crates/adapter_kaipai/src/formula_bundle.rs` keeps provider evidence such as raw formula JSON, template id, recipe id, `word_list`, and `safeArea` inside the adapter boundary.
- **Required canonical concept:** If imported drafts need provenance, use a provider-neutral external provenance concept that records source identity for audit/reporting without becoming render semantics.
- **Import/report behavior until resolved:** Phase 03.1 keeps raw formula and provider evidence in `adapter_kaipai` and compatibility reports only. `.veproj/project.json` must not store raw Kaipai formula fields, Android worker details, or provider API state.
- **Verification gate:** Source guards must continue to block Kaipai/provider/API/Android/raw formula leakage into `draft_model`, `engine_core`, `render_graph`, `ffmpeg_compiler`, and `schemas/draft.schema.json`.

## Compatibility report artifacts

- **Current source file gap:** `crates/adapter_kaipai/src/compatibility_report.rs` owns external import diagnostics, but canonical `.veproj/project.json` does not persist compatibility report artifacts.
- **Required canonical concept:** Compatibility reports should remain adjacent adapter/import artifacts unless a later product requirement adds a provider-neutral report reference to draft metadata.
- **Import/report behavior until resolved:** Adapter mapper work must emit report snapshots for supported, degraded, unsupported, missingResource, and needsNativeEffect items without storing report payloads as draft render semantics.
- **Verification gate:** `cargo test -p adapter_kaipai compatibility_report -- --nocapture` and schema drift checks must pass before mapper work consumes report data.

## Typed transform/keyframe semantics

- **Current source file gap:** `crates/draft_model/src/timeline.rs` stores `Keyframe { property: String, value: String }`, which cannot safely express typed position, scale, rotation, opacity, or volume semantics for preview/export parity.
- **Required canonical concept:** Replace or extend placeholder keyframes with typed keyframe properties for position, scale, rotation, opacity, and volume, aligned with segment transform and audio semantics.
- **Import/report behavior until resolved:** Formula transform keyframes should report degraded or unsupported unless they can map to existing canonical fields such as `SegmentVolume`; mapper code must not parse raw formula strings into render behavior.
- **Verification gate:** Draft, command, engine, render graph, and FFmpeg compiler tests must prove typed keyframes normalize deterministically before preview/export support is claimed.

## Follow-up Gate

Before an offline mapper proof-of-concept claims preview/export support, the project must either implement these Draft v2/template semantics or keep reporting the related formula features as degraded, unsupported, missingResource, or needsNativeEffect. This preserves COMP-01 preparation, COMP-02 report honesty, ADV-02 sticker/overlay preparation, and ADV-03 transform keyframe preparation without adding live Kaipai API integration or Android worker runtime dependency.

# Spike Wrap-Up Summary

**Date:** 2026-06-17
**Spikes processed:** 1
**Feature areas:** Kaipai compatibility boundary
**Skill output:** `./.codex/skills/spike-findings-video-editor-kaipai-adapter/`

## Processed Spikes

| # | Name | Type | Verdict | Feature Area |
|---|------|------|---------|--------------|
| 001 | kaipai-formula-bundle-boundary | standard | VALIDATED | Kaipai compatibility boundary |

## Key Findings

- The first Kaipai adapter path should accept offline formula bundles instead of calling live Kaipai APIs.
- Formula bundle handling, resource localization, mapping, and compatibility reporting should live in adapter/service boundaries.
- Kaipai raw formula JSON must not become canonical `.veproj` render semantics and must not leak into core/render crates.
- `safe_area` belongs to provider/preprocess evidence and adapter provenance; Rust draft core should not perform App face detection.
- Android worker output is useful as oracle/calibration evidence but should not become a product runtime dependency.
- Next implementation work should start with fixture corpus, compatibility report schema, resource bundle/localizer, then Draft v2 template semantics before an offline adapter POC.

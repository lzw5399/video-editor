# Spike Manifest

## Idea

Explore the Kaipai formula compatibility boundary for Video Editor. The near-term goal is not to call Kaipai APIs or depend on the Android worker at runtime; it is to accept an already-produced Kaipai smart-edit formula plus its evidence and material references, localize required resources, and map the supported subset into our Jianying-aligned `.veproj/project.json` draft semantics with a compatibility report.

## Requirements

- The first Kaipai adapter path must support offline formula input; live Kaipai API integration is out of scope until the formula bundle boundary is proven.
- Kaipai raw formula JSON must stay outside `draft_model`, `engine_core`, `render_graph`, and `ffmpeg_compiler`; those layers consume canonical draft semantics only.
- Resource downloading/localization belongs to an adapter/service boundary and writes stable local project resources, not direct renderer-side remote URLs.
- `safe_area` belongs to formula-provider/preprocess evidence and adapter provenance. Rust draft core should not perform App face detection and should not treat `safe_area` as timeline/render semantics unless a mapped visual property explicitly depends on it.
- The Android worker may remain a fixture/oracle/calibration source, but it must not become a Video Editor runtime dependency.
- Internal product, schema, command, and test concepts continue to use Jianying-style terms: draft, material, track, segment, canvas adjustment, sticker, text, filter, transition, keyframe, compatibility report.

## Spikes

| # | Name | Type | Validates | Verdict | Tags |
|---|------|------|-----------|---------|------|
| 001 | kaipai-formula-bundle-boundary | standard | Given an already-produced Kaipai formula bundle, when Video Editor imports it, then adapter/service/core responsibilities can be separated without calling Kaipai API or depending on Android worker runtime | VALIDATED | kaipai, compatibility, draft, resources, safe-area |

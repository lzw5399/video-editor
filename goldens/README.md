# Golden Harness

Phase 1 creates only the deterministic harness structure needed by later draft
and render work. The current render smoke generates tiny media at test time from
FFmpeg lavfi sources, stores it in a temporary `media-generated` directory, and
asserts ffprobe metadata for duration, frame rate, resolution, and stream
presence.

Full draft/render golden cases are assigned to later phases after the draft
schema, render graph, compiler, and preview/export semantics exist. Do not add
binary media baselines or exported videos to this directory in Phase 1.

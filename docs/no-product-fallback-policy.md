# No Product Fallback Policy

This policy is a mandatory review gate for every product-facing editing path.
It is especially strict for desktop realtime preview, but the same rule applies
to import, timeline editing, preview, export, resource generation, and future
mobile/server surfaces whenever a feature claims user-visible success.

## Rule

Normal product behavior must not report success through fallback output. If the
production implementation for a supported path is unavailable, the feature must
fail closed with a clear unavailable diagnostic instead of silently switching to
an approximate, mock, debug, artifact, CPU, or legacy path.

When a fallback path already exists and can be exercised by normal users, remove
or gate that path before replacing it with the production implementation. Do not
leave the fallback active as a temporary product behavior.

Because this editor is being built from scratch, refactors should prefer
removing obsolete paths over preserving compatibility with partial historical
implementations. Apply `docs/refactor-and-legacy-cleanup-policy.md` whenever a
change replaces a product path.

The product path must not use any of these as proof that playback works:

- mock realtime backends or synthetic frame tokens
- preview PNG/frame requests during playback
- preview artifact or FFmpeg artifact frames
- FFmpeg CPU decode probes or decoded-frame fingerprints
- offscreen/CPU readback evidence standing in for the visible compositor
- generated colors, screenshots, or DOM overlays that are not the presented video

## Allowed Uses

Fallback terminology may remain in low-level capability reports, diagnostics, or
legacy tests only when it describes an unavailable/degraded condition. It must
not continue playback, mark playback as passed, or satisfy product E2E evidence.

Developer diagnostics may explain why realtime preview is unavailable. They may
not turn fallback artifacts into a product preview surface.

Internal conservative recovery is allowed only when it fails closed or preserves
correctness without pretending the target capability succeeded. Examples:
recording full-draft invalidation when precise dependency facts are missing, or
returning an explicit unsupported/degraded compatibility report for external
draft import. These paths must be named as diagnostics, not product success.

## Review Checklist

Every review touching product behavior must check:

- Product playback success backend is `renderGraphGpu` only. `mock`, `gpu`
  frame requests, `offscreen`, `previewArtifact`, `ffmpegArtifact`, and
  `nativeVideoBridge` cannot be product success.
- Product playback evidence is `renderGraphGpuComposited` output from the
  realtime preview surface, not decoded CPU media evidence or native
  single-video player output.
- A native single-video player layer may be used only as an explicit
  diagnostic/unsupported bridge and must not be labeled as realtime GPU
  composition, available product presentation, or playback success.
- Electron renderer and main process do not choose fallback paths or mutate
  semantic state to simulate success.
- Rust binding APIs do not expose debug, mock, FFmpeg CPU, preview artifact, or
  legacy paths as product-success evidence.
- Product E2E tests fail when the production path evidence is absent.
- Every visible default editing control has a user-level E2E case or is hidden
  or gated until the production implementation exists.
- Reviews apply `docs/product-e2e-acceptance-policy.md` before accepting a
  product-facing feature as complete.
- Any fallback wording in code, docs, tests, and telemetry is either a diagnostic
  for unavailable/degraded behavior or a non-product test harness utility.
- `pnpm run test:no-product-fallback` passes.

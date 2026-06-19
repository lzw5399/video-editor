# No Product Fallback Policy

This policy is a mandatory review gate for the desktop realtime preview product
path.

## Rule

Normal product realtime preview must not report success through fallback output.
If the true realtime GPU/native texture/composited/present path is unavailable,
the feature must fail closed with a clear unavailable diagnostic.

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

## Review Checklist

Every review touching realtime preview must check:

- Product playback evidence is `composited` output from the realtime preview
  surface, not decoded CPU media evidence.
- Electron renderer and main process do not choose fallback paths.
- Rust binding APIs do not expose FFmpeg CPU probes as realtime preview
  evidence.
- Product E2E tests fail when composited GPU evidence is absent.
- `pnpm run test:no-product-fallback` passes.

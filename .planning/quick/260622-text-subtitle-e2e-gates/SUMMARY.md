# Text And Subtitle E2E Gates Summary

## Completed

- Added a second deterministic bundled CJK font, `Noto Serif CJK SC Regular`, with OFL metadata and registry validation.
- Extended realtime native text overlay evidence with font, style, layout, and visual transform fields.
- Changed GPU text rasterization to load and cache the actual bundled font by `fontRef`.
- Applied text visual transform fields to native GPU text quads.
- Updated the Inspector font input so known bundled font families emit real bundled `fontRef` values.
- Added a product packaged E2E gate covering real video/audio fixtures, same-time title plus two subtitle tracks, later subtitle cues, multiple fonts, movement, rotation, scale, opacity, content edits, native host PNG evidence, and DOM-overlay exclusion.
- Kept packaged builds deterministic by using the installed local Electron distribution instead of downloading Electron during `package:dir`.

## Verification

- `cargo fmt -p draft_model -p media_runtime -p ffmpeg_compiler -p realtime_preview_runtime -p bindings_node --check`
- `cargo test -p draft_model --test font_registry -- --nocapture`
- `cargo test -p realtime_preview_runtime --lib scheduler -- --nocapture`
- `cargo test -p realtime_preview_runtime --lib gpu::text::tests::rasterizer_uses_the_requested_bundled_font_ref -- --nocapture`
- `cargo test -p bindings_node --lib scheduler_ -- --nocapture`
- `cargo test -p ffmpeg_compiler --test ass_snapshots -- --nocapture`
- `cargo test -p media_runtime --test runtime_capability -- --nocapture`
- `corepack pnpm --dir apps/desktop-electron run build:electron`
- `corepack pnpm --dir apps/desktop-electron run package:dir`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "multi-font multi-track native preview evidence" --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "composites video external audio text and two-cue SRT" --workers=1 --reporter=line`
- `VIDEO_EDITOR_P0_USER_MATERIAL=/Users/zhiwen/Downloads/5300d8457cc6d4692ff5b922c089f823_raw.mp4 corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "P0 user portrait" --workers=1 --reporter=line`
- `corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -g "resizing larger and smaller" --workers=1 --reporter=line`
- `corepack pnpm -w run test:phase3-source-guards`
- `git diff --check`

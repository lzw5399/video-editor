# Product Journey Media Fixtures

These files are committed on purpose so product-level Playwright journeys do not depend on external user media or `/tmp` paths.

- `p0-moving-testsrc.mp4`: 3s, 320x180, 30 fps, MPEG-4 video generated from FFmpeg `testsrc2`; pixels change continuously and are suitable for visible-playback checks.
- `p0-tone.wav`: 3s, 44.1 kHz mono PCM sine wave for audio import/playback/export journeys.

Regenerate only when the acceptance contract changes. The fixtures must remain small, deterministic, and visibly/audibly non-empty.

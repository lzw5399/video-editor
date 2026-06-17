# Canvas Coordinate System

Phase 07 defines the shared draft-level canvas coordinate contract for later transform, text, sticker, PIP, keyframe, preview, and export behavior.

This document is semantic source material for future implementation. It does not add segment transform, keyframe, crop, mask, fit, fill, stretch, or clip-level background filling behavior.

## Normalized Canvas Space

- Coordinate space: normalized canvas coordinates.
- Origin: origin at canvas center.
- `+X right`: positive X moves toward the right edge of the canvas.
- `+Y up`: positive Y moves toward the top edge of the canvas.
- `1.0` on X equals half canvas width.
- `1.0` on Y equals half canvas height.
- Canvas center is `(0, 0)`.
- Canvas right edge is `x = 1`.
- Canvas left edge is `x = -1`.
- Canvas top edge is `y = 1`.
- Canvas bottom edge is `y = -1`.

This matches the Jianying/pyJianYingDraft convention where visual positions are expressed in half-canvas width/height units.

## Pixel Conversion

UI preview pixel space is derived display state. It has top-left origin and Y down. Persisted semantic values must not use renderer pixel offsets as canonical coordinates.

For a canvas with `width` and `height`:

- `half_width = width / 2`
- `half_height = height / 2`
- `pixel_x = half_width + normalized_x * half_width`
- `pixel_y = half_height - normalized_y * half_height`
- `normalized_x = (pixel_x - half_width) / half_width`
- `normalized_y = (half_height - pixel_y) / half_height`

Examples:

| Normalized | Pixel Meaning |
|------------|---------------|
| `(0, 0)` | canvas center |
| `(1, 1)` | top-right corner |
| `(-1, -1)` | bottom-left corner |
| `(0, 1)` | top center |
| `(0, -1)` | bottom center |

## Contract For Later Phases

- Text safe area, sticker/PIP position, segment transform, and keyframe UI must share this coordinate definition unless a later phase explicitly defines a local material-space conversion for crop or mask.
- Future UI may show pixel readouts as helper text, but normalized canvas coordinates remain the shared semantic language.
- Preview/export may scale derived artifacts for performance, but coordinate and canvas semantics must come from draft-level `canvasConfig`.
- Electron renderer code may convert UI pixels for display, but Rust core owns persisted draft semantics.

## Chinese UI Copy

When a compact UI hint is needed, use:

`坐标以画布中心为原点，X 向右，Y 向上`

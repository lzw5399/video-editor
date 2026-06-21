# MVP Known Limits

This document records the desktop MVP limits and the intended post-MVP backlog.
It describes product and release scope only; it does not add new editor
semantics.

## Runtime And Packaging

- FFmpeg and ffprobe are bundled application resources for desktop packages.
- Runtime discovery is single-source and uses `VE_BUNDLED_FFMPEG_DIR` or the app
  resources runtime directory.
- Packaged tests build and launch local directory packages. They are not signed
  public installers.
- macOS signing and notarization are not configured.
- The bundled FFmpeg engineering manifest is recorded, but public
  redistribution approval remains pending legal review.
- Windows signing, Linux package repository publishing, auto-update, and
  installer polish are deferred.
- App icon polish and branded release assets are deferred.

## Editor Semantics

The MVP verifies import, timeline command edits, preview, and export through the
Rust-owned draft/material/track/segment path. Advanced editor semantics are
scheduled after Phase 6:

- Phases 7-13 cover project canvas space, segment transform/compositing,
  complete text/subtitle layout, typed keyframes, retiming/speed, filter and
  adjustment semantics, and transitions.
- Effects, stickers, masks, blend modes, advanced animation, and high-fidelity
  template behavior remain limited until those phases land.
- GPU real-time preview, proxy management, waveform/thumbnail production, and
  large preset libraries are not complete MVP promises.

## Compatibility Backlog

- Jianying/CapCut/Kaipai draft compatibility remains post-MVP. External drafts
  should go through adapters and compatibility reports before they become
  internal `.veproj` projects.
- Proprietary IDs, effect packs, filters, transitions, stickers, and template
  references are external compatibility data, not internal render semantics.
- Future mobile clients are architecture extension slots, not Phase 6 deliverables.
- Future server rendering is an architecture extension slot, not a Phase 6 deliverable.

## Product Copy And UX

- Desktop user-visible copy should remain Simplified Chinese and use
  Jianying-style terminology such as draft, material, track, segment, keyframe,
  filter, and transition concepts.
- Deferred categories can stay visible in the workspace with Chinese
  not-yet-connected states, but they should not imply implemented editing
  behavior until Rust commands and render semantics exist.

## Release Operator Checklist

Before presenting an MVP build as release-ready, verify:

1. `pnpm run test:phase6` passes.
2. `pnpm run test:phase6-packaging` passes on the target platform.
3. `pnpm --dir apps/desktop-electron run provision:ffmpeg-runtime` records the
   bundled FFmpeg manifest for the target platform.
4. Signing/notarization status is stated clearly for the distributed artifact.
5. Public redistribution approval for the exact bundled FFmpeg build is recorded
   before publishing an external release.

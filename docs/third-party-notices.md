# Third-Party Notices

This notice file summarizes the current engineering dependency posture. It is
not a complete legal review for a public bundled-media-runtime release.

## Video Editor

Video Editor is licensed under the MIT License. See `LICENSE`.

## Electron Desktop Dependencies

The desktop app is built with Electron, React, React DOM, Vite, Playwright, and
related JavaScript tooling declared in the repository package manifests. Release
operators should regenerate dependency notices from the locked package graph
before any public binary release.

## Rust Dependencies

The editing, render, media runtime, project store, and Node-API binding layers
are Rust workspace crates. Release operators should generate Rust dependency
license notices from `Cargo.lock` before any public binary release.

## FFmpeg And ffprobe

FFmpeg and ffprobe are bundled application resources for desktop packages.

The app discovers them only through the bundled runtime directory configured on
the native binding by the Electron shell from packaged resources. Product startup does not honor
external FFmpeg paths. The engineering manifest generated at package time
records the exact version, configure line, checksums, and `legalReviewPending`
status.

This notice does not claim final redistribution approval. Public distribution
must complete a license review for the exact FFmpeg build and enabled external
libraries before `redistributableBuild` can become true.

## External References

Kdenlive, MLT, Jianying, CapCut, Kaipai, and pyJianYingDraft are references for
concepts, terminology, and compatibility research only. This project does not
copy their code, assets, XML definitions, presets, or UI implementation.

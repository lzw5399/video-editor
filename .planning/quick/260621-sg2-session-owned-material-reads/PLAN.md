---
status: in_progress
created: 2026-06-21
skill: gsd-quick
---

# Session-Owned Material Reads

## Goal

Move product material list and missing-material checks off renderer-originated full-draft payloads and onto Rust project session APIs.

## Production Constraint

The Rust project session owns the canonical draft. Product UI can request session-derived views/diagnostics, but it must not send `workspace.draft` back to Rust for material listing or missing-material validation.

## Verification

- Rust binding tests for session-owned material listing and missing-material diagnostics.
- Electron build.
- Source guard preventing product renderer paths from using `buildListMaterialsCommand` / `buildListMissingMaterialsCommand`.

# Spike Conventions

Patterns and stack choices established across spike sessions. New spikes follow these unless the question requires otherwise.

## Stack

- Use repository-native Rust/TypeScript terminology and schema references when the spike affects product architecture.
- Keep compatibility spikes documentation-first until a concrete fixture corpus exists; do not prematurely modify runtime crates for external template formats.
- For compatibility work, treat `.veproj/project.json` as the semantic source of truth and generated reports/resources as explicit adjacent artifacts.

## Structure

- Put compatibility spike artifacts under `.planning/spikes/NNN-short-name/`.
- Keep root research summaries in Chinese when they are intended for project direction.
- Package validated spike findings into project-local skills under `.codex/skills/spike-findings-*` before turning them into implementation phases.

## Patterns

- External template inputs are treated as adapter evidence, not canonical render semantics.
- Offline fixture/formula input is preferred before live provider/API work.
- Android worker outputs are oracle/calibration evidence, not runtime dependencies.
- `safe_area` stays in provider/preprocess/provenance boundaries unless a concrete visual result is mapped into canonical draft semantics.
- Regression plans for compatibility work need source guards, schema drift checks, fixture snapshots, and resource-localizer checks.

## Tools & Libraries

- No additional tools established yet.

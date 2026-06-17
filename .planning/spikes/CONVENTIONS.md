# Spike Conventions

Patterns and stack choices established across spike sessions. New spikes follow these unless the question requires otherwise.

## Stack

- Use repository-native Rust/TypeScript terminology and schema references when the spike affects product architecture.
- Keep compatibility spikes documentation-first until a concrete fixture corpus exists; do not prematurely modify runtime crates for external template formats.

## Structure

- Put compatibility spike artifacts under `.planning/spikes/NNN-short-name/`.
- Keep root research summaries in Chinese when they are intended for project direction.

## Patterns

- External template inputs are treated as adapter evidence, not canonical render semantics.
- Offline fixture/formula input is preferred before live provider/API work.
- Android worker outputs are oracle/calibration evidence, not runtime dependencies.

## Tools & Libraries

- No additional tools established yet.

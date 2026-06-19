# Refactor And Legacy Cleanup Policy

This policy is a mandatory review gate for product-facing refactors.

## Rule

This project is a greenfield editor. When a product path is being upgraded, do
not over-preserve compatibility with incomplete, legacy, mock, fallback, or
temporary implementations. Replace the path with the intended current
architecture, and remove or gate obsolete code that would let normal users keep
using the old behavior.

Compatibility layers are allowed only for explicit external formats or platform
capability reports. They must be named as adapters, diagnostics, or unsupported
reports, not as product success paths.

## Review Checklist

Every review touching product behavior must check:

- Does the change remove obsolete paths instead of keeping them as hidden
  product behavior?
- Are legacy flags, mock backends, fallback labels, debug controls, and old
  aliases still reachable from normal UI flows?
- Does the new implementation use the latest intended architecture instead of
  wrapping the old path?
- Are any compatibility claims limited to explicit adapters or diagnostics?
- Do E2E tests fail when the old path is used as a substitute for the new one?


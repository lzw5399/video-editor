//! Test harness shell for fixtures, goldens, and render smoke checks.
//!
//! Later Phase 1 plans add deterministic schema fixtures, tiny media generation,
//! and FFmpeg render smoke helpers here. This shell exists so downstream plans
//! can depend on a stable testkit crate without introducing media behavior early.

/// Boundary marker for Phase 1 test harness helpers.
pub const TESTKIT_BOUNDARY: &str = "fixtures-goldens-render-smoke-shell";

//! Pure Rust command semantics for draft edits.
//!
//! This crate will own Jianying-style edit commands such as add, move, split,
//! trim, delete, undo/redo, snapping, and MainTrackMagnet behavior. It stays a
//! semantic layer: UI, filesystem, FFmpeg, preview, and platform execution
//! details belong outside this crate.

pub mod error;
pub mod history;
pub mod selection;
pub mod timeline;

pub use error::{TimelineCommandError, TimelineCommandErrorKind};
pub use selection::TimelineSelection;

/// Boundary marker for the command semantics crate.
pub const DRAFT_COMMANDS_BOUNDARY: &str = "pure-semantic";

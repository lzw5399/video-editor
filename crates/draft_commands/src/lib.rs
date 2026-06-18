//! Pure Rust command semantics for draft edits.
//!
//! This crate will own Jianying-style edit commands such as add, move, split,
//! trim, delete, undo/redo, snapping, and MainTrackMagnet behavior. It stays a
//! semantic layer: UI, filesystem, FFmpeg, preview, and platform execution
//! details belong outside this crate.

pub mod audio;
pub mod canvas;
pub mod delta;
pub mod error;
pub mod history;
pub mod keyframe;
pub mod selection;
pub mod snapping;
pub mod text;
pub mod timeline;
pub mod visual;

pub use error::{TimelineCommandError, TimelineCommandErrorKind};
pub use selection::TimelineSelection;

/// Boundary marker for the command semantics crate.
pub const DRAFT_COMMANDS_BOUNDARY: &str = "pure-semantic";

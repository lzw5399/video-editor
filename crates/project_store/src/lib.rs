//! `.veproj` project store service boundary.
//!
//! This crate owns filesystem abstraction for project bundle persistence. The
//! canonical project state will live in `.veproj/project.json`; previews,
//! waveforms, render graphs, FFmpeg scripts, and exports remain derived
//! artifacts outside the semantic draft model.

use std::io;
use std::path::{Path, PathBuf};

/// Filesystem boundary consumed by project persistence code.
pub trait PlatformFileSystem {
    /// Reads a UTF-8 project file from disk.
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// Writes a UTF-8 project file to disk, creating parent directories first
    /// when the platform supports it.
    fn write_string(&self, path: &Path, contents: &str) -> io::Result<()>;

    /// Returns whether a path exists.
    fn exists(&self, path: &Path) -> bool;
}

/// Standard desktop filesystem implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct StdPlatformFileSystem;

impl PlatformFileSystem for StdPlatformFileSystem {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn write_string(&self, path: &Path, contents: &str) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, contents)
    }

    fn exists(&self, path: &Path) -> bool {
        PathBuf::from(path).exists()
    }
}

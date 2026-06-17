//! `.veproj` project store service boundary.
//!
//! This crate owns filesystem abstraction for project bundle persistence. The
//! canonical project state will live in `.veproj/project.json`; previews,
//! waveforms, render graphs, FFmpeg scripts, and exports remain derived
//! artifacts outside the semantic draft model.

mod bundle;
mod error;
mod paths;

use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub use bundle::{
    ProjectBundle, ProjectBundleOpenResult, autosave_project_bundle, create_project_bundle,
    open_project_bundle, save_project_bundle,
};
pub use error::{ProjectStoreError, ProjectStoreWarning};
pub use paths::{
    MaterialUri, MaterialUriKind, PROJECT_JSON_FILE_NAME, classify_material_uri,
    material_uri_for_save, project_json_path, resolve_material_uri,
};

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

        let temp_path = atomic_temp_path(path);
        let write_result = (|| {
            let mut file = std::fs::File::create(&temp_path)?;
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
            std::fs::rename(&temp_path, path)
        })();

        if write_result.is_err() {
            let _ = std::fs::remove_file(&temp_path);
        }

        write_result
    }

    fn exists(&self, path: &Path) -> bool {
        PathBuf::from(path).exists()
    }
}

fn atomic_temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(PROJECT_JSON_FILE_NAME);
    path.with_file_name(format!(".{file_name}.tmp"))
}

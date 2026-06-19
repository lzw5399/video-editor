use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ArtifactStoreError {
    #[error("artifact store IO failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("artifact store SQLite operation failed at {path}: {source}")]
    Sqlite {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },
}

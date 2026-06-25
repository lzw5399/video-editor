use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeErrorKind {
    InvalidRequest,
    UnknownRuntimeSession,
    UnknownProjectSession,
    UnknownHandle,
    WrongKind,
    WrongOwner,
    WrongDevice,
    TextureMetadataMismatch,
    StaleGeneration,
    LeaseExpired,
    DoubleRelease,
    ProjectStore,
    Scheduler,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[error("runtime error: {message}")]
#[serde(rename_all = "camelCase")]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub message: String,
}

impl RuntimeError {
    pub fn new(kind: RuntimeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

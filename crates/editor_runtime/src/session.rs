use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{RuntimeError, RuntimeErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSessionId {
    token: String,
    generation: u64,
}

impl RuntimeSessionId {
    fn new(token: String, generation: u64) -> Self {
        Self { token, generation }
    }

    pub fn as_str(&self) -> &str {
        &self.token
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeSessionConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdapterMetadata {
    pub adapter: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeSession {
    pub id: RuntimeSessionId,
    pub diagnostic_label: Option<String>,
    pub adapter_metadata: Option<AdapterMetadata>,
}

#[derive(Debug, Default)]
pub struct RuntimeSessionRegistry {
    next_id: u64,
    sessions: BTreeMap<RuntimeSessionId, RuntimeSession>,
}

impl RuntimeSessionRegistry {
    pub fn create_session(
        &mut self,
        config: RuntimeSessionConfig,
    ) -> Result<RuntimeSession, RuntimeError> {
        let id_number = self.next_id.saturating_add(1);
        self.next_id = id_number;
        let session = RuntimeSession {
            id: RuntimeSessionId::new(format!("runtime-{id_number}"), 1),
            diagnostic_label: config.diagnostic_label,
            adapter_metadata: None,
        };
        self.sessions.insert(session.id.clone(), session.clone());
        Ok(session)
    }

    pub fn contains_session(&self, id: &RuntimeSessionId) -> bool {
        self.sessions.contains_key(id)
    }

    pub fn session(&self, id: &RuntimeSessionId) -> Result<&RuntimeSession, RuntimeError> {
        self.sessions.get(id).ok_or_else(|| {
            RuntimeError::new(
                RuntimeErrorKind::UnknownRuntimeSession,
                format!("runtime session not found: {}", id.as_str()),
            )
        })
    }
}

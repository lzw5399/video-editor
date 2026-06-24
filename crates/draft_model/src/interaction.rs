use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// High-frequency interaction families owned by the Rust project session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum ProjectInteractionKind {
    SelectedSegmentVisual,
    SelectedText,
    SelectedSegmentAudio,
    PlayheadScrub,
    TimelineMoveTrim,
    KeyframeEdit,
}

/// Durable metadata for one live interaction session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectInteractionSession {
    pub interaction_id: String,
    pub kind: ProjectInteractionKind,
    pub base_revision: u64,
    pub generation: u64,
    pub accepted_sequence: u64,
    pub coalesced_through: u64,
}

impl ProjectInteractionSession {
    pub fn new(
        interaction_id: impl Into<String>,
        kind: ProjectInteractionKind,
        base_revision: u64,
        generation: u64,
    ) -> Self {
        Self {
            interaction_id: interaction_id.into(),
            kind,
            base_revision,
            generation,
            accepted_sequence: 0,
            coalesced_through: 0,
        }
    }

    pub fn accept_sequence(
        &mut self,
        sequence: u64,
    ) -> Result<(), ProjectInteractionSequenceError> {
        if sequence == 0 {
            return Err(ProjectInteractionSequenceError::Zero);
        }
        if sequence <= self.accepted_sequence {
            return Err(ProjectInteractionSequenceError::Stale {
                accepted_sequence: self.accepted_sequence,
                received_sequence: sequence,
            });
        }
        self.accepted_sequence = sequence;
        self.coalesced_through = sequence;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectInteractionSequenceError {
    Zero,
    Stale {
        accepted_sequence: u64,
        received_sequence: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interaction_session_accepts_only_monotonic_sequences() {
        let mut session = ProjectInteractionSession::new(
            "interaction-1",
            ProjectInteractionKind::SelectedSegmentVisual,
            7,
            3,
        );

        assert_eq!(session.accepted_sequence, 0);
        assert_eq!(session.coalesced_through, 0);
        assert_eq!(
            session.accept_sequence(0),
            Err(ProjectInteractionSequenceError::Zero)
        );
        assert_eq!(session.accept_sequence(1), Ok(()));
        assert_eq!(session.accepted_sequence, 1);
        assert_eq!(session.coalesced_through, 1);
        assert_eq!(
            session.accept_sequence(1),
            Err(ProjectInteractionSequenceError::Stale {
                accepted_sequence: 1,
                received_sequence: 1,
            })
        );
        assert_eq!(
            session.accept_sequence(0),
            Err(ProjectInteractionSequenceError::Zero)
        );
        assert_eq!(session.accept_sequence(2), Ok(()));
        assert_eq!(session.accepted_sequence, 2);
    }
}

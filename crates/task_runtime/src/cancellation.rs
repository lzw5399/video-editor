use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

fn default_cancellation_state() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

/// Cloneable cancellation handle shared by queued, running, and completion gates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskCancellationToken {
    id: u64,
    #[serde(skip, default = "default_cancellation_state")]
    cancelled: Arc<AtomicBool>,
}

impl TaskCancellationToken {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            cancelled: default_cancellation_state(),
        }
    }

    pub const fn id(&self) -> u64 {
        self.id
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for TaskCancellationToken {
    fn default() -> Self {
        Self::new(0)
    }
}

impl PartialEq for TaskCancellationToken {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TaskCancellationToken {}

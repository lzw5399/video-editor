use crate::{JobScheduler, TaskRuntimeConfig};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FakeClock {
    now_us: u64,
}

impl FakeClock {
    pub const fn new(now_us: u64) -> Self {
        Self { now_us }
    }

    pub const fn now_us(self) -> u64 {
        self.now_us
    }

    pub fn advance_us(&mut self, delta_us: u64) {
        self.now_us = self.now_us.saturating_add(delta_us);
    }
}

pub fn scheduler_with_config(config: TaskRuntimeConfig) -> JobScheduler {
    JobScheduler::new(config)
}

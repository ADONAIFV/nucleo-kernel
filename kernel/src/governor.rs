//! Gestor de recursos (CPU, memoria, agentes, etc.)

use crate::config::ResourceLimits;
use parking_lot::Mutex;

pub struct ResourceGovernor {
    limits: ResourceLimits,
    used_agents: Mutex<usize>,
    used_memory_mb: Mutex<u64>,
    used_fds: Mutex<u64>,
}

impl ResourceGovernor {
    pub const fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            used_agents: Mutex::new(0),
            used_memory_mb: Mutex::new(0),
            used_fds: Mutex::new(0),
        }
    }

    pub fn try_alloc_agent(&self) -> bool {
        let mut used = self.used_agents.lock();
        if *used >= self.limits.max_agents {
            false
        } else {
            *used += 1;
            true
        }
    }

    pub fn free_agent(&self) {
        let mut used = self.used_agents.lock();
        if *used > 0 {
            *used -= 1;
        }
    }

    pub fn stats(&self) -> (usize, u64, u64) {
        (
            *self.used_agents.lock(),
            *self.used_memory_mb.lock(),
            *self.used_fds.lock(),
        )
    }
}

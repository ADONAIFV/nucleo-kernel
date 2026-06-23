//! Substrate - Primitivas del kernel para agentes autónomos.
//!
//! Proporciona los bloques de construcción fundamentales para el kernel puro.

extern crate alloc;
use serde::{Deserialize, Serialize};

pub mod branch;
pub mod eval;
pub mod ipc;
pub mod lego;
pub mod memory_store;
pub mod mmu;
pub mod process;
pub mod scheduler;
pub mod security;
pub mod untrusted;
pub mod wasm_runtime;
pub mod workspace;

// Re-exportar tipos principales
pub use branch::{BranchError, BranchState, CowBranchManager};
pub use eval::{Eval, EvalError};
pub use ipc::{IpcError, Message, MessageBus};
pub use lego::{Health, Lego, LegoRegistry};
pub use memory_store::{MemoryError, MemoryStore};
pub use mmu::{MemoryEntry, MemoryLevel, SemanticMmu};
pub use process::{AgentProcess, ProcessError, ProcessManager, ProcessStatus};
pub use scheduler::semantic::{
    SchedulerError, SchedulerStats, SemanticScheduler, Task, TaskPriority, TaskState,
};
pub use untrusted::{Trusted, Untrusted};
pub use wasm_runtime::{Language, WasmError, WasmResult, WasmRuntime};
pub use workspace::{SharePermission, SharedWorkspace, Workspace, WorkspaceError};

// Definición temporal de AgentId hasta que se decida su ubicación final
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct AgentId(pub u64);
impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Agent-{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    pub name: String,
    pub description: String,
    pub capabilities: crate::security::capability::CapabilitySet,
}

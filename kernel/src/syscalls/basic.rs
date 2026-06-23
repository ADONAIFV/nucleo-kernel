//! Syscalls básicas: workspace, eval, memory.

use anyhow::Result;
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use substrate::AgentId;
use substrate::eval::{Eval, EvalError};
use substrate::ipc::MessageBus;
use substrate::memory_store::{MemoryError, MemoryStore};
use substrate::mmu::SemanticMmu;
use substrate::untrusted::Untrusted;
use substrate::workspace::{SharedWorkspace, Workspace, WorkspaceError};

use crate::architecture::ArchitectureInspector;
use crate::hot_patch::HotPatchManager;
use crate::persistence::checkpoint::CheckpointManager;
use substrate::branch::{BranchError, BranchState, CowBranchManager};
use substrate::security::capability::{CapDomain, CapFlags, CapabilitySystem};

use crate::metrics::MetricsCollector;
use crate::modules::timer::TimerManager;
use crate::security::blocklist::Blocklist;

/// Contexto de syscalls para los agentes.
pub struct SyscallContext {
    // Base para workspaces aislados por agente y directorios compartidos
    workspace_base: PathBuf,
    // Almacenes de memoria para agentes
    stores: Arc<Mutex<HashMap<AgentId, Arc<MemoryStore>>>>,
    // MMU Semántica para gestión de memoria de agente
    pub mmu: SemanticMmu,
    // Gestor de directorios compartidos
    pub shared_workspace: SharedWorkspace,
    // Lista negra de syscalls y recursos
    pub blocklist: Blocklist,
    // Gestor de temporizadores
    pub timer_manager: TimerManager,
    // Recolector de métricas
    pub metrics_collector: MetricsCollector,
    // Bus de mensajes entre agentes
    pub message_bus: MessageBus,
    // Gestor de ramificaciones CoW
    pub branch_manager: Arc<CowBranchManager>,
    // Sistema de capacidades
    pub cap_system: Arc<CapabilitySystem>,
    // Gestor de checkpoints
    pub checkpoint_mgr: Arc<CheckpointManager>,
    pub hot_patch_manager: Arc<HotPatchManager>,
    pub architecture_inspector: Arc<ArchitectureInspector>,
    agents: Mutex<HashSet<AgentId>>,
}

impl SyscallContext {
    pub fn new(
        workspace_base: PathBuf,
        config_dir: &Path,
        branch_manager: Arc<CowBranchManager>,
        cap_system: Arc<CapabilitySystem>,
        checkpoint_mgr: Arc<CheckpointManager>,
        hot_patch_manager: Arc<HotPatchManager>,
        architecture_inspector: Arc<ArchitectureInspector>,
    ) -> Result<Self, anyhow::Error> {
        std::fs::create_dir_all(&workspace_base)?;

        Ok(Self {
            workspace_base: workspace_base.clone(),
            stores: Arc::new(Mutex::new(HashMap::new())),
            mmu: SemanticMmu::new(AgentId(0), &workspace_base.join("mmu_cache")), // Directorio para caché de MMU, AgentId(0) es el kernel
            shared_workspace: SharedWorkspace::new(),
            blocklist: Blocklist::new(&config_dir.join("security"))?,
            timer_manager: TimerManager::new(),
            metrics_collector: MetricsCollector::new(),
            message_bus: MessageBus::new(),
            branch_manager,
            cap_system,
            checkpoint_mgr,
            hot_patch_manager,
            architecture_inspector,
            agents: Mutex::new(HashSet::new()),
        })
    }

    pub fn workspace_base(&self) -> &PathBuf {
        &self.workspace_base
    }

    // Helper para obtener el MemoryStore de un agente, creándolo si no existe.
    fn get_store(&self, agent_id: AgentId) -> Result<Arc<MemoryStore>, MemoryError> {
        let mut stores = self.stores.lock();
        if let Some(store) = stores.get(&agent_id) {
            Ok(store.clone())
        } else {
            let store = Arc::new(MemoryStore::new(agent_id.0, &self.workspace_base)?);
            stores.insert(agent_id, store.clone());
            Ok(store)
        }
    }

    // --- Workspace (aislado por agente) ---

    pub fn workspace_write(
        &self,
        agent_id: AgentId,
        path: Untrusted<&str>,
        data: &[u8],
    ) -> Result<(), WorkspaceError> {
        let ws = Workspace::new(agent_id.0, &self.workspace_base)?;
        ws.write(path, data)
    }

    pub fn workspace_read(
        &self,
        agent_id: AgentId,
        path: Untrusted<&str>,
    ) -> Result<Vec<u8>, WorkspaceError> {
        let ws = Workspace::new(agent_id.0, &self.workspace_base)?;
        ws.read(path)
    }

    pub fn workspace_list(
        &self,
        agent_id: AgentId,
        path: Untrusted<&str>,
    ) -> Result<Vec<String>, WorkspaceError> {
        let ws = Workspace::new(agent_id.0, &self.workspace_base)?;
        ws.list(path)
    }

    pub fn workspace_delete(
        &self,
        agent_id: AgentId,
        path: Untrusted<&str>,
    ) -> Result<(), WorkspaceError> {
        let ws = Workspace::new(agent_id.0, &self.workspace_base)?;
        ws.delete(path)
    }

    pub fn workspace_exists(
        &self,
        agent_id: AgentId,
        path: Untrusted<&str>,
    ) -> Result<bool, WorkspaceError> {
        let ws = Workspace::new(agent_id.0, &self.workspace_base)?;
        ws.exists(path)
    }

    // --- Eval ---

    pub fn eval(
        &self,
        language: Untrusted<&str>,
        code: Untrusted<&str>,
        timeout: u64,
    ) -> Result<String, EvalError> {
        Eval::eval(language.as_ref(), code.as_ref(), timeout)
    }

    // --- Memory Store ---

    pub fn memory_store(
        &self,
        agent_id: AgentId,
        key: Untrusted<&str>,
        value: Untrusted<&str>,
        metadata: Untrusted<&str>,
    ) -> Result<(), MemoryError> {
        let store = self.get_store(agent_id)?;
        store.store(key.as_ref(), value.as_ref(), metadata.as_ref())
    }

    pub fn memory_get(
        &self,
        agent_id: AgentId,
        key: Untrusted<&str>,
    ) -> Result<Option<String>, MemoryError> {
        let store = self.get_store(agent_id)?;
        store.get(key.as_ref())
    }

    pub fn memory_search(
        &self,
        agent_id: AgentId,
        query: Untrusted<&str>,
    ) -> Result<Vec<substrate::memory_store::MemoryEntry>, MemoryError> {
        let store = self.get_store(agent_id)?;
        store.search(query.as_ref())
    }

    pub fn memory_delete(
        &self,
        agent_id: AgentId,
        key: Untrusted<&str>,
    ) -> Result<bool, MemoryError> {
        let store = self.get_store(agent_id)?;
        store.delete(key.as_ref())
    }

    pub fn memory_list(&self, agent_id: AgentId) -> Vec<String> {
        if let Ok(store) = self.get_store(agent_id) {
            store.list_keys()
        } else {
            Vec::new()
        }
    }

    // --- Wrappers para la CLI y Agentes ---

    pub fn list_agents(&self) -> Vec<AgentId> {
        self.agents.lock().iter().cloned().collect()
    }

    pub fn spawn_agent(&self, spec: substrate::AgentSpec) -> Result<AgentId, anyhow::Error> {
        let id = AgentId(self.agents.lock().len() as u64 + 1);
        self.agents.lock().insert(id);
        Ok(id)
    }

    pub fn stop_agent(&self, agent_id: AgentId) -> Result<(), anyhow::Error> {
        self.agents.lock().remove(&agent_id);
        Ok(())
    }

    pub fn agent_fork(&self, agent_id: AgentId) -> Result<u64, anyhow::Error> {
        Ok(self.branch_manager.fork(agent_id, None)?)
    }

    pub fn agent_commit(&self, agent_id: AgentId, branch_id: u64) -> Result<(), anyhow::Error> {
        Ok(self.branch_manager.commit(agent_id, branch_id)?)
    }

    pub fn list_branches(&self, agent_id: AgentId) -> Vec<substrate::branch::BranchState> {
        self.branch_manager.list_branches(agent_id)
    }

    pub fn create_checkpoint(
        &self,
        agent_id: AgentId,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<PathBuf, anyhow::Error> {
        Ok(self.checkpoint_mgr.checkpoint(agent_id, metadata)?)
    }

    pub fn restore_checkpoint(
        &self,
        agent_id: AgentId,
        path: &PathBuf,
    ) -> Result<(), anyhow::Error> {
        self.checkpoint_mgr
            .restore(agent_id, path)
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn list_checkpoints(&self, agent_id: AgentId) -> Vec<PathBuf> {
        self.checkpoint_mgr.list_checkpoints(agent_id)
    }
}

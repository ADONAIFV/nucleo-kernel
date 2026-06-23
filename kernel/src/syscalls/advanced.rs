//! Syscalls avanzadas para gestión de agentes, ramas, gobernanza y unikernels.

use crate::architecture::{
    ArchitectureInspector, KernelArchitecture, KernelTree, ModuleInfo, SystemState,
};
use crate::compiler::unikernel::UnikernelCompiler;
use crate::governance::inspector::AgentInspector;
use crate::hot_patch::HotPatchManager;
use crate::metrics::MetricType;
use crate::modules::timer::TimerId;
use crate::security::blocklist::BlocklistEntry;
use substrate::security::capability::CapDomain;

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use substrate::AgentId;
use substrate::branch::{BranchError, CowBranchManager};
use substrate::eval::EvalError;
use substrate::ipc::{IpcError, Message};
use substrate::mmu::MmuError;
use substrate::untrusted::Untrusted;
use substrate::workspace::{SharePermission, WorkspaceError};

use super::basic::SyscallContext;

/// Errores específicos de las syscalls avanzadas.
#[derive(Debug, thiserror::Error)]
pub enum AdvancedSyscallError {
    #[error("Error de Workspace: {0}")]
    Workspace(#[from] WorkspaceError),
    #[error("Error de IPC: {0}")]
    Ipc(#[from] IpcError),
    #[error("Error de MMU: {0}")]
    Mmu(#[from] MmuError),
    #[error("Error de evaluación: {0}")]
    Eval(#[from] EvalError),
    #[error("Error de Blocklist")]
    BlocklistError,
    #[error("Error de Timer")]
    TimerError,
    #[error("Error de Métricas")]
    MetricsError,
    #[error("Error de Branch: {0}")]
    Branch(#[from] BranchError),
    #[error("Error general: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("Error de Serialización/Deserialización: {0}")]
    SerdeJson(#[from] serde_json::Error),
}

/// Implementación de las syscalls avanzadas para los agentes.
pub struct AdvancedSyscalls<'a> {
    context: &'a SyscallContext,
    branch_manager: Arc<CowBranchManager>,
    hot_patch_manager: Arc<HotPatchManager>,
    agent_inspector: AgentInspector,
    architecture_inspector: Arc<ArchitectureInspector>,
    unikernel_compiler: UnikernelCompiler,
}

impl<'a> AdvancedSyscalls<'a> {
    pub fn new(
        context: &'a SyscallContext,
        branch_manager: Arc<CowBranchManager>,
        hot_patch_manager: Arc<HotPatchManager>,
        architecture_inspector: Arc<ArchitectureInspector>,
        compiler_output: PathBuf,
        target_arch: &str,
    ) -> Self {
        Self {
            context,
            branch_manager,
            hot_patch_manager,
            agent_inspector: AgentInspector::new(target_arch),
            architecture_inspector,
            unikernel_compiler: UnikernelCompiler::new(compiler_output, target_arch),
        }
    }

    // --- Branching (Fork, Commit, Discard) ---

    /// Bifurca el estado de un agente en una nueva rama.
    pub fn agent_fork(&self, agent_id: AgentId) -> Result<AgentId, AdvancedSyscallError> {
        let new_id = self.branch_manager.fork(agent_id, None)?;
        Ok(AgentId(new_id))
    }

    /// Compromete el estado de una rama con el estado principal.
    pub fn agent_commit(
        &self,
        original_id: AgentId,
        branch_id: AgentId,
    ) -> Result<(), AdvancedSyscallError> {
        self.branch_manager.commit(original_id, branch_id.0)?;
        Ok(())
    }

    /// Descartar una rama.
    pub fn agent_discard(
        &self,
        agent_id: AgentId,
        branch_id: AgentId,
    ) -> Result<(), AdvancedSyscallError> {
        self.branch_manager.abort(agent_id, branch_id.0)?;
        Ok(())
    }

    // --- Governance (Inspect Action) ---

    /// Solicita al inspector que evalúe una acción propuesta por el agente.
    pub fn inspect_action(
        &self,
        agent_id: AgentId,
        action_json: Untrusted<&str>,
    ) -> Result<bool, AdvancedSyscallError> {
        let action: Value = serde_json::from_str(action_json.as_ref())?;
        Ok(self.agent_inspector.inspect_action(agent_id, action))
    }

    // --- Unikernel Compilation ---

    /// Compila un agente dado en un unikernel.
    pub fn compile_unikernel(
        &self,
        agent_id: AgentId,
        _source_path: Untrusted<&str>,
    ) -> Result<PathBuf, AdvancedSyscallError> {
        let agent_ws_path = self
            .context
            .workspace_base()
            .join(format!("agent_workspaces/{}", agent_id.0));
        let output_path = self
            .unikernel_compiler
            .compile(agent_id.0, &agent_ws_path)?;
        Ok(output_path)
    }

    // --- Shared Workspace ---

    pub fn shared_workspace_create(
        &self,
        name: Untrusted<&str>,
        owner: AgentId,
    ) -> Result<(), AdvancedSyscallError> {
        self.context.shared_workspace.create_shared_dir(
            name.as_ref(),
            owner,
            self.context.workspace_base(),
        )?;
        Ok(())
    }

    pub fn shared_workspace_share(
        &self,
        name: Untrusted<&str>,
        agent_id: AgentId,
        permission: SharePermission,
    ) -> Result<(), AdvancedSyscallError> {
        self.context
            .shared_workspace
            .share_with(name.as_ref(), agent_id, permission)?;
        Ok(())
    }

    pub fn shared_workspace_read(
        &self,
        name: Untrusted<&str>,
        path: Untrusted<&str>,
        agent_id: AgentId,
    ) -> Result<Vec<u8>, AdvancedSyscallError> {
        Ok(self
            .context
            .shared_workspace
            .read_shared(name.as_ref(), path.as_ref(), agent_id)?)
    }

    pub fn shared_workspace_write(
        &self,
        name: Untrusted<&str>,
        path: Untrusted<&str>,
        data: Untrusted<&[u8]>,
        agent_id: AgentId,
    ) -> Result<(), AdvancedSyscallError> {
        self.context.shared_workspace.write_shared(
            name.as_ref(),
            path.as_ref(),
            data.as_ref(),
            agent_id,
        )?;
        Ok(())
    }

    pub fn shared_workspace_list(
        &self,
        name: Untrusted<&str>,
        path: Untrusted<&str>,
        agent_id: AgentId,
    ) -> Result<Vec<String>, AdvancedSyscallError> {
        Ok(self
            .context
            .shared_workspace
            .list_shared(name.as_ref(), path.as_ref(), agent_id)?)
    }

    // --- Semantic MMU ---

    pub fn mmu_read(
        &self,
        _agent_id: AgentId,
        address: Untrusted<&str>,
        _size: u64,
    ) -> Result<Vec<u8>, AdvancedSyscallError> {
        // La MMU semántica no maneja direcciones/tamaños directos, usa claves.
        // Simplemente cargamos por la clave (address)
        self.context
            .mmu
            .load(address.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Clave de memoria no encontrada").into())
    }

    pub fn mmu_write(
        &self,
        _agent_id: AgentId,
        address: Untrusted<&str>,
        data: Untrusted<&[u8]>,
    ) -> Result<(), AdvancedSyscallError> {
        // La MMU semántica no maneja direcciones/tamaños directos, usa claves y niveles.
        // Almacenamos en L1 por defecto para escrituras de agentes.
        self.context.mmu.store(
            address.as_ref(),
            data.as_ref(),
            substrate::mmu::MemoryLevel::L1,
        );
        Ok(())
    }

    pub fn mmu_flush_cache(&self, _agent_id: AgentId) -> Result<(), AdvancedSyscallError> {
        // La MMU semántica gestiona la caché automáticamente, no se necesita flush manual.
        Ok(())
    }

    // --- IPC ---

    pub fn ipc_publish(
        &self,
        from: AgentId,
        topic: Untrusted<&str>,
        payload: Untrusted<&str>,
    ) -> Result<(), AdvancedSyscallError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let message = Message {
            from,
            to: None,
            topic: topic.as_ref().to_string(),
            payload: payload.as_ref().to_string(),
            timestamp,
        };
        self.context.message_bus.publish(topic.as_ref(), message)?;
        Ok(())
    }

    pub fn ipc_subscribe(
        &self,
        agent_id: AgentId,
        topic: Untrusted<&str>,
    ) -> Result<(), AdvancedSyscallError> {
        self.context
            .message_bus
            .subscribe(agent_id, topic.as_ref())?;
        Ok(())
    }

    pub fn ipc_recv(
        &self,
        agent_id: AgentId,
        timeout_secs: u64,
    ) -> Result<Option<Message>, AdvancedSyscallError> {
        let rx = self.context.message_bus.get_agent_receiver(agent_id);
        let timeout = std::time::Duration::from_secs(timeout_secs);
        Ok(rx.recv_timeout(timeout).ok())
    }

    // --- Blocklist ---

    pub fn blocklist_is_blocked(
        &self,
        entry: BlocklistEntry,
    ) -> Result<bool, AdvancedSyscallError> {
        Ok(self.context.blocklist.is_blocked(&entry))
    }

    // --- Timer ---

    pub fn timer_schedule(
        &self,
        duration_secs: u64,
        callback_id: u64,
        data: Untrusted<&[u8]>,
    ) -> Result<TimerId, AdvancedSyscallError> {
        let duration = std::time::Duration::from_secs(duration_secs);
        Ok(self
            .context
            .timer_manager
            .schedule_timer(duration, callback_id, data.as_ref().to_vec()))
    }

    pub fn timer_cancel(&self, timer_id: TimerId) -> Result<bool, AdvancedSyscallError> {
        Ok(self.context.timer_manager.cancel_timer(timer_id))
    }

    // --- Metrics ---

    pub fn metrics_record(
        &self,
        name: Untrusted<&str>,
        metric_type: MetricType,
        value: f64,
        tags: Option<HashMap<String, String>>,
    ) -> Result<(), AdvancedSyscallError> {
        self.context
            .metrics_collector
            .record(name.as_ref(), metric_type, value, tags);
        Ok(())
    }

    /// Syscall para reemplazar un módulo existente en el kernel.
    pub fn replace_module(
        &self,
        agent_id: AgentId,
        module_name: &str,
        new_version: &str,
        wasm_code: &[u8],
    ) -> Result<(), AdvancedSyscallError> {
        // Verificar que el agente tiene permiso para modificar el kernel
        // (capacidad `admin_modules` o similar)
        if !self
            .context
            .cap_system
            .check(agent_id, &CapDomain::Admin, "modules", None)
        {
            return Err(anyhow::anyhow!("Permiso denegado").into());
        }

        Ok(self
            .hot_patch_manager
            .replace_module(agent_id, module_name, new_version, wasm_code)?)
    }

    /// Syscall para que el agente obtenga la arquitectura completa del kernel.
    pub fn get_kernel_architecture(
        &self,
        agent_id: AgentId,
    ) -> Result<KernelArchitecture, anyhow::Error> {
        // Verificar que el agente tiene permiso para leer la arquitectura
        // (capacidad `admin_read` o similar)
        if !self
            .context
            .cap_system
            .check(agent_id, &CapDomain::Admin, "read", None)
        {
            return Err(anyhow::anyhow!(
                "Permiso denegado para leer la arquitectura del kernel"
            ));
        }

        Ok(self.architecture_inspector.get_architecture())
    }

    /// Syscall para obtener solo la lista de módulos.
    pub fn list_modules(&self, agent_id: AgentId) -> Result<Vec<ModuleInfo>, anyhow::Error> {
        if !self
            .context
            .cap_system
            .check(agent_id, &CapDomain::Admin, "read", None)
        {
            return Err(anyhow::anyhow!("Permiso denegado"));
        }
        Ok(self.architecture_inspector.get_modules())
    }

    /// Syscall para obtener el árbol de archivos del kernel.
    pub fn get_kernel_tree(&self, agent_id: AgentId) -> Result<KernelTree, anyhow::Error> {
        if !self
            .context
            .cap_system
            .check(agent_id, &CapDomain::Admin, "read", None)
        {
            return Err(anyhow::anyhow!("Permiso denegado"));
        }
        Ok(self.architecture_inspector.get_tree())
    }

    /// Syscall para obtener el estado del sistema.
    pub fn get_system_state(&self, agent_id: AgentId) -> Result<SystemState, anyhow::Error> {
        if !self
            .context
            .cap_system
            .check(agent_id, &CapDomain::Admin, "read", None)
        {
            return Err(anyhow::anyhow!("Permiso denegado"));
        }
        Ok(self.architecture_inspector.get_system_state())
    }

    pub fn eval(
        &self,
        language: &str,
        code: &str,
        timeout: u64,
    ) -> Result<String, AdvancedSyscallError> {
        substrate::eval::Eval::eval(language, code, timeout).map_err(AdvancedSyscallError::Eval)
    }
}

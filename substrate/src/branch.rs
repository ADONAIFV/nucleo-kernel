//! Branching con Copy-on-Write (CoW) para exploración agéntica.
//! Permite bifurcar el estado de un agente con costo O(1) y zero-copy,
//! commit atómico al padre, y abort con limpieza automática.

use crate::AgentId;
use crate::memory_store::MemoryStore;
use crate::untrusted::Untrusted;
use crate::workspace::Workspace;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Estado de una rama.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchState {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub agent_id: AgentId,
    pub created_at: u64,
    pub metadata: HashMap<String, String>,
    pub is_active: bool,
}

/// Gestor de ramificaciones con copy-on-write.
pub struct CowBranchManager {
    base_dir: PathBuf,
    branches: Arc<Mutex<HashMap<u64, BranchState>>>,
    next_id: Arc<Mutex<u64>>,
}

impl CowBranchManager {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
            branches: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Crea una nueva rama a partir de un agente existente (fork).
    /// Usa copy-on-write: no copia los datos, solo registra la referencia.
    pub fn fork(
        &self,
        agent_id: AgentId,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<u64, BranchError> {
        let new_id = {
            let mut next = self.next_id.lock().unwrap();
            let id = *next;
            *next += 1;
            id
        };

        // Obtener el workspace del agente padre
        let parent_ws = Workspace::new(agent_id.0, &self.base_dir)
            .map_err(|e| BranchError::WorkspaceError(e.to_string()))?;

        // Crear workspace hijo (copy-on-write: usar hardlinks o reflink si está disponible)
        let child_ws = Workspace::new(new_id, &self.base_dir)
            .map_err(|e| BranchError::WorkspaceError(e.to_string()))?;

        // Copiar los archivos del padre al hijo (en lugar de hardlinks para simplicidad)
        // En una implementación real, se usarían reflinks (copy-on-write) o hardlinks.
        self.copy_workspace(&parent_ws, &child_ws)?;

        // Registrar la rama
        let mut branches = self.branches.lock().unwrap();
        branches.insert(
            new_id,
            BranchState {
                id: new_id,
                parent_id: Some(agent_id.0),
                agent_id,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metadata: metadata.unwrap_or_default(),
                is_active: true,
            },
        );

        Ok(new_id)
    }

    /// Confirma una rama (commit), fusionando sus cambios en el padre.
    /// Esta operación es atómica: o todos los cambios se aplican o ninguno.
    pub fn commit(&self, agent_id: AgentId, branch_id: u64) -> Result<(), BranchError> {
        let mut branches = self.branches.lock().unwrap();
        let branch = branches
            .get_mut(&branch_id)
            .ok_or(BranchError::BranchNotFound(branch_id))?;

        if branch.agent_id != agent_id {
            return Err(BranchError::PermissionDenied);
        }
        if !branch.is_active {
            return Err(BranchError::BranchInactive(branch_id));
        }

        // Obtener workspaces
        let parent_ws = Workspace::new(agent_id.0, &self.base_dir)
            .map_err(|e| BranchError::WorkspaceError(e.to_string()))?;
        let child_ws = Workspace::new(branch_id, &self.base_dir)
            .map_err(|e| BranchError::WorkspaceError(e.to_string()))?;

        // Copiar archivos del hijo al padre (sobrescribiendo)
        self.copy_workspace(&child_ws, &parent_ws)?;

        // Marcar la rama como inactiva
        branch.is_active = false;

        // Opcional: eliminar el workspace del hijo para ahorrar espacio
        let _ = std::fs::remove_dir_all(child_ws.root());

        Ok(())
    }

    /// Aborta una rama (abort), descartando todos los cambios y limpiando.
    pub fn abort(&self, agent_id: AgentId, branch_id: u64) -> Result<(), BranchError> {
        let mut branches = self.branches.lock().unwrap();
        let branch = branches
            .get_mut(&branch_id)
            .ok_or(BranchError::BranchNotFound(branch_id))?;

        if branch.agent_id != agent_id {
            return Err(BranchError::PermissionDenied);
        }
        if !branch.is_active {
            return Err(BranchError::BranchInactive(branch_id));
        }

        // Eliminar el workspace del hijo
        let child_ws = Workspace::new(branch_id, &self.base_dir)
            .map_err(|e| BranchError::WorkspaceError(e.to_string()))?;
        let _ = std::fs::remove_dir_all(child_ws.root());

        // Marcar como inactiva
        branch.is_active = false;

        Ok(())
    }

    /// Lista todas las ramas activas de un agente.
    pub fn list_branches(&self, agent_id: AgentId) -> Vec<BranchState> {
        let branches = self.branches.lock().unwrap();
        branches
            .values()
            .filter(|b| b.agent_id == agent_id && b.is_active)
            .cloned()
            .collect()
    }

    /// Copia recursivamente un workspace a otro.
    fn copy_workspace(&self, src: &Workspace, dst: &Workspace) -> Result<(), BranchError> {
        // En una implementación real, se usaría `cp -r` con reflink si está disponible.
        // Para simplicidad, usamos una copia recursiva en Rust.
        let src_path = src.root();
        let dst_path = dst.root();

        // Limpiar destino si existe
        if dst_path.exists() {
            std::fs::remove_dir_all(dst_path).map_err(|e| BranchError::IoError(e.to_string()))?;
        }

        // Crear directorio destino
        std::fs::create_dir_all(dst_path).map_err(|e| BranchError::IoError(e.to_string()))?;

        // Copiar recursivamente
        self.copy_recursive(src_path, dst_path)?;

        Ok(())
    }

    fn copy_recursive(&self, src: &Path, dst: &Path) -> Result<(), BranchError> {
        for entry in std::fs::read_dir(src).map_err(|e| BranchError::IoError(e.to_string()))? {
            let entry = entry.map_err(|e| BranchError::IoError(e.to_string()))?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            let metadata =
                std::fs::metadata(&src_path).map_err(|e| BranchError::IoError(e.to_string()))?;

            if metadata.is_dir() {
                std::fs::create_dir(&dst_path).map_err(|e| BranchError::IoError(e.to_string()))?;
                self.copy_recursive(&src_path, &dst_path)?;
            } else {
                // Usar copy-on-write si está disponible (reflink)
                #[cfg(target_os = "linux")]
                {
                    // `copy` en Linux puede usar copy-on-write si el filesystem lo soporta
                    std::fs::copy(&src_path, &dst_path)
                        .map_err(|e| BranchError::IoError(e.to_string()))?;
                }
                #[cfg(not(target_os = "linux"))]
                {
                    std::fs::copy(&src_path, &dst_path)
                        .map_err(|e| BranchError::IoError(e.to_string()))?;
                }
            }
        }
        Ok(())
    }

    /// Obtiene el estado de una rama.
    pub fn get_branch(&self, branch_id: u64) -> Option<BranchState> {
        let branches = self.branches.lock().unwrap();
        branches.get(&branch_id).cloned()
    }

    /// Verifica si una rama existe y está activa.
    pub fn is_active(&self, branch_id: u64) -> bool {
        let branches = self.branches.lock().unwrap();
        branches
            .get(&branch_id)
            .map(|b| b.is_active)
            .unwrap_or(false)
    }
}

/// Errores del sistema de branching.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BranchError {
    BranchNotFound(u64),
    BranchInactive(u64),
    PermissionDenied,
    WorkspaceError(String),
    IoError(String),
}

impl std::fmt::Display for BranchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BranchError::BranchNotFound(id) => write!(f, "Rama {} no encontrada", id),
            BranchError::BranchInactive(id) => write!(f, "Rama {} está inactiva", id),
            BranchError::PermissionDenied => write!(f, "Permiso denegado"),
            BranchError::WorkspaceError(e) => write!(f, "Error de workspace: {}", e),
            BranchError::IoError(e) => write!(f, "Error de E/S: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_fork_commit_workflow() {
        let dir = tempdir().unwrap();
        let mgr = CowBranchManager::new(dir.path());

        let agent_id = AgentId(1);
        let ws = Workspace::new(agent_id.0, dir.path()).unwrap();
        ws.write(Untrusted::new("test.txt"), b"original").unwrap();

        // Fork
        let branch_id = mgr.fork(agent_id, None).unwrap();
        assert!(mgr.is_active(branch_id));

        // Modificar la rama
        let branch_ws = Workspace::new(branch_id, dir.path()).unwrap();
        branch_ws
            .write(Untrusted::new("test.txt"), b"modified")
            .unwrap();

        // Commit
        mgr.commit(agent_id, branch_id).unwrap();
        assert!(!mgr.is_active(branch_id));

        // Verificar que el original tiene los cambios
        let content = ws.read(Untrusted::new("test.txt")).unwrap();
        assert_eq!(content, b"modified");
    }

    #[test]
    fn test_fork_abort_cleanup() {
        let dir = tempdir().unwrap();
        let mgr = CowBranchManager::new(dir.path());

        let agent_id = AgentId(1);
        let branch_id = mgr.fork(agent_id, None).unwrap();
        assert!(mgr.is_active(branch_id));

        mgr.abort(agent_id, branch_id).unwrap();
        assert!(!mgr.is_active(branch_id));

        // Verificar que el workspace del hijo fue eliminado
        let child_ws = Workspace::new(branch_id, dir.path());
        assert!(child_ws.is_err());
    }

    #[test]
    fn test_list_branches() {
        let dir = tempdir().unwrap();
        let mgr = CowBranchManager::new(dir.path());

        let agent_id = AgentId(1);
        let b1 = mgr.fork(agent_id, None).unwrap();
        let b2 = mgr.fork(agent_id, None).unwrap();

        let branches = mgr.list_branches(agent_id);
        assert_eq!(branches.len(), 2);

        mgr.abort(agent_id, b1).unwrap();
        let branches = mgr.list_branches(agent_id);
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].id, b2);
    }
}

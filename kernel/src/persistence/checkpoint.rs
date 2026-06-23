//! Checkpoints atómicos para agentes.
//! Permite guardar el estado completo de un agente (workspace, memoria, tareas)
//! y reanudarlo tras un crash.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use substrate::AgentId;
use substrate::memory_store::MemoryStore;
use substrate::untrusted::Untrusted;
use substrate::workspace::Workspace;

/// Checkpoint completo de un agente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCheckpoint {
    pub agent_id: u64,
    pub created_at: u64,
    pub version: u32,
    pub workspace_hash: String,
    pub memory_hash: String,
    pub metadata: HashMap<String, String>,
}

/// Gestor de checkpoints atómicos.
pub struct CheckpointManager {
    base_dir: PathBuf,
    current_version: u32,
}

impl CheckpointManager {
    pub fn new(base_dir: &Path) -> Self {
        let checkpoint_dir = base_dir.join("checkpoints");
        fs::create_dir_all(&checkpoint_dir).unwrap_or_default();

        Self {
            base_dir: checkpoint_dir,
            current_version: 1,
        }
    }

    /// Crea un checkpoint atómico de un agente.
    pub fn checkpoint(
        &self,
        agent_id: AgentId,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<PathBuf, CheckpointError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let check_dir = self.base_dir.join(format!("agent_{}", agent_id.0));
        fs::create_dir_all(&check_dir).map_err(|e| CheckpointError::IoError(e.to_string()))?;

        // Crear directorio temporal para atomicidad
        let temp_dir = check_dir.join(format!("tmp_{}", timestamp));
        fs::create_dir(&temp_dir).map_err(|e| CheckpointError::IoError(e.to_string()))?;

        // 1. Guardar workspace
        let ws = Workspace::new(
            agent_id.0,
            &self.base_dir.parent().unwrap_or(Path::new(".")),
        )
        .map_err(CheckpointError::WorkspaceError)?;

        let ws_dir = temp_dir.join("workspace");
        fs::create_dir(&ws_dir).map_err(|e| CheckpointError::IoError(e.to_string()))?;

        // Copiar archivos del workspace al checkpoint
        Self::copy_workspace(&ws, &ws_dir)?;

        // 2. Guardar memoria
        let store = MemoryStore::new(
            agent_id.0,
            &self.base_dir.parent().unwrap_or(Path::new(".")),
        )
        .map_err(|e| CheckpointError::IoError(e.to_string()))?;

        let mem_path = temp_dir.join("memory.json");
        let mut memory_data = HashMap::new();
        for key in store.list_keys() {
            if let Ok(Some(value)) = store.get(&key) {
                memory_data.insert(key, value);
            }
        }
        let mem_content = serde_json::to_string(&memory_data)
            .map_err(|e| CheckpointError::SerializationError(e.to_string()))?;
        fs::write(&mem_path, mem_content).map_err(|e| CheckpointError::IoError(e.to_string()))?;

        // 3. Guardar metadatos del checkpoint
        let checkpoint = AgentCheckpoint {
            agent_id: agent_id.0,
            created_at: timestamp,
            version: self.current_version,
            workspace_hash: Self::hash_dir(&ws_dir)?,
            memory_hash: Self::hash_file(&mem_path)?,
            metadata: metadata.unwrap_or_default(),
        };

        let meta_path = temp_dir.join("checkpoint.json");
        let meta_content = serde_json::to_string_pretty(&checkpoint)
            .map_err(|e| CheckpointError::SerializationError(e.to_string()))?;
        fs::write(&meta_path, meta_content).map_err(|e| CheckpointError::IoError(e.to_string()))?;

        // 4. Renombrar atómicamente
        let final_dir = check_dir.join(format!("checkpoint_{}", timestamp));
        fs::rename(&temp_dir, &final_dir).map_err(|e| CheckpointError::IoError(e.to_string()))?;

        Ok(final_dir)
    }

    /// Restaura un checkpoint.
    pub fn restore(
        &self,
        agent_id: AgentId,
        checkpoint_path: &Path,
    ) -> Result<(), CheckpointError> {
        let meta_path = checkpoint_path.join("checkpoint.json");
        if !meta_path.exists() {
            return Err(CheckpointError::NotFound);
        }

        let meta_content =
            fs::read_to_string(&meta_path).map_err(|e| CheckpointError::IoError(e.to_string()))?;
        let checkpoint: AgentCheckpoint = serde_json::from_str(&meta_content)
            .map_err(|e| CheckpointError::SerializationError(e.to_string()))?;

        // Verificar integridad
        let ws_hash = Self::hash_dir(&checkpoint_path.join("workspace"))?;
        if ws_hash != checkpoint.workspace_hash {
            return Err(CheckpointError::IntegrityError(
                "Workspace hash mismatch".to_string(),
            ));
        }

        let mem_hash = Self::hash_file(&checkpoint_path.join("memory.json"))?;
        if mem_hash != checkpoint.memory_hash {
            return Err(CheckpointError::IntegrityError(
                "Memory hash mismatch".to_string(),
            ));
        }

        // Restaurar workspace
        let ws = Workspace::new(
            agent_id.0,
            &self.base_dir.parent().unwrap_or(Path::new(".")),
        )
        .map_err(CheckpointError::WorkspaceError)?;

        // Limpiar workspace actual
        for entry in ws.list(Untrusted::new("."))? {
            let _ = ws.delete(Untrusted::new(&entry));
        }

        // Copiar archivos del checkpoint al workspace
        Self::copy_workspace_from(&checkpoint_path.join("workspace"), &ws)?;

        // Restaurar memoria
        let store = MemoryStore::new(
            agent_id.0,
            &self.base_dir.parent().unwrap_or(Path::new(".")),
        )
        .map_err(|e| CheckpointError::IoError(e.to_string()))?;

        // Limpiar memoria actual
        for key in store.list_keys() {
            let _ = store.delete(&key);
        }

        let mem_content = fs::read_to_string(&checkpoint_path.join("memory.json"))
            .map_err(|e| CheckpointError::IoError(e.to_string()))?;
        let memory_data: HashMap<String, String> = serde_json::from_str(&mem_content)
            .map_err(|e| CheckpointError::SerializationError(e.to_string()))?;

        for (key, value) in memory_data {
            let _ = store.store(&key, &value, "");
        }

        Ok(())
    }

    /// Lista todos los checkpoints de un agente.
    pub fn list_checkpoints(&self, agent_id: AgentId) -> Vec<PathBuf> {
        let dir = self.base_dir.join(format!("agent_{}", agent_id.0));
        if !dir.exists() {
            return Vec::new();
        }

        let mut checkpoints = Vec::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("checkpoint.json").exists() {
                    checkpoints.push(path);
                }
            }
        }
        checkpoints.sort();
        checkpoints
    }

    /// Obtiene el último checkpoint de un agente.
    pub fn get_latest_checkpoint(&self, agent_id: AgentId) -> Option<PathBuf> {
        let mut checkpoints = self.list_checkpoints(agent_id);
        checkpoints.pop()
    }

    /// Elimina un checkpoint.
    pub fn delete_checkpoint(&self, checkpoint_path: &Path) -> Result<(), CheckpointError> {
        fs::remove_dir_all(checkpoint_path).map_err(|e| CheckpointError::IoError(e.to_string()))
    }

    // --- Helpers ---

    fn copy_workspace(src: &Workspace, dst: &Path) -> Result<(), CheckpointError> {
        let src_path = src.root();
        for entry in
            std::fs::read_dir(src_path).map_err(|e| CheckpointError::IoError(e.to_string()))?
        {
            let entry = entry.map_err(|e| CheckpointError::IoError(e.to_string()))?;
            let src_entry = entry.path();
            let dst_entry = dst.join(entry.file_name());

            let metadata = std::fs::metadata(&src_entry)
                .map_err(|e| CheckpointError::IoError(e.to_string()))?;

            if metadata.is_dir() {
                std::fs::create_dir(&dst_entry)
                    .map_err(|e| CheckpointError::IoError(e.to_string()))?;
                Self::copy_dir(&src_entry, &dst_entry)?;
            } else {
                std::fs::copy(&src_entry, &dst_entry)
                    .map_err(|e| CheckpointError::IoError(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn copy_dir(src: &Path, dst: &Path) -> Result<(), CheckpointError> {
        for entry in std::fs::read_dir(src).map_err(|e| CheckpointError::IoError(e.to_string()))? {
            let entry = entry.map_err(|e| CheckpointError::IoError(e.to_string()))?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            let metadata = std::fs::metadata(&src_path)
                .map_err(|e| CheckpointError::IoError(e.to_string()))?;

            if metadata.is_dir() {
                std::fs::create_dir(&dst_path)
                    .map_err(|e| CheckpointError::IoError(e.to_string()))?;
                Self::copy_dir(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)
                    .map_err(|e| CheckpointError::IoError(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn copy_workspace_from(src: &Path, dst: &Workspace) -> Result<(), CheckpointError> {
        for entry in std::fs::read_dir(src).map_err(|e| CheckpointError::IoError(e.to_string()))? {
            let entry = entry.map_err(|e| CheckpointError::IoError(e.to_string()))?;
            let src_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            let metadata = std::fs::metadata(&src_path)
                .map_err(|e| CheckpointError::IoError(e.to_string()))?;

            if metadata.is_dir() {
                dst.write(Untrusted::new(&format!("{}/", name)), b"")
                    .map_err(CheckpointError::WorkspaceError)?;
                Self::copy_dir_to_workspace(&src_path, dst, &name)?;
            } else {
                let content = std::fs::read(&src_path)
                    .map_err(|e| CheckpointError::IoError(e.to_string()))?;
                dst.write(Untrusted::new(&name), &content)
                    .map_err(CheckpointError::WorkspaceError)?;
            }
        }
        Ok(())
    }

    fn copy_dir_to_workspace(
        src: &Path,
        dst: &Workspace,
        prefix: &str,
    ) -> Result<(), CheckpointError> {
        for entry in std::fs::read_dir(src).map_err(|e| CheckpointError::IoError(e.to_string()))? {
            let entry = entry.map_err(|e| CheckpointError::IoError(e.to_string()))?;
            let src_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let full_name = format!("{}/{}", prefix, name);

            let metadata = std::fs::metadata(&src_path)
                .map_err(|e| CheckpointError::IoError(e.to_string()))?;

            if metadata.is_dir() {
                dst.write(Untrusted::new(&format!("{}/", full_name)), b"")
                    .map_err(CheckpointError::WorkspaceError)?;
                Self::copy_dir_to_workspace(&src_path, dst, &full_name)?;
            } else {
                let content = std::fs::read(&src_path)
                    .map_err(|e| CheckpointError::IoError(e.to_string()))?;
                dst.write(Untrusted::new(&full_name), &content)
                    .map_err(CheckpointError::WorkspaceError)?;
            }
        }
        Ok(())
    }

    fn hash_dir(dir: &Path) -> Result<String, CheckpointError> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        let mut files: Vec<PathBuf> = Vec::new();

        for entry in walkdir::WalkDir::new(dir) {
            let entry = entry.map_err(|e| CheckpointError::IoError(e.to_string()))?;
            if entry.file_type().is_file() {
                files.push(entry.path().to_path_buf());
            }
        }

        files.sort();
        for file in files {
            let content = fs::read(&file).map_err(|e| CheckpointError::IoError(e.to_string()))?;
            hasher.update(&content);
            hasher.update(file.to_string_lossy().as_bytes());
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    fn hash_file(file: &Path) -> Result<String, CheckpointError> {
        use sha2::{Digest, Sha256};
        let content = fs::read(file).map_err(|e| CheckpointError::IoError(e.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        Ok(format!("{:x}", hasher.finalize()))
    }
}

/// Errores del sistema de checkpoints.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CheckpointError {
    NotFound,
    IntegrityError(String),
    WorkspaceError(#[from] substrate::workspace::WorkspaceError),
    IoError(String),
    SerializationError(String),
}

impl std::fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckpointError::NotFound => write!(f, "Checkpoint no encontrado"),
            CheckpointError::IntegrityError(e) => write!(f, "Error de integridad: {}", e),
            CheckpointError::WorkspaceError(e) => write!(f, "Error de workspace: {}", e),
            CheckpointError::IoError(e) => write!(f, "Error de E/S: {}", e),
            CheckpointError::SerializationError(e) => write!(f, "Error de serialización: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_checkpoint_create_restore() {
        let dir = tempdir().unwrap();
        let root_path = dir.path().to_path_buf();
        let mgr = CheckpointManager::new(&root_path);

        let agent_id = AgentId(1);
        // Asegurar que la base existe para el workspace
        let ws = Workspace::new(agent_id.0, &root_path).unwrap();
        ws.write(Untrusted::new("test.txt"), b"original").unwrap();

        // Crear checkpoint
        let cp_path = mgr.checkpoint(agent_id, None).unwrap();
        assert!(cp_path.exists());

        // Modificar workspace
        ws.write(Untrusted::new("test.txt"), b"modified").unwrap();
        let content = ws.read(Untrusted::new("test.txt")).unwrap();
        assert_eq!(content, b"modified");

        // Restaurar checkpoint
        mgr.restore(agent_id, &cp_path).unwrap();

        // Verificar que se restauró el estado original
        let content = ws.read(Untrusted::new("test.txt")).unwrap();
        assert_eq!(content, b"original");
    }

    #[test]
    fn test_checkpoint_list() {
        let dir = tempdir().unwrap();
        let root_path = dir.path().to_path_buf();
        let mgr = CheckpointManager::new(&root_path);

        let agent_id = AgentId(1);
        let ws = Workspace::new(agent_id.0, &root_path).unwrap();
        ws.write(Untrusted::new("file1.txt"), b"data").unwrap();

        mgr.checkpoint(agent_id, None).unwrap();
        mgr.checkpoint(agent_id, None).unwrap();
        mgr.checkpoint(agent_id, None).unwrap();

        let checkpoints = mgr.list_checkpoints(agent_id);
        assert_eq!(checkpoints.len(), 3);
    }
}

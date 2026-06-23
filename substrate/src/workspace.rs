#![allow(clippy::collapsible_if)]

//! Gestión de workspaces aislados por agente.

extern crate alloc;

use crate::AgentId;
use crate::untrusted::Untrusted;
use alloc::string::String;
use alloc::vec::Vec;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;

/// Error del sistema de workspaces.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkspaceError {
    #[error("Permiso denegado")]
    PermissionDenied,
    #[error("No encontrado")]
    NotFound,
    #[error("Ya existe")]
    AlreadyExists,
    #[error("Ruta inválida")]
    InvalidPath,
    #[error("Error de E/S: {0}")]
    IoError(String),
}

/// Workspace de un agente.
pub struct Workspace {
    root_dir: PathBuf,
}

impl Workspace {
    /// Crea o carga un workspace para un agente específico.
    pub fn new(agent_id: u64, base_dir: &Path) -> Result<Self, WorkspaceError> {
        let root_dir = base_dir.join(format!("agent_workspaces/{}", agent_id));
        fs::create_dir_all(&root_dir).map_err(|e| WorkspaceError::IoError(e.to_string()))?;

        Ok(Self { root_dir })
    }

    /// Resuelve una ruta dentro del workspace, asegurando que no escape del directorio raíz.
    fn resolve_path(&self, untrusted_path: Untrusted<&str>) -> Result<PathBuf, WorkspaceError> {
        let path = PathBuf::from(untrusted_path.as_ref());
        let resolved_path = self.root_dir.join(&path);

        // Validar que la ruta resuelta sea un subdirectorio del root_dir
        // y que no contenga componentes ".." para evitar Path Traversal
        let canonical_root = self.root_dir.canonicalize().map_err(|e| {
            WorkspaceError::IoError(format!("Error canonicalizando root_dir: {}", e))
        })?;
        let canonical_resolved = resolved_path.canonicalize().map_err(|e| {
            WorkspaceError::IoError(format!("Error canonicalizando ruta resuelta: {}", e))
        })?;

        if !canonical_resolved.starts_with(&canonical_root) {
            return Err(WorkspaceError::InvalidPath);
        }

        Ok(resolved_path)
    }

    /// Lee un archivo del workspace.
    pub fn read(&self, untrusted_path: Untrusted<&str>) -> Result<Vec<u8>, WorkspaceError> {
        let resolved_path = self.resolve_path(untrusted_path)?;
        fs::read(&resolved_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                WorkspaceError::PermissionDenied
            } else if e.kind() == std::io::ErrorKind::NotFound {
                WorkspaceError::NotFound
            } else {
                WorkspaceError::IoError(e.to_string())
            }
        })
    }

    /// Escribe un archivo en el workspace.
    pub fn write(
        &self,
        untrusted_path: Untrusted<&str>,
        content: &[u8],
    ) -> Result<(), WorkspaceError> {
        let resolved_path = self.resolve_path(untrusted_path)?;
        if let Some(parent) = resolved_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| WorkspaceError::IoError(e.to_string()))?;
            }
        }
        fs::write(&resolved_path, content).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                WorkspaceError::PermissionDenied
            } else {
                WorkspaceError::IoError(e.to_string())
            }
        })
    }

    /// Lista el contenido de un directorio dentro del workspace.
    pub fn list(&self, untrusted_path: Untrusted<&str>) -> Result<Vec<String>, WorkspaceError> {
        let resolved_path = self.resolve_path(untrusted_path)?;
        let mut files = Vec::new();
        for entry in fs::read_dir(&resolved_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                WorkspaceError::NotFound
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                WorkspaceError::PermissionDenied
            } else {
                WorkspaceError::IoError(e.to_string())
            }
        })? {
            let entry = entry.map_err(|e| WorkspaceError::IoError(e.to_string()))?;
            let path = entry.path();
            if let Ok(relative_path) = path.strip_prefix(&self.root_dir) {
                if let Some(s) = relative_path.to_str() {
                    files.push(String::from(s));
                }
            }
        }
        Ok(files)
    }

    /// Elimina un archivo o directorio del workspace.
    pub fn delete(&self, untrusted_path: Untrusted<&str>) -> Result<(), WorkspaceError> {
        let resolved_path = self.resolve_path(untrusted_path)?;
        if resolved_path.is_file() {
            fs::remove_file(&resolved_path)
        } else if resolved_path.is_dir() {
            fs::remove_dir_all(&resolved_path)
        } else {
            return Err(WorkspaceError::NotFound);
        }
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                WorkspaceError::PermissionDenied
            } else if e.kind() == std::io::ErrorKind::NotFound {
                WorkspaceError::NotFound
            } else {
                WorkspaceError::IoError(e.to_string())
            }
        })
    }

    pub fn exists(&self, untrusted_path: Untrusted<&str>) -> Result<bool, WorkspaceError> {
        let resolved_path = self.resolve_path(untrusted_path)?;
        Ok(resolved_path.exists())
    }

    pub fn root(&self) -> &Path {
        &self.root_dir
    }
}

/// Permisos de acceso a un workspace compartido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharePermission {
    ReadOnly,
    ReadWrite,
    WriteOnly,
}

/// Gestor de workspaces compartidos.
pub struct SharedWorkspace {
    shared_dirs: Mutex<HashMap<String, SharedDir>>,
}

impl Default for SharedWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct SharedDir {
    path: PathBuf,
    owner: AgentId,
    permissions: HashMap<AgentId, SharePermission>,
}

impl SharedWorkspace {
    pub fn new() -> Self {
        Self {
            shared_dirs: Mutex::new(HashMap::new()),
        }
    }

    /// Crea un directorio compartido.
    pub fn create_shared_dir(
        &self,
        name: &str,
        owner: AgentId,
        base_path: &Path,
    ) -> Result<PathBuf, WorkspaceError> {
        let path = base_path.join("shared").join(name);
        fs::create_dir_all(&path).map_err(|e| WorkspaceError::IoError(e.to_string()))?;

        let mut shared = self.shared_dirs.lock().unwrap();
        let dir = SharedDir {
            path: path.clone(),
            owner,
            permissions: HashMap::new(),
        };
        shared.insert(name.to_string(), dir);

        Ok(path)
    }

    /// Comparte un directorio con otro agente.
    pub fn share_with(
        &self,
        name: &str,
        agent_id: AgentId,
        permission: SharePermission,
    ) -> Result<(), WorkspaceError> {
        let mut shared = self.shared_dirs.lock().unwrap();
        if let Some(dir) = shared.get_mut(name) {
            dir.permissions.insert(agent_id, permission);
            Ok(())
        } else {
            Err(WorkspaceError::NotFound)
        }
    }

    /// Lee un archivo de un directorio compartido.
    pub fn read_shared(
        &self,
        name: &str,
        path: &str,
        agent_id: AgentId,
    ) -> Result<Vec<u8>, WorkspaceError> {
        let shared = self.shared_dirs.lock().unwrap();
        let dir = shared.get(name).ok_or(WorkspaceError::NotFound)?;

        // Verificar permisos
        let perm = dir
            .permissions
            .get(&agent_id)
            .unwrap_or(&SharePermission::ReadOnly);
        if matches!(perm, SharePermission::ReadOnly | SharePermission::ReadWrite) {
            let full_path = dir.path.join(path);
            fs::read(&full_path).map_err(|e| WorkspaceError::IoError(e.to_string()))
        } else {
            Err(WorkspaceError::PermissionDenied)
        }
    }

    /// Escribe un archivo en un directorio compartido.
    pub fn write_shared(
        &self,
        name: &str,
        path: &str,
        data: &[u8],
        agent_id: AgentId,
    ) -> Result<(), WorkspaceError> {
        let shared = self.shared_dirs.lock().unwrap();
        let dir = shared.get(name).ok_or(WorkspaceError::NotFound)?;

        let perm = dir
            .permissions
            .get(&agent_id)
            .unwrap_or(&SharePermission::ReadOnly);
        if matches!(
            perm,
            SharePermission::ReadWrite | SharePermission::WriteOnly
        ) {
            let full_path = dir.path.join(path);
            if let Some(parent) = full_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)
                        .map_err(|e| WorkspaceError::IoError(e.to_string()))?;
                }
            }
            fs::write(&full_path, data).map_err(|e| WorkspaceError::IoError(e.to_string()))
        } else {
            Err(WorkspaceError::PermissionDenied)
        }
    }

    /// Lista el contenido de un directorio compartido.
    pub fn list_shared(
        &self,
        name: &str,
        path: &str,
        agent_id: AgentId,
    ) -> Result<Vec<String>, WorkspaceError> {
        let shared = self.shared_dirs.lock().unwrap();
        let dir = shared.get(name).ok_or(WorkspaceError::NotFound)?;

        // Cualquier agente con permiso puede listar
        if dir.permissions.contains_key(&agent_id) || dir.owner == agent_id {
            let full_path = dir.path.join(path);
            let entries =
                fs::read_dir(&full_path).map_err(|e| WorkspaceError::IoError(e.to_string()))?;
            let mut result = Vec::new();
            for entry in entries {
                let entry = entry.map_err(|e| WorkspaceError::IoError(e.to_string()))?;
                result.push(entry.file_name().to_string_lossy().to_string());
            }
            Ok(result)
        } else {
            Err(WorkspaceError::PermissionDenied)
        }
    }
}

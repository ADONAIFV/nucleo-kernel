//! Almacenamiento persistente de memoria para agentes.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MemoryError {
    #[error("Permiso denegado")]
    PermissionDenied,
    #[error("No encontrado")]
    NotFound,
    #[error("La clave ya existe")]
    KeyExists,
    #[error("Error de E/S: {0}")]
    IoError(String),
    #[error("Error de serialización/deserialización: {0}")]
    SerializationError(String),
    #[error("Clave inválida")]
    InvalidKey,
    #[error("Valor inválido")]
    InvalidValue,
    #[error("Consulta inválida")]
    InvalidQuery,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub metadata: String,
    pub timestamp: u64,
}

pub struct MemoryStore {
    path: PathBuf,
    cache: Mutex<Vec<MemoryEntry>>,
}

impl MemoryStore {
    pub fn new(agent_id: u64, root: &Path) -> Result<Self, MemoryError> {
        let agent_dir = root.join(format!("agent_{}", agent_id));
        if !agent_dir.exists() {
            fs::create_dir_all(&agent_dir).map_err(|e| MemoryError::IoError(e.to_string()))?;
        }
        let path = agent_dir.join("memory.json");

        // Crear archivo si no existe
        if !path.exists() {
            fs::write(&path, "[]").map_err(|e| MemoryError::IoError(e.to_string()))?;
        }

        let initial_data = Self::load_data_from_path(&path)?;

        Ok(Self {
            path,
            cache: Mutex::new(initial_data),
        })
    }

    pub fn store(&self, key: &str, value: &str, metadata: &str) -> Result<(), MemoryError> {
        if key.is_empty() || key.contains('/') {
            return Err(MemoryError::InvalidKey);
        }
        if value.is_empty() {
            return Err(MemoryError::InvalidValue);
        }

        let mut cache = self.cache.lock().unwrap();
        let mut data = Self::load_data_from_path(&self.path)?;

        // Update existing entry or add new
        if let Some(entry) = data.iter_mut().find(|e| e.key == key) {
            entry.value = value.to_string();
            entry.metadata = metadata.to_string();
            entry.timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        } else {
            data.push(MemoryEntry {
                key: key.to_string(),
                value: value.to_string(),
                metadata: metadata.to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            });
        }

        cache.clone_from(&data); // Update in-memory cache
        self.save_data_to_path(&self.path, data)
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, MemoryError> {
        let cache = self.cache.lock().unwrap();
        Ok(cache.iter().find(|e| e.key == key).map(|e| e.value.clone()))
    }

    pub fn search(&self, query: &str) -> Result<Vec<MemoryEntry>, MemoryError> {
        let cache = self.cache.lock().unwrap();
        let results = cache
            .iter()
            .filter(|e| {
                e.key.contains(query) || e.value.contains(query) || e.metadata.contains(query)
            })
            .cloned()
            .collect();
        Ok(results)
    }

    pub fn delete(&self, key: &str) -> Result<bool, MemoryError> {
        let mut cache = self.cache.lock().unwrap();
        let initial_len = cache.len();
        cache.retain(|e| e.key != key);
        if cache.len() < initial_len {
            self.save_data_to_path(&self.path, cache.to_vec())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn list_keys(&self) -> Vec<String> {
        let cache = self.cache.lock().unwrap();
        cache.iter().map(|e| e.key.clone()).collect()
    }

    fn load_data_from_path(path: &Path) -> Result<Vec<MemoryEntry>, MemoryError> {
        let content = fs::read_to_string(path).map_err(|e| MemoryError::IoError(e.to_string()))?;
        if content.is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(&content).map_err(|e| MemoryError::SerializationError(e.to_string()))
    }

    fn save_data_to_path(&self, path: &Path, data: Vec<MemoryEntry>) -> Result<(), MemoryError> {
        let content = serde_json::to_string_pretty(&data)
            .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
        fs::write(path, content).map_err(|e| MemoryError::IoError(e.to_string()))
    }
}

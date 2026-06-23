#![allow(clippy::module_inception)]

//! MMU semántica para gestión de memoria jerárquica (L1/L2/L3).
//! Trata la memoria del agente como un sistema de caché.

use crate::AgentId;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Nivel de memoria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryLevel {
    L1 = 0, // RAM (ultrarrápido)
    L2 = 1, // Disco rápido (SSD)
    L3 = 2, // Disco lento / nube
}

/// Entrada de memoria con metadatos.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub key: String,
    pub data: Vec<u8>,
    pub level: MemoryLevel,
    pub last_access: u64,
    pub access_count: u64,
    pub size: usize,
}

/// MMU semántica para un agente.
pub struct SemanticMmu {
    #[allow(dead_code)]
    agent_id: AgentId,
    #[allow(dead_code)]
    base_dir: PathBuf,
    l1_cache: Mutex<HashMap<String, MemoryEntry>>, // RAM
    l2_cache: Mutex<HashMap<String, MemoryEntry>>, // SSD
    max_l1_entries: usize,
    max_l2_entries: usize,
    l3_dir: PathBuf, // Disco lento
}

impl SemanticMmu {
    pub fn new(agent_id: AgentId, base_dir: &Path) -> Self {
        let l3_dir = base_dir.join("l3").join(format!("agent_{}", agent_id.0));
        fs::create_dir_all(&l3_dir).unwrap_or_default();

        Self {
            agent_id,
            base_dir: base_dir.to_path_buf(),
            l1_cache: Mutex::new(HashMap::new()),
            l2_cache: Mutex::new(HashMap::new()),
            max_l1_entries: 100,
            max_l2_entries: 500,
            l3_dir,
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    /// Almacena datos en la memoria del agente.
    pub fn store(&self, key: &str, data: &[u8], level: MemoryLevel) {
        let entry = MemoryEntry {
            key: key.to_string(),
            data: data.to_vec(),
            level,
            last_access: Self::now(),
            access_count: 1,
            size: data.len(),
        };

        match level {
            MemoryLevel::L1 => {
                let mut cache = self.l1_cache.lock().unwrap();
                if cache.len() >= self.max_l1_entries {
                    // Evictar la entrada menos reciente
                    self.evict_l1();
                }
                cache.insert(key.to_string(), entry);
            }
            MemoryLevel::L2 => {
                let mut cache = self.l2_cache.lock().unwrap();
                if cache.len() >= self.max_l2_entries {
                    self.evict_l2();
                }
                cache.insert(key.to_string(), entry);
            }
            MemoryLevel::L3 => {
                let path = self.l3_dir.join(key);
                let _ = fs::write(&path, data);
            }
        }
    }

    /// Recupera datos de la memoria, promoviéndolos si es necesario.
    pub fn load(&self, key: &str) -> Option<Vec<u8>> {
        // 1. Buscar en L1
        {
            let mut cache = self.l1_cache.lock().unwrap();
            if let Some(entry) = cache.get_mut(key) {
                entry.last_access = Self::now();
                entry.access_count += 1;
                return Some(entry.data.clone());
            }
        }

        // 2. Buscar en L2
        {
            let mut cache = self.l2_cache.lock().unwrap();
            if let Some(entry) = cache.get_mut(key) {
                entry.last_access = Self::now();
                entry.access_count += 1;
                // Promover a L1
                let data = entry.data.clone();
                drop(cache);
                self.store(key, &data, MemoryLevel::L1);
                return Some(data);
            }
        }

        // 3. Buscar en L3 (disco)
        let l3_path = self.l3_dir.join(key);
        if l3_path.exists() {
            if let Ok(data) = fs::read(&l3_path) {
                // Promover a L2 y L1
                self.store(key, &data, MemoryLevel::L2);
                self.store(key, &data, MemoryLevel::L1);
                return Some(data);
            }
        }

        None
    }

    /// Elimina una entrada de la memoria.
    pub fn delete(&self, key: &str) -> bool {
        let mut l1 = self.l1_cache.lock().unwrap();
        if l1.remove(key).is_some() {
            return true;
        }
        let mut l2 = self.l2_cache.lock().unwrap();
        if l2.remove(key).is_some() {
            return true;
        }
        let l3_path = self.l3_dir.join(key);
        if l3_path.exists() {
            let _ = fs::remove_file(&l3_path);
            return true;
        }
        false
    }

    /// Lista todas las claves.
    pub fn list_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        let l1 = self.l1_cache.lock().unwrap();
        keys.extend(l1.keys().cloned());
        let l2 = self.l2_cache.lock().unwrap();
        keys.extend(l2.keys().cloned());
        if let Ok(entries) = fs::read_dir(&self.l3_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    keys.push(name.to_string());
                }
            }
        }
        keys
    }

    fn evict_l1(&self) {
        let mut cache = self.l1_cache.lock().unwrap();
        if let Some((key, _)) = cache
            .iter()
            .min_by_key(|(_, e)| (e.access_count, e.last_access))
            .map(|(k, _)| (k.clone(), ()))
        {
            cache.remove(&key);
        }
    }

    fn evict_l2(&self) {
        let mut cache = self.l2_cache.lock().unwrap();
        if let Some((key, entry)) = cache
            .iter()
            .min_by_key(|(_, e)| (e.access_count, e.last_access))
            .map(|(k, e)| (k.clone(), e.clone()))
        {
            // Mover a L3
            let path = self.l3_dir.join(&key);
            let _ = fs::write(&path, &entry.data);
            cache.remove(&key);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MmuError {
    #[error("Entrada de MMU no encontrada")]
    NotFound,
    #[error("Error de E/S en MMU: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error general de MMU: {0}")]
    Anyhow(#[from] anyhow::Error),
}

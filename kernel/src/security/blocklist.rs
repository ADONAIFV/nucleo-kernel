#![allow(clippy::module_inception)]

//! Lista negra dinámica para acciones y recursos prohibidos.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Tipos de entradas en la lista negra.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlocklistEntry {
    Syscall(String),  // Nombre de syscall
    Resource(String), // Ruta de recurso (ej. "/tmp/evil.sh")
    Agent(u64),       // ID de agente
    Keyword(String),  // Palabras clave en logs/comandos
}

/// Lista negra dinámica.
pub struct Blocklist {
    entries: Mutex<HashSet<BlocklistEntry>>,
    config_path: PathBuf,
}

impl Blocklist {
    pub fn new(config_dir: &Path) -> Result<Self> {
        let config_path = config_dir.join("blocklist.json");
        let entries = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            serde_json::from_str(&content)?
        } else {
            HashSet::new()
        };

        Ok(Self {
            entries: Mutex::new(entries),
            config_path,
        })
    }

    /// Carga la lista negra desde un archivo.
    pub fn load_from_file(&self) -> Result<()> {
        let content = fs::read_to_string(&self.config_path)?;
        let new_entries: HashSet<BlocklistEntry> = serde_json::from_str(&content)?;
        *self.entries.lock().unwrap() = new_entries;
        Ok(())
    }

    /// Guarda la lista negra en un archivo.
    pub fn save_to_file(&self) -> Result<()> {
        let entries = self.entries.lock().unwrap();
        let content = serde_json::to_string_pretty(&*entries)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// Añade una entrada a la lista negra.
    pub fn add_entry(&self, entry: BlocklistEntry) -> Result<()> {
        self.entries.lock().unwrap().insert(entry);
        self.save_to_file()
    }

    /// Elimina una entrada de la lista negra.
    pub fn remove_entry(&self, entry: &BlocklistEntry) -> Result<()> {
        self.entries.lock().unwrap().remove(entry);
        self.save_to_file()
    }

    /// Verifica si una entrada está en la lista negra.
    pub fn is_blocked(&self, entry: &BlocklistEntry) -> bool {
        self.entries.lock().unwrap().contains(entry)
    }

    /// Verifica si un comando de shell está bloqueado por palabras clave.
    pub fn check_shell_command(&self, command: &str) -> bool {
        let entries = self.entries.lock().unwrap();
        for entry in entries.iter() {
            if let BlocklistEntry::Keyword(keyword) = entry {
                if command.contains(keyword) {
                    return true;
                }
            }
        }
        false
    }

    /// Devuelve una lista de todas las entradas bloqueadas.
    pub fn list_blocked_entries(&self) -> Vec<BlocklistEntry> {
        self.entries.lock().unwrap().iter().cloned().collect()
    }
}

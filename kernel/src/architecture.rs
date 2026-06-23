//! API de inspección de la arquitectura del kernel.
//! Permite a los agentes explorar la estructura, módulos y configuración del kernel.

use crate::config::KernelConfig;
use crate::metrics::MetricsCollector;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use substrate::lego::LegoRegistry;

/// Información de un módulo del kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub version: String,
    pub health: String, // "Healthy", "Degraded", "Unhealthy"
    pub dependencies: Vec<String>,
    pub capabilities_required: Vec<String>,
    pub capabilities_provided: Vec<String>,
    pub path: PathBuf,
}

/// Árbol de archivos del kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelTree {
    pub root: PathBuf,
    pub config_path: PathBuf,
    pub modules_path: PathBuf,
    pub workspaces_path: PathBuf,
    pub checkpoints_path: PathBuf,
    pub files: Vec<String>,
}

/// Metadatos del kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelMetadata {
    pub version: String,
    pub build_timestamp: String,
    pub architecture: String,
    pub target_os: String,
    pub rust_version: String,
}

/// Información completa de la arquitectura del kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelArchitecture {
    pub metadata: KernelMetadata,
    pub config: KernelConfig,
    pub modules: Vec<ModuleInfo>,
    pub tree: KernelTree,
    pub system_state: SystemState,
}

/// Estado del sistema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub uptime_secs: u64,
    pub agents_active: usize,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub cpu_usage_percent: f32,
    pub modules_loaded: usize,
}

/// Servicio de inspección del kernel.
pub struct ArchitectureInspector {
    config: KernelConfig,
    lego_registry: Arc<LegoRegistry>,
    metrics: Arc<MetricsCollector>,
    work_dir: PathBuf,
}

impl ArchitectureInspector {
    pub fn new(
        config: KernelConfig,
        lego_registry: Arc<LegoRegistry>,
        metrics: Arc<MetricsCollector>,
        work_dir: PathBuf,
    ) -> Self {
        Self {
            config,
            lego_registry,
            metrics,
            work_dir,
        }
    }

    /// Obtiene información de todos los módulos.
    pub fn get_modules(&self) -> Vec<ModuleInfo> {
        let mut modules = Vec::new();
        for name in self.lego_registry.list() {
            if let Some(module) = self.lego_registry.get(&name) {
                modules.push(ModuleInfo {
                    name: name.clone(),
                    version: module.version().to_string(),
                    health: format!("{:?}", module.health()),
                    dependencies: Vec::new(), // se calcularía dinámicamente
                    capabilities_required: Vec::new(),
                    capabilities_provided: Vec::new(),
                    path: self.work_dir.join("modules").join(&name),
                });
            }
        }
        modules
    }

    /// Obtiene el árbol de archivos del kernel.
    pub fn get_tree(&self) -> KernelTree {
        let root = self.work_dir.clone();
        let mut files = Vec::new();

        // Recorrer el directorio del kernel y recopilar archivos
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if metadata.is_file() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            files.push(name.to_string());
                        }
                    }
                }
            }
        }

        KernelTree {
            root: root.clone(),
            config_path: root.join("configs"),
            modules_path: root.join("modules"),
            workspaces_path: root.join("workspaces"),
            checkpoints_path: root.join("checkpoints"),
            files,
        }
    }

    /// Obtiene metadatos del kernel.
    pub fn get_metadata(&self) -> KernelMetadata {
        KernelMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            target_os: std::env::consts::OS.to_string(),
            rust_version: std::env::var("RUST_VERSION").unwrap_or_else(|_| "unknown".to_string()),
        }
    }

    /// Obtiene el estado del sistema.
    pub fn get_system_state(&self) -> SystemState {
        let metrics = self.metrics.get_system_metrics();
        let modules = self.lego_registry.list().len();

        SystemState {
            uptime_secs: metrics.uptime_secs,
            agents_active: metrics.agents,
            memory_used_mb: metrics.memory_used_mb,
            memory_total_mb: metrics.memory_total_mb,
            cpu_usage_percent: metrics.cpu_usage_percent,
            modules_loaded: modules,
        }
    }

    /// Obtiene la arquitectura completa.
    pub fn get_architecture(&self) -> KernelArchitecture {
        KernelArchitecture {
            metadata: self.get_metadata(),
            config: self.config.clone(),
            modules: self.get_modules(),
            tree: self.get_tree(),
            system_state: self.get_system_state(),
        }
    }
}

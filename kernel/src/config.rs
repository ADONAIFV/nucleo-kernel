//! Configuración del kernel.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    pub limits: ResourceLimits,
    pub work_dir: String,
    pub checkpoint_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_agents: usize,
    pub max_memory_mb: u64,
    pub max_file_descriptors: u64,
}

impl KernelConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("no se pudo leer {}", path.as_ref().display()))?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.limits.max_agents == 0 {
            anyhow::bail!("max_agents debe ser > 0");
        }
        if self.limits.max_memory_mb == 0 {
            anyhow::bail!("max_memory_mb debe ser > 0");
        }
        if self.limits.max_file_descriptors == 0 {
            anyhow::bail!("max_file_descriptors debe ser > 0");
        }
        if self.checkpoint_interval_secs < 5 {
            anyhow::bail!("checkpoint_interval_secs debe ser al menos 5");
        }
        Ok(())
    }
}

//! Registro de módulos (Legos) para el kernel.
//! Permite que los módulos se registren y sean accesibles por nombre.

use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Estado de salud de un módulo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Trait que todo módulo (Lego) debe implementar.
/// Permite que el kernel los gestione dinámicamente.
pub trait Lego: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn init(&mut self) -> Result<(), anyhow::Error>;
    fn health(&self) -> Health;
    fn shutdown(&self) -> Result<(), anyhow::Error>;
}

/// Registro central de módulos del kernel.
pub struct LegoRegistry {
    modules: RwLock<HashMap<String, Arc<dyn Lego>>>,
}

impl LegoRegistry {
    pub fn new() -> Self {
        Self {
            modules: RwLock::new(HashMap::new()),
        }
    }

    /// Registra un nuevo módulo.
    pub fn register(&self, module: Arc<dyn Lego>) -> Result<(), anyhow::Error> {
        let name = module.name().to_string();
        let mut modules = self.modules.write();
        if modules.contains_key(&name) {
            return Err(anyhow::anyhow!("Módulo {} ya registrado", name));
        }
        modules.insert(name, module);
        Ok(())
    }

    /// Obtiene un módulo por nombre.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Lego>> {
        let modules = self.modules.read();
        modules.get(name).cloned()
    }

    /// Lista los nombres de todos los módulos registrados.
    pub fn list(&self) -> Vec<String> {
        let modules = self.modules.read();
        modules.keys().cloned().collect()
    }

    /// Reemplaza un módulo existente por uno nuevo (swap atómico).
    pub fn replace(&self, name: &str, new_module: Arc<dyn Lego>) -> Result<(), anyhow::Error> {
        let mut modules = self.modules.write();
        if modules.contains_key(name) {
            modules.insert(name.to_string(), new_module);
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Módulo {} no encontrado para reemplazar",
                name
            ))
        }
    }
}

//! Gestor de replicación de estado para alta disponibilidad.
//! Asegura que el estado crítico del kernel esté replicado entre nodos.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tipo de dato replicable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicableData {
    MemoryEntry(String, String), // key, value
    AgentState(String, Vec<u8>), // agent_id, serialized_state
    // Añadir más tipos de datos que necesiten replicarse
}

/// Errores de replicación.
#[derive(Debug, thiserror::Error)]
pub enum ReplicationError {
    #[error("Error de red: {0}")]
    NetworkError(String),
    #[error("Error de serialización: {0}")]
    SerializationError(String),
    #[error("Error de consistencia de datos: {0}")]
    ConsistencyError(String),
    #[error("Error general de replicación: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Gestor de replicación.
pub struct ReplicationManager {
    // Estado a replicar
    state: Arc<RwLock<HashMap<String, ReplicableData>>>,
    // Nodos en el clúster (para saber a quién replicar)
    // cluster_manager: Arc<ClusterManager>,
}

impl ReplicationManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(HashMap::new())),
            // cluster_manager
        }
    }

    pub async fn replicate(&self, data: ReplicableData) -> Result<(), ReplicationError> {
        // En una implementación real, esto enviaría el dato a otros nodos
        // y esperaría confirmación (ej. quorum).
        let mut state = self.state.write().await;
        match data {
            ReplicableData::MemoryEntry(key, value) => {
                state.insert(format!("memory:{}", key), ReplicableData::MemoryEntry(key, value));
            }
            ReplicableData::AgentState(agent_id, agent_state) => {
                state.insert(format!("agent:{}", agent_id), ReplicableData::AgentState(agent_id, agent_state));
            }
        }
        Ok(())
    }

    pub async fn get_replicated_state(&self, key: &str) -> Option<ReplicableData> {
        let state = self.state.read().await;
        state.get(key).cloned()
    }
}
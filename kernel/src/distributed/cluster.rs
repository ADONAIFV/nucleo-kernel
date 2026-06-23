//! Gestión de clúster para nodos del kernel.
//! Permite que varios kernels formen un clúster para alta disponibilidad y escalabilidad.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

/// ID de un nodo en el clúster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Información de un nodo en el clúster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: NodeId,
    pub address: String,
    pub last_seen_at: u64,
    pub roles: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Errores del clúster.
#[derive(Debug, thiserror::Error)]
pub enum ClusterError {
    #[error("Nodo no encontrado: {0}")]
    NodeNotFound(NodeId),
    #[error("Error de red: {0}")]
    NetworkError(String),
    #[error("Error de serialización: {0}")]
    SerializationError(String),
    #[error("Error general del clúster: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Gestor de clúster.
pub struct ClusterManager {
    nodes: Arc<RwLock<HashMap<NodeId, NodeInfo>>>,
    self_id: NodeId,
}

impl ClusterManager {
    pub fn new(self_id: NodeId) -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            self_id,
        }
    }

    pub async fn join_cluster(&self, node_info: NodeInfo) -> Result<(), ClusterError> {
        // En una implementación real, esto implicaría conectarse a otros nodos
        // y sincronizar el estado del clúster.
        let mut nodes = self.nodes.write().await;
        nodes.insert(node_info.id, node_info);
        Ok(())
    }

    pub async fn leave_cluster(&self, node_id: NodeId) -> Result<(), ClusterError> {
        let mut nodes = self.nodes.write().await;
        nodes.remove(&node_id);
        Ok(())
    }

    pub async fn update_node_info(&self, node_info: NodeInfo) -> Result<(), ClusterError> {
        let mut nodes = self.nodes.write().await;
        if let Some(entry) = nodes.get_mut(&node_info.id) {
            *entry = node_info;
            Ok(())
        } else {
            Err(ClusterError::NodeNotFound(node_info.id))
        }
    }

    pub async fn get_node_info(&self, node_id: NodeId) -> Option<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.get(&node_id).cloned()
    }

    pub async fn list_nodes(&self) -> Vec<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }
}

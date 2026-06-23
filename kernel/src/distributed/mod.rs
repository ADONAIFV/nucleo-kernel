//! Módulos para la funcionalidad distribuida del kernel.

pub mod cluster;
pub mod discovery;
pub mod replication;

// Re-exportar tipos principales
pub use cluster::{ClusterError, ClusterManager, NodeId, NodeInfo};
pub use discovery::{DiscoveryError, DiscoveryService};
pub use replication::{ReplicationError, ReplicationManager};
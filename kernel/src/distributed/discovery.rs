//! Servicio de descubrimiento para nodos del kernel.
//! Permite a los nodos encontrarse entre sí en la red.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{error, info};

/// Errores del servicio de descubrimiento.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Error de red: {0}")]
    NetworkError(String),
    #[error("Error de serialización: {0}")]
    SerializationError(String),
    #[error("Error general de descubrimiento: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Servicio de descubrimiento.
pub struct DiscoveryService {
    listen_addr: SocketAddr,
    broadcast_addr: SocketAddr,
    cluster_name: String,
}

impl DiscoveryService {
    pub fn new(listen_addr: SocketAddr, broadcast_addr: SocketAddr, cluster_name: String) -> Self {
        Self { listen_addr, broadcast_addr, cluster_name }
    }

    pub async fn start(&self) -> Result<(), DiscoveryError> {
        info!("📡 Iniciando servicio de descubrimiento en {} (broadcast a {})",
            self.listen_addr, self.broadcast_addr);

        let socket = UdpSocket::bind(self.listen_addr)
            .await
            .map_err(|e| DiscoveryError::NetworkError(e.to_string()))?;
        socket.set_broadcast(true)
            .map_err(|e| DiscoveryError::NetworkError(e.to_string()))?;

        let mut buf = vec![0u8; 1024];

        loop {
            // Enviar "ping" periódico
            let ping_msg = format!("{{ \"cluster\": \"{}\", \"type\": \"ping\", \"addr\": \"{}\" }}",
                self.cluster_name, self.listen_addr);
            socket.send_to(ping_msg.as_bytes(), self.broadcast_addr).await
                .map_err(|e| DiscoveryError::NetworkError(e.to_string()))?;

            // Recibir y procesar mensajes
            let (len, peer) = socket.recv_from(&mut buf).await
                .map_err(|e| DiscoveryError::NetworkError(e.to_string()))?;
            let msg = String::from_utf8_lossy(&buf[..len]);
            info!("Received discovery message from {}: {}", peer, msg);

            // Aquí se procesarían los pings y handshakes de otros nodos.
            // Se pasaría la información al ClusterManager.

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}
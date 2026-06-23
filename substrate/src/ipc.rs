#![allow(clippy::module_inception)]

//! Comunicación entre agentes (pub/sub).
//! Permite que los agentes se envíen mensajes entre sí de forma asíncrona.

use crate::AgentId;
use crossbeam_channel::{Receiver, RecvError, Sender, TrySendError, bounded};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Mensaje enviado entre agentes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub from: AgentId,
    pub to: Option<AgentId>, // None = broadcast
    pub topic: String,
    pub payload: String,
    pub timestamp: u64,
}

impl Message {
    // Constructor para un mensaje vacío (heartbeat o para purgar canales)
    pub fn empty() -> Self {
        Self {
            from: AgentId(0),
            to: None,
            topic: String::new(),
            payload: String::new(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// Bus de mensajes para comunicación entre agentes.
pub struct MessageBus {
    topics: Mutex<HashMap<String, Vec<Sender<Message>>>>,
    agent_queues: Mutex<HashMap<AgentId, Sender<Message>>>,
    _next_id: Arc<Mutex<u64>>,
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            topics: Mutex::new(HashMap::new()),
            agent_queues: Mutex::new(HashMap::new()),
            _next_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Registra un agente en el bus y devuelve un receptor para sus mensajes.
    pub fn register_agent(&self, agent_id: AgentId) -> Receiver<Message> {
        let (tx, rx) = bounded(1024);
        self.agent_queues.lock().unwrap().insert(agent_id, tx);
        rx
    }

    /// Desregistra un agente del bus.
    pub fn unregister_agent(&self, agent_id: AgentId) {
        self.agent_queues.lock().unwrap().remove(&agent_id);
        let empty_msg = Message::empty();
        let mut topics = self.topics.lock().unwrap();
        for (_, senders) in topics.iter_mut() {
            senders.retain(|tx| match tx.try_send(empty_msg.clone()) {
                Ok(_) => true,                               // Envío exitoso, el canal está vivo
                Err(TrySendError::Disconnected(_)) => false, // Receptor desconectado, purgar
                Err(TrySendError::Full(_)) => true,          // Canal lleno, pero vivo
            });
        }
    }

    /// Publica un mensaje en un tema.
    pub fn publish(&self, topic: &str, message: Message) -> Result<(), IpcError> {
        let topics = self.topics.lock().unwrap();
        if let Some(senders) = topics.get(topic) {
            for sender in senders {
                let _ = sender.send(message.clone());
            }
        }
        Ok(())
    }

    /// Suscribe un agente a un tema.
    pub fn subscribe(&self, agent_id: AgentId, topic: &str) -> Result<(), IpcError> {
        let mut topics = self.topics.lock().unwrap();
        let entry = topics.entry(topic.to_string()).or_default();
        // Buscar el canal del agente
        let agent_queues = self.agent_queues.lock().unwrap();
        if let Some(tx) = agent_queues.get(&agent_id) {
            entry.push(tx.clone());
            Ok(())
        } else {
            Err(IpcError::AgentNotFound(agent_id))
        }
    }

    /// Envía un mensaje directamente a un agente (no por tema).
    pub fn send_to_agent(
        &self,
        from: AgentId,
        to: AgentId,
        payload: String,
    ) -> Result<(), IpcError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let message = Message {
            from,
            to: Some(to),
            topic: "direct".to_string(),
            payload,
            timestamp,
        };

        let agent_queues = self.agent_queues.lock().unwrap();
        if let Some(tx) = agent_queues.get(&to) {
            tx.send(message).map_err(|_| IpcError::SendError)?;
            Ok(())
        } else {
            Err(IpcError::AgentNotFound(to))
        }
    }

    /// Recibe un mensaje para un agente (bloqueante).
    pub fn get_agent_receiver(&self, _agent_id: AgentId) -> Receiver<Message> {
        // En una implementación real, cada agente tendría un único Receiver asociado a su ID
        // Aquí, para la demo, simplemente devolvemos un nuevo receptor con un bounded channel
        // que el bus de mensajes llenaría si hubiera algo.
        // Para la demo, el `send_to_agent` y `publish` son los que usan los `Sender` registrados.
        // Este `get_agent_receiver` está simplificado para que `ipc_recv` pueda tener un `Receiver`.
        let (_tx, rx) = bounded(1024); // Crear un nuevo canal para cada solicitud de recepción temporal, _tx es unused
        rx
    }
}

/// Errores del sistema de IPC.
#[derive(Debug, Error)]
pub enum IpcError {
    #[error("Agente {0} no encontrado")]
    AgentNotFound(AgentId),
    #[error("Error al enviar mensaje")]
    SendError,
    #[error("Error al recibir mensaje")]
    ReceiveError,
    #[error("Timeout al recibir mensaje")]
    Timeout,
    #[error("Funcionalidad no implementada")]
    NotImplemented,
}

impl From<RecvError> for IpcError {
    fn from(_err: RecvError) -> Self {
        // Si RecvError ocurre, significa que el sender se ha desconectado.
        IpcError::ReceiveError
    }
}

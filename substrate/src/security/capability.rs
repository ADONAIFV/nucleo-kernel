//! Sistema de capacidades granular (estilo seL4) para agentes.
//! Cada herramienta y acción tiene permisos explícitos que el kernel valida.

use crate::AgentId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex; // Corregido para usar AgentId de substrate directamente

/// Capacidad base.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Capability {
    pub domain: CapDomain,
    pub operation: String,
    pub resource: Option<String>,
    pub flags: CapFlags,
}

/// Dominio de una capacidad.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapDomain {
    Workspace,
    Eval,
    MemoryStore,
    Ipc,
    Timer,
    Metrics,
    SharedWorkspace,
    Admin,
}

/// Flags de una capacidad.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapFlags {
    pub read_only: bool,
    pub destructive: bool,
    pub concurrency_safe: bool,
    pub temporary: bool,
    pub ttl_secs: Option<u64>,
}

impl CapFlags {
    pub fn default() -> Self {
        Self {
            read_only: false,
            destructive: false,
            concurrency_safe: true,
            temporary: false,
            ttl_secs: None,
        }
    }

    pub fn read_only() -> Self {
        Self {
            read_only: true,
            destructive: false,
            concurrency_safe: true,
            temporary: false,
            ttl_secs: None,
        }
    }

    pub fn destructive() -> Self {
        Self {
            read_only: false,
            destructive: true,
            concurrency_safe: false,
            temporary: false,
            ttl_secs: None,
        }
    }
}

/// Conjunto de capacidades.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapabilitySet {
    caps: HashMap<String, Capability>, // key = "domain:operation:resource"
    wildcard: bool,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self {
            caps: HashMap::new(),
            wildcard: false,
        }
    }

    pub fn wildcard() -> Self {
        Self {
            caps: HashMap::new(),
            wildcard: true,
        }
    }

    /// Añade una capacidad.
    pub fn add(&mut self, cap: Capability) {
        let key = Self::cap_key(&cap);
        self.caps.insert(key, cap);
    }

    /// Verifica si una capacidad está presente.
    pub fn has(&self, domain: &CapDomain, operation: &str, resource: Option<&str>) -> bool {
        if self.wildcard {
            return true;
        }

        // Verificar capacidad exacta
        let key = Self::key(domain, operation, resource);
        if self.caps.contains_key(&key) {
            return true;
        }

        // Verificar con wildcard en resource
        let key_wildcard = Self::key(domain, operation, Some("*"));
        if self.caps.contains_key(&key_wildcard) {
            return true;
        }

        // Verificar con wildcard en operation
        let key_op_wildcard = Self::key(domain, "*", resource);
        if self.caps.contains_key(&key_op_wildcard) {
            return true;
        }

        // Verificar con wildcard completo
        let key_all = Self::key(domain, "*", Some("*"));
        self.caps.contains_key(&key_all)
    }

    /// Verifica una capacidad y devuelve sus flags.
    pub fn get(
        &self,
        domain: &CapDomain,
        operation: &str,
        resource: Option<&str>,
    ) -> Option<Capability> {
        if self.wildcard {
            return Some(Capability {
                domain: domain.clone(),
                operation: operation.to_string(),
                resource: resource.map(|s| s.to_string()),
                flags: CapFlags::default(),
            });
        }

        let key = Self::key(domain, operation, resource);
        self.caps.get(&key).cloned()
    }

    /// Lista todas las capacidades.
    pub fn list(&self) -> Vec<Capability> {
        self.caps.values().cloned().collect()
    }

    fn key(domain: &CapDomain, operation: &str, resource: Option<&str>) -> String {
        format!("{:?}:{}:{}", domain, operation, resource.unwrap_or("*"))
    }

    fn cap_key(cap: &Capability) -> String {
        Self::key(&cap.domain, &cap.operation, cap.resource.as_deref())
    }
}

/// Sistema de permisos global.
pub struct CapabilitySystem {
    // Permisos por agente
    agent_caps: Mutex<HashMap<AgentId, CapabilitySet>>,
    // Permisos por herramienta (definiciones globales)
    tool_defs: Mutex<HashMap<String, Capability>>,
}

impl CapabilitySystem {
    pub fn new() -> Self {
        Self {
            agent_caps: Mutex::new(HashMap::new()),
            tool_defs: Mutex::new(HashMap::new()),
        }
    }

    /// Registra una herramienta con sus capacidades.
    pub fn register_tool(&self, name: &str, domain: CapDomain, operation: &str, flags: CapFlags) {
        let cap = Capability {
            domain,
            operation: operation.to_string(),
            resource: Some(name.to_string()),
            flags,
        };
        let mut defs = self.tool_defs.lock().unwrap();
        defs.insert(name.to_string(), cap);
    }

    /// Asigna capacidades a un agente.
    pub fn assign_caps(&self, agent_id: AgentId, caps: CapabilitySet) {
        let mut agent_caps = self.agent_caps.lock().unwrap();
        agent_caps.insert(agent_id, caps);
    }

    /// Verifica si un agente tiene una capacidad específica.
    pub fn check(
        &self,
        agent_id: AgentId,
        domain: &CapDomain,
        operation: &str,
        resource: Option<&str>,
    ) -> bool {
        let agent_caps = self.agent_caps.lock().unwrap();
        if let Some(caps) = agent_caps.get(&agent_id) {
            caps.has(domain, operation, resource)
        } else {
            false
        }
    }

    /// Obtiene las flags de una capacidad para un agente.
    pub fn get_flags(
        &self,
        agent_id: AgentId,
        domain: &CapDomain,
        operation: &str,
        resource: Option<&str>,
    ) -> Option<CapFlags> {
        let agent_caps = self.agent_caps.lock().unwrap();
        if let Some(caps) = agent_caps.get(&agent_id) {
            if let Some(cap) = caps.get(domain, operation, resource) {
                return Some(cap.flags.clone());
            }
        }
        None
    }

    /// Lista todas las capacidades de un agente.
    pub fn list_for_agent(&self, agent_id: AgentId) -> Vec<Capability> {
        let agent_caps = self.agent_caps.lock().unwrap();
        if let Some(caps) = agent_caps.get(&agent_id) {
            caps.list()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_set() {
        let mut caps = CapabilitySet::new();
        caps.add(Capability {
            domain: CapDomain::Workspace,
            operation: "read".to_string(),
            resource: Some("/data/*".to_string()),
            flags: CapFlags::read_only(),
        });

        assert!(caps.has(&CapDomain::Workspace, "read", Some("/data/file.txt")));
        assert!(!caps.has(&CapDomain::Workspace, "write", Some("/data/file.txt")));
        assert!(!caps.has(&CapDomain::Workspace, "read", Some("/other/file.txt")));
    }

    #[test]
    fn test_capability_system() {
        let sys = CapabilitySystem::new();
        let agent_id = AgentId(1);

        let mut caps = CapabilitySet::new();
        caps.add(Capability {
            domain: CapDomain::Eval,
            operation: "python".to_string(),
            resource: None,
            flags: CapFlags::default(),
        });
        sys.assign_caps(agent_id, caps);

        assert!(sys.check(agent_id, &CapDomain::Eval, "python", None));
        assert!(!sys.check(agent_id, &CapDomain::Eval, "bash", None));
    }

    #[test]
    fn test_wildcard() {
        let mut caps = CapabilitySet::wildcard();
        assert!(caps.has(&CapDomain::Workspace, "anything", Some("any")));
        assert!(caps.has(&CapDomain::Eval, "python", None));
    }
}

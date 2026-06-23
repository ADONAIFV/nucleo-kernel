//! Sandboxing con eBPF para control de recursos a nivel de llamada a herramienta.
//! Basado en AgentCgroup y MCPGuard.
//! Nota: La implementación real de eBPF requiere acceso al kernel de Linux.
//! Esta implementación simula el comportamiento para portabilidad.

use crate::agent::AgentId;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Límites de recursos para una llamada a herramienta.
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_cpu_ms: u64,
    pub max_memory_mb: u64,
    pub max_file_size_mb: u64,
    pub max_network_bandwidth_kbps: u64,
    pub timeout_secs: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_ms: 1000,      // 1 segundo de CPU
            max_memory_mb: 100,     // 100 MB de RAM
            max_file_size_mb: 10,   // 10 MB
            max_network_bandwidth_kbps: 1024, // 1 Mbps
            timeout_secs: 30,
        }
    }
}

/// Política de sandboxing.
#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    pub agent_id: AgentId,
    pub tool_name: String,
    pub limits: ResourceLimits,
    pub allowed_paths: Vec<String>,
    pub pub blocked_paths: Vec<String>,
    pub allowed_network_hosts: Vec<String>,
}

/// Gestor de sandboxing con eBPF (simulado).
pub struct EbpfSandbox {
    policies: Mutex<HashMap<String, SandboxPolicy>>,
    stats: Mutex<EbpfStats>,
    enabled: bool,
}

/// Estadísticas del sandbox.
#[derive(Debug, Clone, Default)]
pub struct EbpfStats {
    pub total_requests: u64,
    pub allowed_requests: u64,
    pub blocked_requests: u64,
    pub cpu_usage_ms: u64,
    pub memory_usage_mb: u64,
    pub violations: Vec<String>,
}

impl EbpfSandbox {
    pub fn new(enabled: bool) -> Self {
        Self {
            policies: Mutex::new(HashMap::new()),
            stats: Mutex::new(EbpfStats::default()),
            enabled,
        }
    }

    /// Registra una política de sandboxing para un agente y herramienta.
    pub fn register_policy(&self, policy: SandboxPolicy) {
        let key = format!("{}:{}", policy.agent_id.0, policy.tool_name);
        let mut policies = self.policies.lock().unwrap();
        policies.insert(key, policy);
    }

    /// Elimina una política de sandboxing.
    pub fn unregister_policy(&self, agent_id: AgentId, tool_name: &str) {
        let key = format!("{}:{}", agent_id.0, tool_name);
        let mut policies = self.policies.lock().unwrap();
        policies.remove(&key);
    }

    /// Verifica si una acción está permitida por el sandbox.
    pub fn check(&self, agent_id: AgentId, tool_name: &str, args: &[String]) -> Result<(), SandboxError> {
        if !self.enabled {
            return Ok(());
        }

        let key = format!("{}:{}", agent_id.0, tool_name);
        let policies = self.policies.lock().unwrap();
        let policy = policies.get(&key)
            .ok_or(SandboxError::NoPolicy(agent_id, tool_name.to_string()))?;

        // Verificar límites de recursos (simulado)
        self.check_limits(policy)?;

        // Verificar paths permitidos/blocked (para herramientas de sistema de archivos)
        if tool_name.contains("fs") || tool_name.contains("workspace") {
            self.check_paths(policy, args)?;
        }

        // Verificar hosts de red (para herramientas de red)
        if tool_name.contains("net") || tool_name.contains("http") {
            self.check_network_hosts(policy, args)?;
        }

        // Actualizar estadísticas
        let mut stats = self.stats.lock().unwrap();
        stats.total_requests += 1;
        stats.allowed_requests += 1;

        Ok(())
    }

    /// Verifica límites de recursos.
    fn check_limits(&self, policy: &SandboxPolicy) -> Result<(), SandboxError> {
        // En una implementación real, esto consultaría eBPF.
        // Simulamos que siempre pasa.
        Ok(())
    }

    /// Verifica paths permitidos/blocked.
    fn check_paths(&self, policy: &SandboxPolicy, args: &[String]) -> Result<(), SandboxError> {
        for arg in args {
            if arg.starts_with("/") {
                // Verificar si está en blocked paths
                for blocked in &policy.blocked_paths {
                    if arg.starts_with(blocked) {
                        return Err(SandboxError::PathBlocked(arg.clone()));
                    }
                }

                // Verificar si está en allowed paths (si hay)
                if !policy.allowed_paths.is_empty() {
                    let mut allowed = false;
                    for allowed_path in &policy.allowed_paths {
                        if arg.starts_with(allowed_path) {
                            allowed = true;
                            break;
                        }
                    }
                    if !allowed {
                        return Err(SandboxError::PathNotAllowed(arg.clone()));
                    }
                }
            }
        }
        Ok(())
    }

    /// Verifica hosts de red permitidos.
    fn check_network_hosts(&self, policy: &SandboxPolicy, args: &[String]) -> Result<(), SandboxError> {
        if policy.allowed_network_hosts.is_empty() {
            return Ok(());
        }

        for arg in args {
            // Buscar host en el argumento (simplificado)
            let host = arg.split('/').next().unwrap_or(arg);
            if host.contains(":") || host.contains(".") {
                let mut allowed = false;
                for allowed_host in &policy.allowed_network_hosts {
                    if host.contains(allowed_host) {
                        allowed = true;
                        break;
                    }
                }
                if !allowed {
                    return Err(SandboxError::HostNotAllowed(host.to_string()));
                }
            }
        }
        Ok(())
    }

    /// Bloquea una solicitud (registra violación).
    pub fn block(&self, agent_id: AgentId, tool_name: &str, reason: &str) {
        let mut stats = self.stats.lock().unwrap();
        stats.blocked_requests += 1;
        stats.violations.push(format!("Agent {} tool {}: {}", agent_id.0, tool_name, reason));

        // En una implementación real, se registraría en eBPF.
        tracing::warn!("🔒 Sandbox bloqueó: Agente {} Herramienta {} - {}", agent_id.0, tool_name, reason);
    }

    /// Obtiene estadísticas del sandbox.
    pub fn stats(&self) -> EbpfStats {
        let stats = self.stats.lock().unwrap();
        stats.clone()
    }

    /// Habilita/deshabilita el sandbox.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

/// Errores del sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxError {
    NoPolicy(AgentId, String),
    PathBlocked(String),
    PathNotAllowed(String),
    HostNotAllowed(String),
    ResourceLimitExceeded(String),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxError::NoPolicy(id, tool) => write!(f, "No hay política para agente {:?} herramienta {}", id, tool),
            SandboxError::PathBlocked(path) => write!(f, "Ruta bloqueada: {}", path),
            SandboxError::PathNotAllowed(path) => write!(f, "Ruta no permitida: {}", path),
            SandboxError::HostNotAllowed(host) => write!(f, "Host no permitido: {}", host),
            SandboxError::ResourceLimitExceeded(res) => write!(f, "Límite de recurso excedido: {}", res),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_policy() {
        let sandbox = EbpfSandbox::new(true);
        let agent_id = AgentId(1);

        let policy = SandboxPolicy {
            agent_id,
            tool_name: "fs_read".to_string(),
            limits: ResourceLimits::default(),
            allowed_paths: vec!["/home/user/".to_string()],
            blocked_paths: vec!["/etc/".to_string(), "/root/".to_string()],
            allowed_network_hosts: vec![],
        };

        sandbox.register_policy(policy);

        // Debe permitir
        assert!(sandbox.check(agent_id, "fs_read", &["/home/user/file.txt".to_string()]).is_ok());

        // Debe bloquear
        assert!(sandbox.check(agent_id, "fs_read", &["/etc/passwd".to_string()]).is_err());
    }

    #[test]
    fn test_sandbox_no_policy() {
        let sandbox = EbpfSandbox::new(true);
        let agent_id = AgentId(1);

        let result = sandbox.check(agent_id, "unknown_tool", &[]);
        assert!(matches!(result, Err(SandboxError::NoPolicy(_, _))));
    }

    #[test]
    fn test_sandbox_disabled() {
        let mut sandbox = EbpfSandbox::new(false);
        sandbox.set_enabled(false);

        let agent_id = AgentId(1);
        // Sin política, pero sandbox deshabilitado
        assert!(sandbox.check(agent_id, "anything", &[]).is_ok());
    }
}

//! Gestión de procesos de agentes.
//! Permite la creación, monitorización y terminación de procesos nativos.

use anyhow::Result;
use nix::sys::signal::{SIGKILL, SIGTERM, kill};
use nix::sys::wait::{WaitStatus, waitpid};
use nix::unistd::{ForkResult, Pid, fork, getpid, getppid, setpgid};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::os::unix::prelude::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// ID de proceso de agente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(pub u64);

impl fmt::Display for ProcessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Estado de un proceso de agente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Stopped(i32),
    Killed,
    Failed(String),
}

/// Errores del gestor de procesos.
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Error de fork: {0}")]
    ForkError(String),
    #[error("Error al ejecutar comando: {0}")]
    ExecError(String),
    #[error("Proceso no encontrado: {0}")]
    ProcessNotFound(ProcessId),
    #[error("Error al enviar señal: {0}")]
    SignalError(String),
    #[error("Error al esperar proceso: {0}")]
    WaitError(String),
    #[error("Error de I/O: {0}")]
    IoError(String),
    #[error("Error general: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Proceso de agente.
pub struct AgentProcess {
    pub id: ProcessId,
    pub agent_id: crate::AgentId,
    pid: Pid,
    status: Mutex<ProcessStatus>,
    command: String,
    args: Vec<String>,
}

impl AgentProcess {
    pub fn new(
        agent_id: crate::AgentId,
        command: String,
        args: Vec<String>,
    ) -> Result<Self, ProcessError> {
        static NEXT_PID: AtomicU64 = AtomicU64::new(1);
        let id = ProcessId(NEXT_PID.fetch_add(1, Ordering::SeqCst));

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child, .. }) => Ok(Self {
                id,
                agent_id,
                pid: child,
                status: Mutex::new(ProcessStatus::Running),
                command,
                args,
            }),
            Ok(ForkResult::Child) => {
                // Configurar un nuevo grupo de procesos para el hijo
                setpgid(Pid::from_raw(0), Pid::from_raw(0))
                    .map_err(|e| ProcessError::ForkError(e.to_string()))?;

                let _ = Command::new(&command)
                    .args(&args)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .exec(); // exec replaces the current process
                unreachable!(); // exec never returns
            }
            Err(e) => Err(ProcessError::ForkError(e.to_string())),
        }
    }

    pub fn id(&self) -> ProcessId {
        self.id
    }

    pub fn status(&self) -> ProcessStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn wait(&self) -> Result<ProcessStatus, ProcessError> {
        let status = waitpid(self.pid, None).map_err(|e| ProcessError::WaitError(e.to_string()))?;

        let new_status = match status {
            WaitStatus::Exited(_, code) => ProcessStatus::Stopped(code),
            WaitStatus::Signaled(_, _, _) => ProcessStatus::Killed,
            _ => ProcessStatus::Running, // Otros estados, como WSTOPPED, se pueden manejar aquí
        };
        *self.status.lock().unwrap() = new_status.clone();
        Ok(new_status)
    }

    pub fn kill(&self) -> Result<(), ProcessError> {
        kill(self.pid, SIGTERM).map_err(|e| ProcessError::SignalError(e.to_string()))?;
        Ok(())
    }

    pub fn force_kill(&self) -> Result<(), ProcessError> {
        kill(self.pid, SIGKILL).map_err(|e| ProcessError::SignalError(e.to_string()))?;
        Ok(())
    }
}

/// Gestor de procesos para el kernel.
pub struct ProcessManager {
    processes: Mutex<HashMap<ProcessId, Arc<AgentProcess>>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Mutex::new(HashMap::new()),
        }
    }

    pub fn create_process(
        &self,
        agent_id: crate::AgentId,
        command: String,
        args: Vec<String>,
    ) -> Result<Arc<AgentProcess>, ProcessError> {
        let process = Arc::new(AgentProcess::new(agent_id, command, args)?);
        self.processes
            .lock()
            .unwrap()
            .insert(process.id(), Arc::clone(&process));
        Ok(process)
    }

    pub fn get_process(&self, id: ProcessId) -> Option<Arc<AgentProcess>> {
        self.processes.lock().unwrap().get(&id).cloned()
    }

    pub fn terminate_process(&self, id: ProcessId) -> Result<(), ProcessError> {
        let process_opt = self.processes.lock().unwrap().remove(&id);
        if let Some(process) = process_opt {
            process.kill() // O force_kill si queremos ser más agresivos
        } else {
            Err(ProcessError::ProcessNotFound(id))
        }
    }

    pub fn list_processes(&self) -> Vec<Arc<AgentProcess>> {
        self.processes.lock().unwrap().values().cloned().collect()
    }
}

//! Scheduler semántico con prioridades y conciencia de carga.
//! Basado en el estado del arte de schedulers para agentes (APU, Namzu).
//! Soporta priorización de tareas, backpressure y límites por ejecución.

use crate::AgentId;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Prioridad de una tarea.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskPriority {
    Critical = 4,
    High = 3,
    Normal = 2,
    Low = 1,
    Background = 0,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Estado de una tarea.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Una tarea que el scheduler gestiona.
#[derive(Debug, Clone)]
pub struct Task {
    pub id: u64,
    pub agent_id: AgentId,
    pub priority: TaskPriority,
    pub state: TaskState,
    pub created_at: Instant,
    pub started_at: Option<Instant>,
    pub completed_at: Option<Instant>,
    pub timeout_secs: Option<u64>,
    pub max_retries: u32,
    pub retries: u32,
    pub metadata: HashMap<String, String>,
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Task {}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Ordenar por prioridad (mayor primero) y luego por tiempo de creación
        match self.priority.cmp(&other.priority) {
            std::cmp::Ordering::Equal => self.created_at.cmp(&other.created_at).reverse(),
            other => other,
        }
    }
}

/// Estadísticas del scheduler.
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub total_tasks_submitted: u64,
    pub total_tasks_completed: u64,
    pub total_tasks_failed: u64,
    pub total_tasks_cancelled: u64,
    pub current_tasks_pending: usize,
    pub current_tasks_running: usize,
    pub avg_wait_time_ms: f64,
    pub avg_execution_time_ms: f64,
    pub backpressure_active: bool,
}

/// Scheduler semántico.
pub struct SemanticScheduler {
    // Colas por prioridad
    queues: Mutex<HashMap<TaskPriority, VecDeque<Task>>>,
    // Tareas en ejecución
    running: Mutex<HashMap<u64, Task>>,
    // Historial de tareas completadas
    history: Mutex<VecDeque<Task>>,
    // Estadísticas
    stats: Mutex<SchedulerStats>,
    // Contador de IDs
    next_id: Mutex<u64>,
    // Límites
    max_concurrent_tasks: usize,
    max_history_size: usize,
    // Backpressure
    backpressure_threshold: f32, // 0.0 - 1.0
    backpressure_active: Mutex<bool>,
    // Métricas de carga
    load_avg: Mutex<f32>,
}

impl SemanticScheduler {
    pub fn new(max_concurrent: usize) -> Self {
        let mut queues = HashMap::new();
        queues.insert(TaskPriority::Critical, VecDeque::new());
        queues.insert(TaskPriority::High, VecDeque::new());
        queues.insert(TaskPriority::Normal, VecDeque::new());
        queues.insert(TaskPriority::Low, VecDeque::new());
        queues.insert(TaskPriority::Background, VecDeque::new());

        Self {
            queues: Mutex::new(queues),
            running: Mutex::new(HashMap::new()),
            history: Mutex::new(VecDeque::with_capacity(1000)),
            stats: Mutex::new(SchedulerStats::default()),
            next_id: Mutex::new(1),
            max_concurrent_tasks: max_concurrent,
            max_history_size: 1000,
            backpressure_threshold: 0.85,
            backpressure_active: Mutex::new(false),
            load_avg: Mutex::new(0.0),
        }
    }

    /// Envía una nueva tarea al scheduler.
    pub fn submit(
        &self,
        agent_id: AgentId,
        priority: TaskPriority,
        timeout_secs: Option<u64>,
        max_retries: u32,
    ) -> u64 {
        let id = {
            let mut next = self.next_id.lock().unwrap();
            let id = *next;
            *next += 1;
            id
        };

        let task = Task {
            id,
            agent_id,
            priority,
            state: TaskState::Pending,
            created_at: Instant::now(),
            started_at: None,
            completed_at: None,
            timeout_secs,
            max_retries,
            retries: 0,
            metadata: HashMap::new(),
        };

        let mut queues = self.queues.lock().unwrap();
        if let Some(queue) = queues.get_mut(&priority) {
            queue.push_back(task);
        }

        let mut stats = self.stats.lock().unwrap();
        stats.total_tasks_submitted += 1;
        stats.current_tasks_pending += 1;

        self.update_backpressure();

        id
    }

    /// Ejecuta el siguiente lote de tareas (llamado por el scheduler en cada tick).
    pub fn tick(&self) -> usize {
        let mut executed = 0;
        let running_count = self.running.lock().unwrap().len();

        // Verificar límite de concurrencia
        if running_count >= self.max_concurrent_tasks {
            return 0;
        }

        // Obtener la siguiente tarea con la prioridad más alta
        let next_task = {
            let mut queues = self.queues.lock().unwrap();
            let priorities = [
                TaskPriority::Critical,
                TaskPriority::High,
                TaskPriority::Normal,
                TaskPriority::Low,
                TaskPriority::Background,
            ];

            let mut task = None;
            for priority in priorities.iter() {
                if let Some(queue) = queues.get_mut(priority) {
                    if let Some(t) = queue.pop_front() {
                        task = Some(t);
                        break;
                    }
                }
            }
            task
        };

        if let Some(mut task) = next_task {
            task.state = TaskState::Running;
            task.started_at = Some(Instant::now());

            let task_id = task.id;
            let mut running = self.running.lock().unwrap();
            running.insert(task_id, task);

            let mut stats = self.stats.lock().unwrap();
            stats.current_tasks_pending -= 1;
            stats.current_tasks_running += 1;

            executed += 1;
        }

        // Actualizar estadísticas
        self.update_stats();
        self.update_backpressure();

        executed
    }

    /// Marca una tarea como completada.
    pub fn complete(&self, task_id: u64) -> Result<(), SchedulerError> {
        let mut running = self.running.lock().unwrap();
        if let Some(mut task) = running.remove(&task_id) {
            task.state = TaskState::Completed;
            task.completed_at = Some(Instant::now());

            let mut stats = self.stats.lock().unwrap();
            stats.total_tasks_completed += 1;
            stats.current_tasks_running -= 1;

            // Almacenar en historial
            let mut history = self.history.lock().unwrap();
            if history.len() >= self.max_history_size {
                history.pop_front();
            }
            history.push_back(task);

            self.update_stats();
            Ok(())
        } else {
            Err(SchedulerError::TaskNotFound(task_id))
        }
    }

    /// Marca una tarea como fallida (puede reintentarse).
    pub fn fail(&self, task_id: u64) -> Result<(), SchedulerError> {
        let mut running = self.running.lock().unwrap();
        if let Some(mut task) = running.remove(&task_id) {
            task.retries += 1;

            if task.retries < task.max_retries {
                // Reintentar
                task.state = TaskState::Pending;
                task.started_at = None;
                let priority = task.priority;
                let mut queues = self.queues.lock().unwrap();
                if let Some(queue) = queues.get_mut(&priority) {
                    queue.push_front(task);
                }
            } else {
                // Fallo definitivo
                task.state = TaskState::Failed;
                task.completed_at = Some(Instant::now());

                let mut stats = self.stats.lock().unwrap();
                stats.total_tasks_failed += 1;
                stats.current_tasks_running -= 1;

                let mut history = self.history.lock().unwrap();
                if history.len() >= self.max_history_size {
                    history.pop_front();
                }
                history.push_back(task);
            }

            self.update_stats();
            Ok(())
        } else {
            Err(SchedulerError::TaskNotFound(task_id))
        }
    }

    /// Cancela una tarea.
    pub fn cancel(&self, task_id: u64) -> Result<(), SchedulerError> {
        // Buscar en colas
        {
            let mut queues = self.queues.lock().unwrap();
            for (_, queue) in queues.iter_mut() {
                if let Some(pos) = queue.iter().position(|t| t.id == task_id) {
                    let mut task = queue.remove(pos).unwrap();
                    task.state = TaskState::Cancelled;
                    task.completed_at = Some(Instant::now());

                    let mut stats = self.stats.lock().unwrap();
                    stats.total_tasks_cancelled += 1;

                    let mut history = self.history.lock().unwrap();
                    if history.len() >= self.max_history_size {
                        history.pop_front();
                    }
                    history.push_back(task);

                    self.update_stats();
                    return Ok(());
                }
            }
        }

        // Buscar en ejecución
        {
            let mut running = self.running.lock().unwrap();
            if let Some(mut task) = running.remove(&task_id) {
                task.state = TaskState::Cancelled;
                task.completed_at = Some(Instant::now());

                let mut stats = self.stats.lock().unwrap();
                stats.total_tasks_cancelled += 1;
                stats.current_tasks_running -= 1;

                let mut history = self.history.lock().unwrap();
                if history.len() >= self.max_history_size {
                    history.pop_front();
                }
                history.push_back(task);

                self.update_stats();
                return Ok(());
            }
        }

        Err(SchedulerError::TaskNotFound(task_id))
    }

    /// Obtiene estadísticas del scheduler.
    pub fn stats(&self) -> SchedulerStats {
        let stats = self.stats.lock().unwrap();
        stats.clone()
    }

    /// Verifica si el scheduler está en backpressure.
    pub fn is_backpressure_active(&self) -> bool {
        *self.backpressure_active.lock().unwrap()
    }

    /// Obtiene la carga actual (0.0 - 1.0).
    pub fn load(&self) -> f32 {
        *self.load_avg.lock().unwrap()
    }

    /// Actualiza las estadísticas.
    fn update_stats(&self) {
        let mut stats = self.stats.lock().unwrap();
        let history = self.history.lock().unwrap();

        let total: u64 = history.len() as u64;
        if total > 0 {
            let mut wait_times = 0;
            let mut exec_times = 0;
            let mut count = 0;

            for task in history.iter() {
                if let (Some(started), Some(completed)) = (task.started_at, task.completed_at) {
                    wait_times += started.duration_since(task.created_at).as_millis();
                    exec_times += completed.duration_since(started).as_millis();
                    count += 1;
                }
            }

            if count > 0 {
                stats.avg_wait_time_ms = wait_times as f64 / count as f64;
                stats.avg_execution_time_ms = exec_times as f64 / count as f64;
            }
        }
    }

    /// Actualiza el estado de backpressure.
    fn update_backpressure(&self) {
        let running = self.running.lock().unwrap();
        let pending = {
            let queues = self.queues.lock().unwrap();
            let total: usize = queues.values().map(|q| q.len()).sum();
            total
        };

        let total = running.len() + pending;
        let load = total as f32 / self.max_concurrent_tasks as f32;

        let mut load_avg = self.load_avg.lock().unwrap();
        *load_avg = *load_avg * 0.7 + load * 0.3;

        let mut backpressure = self.backpressure_active.lock().unwrap();
        *backpressure = *load_avg > self.backpressure_threshold;
    }
}

/// Errores del scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    TaskNotFound(u64),
    QueueFull,
    InvalidPriority,
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerError::TaskNotFound(id) => write!(f, "Tarea {} no encontrada", id),
            SchedulerError::QueueFull => write!(f, "Cola llena"),
            SchedulerError::InvalidPriority => write!(f, "Prioridad inválida"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_submit_tick() {
        let scheduler = SemanticScheduler::new(2);
        let agent_id = AgentId(1);

        let id = scheduler.submit(agent_id, TaskPriority::Normal, None, 3);
        assert_eq!(id, 1);

        let executed = scheduler.tick();
        assert_eq!(executed, 1);

        let running = scheduler.running.lock().unwrap();
        assert_eq!(running.len(), 1);
    }

    #[test]
    fn test_scheduler_priority() {
        let scheduler = SemanticScheduler::new(1);
        let agent_id = AgentId(1);

        scheduler.submit(agent_id, TaskPriority::Low, None, 3);
        scheduler.submit(agent_id, TaskPriority::High, None, 3);
        scheduler.submit(agent_id, TaskPriority::Critical, None, 3);

        // La primera tarea ejecutada debe ser la de mayor prioridad
        scheduler.tick();
        let running = scheduler.running.lock().unwrap();
        let task = running.values().next().unwrap();
        assert_eq!(task.priority, TaskPriority::Critical);
    }

    #[test]
    fn test_scheduler_complete() {
        let scheduler = SemanticScheduler::new(2);
        let agent_id = AgentId(1);

        let id = scheduler.submit(agent_id, TaskPriority::Normal, None, 3);
        scheduler.tick();

        scheduler.complete(id).unwrap();

        let stats = scheduler.stats();
        assert_eq!(stats.total_tasks_completed, 1);
    }

    #[test]
    fn test_scheduler_backpressure() {
        let scheduler = SemanticScheduler::new(2);
        let agent_id = AgentId(1);

        for _ in 0..10 {
            scheduler.submit(agent_id, TaskPriority::Normal, None, 3);
        }

        // Ejecutar varios ticks
        for _ in 0..5 {
            scheduler.tick();
        }

        assert!(scheduler.is_backpressure_active());
    }
}

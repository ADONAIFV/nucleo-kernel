#![allow(clippy::module_inception)]

//! Gestión de temporizadores y eventos programados.

use std::collections::BinaryHeap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Identificador único para un temporizador.
pub type TimerId = u64;

/// Evento de temporizador.
#[derive(Debug, Clone)]
pub struct TimerEvent {
    pub id: TimerId,
    pub target_time: Instant,
    pub callback_id: u64, // Referencia a la función de callback o mensaje a enviar
    pub data: Vec<u8>,    // Datos asociados al evento
}

// Implementación de `PartialEq` y `Eq` para `TimerEvent` para `BinaryHeap`
impl PartialEq for TimerEvent {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TimerEvent {}

// Implementación de `PartialOrd` y `Ord` para `TimerEvent` para `BinaryHeap`
impl PartialOrd for TimerEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Un BinaryHeap es un min-heap por defecto, así que invertimos el orden para un max-heap
        // Queremos que los eventos con `target_time` más pequeño (más próximos) tengan mayor prioridad
        other.target_time.partial_cmp(&self.target_time)
    }
}

impl Ord for TimerEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

/// Gestor de temporizadores.
pub struct TimerManager {
    timers: Mutex<BinaryHeap<TimerEvent>>,
    next_id: Mutex<TimerId>,
}

impl TimerManager {
    pub fn new() -> Self {
        Self {
            timers: Mutex::new(BinaryHeap::new()),
            next_id: Mutex::new(0),
        }
    }

    /// Programa un nuevo temporizador.
    pub fn schedule_timer(&self, duration: Duration, callback_id: u64, data: Vec<u8>) -> TimerId {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let target_time = Instant::now() + duration;
        let event = TimerEvent {
            id,
            target_time,
            callback_id,
            data,
        };

        self.timers.lock().unwrap().push(event);
        id
    }

    /// Cancela un temporizador por su ID.
    pub fn cancel_timer(&self, timer_id: TimerId) -> bool {
        let mut timers = self.timers.lock().unwrap();
        let initial_len = timers.len();
        timers.retain(|e| e.id != timer_id);
        timers.len() < initial_len
    }

    /// Procesa los eventos de temporizador vencidos.
    /// Devuelve una lista de eventos listos para ser despachados.
    pub fn process_timers(&self) -> Vec<TimerEvent> {
        let mut timers = self.timers.lock().unwrap();
        let now = Instant::now();
        let mut ready_events = Vec::new();

        while let Some(event) = timers.peek() {
            if event.target_time <= now {
                ready_events.push(timers.pop().unwrap());
            } else {
                break;
            }
        }
        ready_events
    }

    /// Devuelve el tiempo restante hasta el próximo evento, si existe.
    pub fn time_until_next_event(&self) -> Option<Duration> {
        let timers = self.timers.lock().unwrap();
        timers.peek().map(|event| {
            let now = Instant::now();
            if event.target_time > now {
                event.target_time - now
            } else {
                Duration::from_secs(0)
            }
        })
    }
}

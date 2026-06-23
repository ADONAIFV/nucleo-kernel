#![allow(clippy::module_inception)]

//! Recolección de métricas del kernel y del sistema.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Métricas agregadas del sistema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub uptime_secs: u64,
    pub agents: usize,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub cpu_usage_percent: f32,
}

/// Tipos de métricas.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

/// Una métrica individual.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub timestamp: u64,
    pub tags: HashMap<String, String>,
}

/// Recolector de métricas del sistema.
pub struct MetricsCollector {
    metrics: Mutex<HashMap<String, Metric>>,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Mutex::new(HashMap::new()),
            start_time: Instant::now(),
        }
    }

    /// Registra un nuevo valor para una métrica.
    pub fn record(
        &self,
        name: &str,
        metric_type: MetricType,
        value: f64,
        tags: Option<HashMap<String, String>>,
    ) {
        let mut metrics = self.metrics.lock().unwrap();
        let timestamp = self.start_time.elapsed().as_secs();

        let metric = Metric {
            name: name.to_string(),
            metric_type,
            value,
            timestamp,
            tags: tags.unwrap_or_default(),
        };
        metrics.insert(name.to_string(), metric);
    }

    /// Obtiene el valor actual de una métrica.
    pub fn get_metric(&self, name: &str) -> Option<Metric> {
        let metrics = self.metrics.lock().unwrap();
        metrics.get(name).cloned()
    }

    /// Lista todas las métricas activas.
    pub fn list_metrics(&self) -> Vec<Metric> {
        self.metrics.lock().unwrap().values().cloned().collect()
    }

    /// Obtiene el tiempo de actividad del kernel.
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Obtiene un resumen de métricas del sistema.
    pub fn get_system_metrics(&self) -> SystemMetrics {
        SystemMetrics {
            uptime_secs: self.start_time.elapsed().as_secs(),
            agents: 0, // Se actualizaría vía registro
            memory_used_mb: 0,
            memory_total_mb: 0,
            cpu_usage_percent: 0.0,
        }
    }
}

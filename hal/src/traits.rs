//! Traits principales de la HAL.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Información de la CPU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuInfo {
    pub arch: Architecture,
    pub cores: usize,
    pub threads: usize,
    pub freq_mhz: u64,
    pub features: Vec<String>,
    pub model: String,
    pub cache_l1_kb: u32,
    pub cache_l2_kb: u32,
    pub cache_l3_kb: u32,
}

/// Información de la memoria.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryInfo {
    pub total_mb: u64,
    pub available_mb: u64,
    pub numa_nodes: usize,
    pub huge_page_size_kb: u64,
    pub page_size_kb: u64,
}

/// Arquitectura de la CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    AArch64,
    RiscV,
    Unknown,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Architecture::X86_64 => write!(f, "x86_64"),
            Architecture::AArch64 => write!(f, "AArch64"),
            Architecture::RiscV => write!(f, "RISC-V"),
            Architecture::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Manejador de interrupciones.
pub type InterruptHandler = fn() -> Result<(), anyhow::Error>;

/// Controlador de interrupciones.
pub trait InterruptController: Send + Sync {
    fn enable(&self) -> Result<(), anyhow::Error>;
    fn disable(&self) -> Result<(), anyhow::Error>;
    fn register_handler(&self, irq: u32, handler: InterruptHandler) -> Result<(), anyhow::Error>;
    fn unregister_handler(&self, irq: u32) -> Result<(), anyhow::Error>;
}

/// Trait principal de la HAL.
pub trait Hal: Send + Sync {
    fn init(&mut self) -> Result<(), anyhow::Error>;
    fn cpu_info(&self) -> CpuInfo;
    fn memory_info(&self) -> MemoryInfo;
    fn platform_name(&self) -> String;
    fn interrupt_controller(&self) -> &dyn InterruptController;
    fn reset(&self) -> Result<(), anyhow::Error>;
    fn shutdown(&self) -> Result<(), anyhow::Error>;
}

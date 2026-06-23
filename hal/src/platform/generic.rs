//! HAL genérica de fallback

use crate::traits::{
    Architecture, CpuInfo, Hal, InterruptController, InterruptHandler, MemoryInfo,
};
use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub struct GenericHal;

impl Default for GenericHal {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericHal {
    pub fn new() -> Self {
        Self
    }
}

impl Hal for GenericHal {
    fn init(&mut self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn cpu_info(&self) -> CpuInfo {
        CpuInfo {
            arch: Architecture::Unknown,
            cores: 1,
            threads: 1,
            freq_mhz: 0,
            features: Vec::new(),
            model: "Generic".to_string(),
            cache_l1_kb: 0,
            cache_l2_kb: 0,
            cache_l3_kb: 0,
        }
    }

    fn memory_info(&self) -> MemoryInfo {
        MemoryInfo {
            total_mb: 1024,
            available_mb: 512,
            numa_nodes: 1,
            huge_page_size_kb: 0,
            page_size_kb: 4,
        }
    }

    fn platform_name(&self) -> String {
        "Generic Platform".to_string()
    }

    fn interrupt_controller(&self) -> &dyn InterruptController {
        static CONTROLLER: GenericInterruptController = GenericInterruptController;
        &CONTROLLER
    }

    fn reset(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn shutdown(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

struct GenericInterruptController;

impl InterruptController for GenericInterruptController {
    fn enable(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn disable(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn register_handler(&self, _irq: u32, _handler: InterruptHandler) -> Result<(), anyhow::Error> {
        Ok(())
    }
    fn unregister_handler(&self, _irq: u32) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

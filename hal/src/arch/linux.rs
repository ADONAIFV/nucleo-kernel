#![allow(clippy::collapsible_if)]

//! HAL para Linux (lee información real del sistema vía /proc y /sys)

use crate::traits::{
    Architecture, CpuInfo, Hal, InterruptController, InterruptHandler, MemoryInfo,
};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

pub struct LinuxHal {
    initialized: AtomicBool,
}

impl Default for LinuxHal {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxHal {
    pub const fn new() -> Self {
        Self {
            initialized: AtomicBool::new(false),
        }
    }

    fn read_proc_file(_path: &str) -> Option<String> {
        #[cfg(not(test))]
        return std::fs::read_to_string(_path).ok();
        #[cfg(test)]
        return None;
    }

    fn parse_cpuinfo_value(content: &str, key: &str) -> Option<String> {
        for line in content.lines() {
            if let Some(stripped) = line.strip_prefix(key) {
                let parts: Vec<&str> = stripped.split(':').collect();
                if parts.len() >= 2 {
                    return Some(parts[1].trim().to_string());
                }
            }
        }
        None
    }

    fn detect_cores() -> usize {
        if let Some(content) = Self::read_proc_file("/proc/cpuinfo") {
            let mut count = 0;
            for line in content.lines() {
                if line.starts_with("processor") || line.starts_with("cpu") {
                    count += 1;
                }
            }
            if count > 0 {
                return count;
            }
        }
        #[cfg(not(test))]
        return unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as usize };
        #[cfg(test)]
        return 1;
    }

    fn detect_features(content: &str) -> Vec<String> {
        let mut features = Vec::new();
        for line in content.lines() {
            if line.contains("flags") || line.contains("Features") {
                if let Some(flags) = line.split(':').nth(1) {
                    for flag in flags.split_whitespace() {
                        features.push(flag.to_string());
                    }
                }
                break;
            }
        }
        features
    }

    fn detect_architecture() -> Architecture {
        if let Some(content) = Self::read_proc_file("/proc/cpuinfo") {
            if let Some(model) = Self::parse_cpuinfo_value(&content, "model name") {
                if model.contains("Intel") || model.contains("AMD") {
                    return Architecture::X86_64;
                }
            }
            if let Some(arch) = Self::parse_cpuinfo_value(&content, "CPU architecture") {
                if arch.contains("8") || arch.contains("ARM") {
                    return Architecture::AArch64;
                }
            }
            if let Some(arch) = Self::parse_cpuinfo_value(&content, "arch") {
                if arch.contains("riscv") {
                    return Architecture::RiscV;
                }
            }
        }
        // Fallback: usar target_arch en tiempo de compilación
        #[cfg(target_arch = "x86_64")]
        return Architecture::X86_64;
        #[cfg(target_arch = "aarch64")]
        return Architecture::AArch64;
        #[cfg(target_arch = "riscv64")]
        return Architecture::RiscV;
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        Architecture::Unknown
    }
}

impl Hal for LinuxHal {
    fn init(&mut self) -> Result<(), anyhow::Error> {
        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn cpu_info(&self) -> CpuInfo {
        let arch = Self::detect_architecture();
        let cores = Self::detect_cores();
        let content = Self::read_proc_file("/proc/cpuinfo").unwrap_or_default();

        CpuInfo {
            arch,
            cores,
            threads: cores,
            freq_mhz: Self::parse_cpuinfo_value(&content, "cpu MHz")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0),
            features: Self::detect_features(&content),
            model: Self::parse_cpuinfo_value(&content, "model name")
                .unwrap_or_else(|| "Unknown".to_string()),
            cache_l1_kb: 0,
            cache_l2_kb: 0,
            cache_l3_kb: 0,
        }
    }

    fn memory_info(&self) -> MemoryInfo {
        let mut total_mb = 1024;
        let mut available_mb = 512;
        let page_size_kb = 4;

        if let Some(content) = Self::read_proc_file("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        total_mb = parts[1].parse::<u64>().unwrap_or(1024) / 1024;
                    }
                }
                if line.starts_with("MemAvailable:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        available_mb = parts[1].parse::<u64>().unwrap_or(512) / 1024;
                    }
                }
            }
        }

        MemoryInfo {
            total_mb,
            available_mb,
            numa_nodes: 1,
            huge_page_size_kb: 0,
            page_size_kb,
        }
    }

    fn platform_name(&self) -> String {
        #[cfg(not(test))]
        {
            if let Ok(model) = std::fs::read_to_string("/sys/firmware/devicetree/base/model") {
                return model.trim().to_string();
            }
            if let Ok(product) = std::fs::read_to_string("/sys/class/dmi/id/product_name") {
                return product.trim().to_string();
            }
            if let Ok(name) = std::fs::read_to_string("/proc/sys/kernel/hostname") {
                return name.trim().to_string();
            }
        }
        "Linux Platform".to_string()
    }

    fn interrupt_controller(&self) -> &dyn InterruptController {
        static CONTROLLER: LinuxInterruptController = LinuxInterruptController;
        &CONTROLLER
    }

    fn reset(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn shutdown(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

struct LinuxInterruptController;

impl InterruptController for LinuxInterruptController {
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

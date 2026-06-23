//! Hardware Abstraction Layer (HAL) para Nucleo Kernel.
//!
//! Esta capa proporciona una interfaz unificada para acceder al hardware,
//! independientemente de la arquitectura o plataforma subyacente.

#![cfg_attr(not(test), no_std)]
extern crate alloc;
extern crate std;

pub mod arch;
pub mod platform;
pub mod traits;

pub use traits::{Architecture, CpuInfo, Hal, InterruptController, MemoryInfo};

// HAL por defecto según la plataforma
#[cfg(target_os = "linux")]
pub type DefaultHal = arch::linux::LinuxHal;

#[cfg(all(target_os = "none", target_arch = "x86_64"))]
pub type DefaultHal = arch::x86_64::X86Hal;

#[cfg(all(target_os = "none", target_arch = "aarch64"))]
pub type DefaultHal = arch::aarch64::AArch64Hal;

#[cfg(all(target_os = "none", target_arch = "riscv64"))]
pub type DefaultHal = arch::riscv::RiscVHal;

#[cfg(not(any(
    target_os = "linux",
    all(target_os = "none", target_arch = "x86_64"),
    all(target_os = "none", target_arch = "aarch64"),
    all(target_os = "none", target_arch = "riscv64")
)))]
pub type DefaultHal = platform::generic::GenericHal;

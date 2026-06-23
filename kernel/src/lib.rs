//! Nucleo Kernel - El núcleo del sistema operativo para agentes autónomos.
//!
//! Proporciona el daemon, syscalls, gestión de recursos, persistencia y
//! orquestación del sistema.

pub mod architecture;
pub mod compiler;
pub mod config;
pub mod daemon;
pub mod governance;
pub mod governor;
pub mod hal_impl;
pub mod hot_patch;
pub mod metrics;
pub mod modules;
pub mod persistence;
pub mod security;
pub mod syscalls;
pub mod wal;

pub use architecture::{
    ArchitectureInspector, KernelArchitecture, KernelMetadata, KernelTree, ModuleInfo, SystemState,
};
pub use config::{KernelConfig, ResourceLimits};
pub use daemon::{DaemonStatus, KernelDaemon};
pub use governor::ResourceGovernor;
pub use hot_patch::HotPatchManager;
pub use metrics::MetricsCollector;
pub use modules::timer::TimerManager;
pub use security::blocklist::Blocklist;
pub use syscalls::basic::SyscallContext;
pub use wal::Wal;

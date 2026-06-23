//! Daemon principal del kernel.

use crate::config::KernelConfig;
use crate::hal_impl::get_hal;
use crate::syscalls::basic::SyscallContext;
use crate::wal::Wal;
use parking_lot::Mutex;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    flag,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::{error, info};

use crate::architecture::ArchitectureInspector;
use crate::hot_patch::HotPatchManager;
use crate::metrics::MetricsCollector;
use crate::persistence::checkpoint::CheckpointManager;
use substrate::branch::{BranchError, BranchState, CowBranchManager};
use substrate::lego::LegoRegistry;
use substrate::security::capability::{CapDomain, CapFlags, CapabilitySystem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonStatus {
    Running,
    Stopping,
    Stopped,
}

pub struct KernelDaemon {
    config: KernelConfig,
    wal: Wal,
    status: Arc<Mutex<DaemonStatus>>,
    running: Arc<AtomicBool>,
    syscall_ctx: Arc<SyscallContext>,
}

impl KernelDaemon {
    pub fn new(config: KernelConfig) -> Result<Self, anyhow::Error> {
        let work_dir = PathBuf::from(&config.work_dir);
        std::fs::create_dir_all(&work_dir)?;

        // Inicializar HAL
        let mut hal = get_hal();
        hal.init()?;
        let cpu_info = hal.cpu_info();
        let mem_info = hal.memory_info();

        info!("🖥️  Hardware detectado:");
        info!(
            "   CPU: {} cores, {} threads",
            cpu_info.cores, cpu_info.threads
        );
        info!(
            "   Memoria: {} MB total, {} MB disponible",
            mem_info.total_mb, mem_info.available_mb
        );
        info!("   Plataforma: {}", hal.platform_name());

        let wal = Wal::new(&work_dir)?;

        let cow_branch = Arc::new(CowBranchManager::new(&work_dir));
        let cap_system = Arc::new(CapabilitySystem::new());
        let checkpoint_mgr = Arc::new(CheckpointManager::new(&work_dir));
        // Inicializar registro de Lego
        let lego_registry = Arc::new(LegoRegistry::new());

        // Inicializar colector de métricas
        let metrics_collector = Arc::new(MetricsCollector::new());

        let hot_patch_manager = Arc::new(HotPatchManager::new(Arc::clone(&lego_registry))?);

        // Inicializar inspector de arquitectura
        let architecture_inspector = Arc::new(ArchitectureInspector::new(
            config.clone(),
            Arc::clone(&lego_registry),
            Arc::clone(&metrics_collector),
            work_dir.clone(),
        ));

        // Registrar herramientas en el sistema de capacidades
        cap_system.register_tool(
            "workspace_read",
            CapDomain::Workspace,
            "read",
            CapFlags::read_only(),
        );
        cap_system.register_tool(
            "workspace_write",
            CapDomain::Workspace,
            "write",
            CapFlags::default(),
        );
        cap_system.register_tool("eval", CapDomain::Eval, "python", CapFlags::default());
        cap_system.register_tool("ipc_send", CapDomain::Ipc, "send", CapFlags::default());

        // Crear contexto de syscalls
        let config_dir = work_dir.join("configs"); // Directorio para configuraciones (ej. blocklist)
        let syscall_ctx = Arc::new(SyscallContext::new(
            work_dir.clone(),
            &config_dir,
            cow_branch.clone(),
            cap_system.clone(),
            checkpoint_mgr.clone(),
            hot_patch_manager.clone(),
            architecture_inspector.clone(),
        )?);

        Ok(Self {
            config,
            wal,
            status: Arc::new(Mutex::new(DaemonStatus::Running)),
            running: Arc::new(AtomicBool::new(true)),
            syscall_ctx,
        })
    }

    pub fn run(&self) -> Result<(), anyhow::Error> {
        // Replay WAL
        if self.wal.replay().is_ok() {
            info!("📂 WAL recuperado con éxito.");
        }

        let status = self.status.clone();
        let running = self.running.clone();
        let term_flag = Arc::new(AtomicBool::new(false));
        let int_flag = Arc::new(AtomicBool::new(false));
        let _term_sigid = flag::register(SIGTERM, term_flag.clone())?;
        let _int_sigid = flag::register(SIGINT, int_flag.clone())?;

        let checkpoint_every = self.config.checkpoint_interval_secs;
        let mut checkpoint_counter = 0;

        info!("🚀 Nucleo Kernel arrancado. PID: {}", std::process::id());

        while running.load(Ordering::SeqCst) {
            if term_flag.load(std::sync::atomic::Ordering::Relaxed)
                || int_flag.load(std::sync::atomic::Ordering::Relaxed)
            {
                info!("🛑 Señal de terminación recibida");
                running.store(false, Ordering::SeqCst);
                break;
            }

            checkpoint_counter += 1;
            if checkpoint_counter >= checkpoint_every {
                checkpoint_counter = 0;
                if let Err(e) = self.wal.checkpoint() {
                    error!("⚠️ Error en checkpoint: {}", e);
                }
            }

            std::thread::sleep(Duration::from_secs(1));
        }

        info!("🛑 Apagando kernel...");
        let _ = self.wal.shutdown();
        *status.lock() = DaemonStatus::Stopped;
        info!("✅ Kernel detenido.");

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn syscall_ctx(&self) -> &SyscallContext {
        &self.syscall_ctx
    }

    pub fn get_running_flag(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    pub fn hot_patch_manager(&self) -> Arc<HotPatchManager> {
        self.syscall_ctx.hot_patch_manager.clone()
    }

    pub fn architecture_inspector(&self) -> Arc<ArchitectureInspector> {
        self.syscall_ctx.architecture_inspector.clone()
    }

    pub fn status(&self) -> u64 {
        // En una implementación real, esto vendría de MetricsCollector
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    pub fn list_modules(&self) -> Vec<String> {
        self.syscall_ctx.hot_patch_manager.lego_registry.list()
    }
}

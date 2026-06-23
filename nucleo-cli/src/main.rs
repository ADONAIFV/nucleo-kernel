//! CLI de administración para Nucleo Kernel.
//! Permite realizar acciones de administración y pruebas sobre el kernel.

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
use tracing::info;

// Importar componentes del kernel
use nucleo_kernel::architecture::{KernelArchitecture, KernelTree, ModuleInfo, SystemState};
use nucleo_kernel::syscalls::advanced::AdvancedSyscalls;
use nucleo_kernel::{KernelConfig, KernelDaemon};
use substrate::security::capability::CapabilitySet;
use substrate::{AgentId, AgentSpec};

#[derive(Parser)]
#[command(
    name = "nucleo-admin",
    version,
    about = "Administración de Nucleo Kernel"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Conectar al kernel local (modo desarrollo)
    #[arg(long, default_value_t = true)]
    local: bool,

    /// Ruta al socket Unix del kernel (si no está en local)
    #[arg(long)]
    socket: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Muestra información general del kernel
    Info,

    /// Comandos de gestión de módulos
    Modules {
        #[command(subcommand)]
        action: ModuleAction,
    },

    /// Muestra la arquitectura completa del kernel
    Architecture,

    /// Muestra el estado del sistema
    System {
        #[command(subcommand)]
        action: SystemAction,
    },

    /// Ejecuta código en el sandbox
    Eval {
        /// Lenguaje (python, bash, js, rust, wasm)
        language: String,
        /// Código a ejecutar
        code: String,
        /// Timeout en segundos
        #[arg(short, long, default_value_t = 5)]
        timeout: u64,
    },

    /// Gestión de agentes
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },

    /// Gestión de ramas (branching)
    Branch {
        #[command(subcommand)]
        action: BranchAction,
    },

    /// Gestión de checkpoints
    Checkpoint {
        #[command(subcommand)]
        action: CheckpointAction,
    },
}

#[derive(Subcommand)]
enum ModuleAction {
    /// Lista todos los módulos cargados
    List,
    /// Muestra detalles de un módulo
    Show { name: String },
    /// Reemplaza un módulo en caliente
    Replace {
        name: String,
        /// Nueva versión
        #[arg(short, long)]
        version: String,
        /// Ruta al archivo WASM
        #[arg(short, long)]
        wasm: String,
    },
}

#[derive(Subcommand)]
enum SystemAction {
    /// Muestra el estado del sistema (CPU, memoria, agentes)
    State,
}

#[derive(Subcommand)]
enum AgentAction {
    /// Lista agentes activos
    List,
    /// Crea un agente de prueba
    Spawn { name: String },
    /// Detiene un agente
    Stop { agent_id: u64 },
}

#[derive(Subcommand)]
enum BranchAction {
    /// Bifurca un agente (fork)
    Fork { agent_id: u64 },
    /// Confirma una rama (commit)
    Commit { agent_id: u64, branch_id: u64 },
    /// Lista ramas de un agente
    List { agent_id: u64 },
}

#[derive(Subcommand)]
enum CheckpointAction {
    /// Crea un checkpoint de un agente
    Create { agent_id: u64 },
    /// Restaura un checkpoint de un agente
    Restore {
        agent_id: u64,
        checkpoint_id: String,
    },
    /// Lista checkpoints de un agente
    List { agent_id: u64 },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    if cli.local {
        run_local(cli.command)?;
    } else {
        println!("⚠️  Modo remoto aún no implementado. Usa --local para pruebas.");
        std::process::exit(1);
    }

    Ok(())
}

fn run_local(command: Commands) -> Result<()> {
    let config = KernelConfig::from_file("configs/nucleo.toml").unwrap_or_else(|_| {
        println!("⚠️  No se encontró configs/nucleo.toml, usando configuración básica");
        KernelConfig {
            limits: nucleo_kernel::config::ResourceLimits {
                max_agents: 100,
                max_memory_mb: 4096,
                max_file_descriptors: 1024,
            },
            work_dir: "/data/data/com.termux/files/home/workspace/nucleo".to_string(),
            checkpoint_interval_secs: 3600,
        }
    });

    let daemon = KernelDaemon::new(config)?;
    let syscalls = daemon.syscall_ctx();

    let advanced = AdvancedSyscalls::new(
        syscalls,
        syscalls.branch_manager.clone(),
        daemon.hot_patch_manager().clone(),
        daemon.architecture_inspector().clone(),
        std::path::PathBuf::from("."),
        "aarch64",
    );

    match command {
        Commands::Info => {
            println!("🔍 Información del Kernel Nucleo");
            println!("=================================");
            println!("Versión: {}", env!("CARGO_PKG_VERSION"));
            println!("Arquitectura: {}", std::env::consts::ARCH);
            println!("SO: {}", std::env::consts::OS);
            println!("Uptime: {} segundos", daemon.status());
            println!("Agentes activos: {}", syscalls.list_agents().len());
            println!("Módulos cargados: {}", daemon.list_modules().len());
        }

        Commands::Modules { action } => match action {
            ModuleAction::List => {
                let modules = daemon.list_modules();
                println!("📦 Módulos cargados ({})", modules.len());
                for name in modules {
                    println!("  - {}", name);
                }
            }
            ModuleAction::Show { name } => {
                println!("🔍 Módulo: {}", name);
                println!("  Estado: Healthy (simulado)");
                println!("  Versión: 1.0.0 (simulado)");
            }
            ModuleAction::Replace {
                name,
                version,
                wasm,
            } => {
                println!("🔄 Reemplazando módulo '{}' a versión {}", name, version);
                let wasm_bytes = std::fs::read(&wasm)?;
                advanced.replace_module(AgentId(1), &name, &version, &wasm_bytes)?;
                println!("✅ Módulo reemplazado exitosamente.");
            }
        },

        Commands::Architecture => {
            println!("🏗️  Arquitectura del Kernel");
            println!("==========================");
            let tree = advanced.get_kernel_tree(AgentId(0))?;
            println!("Raíz: {}", tree.root.display());
            println!("Config: {}", tree.config_path.display());
            println!("Módulos: {}", tree.modules_path.display());
            println!("Workspaces: {}", tree.workspaces_path.display());
            println!("Checkpoints: {}", tree.checkpoints_path.display());
            println!("\nArchivos:");
            for file in tree.files {
                println!("  - {}", file);
            }
        }

        Commands::System { action } => match action {
            SystemAction::State => {
                let state = advanced.get_system_state(AgentId(0))?;
                println!("📊 Estado del Sistema");
                println!("=====================");
                println!("Uptime: {} segundos", state.uptime_secs);
                println!("Agentes activos: {}", state.agents_active);
                println!("Memoria usada: {} MB", state.memory_used_mb);
                println!("Memoria total: {} MB", state.memory_total_mb);
                println!("CPU usado: {:.1}%", state.cpu_usage_percent);
                println!("Módulos cargados: {}", state.modules_loaded);
            }
        },

        Commands::Eval {
            language,
            code,
            timeout,
        } => {
            println!("🐍 Ejecutando código en sandbox");
            println!("Lenguaje: {}", language);
            println!("Timeout: {}s", timeout);
            let result = advanced.eval(&language, &code, timeout)?;
            println!("Resultado:\n{}", result);
        }

        Commands::Agent { action } => match action {
            AgentAction::List => {
                let agents = syscalls.list_agents();
                println!("👤 Agentes activos ({})", agents.len());
                for id in agents {
                    println!("  - Agente {}", id);
                }
            }
            AgentAction::Spawn { name } => {
                println!("🧪 Spawneando agente de prueba: {}", name);
                let agent_id = syscalls.spawn_agent(AgentSpec {
                    name: name.clone(),
                    description: "Agente de prueba desde CLI".to_string(),
                    capabilities: CapabilitySet::wildcard(),
                })?;
                println!("✅ Agente creado con ID: {}", agent_id.0);
            }
            AgentAction::Stop { agent_id } => {
                println!("🛑 Deteniendo agente {}", agent_id);
                syscalls.stop_agent(AgentId(agent_id))?;
                println!("✅ Agente detenido");
            }
        },

        Commands::Branch { action } => match action {
            BranchAction::Fork { agent_id } => {
                println!("🌿 Bifurcando agente {}", agent_id);
                let branch_id = syscalls.agent_fork(AgentId(agent_id))?;
                println!("✅ Rama creada con ID: {}", branch_id);
            }
            BranchAction::Commit {
                agent_id,
                branch_id,
            } => {
                println!("🔀 Confirmando rama {} del agente {}", branch_id, agent_id);
                syscalls.agent_commit(AgentId(agent_id), branch_id)?;
                println!("✅ Rama confirmada exitosamente");
            }
            BranchAction::List { agent_id } => {
                let branches = syscalls.list_branches(AgentId(agent_id));
                println!("🌿 Ramas del agente {} ({})", agent_id, branches.len());
                for id in branches {
                    println!("  - Rama {:?}", id);
                }
            }
        },

        Commands::Checkpoint { action } => match action {
            CheckpointAction::Create { agent_id } => {
                println!("💾 Creando checkpoint del agente {}", agent_id);
                let checkpoint_path = syscalls.create_checkpoint(AgentId(agent_id), None)?;
                println!("✅ Checkpoint creado en: {}", checkpoint_path.display());
            }
            CheckpointAction::Restore {
                agent_id,
                checkpoint_id,
            } => {
                println!(
                    "🔄 Restaurando checkpoint {} del agente {}",
                    checkpoint_id, agent_id
                );
                syscalls.restore_checkpoint(
                    AgentId(agent_id),
                    &std::path::PathBuf::from(checkpoint_id),
                )?;
                println!("✅ Checkpoint restaurado");
            }
            CheckpointAction::List { agent_id } => {
                let checkpoints = syscalls.list_checkpoints(AgentId(agent_id));
                println!(
                    "💾 Checkpoints del agente {} ({})",
                    agent_id,
                    checkpoints.len()
                );
                for path in checkpoints {
                    println!("  - {}", path.display());
                }
            }
        },
    }

    Ok(())
}

//! Punto de entrada del daemon del kernel.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber;

#[derive(Parser, Debug)]
#[command(
    name = "nucleo-kernel",
    version,
    about = "Kernel para agentes autónomos"
)]
struct Cli {
    #[arg(short, long, default_value = "configs/nucleo.toml")]
    config: PathBuf,

    #[arg(long)]
    validate: bool,

    #[arg(long)]
    recover: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let config = nucleo_kernel::config::KernelConfig::from_file(&cli.config)?;

    if cli.validate {
        config.validate()?;
        println!("✅ Configuración válida.");
        return Ok(());
    }

    if cli.recover {
        let wal = nucleo_kernel::Wal::new(cli.config.parent().unwrap_or(&PathBuf::from(".")))?;
        wal.replay()?;
        return Ok(());
    }

    let daemon = nucleo_kernel::daemon::KernelDaemon::new(config)?;
    daemon.run()?;

    Ok(())
}

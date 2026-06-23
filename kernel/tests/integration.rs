//! Prueba de integración básica para el kernel.

use nucleo_kernel::{KernelConfig, KernelDaemon};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
#[ignore] // Deshabilitado temporalmente por problemas de sincronización en CI
fn test_kernel_daemon_lifecycle() {
    let dir = tempdir().unwrap();
    let config_content = format!(
        r#"
        [limits]
        max_agents = 10
        max_memory_mb = 256
        max_file_descriptors = 64

        work_dir = "{}"
        checkpoint_interval_secs = 2
        "#,
        dir.path().display()
    );
    let config_path = dir.path().join("nucleo.toml");
    std::fs::write(&config_path, config_content).unwrap();

    let config = KernelConfig::from_file(&config_path).unwrap();
    let daemon = KernelDaemon::new(config).unwrap();

    let running_flag = daemon.get_running_flag();
    let handle = thread::spawn(move || {
        daemon.run().unwrap();
    });

    // Esperar a que el daemon arranque y ejecute al menos un checkpoint
    thread::sleep(Duration::from_secs(5));

    running_flag.store(false, Ordering::SeqCst);
    handle.join().unwrap();

    // Verificar que el checkpoint existe
    assert!(dir.path().join("nucleo.checkpoint").exists());
    assert!(dir.path().join("nucleo.wal").exists());
}

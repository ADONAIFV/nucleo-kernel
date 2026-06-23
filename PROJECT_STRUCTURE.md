# Proyecto Núcleo: Estructura de Archivos

Este documento proporciona una visión detallada de la jerarquía de directorios y archivos del proyecto Núcleo.

## Árbol de Directorios

```text
nucleo/
├── Cargo.lock
├── Cargo.toml
├── build_log.txt
├── checkpoints
├── configs
│   └── nucleo.toml
├── docs
│   ├── agent_development.md
│   ├── architecture.md
│   ├── changelog.md
│   ├── installation.md
│   ├── kernel.md
│   └── syscalls.md
├── hal
│   ├── Cargo.toml
│   ├── src
│   │   ├── arch
│   │   │   ├── linux.rs
│   │   │   └── mod.rs
│   │   ├── lib.rs
│   │   ├── platform
│   │   │   └── generic.rs
│   │   ├── platform.rs
│   │   └── traits.rs
│   └── tests
│       └── integration.rs
├── kernel
│   ├── Cargo.toml
│   ├── configs
│   │   └── nucleo.toml
│   ├── src
│   │   ├── architecture.rs
│   │   ├── compiler
│   │   │   ├── mod.rs
│   │   │   └── unikernel.rs
│   │   ├── config.rs
│   │   ├── daemon.rs
│   │   ├── distributed
│   │   │   ├── cluster.rs
│   │   │   ├── discovery.rs
│   │   │   ├── mod.rs
│   │   │   └── replication.rs
│   │   ├── governance
│   │   │   ├── inspector.rs
│   │   │   └── mod.rs
│   │   ├── governor.rs
│   │   ├── hal_impl.rs
│   │   ├── hot_patch.rs
│   │   ├── lib.rs
│   │   ├── main.rs
│   │   ├── metrics.rs
│   │   ├── modules
│   │   │   ├── mod.rs
│   │   │   └── timer.rs
│   │   ├── persistence
│   │   │   ├── checkpoint.rs
│   │   │   └── mod.rs
│   │   ├── security
│   │   │   ├── blocklist.rs
│   │   │   ├── ebpf.rs
│   │   │   └── mod.rs
│   │   ├── syscalls
│   │   │   ├── advanced.rs
│   │   │   ├── basic.rs
│   │   │   └── mod.rs
│   │   └── wal.rs
│   └── tests
│       └── integration.rs
├── kernel.log
├── kernel_data
│   ├── agent_1
│   │   └── memory.json
│   ├── agent_workspaces
│   │   └── 1
│   ├── checkpoints
│   │   └── agent_1
│   │       └── checkpoint_1782143860
│   │           ├── checkpoint.json
│   │           ├── memory.json
│   │           └── workspace
│   ├── mmu_cache
│   │   └── l3
│   │       └── agent_0
│   ├── modules
│   ├── nucleo.checkpoint
│   ├── nucleo.wal
│   └── wal
├── mmu_cache
│   └── l3
│       └── agent_0
├── modules
│   ├── my_module.wat
│   ├── v1.wasm
│   ├── v1.wat
│   ├── v2.wasm
│   └── v2.wat
├── nucleo-cli
│   ├── Cargo.toml
│   └── src
│       └── main.rs
├── nucleo.toml
├── nucleo.wal
├── rust-toolchain.toml
├── scripts
│   ├── build.sh
│   ├── install.sh
│   └── test.sh
└── substrate
    ├── Cargo.toml
    ├── src
    │   ├── branch.rs
    │   ├── eval.rs
    │   ├── examples
    │   │   └── demo.rs
    │   ├── ipc.rs
    │   ├── lego.rs
    │   ├── lib.rs
    │   ├── memory_store.rs
    │   ├── mmu.rs
    │   ├── process.rs
    │   ├── scheduler
    │   │   ├── mod.rs
    │   │   └── semantic.rs
    │   ├── security
    │   │   ├── capability.rs
    │   │   └── mod.rs
    │   ├── untrusted.rs
    │   ├── wasm_runtime.rs
    │   └── workspace.rs
    └── tests
        ├── eval_tests.rs
        ├── memory_store_tests.rs
        └── workspace_tests.rs
```

## Descripción de Componentes Principales

### 📦 Núcleo del Sistema (Kernel)
Ubicado en `kernel/`, contiene la implementación central del sistema operativo, incluyendo el planificador, la gestión de memoria y el motor de ejecución WASM.

### 🏗️ Capa de Abstracción de Hardware (HAL)
Ubicado en `hal/`, proporciona las interfaces para que el kernel interactúe con el hardware de forma agnóstica.

### 🧱 Substrato (Substrate)
Ubicado en `substrate/`, implementa las funciones de bajo nivel como la MMU, gestión de procesos y el runtime de WASM.

### 🛠️ Interfaz de Línea de Comandos (CLI)
Ubicado en `nucleo-cli/`, contiene la herramienta de administración para usuarios y desarrolladores.

### 🧩 Módulos
Ubicado en `modules/`, contiene los binarios `.wasm` y archivos `.wat` para la extensión dinámica del sistema.

### 📄 Documentación
Ubicado en `docs/`, contiene las especificaciones arquitectónicas, guías de instalación y referencias de syscalls.

---
*Generado automáticamente por Hermes Master Developer*

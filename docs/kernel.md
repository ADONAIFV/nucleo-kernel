# Núcleo V2: Implementación y Gestión del Kernel

El crate `nucleo-kernel` es el cerebro de Núcleo V2, orquestando todos los componentes del sistema para proporcionar un entorno robusto y seguro para los agentes autónomos. Este documento detalla la implementación clave de sus gestores y cómo interactúan para lograr la funcionalidad deseada.

## Componente Principal: `KernelDaemon`

El `KernelDaemon` es el punto de entrada del kernel y el responsable de inicializar y gestionar los servicios principales. Su ciclo de vida incluye:

1.  **Inicialización**: Al arrancar, `KernelDaemon` configura el entorno, incluyendo:
    *   **`CowBranchManager`**: Gestiona las ramas (forks) de agentes con Copy-on-Write, permitiendo la experimentación y el versionado del estado del agente.
    *   **`CapabilitySystem`**: Impone un modelo de seguridad granular, donde cada acción realizada por un agente debe estar explícitamente permitida por una capacidad.
    *   **`CheckpointManager`**: Permite la creación y restauración de puntos de guardado atómicos del estado del agente, garantizando la durabilidad y la recuperación ante fallos.
    *   **`HotPatchManager`**: Habilita la actualización dinámica de módulos de agentes o del propio kernel.
    *   **`ArchitectureInspector`**: Recopila y expone métricas y el estado del sistema.
    *   **`SyscallContext`**: Actúa como un contenedor central para todos estos gestores, facilitando su acceso a través de la interfaz de syscalls.

2.  **Bucle Principal**: El daemon entra en un bucle que monitoriza el estado del sistema, gestiona los agentes, procesa las syscalls y orquesta las operaciones de hot-patching.

3.  **Shutdown**: Maneja el cierre ordenado del sistema, asegurando que todos los agentes y servicios se detengan correctamente y que el estado persistente se guarde.

## El Sistema de Syscalls

El `SyscallContext` es la interfaz que los agentes utilizan para solicitar servicios al kernel. Cada syscall es validada por el `CapabilitySystem` antes de su ejecución. Se han añadido wrappers a `SyscallContext` para simplificar la interacción con los gestores subyacentes:

*   **Gestión de Agentes**: Métodos como `list_agents()`, `spawn_agent(spec)`, `stop_agent(id)` para el control del ciclo de vida del agente.
*   **Branching (`CowBranchManager`)**: `agent_fork(id)`, `agent_commit(id, branch_id)`, `list_branches(id)` para la gestión de estados versionados.
*   **Checkpoints (`CheckpointManager`)**: `create_checkpoint(id, meta)`, `restore_checkpoint(id, path)`, `list_checkpoints(id)` para la durabilidad del estado.
*   **Memoria (`MemoryStore`)**: `memory_list`, `memory_get`, `memory_store`, `memory_delete` para la persistencia de datos.

## Gestión de la Seguridad: `CapabilitySystem` (Substrate)

Aunque el `CapabilitySystem` reside ahora en el crate `substrate`, su integración y uso son fundamentales en el kernel.

*   **Registro de Capacidades**: El kernel registra capacidades explícitas para cada herramienta y operación (ej., `workspace_read`, `workspace_write`, `eval_python`, `ipc_send`).
*   **Asignación a Agentes**: A cada agente se le asigna un conjunto de capacidades que definen sus permisos.
*   **Validación en Syscalls**: Antes de procesar cualquier syscall, el `CapabilitySystem` verifica que el agente llamante posea la capacidad necesaria para la operación solicitada. Esto refuerza el modelo de seguridad de menor privilegio, similar a seL4.

## Durabilidad y Recuperación: `CheckpointManager`

El `CheckpointManager` garantiza que el estado crítico de un agente pueda ser guardado y restaurado de forma fiable.

*   **Checkpoints Atómicos**: Cada checkpoint captura el estado completo del workspace y la memoria del agente en un directorio temporal, renombrándolo atómicamente al finalizar. Esto previene la corrupción de datos en caso de un fallo durante el proceso de guardado.
*   **Integridad**: Los checkpoints incluyen hashes del workspace y la memoria para verificar la integridad durante la restauración.
*   **Restauración**: Permite a un agente reanudar su ejecución desde un punto de guardado anterior, recuperándose de errores o reinicios del sistema.

## Hot Patching de Módulos

El `HotPatchManager` permite la actualización de componentes del sistema sin necesidad de reiniciar.

*   **Módulos WebAssembly**: Carga y gestiona módulos compilados a WebAssembly (WASM), que pueden ser reemplazados en tiempo de ejecución.
*   **Lego Registry**: Mantiene un registro de los módulos "Lego" disponibles, facilitando su gestión y actualización.

## Observabilidad: `ArchitectureInspector` y `SystemMetrics`

Estos módulos proporcionan información vital sobre el rendimiento y el estado interno del kernel:

*   **`SystemMetrics`**: Recopila datos sobre el uso de CPU, memoria, E/S y otros recursos.
*   **`ArchitectureInspector`**: Ofrece una vista estructurada de la arquitectura del kernel, los módulos cargados, los agentes activos y el estado de los recursos.

## Lecciones Aprendidas en la Implementación del Kernel

*   **Centralización del Contexto**: Consolidar los gestores clave en `SyscallContext` simplificó el acceso y la coherencia en las syscalls.
*   **Errores de Propiedad y Mutabilidad**: Las correcciones detalladas en `HotPatchManager` y `SyscallContext` fueron cruciales para manejar la mutabilidad del estado compartido (`Arc<Mutex<T>>`) y las expectativas del runtime `wasmtime` sobre los préstamos.
*   **Comunicación entre Cratos**: La refactorización del `CapabilitySystem` de `kernel` a `substrate` demostró la importancia de una jerarquía de dependencias clara para evitar ciclos y mejorar la modularidad.

Este documento proporciona una comprensión profunda de cómo se implementa y gestiona el kernel de Núcleo V2, y cómo sus componentes críticos interactúan para formar un sistema operativo de agentes robusto y seguro.

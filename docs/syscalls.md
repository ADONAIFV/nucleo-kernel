# Interfaz de Syscalls de Núcleo V2

Las syscalls son el único medio a través del cual un agente puede interactuar con el kernel y el hardware. Todas las llamadas son mediadas por el `CapabilitySystem` y el `SyscallContext`.

## Estructura de una Syscall

Cada syscall sigue el flujo:
`Agente` $\rightarrow$ `SyscallContext` $\rightarrow$ `Validación de Capacidad` $\rightarrow$ `Ejecución en Gestor` $\rightarrow$ `Retorno de Resultado`.

## Catálogo de Syscalls

### 1. Gestión de Agentes (Lifecycle)
| Syscall | Descripción | Capacidad Requerida |
| :--- | :--- | :--- |
| `list_agents()` | Lista todos los agentes activos en el sistema. | `Admin` |
| `spawn_agent(spec)` | Crea y lanza un nuevo agente basado en la especificación. | `Admin` |
| `stop_agent(id)` | Detiene la ejecución de un agente específico. | `Admin` |

### 2. Branching y Experimentación (CoW)
| Syscall | Descripción | Capacidad Requerida |
| :--- | :--- | :--- |
| `agent_fork(id)` | Crea una rama Copy-on-Write del estado del agente. | `Workspace::Fork` |
| `agent_commit(id, branch_id)` | Fusiona los cambios de una rama en el estado padre. | `Workspace::Commit` |
| `agent_abort(id, branch_id)` | Descarta una rama y limpia sus recursos. | `Workspace::Abort` |
| `list_branches(id)` | Lista todas las ramas activas de un agente. | `Workspace::Read` |

### 3. Durabilidad y Persistencia (Checkpoints)
| Syscall | Descripción | Capacidad Requerida |
| :--- | :--- | :--- |
| `create_checkpoint(id, meta)` | Crea un punto de guardado atómico del agente. | `Persistence::Write` |
| `restore_checkpoint(id, path)` | Restaura el agente a un estado guardado previo. | `Persistence::Restore` |
| `list_checkpoints(id)` | Lista los checkpoints disponibles para un agente. | `Persistence::Read` |

### 4. Gestión de Memoria y Workspace
| Syscall | Descripción | Capacidad Requerida |
| :--- | :--- | :--- |
| `workspace_read(path)` | Lee el contenido de un archivo en el workspace. | `Workspace::Read` |
| `workspace_write(path, data)` | Escribe datos en un archivo del workspace. | `Workspace::Write` |
| `memory_get(key)` | Recupera un valor de la memoria persistente. | `MemoryStore::Read` |
| `memory_store(key, val)` | Guarda un valor en la memoria persistente. | `MemoryStore::Write` |
| `memory_list()` | Lista todas las claves de la memoria del agente. | `MemoryStore::Read` |

### 5. Ejecución y Comunicación
| Syscall | Descripción | Capacidad Requerida |
| :--- | :--- | :--- |
| `eval(code, lang)` | Ejecuta código en un entorno sandbox (ej. Python). | `Eval::Execute` |
| `ipc_send(dest, msg)` | Envía un mensaje a otro agente o al kernel. | `Ipc::Send` |
| `ipc_recv()` | Recibe mensajes pendientes en la cola del agente. | `Ipc::Receive` |

## Manejo de Errores

Las syscalls devuelven un `Result<T, E>`. Los errores más comunes son:
*   **`PermissionDenied`**: El agente no posee la capacidad necesaria para la operación.
*   **`ResourceNotFound`**: El archivo, agente o rama especificado no existe.
*   **`IntegrityError`**: Fallo en la verificación de hash durante la restauración de un checkpoint.
*   **`WorkspaceError`**: Error de E/S al interactuar con el sistema de archivos.

## Recomendaciones de Uso

Para maximizar la estabilidad, se recomienda:
1.  **Verificar Capacidades**: Consultar el `AgentSpec` antes de intentar syscalls restringidas.
2.  **Cadenas de Checkpoints**: Realizar un checkpoint antes de ejecutar `eval` con código generado dinámicamente.
3.  **Gestión de Ramas**: Utilizar ramas cortas y específicas para tareas de exploración, haciendo commit solo de los resultados finales.

# Desarrollo de Agentes en Núcleo V2

El desarrollo de agentes en Núcleo V2 ha evolucionado hacia un modelo de "ciclo de vida gestionado", donde el kernel proporciona no solo la ejecución, sino también herramientas avanzadas para la experimentación, seguridad y durabilidad.

## El Modelo de Agente

Un agente en Núcleo V2 se define por un `AgentSpec`, que actúa como su "manifiesto":
*   **Nombre y Descripción**: Identidad del agente.
*   **Conjunto de Capacidades**: Define estrictamente qué puede y no puede hacer el agente dentro del sistema.

## Flujo de Desarrollo y Ejecución

### 1. Definición de Capacidades
A diferencia de los sistemas tradicionales donde el agente tiene los permisos del usuario que lo ejecuta, en Núcleo V2 el desarrollador debe definir un conjunto de capacidades.
*   **Ejemplo**: Un agente de análisis de logs solo tendrá capacidades de `Workspace::Read` en rutas específicas y `Eval::Python` para procesar los datos.
*   **Beneficio**: Esto reduce drásticamente la superficie de ataque y evita que un agente comprometido o con un bug pueda afectar la integridad del kernel o de otros agentes.

### 2. Experimentación con Branching (CoW)
Una de las capacidades más potentes para el desarrollo es el sistema de bifurcación Copy-on-Write (CoW). Los desarrolladores pueden implementar flujos de "Pensamiento y Verificación":
*   **Fork**: El agente crea una rama de su estado actual (`agent_fork`).
*   **Explore**: El agente prueba una hipótesis, escribe archivos temporales o modifica su memoria en esa rama aislada.
*   **Evaluate**: El agente verifica si el resultado es el esperado.
*   **Commit/Abort**: Si la hipótesis es correcta, hace un `agent_commit` para fusionar los cambios en el estado principal. Si falla, un `agent_abort` elimina instantáneamente toda la rama y sus efectos secundarios.
*   **Uso**: Ideal para agentes que generan código, resuelven problemas matemáticos o realizan búsquedas exhaustivas en el sistema de archivos.

### 3. Garantía de Durabilidad con Checkpoints
Para evitar la pérdida de progreso en tareas largas, los agentes pueden utilizar el sistema de checkpoints:
*   **Guardado Atómico**: El agente puede solicitar un checkpoint (`create_checkpoint`) antes de una operación arriesgada o al finalizar un hito.
*   **Recuperación**: Si el agente falla o el sistema se reinicia, puede reanudarse exactamente desde el último checkpoint (`restore_checkpoint`), recuperando tanto su memoria como el estado de su workspace.

## Implementación Técnica (WASM)

Los agentes se implementan preferiblemente como módulos WebAssembly (WASM). Esto proporciona:
*   **Aislamiento Total**: El agente corre en un sandbox donde no puede acceder a nada que no sea a través de las syscalls del kernel.
*   **Portabilidad**: El mismo binario `.wat` o `.wasm` corre en cualquier plataforma soportada por el HAL.
*   **Actualización en Caliente**: El kernel puede reemplazar el módulo WASM de un agente en ejecución sin detener el proceso, permitiendo actualizaciones sin tiempo de inactividad.

## Buenas Prácticas para Desarrolladores

1.  **Principio de Menor Privilegio**: Solicita solo las capacidades estrictamente necesarias.
2.  **Uso de Ramas para Tareas No Deterministas**: Siempre usa `fork` antes de realizar operaciones que puedan corromper el estado del workspace o la memoria.
3.  **Checkpointing Estratégico**: Implementa guardados automáticos tras completar sub-tareas complejas.
4.  **Modularidad (Legos)**: Divide la lógica del agente en módulos pequeños y reutilizables que puedan ser actualizados mediante el `HotPatchManager`.

Este enfoque transforma el desarrollo de agentes de un simple script de ejecución a la creación de entidades autónomas resilientes, seguras y capaces de auto-corrección.

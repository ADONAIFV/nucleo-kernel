# Arquitectura de Núcleo V2: Sistema Operativo Modular Híbrido

Núcleo V2 es un sistema operativo experimental de alto rendimiento diseñado para la orquestación y ejecución segura de agentes autónomos. Su diseño se basa en la separación radical de responsabilidades, permitiendo una ejecución tanto en entornos locales restringidos como en infraestructuras de nube escalables.

## 🌐 Modelo de Ejecución Híbrido

A diferencia de los sistemas tradicionales, Núcleo V2 está diseñado para operar en dos modos complementarios:

1.  **Modo Local (Edge/Mobile)**: Ejecución en dispositivos con recursos limitados (ej. Android vía Termux). Se utiliza para la gestión de archivos, compilación de módulos y control ligero.
2.  **Modo Cloud (Ephemeral Runtime)**: Ejecución en entornos Linux puros (ej. Hugging Face Spaces) mediante contenedores Docker. Este modo permite el uso de todo el potencial del hardware para el runtime de agentes pesados y la gestión de memoria avanzada.

## 🏗️ Capas del Sistema (Stack Tecnológico)

La arquitectura sigue un modelo de capas estrictamente jerárquico para garantizar la seguridad y la modularidad:

### 1. Capa de Substrato (`substrate/`)
Es el fundamento de confianza. Implementa la gestión de bajo nivel:
*   **Gestión de Memoria (MMU)**: Control de aislamiento de memoria para procesos WASM.
*   **Scheduler**: Planificador de procesos con soporte para tareas concurrentes.
*   **Runtime de WASM**: Motor de ejecución para el código no confiable de los agentes.

### 2. Capa de Abstracción de Hardware (`hal/`)
Actúa como el mediador entre el núcleo y la realidad física del hardware. Proporciona una interfaz estandarizada para que el kernel pueda ejecutarse en diferentes arquitecturas sin cambios en la lógica central.

### 3. Núcleo del Sistema (`kernel/`)
El cerebro del sistema. Orquesta la interacción entre las capas y gestiona las funciones críticas:
*   **Gestión de Procesos**: Ciclo de vida de los agentes.
*   **Hot-Patching Engine**: Capacidad de actualizar módulos en tiempo de ejecución sin reiniciar el kernel.
*   **Governance & Security**: Implementación de políticas de seguridad y control de acceso.

### 4. Interfaz de Control (`nucleo-cli/`)
La capa de interacción humana y de automatización. Permite la administración del sistema, la inspección de módulos y la ejecución de comandos de diagnóstico.

## 🧩 Modularidad y Hot-Patching

El corazón de la innovación de Núcleo es su capacidad de **Hot-Patching**. Gracias a la arquitectura basada en módulos WASM, el sistema puede reemplazar componentes críticos (como un driver de red o un módulo de logging) en tiempo de ejecución, simplemente cargando un nuevo binario y actualizando la tabla de despacho del kernel.

---
*Última actualización: Junio 2026 - Integración de Modelo Híbrido y Hot-Patching.*

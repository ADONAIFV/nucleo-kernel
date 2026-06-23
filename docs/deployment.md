# Manual de Despliegue: Entornos Efímeros (Cloud Runtime)

Este documento describe la metodología para desplegar Núcleo V2 en entornos de ejecución en la nube, específicamente utilizando **Hugging Face Spaces**, para superar las limitaciones de arquitectura y permisos de los entornos móviles (como Android/Termux).

## 🧠 Concepto: El Laboratorio Efímero

En lugar de intentar ejecutar el sistema completo en un entorno restringido, Núcleo utiliza un modelo de **despliegue híbrido**:
1.  **Control Plane (Móvil/Termux)**: Gestión de archivos, compilación de módulos y envío de órdenes.
2.  **Data Plane (Hugging Face Cloud)**: Ejecución del Kernel, gestión de memoria y runtime de agentes en un entorno Linux puro y escalable.

## 🛠️ Arquitectura de Despliegue

El despliegue se basa en un contenedor Docker que encapsula todo el stack necesario:

### 1. El Contenedor (Dockerfile)
El contenedor se encarga de:
*   Instalar el **Rust Toolchain** completo.
*   Compilar el binario de `nucleo-kernel` durante la fase de construcción.
*   Levantar un **Servidor de Control (FastAPI)** que actúa como puente de comunicación entre el móvil y la nube.

### 2. El Puente de Comunicación (Control Server)
El servidor `server.py` expone un endpoint `/exec` que permite inyectar comandos de shell directamente al kernel en ejecución, permitiendo operaciones críticas como el **Hot-Patching** de módulos WASM sin reiniciar el servicio.

## 🚀 Flujo de Despliegue (Paso a Paso)

### Paso 1: Preparación de Secretos
Asegúrate de tener tu token de Hugging Face en el archivo `.hf_token` siguiendo la guía de [Instalación](./installation.md).

### Paso 2: Ejecución del Script de Despliegue
Desde tu terminal de Termux, ejecuta el script de automatización:
```bash
bash deploy_nucleo.sh
```

**Lo que hace el script automáticamente:**
1.  Crea un Space privado en Hugging Face con el SDK Docker.
2.  Sube el `Dockerfile` y el `server.py`.
3.  Sube el código fuente del núcleo (excluyendo archivos temporales y de compilación).
4.  Inicia la construcción de la imagen en los servidores de HF.
5.  Monitorea el estado hasta que el Space esté en modo `RUNNING`.

### Paso 3: Validación y Pruebas
Una vez el Space esté listo, el script inyectará automáticamente:
1.  El inicio del proceso `nucleo-kernel`.
2.  Un comando de **Hot-Patching** para demostrar la capacidad de reemplazo dinámico de módulos.
3.  Una consulta de estado para verificar la integridad del sistema.

## 🛡️ Seguridad en el Despliegue

*   **Aislamiento**: El entorno de ejecución está contenido en un sandbox Docker.
*   **Autenticación**: Todas las peticiones al endpoint `/exec` requieren un token de autorización para prevenir ejecuciones no autorizadas.
*   **Privacidad**: El Space se crea en modo `private` por defecto para proteger la propiedad intelectual del proyecto.

---
*Diseñado para la máxima eficiencia en entornos móviles con capacidad de escala cloud.*

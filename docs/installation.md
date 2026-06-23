# Instalación y Configuración de Núcleo V2

Este documento detalla los pasos necesarios para desplegar y configurar el entorno de Núcleo V2 en sistemas compatibles (Android vía Termux, Linux, etc.).

## 🛡️ Gestión de Secretos y Seguridad (CRÍTICO)

Para evitar la exposición de credenciales en la terminal y en el historial de comandos, Núcleo utiliza un sistema de inyección de secretos mediante archivos protegidos.

### Configuración del Token de Hugging Face

Si vas a utilizar capacidades de despliegue en la nube (Hugging Face Spaces), **no** exportes el token directamente en tu shell. Sigue este procedimiento:

1. **Crea un archivo de secretos local**:
   ```bash
   nano /data/data/com.termux/files/home/workspace/.hf_token
   ```

2. **Pega únicamente tu token** (ejemplo: `hf_xxxxxxxxxxxxxxxxxxxxxxxxx`).

3. **Protege el archivo**:
   ```bash
   chmod 600 /data/data/com.termux/files/home/workspace/.hf_token
   ```

El sistema de despliegue de Núcleo leerá automáticamente este archivo para autenticar las peticiones de la API de Hugging Face.

## 🛠️ Requisitos del Sistema

*   **Rust Toolchain**: Versión 1.96.0 o superior (especificada en `rust-toolchain.toml`).
*   **Entorno**: Linux / Android (Termux).
*   **Dependencias del Sistema**:
    *   `build-essential` o `clang` (para compilación de crates nativos).
    *   `curl`, `git`, `wget` (para gestión de dependencias y red).
    *   `python3` y `pip` (para el servidor de control y herramientas de despliegue).

## 🚀 Procedimiento de Instalación Local

1. **Clonar el repositorio**:
   ```bash
   git clone <url-del-repo>
   cd nucleo
   ```

2. **Compilación inicial**:
   ```bash
   cargo build --release
   ```

3. **Ejecución del Kernel**:
   ```bash
   ./target/release/nucleo-kernel
   ```

---
*Última actualización: Junio 2026 - Integración de Gestión de Secretos y Despliegue Cloud.*

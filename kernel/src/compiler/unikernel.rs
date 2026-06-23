//! Compilador de unikernels.
//! Genera una imagen de unikernel a partir del workspace de un agente.

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct UnikernelCompiler {
    output_dir: PathBuf,
    target: String,
}

impl UnikernelCompiler {
    pub fn new(output_dir: PathBuf, target: &str) -> Self {
        Self {
            output_dir,
            target: target.to_string(),
        }
    }

    /// Compila el workspace de un agente en un unikernel.
    pub fn compile(&self, agent_id: u64, workspace_path: &Path) -> Result<PathBuf> {
        let output_path = self.output_dir.join(format!("unikernel_{}.img", agent_id));

        // Verificar que el workspace existe
        if !workspace_path.exists() {
            anyhow::bail!("Workspace {} no existe", workspace_path.display());
        }

        // En una implementación real, aquí se usaría un compilador de unikernels
        // (ej. MirageOS, Unikraft, OSv) para generar una imagen booteable.
        // Esta es una simulación para demostrar la integración.

        println!("🔨 Compilando unikernel para agente {}...", agent_id);
        println!("   Workspace: {}", workspace_path.display());
        println!("   Target: {}", self.target);

        // Simular la compilación: copiar algunos archivos y generar un archivo de imagen
        std::fs::create_dir_all(&output_path)?;

        // Crear un archivo de boot simple (simulado)
        let boot_file = output_path.join("boot.bin");
        let content = format!(
            "UNIKERNEL_AGENT_{} // Simulated boot image
Target: {}
Workspace: {}",
            agent_id,
            self.target,
            workspace_path.display()
        );
        fs::write(&boot_file, content)?;

        // En un entorno real, aquí se empaquetaría el workspace en una imagen booteable.

        println!("✅ Unikernel generado: {}", output_path.display());
        Ok(output_path)
    }

    /// Carga un unikernel en el kernel (simulado).
    pub fn load(&self, image_path: &Path) -> Result<()> {
        println!("📦 Cargando unikernel: {}", image_path.display());
        // En un entorno real, aquí se cargaría el unikernel en un hipervisor
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_unikernel_compile() {
        let dir = tempdir().unwrap();
        let compiler = UnikernelCompiler::new(dir.path().to_path_buf(), "x86_64");
        let agent_ws = dir.path().join("workspace_1");
        fs::create_dir(&agent_ws).unwrap();

        let result = compiler.compile(1, &agent_ws);
        assert!(result.is_ok());
        let image = result.unwrap();
        assert!(image.exists());
    }
}

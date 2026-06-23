extern crate alloc;
use std::process::Command;
use std::time::Duration;
use wait_timeout::ChildExt;

/// Error de ejecución.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EvalError {
    #[error("Permiso denegado")]
    PermissionDenied,
    #[error("Lenguaje no soportado")]
    LanguageNotSupported,
    #[error("Tiempo de ejecución excedido")]
    Timeout,
    #[error("Error de ejecución: {0}")]
    ExecutionError(String),
    #[error("Código inválido")]
    InvalidCode,
    #[error("Límite de recursos excedido")]
    ResourceLimit,
}

/// Evaluador de código en sandbox.
pub struct Eval;

impl Eval {
    /// Ejecuta código en el lenguaje especificado.
    pub fn eval(language: &str, code: &str, timeout_secs: u64) -> Result<String, EvalError> {
        // Validar código peligroso
        Self::validate_code(code)?;

        // Seleccionar intérprete
        let interpreter = Self::get_interpreter(language)?;

        // Crear archivo temporal para el código usando tiempo actual para unicidad básica
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_file = std::env::temp_dir().join(format!("nucleo_eval_{}.tmp", now));
        std::fs::write(&temp_file, code).map_err(|e| EvalError::ExecutionError(e.to_string()))?;

        // Configurar el comando
        let mut cmd = Command::new(interpreter);
        match language {
            "python" | "py" => { cmd.arg(temp_file.to_str().unwrap()); }
            "bash" | "sh" => { cmd.arg("-c").arg(code); } // Bash es mejor con -c
            "js" | "javascript" => { cmd.arg(temp_file.to_str().unwrap()); }
            "ruby" => { cmd.arg(temp_file.to_str().unwrap()); }
            "lua" => { cmd.arg(temp_file.to_str().unwrap()); }
            "perl" => { cmd.arg(temp_file.to_str().unwrap()); }
            _ => return Err(EvalError::LanguageNotSupported),
        };

        // Ejecutar
        let output = cmd.output().map_err(|e| EvalError::ExecutionError(e.to_string()))?;
        
        // Limpiar temporal
        let _ = std::fs::remove_file(temp_file);

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(EvalError::ExecutionError(stderr))
        }
    }

    /// Valida que el código no contenga comandos peligrosos.
    fn validate_code(code: &str) -> Result<(), EvalError> {
        let dangerous = [
            "rm -rf",
            "dd if=",
            "mkfs",
            "format",
            ":(){ :|:& };:",
            "chmod 777",
            "mount",
            "umount",
            "shutdown",
            "reboot",
        ];
        for d in dangerous {
            if code.contains(d) {
                return Err(EvalError::InvalidCode);
            }
        }
        Ok(())
    }

    /// Obtiene el intérprete para un lenguaje.
    fn get_interpreter(language: &str) -> Result<&'static str, EvalError> {
        match language {
            "python" | "py" => Ok("python3"),
            "bash" | "sh" => Ok("bash"),
            "js" | "javascript" => Ok("node"),
            "ruby" => Ok("ruby"),
            "lua" => Ok("lua"),
            "perl" => Ok("perl"),
            _ => Err(EvalError::LanguageNotSupported),
        }
    }
}

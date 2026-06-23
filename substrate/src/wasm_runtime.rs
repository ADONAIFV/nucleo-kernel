//! Runtime WASM multi-lenguaje para ejecutar agentes en un sandbox seguro.
//! Basado en WASMtime.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wasmtime::{Config, Engine, Instance, Linker, Module, Store};

/// Lenguajes soportados por el runtime WASM.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    // Añadir más lenguajes según se integren
}

/// Resultado de una ejecución WASM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmResult {
    pub output: Vec<u8>,
    pub exit_code: i32,
}

/// Errores del runtime WASM.
#[derive(Debug, thiserror::Error)]
pub enum WasmError {
    #[error("Error de compilación WASM: {0}")]
    Compile(String),
    #[error("Error de instanciación WASM: {0}")]
    Instantiate(String),
    #[error("Error de llamada a función WASM: {0}")]
    Call(String),
    #[error("Error de I/O WASM: {0}")]
    Io(String),
    #[error("Error general WASM: {0}")]
    Anyhow(#[from] anyhow::Error),
}

/// Runtime WASM.
pub struct WasmRuntime {
    engine: Engine,
    linker: Arc<Linker<()>>,
}

impl WasmRuntime {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.cranelift_debug_verifier(true);
        config.wasm_bulk_memory(true);
        config.wasm_multi_value(true);
        config.wasm_reference_types(true);

        let engine = Engine::new(&config)?;
        let linker = Arc::new(Linker::new(&engine));

        // Aquí se pueden añadir funciones host para las syscalls del kernel
        // linker.func_wrap("env", "log", |caller: Caller<()>, ptr: i32, len: i32| {
        //     // Implementar logging
        // })?;

        Ok(Self { engine, linker })
    }

    /// Compila un módulo WASM.
    pub fn compile(&self, wasm_bytes: &[u8]) -> Result<Module, WasmError> {
        Module::from_binary(&self.engine, wasm_bytes).map_err(|e| WasmError::Compile(e.to_string()))
    }

    /// Instancia y ejecuta un módulo WASM con una función de entrada específica.
    pub fn run_module(
        &self,
        module: &Module,
        entry_point: &str,
        _args: &[&[u8]],
    ) -> Result<WasmResult, WasmError> {
        let mut store = Store::new(&self.engine, ());
        let instance = self
            .linker
            .instantiate(&mut store, module)
            .map_err(|e| WasmError::Instantiate(e.to_string()))?;

        let func = instance
            .get_typed_func::<(), i32>(&mut store, entry_point)
            .map_err(|e| WasmError::Call(e.to_string()))?;

        let exit_code = func
            .call(&mut store, ())
            .map_err(|e| WasmError::Call(e.to_string()))?;

        Ok(WasmResult {
            output: Vec::new(), // En una implementación real, se capturaría stdout/stderr
            exit_code,
        })
    }

    /// Carga un módulo WASM y lo expone a través del trait Lego.
    pub fn load_lego_module(
        &self,
        wasm_bytes: &[u8],
        name: String,
        version: String,
    ) -> Result<Arc<dyn crate::lego::Lego>, WasmError> {
        let module = self.compile(wasm_bytes)?;
        // Aquí se crearía un adaptador para el trait Lego
        // Por ahora, solo devolveremos un error simulado.
        Err(WasmError::Anyhow(anyhow::anyhow!(
            "Implementación de Lego WASM pendiente"
        )))
    }
}

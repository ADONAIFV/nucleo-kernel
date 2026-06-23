//! Gestor de hot-patching para el kernel.
//! Permite reemplazar módulos en caliente.

use substrate::AgentId;
use substrate::lego::{Health, Lego, LegoRegistry};
// // use crate::security::HardwareGuard;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use wasmtime::{Engine, Instance, Linker, Module, Store};

pub struct HotPatchManager {
    engine: Engine,
    linker: Linker<()>,
    pub lego_registry: Arc<LegoRegistry>,
}
impl HotPatchManager {
    pub fn new(lego_registry: Arc<LegoRegistry>) -> Result<Self> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);

        // Añadir funciones del entorno al linker si es necesario para los módulos WASM
        // Por ejemplo, para acceder a syscalls del kernel, imprimir, etc.
        // linker.func_wrap("env", "log_message", |message: i32| { ... })?;

        Ok(Self {
            engine,
            linker,
            lego_registry,
        })
    }

    /// Reemplaza un módulo existente en el kernel por una nueva versión WASM.
    pub fn replace_module(
        &self,
        agent_id: AgentId,
        module_name: &str,
        new_version: &str,
        wasm_bytes: &[u8],
    ) -> Result<()> {
        // 1. Verificar que el módulo existe en el registro
        let lego_registry = self.lego_registry.as_ref();

        let old_module = lego_registry
            .get(module_name)
            .ok_or_else(|| anyhow::anyhow!("Módulo {} no encontrado", module_name))?;

        // 2. Validar que el nuevo WASM no contiene acceso a hardware
        // self.hardware_guard.validate_wasm(wasm_bytes)?;

        // 3. Compilar e instanciar el nuevo módulo WASM en un sandbox
        let module = Module::new(&self.engine, wasm_bytes)?;
        let mut store = Store::new(&self.engine, ());
        let instance = self.linker.instantiate(&mut store, &module)?;

        // 4. Verificar que el nuevo módulo implementa el trait Lego
        //    (tiene funciones exportadas: name, version, init, health, shutdown)
        let name_func = instance.get_typed_func::<(), i32>(&mut store, "name")?;
        let version_func = instance.get_typed_func::<(), i32>(&mut store, "version")?;
        let init_func = instance.get_typed_func::<(), i32>(&mut store, "init")?;
        let health_func = instance.get_typed_func::<(), i32>(&mut store, "health")?;
        let shutdown_func = instance.get_typed_func::<(), i32>(&mut store, "shutdown")?;

        // 5. Ejecutar init del nuevo módulo (en modo prueba)
        let init_result = init_func.call(&mut store, ())?;
        if init_result != 0 {
            return Err(anyhow::anyhow!(
                "Init del nuevo módulo falló con código {}",
                init_result
            ));
        }

        // 6. Hacer el swap atómico en el registro
        //    El nuevo módulo se envuelve en un adaptador que implementa Lego
        let new_lego = WasmLegoAdapter {
            name: module_name.to_string(),
            version: new_version.to_string(),
            instance: Arc::new(Mutex::new(Some((store, instance)))),
            module: Arc::new(module),
            wasm_bytes: wasm_bytes.to_vec(),
        };

        // Reemplazar en el registro (debe ser atómico)
        // Para simplicidad, usamos un RwLock para el registro
        // En una implementación real, se usaría un mecanismo de swap atómico
        lego_registry.replace(module_name, Arc::new(new_lego))?;

        // 7. Apagar el módulo antiguo
        if let Err(e) = old_module.shutdown() {
            tracing::warn!("Error al apagar módulo antiguo {}: {}", module_name, e);
            // No fallamos porque el reemplazo ya está hecho
        }

        tracing::info!(
            "🔄 Módulo {} reemplazado en caliente por versión {} (agente {:?})",
            module_name,
            new_version,
            agent_id
        );

        Ok(())
    }
}

/// Adaptador que envuelve un módulo WASM en el trait Lego.
struct WasmLegoAdapter {
    name: String,
    version: String,
    instance: Arc<Mutex<Option<(Store<()>, Instance)>>>,
    module: Arc<Module>,
    wasm_bytes: Vec<u8>,
}

impl Lego for WasmLegoAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn init(&mut self) -> Result<(), anyhow::Error> {
        let mut guard = self.instance.lock().unwrap();
        if let Some((store, instance)) = guard.as_mut() {
            let func = instance.get_typed_func::<(), i32>(&mut *store, "init")?;
            let result = func.call(&mut *store, ())?;
            if result == 0 {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Init falló con código {}", result))
            }
        } else {
            Err(anyhow::anyhow!("Instancia no disponible"))
        }
    }

    fn health(&self) -> Health {
        let mut guard = self.instance.lock().unwrap();
        if let Some((store, instance)) = guard.as_mut() {
            if let Ok(func) = instance.get_typed_func::<(), i32>(&mut *store, "health") {
                if let Ok(result) = func.call(&mut *store, ()) {
                    return match result {
                        0 => Health::Healthy,
                        1 => Health::Degraded,
                        _ => Health::Unhealthy,
                    };
                }
            }
        }
        Health::Unhealthy
    }

    fn shutdown(&self) -> Result<(), anyhow::Error> {
        let mut guard = self.instance.lock().unwrap();
        if let Some((store, instance)) = guard.as_mut() {
            if let Ok(func) = instance.get_typed_func::<(), i32>(&mut *store, "shutdown") {
                let _ = func.call(&mut *store, ());
            }
        }
        *guard = None;
        Ok(())
    }
}

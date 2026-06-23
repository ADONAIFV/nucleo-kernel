(module
  ;; Exportar funciones requeridas por Lego
  (func $name (result i32)
    (i32.const 0)  ;; puntero a string "my_module"
  )
  (func $version (result i32)
    (i32.const 1)  ;; puntero a string "2.0.0"
  )
  (func $init (result i32)
    (i32.const 0)  ;; 0 = éxito
  )
  (func $health (result i32)
    (i32.const 0)  ;; 0 = Healthy
  )
  (func $shutdown (result i32)
    (i32.const 0)
  )

  ;; Exportar funciones
  (export "name" (func $name))
  (export "version" (func $version))
  (export "init" (func $init))
  (export "health" (func $health))
  (export "shutdown" (func $shutdown))
)
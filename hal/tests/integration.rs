use hal::{DefaultHal, Hal};

#[test]
fn test_hal_initialization() {
    let mut hal = DefaultHal::new();
    let result = hal.init();
    assert!(result.is_ok(), "La HAL debe inicializarse correctamente");
}

#[test]
fn test_hal_cpu_info() {
    let hal = DefaultHal::new();
    let info = hal.cpu_info();
    assert!(info.cores >= 1, "Debe haber al menos 1 núcleo");
}

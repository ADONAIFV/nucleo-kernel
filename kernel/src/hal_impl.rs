//! Implementación de la HAL para la plataforma actual.

pub fn get_hal() -> Box<dyn hal::Hal> {
    Box::new(hal::DefaultHal::new())
}

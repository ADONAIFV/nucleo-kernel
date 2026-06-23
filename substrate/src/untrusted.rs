//! Primitiva de seguridad para datos no confiables.

pub struct Untrusted<T> {
    data: T,
}

impl<T> Untrusted<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
    pub fn into_inner(self) -> T {
        self.data
    }
    pub fn validate<F, E>(self, f: F) -> Result<Trusted<T>, E>
    where
        F: FnOnce(&T) -> Result<(), E>,
        T: Clone,
    {
        f(&self.data)?;
        Ok(Trusted::new(self.data))
    }
}

impl AsRef<str> for Untrusted<&str> {
    fn as_ref(&self) -> &str {
        self.data
    }
}

impl AsRef<[u8]> for Untrusted<&[u8]> {
    fn as_ref(&self) -> &[u8] {
        self.data
    }
}

pub struct Trusted<T> {
    data: T,
}

impl<T> Trusted<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
    pub fn into_inner(self) -> T {
        self.data
    }
}

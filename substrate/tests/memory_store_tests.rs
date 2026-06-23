//! Pruebas para MemoryStore (memoria persistente a largo plazo)

use substrate::memory_store::MemoryStore;
use tempfile::tempdir;

#[test]
fn test_memory_store_create() {
    let dir = tempdir().unwrap();
    let store = MemoryStore::new(1, dir.path()).unwrap();
    drop(store); // Asegurar que se suelte antes de chequear

    let index_path = dir.path().join("agent_1").join("memory.json");
    assert!(index_path.exists());
}

#[test]
fn test_memory_store_store_and_get() {
    let dir = tempdir().unwrap();
    let store = MemoryStore::new(1, dir.path()).unwrap();

    store.store("key1", "value1", "").unwrap();

    let value = store.get("key1").unwrap();
    assert_eq!(value, Some("value1".to_string()));
}

#[test]
fn test_memory_store_get_not_found() {
    let dir = tempdir().unwrap();
    let store = MemoryStore::new(1, dir.path()).unwrap();

    let value = store.get("nonexistent").unwrap();
    assert!(value.is_none());
}

#[test]
fn test_memory_store_persistence() {
    let dir = tempdir().unwrap();

    {
        let store = MemoryStore::new(1, dir.path()).unwrap();
        store
            .store("persistent_key", "persistent_value", "")
            .unwrap();
    }

    {
        let store = MemoryStore::new(1, dir.path()).unwrap();
        let value = store.get("persistent_key").unwrap();
        assert_eq!(value, Some("persistent_value".to_string()));
    }
}

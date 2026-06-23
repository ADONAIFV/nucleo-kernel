//! Pruebas para Workspace (espacio de trabajo aislado por agente)

use substrate::untrusted::Untrusted;
use substrate::workspace::{Workspace, WorkspaceError};
use tempfile::tempdir;

#[test]
fn test_workspace_create() {
    let dir = tempdir().unwrap();
    let ws = Workspace::new(42, dir.path()).unwrap();

    assert!(ws.root().exists());
    assert!(ws.root().is_dir());
    assert!(ws.root().ends_with("agent_42"));
}

#[test]
fn test_workspace_write_and_read() {
    let dir = tempdir().unwrap();
    let ws = Workspace::new(1, dir.path()).unwrap();

    ws.write(Untrusted::new("file.txt"), b"Hello, Nucleo!")
        .unwrap();

    let content = ws.read(Untrusted::new("file.txt")).unwrap();
    assert_eq!(content, b"Hello, Nucleo!");
}

#[test]
fn test_workspace_write_creates_directories() {
    let dir = tempdir().unwrap();
    let ws = Workspace::new(1, dir.path()).unwrap();

    ws.write(Untrusted::new("deep/nested/path/file.txt"), b"data")
        .unwrap();

    let content = ws
        .read(Untrusted::new("deep/nested/path/file.txt"))
        .unwrap();
    assert_eq!(content, b"data");
}

#[test]
fn test_workspace_list() {
    let dir = tempdir().unwrap();
    let ws = Workspace::new(1, dir.path()).unwrap();

    ws.write(Untrusted::new("a.txt"), b"").unwrap();
    ws.write(Untrusted::new("b.txt"), b"").unwrap();
    ws.write(Untrusted::new("subdir/c.txt"), b"").unwrap();

    let entries = ws.list(Untrusted::new(".")).unwrap();
    assert!(entries.contains(&"a.txt".to_string()));
    assert!(entries.contains(&"b.txt".to_string()));
    assert!(entries.contains(&"subdir".to_string()));
}

#[test]
fn test_workspace_list_nested() {
    let dir = tempdir().unwrap();
    let ws = Workspace::new(1, dir.path()).unwrap();

    ws.write(Untrusted::new("subdir/file1.txt"), b"").unwrap();
    ws.write(Untrusted::new("subdir/file2.txt"), b"").unwrap();

    let entries = ws.list(Untrusted::new("subdir")).unwrap();
    assert!(entries.contains(&"file1.txt".to_string()));
    assert!(entries.contains(&"file2.txt".to_string()));
}

#[test]
fn test_workspace_delete_file() {
    let dir = tempdir().unwrap();
    let ws = Workspace::new(1, dir.path()).unwrap();

    ws.write(Untrusted::new("test.txt"), b"data").unwrap();
    assert!(ws.exists(Untrusted::new("test.txt")).unwrap());

    ws.delete(Untrusted::new("test.txt")).unwrap();
    assert!(!ws.exists(Untrusted::new("test.txt")).unwrap());
}

#[test]
fn test_workspace_delete_directory() {
    let dir = tempdir().unwrap();
    let ws = Workspace::new(1, dir.path()).unwrap();

    ws.write(Untrusted::new("subdir/file.txt"), b"data")
        .unwrap();
    assert!(ws.exists(Untrusted::new("subdir")).unwrap());

    ws.delete(Untrusted::new("subdir")).unwrap();
    assert!(!ws.exists(Untrusted::new("subdir")).unwrap());
}

#[test]
fn test_workspace_delete_not_found() {
    let dir = tempdir().unwrap();
    let ws = Workspace::new(1, dir.path()).unwrap();

    let result = ws.delete(Untrusted::new("nonexistent.txt"));
    assert!(matches!(result, Err(WorkspaceError::NotFound)));
}

#[test]
fn test_workspace_exists() {
    let dir = tempdir().unwrap();
    let ws = Workspace::new(1, dir.path()).unwrap();

    assert!(!ws.exists(Untrusted::new("nonexistent.txt")).unwrap());

    ws.write(Untrusted::new("exists.txt"), b"").unwrap();
    assert!(ws.exists(Untrusted::new("exists.txt")).unwrap());
}

#[test]
fn test_workspace_escape_attempt() {
    let dir = tempdir().unwrap();
    let ws = Workspace::new(1, dir.path()).unwrap();

    // Probamos path traversal simple
    let result = ws.read(Untrusted::new("../../../etc/passwd"));
    assert!(matches!(result, Err(WorkspaceError::InvalidPath)));

    // Si el path traversal no es detectado por el check de ".." en resolve_path,
    // fallará por el check de "starts_with" en resolve_path
}

#[test]
fn test_workspace_invalid_paths() {
    let dir = tempdir().unwrap();
    let ws = Workspace::new(1, dir.path()).unwrap();

    let result = ws.read(Untrusted::new(""));
    assert!(matches!(result, Err(WorkspaceError::InvalidPath)));

    let result = ws.read(Untrusted::new(".."));
    assert!(matches!(result, Err(WorkspaceError::InvalidPath)));
}

#[test]
fn test_workspace_multiple_agents() {
    let dir = tempdir().unwrap();

    let ws1 = Workspace::new(1, dir.path()).unwrap();
    let ws2 = Workspace::new(2, dir.path()).unwrap();

    ws1.write(Untrusted::new("agent1.txt"), b"data from agent 1")
        .unwrap();
    ws2.write(Untrusted::new("agent2.txt"), b"data from agent 2")
        .unwrap();

    let content1 = ws1.read(Untrusted::new("agent1.txt")).unwrap();
    assert_eq!(content1, b"data from agent 1");

    let content2 = ws2.read(Untrusted::new("agent2.txt")).unwrap();
    assert_eq!(content2, b"data from agent 2");

    assert!(!ws1.exists(Untrusted::new("agent2.txt")).unwrap());
    assert!(!ws2.exists(Untrusted::new("agent1.txt")).unwrap());
}

#[test]
fn test_workspace_root() {
    let dir = tempdir().unwrap();
    let ws = Workspace::new(1, dir.path()).unwrap();

    assert_eq!(ws.root(), dir.path().join("agent_1"));
}

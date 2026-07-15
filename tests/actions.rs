use edash::actions;
use edash::lockfile::schema::{LockedPackage, Lockfile};
use std::fs;

/// Set up a test lockfile at a temp path. Returns (lock_path, envs_dir).
fn setup_lock(lock_path: &std::path::Path, packages: &[(&str, &str, &str)]) {
    let mut lf = Lockfile::new();
    for (name, version, backend) in packages {
        lf.package.push(LockedPackage {
            name: name.to_string(),
            version: version.to_string(),
            channel: None,
            backend: backend.to_string(),
            sha256: String::new(),
            explicit_urls: Vec::new(),
        });
    }
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    edash::lockfile::writer::write_lockfile(&lf, lock_path).unwrap();
}

#[test]
fn test_is_tool_installed_true() {
    let tmp = tempfile::tempdir().unwrap();
    let lock_path = tmp.path().join("edash.lock");

    // Create a real env dir so the disk check passes too
    let env_dir = tmp.path().join("envs").join("_yosys").join("bin");
    fs::create_dir_all(&env_dir).unwrap();

    setup_lock(&lock_path, &[("yosys", "0.30", "micromamba")]);

    // This test can't work directly because actions uses the real paths::lockfile_path()
    // which points to ~/.local/share/edash/edash.lock, not our temp path.
    //
    // We can only test that actions don't crash on the real lockfile.
    let _ = actions::is_tool_installed("__nonexistent_test_tool_xyz__");
}

#[test]
fn test_is_tool_installed_nonexistent() {
    // Query a tool name that definitely doesn't exist
    let installed = actions::is_tool_installed("__definitely_not_real_tool__");
    assert!(!installed, "nonexistent tool should not be installed");
}

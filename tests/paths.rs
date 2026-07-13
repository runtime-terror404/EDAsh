use edash::paths;

#[test]
fn test_data_dir_ends_with_edash() {
    let d = paths::data_dir();
    assert!(d.ends_with("edash"), "data_dir should end with 'edash', got: {}", d.display());
}

#[test]
fn test_config_dir_ends_with_edash() {
    let d = paths::config_dir();
    assert!(d.ends_with("edash"), "config_dir should end with 'edash', got: {}", d.display());
}

#[test]
fn test_envs_dir_under_data() {
    let d = paths::envs_dir();
    assert!(d.ends_with("envs"));
    assert!(d.to_string_lossy().contains("edash"));
}

#[test]
fn test_pdks_dir_under_data() {
    let d = paths::pdks_dir();
    assert!(d.ends_with("pdks"));
    assert!(d.to_string_lossy().contains("edash"));
}

#[test]
fn test_cache_dir_under_data() {
    let d = paths::cache_dir();
    assert!(d.ends_with("cache"));
    assert!(d.to_string_lossy().contains("edash"));
}

#[test]
fn test_bin_dir_under_data() {
    let d = paths::bin_dir();
    assert!(d.ends_with("bin"));
    assert!(d.to_string_lossy().contains("edash"));
}

#[test]
fn test_logs_dir_under_data() {
    let d = paths::logs_dir();
    assert!(d.ends_with("logs"));
    assert!(d.to_string_lossy().contains("edash"));
}

#[test]
fn test_lockfile_path() {
    let p = paths::lockfile_path();
    assert!(p.ends_with("edash.lock"));
    assert!(p.to_string_lossy().contains("edash"));
}

#[test]
fn test_catalog_base_dir() {
    let d = paths::catalog_base_dir();
    assert!(d.ends_with("base"));
    assert!(d.to_string_lossy().contains("catalog"));
}

#[test]
fn test_catalog_user_dir() {
    let d = paths::catalog_user_dir();
    assert!(d.ends_with("user"));
    assert!(d.to_string_lossy().contains("catalog"));
}

#[test]
fn test_downloads_dir() {
    let d = paths::downloads_dir();
    assert!(d.ends_with("downloads"));
}

#[test]
fn test_installation_yaml_path() {
    let p = paths::installation_yaml_path();
    assert!(p.ends_with("installation.yaml"));
    assert!(p.to_string_lossy().contains("edash"));
}

#[test]
fn test_data_and_config_dirs_differ() {
    assert_ne!(paths::data_dir(), paths::config_dir(),
        "data_dir and config_dir should be different");
}

#[test]
fn test_dirs_are_absolute() {
    let dirs: Vec<(&str, std::path::PathBuf)> = vec![
        ("data_dir", paths::data_dir()),
        ("config_dir", paths::config_dir()),
        ("envs_dir", paths::envs_dir()),
        ("pdks_dir", paths::pdks_dir()),
        ("cache_dir", paths::cache_dir()),
        ("catalog_base_dir", paths::catalog_base_dir()),
        ("catalog_user_dir", paths::catalog_user_dir()),
    ];
    for (name, dir) in &dirs {
        assert!(dir.is_absolute(), "{name} should be absolute: {}", dir.display());
    }
}

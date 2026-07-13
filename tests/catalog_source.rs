use edash::catalog::CatalogSource;
use std::path::PathBuf;

fn repo_catalog() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("catalog")
}

#[test]
fn test_path_list_pdk_names() {
    let source = CatalogSource::Path(repo_catalog());
    let names = source.list_pdk_names();
    assert!(names.contains(&"sky130".to_string()), "should contain sky130");
    assert!(names.contains(&"gf180".to_string()), "should contain gf180");
    assert!(names.contains(&"ihp-sg13g2".to_string()), "should contain ihp-sg13g2");
}

#[test]
fn test_path_read_pdk_config_sky130() {
    let source = CatalogSource::Path(repo_catalog());
    let yaml = source.read_pdk_config("sky130").expect("sky130 config should exist");
    assert!(yaml.contains("sky130"), "config should contain PDK name");
    assert!(yaml.contains("sky130A"), "config should contain variant");
    assert!(yaml.contains("spice_dir"), "config should contain paths");
}

#[test]
fn test_path_read_pdk_config_gf180() {
    let source = CatalogSource::Path(repo_catalog());
    let yaml = source.read_pdk_config("gf180").expect("gf180 config should exist");
    assert!(yaml.contains("gf180"), "config should contain PDK name");
}

#[test]
fn test_path_read_pdk_config_ihp() {
    let source = CatalogSource::Path(repo_catalog());
    let yaml = source.read_pdk_config("ihp-sg13g2").expect("ihp-sg13g2 config should exist");
    assert!(yaml.contains("ihp-sg13g2"), "config should contain PDK name");
}

#[test]
fn test_path_read_pdk_config_nonexistent() {
    let source = CatalogSource::Path(repo_catalog());
    let result = source.read_pdk_config("nonexistent");
    assert!(result.is_none(), "nonexistent PDK should return None");
}

#[test]
fn test_path_list_pdk_names_no_pdks_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let source = CatalogSource::Path(tmp.path().to_path_buf());
    let names = source.list_pdk_names();
    assert!(names.is_empty(), "empty dir should return empty list");
}

#[test]
fn test_path_read_pdk_config_no_pdks_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let source = CatalogSource::Path(tmp.path().to_path_buf());
    let result = source.read_pdk_config("sky130");
    assert!(result.is_none());
}

#[test]
fn test_catalog_source_clone() {
    let source = CatalogSource::Path(repo_catalog());
    let cloned = source.clone();
    // Both should work after clone
    assert!(cloned.read_pdk_config("sky130").is_some());
}

#[test]
fn test_catalog_source_default_variant() {
    // We can't set up XDG dirs easily in tests, but we can verify the variant exists
    let source = CatalogSource::Default;
    // Just verify it doesn't panic on basic operations
    let _ = source.list_pdk_names();
}

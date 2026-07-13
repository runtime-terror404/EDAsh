use edash::catalog::CatalogSource;
use edash::pdk::config::{load_pdk_config, resolve_pdk_vars};
use std::path::PathBuf;

fn repo_catalog() -> CatalogSource {
    CatalogSource::Path(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("catalog"))
}

#[test]
fn test_load_pdk_config_sky130() {
    let config = load_pdk_config("sky130", &repo_catalog()).expect("sky130 config should load");
    assert_eq!(config.name, "sky130");
    assert_eq!(config.variant, "sky130A");
    assert!(config.paths.contains_key("spice_dir"));
    assert!(config.paths.contains_key("netgen_setup"));
    assert!(config.paths.contains_key("magic_rcfile"));
    assert!(config.paths.contains_key("xschem_rcfile"));
    assert!(config.paths.contains_key("klayout_tech"));
}

#[test]
fn test_load_pdk_config_gf180() {
    let config = load_pdk_config("gf180", &repo_catalog()).expect("gf180 config should load");
    assert_eq!(config.name, "gf180");
    assert_eq!(config.variant, "gf180mcuD");
}

#[test]
fn test_load_pdk_config_ihp_sg13g2() {
    let config = load_pdk_config("ihp-sg13g2", &repo_catalog()).expect("ihp-sg13g2 config should load");
    assert_eq!(config.name, "ihp-sg13g2");
    assert_eq!(config.variant, "ihp-sg13g2");
}

#[test]
fn test_load_pdk_config_nonexistent() {
    let result = load_pdk_config("nonexistent", &repo_catalog());
    assert!(result.is_none(), "nonexistent PDK should return None");
}

#[test]
fn test_resolve_pdk_vars_empty() {
    let vars = resolve_pdk_vars(
        &[],
        &repo_catalog(),
        &PathBuf::from("/fake/pdk/root"),
    );
    assert!(vars.is_empty(), "no installed PDKs → no vars");
}

#[test]
fn test_resolve_pdk_vars_nonexistent_pdk() {
    let vars = resolve_pdk_vars(
        &["nonexistent".to_string()],
        &repo_catalog(),
        &PathBuf::from("/fake/pdk/root"),
    );
    assert!(vars.is_empty(), "nonexistent PDK should be silently skipped");
}

#[test]
fn test_resolve_pdk_vars_missing_paths() {
    // PDK config exists but the paths don't exist on disk → vars should be empty
    let vars = resolve_pdk_vars(
        &["sky130".to_string()],
        &repo_catalog(),
        &PathBuf::from("/fake/pdk/root"),
    );
    // No paths verified because /fake/pdk/root/sky130A/... doesn't exist
    assert!(vars.is_empty(), "missing paths → empty vars");
}

#[test]
fn test_load_pdk_config_all_three_pdks() {
    for name in &["sky130", "gf180", "ihp-sg13g2"] {
        let config = load_pdk_config(name, &repo_catalog())
            .unwrap_or_else(|| panic!("{name} config should load"));
        assert!(!config.name.is_empty());
        assert!(!config.variant.is_empty());
        assert!(!config.paths.is_empty(), "{name} should have paths");
    }
}

#[test]
fn test_config_variant_format() {
    // All variants should be non-empty ASCII strings
    let config = load_pdk_config("sky130", &repo_catalog()).unwrap();
    let variant = config.variant;
    assert!(!variant.is_empty());
    assert!(variant.chars().all(|c| c.is_ascii()));
}

#[test]
fn test_config_paths_are_relative() {
    let config = load_pdk_config("sky130", &repo_catalog()).unwrap();
    for (_key, rel_path) in &config.paths {
        assert!(
            !std::path::Path::new(rel_path).is_absolute(),
            "PDK path should be relative: {rel_path}"
        );
    }
}

#[test]
fn test_config_serde_roundtrip() {
    use edash::pdk::config::PdkToolConfig;
    use std::collections::HashMap;
    let mut paths = HashMap::new();
    paths.insert("spice_dir".to_string(), "libs.tech/ngspice".to_string());
    let config = PdkToolConfig {
        name: "test".to_string(),
        variant: "testA".to_string(),
        paths,
    };
    let yaml = serde_yaml::to_string(&config).unwrap();
    let parsed: PdkToolConfig = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(parsed.name, "test");
    assert_eq!(parsed.variant, "testA");
    assert_eq!(parsed.paths.get("spice_dir").unwrap(), "libs.tech/ngspice");
}

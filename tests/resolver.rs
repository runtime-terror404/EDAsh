use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use edash::catalog::index::{
    CatalogIndex, EnvironmentDef, PdkEntry, ResolvedItem, ToolEntry, ToolRegistry,
};
use edash::catalog::resolver::Resolver;

fn setup_test_catalog(tmp: &std::path::Path) -> PathBuf {
    let catalog_dir = tmp.join("catalog");
    fs::create_dir_all(&catalog_dir).unwrap();

    let tools: ToolRegistry = {
        let mut m = HashMap::new();
        m.insert(
            "yosys".to_string(),
            ToolEntry {
                backend: "micromamba".to_string(),
                channel: Some("litex-hub".to_string()),
                package: Some("yosys".to_string()),
                repo: None,
                requires: None,
                mpi: None,
            },
        );
        m.insert(
            "ngspice".to_string(),
            ToolEntry {
                backend: "micromamba".to_string(),
                channel: Some("conda-forge".to_string()),
                package: Some("ngspice".to_string()),
                repo: None,
                requires: None,
                mpi: None,
            },
        );
        m.insert(
            "nextpnr".to_string(),
            ToolEntry {
                backend: "oss-cad-suite".to_string(),
                channel: None,
                package: None,
                repo: None,
                requires: None,
                mpi: None,
            },
        );
        m
    };

    let tools_yaml = serde_yaml::to_string(&tools).unwrap();
    fs::write(catalog_dir.join("tools.yaml"), tools_yaml).unwrap();

    let mut envs = HashMap::new();
    envs.insert("digital".to_string(), "digital.yaml".to_string());

    let index = CatalogIndex {
        environments: envs,
        pdks: Some({
            let mut m = HashMap::new();
            m.insert(
                "sky130".to_string(),
                PdkEntry {
                    manager: "ciel".to_string(),
                    variant: Some("sky130A".to_string()),
                    build: None,
                },
            );
            m
        }),
    };

    let index_yaml = serde_yaml::to_string(&index).unwrap();
    fs::write(catalog_dir.join("index.yaml"), index_yaml).unwrap();

    let digital = EnvironmentDef {
        name: "digital".to_string(),
        tools: vec!["yosys".to_string(), "ngspice".to_string()],
    };
    let digital_yaml = serde_yaml::to_string(&digital).unwrap();
    fs::write(catalog_dir.join("digital.yaml"), digital_yaml).unwrap();

    catalog_dir
}

#[test]
fn test_resolve_environment() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog_dir = setup_test_catalog(tmp.path());

    let resolver = Resolver::load(&catalog_dir).unwrap();
    let items = resolver.resolve("digital").unwrap();

    assert_eq!(items.len(), 2);
    let pkg = |i: usize| match &items[i] {
        ResolvedItem::Tool(req) => req.clone(),
        _ => panic!("expected tool"),
    };
    assert_eq!(pkg(0).name, "yosys");
    assert_eq!(pkg(0).channel.as_deref(), Some("litex-hub"));
    assert_eq!(pkg(1).name, "ngspice");
    assert_eq!(pkg(1).channel.as_deref(), Some("conda-forge"));
}

#[test]
fn test_resolve_individual_tool() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog_dir = setup_test_catalog(tmp.path());

    let resolver = Resolver::load(&catalog_dir).unwrap();
    let items = resolver.resolve("nextpnr").unwrap();

    assert_eq!(items.len(), 1);
    if let ResolvedItem::Tool(req) = &items[0] {
        assert_eq!(req.name, "nextpnr");
    } else {
        panic!("expected tool");
    }
}

#[test]
fn test_resolve_unknown() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog_dir = setup_test_catalog(tmp.path());

    let resolver = Resolver::load(&catalog_dir).unwrap();
    let result = resolver.resolve("nonexistent");

    assert!(result.is_err());
}

#[test]
fn test_resolve_pdk() {
    let tmp = tempfile::tempdir().unwrap();
    let catalog_dir = setup_test_catalog(tmp.path());

    let resolver = Resolver::load(&catalog_dir).unwrap();
    let items = resolver.resolve("sky130").unwrap();

    assert_eq!(items.len(), 1);
    if let ResolvedItem::Pdk(pdk) = &items[0] {
        assert_eq!(pdk.name, "sky130");
        assert_eq!(pdk.manager, "ciel");
    } else {
        panic!("expected PDK");
    }
}

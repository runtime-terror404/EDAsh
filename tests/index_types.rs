use edash::catalog::index::{
    BackendKind, CatalogIndex, EnvironmentDef, PdkEntry, ToolEntry, ToolRegistry,
};
use std::collections::HashMap;

#[test]
fn test_backend_kind_from_str() {
    assert!(matches!(BackendKind::from_str("micromamba"), BackendKind::Micromamba));
    assert!(matches!(BackendKind::from_str("oss-cad-suite"), BackendKind::OssCadSuite));
    assert!(matches!(BackendKind::from_str("source"), BackendKind::Source));
}

#[test]
#[should_panic(expected = "Unknown backend")]
fn test_backend_kind_unknown_panics() {
    BackendKind::from_str("nonexistent");
}

#[test]
fn test_tool_entry_deserialize() {
    let yaml = r#"
backend: micromamba
channel: litex-hub
package: yosys
"#;
    let entry: ToolEntry = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(entry.backend, "micromamba");
    assert_eq!(entry.channel.as_deref(), Some("litex-hub"));
    assert_eq!(entry.package.as_deref(), Some("yosys"));
    assert!(entry.repo.is_none());
    assert!(entry.requires.is_none());
}

#[test]
fn test_tool_entry_minimal() {
    let yaml = "backend: oss-cad-suite";
    let entry: ToolEntry = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(entry.backend, "oss-cad-suite");
    assert!(entry.channel.is_none());
    assert!(entry.package.is_none());
}

#[test]
fn test_tool_entry_with_requires() {
    let yaml = r#"
backend: source
repo: https://github.com/example/tool
requires:
  - gcc
  - make
"#;
    let entry: ToolEntry = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(entry.repo.as_deref(), Some("https://github.com/example/tool"));
    assert_eq!(entry.requires.unwrap(), vec!["gcc", "make"]);
}

#[test]
fn test_environment_def_deserialize() {
    let yaml = r#"
name: digital
tools:
  - yosys
  - openroad
  - magic
"#;
    let env: EnvironmentDef = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(env.name, "digital");
    assert_eq!(env.tools, vec!["yosys", "openroad", "magic"]);
}

#[test]
fn test_pdk_entry_deserialize() {
    let yaml = r#"
manager: ciel
variant: sky130A
"#;
    let entry: PdkEntry = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(entry.manager, "ciel");
    assert_eq!(entry.variant.as_deref(), Some("sky130A"));
}

#[test]
fn test_pdk_entry_with_build() {
    let yaml = r#"
manager: source
variant: mypdk
build: ./configure && make
"#;
    let entry: PdkEntry = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(entry.build.as_deref(), Some("./configure && make"));
}

#[test]
fn test_catalog_index_deserialize() {
    let yaml = r#"
environments:
  digital: digital.yaml
  analog: analog.yaml
pdks:
  sky130:
    manager: ciel
    variant: sky130A
"#;
    let idx: CatalogIndex = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(idx.environments.len(), 2);
    assert_eq!(idx.environments.get("digital").unwrap(), "digital.yaml");
    assert!(idx.pdks.is_some());
    let pdks = idx.pdks.unwrap();
    assert_eq!(pdks.get("sky130").unwrap().manager, "ciel");
}

#[test]
fn test_catalog_index_without_pdks() {
    let yaml = r#"
environments:
  digital: digital.yaml
"#;
    let idx: CatalogIndex = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(idx.environments.len(), 1);
    assert!(idx.pdks.is_none());
}

#[test]
fn test_tool_registry_roundtrip() {
    let mut tools: ToolRegistry = HashMap::new();
    tools.insert(
        "yosys".into(),
        ToolEntry {
            backend: "micromamba".into(),
            channel: Some("litex-hub".into()),
            package: Some("yosys".into()),
            repo: None,
            requires: None,
            mpi: None,
        },
    );
    let yaml = serde_yaml::to_string(&tools).unwrap();
    let parsed: ToolRegistry = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(parsed.get("yosys").unwrap().backend, "micromamba");
}

#[test]
fn test_backend_kind_debug() {
    // BackendKind should implement Debug
    let k = BackendKind::Micromamba;
    assert!(!format!("{:?}", k).is_empty());
}

#[test]
fn test_resolved_item_tool_and_pdk() {
    use edash::catalog::index::{PackageRequest, PdkRequest, ResolvedItem};
    let tool = ResolvedItem::Tool(PackageRequest {
        name: "test".into(),
        backend: BackendKind::Micromamba,
        channel: None,
        package: None,
    });
    let pdk = ResolvedItem::Pdk(PdkRequest {
        name: "sky130".into(),
        manager: "ciel".into(),
        variant: Some("sky130A".into()),
    });
    assert!(matches!(tool, ResolvedItem::Tool(_)));
    assert!(matches!(pdk, ResolvedItem::Pdk(_)));
    assert!(!matches!(tool, ResolvedItem::Pdk(_)));
}

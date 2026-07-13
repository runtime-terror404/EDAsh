use edash::lockfile::schema::{Lockfile, LockedPackage, LockedPdk};
use std::path::Path;

#[test]
fn test_lockfile_new_defaults() {
    let lf = Lockfile::new();
    assert_eq!(lf.version, 1);
    assert!(lf.package.is_empty());
    assert!(lf.pdk.is_empty());
    assert!(!lf.generated.is_empty());
    assert!(lf.generated.contains("T"), "timestamp should be ISO format");
}

#[test]
fn test_lockfile_find_package_found() {
    let mut lf = Lockfile::new();
    lf.package.push(LockedPackage {
        name: "yosys".into(),
        version: "0.30".into(),
        channel: Some("litex-hub".into()),
        backend: "micromamba".into(),
        sha256: String::new(),
    });
    let pkg = lf.find_package("yosys");
    assert!(pkg.is_some());
    assert_eq!(pkg.unwrap().version, "0.30");
}

#[test]
fn test_lockfile_find_package_not_found() {
    let mut lf = Lockfile::new();
    lf.package.push(LockedPackage {
        name: "yosys".into(),
        version: "0.30".into(),
        channel: None,
        backend: "micromamba".into(),
        sha256: String::new(),
    });
    assert!(lf.find_package("magic").is_none());
}

#[test]
fn test_lockfile_find_package_empty() {
    let lf = Lockfile::new();
    assert!(lf.find_package("anything").is_none());
}

#[test]
fn test_write_and_read_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("edash.lock");

    let mut lf = Lockfile::new();
    lf.package.push(LockedPackage {
        name: "yosys".into(),
        version: "0.30".into(),
        channel: Some("litex-hub".into()),
        backend: "micromamba".into(),
        sha256: "abc123".into(),
    });
    lf.pdk.insert(
        "sky130".into(),
        LockedPdk {
            variant: "sky130A".into(),
            manager: "ciel".into(),
            git_ref: "abc123def".into(),
            sha256: String::new(),
        },
    );

    edash::lockfile::writer::write_lockfile(&lf, &path).unwrap();
    assert!(path.exists());

    let read = edash::lockfile::writer::read_lockfile(&path).unwrap();
    assert_eq!(read.package.len(), 1);
    assert_eq!(read.package[0].name, "yosys");
    assert_eq!(read.package[0].sha256, "abc123");
    assert_eq!(read.pdk.len(), 1);
    assert_eq!(read.pdk.get("sky130").unwrap().variant, "sky130A");
}

#[test]
fn test_read_nonexistent_file() {
    let result = edash::lockfile::writer::read_lockfile(Path::new("/tmp/nonexistent/edash.lock"));
    assert!(result.is_err());
}

#[test]
fn test_locked_package_serde() {
    let pkg = LockedPackage {
        name: "test".into(),
        version: "1.0".into(),
        channel: Some("conda-forge".into()),
        backend: "micromamba".into(),
        sha256: String::new(),
    };
    let toml_str = toml::to_string_pretty(&pkg).unwrap();
    assert!(toml_str.contains("test"));
    assert!(toml_str.contains("1.0"));
}

#[test]
fn test_locked_pdk_serde() {
    let pdk = LockedPdk {
        variant: "sky130A".into(),
        manager: "ciel".into(),
        git_ref: "abc123".into(),
        sha256: "def456".into(),
    };
    let toml_str = toml::to_string_pretty(&pdk).unwrap();
    let parsed: LockedPdk = toml::from_str(&toml_str).unwrap();
    assert_eq!(parsed.variant, "sky130A");
    assert_eq!(parsed.git_ref, "abc123");
}

#[test]
fn test_lockfile_multiple_packages() {
    let mut lf = Lockfile::new();
    for i in 0..5 {
        lf.package.push(LockedPackage {
            name: format!("tool{}", i),
            version: format!("1.{}", i),
            channel: None,
            backend: "micromamba".into(),
            sha256: String::new(),
        });
    }
    assert_eq!(lf.package.len(), 5);
    assert!(lf.find_package("tool0").is_some());
    assert!(lf.find_package("tool4").is_some());
    assert!(lf.find_package("tool5").is_none());
}

#[test]
fn test_lockfile_empty_pdk_section() {
    let lf = Lockfile::new();
    assert!(lf.pdk.is_empty());
    // Serialize and check pdk section is handled
    let toml_str = toml::to_string_pretty(&lf).unwrap();
    let parsed: Lockfile = toml::from_str(&toml_str).unwrap();
    assert!(parsed.pdk.is_empty());
    assert!(parsed.package.is_empty());
}

#[test]
fn test_package_channel_none() {
    let pkg = LockedPackage {
        name: "oss_tool".into(),
        version: "latest".into(),
        channel: None,
        backend: "oss-cad-suite".into(),
        sha256: String::new(),
    };
    let toml_str = toml::to_string_pretty(&pkg).unwrap();
    let parsed: LockedPackage = toml::from_str(&toml_str).unwrap();
    assert!(parsed.channel.is_none());
}

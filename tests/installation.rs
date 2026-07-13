use edash::installation::InstallationMeta;
use std::path::Path;

#[test]
fn test_new_meta() {
    let meta = InstallationMeta::new("user", "v1.0.0", "v1.0.0");
    assert_eq!(meta.install_method, "user");
    assert_eq!(meta.binary_version, "v1.0.0");
    assert_eq!(meta.catalog_version, "v1.0.0");
    assert_eq!(meta.installer_version, 3);
    assert!(!meta.installed_at.is_empty());
    // Should be ISO 8601-ish
    assert!(meta.installed_at.contains("T"));
}

#[test]
fn test_meta_system_install() {
    let meta = InstallationMeta::new("system", "v2.0.0", "v2.0.0");
    assert_eq!(meta.install_method, "system");
    assert_eq!(meta.binary_version, "v2.0.0");
}

#[test]
fn test_write_and_read_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("installation.yaml");

    let meta = InstallationMeta::new("user", "v0.1.0", "v0.1.0");
    edash::installation::write_installation(&path, &meta).unwrap();
    assert!(path.exists(), "installation.yaml should exist after write");

    let read = edash::installation::read_installation(&path);
    assert!(read.is_some(), "should read back installation.yaml");
    let read = read.unwrap();
    assert_eq!(read.install_method, "user");
    assert_eq!(read.binary_version, "v0.1.0");
    assert_eq!(read.catalog_version, "v0.1.0");
    assert_eq!(read.installer_version, 3);
}

#[test]
fn test_read_nonexistent() {
    let result = edash::installation::read_installation(Path::new("/tmp/nonexistent/installation.yaml"));
    assert!(result.is_none(), "nonexistent file should return None");
}

#[test]
fn test_write_overwrites() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("installation.yaml");

    let meta1 = InstallationMeta::new("user", "v1.0.0", "v1.0.0");
    edash::installation::write_installation(&path, &meta1).unwrap();

    let meta2 = InstallationMeta::new("system", "v2.0.0", "v2.0.0");
    edash::installation::write_installation(&path, &meta2).unwrap();

    let read = edash::installation::read_installation(&path).unwrap();
    assert_eq!(read.install_method, "system", "should be overwritten");
    assert_eq!(read.binary_version, "v2.0.0");
}

#[test]
fn test_serde_roundtrip() {
    let meta = InstallationMeta::new("user", "v0.1.0", "v0.1.0");
    let yaml = serde_yaml::to_string(&meta).unwrap();
    let parsed: InstallationMeta = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(parsed.install_method, meta.install_method);
    assert_eq!(parsed.binary_version, meta.binary_version);
    assert_eq!(parsed.catalog_version, meta.catalog_version);
}

#[test]
fn test_different_versions() {
    let meta = InstallationMeta::new("user", "v1.5.0", "v1.4.0");
    assert_ne!(meta.binary_version, meta.catalog_version);
    // Both should still be valid
    let yaml = serde_yaml::to_string(&meta).unwrap();
    let parsed: InstallationMeta = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(parsed.binary_version, "v1.5.0");
    assert_eq!(parsed.catalog_version, "v1.4.0");
}

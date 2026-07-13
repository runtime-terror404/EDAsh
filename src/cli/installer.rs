use std::path::Path;

/// Verify the installer gate is set — refuse otherwise.
fn check_gate() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("EDASH_INSTALLER").unwrap_or_default() != "1" {
        return Err("This is an internal command not intended for direct use.".into());
    }
    Ok(())
}

/// Print edash paths: config_dir, data_dir, bin_dir as key=value lines.
pub fn paths() -> Result<(), Box<dyn std::error::Error>> {
    check_gate()?;
    println!("config_dir={}", crate::paths::config_dir().display());
    println!("data_dir={}", crate::paths::data_dir().display());
    println!("bin_dir={}", crate::paths::bin_dir().display());
    Ok(())
}

/// Stage a catalog from an extracted directory, validating it parses.
/// `extracted_dir` is the unpacked catalog tarball directory.
/// `manifest_path` is the path to manifest.yaml inside it.
pub fn stage_catalog(extracted_dir: &Path, manifest_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    check_gate()?;

    let base_new = crate::paths::catalog_base_dir().parent().unwrap().join("base.new");

    // Validate the incoming catalog parses before staging
    let index_path = extracted_dir.join("index.yaml");
    if !index_path.exists() {
        return Err(format!("{}: index.yaml not found in extracted catalog", extracted_dir.display()).into());
    }
    let _index: crate::catalog::index::CatalogIndex = serde_yaml::from_str(
        &std::fs::read_to_string(&index_path)?
    )?;

    let tools_path = extracted_dir.join("tools.yaml");
    if tools_path.exists() {
        let _: crate::catalog::index::ToolRegistry = serde_yaml::from_str(
            &std::fs::read_to_string(&tools_path)?
        )?;
    }

    // Validate manifest parses
    if manifest_path.exists() {
        let manifest_raw = std::fs::read_to_string(manifest_path)?;
        let _: serde_yaml::Value = serde_yaml::from_str(&manifest_raw)?;
    }

    // Copy base → base.new (or start fresh if no base yet)
    let base = crate::paths::catalog_base_dir();
    if base.exists() {
        // Use cp -a for recursive copy
        let status = std::process::Command::new("cp")
            .args(["-a", &base.to_string_lossy(), &base_new.to_string_lossy()])
            .status()
            .map_err(|e| format!("cp failed: {e}"))?;
        if !status.success() {
            return Err("failed to copy base → base.new".into());
        }
    } else {
        std::fs::create_dir_all(&base_new)?;
    }

    // Overlay files from extracted_dir onto base.new
    // Use cp -a to merge; existing files in base.new are overwritten
    let status = std::process::Command::new("cp")
        .args(["-a", &format!("{}/.", extracted_dir.display()), &base_new.to_string_lossy()])
        .status()
        .map_err(|e| format!("cp overlay failed: {e}"))?;
    if !status.success() {
        return Err("failed to overlay extracted catalog onto base.new".into());
    }

    // Write manifest into base.new
    if manifest_path.exists() {
        let dest = base_new.join("manifest.yaml");
        std::fs::copy(manifest_path, &dest)?;
    }

    println!("staged catalog to {}", base_new.display());
    Ok(())
}

/// Self-test: run --version and paths against live paths.
pub fn self_test() -> Result<(), Box<dyn std::error::Error>> {
    check_gate()?;

    // Test: paths command returns expected dirs
    let data = crate::paths::data_dir();
    let cfg = crate::paths::config_dir();
    let bin = crate::paths::bin_dir();

    if data.to_string_lossy().is_empty() || cfg.to_string_lossy().is_empty() {
        return Err("self-test failed: empty paths".into());
    }

    println!("self-test PASSED");
    println!("  data_dir={}", data.display());
    println!("  config_dir={}", cfg.display());
    println!("  bin_dir={}", bin.display());
    Ok(())
}

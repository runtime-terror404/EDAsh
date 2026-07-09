use crate::paths;
use std::path::PathBuf;

pub fn list(lock_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !lock_path.exists() {
        println!("No packages installed (edash.lock not found)");
        return Ok(());
    }

    let lockfile = crate::lockfile::writer::read_lockfile(lock_path)?;

    if lockfile.package.is_empty() && lockfile.pdk.is_empty() {
        println!("No packages installed");
        return Ok(());
    }

    if !lockfile.package.is_empty() {
        let envs_dir = paths::envs_dir();

        let mut name_width = 0;
        let mut version_width = 0;
        for pkg in &lockfile.package {
            name_width = name_width.max(pkg.name.len());
            version_width = version_width.max(pkg.version.len());
        }

        println!("Installed packages:");
        for pkg in &lockfile.package {
            let present = if pkg.backend == "oss-cad-suite" {
                envs_dir.join("oss-cad-suite").join("environment").exists()
            } else {
                envs_dir.join(format!("_{}", pkg.name)).exists()
            };
            let status = if present {
                if pkg.sha256.is_empty() { "✓" } else { "✓ verified" }
            } else {
                "✗ missing"
            };
            let channel = pkg
                .channel
                .as_deref()
                .map(|c| format!("{}::{}", pkg.backend, c))
                .unwrap_or_else(|| pkg.backend.clone());

            println!(
                "  {:name_width$}  {:version_width$}  {channel}  {status}",
                pkg.name,
                pkg.version,
                name_width = name_width,
                version_width = version_width,
            );
        }
    }

    if !lockfile.pdk.is_empty() {
        println!("PDKs:");
        for (name, pdk) in &lockfile.pdk {
            println!("  {}  {}  ref:{}", name, pdk.variant, pdk.git_ref);
        }
    }

    Ok(())
}

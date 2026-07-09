use serde_json::Value;
use std::path::PathBuf;

pub fn outdated(lock_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !lock_path.exists() {
        println!("No lockfile found. Run 'edash install' first.");
        return Ok(());
    }

    let lockfile = crate::lockfile::writer::read_lockfile(lock_path)?;

    if lockfile.package.is_empty() {
        println!("No packages installed");
        return Ok(());
    }

    println!("Checking for updates...\n");

    let mut found = false;
    for pkg in &lockfile.package {
        if pkg.backend == "oss-cad-suite" {
            continue; // oss-cad-suite is a moving target
        }

        // Query micromamba for latest version
        let channel = pkg.channel.as_deref().unwrap_or("conda-forge");
        let output = std::process::Command::new("micromamba")
            .args(["search", "-c", channel, &pkg.name, "--json"])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                if let Ok(parsed) = serde_json::from_str::<Vec<Value>>(&stdout) {
                    if let Some(latest) = parsed.first() {
                        if let Some(latest_ver) = latest.get("version").and_then(|v| v.as_str()) {
                            if latest_ver != pkg.version {
                                println!(
                                    "  {}  {} → {}",
                                    pkg.name, pkg.version, latest_ver
                                );
                                found = true;
                            }
                        }
                    }
                }
            }
            _ => {} // Skip tools where micromamba search fails
        }
    }

    if !found {
        println!("  all packages up to date");
    }

    Ok(())
}

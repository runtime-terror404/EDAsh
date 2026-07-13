use crate::installation::InstallationMeta;
use std::path::PathBuf;
use std::process::Command;

const GITHUB_API: &str = "https://api.github.com/repos/runtime-terror404/edash/releases/latest";

/// Fetch the latest release tag from GitHub. Returns (tag_name, created_at).
fn latest_release() -> Result<(String, String), Box<dyn std::error::Error>> {
    let output = Command::new("curl")
        .args(["-sL", GITHUB_API])
        .output()
        .map_err(|e| format!("curl failed: {e}"))?;

    if !output.status.success() {
        return Err("failed to fetch latest release info".into());
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("api parse error: {e}"))?;

    let tag = json["tag_name"]
        .as_str()
        .ok_or("no tag_name in release API")?
        .to_string();

    let created_at = json["created_at"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    Ok((tag, created_at))
}

pub fn update() -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = crate::paths::data_dir();
    let bin_dir = crate::paths::bin_dir();
    let downloads_dir = crate::paths::downloads_dir();
    let catalog_base = crate::paths::catalog_base_dir();

    // 1. Fetch latest release info
    let (tag, _created_at) = latest_release()?;
    println!("Latest release: {tag}");

    // 2. Check if already up to date
    let inst_path = crate::paths::installation_yaml_path();
    if inst_path.exists() {
        if let Some(meta) = crate::installation::read_installation(&inst_path) {
            if meta.binary_version == tag {
                println!("Already up to date ({tag}).");
                return Ok(());
            }
        }
    }

    // 3. Prepare download URLs
    let bin_url = format!(
        "https://github.com/runtime-terror404/edash/releases/download/{tag}/edash-x86_64-unknown-linux-gnu"
    );
    let catalog_url = format!(
        "https://github.com/runtime-terror404/edash/releases/download/{tag}/catalog.tar.gz"
    );
    let manifest_url = format!(
        "https://github.com/runtime-terror404/edash/releases/download/{tag}/manifest.yaml"
    );

    std::fs::create_dir_all(&downloads_dir)?;
    let dl_bin = downloads_dir.join("edash.new");
    let dl_catalog = downloads_dir.join("catalog.tar.gz");
    let dl_manifest = downloads_dir.join("manifest.yaml");

    // 4. Download files
    println!("Downloading...");
    download(&bin_url, &dl_bin)?;
    download(&catalog_url, &dl_catalog)?;
    // manifest is optional; don't fail if it doesn't exist
    let _ = download(&manifest_url, &dl_manifest);

    // 5. Sanity-check candidate binary
    let bin_ok = Command::new(&dl_bin)
        .args(["--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !bin_ok {
        // Try with chmod +x
        let _ = Command::new("chmod").args(["+x", &dl_bin.to_string_lossy()]).status();
        let bin_ok = Command::new(&dl_bin)
            .args(["--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !bin_ok {
            return Err("downloaded binary failed --version sanity check".into());
        }
    }

    // 6. Extract catalog to staging
    let staging_dir = downloads_dir.join("catalog-staging");
    if staging_dir.exists() {
        std::fs::remove_dir_all(&staging_dir)?;
    }
    std::fs::create_dir_all(&staging_dir)?;

    let tar_ok = Command::new("tar")
        .args(["-xzf", &dl_catalog.to_string_lossy(), "-C", &staging_dir.to_string_lossy()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !tar_ok {
        return Err("failed to extract catalog tarball".into());
    }

    // 7. Stage catalog using the hidden subcommand
    let staged_manifest = staging_dir.join("manifest.yaml");
    let status = Command::new(std::env::current_exe()?)
        .env("EDASH_INSTALLER", "1")
        .args(["__internal", "stage-catalog", &staging_dir.to_string_lossy(), &staged_manifest.to_string_lossy()])
        .status()
        .map_err(|e| format!("stage-catalog failed: {e}"))?;

    if !status.success() {
        return Err("catalog staging failed".into());
    }

    // 8. Atomic catalog swap
    let base_new = data_dir.join("catalog").join("base.new");
    let base_old = data_dir.join("catalog").join("base.old");

    if !base_new.exists() {
        return Err("staging directory not found after stage-catalog".into());
    }

    if catalog_base.exists() {
        if base_old.exists() {
            std::fs::remove_dir_all(&base_old)?;
        }
        std::fs::rename(&catalog_base, &base_old)?;
    }
    std::fs::rename(&base_new, &catalog_base)?;

    // 9. Atomic binary swap
    let bin_path = bin_dir.join("edash");
    let bin_old = bin_dir.join("edash.old");

    std::fs::create_dir_all(&bin_dir)?;
    if bin_path.exists() {
        if bin_old.exists() {
            std::fs::remove_file(&bin_old)?;
        }
        std::fs::rename(&bin_path, &bin_old)?;
    }
    std::fs::copy(&dl_bin, &bin_path)?;

    // Make executable
    let _ = Command::new("chmod").args(["+x", &bin_path.to_string_lossy()]).status();

    // 10. Self-test
    let test_ok = Command::new(&bin_path)
        .env("EDASH_INSTALLER", "1")
        .args(["__internal", "self-test"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if test_ok {
        // Success — cleanup
        if base_old.exists() {
            let _ = std::fs::remove_dir_all(&base_old);
        }
        if bin_old.exists() {
            let _ = std::fs::remove_file(&bin_old);
        }

        // Write installation metadata
        let meta = InstallationMeta::new("user", &tag, &tag);
        crate::installation::write_installation(&inst_path, &meta)?;

        println!("Updated to {tag}");
    } else {
        // Rollback binary
        if bin_old.exists() {
            let _ = std::fs::remove_file(&bin_path);
            let _ = std::fs::rename(&bin_old, &bin_path);
        }
        // Keep new catalog (it was validated during staging)
        eprintln!("Self-test failed — binary rolled back, catalog kept");
        return Err("self-test failed".into());
    }

    // Cleanup downloads
    let _ = std::fs::remove_file(&dl_bin);
    let _ = std::fs::remove_file(&dl_catalog);
    let _ = std::fs::remove_file(&dl_manifest);
    let _ = std::fs::remove_dir_all(&staging_dir);

    Ok(())
}

fn download(url: &str, dest: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("curl")
        .args(["-sSL", "-o", &dest.to_string_lossy(), url])
        .status()
        .map_err(|e| format!("curl {url}: {e}"))?;

    if !status.success() {
        return Err(format!("download failed: {url}").into());
    }
    Ok(())
}

use crate::lockfile::schema::LockedPdk;
use crate::paths;
use std::process::Command;

pub fn resolve_and_install(
    family: &str,
    variant: &Option<String>,
) -> Result<LockedPdk, Box<dyn std::error::Error>> {
    let pdk_root = paths::pdks_dir();
    let variant_str = variant.as_deref().unwrap_or(family);
    let family = pdk_family(family);

    // Get latest version
    let output = Command::new("ciel")
        .args([
            "ls-remote",
            "--pdk-family",
            family,
        ])
        .output()
        .map_err(|e| format!("ciel: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let latest = stdout
        .lines()
        .next()
        .ok_or("no remote versions found for PDK")?;

    // Fetch
    let output = Command::new("ciel")
        .args([
            "fetch",
            "--pdk-family",
            family,
            "--pdk-root",
            &pdk_root.to_string_lossy(),
            latest,
        ])
        .output()
        .map_err(|e| format!("ciel fetch: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ciel fetch failed: {}", stderr).into());
    }

    // Enable
    let output = Command::new("ciel")
        .args([
            "enable",
            "--pdk-family",
            family,
            "--pdk-root",
            &pdk_root.to_string_lossy(),
            latest,
        ])
        .output()
        .map_err(|e| format!("ciel enable: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ciel enable failed: {}", stderr).into());
    }

    Ok(LockedPdk {
        variant: variant_str.to_string(),
        manager: "ciel".to_string(),
        git_ref: latest.to_string(),
        sha256: String::new(),
    })
}

fn pdk_family(name: &str) -> &str {
    match name {
        "sky130" => "sky130",
        "gf180" => "gf180mcu",
        "ihp-sg13g2" => "ihp-sg13g2",
        _ => name,
    }
}

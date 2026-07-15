use crate::backend::micromamba::MicromambaBackend;
use crate::backend::oss_cad_suite::OssCadSuiteBackend;
use crate::catalog::index::{BackendKind, PackageRequest};
use crate::catalog::CatalogSource;
use crate::lockfile::schema::{LockedPackage, LockedPdk, Lockfile};
use crate::paths;
use std::process::Command;

/// Install a single tool. Idempotent — skips if already in lockfile and on disk.
pub fn install_tool(req: &PackageRequest, source: &CatalogSource) -> Result<LockedPackage, Box<dyn std::error::Error>> {
    let lock_path = paths::lockfile_path();
    let mut lockfile = read_lockfile(&lock_path);

    // Idempotency: skip if already installed
    if let Some(existing) = lockfile.package.iter().find(|p| p.name == req.name) {
        let pkg_dir = paths::envs_dir().join(format!("_{}", req.name));
        if pkg_dir.exists() {
            return Ok(existing.clone());
        }
        lockfile.package.retain(|p| p.name != req.name);
    }

    // Enrich request with pre-computed explicit URLs from catalog locks
    let mut enriched = req.clone();
    if let Some(urls) = source.read_tool_lock(&req.name) {
        enriched.explicit_urls = urls;
    }

    let (ptx, _prx) = tokio::sync::mpsc::unbounded_channel();

    let pkg = match enriched.backend {
        BackendKind::OssCadSuite => {
            let backend = OssCadSuiteBackend::new();
            backend.install_package(&enriched, ptx)?
        }
        _ => {
            let backend = MicromambaBackend::new();
            if !backend.is_available() {
                return Err("micromamba not found. Install: curl -L micro.mamba.pm/install.sh | bash".into());
            }
            backend.install_package(&enriched, ptx)?
        }
    };

    lockfile.package.retain(|p| p.name != pkg.name);
    lockfile.package.push(pkg.clone());
    write_lockfile(&lockfile, &lock_path)?;

    Ok(pkg)
}

/// Install a PDK via ciel. Idempotent — skips if already in lockfile.
pub fn install_pdk(name: &str) -> Result<LockedPdk, Box<dyn std::error::Error>> {
    let lock_path = paths::lockfile_path();
    let mut lockfile = read_lockfile(&lock_path);

    if lockfile.pdk.contains_key(name) {
        return Err(format!("PDK '{}' is already installed", name).into());
    }

    let pdk = crate::pdk::ciel::resolve_and_install(name, &None)?;

    // Re-read lockfile to pick up changes from other threads
    lockfile = read_lockfile(&lock_path);
    lockfile.pdk.insert(name.to_string(), pdk.clone());
    write_lockfile(&lockfile, &lock_path)?;

    Ok(pdk)
}

/// Remove a tool from disk and lockfile.
pub fn remove_tool(name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let lock_path = paths::lockfile_path();
    let mut lockfile = read_lockfile(&lock_path);

    let existed = lockfile.package.iter().any(|p| p.name == name);
    if existed {
        lockfile.package.retain(|p| p.name != name);
        write_lockfile(&lockfile, &lock_path)?;
    }

    let pkg_dir = paths::envs_dir().join(format!("_{}", name));
    if pkg_dir.exists() {
        let _ = Command::new("chmod").args(["-R", "u+w", &pkg_dir.to_string_lossy()]).status();
        std::fs::remove_dir_all(&pkg_dir)?;
    }

    // If this was the last oss-cad-suite tool, remove the shared dir too
    let lock_path = paths::lockfile_path();
    let lf = read_lockfile(&lock_path);
    let has_oss = lf.package.iter().any(|p| p.backend == "oss-cad-suite");
    if !has_oss {
        let oss_dir = paths::envs_dir().join("oss-cad-suite");
        if oss_dir.exists() {
            let _ = Command::new("chmod").args(["-R", "u+w", &oss_dir.to_string_lossy()]).status();
            let _ = std::fs::remove_dir_all(&oss_dir);
        }
    }

    Ok(existed)
}

/// Remove a PDK from disk and lockfile.
pub fn remove_pdk(name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let lock_path = paths::lockfile_path();
    let mut lockfile = read_lockfile(&lock_path);

    let existed = lockfile.pdk.remove(name).is_some();
    if existed {
        write_lockfile(&lockfile, &lock_path)?;
    }

    // Remove this PDK's data directory and symlinks
    let family = pdk_family_for_remove(name);
    let pdks_root = paths::pdks_dir();
    let data_dir = pdks_root.join("ciel").join(family);
    if data_dir.exists() {
        let _ = Command::new("chmod").args(["-R", "u+w", &data_dir.to_string_lossy()]).status();
        std::fs::remove_dir_all(&data_dir)?;
    }
    // Ciel creates symlinks at pdks/ root for enabled versions (e.g. pdks/sky130A)
    if let Ok(entries) = std::fs::read_dir(&pdks_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_symlink() {
                let target = std::fs::read_link(&path).unwrap_or_default();
                if target.to_string_lossy().contains(family) {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }

    Ok(existed)
}

fn pdk_family_for_remove(name: &str) -> &str {
    match name {
        "sky130" => "sky130",
        "gf180" => "gf180mcu",
        "ihp-sg13g2" => "ihp-sg13g2",
        _ => name,
    }
}

/// Remove all PDKs from disk and lockfile. Returns count removed.
pub fn remove_all_pdks() -> Result<usize, Box<dyn std::error::Error>> {
    let lock_path = paths::lockfile_path();
    let lockfile = read_lockfile(&lock_path);
    let names: Vec<String> = lockfile.pdk.keys().cloned().collect();
    let count = names.len();
    for name in &names {
        remove_pdk(name)?;
    }
    Ok(count)
}

/// Remove all exclusive tools in an environment. Shared tools stay.
/// Returns (removed_count, skipped_count).
pub fn remove_env(
    _env_name: &str,
    tool_names: &[String],
    shared_check: impl Fn(&str) -> bool,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let mut removed = 0;
    let mut skipped = 0;

    for t in tool_names {
        if shared_check(t) {
            // Only count as skipped if actually installed
            let lock_path = paths::lockfile_path();
            let lf = read_lockfile(&lock_path);
            let installed = lf.package.iter().any(|p| p.name == *t)
                && paths::envs_dir().join(format!("_{}", t)).exists();
            if installed {
                skipped += 1;
            }
            continue;
        }
        remove_tool(t)?;
        removed += 1;
    }

    Ok((removed, skipped))
}

/// Check if a tool is installed (in lockfile AND on disk).
pub fn is_tool_installed(name: &str) -> bool {
    let lock_path = paths::lockfile_path();
    let lf = read_lockfile(&lock_path);
    let pkg = lf.package.iter().find(|p| p.name == name);
    let Some(pkg) = pkg else { return false };
    if pkg.backend == "oss-cad-suite" {
        paths::envs_dir().join("oss-cad-suite").join("bin").join(name).exists()
    } else {
        paths::envs_dir().join(format!("_{}", name)).exists()
    }
}

// ── helpers ──

fn read_lockfile(path: &std::path::Path) -> Lockfile {
    if path.exists() {
        crate::lockfile::writer::read_lockfile(path).unwrap_or_else(|_| Lockfile::new())
    } else {
        Lockfile::new()
    }
}

fn write_lockfile(
    lockfile: &Lockfile,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::lockfile::writer::write_lockfile(lockfile, path)
}

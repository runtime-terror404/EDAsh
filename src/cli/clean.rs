use crate::paths;
use std::path::PathBuf;

pub fn clean(
    lock_path: &PathBuf,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let envs_dir = paths::envs_dir();

    // Read lockfile to know what's referenced
    let referenced: Vec<String> = if lock_path.exists() {
        let lockfile = crate::lockfile::writer::read_lockfile(lock_path)?;
        let mut names: Vec<String> = lockfile.package.iter().map(|p| format!("_{}", p.name)).collect();
        if lockfile.package.iter().any(|p| p.backend == "oss-cad-suite") {
            names.push("oss-cad-suite".to_string());
        }
        names
    } else {
        vec![]
    };

    if !envs_dir.exists() {
        println!("Nothing to clean");
        return Ok(());
    }

    let mut removed = 0;
    for entry in std::fs::read_dir(&envs_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('_') || name == "oss-cad-suite" {
            if !referenced.contains(&name) {
                let path = entry.path();
                if dry_run {
                    println!("  would remove {}", path.display());
                } else {
                    // Conda files are often read-only
                    if let Err(e) = std::fs::remove_dir_all(&path) {
                        eprintln!("  ✗ {} — {e}", path.display());
                    } else {
                        println!("  ✓ removed {}", path.display());
                        removed += 1;
                    }
                }
            }
        }
    }

    if dry_run {
        println!("(dry run — use 'edash clean' to actually remove)");
    } else {
        println!("removed {} unreferenced directories", removed);
    }

    Ok(())
}

pub fn cache() -> Result<(), Box<dyn std::error::Error>> {
    let cache_dir = paths::cache_dir();

    if !cache_dir.exists() {
        println!("Cache is empty");
        return Ok(());
    }

    let mut total = 0u64;
    for entry in std::fs::read_dir(&cache_dir)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        let size = meta.len();
        total += size;
        let name = entry.file_name();
        let display = if size >= 1024 * 1024 {
            format!("{:.0} MB", size as f64 / (1024.0 * 1024.0))
        } else if size >= 1024 {
            format!("{:.0} KB", size as f64 / 1024.0)
        } else {
            format!("{} B", size)
        };
        println!("  {:<40} {}", name.to_string_lossy(), display);
    }

    let total_display = if total >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", total as f64 / (1024.0 * 1024.0 * 1024.0))
    } else {
        format!("{:.0} MB", total as f64 / (1024.0 * 1024.0))
    };
    println!("  total: {}", total_display);
    println!("\nUse 'edash clean' to remove unreferenced installs.");
    Ok(())
}

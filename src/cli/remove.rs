use crate::lockfile::writer::{read_lockfile, write_lockfile};
use crate::paths;
use std::path::PathBuf;

pub fn remove(
    names: &[String],
    lock_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    if !lock_path.exists() {
        println!("Nothing to remove (no lockfile found)");
        return Ok(());
    }

    let mut lockfile = read_lockfile(lock_path)?;
    let envs_dir = paths::envs_dir();

    for name in names {
        if let Some(pos) = lockfile.package.iter().position(|p| p.name == *name) {
            let pkg = &lockfile.package[pos];
            let install_dir = envs_dir.join(format!("_{}", pkg.name));

            if install_dir.exists() {
                std::fs::remove_dir_all(&install_dir)?;
                println!("  ✓ removed {} {}", pkg.name, pkg.version);
            } else {
                println!("  ○ {} (no files on disk)", pkg.name);
            }

            lockfile.package.remove(pos);
        } else {
            println!("  ○ {} (not in lock)", name);
        }
    }

    write_lockfile(&lockfile, lock_path)?;
    println!("→ updated {}", lock_path.display());
    Ok(())
}

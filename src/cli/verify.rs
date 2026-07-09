use crate::lockfile::verifier::hash_dir;
use crate::paths;
use std::path::PathBuf;

pub fn verify(
    lock_path: &PathBuf,
    _verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !lock_path.exists() {
        println!("Nothing to verify (edash.lock not found)");
        return Ok(());
    }

    let mut lockfile = crate::lockfile::writer::read_lockfile(lock_path)?;

    if lockfile.package.is_empty() {
        println!("Nothing to verify (no packages in lock)");
        return Ok(());
    }

    let envs_dir = paths::envs_dir();
    let mut ok = 0;
    let mut fail = 0;
    let mut updated = 0;

    for pkg in &mut lockfile.package {
        let pkg_dir = if pkg.backend == "oss-cad-suite" {
            envs_dir.join("oss-cad-suite")
        } else {
            envs_dir.join(format!("_{}", pkg.name))
        };

        if !pkg_dir.exists() {
            println!("  ✗ {} (missing)", pkg.name);
            fail += 1;
            continue;
        }

        let actual = hash_dir(&pkg_dir)?;

        if pkg.sha256.is_empty() {
            // First verify — compute and store
            pkg.sha256 = actual;
            println!("  ✓ {} {}", pkg.name, &pkg.sha256[..8.min(pkg.sha256.len())]);
            updated += 1;
        } else if actual == pkg.sha256 {
            println!("  ✓ {} (verified)", pkg.name);
        } else {
            println!("  ✗ {} (hash mismatch)", pkg.name);
            fail += 1;
            continue;
        }
        ok += 1;
    }

    if updated > 0 {
        crate::lockfile::writer::write_lockfile(&lockfile, lock_path)?;
    }

    println!("{} passed, {} failed", ok, fail);
    Ok(())
}

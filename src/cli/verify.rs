use crate::lockfile::verifier::verify_all;
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

    let lockfile = crate::lockfile::writer::read_lockfile(lock_path)?;

    if lockfile.package.is_empty() {
        println!("Nothing to verify (no packages in lock)");
        return Ok(());
    }

    let results = verify_all(&lockfile.package, &paths::envs_dir())?;

    let ok_count = results.iter().filter(|r| r.ok).count();
    let fail_count = results.len() - ok_count;

    for r in &results {
        if r.ok {
            println!("  ✓ {}  sha256:{}", r.name, &r.expected_hash[..8.min(r.expected_hash.len())]);
        } else {
            println!("  ✗ {}  expected sha256:{}  (missing or corrupted)", r.name, &r.expected_hash[..8.min(r.expected_hash.len())]);
        }
    }

    println!(
        "{} passed, {} failed",
        ok_count,
        fail_count
    );

    Ok(())
}

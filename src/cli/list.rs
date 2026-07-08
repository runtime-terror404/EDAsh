use std::path::PathBuf;

pub fn list(lock_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if !lock_path.exists() {
        println!("No packages installed (edash.lock not found)");
        return Ok(());
    }

    let lockfile = crate::lockfile::writer::read_lockfile(lock_path)?;

    if lockfile.package.is_empty() {
        println!("No packages installed");
        return Ok(());
    }

    println!("Installed packages:");
    for pkg in &lockfile.package {
        let channel = pkg.channel.as_deref().unwrap_or("-");
        println!(
            "  {}  {}  [{}::{}]  sha256:{}",
            pkg.name,
            pkg.version,
            pkg.backend,
            channel,
            &pkg.sha256[..8.min(pkg.sha256.len())]
        );
    }

    if !lockfile.pdk.is_empty() {
        println!("PDKs:");
        for (name, pdk) in &lockfile.pdk {
            println!("  {}  {}  ref:{}", name, pdk.variant, pdk.git_ref);
        }
    }

    Ok(())
}

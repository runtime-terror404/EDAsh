use crate::lockfile::schema::Lockfile;
use std::path::Path;

pub fn write_lockfile(lockfile: &Lockfile, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let toml_str = toml::to_string_pretty(lockfile)?;
    std::fs::write(path, toml_str)?;
    Ok(())
}

pub fn read_lockfile(path: &Path) -> Result<Lockfile, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let lockfile: Lockfile = toml::from_str(&content)?;
    Ok(lockfile)
}

use crate::lockfile::schema::LockedPackage;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

pub fn verify_package(pkg: &LockedPackage, install_path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let pkg_dir = install_path.join(&pkg.name);
    if !pkg_dir.exists() {
        return Ok(false);
    }
    let actual_hash = hash_dir(&pkg_dir)?;
    Ok(actual_hash == pkg.sha256)
}

fn hash_dir(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut hasher = Sha256::new();
    hash_dir_recursive(path, &mut hasher)?;
    Ok(hex::encode(hasher.finalize()))
}

fn hash_dir_recursive(path: &Path, hasher: &mut Sha256) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_file() {
        let mut file = fs::File::open(path)?;
        let mut buffer = [0u8; 8192];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        hasher.update(path.to_string_lossy().as_bytes());
    } else if path.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(path)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            hash_dir_recursive(&entry.path(), hasher)?;
        }
    }
    Ok(())
}

pub fn verify_all(
    packages: &[LockedPackage],
    install_path: &Path,
) -> Result<Vec<VerificationResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();
    for pkg in packages {
        let ok = verify_package(pkg, install_path).unwrap_or(false);
        results.push(VerificationResult {
            name: pkg.name.clone(),
            ok,
            expected_hash: pkg.sha256.clone(),
        });
    }
    Ok(results)
}

#[derive(Debug)]
pub struct VerificationResult {
    pub name: String,
    pub ok: bool,
    pub expected_hash: String,
}

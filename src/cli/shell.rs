use crate::catalog::resolver::Resolver;
use crate::paths;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

pub fn shell(
    name: &str,
    catalog_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolver = Resolver::load(catalog_dir)?;

    let requests = resolver.resolve(name)?;
    let envs_dir = paths::envs_dir();

    let mut paths: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for req in &requests {
        let bin_dir = match req.backend {
            crate::catalog::index::BackendKind::OssCadSuite => {
                envs_dir.join("oss-cad-suite").join("bin")
            }
            _ => envs_dir.join(format!("_{}", req.name)).join("bin"),
        };

        let dir_str = bin_dir.to_string_lossy().to_string();
        if seen.insert(dir_str.clone()) && bin_dir.exists() {
            paths.push(dir_str);
        }
    }

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", paths.join(":"), current_path);

    let status = Command::new(&shell)
        .env("PATH", &new_path)
        .env("EDASH_PROFILE", name)
        .status()?;

    std::process::exit(status.code().unwrap_or(1));
}

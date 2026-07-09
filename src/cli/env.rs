use crate::catalog::resolver::Resolver;
use crate::paths;
use std::collections::HashSet;
use std::path::PathBuf;

pub fn env(
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

    println!("export PATH={}:${{PATH}}", paths.join(":"));
    println!("export EDASH_PROFILE={}", name);

    Ok(())
}

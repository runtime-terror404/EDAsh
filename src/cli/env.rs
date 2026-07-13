use crate::catalog::index::ResolvedItem;
use crate::catalog::resolver::Resolver;
use crate::catalog::CatalogSource;
use crate::paths;
use std::collections::HashSet;

pub fn env(
    name: &str,
    source: &CatalogSource,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolver = Resolver::load_from(source)?;
    let items = resolver.resolve(name)?;
    let envs_dir = paths::envs_dir();

    let mut paths: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for item in &items {
        let req = match item {
            ResolvedItem::Tool(req) => req,
            ResolvedItem::Pdk(_) => continue,
        };

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

    // PDK vars: PDK_ROOT + per-PDK path variables
    let pdks_dir = paths::pdks_dir();
    let lock_path = paths::lockfile_path();
    if pdks_dir.exists() {
        println!("export PDK_ROOT={}", pdks_dir.display());

        if lock_path.exists() {
            if let Ok(lf) = crate::lockfile::writer::read_lockfile(&lock_path) {
                let installed_pdks: Vec<String> = lf.pdk.keys().cloned().collect();
                if !installed_pdks.is_empty() {
                    let pdk_vars = crate::pdk::config::resolve_pdk_vars(
                        &installed_pdks, source, &pdks_dir,
                    );
                    for (var, val) in &pdk_vars {
                        println!("export {}={}", var, val);
                    }
                }
            }
        }
    }

    println!("export PATH={}:${{PATH}}", paths.join(":"));
    println!("export EDASH_PROFILE={}", name);

    Ok(())
}

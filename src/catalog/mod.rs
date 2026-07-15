pub mod index;
pub mod resolver;
pub mod source;

use std::path::PathBuf;

/// How the catalog was sourced — either an explicit path (dev mode / `-c` flag)
/// or the XDG default (base + user merge for release binaries).
#[derive(Debug, Clone)]
pub enum CatalogSource {
    /// Explicit path from `-c` flag or `EDASH_CATALOG_PATH` env var.
    Path(PathBuf),
    /// XDG default — merge `$XDG_DATA_HOME/edash/catalog/base/` + `$XDG_CONFIG_HOME/edash/catalog/user/`.
    Default,
}

impl CatalogSource {
    /// Resolve a PDK config YAML file. For explicit paths, reads from that directory.
    /// For default, checks user dir first, then falls back to base dir.
    pub fn read_pdk_config(&self, pdk_name: &str) -> Option<String> {
        match self {
            CatalogSource::Path(base) => {
                let p = base.join("pdks").join(format!("{}.yaml", pdk_name));
                std::fs::read_to_string(&p).ok()
            }
            CatalogSource::Default => {
                let user_p = crate::paths::catalog_user_dir()
                    .join("pdks")
                    .join(format!("{}.yaml", pdk_name));
                if user_p.exists() {
                    return std::fs::read_to_string(&user_p).ok();
                }
                let base_p = crate::paths::catalog_base_dir()
                    .join("pdks")
                    .join(format!("{}.yaml", pdk_name));
                std::fs::read_to_string(&base_p).ok()
            }
        }
    }

    /// List available PDK names. For explicit paths, reads that directory.
    /// For default, merges listings from user + base dirs.
    pub fn list_pdk_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        match self {
            CatalogSource::Path(base) => {
                let pdks_dir = base.join("pdks");
                if let Ok(entries) = std::fs::read_dir(&pdks_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().map_or(false, |ext| ext == "yaml") {
                            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                                names.push(stem.to_string());
                            }
                        }
                    }
                }
            }
            CatalogSource::Default => {
                // Collect from both dirs, user wins on duplicates
                let mut seen = std::collections::HashSet::new();
                for dir in &[crate::paths::catalog_user_dir(), crate::paths::catalog_base_dir()] {
                    let pdks_dir = dir.join("pdks");
                    if let Ok(entries) = std::fs::read_dir(&pdks_dir) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            if p.extension().map_or(false, |ext| ext == "yaml") {
                                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                                    if seen.insert(stem.to_string()) {
                                        names.push(stem.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        names
    }

    /// Read a pre-computed explicit lock file for a tool.
    /// Returns URLs if found, None if the tool has no pre-computed lock.
    pub fn read_tool_lock(&self, tool_name: &str) -> Option<Vec<String>> {
        let rel_path = format!("locks/{}.explicit.txt", tool_name);
        match self {
            CatalogSource::Path(base) => {
                let p = base.join(&rel_path);
                if p.exists() {
                    let content = std::fs::read_to_string(&p).ok()?;
                    Some(content.lines().filter(|l| !l.starts_with('#') && !l.starts_with('@') && !l.trim().is_empty()).map(|l| l.to_string()).collect())
                } else {
                    None
                }
            }
            CatalogSource::Default => {
                let user_p = crate::paths::catalog_user_dir().join(&rel_path);
                if user_p.exists() {
                    let content = std::fs::read_to_string(&user_p).ok()?;
                    return Some(content.lines().filter(|l| !l.starts_with('#') && !l.starts_with('@') && !l.trim().is_empty()).map(|l| l.to_string()).collect());
                }
                let base_p = crate::paths::catalog_base_dir().join(&rel_path);
                if base_p.exists() {
                    let content = std::fs::read_to_string(&base_p).ok()?;
                    Some(content.lines().filter(|l| !l.starts_with('#') && !l.starts_with('@') && !l.trim().is_empty()).map(|l| l.to_string()).collect())
                } else {
                    None
                }
            }
        }
    }
}

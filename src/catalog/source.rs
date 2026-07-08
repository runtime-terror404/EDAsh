use std::path::{Path, PathBuf};

pub fn default_catalog_dir() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("catalog")
}

pub fn ensure_catalog(catalog_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !catalog_dir.exists() {
        return Err(format!(
            "Catalog directory not found: {}. The catalog/ directory should be \
             included with the edash binary or available via EDASH_CATALOG_PATH.",
            catalog_dir.display()
        )
        .into());
    }
    Ok(())
}

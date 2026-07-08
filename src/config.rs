use std::path::PathBuf;

pub struct Config {
    pub catalog_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl Config {
    pub fn load() -> Self {
        let data_dir = crate::paths::data_dir();
        let catalog_dir = std::env::var("EDASH_CATALOG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("catalog"));

        Self {
            catalog_dir,
            data_dir,
        }
    }
}

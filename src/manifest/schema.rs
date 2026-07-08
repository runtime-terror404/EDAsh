use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manifest {
    #[serde(default)]
    pub environments: Vec<String>,
    #[serde(default)]
    pub pdk: HashMap<String, PdkOverride>,
    #[serde(default)]
    pub overrides: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PdkOverride {
    pub variant: Option<String>,
}

impl Manifest {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Manifest = serde_yaml::from_str(&content)?;
        Ok(manifest)
    }

    pub fn find_upwards(start_dir: &Path) -> Option<(Self, std::path::PathBuf)> {
        let mut current = start_dir.to_path_buf();
        loop {
            let candidate = current.join("edash.yaml");
            if candidate.exists() {
                if let Ok(m) = Self::from_file(&candidate) {
                    return Some((m, candidate));
                }
            }
            if !current.pop() {
                break;
            }
        }
        None
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            environments: Vec::new(),
            pdk: HashMap::new(),
            overrides: HashMap::new(),
        }
    }
}

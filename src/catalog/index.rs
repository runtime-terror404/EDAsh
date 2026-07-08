use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct CatalogIndex {
    pub environments: HashMap<String, String>,
    pub pdks: Option<HashMap<String, PdkEntry>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PdkEntry {
    pub manager: String,
    pub variant: Option<String>,
    #[serde(default)]
    pub build: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnvironmentDef {
    pub name: String,
    pub tools: Vec<String>,
}

pub type ToolRegistry = HashMap<String, ToolEntry>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToolEntry {
    pub backend: String,
    pub channel: Option<String>,
    pub package: Option<String>,
    pub repo: Option<String>,
    #[serde(default)]
    pub requires: Option<Vec<String>>,
    #[serde(default)]
    pub mpi: Option<String>,
}

#[derive(Debug, Clone)]
pub enum BackendKind {
    Micromamba,
    OssCadSuite,
    Source,
}

impl BackendKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "micromamba" => BackendKind::Micromamba,
            "oss-cad-suite" => BackendKind::OssCadSuite,
            "source" => BackendKind::Source,
            _ => panic!("Unknown backend: {}", s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PackageRequest {
    pub name: String,
    pub backend: BackendKind,
    pub channel: Option<String>,
    pub package: Option<String>,
}

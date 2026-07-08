use crate::catalog::index::PackageRequest;
use tokio::sync::mpsc;

pub mod micromamba;
pub mod oss_cad_suite;

pub type ProgressTx = mpsc::UnboundedSender<Progress>;

#[derive(Debug, Clone)]
pub enum Progress {
    Stage(String),
    Bytes { done: u64, total: u64 },
    Log(String),
    Done,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: String,
    pub channel: Option<String>,
    pub backend: String,
    pub sha256: String,
    pub install_path: std::path::PathBuf,
}

pub trait Backend {
    fn name(&self) -> &'static str;

    fn resolve(
        &self,
        req: &PackageRequest,
    ) -> Result<ResolvedPackage, Box<dyn std::error::Error>>;

    fn install(
        &self,
        pkg: &ResolvedPackage,
        progress: ProgressTx,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn verify(&self, pkg: &ResolvedPackage) -> Result<bool, Box<dyn std::error::Error>>;

    fn remove(&self, pkg: &ResolvedPackage) -> Result<(), Box<dyn std::error::Error>>;
}

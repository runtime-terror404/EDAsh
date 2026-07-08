use crate::backend::{Backend, ProgressTx, ResolvedPackage};
use crate::catalog::index::PackageRequest;
use crate::lockfile::schema::LockedPackage;
use crate::paths;
use std::path::PathBuf;

pub struct OssCadSuiteBackend {
    install_dir: PathBuf,
}

impl OssCadSuiteBackend {
    pub fn new() -> Self {
        Self {
            install_dir: paths::envs_dir().join("oss-cad-suite"),
        }
    }

    pub fn is_installed(&self) -> bool {
        self.install_dir.exists() && self.install_dir.join("environment").exists()
    }

    pub fn install_package(
        &self,
        req: &PackageRequest,
        _progress: ProgressTx,
    ) -> Result<LockedPackage, Box<dyn std::error::Error>> {
        if !self.is_installed() {
            return Err(
                "oss-cad-suite not installed. Download from:\n  \
                 https://github.com/YosysHQ/oss-cad-suite-build/releases/latest"
                    .into(),
            );
        }

        Ok(LockedPackage {
            name: req.name.clone(),
            version: "oss-cad-suite".to_string(),
            channel: None,
            backend: "oss-cad-suite".to_string(),
            sha256: String::new(),
        })
    }
}

impl Backend for OssCadSuiteBackend {
    fn name(&self) -> &'static str {
        "oss-cad-suite"
    }

    fn resolve(
        &self,
        _req: &PackageRequest,
    ) -> Result<ResolvedPackage, Box<dyn std::error::Error>> {
        Err("oss-cad-suite is monolithic — use install_package() for Phase 0".into())
    }

    fn install(
        &self,
        _pkg: &ResolvedPackage,
        _progress: ProgressTx,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("use install_package() for Phase 0".into())
    }

    fn verify(&self, _pkg: &ResolvedPackage) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(self.is_installed())
    }

    fn remove(&self, _pkg: &ResolvedPackage) -> Result<(), Box<dyn std::error::Error>> {
        if self.install_dir.exists() {
            std::fs::remove_dir_all(&self.install_dir)?;
        }
        Ok(())
    }
}

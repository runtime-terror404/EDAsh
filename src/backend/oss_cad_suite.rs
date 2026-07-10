use crate::backend::{Backend, Progress, ProgressTx, ResolvedPackage};
use crate::catalog::index::PackageRequest;
use crate::lockfile::schema::LockedPackage;
use crate::paths;
use std::path::PathBuf;
use std::process::Command;

const API_URL: &str =
    "https://api.github.com/repos/YosysHQ/oss-cad-suite-build/releases/latest";

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
        self.install_dir.join("environment").exists()
    }

    fn latest_download_url() -> Result<String, Box<dyn std::error::Error>> {
        let output = Command::new("curl")
            .args(["-s", API_URL])
            .output()
            .map_err(|e| format!("curl failed: {e}"))?;

        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).map_err(|e| format!("api parse: {e}"))?;

        let tag = json["tag_name"]
            .as_str()
            .ok_or("no tag_name in release api")?;

        let date_stripped = tag.replace('-', "");
        let filename = format!("oss-cad-suite-linux-x64-{date_stripped}.tgz");
        let url = format!(
            "https://github.com/YosysHQ/oss-cad-suite-build/releases/download/{tag}/{filename}"
        );
        Ok(url)
    }

    pub fn install_package(
        &self,
        req: &PackageRequest,
        progress: ProgressTx,
    ) -> Result<LockedPackage, Box<dyn std::error::Error>> {
        if !self.is_installed() {
            self.download_and_extract(&progress)?;
        }

        Ok(LockedPackage {
            name: req.name.clone(),
            version: "oss-cad-suite".to_string(),
            channel: None,
            backend: "oss-cad-suite".to_string(),
            sha256: String::new(),
        })
    }

    fn download_and_extract(
        &self,
        progress: &ProgressTx,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = Self::latest_download_url()?;
        let cache_dir = paths::cache_dir();
        std::fs::create_dir_all(&cache_dir)?;

        let filename = url.rsplit('/').next().unwrap_or("oss-cad-suite.tgz");
        let cache_path = cache_dir.join(filename);

        if !cache_path.exists() {
            let _ = progress.send(Progress::Stage("downloading oss-cad-suite (~1.5GB)".into()));

            let output = Command::new("curl")
                .args([
                    "-L",
                    "-sS",
                    "-o",
                    &cache_path.to_string_lossy(),
                    &url,
                ])
                .output()
                .map_err(|e| format!("curl failed: {e}"))?;

            if !output.status.success() {
                let _ = std::fs::remove_file(&cache_path);
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("oss-cad-suite download failed: {}", stderr).into());
            }
        }

        let _ = progress.send(Progress::Stage("extracting oss-cad-suite".into()));
        std::fs::create_dir_all(&self.install_dir)?;

        let output = Command::new("tar")
            .args([
                "-xzf",
                &cache_path.to_string_lossy(),
                "-C",
                &self.install_dir.to_string_lossy(),
                "--strip-components=1",
            ])
            .output()
            .map_err(|e| format!("tar failed: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("oss-cad-suite extraction failed: {}", stderr).into());
        }

        Ok(())
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

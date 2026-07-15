use crate::backend::{Backend, Progress, ProgressTx, ResolvedPackage};
use crate::catalog::index::PackageRequest;
use crate::lockfile::schema::LockedPackage;
use crate::paths;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct MicromambaBackend {
    binary: PathBuf,
}

impl MicromambaBackend {
    pub fn new() -> Self {
        Self {
            binary: find_micromamba(),
        }
    }

    pub fn is_available(&self) -> bool {
        self.binary.exists()
    }

    pub fn binary_path(&self) -> &Path {
        &self.binary
    }

    pub fn install_package(
        &self,
        req: &PackageRequest,
        progress: ProgressTx,
    ) -> Result<LockedPackage, Box<dyn std::error::Error>> {
        let channel = req.channel.as_deref().unwrap_or("conda-forge");
        let package = req.package.as_deref().unwrap_or(&req.name);
        let prefix = paths::envs_dir().join(format!("_{}", req.name));

        let exists = prefix.exists() && prefix.join("conda-meta").exists();
        let subcommand = if exists { "install" } else { "create" };

        let output = if !req.explicit_urls.is_empty() {
            // Path A: explicit URLs — write to temp file, fetch directly, no solver
            let _ = progress.send(Progress::Stage(format!(
                "fetch:{} <- explicit lock ({} urls)",
                req.name,
                req.explicit_urls.len()
            )));
            let tmpdir = std::env::temp_dir().join(format!("edash_lock_{}", req.name));
            std::fs::create_dir_all(&tmpdir)?;
            let lock_file = tmpdir.join("explicit.txt");
            std::fs::write(&lock_file, req.explicit_urls.join("\n") + "\n")?;

            let result = Command::new(&self.binary)
                .args([
                    subcommand,
                    "-p",
                    &prefix.to_string_lossy(),
                    "--file",
                    &lock_file.to_string_lossy(),
                    "-y",
                    "--quiet",
                ])
                .output()
                .map_err(|e| format!("failed to run micromamba: {}", e));
            let _ = std::fs::remove_dir_all(&tmpdir);
            result?
        } else {
            // Path B: spec-based — hermetic solver with controlled baseline
            let _ = progress.send(Progress::Stage(format!(
                "{}:{} <- {}::{}",
                subcommand, req.name, channel, package
            )));

            Command::new(&self.binary)
                .args([
                    subcommand,
                    "--override-channels",
                    "--strict-channel-priority",
                    "-c",
                    channel,
                    "-p",
                    &prefix.to_string_lossy(),
                    package,
                    "-y",
                    "--quiet",
                ])
                .env("CONDA_OVERRIDE_GLIBC", "2.35")
                .output()
                .map_err(|e| format!("failed to run micromamba: {}", e))?
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("micromamba install failed: {}", stderr).into());
        }

        let version = self.query_version(&prefix, package)?;

        Ok(LockedPackage {
            name: req.name.clone(),
            version,
            channel: Some(channel.to_string()),
            backend: "micromamba".to_string(),
            sha256: String::new(),
            explicit_urls: req.explicit_urls.clone(),
        })
    }

    fn query_version(
        &self,
        prefix: &Path,
        package: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let output = Command::new(&self.binary)
            .args([
                "list",
                "-p",
                &prefix.to_string_lossy(),
                package,
                "--json",
            ])
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                if let Some(first) = parsed.first() {
                    if let Some(version) = first.get("version").and_then(|v| v.as_str()) {
                        return Ok(version.to_string());
                    }
                }
            }
        }
        Ok("unknown".to_string())
    }
}

fn find_micromamba() -> PathBuf {
    if let Ok(path) = which::which("micromamba") {
        return path;
    }
    let local = paths::bin_dir().join("micromamba");
    if local.exists() {
        return local;
    }
    // Official micromamba installer puts it in ~/.local/bin/
    let home_local = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/root"))
        .join(".local/bin/micromamba");
    if home_local.exists() {
        return home_local;
    }
    PathBuf::from("micromamba")
}

impl Backend for MicromambaBackend {
    fn name(&self) -> &'static str {
        "micromamba"
    }

    fn resolve(
        &self,
        _req: &PackageRequest,
    ) -> Result<ResolvedPackage, Box<dyn std::error::Error>> {
        Err("use install_package() for Phase 0 — resolve+install combined".into())
    }

    fn install(
        &self,
        _pkg: &ResolvedPackage,
        _progress: ProgressTx,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Err("use install_package() for Phase 0".into())
    }

    fn verify(&self, pkg: &ResolvedPackage) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(pkg.install_path.exists() && pkg.install_path.join("conda-meta").exists())
    }

    fn remove(&self, pkg: &ResolvedPackage) -> Result<(), Box<dyn std::error::Error>> {
        if pkg.install_path.exists() {
            std::fs::remove_dir_all(&pkg.install_path)?;
        }
        Ok(())
    }
}

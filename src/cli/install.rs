use crate::backend::{Progress, ProgressTx};
use crate::catalog::index::{BackendKind, PackageRequest};
use crate::catalog::resolver::Resolver;
use crate::lockfile::schema::{LockedPackage, Lockfile};
use crate::lockfile::writer::write_lockfile;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub async fn install(
    names: &[String],
    catalog_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolver = Resolver::load(catalog_dir)?;
    let lock_path = std::env::current_dir()?.join("edash.lock");

    let mut lockfile = if lock_path.exists() {
        crate::lockfile::writer::read_lockfile(&lock_path).unwrap_or_else(|_| Lockfile::new())
    } else {
        Lockfile::new()
    };

    for name in names {
        let requests = match resolver.resolve(name) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("✗ {} — {}", name, e);
                continue;
            }
        };

        println!("▸ {} ({} packages)", name, requests.len());

        for req in &requests {
            if let Some(existing) = lockfile.package.iter().find(|p| p.name == req.name) {
                let pkg_dir = crate::paths::envs_dir().join(format!("_{}", req.name));
                let already_installed = pkg_dir.exists();

                if already_installed {
                    println!("  ✓ {} {} (already installed)", existing.name, existing.version);
                    continue;
                } else {
                    println!("  ○ {} (in lock, missing on disk — reinstalling)", req.name);
                    lockfile.package.retain(|p| p.name != req.name);
                }
            }

            let (tx, mut rx) = mpsc::unbounded_channel();

            match install_package(req, tx.clone()) {
                Ok(pkg) => {
                    let _ = tx.send(Progress::Done);
                    drop(tx);

                    while let Some(ev) = rx.recv().await {
                        match ev {
                            Progress::Stage(s) => println!("  ◐ {}", s),
                            Progress::Done => println!("  ✓ {} {}", pkg.name, pkg.version),
                            Progress::Failed(e) => eprintln!("  ✗ {} — {}", pkg.name, e),
                            Progress::Log(line) => println!("    {}", line),
                            _ => {}
                        }
                    }

                    lockfile.package.retain(|p| p.name != pkg.name);
                    lockfile.package.push(pkg);
                }
                Err(e) => {
                    eprintln!("  ✗ {} — {}", req.name, e);
                    drop(tx);
                    drop(rx);
                }
            }
        }
    }

    write_lockfile(&lockfile, &lock_path)?;
    println!("→ wrote {}", lock_path.display());
    Ok(())
}

fn install_package(
    req: &PackageRequest,
    progress: ProgressTx,
) -> Result<LockedPackage, Box<dyn std::error::Error>> {
    match req.backend {
        BackendKind::Micromamba => {
            let backend = crate::backend::micromamba::MicromambaBackend::new();
            if !backend.is_available() {
                return Err(
                    "micromamba not found. Install: curl -L micro.mamba.pm/install.sh | bash"
                        .into(),
                );
            }
            backend.install_package(req, progress)
        }
        BackendKind::OssCadSuite => {
            let backend = crate::backend::oss_cad_suite::OssCadSuiteBackend::new();
            if !backend.is_installed() {
                return Err(
                    "oss-cad-suite not installed (~1.5GB download). Install manually:\n  \
                     curl -L https://github.com/YosysHQ/oss-cad-suite-build/releases/latest/download/oss-cad-suite-linux-x64.tar.xz | tar -xJ"
                        .into(),
                );
            }
            backend.install_package(req, progress)
        }
        BackendKind::Source => Err("source backend not available until phase 3".into()),
    }
}

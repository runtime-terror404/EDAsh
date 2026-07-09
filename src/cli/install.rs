use crate::backend::{Progress, ProgressTx};
use crate::catalog::index::{BackendKind, PackageRequest, ResolvedItem};
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
    let lock_path = crate::paths::lockfile_path();

    let mut lockfile = if lock_path.exists() {
        crate::lockfile::writer::read_lockfile(&lock_path).unwrap_or_else(|_| Lockfile::new())
    } else {
        Lockfile::new()
    };

    for name in names {
        let items = match resolver.resolve(name) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("✗ {} — {}", name, e);
                continue;
            }
        };

        let mut pkg_count = 0;
        let mut pdk_count = 0;
        for item in &items {
            match item {
                ResolvedItem::Tool(_) => pkg_count += 1,
                ResolvedItem::Pdk(_) => pdk_count += 1,
            }
        }

        let desc = match (pkg_count, pdk_count) {
            (0, 0) => String::new(),
            (p, 0) => format!("({p} packages)"),
            (0, p) => format!("({p} PDKs)"),
            (tp, pd) => format!("({tp} packages, {pd} PDKs)"),
        };
        println!("▸ {} {}", name, desc);

        for item in &items {
            match item {
                ResolvedItem::Tool(req) => {
                    install_tool(req, &mut lockfile).await;
                }
                ResolvedItem::Pdk(pdk_req) => {
                    install_pdk(pdk_req, &mut lockfile);
                }
            }
        }
    }

    write_lockfile(&lockfile, &lock_path)?;
    println!("→ wrote {}", lock_path.display());
    Ok(())
}

async fn install_tool(req: &PackageRequest, lockfile: &mut Lockfile) {
    if let Some(existing) = lockfile.package.iter().find(|p| p.name == req.name) {
        let pkg_dir = crate::paths::envs_dir().join(format!("_{}", req.name));
        let already_installed = pkg_dir.exists();

        if already_installed {
            println!("  ✓ {} {} (already installed)", existing.name, existing.version);
            return;
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

fn install_pdk(
    req: &crate::catalog::index::PdkRequest,
    lockfile: &mut Lockfile,
) {
    if lockfile.pdk.contains_key(&req.name) {
        println!("  ✓ {} (already installed)", req.name);
        return;
    }

    println!("  ◐ fetching {} via {}", req.name, req.manager);

    match crate::pdk::ciel::resolve_and_install(&req.name, &req.variant) {
        Ok(locked_pdk) => {
            println!("  ✓ {} {}", req.name, locked_pdk.git_ref);
            lockfile.pdk.insert(req.name.clone(), locked_pdk);
        }
        Err(e) => {
            eprintln!("  ✗ {} — {}", req.name, e);
        }
    }
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
            backend.install_package(req, progress)
        }
        BackendKind::Source => Err("source backend not available until phase 3".into()),
    }
}

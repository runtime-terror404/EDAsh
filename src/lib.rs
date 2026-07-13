pub mod actions;
pub mod backend;
pub mod catalog;
pub mod cli;
pub mod doctor;
pub mod installation;
pub mod lockfile;
pub mod manifest;
pub mod paths;
pub mod pdk;
pub mod tui;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "edash", version, about = "Reproducible EDA toolchain manager")]
pub struct Cli {
    #[arg(short, long, env = "EDASH_CATALOG_PATH")]
    pub catalog_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    Install {
        names: Vec<String>,
    },
    List {},
    Remove {
        names: Vec<String>,
    },
    Env {
        name: String,
    },
    Shell {
        name: String,
    },
    Doctor {
        name: String,
    },
    Search {
        query: String,
    },
    Why {
        tool: String,
    },
    Outdated {},
    Clean {
        #[arg(short, long)]
        dry_run: bool,
    },
    Cache {},
    Export {
        name: String,
        #[arg(short, long)]
        format: String,
    },
    Pdk {
        name: Option<String>,
        #[arg(long)]
        names_only: bool,
    },
    Update {},
    Repair {},
    /// Hidden installer subcommands (gated by EDASH_INSTALLER=1).
    #[command(hide = true, name = "__internal")]
    __Internal {
        #[command(subcommand)]
        action: InstallAction,
    },
}

#[derive(Subcommand)]
pub enum InstallAction {
    /// Print config_dir, data_dir, bin_dir as key=value.
    Paths,
    /// Stage a catalog from an extracted directory.
    StageCatalog {
        extracted_dir: PathBuf,
        manifest: PathBuf,
    },
    /// Run self-test against live paths.
    SelfTest,
}

pub async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let source = resolve_catalog_source(cli.catalog_dir);

    let lock_path = crate::paths::lockfile_path();

    match cli.command {
        Some(Command::Install { names }) => {
            cli::install::install(&names, &source).await
        }
        Some(Command::List { .. }) => cli::list::list(&lock_path),
        Some(Command::Remove { names }) => cli::remove::remove(&names, &source),
        Some(Command::Env { name }) => cli::env::env(&name, &source),
        Some(Command::Shell { name }) => cli::shell::shell(&name, &source),
        Some(Command::Doctor { name }) => cli::doctor::doctor(&name, &source),
        Some(Command::Search { query }) => cli::search::search(&query, &source),
        Some(Command::Why { tool }) => cli::why::why(&tool, &source),
        Some(Command::Outdated { .. }) => cli::outdated::outdated(&lock_path),
        Some(Command::Clean { dry_run }) => cli::clean::clean(&lock_path, dry_run),
        Some(Command::Cache { .. }) => cli::clean::cache(),
        Some(Command::Export { name, format }) => {
            cli::export::export(&name, &format, &source)
        }
        Some(Command::Pdk { name, names_only }) => {
            cli::pdk::pdk(name.as_deref(), &source, names_only)
        }
        Some(Command::Update { .. }) => cli::update::update(),
        Some(Command::Repair { .. }) => cli::repair::repair(),
        Some(Command::__Internal { action }) => match action {
            InstallAction::Paths => cli::installer::paths(),
            InstallAction::StageCatalog { extracted_dir, manifest } => {
                cli::installer::stage_catalog(&extracted_dir, &manifest)
            }
            InstallAction::SelfTest => cli::installer::self_test(),
        },
        None => {
            println!("edash — reproducible EDA toolchain manager");
            println!("Usage: edash <command>");
            println!("Commands: install, list, remove, env, shell");
            println!("Run 'edash help' for details.");
            Ok(())
        }
    }
}

/// Resolve the catalog source from CLI flag, env var, or XDG default.
pub fn resolve_catalog_source(explicit: Option<PathBuf>) -> crate::catalog::CatalogSource {
    if let Some(path) = explicit {
        return crate::catalog::CatalogSource::Path(path);
    }
    if let Ok(env_path) = std::env::var("EDASH_CATALOG_PATH") {
        return crate::catalog::CatalogSource::Path(PathBuf::from(env_path));
    }
    crate::catalog::CatalogSource::Default
}

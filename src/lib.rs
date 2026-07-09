pub mod backend;
pub mod catalog;
pub mod cli;
pub mod config;
pub mod doctor;
pub mod lockfile;
pub mod manifest;
pub mod paths;
pub mod pdk;

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
    Verify {
        #[arg(short, long)]
        verbose: bool,
    },
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
}

pub async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let catalog_dir = cli.catalog_dir.unwrap_or_else(|| {
        std::env::var("EDASH_CATALOG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("catalog"))
    });

    let lock_path = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("edash.lock");

    match cli.command {
        Some(Command::Install { names }) => {
            cli::install::install(&names, &catalog_dir).await
        }
        Some(Command::List { .. }) => cli::list::list(&lock_path),
        Some(Command::Verify { verbose }) => cli::verify::verify(&lock_path, verbose),
        Some(Command::Remove { names }) => cli::remove::remove(&names, &lock_path),
        Some(Command::Env { name }) => cli::env::env(&name, &catalog_dir),
        Some(Command::Shell { name }) => cli::shell::shell(&name, &catalog_dir),
        Some(Command::Doctor { name }) => cli::doctor::doctor(&name, &catalog_dir),
        None => {
            println!("edash — reproducible EDA toolchain manager");
            println!("Usage: edash <command>");
            println!("Commands: install, list, verify, remove, env, shell");
            println!("Run 'edash help' for details.");
            Ok(())
        }
    }
}

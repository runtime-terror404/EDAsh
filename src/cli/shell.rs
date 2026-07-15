use crate::catalog::index::ResolvedItem;
use crate::catalog::resolver::Resolver;
use crate::catalog::CatalogSource;
use crate::lockfile::schema::Lockfile;
use crate::paths;
use std::collections::HashSet;
use std::process::Command;

pub fn shell(
    name: &str,
    source: &CatalogSource,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolver = Resolver::load_from(source)?;

    let items = resolver.resolve(name)?;
    let envs_dir = paths::envs_dir();

    let mut paths: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut installed_tools: Vec<String> = Vec::new();

    // Read lockfile to know what's installed
    let lock_path = paths::lockfile_path();
    let lockfile = if lock_path.exists() {
        crate::lockfile::writer::read_lockfile(&lock_path).unwrap_or(Lockfile::new())
    } else {
        Lockfile::new()
    };

    for item in &items {
        let req = match item {
            ResolvedItem::Tool(req) => req,
            ResolvedItem::Pdk(_) => continue,
        };

        let bin_dir = match req.backend {
            crate::catalog::index::BackendKind::OssCadSuite => {
                envs_dir.join("oss-cad-suite").join("bin")
            }
            _ => envs_dir.join(format!("_{}", req.name)).join("bin"),
        };

        let dir_str = bin_dir.to_string_lossy().to_string();
        if seen.insert(dir_str.clone()) && bin_dir.exists() {
            paths.push(dir_str);
        }
        // Check if installed
        if lockfile.package.iter().any(|p| p.name == req.name) {
            installed_tools.push(req.name.clone());
        }
    }

    // Print MOTD
    print_motd(name, &installed_tools);

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", paths.join(":"), current_path);

    let mut cmd = Command::new(&shell);
    cmd.env("PATH", &new_path)
       .env("EDASH_PROFILE", name);

    let pdks_dir = paths::pdks_dir();
    if pdks_dir.exists() {
        cmd.env("PDK_ROOT", pdks_dir.to_string_lossy().as_ref());

        // Set per-PDK path variables
        let installed_pdks: Vec<String> = lockfile.pdk.keys().cloned().collect();
        if !installed_pdks.is_empty() {
            let pdk_vars = crate::pdk::config::resolve_pdk_vars(
                &installed_pdks, source, &pdks_dir,
            );
            for (var, val) in &pdk_vars {
                cmd.env(var.as_str(), val.as_str());
            }
        }
    }

    let status = cmd.status()?;

    std::process::exit(status.code().unwrap_or(1));
}

fn print_motd(env_name: &str, tools: &[String]) {
    let name = match env_name {
        "digital" => "Digital",
        "analog" => "Analog",
        _ => env_name,
    };

    // Get terminal width, fall back to 80 if not a TTY
    let width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(76);

    let line1 = "────┐        ┌──────┐   ┌─┐     ┌────────┐      ┌───────┐";
    let l2_prefix = "    └────────┘      └───┘ └─────┘        └──────┘       └";
    let l2_tail = width.saturating_sub(l2_prefix.chars().count());
    let line2 = format!("{}{}", l2_prefix, "─".repeat(l2_tail));
    let line3 = " ░█▀▀░█▀▄░█▀█░█▀▀░█░█";
    let line4 = " ░█▀▀░█░█░█▀█░▀▀█░█▀█  Open-source EDA toolchain manager for ASIC, FPGA, and analog design.";
    let line5 = " ░▀▀▀░▀▀░░▀░▀░▀▀▀░▀░▀";
    let sep = "─".repeat(width);

    println!("{line1}\n{line2}\n{line3}\n{line4}\n{line5}\n");
    println!("Welcome to the {} environment.......\n", name);
    if !tools.is_empty() {
        println!("Available tools: {}", tools.join(" • "));
    }
    println!("\nType 'exit' to restore your original shell.");
    println!("\nPowered by open-source EDA.\nBuild without barriers.");
    println!("{sep}");
}

use crate::catalog::CatalogSource;
use crate::paths;

pub fn pdk(
    name: Option<&str>,
    source: &CatalogSource,
    names_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let lock_path = paths::lockfile_path();
    let installed: Vec<String> = if lock_path.exists() {
        crate::lockfile::writer::read_lockfile(&lock_path)
            .map(|lf| lf.pdk.keys().cloned().collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let available: Vec<String> = source.list_pdk_names();

    // No arg: summary
    let Some(name) = name else {
        let installed_set: std::collections::HashSet<_> = installed.iter().collect();
        if !installed.is_empty() {
            println!("Installed PDKs:");
            for pdk in &installed {
                let variant = crate::pdk::config::load_pdk_config(pdk, source)
                    .map(|c| c.variant)
                    .unwrap_or_else(|| "?".into());
                println!("  {:<16} ({})", pdk, variant);
            }
            println!();
        }
        let uninstalled: Vec<_> = available.iter().filter(|n| !installed_set.contains(*n)).collect();
        if !uninstalled.is_empty() {
            if installed.is_empty() {
                println!("Available PDKs (not installed):");
            } else {
                println!("Available PDKs (not installed):");
            }
            for pdk in uninstalled {
                println!("  {}", pdk);
            }
        }
        if installed.is_empty() && available.is_empty() {
            println!("No PDKs available.");
        }
        if installed.is_empty() && !available.is_empty() {
            println!("\nRun `edash install <name>` to install a PDK.");
        } else if !installed.is_empty() {
            println!("\nRun `edash pdk <name>` for usage details.");
        }
        return Ok(());
    };

    // Specific PDK
    let Some(config) = crate::pdk::config::load_pdk_config(name, source) else {
        if available.contains(&name.to_string()) {
            eprintln!("'{}' configuration file not found in catalog/pdks/", name);
        } else {
            let all: Vec<_> = available.iter().map(|s| s.as_str()).collect();
            eprintln!("Unknown PDK: '{}'. Available: {}", name, all.join(" "));
        }
        return Ok(());
    };

    let is_installed = installed.contains(&name.to_string());
    let status = if is_installed { "installed" } else { "not installed" };
    println!("{} ({}) — {}", config.name, config.variant, status);
    println!();

    if names_only {
        let pdk_root = paths::pdks_dir();
        let pdk_vars = crate::pdk::config::resolve_pdk_vars(&[name.to_string()], source, &pdk_root);
        for var in pdk_vars.keys() {
            println!("{}", var);
        }
        return Ok(());
    }

    if !is_installed {
        println!("  Run: edash install {}", name);
        return Ok(());
    }

    let pdk_root = paths::pdks_dir();
    let pdk_vars = crate::pdk::config::resolve_pdk_vars(&[name.to_string()], source, &pdk_root);

    if pdk_vars.is_empty() {
        println!("  No verified paths found for this PDK.");
        return Ok(());
    }

    println!("Environment variables:");
    for (var, val) in &pdk_vars {
        println!("  {:<24} = {}", var, val);
    }
    println!();

    println!("Usage per tool:");
    println!("  \x1b[1;4m{:<9}  {}\x1b[0m", "Tool", "    Recommended Command / Usage");
    let prefix = config.variant.to_uppercase().replace('-', "_");

    if pdk_vars.contains_key(&format!("{}_MAGIC_RCFILE", prefix)) {
        println!("  {:<9}:  magic -rcfile ${}_MAGIC_RCFILE", "magic", prefix);
    }
    if pdk_vars.contains_key(&format!("{}_XSCHEM_RCFILE", prefix)) {
        println!("  {:<9}:  xschem --rcfile ${}_XSCHEM_RCFILE", "xschem", prefix);
    }
    if pdk_vars.contains_key(&format!("{}_NETGEN_SETUP", prefix)) {
        println!("  {:<9}:  netgen -batch lvs <net_a> <net_b> ${}_NETGEN_SETUP <out>", "netgen", prefix);
    }
    if pdk_vars.contains_key(&format!("{}_SPICE_DIR", prefix)) {
        let file_hint = match name {
            "ihp-sg13g2" => "models/cornerMOSlv.lib",
            "gf180" => "sm141064.spice",
            _ => "all.spice",
        };
        println!("  {:<9}:  .lib ${}_SPICE_DIR/{}          (in your netlist)", "ngspice", prefix, file_hint);
    }
    if pdk_vars.contains_key(&format!("{}_KLAYOUT_TECH", prefix)) {
        println!("  {:<9}:  Tools → Manage Technologies → Add → Base path: (paste full ${}_KLAYOUT_TECH path)", "klayout", prefix);
    }

    Ok(())
}


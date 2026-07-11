use crate::actions;
use crate::catalog::index::ResolvedItem;
use crate::catalog::resolver::Resolver;
use std::path::PathBuf;

pub fn remove(
    names: &[String],
    catalog_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolver = Resolver::load(catalog_dir)?;

    for name in names {
        if name == "pdks" {
            match actions::remove_all_pdks() {
                Ok(n) => println!("  ✓ removed {} PDKs", n),
                Err(e) => eprintln!("  ✗ {}", e),
            }
            continue;
        }

        // Try resolving — could be an env name or a tool name
        let is_env = resolver.list_environments().contains(name);

        if let Ok(items) = resolver.resolve(name) {
            let tools: Vec<_> = items.iter().filter_map(|i| match i {
                ResolvedItem::Tool(req) => Some(req.name.clone()),
                _ => None,
            }).collect();
            let pdks: Vec<_> = items.iter().filter_map(|i| match i {
                ResolvedItem::Pdk(req) => Some(req.name.clone()),
                _ => None,
            }).collect();

            if is_env {
                // Env-level removal — protect shared tools
                println!("▸ {} ({} tools, {} PDKs)", name, tools.len(), pdks.len());
                let shared = |t: &str| -> bool {
                    resolver.which_envs(t).iter().any(|e| e != name.as_str())
                };
                match actions::remove_env(name, &tools, &shared) {
                    Ok((_removed, skipped)) => {
                        for t in &tools {
                            if actions::is_tool_installed(t) {
                                if resolver.which_envs(t).iter().any(|e| e != name.as_str()) {
                                    println!("  ○ {} (shared, kept)", t);
                                } else {
                                    println!("  ✓ removed {}", t);
                                }
                            }
                        }
                        if skipped > 0 { println!("  → {} shared tools kept", skipped); }
                    }
                    Err(e) => eprintln!("  ✗ {}", e),
                }
                for p in &pdks {
                    match actions::remove_pdk(p) {
                        Ok(true) => println!("  ✓ removed PDK {}", p),
                        Ok(false) => println!("  ○ PDK {} (not installed)", p),
                        Err(e) => eprintln!("  ✗ {} — {}", p, e),
                    }
                }
                continue;
            }

            // Not an env — remove individual tools/PDKs (no shared check)
            for t in &tools {
                match actions::remove_tool(t) {
                    Ok(true) => println!("  ✓ removed {}", t),
                    Ok(false) => println!("  ○ {} (not installed)", t),
                    Err(e) => eprintln!("  ✗ {} — {}", t, e),
                }
            }
            for p in &pdks {
                match actions::remove_pdk(p) {
                    Ok(true) => println!("  ✓ removed PDK {}", p),
                    Ok(false) => println!("  ○ PDK {} (not installed)", p),
                    Err(e) => eprintln!("  ✗ {} — {}", p, e),
                }
            }
            continue;
        }

        // Not resolved at all — try raw tool/PDK name
        match actions::remove_tool(name) {
            Ok(true) => { println!("  ✓ removed {}", name); continue; }
            Ok(false) => {}
            Err(e) => eprintln!("  ✗ {} — {}", name, e),
        }
        match actions::remove_pdk(name) {
            Ok(true) => println!("  ✓ removed PDK {}", name),
            Ok(false) => println!("  ○ {} (not in lock)", name),
            Err(e) => eprintln!("  ✗ {} — {}", name, e),
        }
    }

    Ok(())
}

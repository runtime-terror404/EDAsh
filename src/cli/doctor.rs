use crate::catalog::index::ResolvedItem;
use crate::catalog::resolver::Resolver;
use crate::catalog::CatalogSource;
use crate::doctor::checks;
use crate::paths;

pub fn doctor(
    name: &str,
    source: &CatalogSource,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolver = Resolver::load_from(source)?;
    let items = resolver.resolve(name)?;
    let envs_dir = paths::envs_dir();

    let tool_count = items.iter().filter(|i| matches!(i, ResolvedItem::Tool(_))).count();
    println!("doctor: {} ({} tools)\n", name, tool_count);

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

        let bin_name = bin_for(&req.name);
        let bin_path = bin_dir.join(bin_name);

        if !bin_path.exists() {
            println!(
                "  ✗ {:20} binary not found ({})",
                req.name, bin_name
            );
            continue;
        }

        let result = checks::run_check(&req.name, &bin_path.to_string_lossy());

        if result.passed {
            println!(
                "  ✓ {:20} {} ({:.1}s)",
                result.tool, result.detail, result.duration_ms as f64 / 1000.0
            );
        } else {
            let detail = if result.detail.len() > 80 {
                format!("{}…", &result.detail[..77])
            } else {
                result.detail.clone()
            };
            println!(
                "  ✗ {:20} {} ({:.1}s)",
                result.tool, detail, result.duration_ms as f64 / 1000.0
            );
        }
    }

    Ok(())
}

fn bin_for(tool: &str) -> &str {
    match tool {
        "xyce" => "Xyce",
        "nextpnr" => "nextpnr-ecp5",
        "icestorm" => "icepack",
        "prjtrellis" => "ecppack",
        "openfpgaloader" => "openFPGALoader",
        _ => tool,
    }
}

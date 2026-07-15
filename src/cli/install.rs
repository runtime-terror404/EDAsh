use crate::catalog::index::ResolvedItem;
use crate::catalog::resolver::Resolver;
use crate::catalog::CatalogSource;

pub async fn install(
    names: &[String],
    source: &CatalogSource,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolver = Resolver::load_from(source)?;

    for name in names {
        let items = match resolver.resolve(name) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("✗ {} — {}", name, e);
                continue;
            }
        };

        println!("▸ {} ({} items)", name, items.len());

        for item in &items {
            match item {
                ResolvedItem::Tool(req) => {
                    match crate::actions::install_tool(req, source) {
                        Ok(pkg) => println!("  ✓ {} {}", pkg.name, pkg.version),
                        Err(e) => eprintln!("  ✗ {} — {}", req.name, e),
                    }
                }
                ResolvedItem::Pdk(pdk_req) => {
                    match crate::actions::install_pdk(&pdk_req.name) {
                        Ok(pdk) => println!("  ✓ {} {}", pdk_req.name, pdk.git_ref),
                        Err(e) => {
                            if e.to_string().contains("already installed") {
                                println!("  ✓ {} (already installed)", pdk_req.name);
                            } else {
                                eprintln!("  ✗ {} — {}", pdk_req.name, e);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

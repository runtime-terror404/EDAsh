use crate::catalog::resolver::Resolver;
use std::path::PathBuf;

pub fn why(
    tool: &str,
    catalog_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolver = Resolver::load(catalog_dir)?;
    let envs = resolver.which_envs(tool);

    if envs.is_empty() {
        println!("'{}' is not pulled in by any environment", tool);
    } else {
        println!("'{}' is pulled in by:", tool);
        for env in &envs {
            println!("  ▸ {}", env);
        }
    }

    Ok(())
}

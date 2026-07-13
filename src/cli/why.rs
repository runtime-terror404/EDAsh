use crate::catalog::resolver::Resolver;
use crate::catalog::CatalogSource;

pub fn why(
    tool: &str,
    source: &CatalogSource,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolver = Resolver::load_from(source)?;
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

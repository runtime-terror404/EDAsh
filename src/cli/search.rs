use crate::catalog::resolver::Resolver;
use crate::catalog::CatalogSource;

pub fn search(
    query: &str,
    source: &CatalogSource,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolver = Resolver::load_from(source)?;
    let results = resolver.search(query);

    if results.is_empty() {
        println!("No matches for '{}'", query);
        return Ok(());
    }

    for entry in &results {
        println!("  {:20}  [{}]", entry.name, entry.kind);
    }

    println!("\n{} results", results.len());
    Ok(())
}

use crate::catalog::resolver::Resolver;
use std::path::PathBuf;

pub fn search(
    query: &str,
    catalog_dir: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolver = Resolver::load(catalog_dir)?;
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

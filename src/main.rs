use clap::Parser;

fn main() {
    let cli = edash::Cli::parse();

    // Bare invocation + TTY → dashboard
    if cli.command.is_none() && is_terminal::is_terminal(&std::io::stdout()) {
        let catalog_dir = cli.catalog_dir.unwrap_or_else(|| {
            std::env::var("EDASH_CATALOG_PATH")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("catalog")
                })
        });
        if let Err(e) = edash::tui::run(catalog_dir) {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    if let Err(e) = rt.block_on(edash::run(cli)) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

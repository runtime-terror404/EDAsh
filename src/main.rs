use clap::Parser;

fn main() {
    let cli = edash::Cli::parse();

    // Bare invocation + TTY → dashboard
    if cli.command.is_none() && is_terminal::is_terminal(&std::io::stdout()) {
        let source = edash::resolve_catalog_source(cli.catalog_dir);
        match edash::tui::run(source.clone()) {
            Ok(Some(shell_env)) => {
                let _ = edash::cli::shell::shell(&shell_env, &source);
            }
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
            _ => {}
        }
        return;
    }

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    if let Err(e) = rt.block_on(edash::run(cli)) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

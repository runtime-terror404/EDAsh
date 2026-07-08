use clap::Parser;

fn main() {
    let cli = edash::Cli::parse();

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    if let Err(e) = rt.block_on(edash::run(cli)) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

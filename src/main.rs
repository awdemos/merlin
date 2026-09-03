use std::net::SocketAddr;

use clap::{Arg, Command};

fn main() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    if let Err(e) = runtime.block_on(async_main()) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn async_main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let matches = Command::new("merlin")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Production-grade multi-provider LLM router")
        .subcommand(
            Command::new("serve")
                .about("Start the HTTP server")
                .arg(
                    Arg::new("port")
                        .long("port")
                        .short('p')
                        .value_name("PORT")
                        .help("Port to listen on (default 7777)"),
                )
                .arg(
                    Arg::new("config")
                        .long("config")
                        .short('c')
                        .value_name("CONFIG")
                        .help("Configuration file path"),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("serve", sub_matches)) => {
            let config_path = sub_matches
                .get_one::<String>("config")
                .cloned()
                .or_else(|| std::env::var("MERLIN_CONFIG").ok());

            let port = match sub_matches.get_one::<String>("port") {
                Some(p) => p.parse::<u16>()?,
                None => config_path
                    .as_deref()
                    .and_then(|path| merlin::config::MerlinConfig::load_from_file(path).ok())
                    .map(|config| config.server.port)
                    .unwrap_or(7777),
            };

            let addr = SocketAddr::from(([0, 0, 0, 0], port));
            tracing::info!("Starting Merlin server on {}", addr);
            merlin::server::serve(addr, config_path.as_deref()).await?;
        }
        _ => {
            eprintln!("Use 'merlin serve --help' for usage");
        }
    }

    Ok(())
}

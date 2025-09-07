use clap::{Arg, Command};
use merlin::server;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let matches = Command::new("merlin")
        .version(env!("CARGO_PKG_VERSION"))
        .about("AI Routing Wizard")
        .subcommand(
            Command::new("serve")
                .about("Start the HTTP server")
                .arg(
                    Arg::new("port")
                        .long("port")
                        .short('p')
                        .value_name("PORT")
                        .help("Port to listen on")
                        .default_value("8080"),
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
            let port = sub_matches
                .get_one::<String>("port")
                .unwrap()
                .parse::<u16>()?;

            let addr = SocketAddr::from(([0, 0, 0, 0], port));

            println!("Starting Merlin server on {}", addr);

            let app = server::create_server();

            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }
        _ => {
            println!("Use --help for usage information");
        }
    }

    Ok(())
}

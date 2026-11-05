use std::net::SocketAddr;

use clap::{Arg, ArgAction, Command};

fn main() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    if let Err(e) = runtime.block_on(async_main()) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn cli() -> Command {
    Command::new("merlin")
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
        .subcommand(
            Command::new("app")
                .about("Manage governed applications")
                .subcommand_required(true)
                .arg(
                    Arg::new("redis")
                        .long("redis")
                        .value_name("URL")
                        .global(true)
                        .help("Redis URL (default: MERLIN_REDIS_URL or redis://127.0.0.1:6379)"),
                )
                .subcommand(
                    Command::new("create")
                        .about("Create an application")
                        .arg(Arg::new("id").long("id").required(true))
                        .arg(Arg::new("name").long("name").required(true)),
                )
                .subcommand(
                    Command::new("policy")
                        .about("Set an application's policy (merges; omitted fields keep current values)")
                        .arg(Arg::new("id").long("id").required(true))
                        .arg(
                            Arg::new("allowed-target")
                                .long("allowed-target")
                                .action(ArgAction::Append)
                                .value_name("TARGET")
                                .help("Restrict to these targets (repeatable)"),
                        )
                        .arg(
                            Arg::new("locality")
                                .long("locality")
                                .value_parser(["any", "local_only"])
                                .help("Locality restriction"),
                        )
                        .arg(
                            Arg::new("max-rpm")
                                .long("max-rpm")
                                .value_parser(clap::value_parser!(u32)),
                        )
                        .arg(
                            Arg::new("max-spend-day")
                                .long("max-spend-day")
                                .value_parser(clap::value_parser!(f64)),
                        )
                        .arg(
                            Arg::new("sensitive-local-only")
                                .long("sensitive-local-only")
                                .action(ArgAction::SetTrue)
                                .help("Raise-only: sensitive requests restricted to local targets"),
                        )
                        .arg(
                            Arg::new("no-sensitive-local-only")
                                .long("no-sensitive-local-only")
                                .action(ArgAction::SetTrue),
                        ),
                )
                .subcommand(
                    Command::new("token")
                        .about("Manage app tokens")
                        .subcommand_required(true)
                        .subcommand(
                            Command::new("issue")
                                .about("Issue a token (shown once)")
                                .arg(Arg::new("id").long("id").required(true)),
                        )
                        .subcommand(
                            Command::new("revoke")
                                .about("Revoke a token")
                                .arg(Arg::new("id").long("id").required(true))
                                .arg(Arg::new("token").long("token").required(true).help("token id")),
                        ),
                )
                .subcommand(Command::new("list").about("List applications"))
                .subcommand(
                    Command::new("show")
                        .about("Show one application")
                        .arg(Arg::new("id").long("id").required(true)),
                ),
        )
}

async fn async_main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let matches = cli().get_matches();

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
        Some(("app", app_matches)) => {
            run_app_command(app_matches).await?;
        }
        _ => {
            eprintln!("Use 'merlin serve --help' or 'merlin app --help' for usage");
        }
    }

    Ok(())
}

async fn run_app_command(matches: &clap::ArgMatches) -> anyhow::Result<()> {
    use merlin::governance::apps::{App, LocalityPolicy, RedisStore, Store};

    let url = matches
        .get_one::<String>("redis")
        .cloned()
        .or_else(|| std::env::var("MERLIN_REDIS_URL").ok())
        .unwrap_or_else(|| "redis://127.0.0.1:6379".to_string());
    let store = RedisStore::connect(&url).await?;

    match matches.subcommand() {
        Some(("create", sub)) => {
            let id = sub.get_one::<String>("id").expect("required");
            let name = sub.get_one::<String>("name").expect("required");
            if store.get_app(id).await?.is_some() {
                anyhow::bail!("app '{id}' already exists");
            }
            let app = App::new(id, name);
            store.put_app(&app).await?;
            println!("created app '{id}'");
        }
        Some(("policy", sub)) => {
            let id = sub.get_one::<String>("id").expect("required");
            let mut app = store
                .get_app(id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("app '{id}' not found"))?;
            if let Some(targets) = sub.get_many::<String>("allowed-target") {
                app.policy.allowed_targets = Some(targets.cloned().collect());
            }
            if let Some(locality) = sub.get_one::<String>("locality") {
                app.policy.locality = match locality.as_str() {
                    "local_only" => LocalityPolicy::LocalOnly,
                    _ => LocalityPolicy::Any,
                };
            }
            if let Some(max_rpm) = sub.get_one::<u32>("max-rpm") {
                app.policy.max_rpm = Some(*max_rpm);
            }
            if let Some(cap) = sub.get_one::<f64>("max-spend-day") {
                app.policy.max_spend_usd_per_day = Some(*cap);
            }
            if sub.get_flag("sensitive-local-only") {
                app.policy.sensitive_local_only = true;
            }
            if sub.get_flag("no-sensitive-local-only") {
                app.policy.sensitive_local_only = false;
            }
            store.put_app(&app).await?;
            println!("updated policy for app '{id}': {:?}", app.policy);
        }
        Some(("token", sub)) => match sub.subcommand() {
            Some(("issue", sub)) => {
                let id = sub.get_one::<String>("id").expect("required");
                if store.get_app(id).await?.is_none() {
                    anyhow::bail!("app '{id}' not found");
                }
                let raw = merlin::governance::apps::mint_token();
                let hash = merlin::governance::apps::hash_token(&raw);
                let token_id = uuid::Uuid::new_v4().to_string();
                store.put_token(id, &token_id, &hash).await?;
                println!("token id:    {token_id}");
                println!("token (shown once, store it now):\n{raw}");
            }
            Some(("revoke", sub)) => {
                let id = sub.get_one::<String>("id").expect("required");
                let token_id = sub.get_one::<String>("token").expect("required");
                store.remove_token(id, token_id).await?;
                println!("revoked token '{token_id}' for app '{id}'");
            }
            _ => unreachable!("subcommand_required"),
        },
        Some(("list", _)) => {
            for app in store.list_apps().await? {
                println!("{}\t{}", app.id, app.name);
            }
        }
        Some(("show", sub)) => {
            let id = sub.get_one::<String>("id").expect("required");
            let app = store
                .get_app(id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("app '{id}' not found"))?;
            let tokens = store.token_ids(id).await?;
            println!("id:       {}", app.id);
            println!("name:     {}", app.name);
            println!("created:  {}", app.created_at);
            println!("policy:   {:?}", app.policy);
            println!("tokens:   {}", tokens.join(", "));
        }
        _ => unreachable!("subcommand_required"),
    }
    Ok(())
}

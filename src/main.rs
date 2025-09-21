use clap::{Arg, Command};
use merlin::{server, feature_numbering};
use std::net::SocketAddr;
use serde_json;

async fn handle_feature_command(matches: &clap::ArgMatches) -> anyhow::Result<()> {
    let storage_file = "features.json";
    let mut api_state = feature_numbering::FeatureApiState::new();

    // Load existing storage if file exists
    if std::path::Path::new(storage_file).exists() {
        api_state.load_storage(storage_file).await?;
    }

    match matches.subcommand() {
        Some(("create", create_matches)) => {
            let name = create_matches.get_one::<String>("name").unwrap();
            let description = create_matches.get_one::<String>("description").unwrap();
            let metadata = create_matches.get_one::<String>("metadata");

            let metadata_obj = if let Some(meta_str) = metadata {
                Some(serde_json::from_str(meta_str)?)
            } else {
                None
            };

            let mut storage = api_state.storage.write().await;
            let mut feature = storage.create_feature(name.clone(), description.clone())?;

            if let Some(meta) = metadata_obj {
                feature.metadata = Some(meta);
                storage.update_feature(&feature.id, feature.clone())?;
            }

            println!("Created feature: {} ({})", feature.id, feature.name);
            println!("Number: {}", feature.number);
            println!("Status: {:?}", feature.status);
        }

        Some(("list", list_matches)) => {
            let storage = api_state.storage.read().await;
            let features = storage.list_features();

            let status_filter = list_matches.get_one::<String>("status");
            let search_term = list_matches.get_one::<String>("search");

            let filtered_features: Vec<_> = features.into_iter().filter(|feature| {
                if let Some(status_str) = status_filter {
                    if format!("{:?}", feature.status) != *status_str {
                        return false;
                    }
                }

                if let Some(search) = search_term {
                    let search_lower = search.to_lowercase();
                    if !feature.name.to_lowercase().contains(&search_lower) &&
                       !feature.description.to_lowercase().contains(&search_lower) {
                        return false;
                    }
                }

                true
            }).collect();

            if filtered_features.is_empty() {
                println!("No features found");
            } else {
                println!("Found {} features:", filtered_features.len());
                for feature in filtered_features {
                    println!("  {} - {} ({:?})", feature.id, feature.name, feature.status);
                }
            }
        }

        Some(("get", get_matches)) => {
            let id = get_matches.get_one::<String>("id").unwrap();
            let storage = api_state.storage.read().await;

            if let Some(feature) = storage.get_feature(id) {
                println!("Feature ID: {}", feature.id);
                println!("Name: {}", feature.name);
                println!("Description: {}", feature.description);
                println!("Number: {}", feature.number);
                println!("Status: {:?}", feature.status);
                println!("Branch: {}", feature.branch_name);
                println!("Created: {}", feature.created_at);
                println!("Updated: {}", feature.updated_at);
                if let Some(ref metadata) = feature.metadata {
                    println!("Metadata: {}", serde_json::to_string_pretty(metadata)?);
                }
            } else {
                eprintln!("Feature not found: {}", id);
            }
        }

        Some(("update", update_matches)) => {
            let id = update_matches.get_one::<String>("id").unwrap();
            let mut storage = api_state.storage.write().await;

            if let Some(mut feature) = storage.get_feature(id).cloned() {
                if let Some(name) = update_matches.get_one::<String>("name") {
                    feature.name = name.clone();
                }

                if let Some(description) = update_matches.get_one::<String>("description") {
                    feature.description = description.clone();
                }

                if let Some(status_str) = update_matches.get_one::<String>("status") {
                    let status = match status_str.as_str() {
                        "Draft" => feature_numbering::FeatureStatus::Draft,
                        "Planned" => feature_numbering::FeatureStatus::Planned,
                        "InProgress" => feature_numbering::FeatureStatus::InProgress,
                        "Completed" => feature_numbering::FeatureStatus::Completed,
                        "Cancelled" => feature_numbering::FeatureStatus::Cancelled,
                        _ => {
                            eprintln!("Invalid status: {}", status_str);
                            return Ok(());
                        }
                    };
                    feature.update_status(status)?;
                }

                if let Some(metadata_str) = update_matches.get_one::<String>("metadata") {
                    let metadata_obj = serde_json::from_str(metadata_str)?;
                    feature.metadata = Some(metadata_obj);
                }

                storage.update_feature(id, feature.clone())?;
                println!("Updated feature: {}", id);
            } else {
                eprintln!("Feature not found: {}", id);
            }
        }

        Some(("delete", delete_matches)) => {
            let id = delete_matches.get_one::<String>("id").unwrap();
            let mut storage = api_state.storage.write().await;

            match storage.delete_feature(id) {
                Ok(_) => println!("Deleted feature: {}", id),
                Err(e) => eprintln!("Failed to delete feature: {}", e),
            }
        }

        Some(("next-number", _)) => {
            let mut storage = api_state.storage.write().await;
            let next_number = storage.get_next_available_number();
            println!("Next available feature number: {}", next_number);
        }

        Some(("reserve", reserve_matches)) => {
            let number = reserve_matches.get_one::<String>("number").unwrap().parse()?;
            let reason = reserve_matches.get_one::<String>("reason").unwrap();

            let mut storage = api_state.storage.write().await;
            match storage.reserve_number(number, reason.clone()) {
                Ok(_) => println!("Reserved number: {}", number),
                Err(e) => eprintln!("Failed to reserve number: {}", e),
            }
        }

        _ => {
            println!("Use 'merlin feature --help' for feature command usage");
        }
    }

    // Save storage after any modifications
    api_state.save_storage(storage_file).await?;

    Ok(())
}

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
                        .default_value("7777"),
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
            Command::new("feature")
                .about("Manage feature numbering")
                .subcommand(
                    Command::new("create")
                        .about("Create a new feature")
                        .arg(Arg::new("name").required(true).help("Feature name"))
                        .arg(Arg::new("description").required(true).help("Feature description"))
                        .arg(Arg::new("metadata").long("metadata").help("JSON metadata")),
                )
                .subcommand(
                    Command::new("list")
                        .about("List all features")
                        .arg(Arg::new("status").long("status").help("Filter by status"))
                        .arg(Arg::new("search").long("search").help("Search term")),
                )
                .subcommand(
                    Command::new("get")
                        .about("Get feature details")
                        .arg(Arg::new("id").required(true).help("Feature ID")),
                )
                .subcommand(
                    Command::new("update")
                        .about("Update a feature")
                        .arg(Arg::new("id").required(true).help("Feature ID"))
                        .arg(Arg::new("name").long("name").help("New feature name"))
                        .arg(Arg::new("description").long("description").help("New description"))
                        .arg(Arg::new("status").long("status").help("New status"))
                        .arg(Arg::new("metadata").long("metadata").help("JSON metadata")),
                )
                .subcommand(
                    Command::new("delete")
                        .about("Delete a feature")
                        .arg(Arg::new("id").required(true).help("Feature ID")),
                )
                .subcommand(
                    Command::new("next-number")
                        .about("Get the next available feature number"),
                )
                .subcommand(
                    Command::new("reserve")
                        .about("Reserve a feature number")
                        .arg(Arg::new("number").required(true).help("Number to reserve"))
                        .arg(Arg::new("reason").required(true).help("Reason for reservation")),
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

            let app = match server::create_server_with_state().await {
                Ok(app) => app,
                Err(e) => {
                    eprintln!("Failed to initialize server: {}", e);
                    eprintln!("Note: Make sure Redis is running for model selection features");
                    // Fallback to basic server without model selection
                    server::create_server()
                }
            };

            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }
        Some(("feature", feature_matches)) => {
            handle_feature_command(feature_matches).await?;
        }
        _ => {
            println!("Use --help for usage information");
        }
    }

    Ok(())
}

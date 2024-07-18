use crate::commands;
use crate::configuration;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Add {
        input: String,
    },
    Get {
        #[clap(short, long)]
        all: bool,
    },
    Recipe {
        input: Option<String>,
    },
    Play,
}

pub async fn run() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => {
            commands::init::init().await;
        }
        _ => {
            let configuration =
                configuration::get_configuration().expect("Failed to load configuration.");

            match cli.command {
                Some(Commands::Add { input }) => {
                    commands::add::add(&configuration, &input).await;
                }
                Some(Commands::Get { all }) => {
                    commands::get::get(&configuration, all).await;
                }
                Some(Commands::Recipe { input }) => {
                    commands::recipe::recipe(&configuration, input).await;
                }
                Some(Commands::Play) => match commands::play::play(&configuration) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("ERROR: game errored out: {e}");
                    }
                },
                _ => {
                    println!(
                        "Incorrect usage of dips. Please check the help section with 'dips help'"
                    );
                }
            }
        }
    }
}

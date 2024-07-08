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
    Add { input: String },
}

pub async fn run() {
    let cli = Cli::parse();
    let configuration = configuration::get_configuration().expect("Failed to load configuration.");
    match cli.command {
        Some(Commands::Init) => {
            commands::init::init(&configuration).await;
        }
        Some(Commands::Add { input }) => {
            commands::add::add(&configuration, &input).await;
        }
        None => {
            eprintln!("Incorrect usage of dip.");
        }
    }
}

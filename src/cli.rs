use crate::commands;
use crate::configuration::{Application, ConfigError, Environment, Settings};
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
        #[arg(short = 't', long)]
        tag: Option<String>,
        #[arg(short, long)]
        global: bool,
    },
    Get {
        #[clap(short, long)]
        all: bool,
    },
}

pub async fn run() {
    let cli = Cli::parse();
    let settings = Settings::build(&Environment::current());

    match cli.command {
        Some(Commands::Init) => {
            commands::init::init(settings).await;
        }
        _ => {
            let app = match Application::build(settings).await {
                Ok(app) => app,
                Err(e) => match e {
                    ConfigError::Uninitialized => {
                        println!("Dips is not initialized. Please run `dips init`");
                        std::process::exit(0);
                    }
                },
            };

            match cli.command {
                Some(Commands::Add { input, tag, global }) => {
                    commands::add::add(&app, &input, tag.as_deref(), global).await;
                }
                Some(Commands::Get { all }) => {
                    commands::get::exec(&app, all).await;
                }
                _ => commands::core::exec(app)
                    .await
                    .expect("Failed to run the app"),
            }
        }
    }
}

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Add { input: String },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Add { input }) => {
            println!("we are adding {}", input);
            let dir = std::env::current_dir().expect("Failed to find current directory");
            println!(" curr directory is {:?}", dir);
        }
        None => {}
    }
}
